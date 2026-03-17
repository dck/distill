use crate::error::Result;
use crate::llm::LlmClient;
use crate::llm::parse::parse_llm_response;
use crate::llm::prompt;
use crate::state::CompressedChunk;

pub async fn refine(client: &LlmClient, chunks: &[CompressedChunk]) -> Result<String> {
    let combined = chunks
        .iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let system = prompt::pass3_system();
    let user = prompt::pass3_user(&combined);

    let response = client.complete(&system, &user).await?;

    match parse_llm_response(&response) {
        Ok(parsed) => Ok(parsed.compressed),
        Err(_) => {
            eprintln!("warning: pass 3 (refinement) failed to parse, keeping pass 2 output");
            Ok(combined)
        }
    }
}
