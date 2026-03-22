pub mod pass1;
pub mod pass2;
pub mod pass3;

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
    client: Arc<LlmClient>,
    chunks: Vec<Chunk>,
    strategy: Arc<dyn CompressionStrategy>,
    checkpoint: Option<(PathBuf, Checkpoint)>,
) -> Result<String> {
    let mut checkpoint = checkpoint;

    if let Some((_, state)) = &checkpoint
        && state.completed_pass >= 1
        && let Some(output) = &state.final_output
    {
        return Ok(output.clone());
    }

    let compressed = run_pass1(client, &chunks, strategy, false, 1, None, &mut checkpoint).await?;
    let output = compressed
        .iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    if let Some((path, state)) = checkpoint.as_mut() {
        state.mark_finished(1, output.clone());
        state.save(path)?;
    }

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
    let mut checkpoint = checkpoint;

    if let Some((_, state)) = &checkpoint
        && state.completed_pass >= 3
        && let Some(output) = &state.final_output
    {
        return Ok(output.clone());
    }

    let chunk_count = chunks.len();

    let pb = console.progress(chunk_count as u64, "Pass 1: Distilling");
    let compressed = run_pass1(
        client.clone(),
        &chunks,
        strategy.clone(),
        parallel,
        jobs,
        Some(&pb),
        &mut checkpoint,
    )
    .await?;
    pb.finish();

    if let Some((path, state)) = checkpoint.as_mut() {
        state.completed_pass = state.completed_pass.max(1);
        state.save(path)?;
    }

    console.pass_done("Pass 1", &format!("Distilled {chunk_count} chapters"));

    let combined = compressed
        .iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let deduplicated = if let Some((_, state)) = &checkpoint
        && state.completed_pass >= 2
    {
        state
            .pass2_output
            .clone()
            .ok_or_else(|| DistillError::CheckpointMismatch {
                field: "pass 2 output",
                expected: "present".into(),
                found: "missing".into(),
            })?
    } else {
        let sp = console.spinner("Pass 2: Deduplicating globally...");
        let deduplicated = pass2::dedup(&client, &combined, strategy.as_ref()).await?;
        sp.finish();
        console.pass_done("Pass 2", "Global redundancy reduced");
        if let Some((path, state)) = checkpoint.as_mut() {
            state.mark_pass2(deduplicated.clone());
            state.save(path)?;
        }
        deduplicated
    };

    if let Some((_, state)) = &checkpoint
        && state.completed_pass >= 3
        && let Some(output) = &state.final_output
    {
        return Ok(output.clone());
    }

    let sp = console.spinner("Pass 3: Refining coherence...");
    let refined = pass3::refine(&client, &deduplicated, strategy.as_ref()).await?;
    sp.finish();
    console.pass_done("Pass 3", "Coherence refined");

    if let Some((path, state)) = checkpoint.as_mut() {
        state.mark_finished(3, refined.clone());
        state.save(path)?;
    }

    Ok(refined)
}

async fn run_pass1(
    client: Arc<LlmClient>,
    chunks: &[Chunk],
    strategy: Arc<dyn CompressionStrategy>,
    parallel: bool,
    jobs: usize,
    progress: Option<&crate::ui::Progress>,
    checkpoint: &mut Option<(PathBuf, Checkpoint)>,
) -> Result<Vec<CompressedChunk>> {
    let mut results =
        restore_checkpointed_chunks(checkpoint.as_ref().map(|(_, state)| state), chunks);

    if parallel {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(jobs));
        let mut handles = Vec::new();

        for chunk in chunks {
            if results[chunk.index].is_some() {
                if let Some(progress) = progress {
                    progress.inc();
                }
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
                    pass1::distill_chunk(client.as_ref(), &chunk, strategy.as_ref()).await
                }),
            ));
        }

        for (chunk_index, section, handle) in handles {
            let result = handle.await.map_err(|e| DistillError::Compression {
                chunk_index,
                section: section.clone(),
                cause: e.to_string(),
            })??;
            results[chunk_index] = Some(result.clone());
            save_checkpoint_chunk(checkpoint, &result)?;
            if let Some(progress) = progress {
                progress.inc();
            }
        }
    } else {
        for chunk in chunks {
            if results[chunk.index].is_some() {
                if let Some(progress) = progress {
                    progress.inc();
                }
                continue;
            }

            let result = pass1::distill_chunk(client.as_ref(), chunk, strategy.as_ref()).await?;
            results[chunk.index] = Some(result.clone());
            save_checkpoint_chunk(checkpoint, &result)?;
            if let Some(progress) = progress {
                progress.inc();
            }
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
        })
        .collect()
}

fn restore_checkpointed_chunks(
    checkpoint: Option<&Checkpoint>,
    chunks: &[Chunk],
) -> Vec<Option<CompressedChunk>> {
    if let Some(state) = checkpoint
        && let Ok(saved) = state.compressed_chunks(chunks)
    {
        let mut restored = vec![None; chunks.len()];
        for chunk in saved {
            let index = chunk.index;
            restored[index] = Some(chunk);
        }
        return restored;
    }

    let mut restored = vec![None; chunks.len()];
    if let Some(state) = checkpoint {
        for (saved, chunk) in state.chunks.iter().zip(chunks) {
            if let Some(content) = &saved.compressed {
                restored[chunk.index] = Some(CompressedChunk {
                    index: chunk.index,
                    header_path: chunk.header_path.clone(),
                    content: content.clone(),
                });
            }
        }
    }

    restored
}

fn save_checkpoint_chunk(
    checkpoint: &mut Option<(PathBuf, Checkpoint)>,
    result: &CompressedChunk,
) -> Result<()> {
    if let Some((path, state)) = checkpoint.as_mut() {
        state.update_chunk(result);
        state.save(path)?;
    }
    Ok(())
}
