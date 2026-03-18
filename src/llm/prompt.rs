use crate::state::StateLedger;

pub fn pass2_system() -> String {
    "You are performing global deduplication on a compressed document.\n\
     You have a rich ledger of concepts, definitions, principles, examples, anti-patterns, \
     and relationships found across all chunks.\n\n\
     Your job:\n\
     1. CONCEPTS: If the same concept appears in multiple chunks, keep the strongest explanation \
        (usually the first or most complete occurrence). In later mentions, compress to 1-2 sentences \
        with a back-reference (e.g., \"As discussed in Chapter 2, ...\").\n\
     2. DEFINITIONS: If a term is defined multiple times, keep only the best definition. \
        Remove later re-definitions entirely.\n\
     3. PRINCIPLES: If a principle/rule is restated across chapters, keep the clearest formulation. \
        Replace repetitions with brief references.\n\
     4. EXAMPLES: If multiple examples illustrate the same concept, keep only the strongest one \
        (the most vivid, concrete, or memorable). Remove or compress weaker examples.\n\
     5. ANTI-PATTERNS: If the same warning appears repeatedly, keep the first and remove later mentions.\n\
     6. RELATIONSHIPS: Use relationship data to detect when the same connection between concepts \
        is explained in different ways — keep the clearest explanation.\n\n\
     Preserve the document's logical flow. Do not remove content that introduces NEW information.\n\n\
     RESPONSE FORMAT:\n\
     <compressed>\n\
     [deduplicated markdown here]\n\
     </compressed>"
        .into()
}

pub fn pass2_user(chunks_content: &str, ledger: &StateLedger) -> String {
    let ledger_json = serde_json::to_string_pretty(ledger).unwrap_or_else(|_| "{}".into());
    format!(
        "FULL LEDGER (use this to identify duplicates across chunks):\n\
         {ledger_json}\n\n\
         CHUNKS TO DEDUPLICATE:\n\
         {chunks_content}"
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
