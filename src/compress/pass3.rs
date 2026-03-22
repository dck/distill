use crate::error::Result;
use crate::llm::LlmClient;
use crate::llm::parse::parse_llm_response;
use crate::llm::strategy::CompressionStrategy;

pub async fn refine(
    client: &LlmClient,
    deduplicated: &str,
    strategy: &dyn CompressionStrategy,
) -> Result<String> {
    let system = strategy.refinement_system();
    let user = strategy.refinement_user(deduplicated);

    let response = client.complete(&system, &user).await?;

    match parse_llm_response(&response) {
        Ok(parsed) => Ok(parsed.compressed),
        Err(_) => {
            crate::ui::warning("refinement pass failed to parse, keeping pass 2 output");
            Ok(deduplicated.to_string())
        }
    }
}
