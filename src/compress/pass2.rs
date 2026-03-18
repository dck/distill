use crate::error::Result;
use crate::llm::LlmClient;
use crate::llm::parse::parse_llm_response;
use crate::llm::prompt;
use crate::state::{CompressedChunk, LedgerDelta, StateLedger};

pub async fn deduplicate(
    client: &LlmClient,
    chunks: &[CompressedChunk],
    ledger: &StateLedger,
) -> Result<Vec<CompressedChunk>> {
    let combined = chunks
        .iter()
        .map(|c| {
            let header = if c.header_path.is_empty() {
                String::new()
            } else {
                format!(
                    "<!-- chunk {} | {} -->\n",
                    c.index,
                    c.header_path.join(" > ")
                )
            };
            format!("{header}{}", c.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let system = prompt::pass2_system();
    let user = prompt::pass2_user(&combined, ledger);

    let response = client.complete(&system, &user).await?;

    match parse_llm_response(&response) {
        Ok(parsed) => {
            let deduped_chunks = reassemble_chunks(chunks, &parsed.compressed);
            Ok(deduped_chunks)
        }
        Err(_) => {
            crate::ui::warning("pass 2 (deduplication) failed to parse, keeping pass 1 output");
            Ok(chunks.to_vec())
        }
    }
}

fn reassemble_chunks(
    original_chunks: &[CompressedChunk],
    deduped_text: &str,
) -> Vec<CompressedChunk> {
    let sections: Vec<&str> = deduped_text.split("---").collect();

    if sections.len() == original_chunks.len() {
        original_chunks
            .iter()
            .enumerate()
            .map(|(i, orig)| CompressedChunk {
                index: orig.index,
                header_path: orig.header_path.clone(),
                content: sections[i].trim().to_string(),
                ledger_updates: LedgerDelta::default(),
            })
            .collect()
    } else {
        vec![CompressedChunk {
            index: 0,
            header_path: original_chunks
                .first()
                .map(|c| c.header_path.clone())
                .unwrap_or_default(),
            content: deduped_text.trim().to_string(),
            ledger_updates: LedgerDelta::default(),
        }]
    }
}
