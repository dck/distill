use crate::error::{DistillError, Result};
use crate::ingest::Document;
use crate::mode::estimate_tokens;

pub async fn ingest_url(url: &str) -> Result<Document> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| DistillError::Ingestion {
            source: url.into(),
            cause: e.to_string(),
        })?;

    let content_type = response
        .headers()
        .get("content-type")
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
        let tmp = std::env::temp_dir().join("distill-download.pdf");
        std::fs::write(&tmp, &bytes).map_err(|e| DistillError::Ingestion {
            source: url.into(),
            cause: e.to_string(),
        })?;
        return crate::ingest::pdf::ingest_pdf(&tmp);
    }

    if content_type.contains("application/epub") || url.ends_with(".epub") {
        let bytes = response
            .bytes()
            .await
            .map_err(|e| DistillError::Ingestion {
                source: url.into(),
                cause: e.to_string(),
            })?;
        let tmp = std::env::temp_dir().join("distill-download.epub");
        std::fs::write(&tmp, &bytes).map_err(|e| DistillError::Ingestion {
            source: url.into(),
            cause: e.to_string(),
        })?;
        return crate::ingest::epub::ingest_epub(&tmp);
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
    let start = html.find("<title>")?;
    let end = html.find("</title>")?;
    let title = &html[start + 7..end];
    Some(title.trim().to_string())
}
