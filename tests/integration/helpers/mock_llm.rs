use serde_json::json;
use wiremock::matchers;
use wiremock::{Mock, MockServer, ResponseTemplate};

pub async fn start_mock_llm() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{
                "message": {
                    "content": "<compressed>\n## Compressed Section\n\nThis is compressed content.\n</compressed>\n<ledger>\n{\"new_concepts\": [], \"new_definitions\": [], \"new_principles\": [], \"new_examples\": [], \"new_anti_patterns\": [], \"new_relationships\": []}\n</ledger>"
                }
            }]
        })))
        .mount(&server)
        .await;

    server
}
