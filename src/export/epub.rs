use std::path::Path;

use crate::error::Result;

pub fn export_epub(
    _content: &str,
    _title: Option<&str>,
    _author: Option<&str>,
    _output_path: Option<&Path>,
) -> Result<()> {
    todo!("EPUB export not yet implemented")
}
