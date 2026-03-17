use std::path::Path;

use crate::error::{DistillError, Result};
use crate::ingest::{Document, InputSource};
use crate::mode::estimate_tokens;

pub fn ingest_epub(path: &Path) -> Result<Document> {
    let mut doc = epub::doc::EpubDoc::new(path).map_err(|e| DistillError::Ingestion {
        source: path.display().to_string(),
        cause: format!("failed to open EPUB: {e}"),
    })?;

    let title = doc.mdata("title").map(|m| m.value.clone());
    let author = doc.mdata("creator").map(|m| m.value.clone());

    let mut parts = Vec::new();
    let num_pages = doc.spine.len();

    for _ in 0..num_pages {
        if let Some((text, _mime)) = doc.get_current_str() {
            let md = html2md::parse_html(&text);
            if !md.trim().is_empty() {
                parts.push(md);
            }
        }
        doc.go_next();
    }

    let content = parts.join("\n\n");
    let tokens = estimate_tokens(&content);

    Ok(Document {
        title,
        author,
        content,
        source: InputSource::File(path.to_path_buf()),
        estimated_tokens: tokens,
    })
}
