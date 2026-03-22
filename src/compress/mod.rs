pub mod pass1;
pub mod pass2;

use crate::error::Result;
use crate::llm::LlmClient;
use crate::llm::strategy::CompressionStrategy;
use crate::segment::Chunk;
use crate::ui::Console;
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
) -> Result<String> {
    let chunk_count = chunks.len();

    // Pass 1: Independent distillation
    let pb = console.progress(chunk_count as u64, "Pass 1: Distilling");

    let compressed = if parallel {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(jobs));
        let mut handles = Vec::new();

        for chunk in &chunks {
            let sem = semaphore.clone();
            let client = client.clone();
            let chunk = chunk.clone();
            let strategy = strategy.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                pass1::distill_chunk(&client, &chunk, strategy.as_ref()).await
            }));
        }

        let mut results = Vec::new();
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle
                .await
                .map_err(|e| crate::error::DistillError::Compression {
                    chunk_index: i,
                    section: String::new(),
                    cause: e.to_string(),
                })??;
            results.push(result);
            pb.inc();
        }
        results
    } else {
        let mut results = Vec::new();
        for chunk in &chunks {
            let result = pass1::distill_chunk(&client, chunk, strategy.as_ref()).await?;
            results.push(result);
            pb.inc();
        }
        results
    };

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
