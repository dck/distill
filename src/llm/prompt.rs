use crate::cli::CompressionLevel;
use crate::state::StateLedger;

pub fn compression_level_policy(level: &CompressionLevel) -> &'static str {
    match level {
        CompressionLevel::Tight => {
            "COMPRESSION LEVEL: tight (~80% of original)\n\
             - Remove repetition and filler only\n\
             - Minimal rewriting\n\
             - Preserve original phrasing wherever possible"
        }
        CompressionLevel::Dense => {
            "COMPRESSION LEVEL: dense (~50% of original)\n\
             - Compress explanations into fewer sentences\n\
             - Merge short paragraphs covering the same point\n\
             - Shorten repeated mentions of the same concept"
        }
        CompressionLevel::Distilled => {
            "COMPRESSION LEVEL: distilled (~30% of original)\n\
             - Aggressive compression\n\
             - Keep only the strongest example per concept\n\
             - Allow intra-section restructuring for clarity"
        }
    }
}

pub fn pass1_system(level: &CompressionLevel) -> String {
    format!(
        "You are a structure-preserving semantic compression engine.\n\
         You compress text while preserving structure, core ideas, meaningful examples, and the author's voice.\n\
         Remove: repetition, filler, long transitions, meta-text.\n\n\
         {}\n\n\
         RESPONSE FORMAT:\n\
         Return your response in exactly this format:\n\
         <compressed>\n\
         [compressed markdown here]\n\
         </compressed>\n\
         <ledger>\n\
         {{\"new_concepts\": [...], \"new_examples\": [...]}}\n\
         </ledger>\n\n\
         Ledger entry format:\n\
         - Concept: {{\"id\": \"concept-NNN\", \"name\": \"...\", \"first_seen_chunk\": N, \"description\": \"...\"}}\n\
         - Example: {{\"id\": \"example-NNN\", \"related_concept\": \"concept-NNN\", \"first_seen_chunk\": N, \"summary\": \"...\"}}\n\n\
         If no new concepts or examples, return empty arrays.",
        compression_level_policy(level)
    )
}

pub fn pass1_user(chunk_content: &str, chunk_index: usize, ledger: &StateLedger) -> String {
    let ledger_json = serde_json::to_string(ledger).unwrap_or_else(|_| "{}".into());
    format!(
        "CHUNK INDEX: {chunk_index}\n\n\
         CURRENT LEDGER (concepts/examples seen so far):\n\
         {ledger_json}\n\n\
         TEXT TO COMPRESS:\n\
         {chunk_content}"
    )
}

pub fn single_pass_system(level: &CompressionLevel) -> String {
    format!(
        "You are a structure-preserving semantic compression engine.\n\
         You compress text while preserving structure, core ideas, meaningful examples, and the author's voice.\n\
         Remove: repetition, filler, long transitions, meta-text.\n\n\
         {}\n\n\
         RESPONSE FORMAT:\n\
         Return your response in exactly this format:\n\
         <compressed>\n\
         [compressed markdown here]\n\
         </compressed>",
        compression_level_policy(level)
    )
}

pub fn single_pass_user(content: &str) -> String {
    format!("TEXT TO COMPRESS:\n\n{content}")
}

pub fn pass2_system() -> String {
    "You are performing global deduplication on a compressed document.\n\
     You have a ledger of all concepts and examples found across chunks.\n\
     Your job:\n\
     1. Identify concepts/examples that appear in multiple chunks\n\
     2. Keep the strongest, most complete version (usually first occurrence)\n\
     3. In later occurrences, compress to 1-2 sentences with a back-reference\n\n\
     RESPONSE FORMAT:\n\
     <compressed>\n\
     [deduplicated markdown here]\n\
     </compressed>"
        .into()
}

pub fn pass2_user(chunks_content: &str, ledger: &StateLedger) -> String {
    let ledger_json = serde_json::to_string_pretty(ledger).unwrap_or_else(|_| "{}".into());
    format!(
        "FULL LEDGER:\n{ledger_json}\n\n\
         CHUNKS TO DEDUPLICATE:\n{chunks_content}"
    )
}

pub fn pass3_system() -> String {
    "You are performing a final refinement pass on a compressed document.\n\
     Fix broken transitions between sections (artifacts of chunk boundaries).\n\
     Smooth tone to match the original author's voice.\n\
     Ensure no dangling references to removed content.\n\
     Do NOT add new content or re-expand compressed sections.\n\n\
     RESPONSE FORMAT:\n\
     <compressed>\n\
     [refined markdown here]\n\
     </compressed>"
        .into()
}

pub fn pass3_user(content: &str) -> String {
    format!("TEXT TO REFINE:\n\n{content}")
}
