#[path = "integration/helpers/mod.rs"]
mod helpers;

use distill::llm::LlmClient;
use distill::llm::strategy;
use std::time::Duration;

#[tokio::test]
async fn mock_llm_returns_compressed_response() {
    let server = helpers::mock_llm::start_mock_llm().await;

    let client = LlmClient::new_with_retry_delays(
        "test-key".into(),
        server.uri(),
        "test-model".into(),
        [Duration::ZERO, Duration::ZERO, Duration::ZERO],
        0,
    );

    let result = client.complete("system prompt", "user input").await;
    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let content = result.unwrap();
    assert!(
        content.contains("<compressed>"),
        "response should contain <compressed> tag"
    );
}

#[tokio::test]
async fn tldr_strategy_produces_structured_extraction() {
    let server = helpers::mock_llm::start_mock_llm_tldr().await;

    let client = LlmClient::new_with_retry_delays(
        "test-key".into(),
        server.uri(),
        "test-model".into(),
        [Duration::ZERO, Duration::ZERO, Duration::ZERO],
        0,
    );

    let strategy = strategy::strategy_for(&distill::cli::CompressionLevel::Tldr);
    let system = strategy.article_system();
    let user = strategy.distill_user("Some article about testing.");

    let result = client.complete(&system, &user).await;
    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let content = result.unwrap();
    assert!(
        content.contains("<compressed>"),
        "response should contain <compressed> tag"
    );
    assert!(
        content.contains("TL;DR"),
        "response should contain TL;DR section"
    );
    assert!(
        content.contains("Key Ideas"),
        "response should contain Key Ideas section"
    );
}
