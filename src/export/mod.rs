pub mod epub;
pub mod html;
pub mod markdown;

use std::path::Path;

use crate::cli::OutputFormat;
use crate::error::Result;

pub fn export(
    content: &str,
    title: Option<&str>,
    author: Option<&str>,
    format: &OutputFormat,
    output_path: Option<&Path>,
) -> Result<()> {
    match format {
        OutputFormat::Md => markdown::export_markdown(content, output_path),
        OutputFormat::Html => html::export_html(content, title, output_path),
        OutputFormat::Epub => epub::export_epub(content, title, author, output_path),
    }
}
