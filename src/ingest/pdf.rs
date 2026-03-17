use std::path::Path;

use crate::error::{DistillError, Result};
use crate::ingest::{Document, InputSource};
use crate::mode::estimate_tokens;

pub fn ingest_pdf(path: &Path) -> Result<Document> {
    let bytes = std::fs::read(path).map_err(|e| DistillError::Ingestion {
        source: path.display().to_string(),
        cause: e.to_string(),
    })?;

    let content = pdf_extract::extract_text_from_mem(&bytes).map_err(|e| {
        DistillError::Ingestion {
            source: path.display().to_string(),
            cause: format!("failed to extract text from PDF: {e}"),
        }
    })?;

    let tokens = estimate_tokens(&content);

    Ok(Document {
        title: None,
        author: None,
        content,
        source: InputSource::File(path.to_path_buf()),
        estimated_tokens: tokens,
    })
}
