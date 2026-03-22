use crate::error::DistillError;

#[derive(Debug)]
pub struct ParsedResponse {
    pub compressed: String,
}

pub fn parse_llm_response(response: &str) -> crate::error::Result<ParsedResponse> {
    let compressed = extract_tag(response, "compressed").ok_or_else(|| {
        DistillError::Compression {
            chunk_index: 0,
            section: String::new(),
            cause: "missing <compressed> tag in LLM response".into(),
        }
    })?;
    Ok(ParsedResponse { compressed })
}

fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)?;
    let end = text.find(&close)?;
    if end <= start {
        return None;
    }
    let content = &text[start + open.len()..end];
    Some(content.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_well_formed_response() {
        let response = r#"<compressed>
## Section Title

Compressed content here.
</compressed>"#;

        let parsed = parse_llm_response(response).unwrap();
        assert!(parsed.compressed.contains("Compressed content"));
    }

    #[test]
    fn test_parse_missing_compressed_tag() {
        let response = "Just some text without tags";
        let result = parse_llm_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_compressed() {
        let response = "<compressed></compressed>";
        let parsed = parse_llm_response(response).unwrap();
        assert_eq!(parsed.compressed, "");
    }

    #[test]
    fn test_parse_single_pass_response() {
        let response = "<compressed>\n# Title\n\nCompressed article.\n</compressed>";
        let parsed = parse_llm_response(response).unwrap();
        assert!(parsed.compressed.contains("Compressed article"));
    }
}
