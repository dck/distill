pub mod epub;
pub mod pdf;
pub mod url;

use crate::error::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Document {
    pub title: Option<String>,
    pub author: Option<String>,
    pub content: String,
    pub estimated_tokens: usize,
}

pub async fn ingest(input: &str) -> Result<Document> {
    if crate::mode::is_url(input) {
        url::ingest_url(input).await
    } else {
        let path = PathBuf::from(input);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "pdf" => pdf::ingest_pdf(&path),
            "epub" => epub::ingest_epub(&path),
            _ => Err(crate::error::DistillError::UnsupportedInput {
                source: input.into(),
                extension: ext,
            }),
        }
    }
}
