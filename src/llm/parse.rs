use crate::state::LedgerDelta;

#[derive(Debug)]
pub struct ParsedResponse {
    pub compressed: String,
    pub ledger: LedgerDelta,
}

pub fn parse_llm_response(response: &str) -> crate::error::Result<ParsedResponse> {
    let compressed = extract_tag(response, "compressed").ok_or_else(|| {
        crate::error::DistillError::Compression {
            chunk_index: 0,
            section: String::new(),
            cause: "missing <compressed> tag in LLM response".into(),
        }
    })?;

    let ledger = extract_tag(response, "ledger")
        .and_then(|json| serde_json::from_str::<LedgerDelta>(&json).ok())
        .unwrap_or_default();

    Ok(ParsedResponse { compressed, ledger })
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
</compressed>
<ledger>
{"new_concepts": [{"id": "concept-001", "name": "Test Concept", "first_seen_chunk": 0, "description": "A test"}], "new_examples": []}
</ledger>"#;

        let parsed = parse_llm_response(response).unwrap();
        assert!(parsed.compressed.contains("Compressed content"));
        assert_eq!(parsed.ledger.new_concepts.len(), 1);
        assert_eq!(parsed.ledger.new_concepts[0].name, "Test Concept");
    }

    #[test]
    fn test_parse_missing_compressed_tag() {
        let response = "Just some text without tags";
        let result = parse_llm_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_ledger_tag() {
        let response = "<compressed>Some text</compressed>";
        let parsed = parse_llm_response(response).unwrap();
        assert_eq!(parsed.compressed, "Some text");
        assert!(parsed.ledger.new_concepts.is_empty());
    }

    #[test]
    fn test_parse_empty_compressed() {
        let response = "<compressed></compressed>\n<ledger>{\"new_concepts\": [], \"new_examples\": []}</ledger>";
        let parsed = parse_llm_response(response).unwrap();
        assert_eq!(parsed.compressed, "");
    }

    #[test]
    fn test_parse_malformed_ledger_json() {
        let response = "<compressed>Good text</compressed>\n<ledger>not valid json</ledger>";
        let parsed = parse_llm_response(response).unwrap();
        assert_eq!(parsed.compressed, "Good text");
        assert!(parsed.ledger.new_concepts.is_empty());
    }

    #[test]
    fn test_parse_single_pass_response() {
        let response = "<compressed>\n# Title\n\nCompressed article.\n</compressed>";
        let parsed = parse_llm_response(response).unwrap();
        assert!(parsed.compressed.contains("Compressed article"));
    }
}
