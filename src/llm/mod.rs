use crate::error::{DistillError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod parse;
pub mod prompt;

const MAX_RETRIES: u32 = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(4),
    Duration::from_secs(16),
];

#[derive(Debug)]
pub struct LlmClient {
    http: reqwest::Client,
    api_key: String,
    api_base: String,
    model: String,
    retry_delays: [Duration; 3],
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

impl LlmClient {
    pub fn new(api_key: String, api_base: String, model: String) -> Self {
        Self::new_with_retry_delays(api_key, api_base, model, DEFAULT_RETRY_DELAYS)
    }

    pub fn new_with_retry_delays(
        api_key: String,
        api_base: String,
        model: String,
        retry_delays: [Duration; 3],
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            api_key,
            api_base,
            model,
            retry_delays,
        }
    }

    pub async fn complete(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.api_base);
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".into(),
                    content: system.into(),
                },
                Message {
                    role: "user".into(),
                    content: user.into(),
                },
            ],
        };

        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(self.retry_delays[(attempt - 1) as usize]).await;
            }

            let response = self
                .http
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let chat_resp: ChatResponse =
                            resp.json().await.map_err(|e| DistillError::Llm {
                                cause: format!("failed to parse response: {e}"),
                            })?;
                        let content = chat_resp
                            .choices
                            .into_iter()
                            .next()
                            .ok_or_else(|| DistillError::Llm {
                                cause: "empty response from LLM".into(),
                            })?
                            .message
                            .content;
                        return Ok(content);
                    }

                    let should_retry = status.as_u16() == 429 || status.is_server_error();
                    let err_msg = format!("HTTP {status}");
                    if should_retry && attempt < MAX_RETRIES {
                        last_err = Some(err_msg);
                        continue;
                    }
                    return Err(DistillError::Llm { cause: err_msg }.into());
                }
                Err(e) => {
                    let is_timeout = e.is_timeout() || e.is_connect();
                    let err_msg = e.to_string();
                    if is_timeout && attempt < MAX_RETRIES {
                        last_err = Some(err_msg);
                        continue;
                    }
                    return Err(DistillError::Llm { cause: err_msg }.into());
                }
            }
        }

        Err(DistillError::Llm {
            cause: format!(
                "exhausted {MAX_RETRIES} retries. last error: {}",
                last_err.unwrap_or_default()
            ),
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client(uri: String) -> LlmClient {
        LlmClient::new_with_retry_delays(
            "test-key".into(),
            uri,
            "test-model".into(),
            [Duration::ZERO, Duration::ZERO, Duration::ZERO],
        )
    }

    #[tokio::test]
    async fn test_successful_completion() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "choices": [{"message": {"content": "compressed output"}}]
                })),
            )
            .mount(&server)
            .await;

        let client = test_client(server.uri());

        let result = client.complete("system prompt", "user prompt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "compressed output");
    }

    #[tokio::test]
    async fn test_retry_on_429() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(429))
            .up_to_n_times(2)
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "choices": [{"message": {"content": "success after retry"}}]
                })),
            )
            .mount(&server)
            .await;

        let client = test_client(server.uri());

        let result = client.complete("sys", "user").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success after retry");
    }

    #[tokio::test]
    async fn test_exhausted_retries() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = test_client(server.uri());

        let result = client.complete("sys", "user").await;
        assert!(result.is_err());
    }
}
