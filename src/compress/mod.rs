pub mod pass1;
pub mod pass2;

use crate::error::{DistillError, Result};
use crate::llm::LlmClient;
use crate::llm::strategy::CompressionStrategy;
use crate::segment::Chunk;
use crate::state::CompressedChunk;
use crate::state::checkpoint::Checkpoint;
use crate::ui::Console;
use std::path::PathBuf;
use std::sync::Arc;

pub async fn single_pass(
    client: &LlmClient,
    chunks: Vec<Chunk>,
    strategy: &dyn CompressionStrategy,
) -> Result<String> {
    let mut compressed = Vec::new();
    for chunk in &chunks {
        let result = pass1::distill_chunk(client, chunk, strategy).await?;
        compressed.push(result);
    }
    let output = compressed
        .iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(output)
}

pub async fn hierarchical(
    client: Arc<LlmClient>,
    chunks: Vec<Chunk>,
    strategy: Arc<dyn CompressionStrategy>,
    parallel: bool,
    jobs: usize,
    console: &Console,
    checkpoint: Option<(PathBuf, Checkpoint)>,
) -> Result<String> {
    let chunk_count = chunks.len();

    // Pass 1: Independent distillation
    let pb = console.progress(chunk_count as u64, "Pass 1: Distilling");
    let compressed = run_pass1(
        client.clone(),
        &chunks,
        strategy.clone(),
        parallel,
        jobs,
        &pb,
        checkpoint,
    )
    .await?;

    pb.finish();
    console.pass_done("Pass 1", &format!("Distilled {chunk_count} chapters"));

    let combined = compressed
        .iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    // Pass 2: Coherence refinement
    let sp = console.spinner("Pass 2: Refining coherence...");
    let refined = pass2::refine(&client, &combined, strategy.as_ref()).await?;
    sp.finish();
    console.pass_done("Pass 2", "Coherence refined");

    Ok(refined)
}

async fn run_pass1(
    client: Arc<LlmClient>,
    chunks: &[Chunk],
    strategy: Arc<dyn CompressionStrategy>,
    parallel: bool,
    jobs: usize,
    progress: &crate::ui::Progress,
    checkpoint: Option<(PathBuf, Checkpoint)>,
) -> Result<Vec<CompressedChunk>> {
    let mut checkpoint = checkpoint;
    let mut results = restore_results(
        chunks,
        checkpoint.as_ref().map(|(_, state)| state),
        progress,
    );

    if parallel {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(jobs));
        let mut handles = Vec::new();

        for chunk in chunks {
            if results[chunk.index].is_some() {
                continue;
            }

            let sem = semaphore.clone();
            let client = client.clone();
            let chunk = chunk.clone();
            let strategy = strategy.clone();
            let section = chunk.header_path.join(" > ");

            handles.push((
                chunk.index,
                section.clone(),
                tokio::spawn(async move {
                    let _permit = sem.acquire().await.map_err(|e| DistillError::Compression {
                        chunk_index: chunk.index,
                        section: section.clone(),
                        cause: e.to_string(),
                    })?;
                    pass1::distill_chunk(&client, &chunk, strategy.as_ref()).await
                }),
            ));
        }

        for (index, section, handle) in handles {
            let result = handle.await.map_err(|e| DistillError::Compression {
                chunk_index: index,
                section,
                cause: e.to_string(),
            })??;
            results[index] = Some(result.clone());
            persist_chunk(&mut checkpoint, &result)?;
            progress.inc();
        }
    } else {
        for chunk in chunks {
            if results[chunk.index].is_some() {
                continue;
            }

            let result = pass1::distill_chunk(&client, chunk, strategy.as_ref()).await?;
            results[chunk.index] = Some(result.clone());
            persist_chunk(&mut checkpoint, &result)?;
            progress.inc();
        }
    }

    results
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            item.ok_or_else(|| DistillError::Compression {
                chunk_index: index,
                section: chunks[index].header_path.join(" > "),
                cause: "missing compressed output".into(),
            })
            .map_err(Into::into)
        })
        .collect()
}

fn restore_results(
    chunks: &[Chunk],
    checkpoint: Option<&Checkpoint>,
    progress: &crate::ui::Progress,
) -> Vec<Option<CompressedChunk>> {
    let mut results = vec![None; chunks.len()];

    if let Some(checkpoint) = checkpoint {
        for chunk in chunks {
            if let Some(content) = checkpoint.compressed_for(chunk.index) {
                results[chunk.index] = Some(CompressedChunk {
                    index: chunk.index,
                    header_path: chunk.header_path.clone(),
                    content: content.to_string(),
                });
                progress.inc();
            }
        }
    }

    results
}

fn persist_chunk(
    checkpoint: &mut Option<(PathBuf, Checkpoint)>,
    result: &CompressedChunk,
) -> Result<()> {
    if let Some((path, state)) = checkpoint.as_mut() {
        state.update_chunk(result.index, result.content.clone());
        if state.all_chunks_compressed() {
            state.completed_pass = 1;
        }
        state.save(path)?;
    }
    Ok(())
}
