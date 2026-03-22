use crate::error::Result;
use crate::llm::LlmClient;
use crate::llm::parse::parse_llm_response;
use crate::llm::strategy::CompressionStrategy;

pub async fn refine(
    client: &LlmClient,
    combined_distilled: &str,
    strategy: &dyn CompressionStrategy,
) -> Result<String> {
    let system = strategy.refinement_system();
    let user = strategy.refinement_user(combined_distilled);

    let response = client.complete(&system, &user).await?;

    match parse_llm_response(&response) {
        Ok(parsed) => Ok(parsed.compressed),
        Err(_) => {
            crate::ui::warning("refinement pass failed to parse, keeping pass 1 output");
            Ok(combined_distilled.to_string())
        }
    }
}
