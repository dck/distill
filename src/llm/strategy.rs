use crate::cli::CompressionLevel;
use crate::state::StateLedger;

/// Defines how a compression level generates prompts and configures the pipeline.
pub trait CompressionStrategy: Send + Sync {
    fn single_pass_system(&self) -> String;
    fn single_pass_user(&self, content: &str) -> String;
    fn pass1_system(&self) -> String;
    fn pass1_user(&self, chunk_content: &str, chunk_index: usize, ledger: &StateLedger) -> String;
    fn supports_multi_pass(&self) -> bool;
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

/// Strategy for tight, dense, and distilled levels.
/// These share the same prompt structure, differing only in the compression policy.
pub struct ProseStrategy {
    policy: &'static str,
}

impl ProseStrategy {
    pub fn tight() -> Self {
        Self {
            policy: "COMPRESSION LEVEL: tight (~80% of original)\n\
                     - Remove repetition and filler only\n\
                     - Minimal rewriting\n\
                     - Preserve original phrasing wherever possible",
        }
    }

    pub fn dense() -> Self {
        Self {
            policy: "COMPRESSION LEVEL: dense (~50% of original)\n\
                     - Compress explanations into fewer sentences\n\
                     - Merge short paragraphs covering the same point\n\
                     - Shorten repeated mentions of the same concept",
        }
    }

    pub fn distilled() -> Self {
        Self {
            policy: "COMPRESSION LEVEL: distilled (~30% of original)\n\
                     - Aggressive compression\n\
                     - Keep only the strongest example per concept\n\
                     - Allow intra-section restructuring for clarity",
        }
    }
}

impl CompressionStrategy for ProseStrategy {
    fn single_pass_system(&self) -> String {
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
            self.policy
        )
    }

    fn single_pass_user(&self, content: &str) -> String {
        format!("TEXT TO COMPRESS:\n\n{content}")
    }

    fn pass1_system(&self) -> String {
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
            self.policy
        )
    }

    fn pass1_user(&self, chunk_content: &str, chunk_index: usize, ledger: &StateLedger) -> String {
        let ledger_json = serde_json::to_string(ledger).unwrap_or_else(|_| "{}".into());
        format!(
            "CHUNK INDEX: {chunk_index}\n\n\
             CURRENT LEDGER (all entries extracted so far — do NOT duplicate these):\n\
             {ledger_json}\n\n\
             TEXT TO COMPRESS:\n\
             {chunk_content}"
        )
    }

    fn supports_multi_pass(&self) -> bool {
        true
    }
}

/// Strategy for TLDR level: structured knowledge extraction, not prose compression.
pub struct TldrStrategy;

impl CompressionStrategy for TldrStrategy {
    fn single_pass_system(&self) -> String {
        "You are a knowledge extraction engine.\n\
         Extract the key ideas, insights, and takeaways from the text.\n\n\
         Output structured markdown in exactly this format inside <compressed> tags:\n\
         <compressed>\n\
         ## TL;DR\n\
         [1-2 sentence summary]\n\n\
         ## Key Ideas\n\
         - [idea]\n\
         - [idea]\n\n\
         ## Insights & Takeaways\n\
         - [insight]\n\n\
         ## Notable Examples\n\
         - [example] (only if genuinely memorable or illustrative)\n\
         </compressed>\n\n\
         Rules:\n\
         - Be concise. Each bullet: 1-2 sentences max.\n\
         - Key Ideas = what the text IS ABOUT (main themes, arguments, frameworks).\n\
         - Insights & Takeaways = what is SURPRISING or NON-OBVIOUS.\n\
         - If there are no notable examples worth keeping, omit that section entirely.\n\
         - Do not editorialize. Use the author's framing.\n\
         - You MUST wrap your output in <compressed></compressed> tags."
            .into()
    }

    fn single_pass_user(&self, content: &str) -> String {
        format!("TEXT TO EXTRACT FROM:\n\n{content}")
    }

    fn pass1_system(&self) -> String {
        self.single_pass_system()
    }

    fn pass1_user(
        &self,
        chunk_content: &str,
        _chunk_index: usize,
        _ledger: &StateLedger,
    ) -> String {
        self.single_pass_user(chunk_content)
    }

    fn supports_multi_pass(&self) -> bool {
        false
    }
}

/// Create the right strategy for a given compression level.
pub fn strategy_for(level: &CompressionLevel) -> Box<dyn CompressionStrategy> {
    match level {
        CompressionLevel::Tight => Box::new(ProseStrategy::tight()),
        CompressionLevel::Dense => Box::new(ProseStrategy::dense()),
        CompressionLevel::Distilled => Box::new(ProseStrategy::distilled()),
        CompressionLevel::Tldr => Box::new(TldrStrategy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateLedger;

    #[test]
    fn prose_tight_single_pass_contains_policy() {
        let strategy = ProseStrategy::tight();
        let prompt = strategy.single_pass_system();
        assert!(prompt.contains("~80% of original"));
        assert!(prompt.contains("compression engine"));
    }

    #[test]
    fn prose_dense_pass1_contains_policy_and_ledger() {
        let strategy = ProseStrategy::dense();
        let prompt = strategy.pass1_system();
        assert!(prompt.contains("~50% of original"));
        assert!(prompt.contains("Ledger entry formats"));
    }

    #[test]
    fn prose_distilled_supports_multi_pass() {
        let strategy = ProseStrategy::distilled();
        assert!(strategy.supports_multi_pass());
    }

    #[test]
    fn prose_pass1_user_includes_chunk_and_ledger() {
        let strategy = ProseStrategy::tight();
        let ledger = StateLedger::default();
        let prompt = strategy.pass1_user("chunk content", 3, &ledger);
        assert!(prompt.contains("CHUNK INDEX: 3"));
        assert!(prompt.contains("chunk content"));
    }

    #[test]
    fn prose_single_pass_user_wraps_content() {
        let strategy = ProseStrategy::tight();
        let prompt = strategy.single_pass_user("article text");
        assert!(prompt.contains("article text"));
    }

    #[test]
    fn tldr_does_not_support_multi_pass() {
        let strategy = TldrStrategy;
        assert!(!strategy.supports_multi_pass());
    }

    #[test]
    fn tldr_single_pass_system_is_extraction_prompt() {
        let strategy = TldrStrategy;
        let prompt = strategy.single_pass_system();
        assert!(prompt.contains("knowledge extraction"));
        assert!(prompt.contains("Key Ideas"));
        assert!(prompt.contains("Insights"));
        assert!(!prompt.contains("compression engine"));
    }

    #[test]
    fn tldr_single_pass_user_wraps_content() {
        let strategy = TldrStrategy;
        let prompt = strategy.single_pass_user("some article text");
        assert!(prompt.contains("some article text"));
    }

    #[test]
    fn strategy_for_returns_correct_types() {
        let tight = strategy_for(&CompressionLevel::Tight);
        assert!(tight.supports_multi_pass());

        let tldr = strategy_for(&CompressionLevel::Tldr);
        assert!(!tldr.supports_multi_pass());
    }
}
