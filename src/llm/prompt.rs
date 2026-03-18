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
        CompressionLevel::Tldr => {
            "COMPRESSION LEVEL: tldr\n\
             - Extract key ideas only\n\
             - Structured output"
        }
    }
}

const LEDGER_SCHEMA: &str = "\
Ledger entry formats (return only entries NEW to this chunk):

- Concept: {\"id\": \"concept-NNN\", \"name\": \"...\", \"first_seen_chunk\": N, \"description\": \"...\"}
  A major idea, topic, or theme the author introduces.

- Definition: {\"id\": \"def-NNN\", \"term\": \"...\", \"meaning\": \"...\", \"first_seen_chunk\": N}
  A specific term the author explicitly defines.

- Principle: {\"id\": \"prin-NNN\", \"name\": \"...\", \"statement\": \"...\", \"related_concept\": \"concept-NNN\", \"first_seen_chunk\": N}
  A rule, guideline, law, or framework the author advocates.

- Example: {\"id\": \"ex-NNN\", \"related_concept\": \"concept-NNN\", \"first_seen_chunk\": N, \"summary\": \"...\"}
  A concrete illustration, case study, anecdote, or story.

- AntiPattern: {\"id\": \"anti-NNN\", \"name\": \"...\", \"description\": \"...\", \"related_concept\": \"concept-NNN\", \"first_seen_chunk\": N}
  A common mistake, pitfall, or \"what NOT to do\" warning.

- Relationship: {\"id\": \"rel-NNN\", \"from_concept\": \"concept-NNN\", \"to_concept\": \"concept-NNN\", \"relation_type\": \"causes|enables|contradicts|extends|requires\", \"first_seen_chunk\": N}
  How two concepts connect to each other.

If no new entries for a category, return an empty array for that field.";

pub fn pass1_system(level: &CompressionLevel) -> String {
    format!(
        "You are a structure-preserving semantic compression engine.\n\
         You compress text while preserving structure, core ideas, meaningful examples, and the author's voice.\n\
         Remove: repetition, filler, long transitions, meta-text (\"we will cover later\", \"as mentioned previously\").\n\n\
         {}\n\n\
         RESPONSE FORMAT:\n\
         Return your response in exactly this format:\n\
         <compressed>\n\
         [compressed markdown here]\n\
         </compressed>\n\
         <ledger>\n\
         {{\n\
           \"new_concepts\": [...],\n\
           \"new_definitions\": [...],\n\
           \"new_principles\": [...],\n\
           \"new_examples\": [...],\n\
           \"new_anti_patterns\": [...],\n\
           \"new_relationships\": [...]\n\
         }}\n\
         </ledger>\n\n\
         {LEDGER_SCHEMA}\n\n\
         IMPORTANT:\n\
         - Only extract entries genuinely present in the text. Do not invent.\n\
         - Check the current ledger below — do NOT re-extract entries already listed.\n\
         - Use concept IDs from the existing ledger when referencing known concepts.",
        compression_level_policy(level)
    )
}

pub fn pass1_user(chunk_content: &str, chunk_index: usize, ledger: &StateLedger) -> String {
    let ledger_json = serde_json::to_string(ledger).unwrap_or_else(|_| "{}".into());
    format!(
        "CHUNK INDEX: {chunk_index}\n\n\
         CURRENT LEDGER (all entries extracted so far — do NOT duplicate these):\n\
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
