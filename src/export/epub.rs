use std::fs::File;
use std::path::Path;

use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};

use crate::error::{DistillError, Result};
use crate::export::html::md_to_html_fragment;

const EPUB_CSS: &str = r#"
body {
    font-family: Georgia, 'Times New Roman', serif;
    line-height: 1.6;
    margin: 1em;
    color: #333;
}
h1, h2, h3 { margin-top: 1.5em; }
h1 { font-size: 1.8em; }
h2 { font-size: 1.4em; }
h3 { font-size: 1.2em; }
"#;

struct Chapter {
    title: String,
    markdown: String,
}

/// Split markdown content by `# ` (h1) headers into chapters.
fn split_chapters(content: &str) -> Vec<Chapter> {
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut current_title = String::new();
    let mut current_body = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            // Flush previous chapter
            if !current_body.is_empty() || !current_title.is_empty() {
                chapters.push(Chapter {
                    title: if current_title.is_empty() {
                        "Untitled".to_string()
                    } else {
                        current_title
                    },
                    markdown: current_body,
                });
            }
            current_title = trimmed.trim_start_matches("# ").to_string();
            current_body = String::new();
        } else {
            if !current_body.is_empty() || !trimmed.is_empty() {
                current_body.push_str(line);
                current_body.push('\n');
            }
        }
    }

    // Flush last chapter
    if !current_body.is_empty() || !current_title.is_empty() {
        chapters.push(Chapter {
            title: if current_title.is_empty() {
                "Untitled".to_string()
            } else {
                current_title
            },
            markdown: current_body,
        });
    }

    // If no chapters were found, wrap everything as a single chapter
    if chapters.is_empty() {
        chapters.push(Chapter {
            title: "Content".to_string(),
            markdown: content.to_string(),
        });
    }

    chapters
}

/// Wrap an HTML fragment in a minimal XHTML document.
fn wrap_xhtml(body_html: &str, title: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\">\n\
         <head>\n\
         <title>{title}</title>\n\
         <link rel=\"stylesheet\" type=\"text/css\" href=\"stylesheet.css\" />\n\
         </head>\n\
         <body>\n\
         <h1>{title}</h1>\n\
         {body}\
         </body>\n\
         </html>",
        title = title,
        body = body_html,
    )
}

pub fn export_epub(
    content: &str,
    title: Option<&str>,
    author: Option<&str>,
    output_path: Option<&Path>,
) -> Result<()> {
    let path = output_path.ok_or_else(|| DistillError::Export {
        cause: "EPUB export requires an output path".to_string(),
    })?;

    let book_title = title.unwrap_or("Untitled");
    let book_author = author.unwrap_or("Unknown");

    let mut builder = EpubBuilder::new(ZipLibrary::new().map_err(epub_err)?)
        .map_err(epub_err)?;

    builder
        .metadata("title", book_title)
        .map_err(epub_err)?;
    builder
        .metadata("author", book_author)
        .map_err(epub_err)?;
    builder
        .stylesheet(EPUB_CSS.as_bytes())
        .map_err(epub_err)?;

    let chapters = split_chapters(content);

    for (i, chapter) in chapters.iter().enumerate() {
        let body_html = md_to_html_fragment(&chapter.markdown);
        let xhtml = wrap_xhtml(&body_html, &chapter.title);
        let filename = format!("chapter_{i}.xhtml");

        builder
            .add_content(
                EpubContent::new(&filename, xhtml.as_bytes()).title(&chapter.title),
            )
            .map_err(epub_err)?;
    }

    let output_file =
        File::create(path).map_err(|e| DistillError::Export { cause: e.to_string() })?;

    builder.generate(output_file).map_err(epub_err)?;

    Ok(())
}

fn epub_err(e: impl std::fmt::Display) -> DistillError {
    DistillError::Export {
        cause: e.to_string(),
    }
}
