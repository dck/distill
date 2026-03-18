pub mod pass1;
pub mod pass2;
pub mod pass3;

use crate::cli::CompressionLevel;
use crate::error::Result;
use crate::llm::LlmClient;
use crate::segment::Chunk;
use crate::state::{CompressedChunk, StateLedger};
use crate::ui::Console;
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
    console: &Console,
) -> Result<String> {
    let chunk_count = chunks.len();
    let mut ledger = StateLedger::default();
    let compressed: Vec<CompressedChunk>;

    // Pass 1: Local compression
    let pb = console.progress(chunk_count as u64, "Pass 1: Compressing");

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
            pb.inc();
        }
        compressed = results;
    } else {
        let mut results = Vec::new();
        for chunk in &chunks {
            let result = pass1::compress_chunk(&client, chunk, level, &ledger).await?;
            ledger.apply_delta(&result.ledger_updates);
            results.push(result);
            pb.inc();
        }
        compressed = results;
    }

    pb.finish();
    console.pass_done("Pass 1", &format!("Compressed {chunk_count} chunks"));

    // Pass 2: Global deduplication
    let sp = console.spinner("Pass 2: Deduplicating...");
    let deduped = pass2::deduplicate(&client, &compressed, &ledger).await?;
    sp.finish();
    console.pass_done("Pass 2", "Deduplicated");

    // Pass 3: Refinement
    let sp = console.spinner("Pass 3: Refining...");
    let refined = pass3::refine(&client, &deduped).await?;
    sp.finish();
    console.pass_done("Pass 3", "Refined");

    Ok(refined)
}
