use crate::error::Result;
use crate::llm::LlmClient;
use crate::llm::parse::{ParsedResponse, parse_llm_response};
use crate::llm::strategy::CompressionStrategy;
use crate::segment::Chunk;
use crate::state::CompressedChunk;
use crate::ui;

pub async fn distill_chunk(
    client: &LlmClient,
    chunk: &Chunk,
    strategy: &dyn CompressionStrategy,
) -> Result<CompressedChunk> {
    let system = strategy.distill_system();
    let user = strategy.distill_user(&chunk.content);

    let response = client.complete(&system, &user).await?;

    let parsed = match parse_llm_response(&response) {
        Ok(p) => p,
        Err(_) => {
            let retry_response = client
                .complete(
                    &format!("{system}\n\nIMPORTANT: You MUST use <compressed> and </compressed> XML tags."),
                    &user,
                )
                .await?;
            parse_llm_response(&retry_response).unwrap_or_else(|_| {
                ui::warning(&format!(
                    "chunk {} could not be parsed after retry, keeping original (section: \"{}\")",
                    chunk.index,
                    chunk.header_path.join(" > ")
                ));
                ParsedResponse {
                    compressed: chunk.content.clone(),
                }
            })
        }
    };

    Ok(CompressedChunk {
        index: chunk.index,
        header_path: chunk.header_path.clone(),
        content: parsed.compressed,
    })
}
