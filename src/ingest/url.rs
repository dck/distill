use crate::error::{DistillError, Result};
use crate::ingest::Document;
use crate::mode::estimate_tokens;
use reqwest::header::CONTENT_TYPE;
use std::io::Write;
use std::time::Duration;
use tempfile::NamedTempFile;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(3)];
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

pub async fn ingest_url(url: &str) -> Result<Document> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| DistillError::Ingestion {
            source: url.into(),
            cause: format!("failed to build HTTP client: {e}"),
        })?;

    let response = fetch_with_retry(&client, url).await?;
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if content_type.contains("application/pdf") || url.ends_with(".pdf") {
        let bytes = response
            .bytes()
            .await
            .map_err(|e| DistillError::Ingestion {
                source: url.into(),
                cause: e.to_string(),
            })?;
        return ingest_downloaded_file(url, &bytes, ".pdf", crate::ingest::pdf::ingest_pdf);
    }

    if content_type.contains("application/epub")
        || content_type.contains("application/epub+zip")
        || url.ends_with(".epub")
    {
        let bytes = response
            .bytes()
            .await
            .map_err(|e| DistillError::Ingestion {
                source: url.into(),
                cause: e.to_string(),
            })?;
        return ingest_downloaded_file(url, &bytes, ".epub", crate::ingest::epub::ingest_epub);
    }

    // HTML — extract article
    let html = response.text().await.map_err(|e| DistillError::Ingestion {
        source: url.into(),
        cause: e.to_string(),
    })?;

    let content = extract_article(&html, url)?;

    if content.split_whitespace().count() < 20 {
        return Err(DistillError::Ingestion {
            source: url.into(),
            cause: "extracted content is too short (fewer than 20 words). This URL may be a JavaScript-rendered SPA — distill cannot process pages that require a browser to render.".into(),
        }
        .into());
    }

    let tokens = estimate_tokens(&content);

    Ok(Document {
        title: extract_title(&html),
        author: None,
        content,
        estimated_tokens: tokens,
    })
}

async fn fetch_with_retry(client: &reqwest::Client, url: &str) -> Result<reqwest::Response> {
    let mut last_error = None;

    for attempt in 0..=RETRY_DELAYS.len() {
        if attempt > 0 {
            tokio::time::sleep(RETRY_DELAYS[attempt - 1]).await;
        }

        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    return Ok(response);
                }

                let body = response.text().await.unwrap_or_default();
                let message = format!("HTTP {status}: {}", preview(&body, 240));
                if (status.as_u16() == 429 || status.is_server_error())
                    && attempt < RETRY_DELAYS.len()
                {
                    last_error = Some(message);
                    continue;
                }
                let cause = if status.as_u16() == 403 {
                    format!(
                        "HTTP 403 Forbidden — this site blocks automated access.\n  \
                        Try saving the page as HTML or PDF and running distill on the file instead."
                    )
                } else {
                    message
                };
                return Err(DistillError::Ingestion {
                    source: url.into(),
                    cause,
                }
                .into());
            }
            Err(e) => {
                let message = e.to_string();
                if (e.is_timeout() || e.is_connect()) && attempt < RETRY_DELAYS.len() {
                    last_error = Some(message);
                    continue;
                }
                return Err(DistillError::Ingestion {
                    source: url.into(),
                    cause: message,
                }
                .into());
            }
        }
    }

    Err(DistillError::Ingestion {
        source: url.into(),
        cause: format!(
            "request failed after retries{}",
            last_error
                .as_deref()
                .map(|err| format!(": {err}"))
                .unwrap_or_default()
        ),
    }
    .into())
}

fn ingest_downloaded_file(
    url: &str,
    bytes: &[u8],
    suffix: &str,
    ingest: fn(&std::path::Path) -> Result<Document>,
) -> Result<Document> {
    let mut file = NamedTempFile::with_suffix(suffix).map_err(|e| DistillError::Ingestion {
        source: url.into(),
        cause: format!("failed to create temp file: {e}"),
    })?;
    file.write_all(bytes).map_err(|e| DistillError::Ingestion {
        source: url.into(),
        cause: format!("failed to write temp file: {e}"),
    })?;
    file.flush().map_err(|e| DistillError::Ingestion {
        source: url.into(),
        cause: format!("failed to flush temp file: {e}"),
    })?;
    ingest(file.path())
}

fn extract_article(html: &str, url: &str) -> Result<String> {
    let parsed_url = ::url::Url::parse(url).map_err(|e| DistillError::Ingestion {
        source: url.into(),
        cause: format!("invalid URL: {e}"),
    })?;

    let product =
        readability::extractor::extract(&mut html.as_bytes(), &parsed_url).map_err(|e| {
            DistillError::Ingestion {
                source: url.into(),
                cause: format!("article extraction failed: {e}"),
            }
        })?;

    let markdown = html2md::parse_html(&product.content);
    Ok(markdown)
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title>")? + "<title>".len();
    let rest = lower.get(start..)?;
    let end = start + rest.find("</title>")?;
    let title = html.get(start..end)?.trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

fn preview(text: &str, max_len: usize) -> String {
    if text.is_empty() {
        "<empty>".into()
    } else if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_case_insensitive() {
        assert_eq!(
            extract_title("<html><head><TITLE>Hello</TITLE></head></html>").as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn test_extract_title_malformed_returns_none() {
        assert_eq!(extract_title("<title>Missing close"), None);
    }

    #[test]
    fn test_preview_truncates() {
        let text = "a".repeat(300);
        assert_eq!(preview(&text, 5), "aaaaa...");
    }
}
