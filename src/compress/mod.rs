pub mod pass1;
pub mod pass2;
pub mod pass3;

use crate::cli::CompressionLevel;
use crate::error::Result;
use crate::llm::LlmClient;
use crate::segment::Chunk;
use crate::state::{CompressedChunk, StateLedger};
use std::sync::Arc;

pub async fn single_pass(
    client: &LlmClient,
    chunks: Vec<Chunk>,
    level: &CompressionLevel,
) -> Result<String> {
    let mut compressed = Vec::new();

    for chunk in &chunks {
        let result = pass1::compress_chunk_single_pass(client, chunk, level).await?;
        compressed.push(result);
    }

    let output = compressed
        .iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(output)
}

pub async fn multi_pass(
    client: Arc<LlmClient>,
    chunks: Vec<Chunk>,
    level: &CompressionLevel,
    parallel: bool,
    jobs: usize,
) -> Result<String> {
    // Pass 1: Local compression
    let mut ledger = StateLedger::default();
    let compressed: Vec<CompressedChunk>;

    if parallel {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(jobs));
        let mut handles = Vec::new();

        for chunk in &chunks {
            let sem = semaphore.clone();
            let client = client.clone();
            let chunk = chunk.clone();
            let level = level.clone();
            let ledger_snapshot = ledger.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                pass1::compress_chunk(&client, &chunk, &level, &ledger_snapshot).await
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            let result = handle
                .await
                .map_err(|e| crate::error::DistillError::Compression {
                    chunk_index: 0,
                    section: String::new(),
                    cause: e.to_string(),
                })??;
            ledger.apply_delta(&result.ledger_updates);
            results.push(result);
        }
        compressed = results;
    } else {
        let mut results = Vec::new();
        for chunk in &chunks {
            let result = pass1::compress_chunk(&client, chunk, level, &ledger).await?;
            ledger.apply_delta(&result.ledger_updates);
            results.push(result);
        }
        compressed = results;
    }

    // Pass 2: Global deduplication
    let deduped = pass2::deduplicate(&client, &compressed, &ledger).await?;

    // Pass 3: Refinement
    let refined = pass3::refine(&client, &deduped).await?;

    Ok(refined)
}
