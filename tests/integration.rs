#[path = "integration/helpers/mod.rs"]
mod helpers;

use distill::llm::LlmClient;
use std::time::Duration;

#[tokio::test]
async fn mock_llm_returns_compressed_response() {
    let server = helpers::mock_llm::start_mock_llm().await;

    let client = LlmClient::new_with_retry_delays(
        "test-key".into(),
        server.uri(),
        "test-model".into(),
        [Duration::ZERO, Duration::ZERO, Duration::ZERO],
    );

    let result = client.complete("system prompt", "user input").await;
    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let content = result.unwrap();
    assert!(
        content.contains("<compressed>"),
        "response should contain <compressed> tag"
    );
    assert!(
        content.contains("<ledger>"),
        "response should contain <ledger> tag"
    );
}
