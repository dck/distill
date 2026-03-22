use std::path::Path;

use crate::error::{DistillError, Result};

const CSS: &str = r#"
body {
    font-family: Georgia, 'Times New Roman', serif;
    line-height: 1.6;
    max-width: 42em;
    margin: 2em auto;
    padding: 0 1em;
    color: #333;
}
h1, h2, h3 { margin-top: 1.5em; }
h1 { font-size: 1.8em; }
h2 { font-size: 1.4em; }
h3 { font-size: 1.2em; }
nav#toc { margin-bottom: 2em; }
nav#toc ul { list-style: none; padding-left: 0; }
nav#toc li { margin: 0.3em 0; }
"#;

/// Convert markdown text to an HTML fragment (no document wrapper).
///
/// Handles: `#`/`##`/`###` headers, paragraphs, blank-line separation.
/// Headers get id attributes derived from their text for internal linking.
pub fn md_to_html_fragment(md: &str) -> String {
    let mut out = String::new();
    let mut in_paragraph = false;

    for line in md.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_paragraph {
                out.push_str("</p>\n");
                in_paragraph = false;
            }
            continue;
        }

        if let Some(header) = parse_header(trimmed) {
            if in_paragraph {
                out.push_str("</p>\n");
                in_paragraph = false;
            }
            let id = slugify(&header.text);
            out.push_str(&format!(
                "<h{level} id=\"{id}\">{text}</h{level}>\n",
                level = header.level,
                id = id,
                text = escape_html(&header.text),
            ));
        } else {
            if !in_paragraph {
                out.push_str("<p>");
                in_paragraph = true;
            } else {
                out.push('\n');
            }
            out.push_str(&escape_html(trimmed));
        }
    }

    if in_paragraph {
        out.push_str("</p>\n");
    }

    out
}

struct Header {
    level: u8,
    text: String,
}

fn parse_header(line: &str) -> Option<Header> {
    let trimmed = line.trim_start();
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 3 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some(Header {
        level: hashes as u8,
        text: rest.trim().to_string(),
    })
}

fn slugify(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Extract `##` headers from markdown for TOC generation.
fn extract_toc_entries(md: &str) -> Vec<(String, String)> {
    md.lines()
        .filter_map(|line| {
            let header = parse_header(line.trim())?;
            if header.level == 2 {
                let slug = slugify(&header.text);
                Some((slug, header.text))
            } else {
                None
            }
        })
        .collect()
}

fn build_toc_html(entries: &[(String, String)]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut html = String::from("<nav id=\"toc\">\n<h2>Contents</h2>\n<ul>\n");
    for (slug, title) in entries {
        html.push_str(&format!(
            "  <li><a href=\"#{slug}\">{title}</a></li>\n",
            slug = slug,
            title = escape_html(title),
        ));
    }
    html.push_str("</ul>\n</nav>\n");
    html
}

fn build_full_html(content: &str, title: Option<&str>) -> String {
    let title_text = title.unwrap_or("Distill Export");
    let toc_entries = extract_toc_entries(content);
    let toc_html = build_toc_html(&toc_entries);
    let body_html = md_to_html_fragment(content);

    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n\
         <style>{css}</style>\n\
         </head>\n\
         <body>\n\
         {toc}\
         {body}\
         </body>\n\
         </html>\n",
        title = escape_html(title_text),
        css = CSS,
        toc = toc_html,
        body = body_html,
    )
}

pub fn export_html(content: &str, title: Option<&str>, output_path: Option<&Path>) -> Result<()> {
    let html = build_full_html(content, title);

    match output_path {
        Some(path) => {
            std::fs::write(path, &html).map_err(|e| DistillError::Export {
                cause: e.to_string(),
            })?;
        }
        None => {
            print!("{html}");
        }
    }
    Ok(())
}
