use crate::cli::CompressionLevel;

pub trait CompressionStrategy: Send + Sync {
    fn distill_system(&self) -> String;
    fn distill_user(&self, content: &str) -> String;
    fn dedup_system(&self) -> String;
    fn dedup_user(&self, combined_distilled: &str) -> String;
    fn refinement_system(&self) -> String;
    fn refinement_user(&self, combined_distilled: &str) -> String;
    fn supports_multi_pass(&self) -> bool;
}

pub struct ProseStrategy {
    policy: &'static str,
}

impl ProseStrategy {
    pub fn tight() -> Self {
        Self {
            policy: "TARGET: Retain ~80% of original length.\n\
                     Remove only clear filler and redundancy. Preserve original phrasing.",
        }
    }
    pub fn dense() -> Self {
        Self {
            policy: "TARGET: Retain ~50% of original length.\n\
                     Compress explanations, merge paragraphs covering the same point.",
        }
    }
    pub fn distilled() -> Self {
        Self {
            policy: "TARGET: Retain ~30% of original length.\n\
                     Aggressive compression. Keep only the strongest example per concept.",
        }
    }
}

const PROSE_SYSTEM: &str = "\
You are distilling a book chapter. Your goal is to remove low-value content \
while preserving the intellectual substance.

REMOVE: filler phrases, redundant restatements of the same idea, excessive \
anecdotes that repeat a point already made, verbose introductions, motivational \
padding, unnecessary transitions, rhetorical questions that add no information.

PRESERVE: key arguments, frameworks, concrete examples (with names and data), \
research citations, actionable advice, important quotes, definitions, \
cause-effect relationships.

This should read like the same book chapter, just shorter — NOT a summary.
Maintain the author's voice and writing style.
Output as markdown with appropriate headings and sub-sections.

RESPONSE FORMAT:
Return your response in exactly this format:
<compressed>
[distilled markdown here]
</compressed>";

const REFINEMENT_SYSTEM: &str = "\
You are performing a final coherence refinement pass across all distilled chapters of a book.

You will receive text that has already been globally deduplicated. Your job:

1. Fix dangling references — if a chapter references something that was cut \
from an earlier chapter, either restore the minimal context or remove the reference.
2. Smooth transitions between chapters where needed.
3. Ensure consistent terminology throughout.
4. Preserve the author's voice and markdown structure.

Do NOT change the overall length significantly. Do NOT add new content. \
Do NOT re-expand distilled sections. This is a refinement pass, not a rewrite.

RESPONSE FORMAT:
<compressed>
[refined markdown here]
</compressed>";

const DEDUP_SYSTEM: &str = "\
You are performing a global deduplication pass across distilled chapters of a book.

You will receive the full set of distilled chapters. Your job:

1. Remove cross-chapter redundancy — if the same point is made in multiple \
chapters, keep the strongest version and compress or remove the others.
2. Preserve chapter ordering and markdown headings.
3. Keep only one strong explanation per repeated concept unless repetition is \
needed for local clarity.
4. Do not add new content or editorial commentary.

This is a deduplication pass, not a rewrite.

RESPONSE FORMAT:
<compressed>
[deduplicated markdown here]
</compressed>";

impl CompressionStrategy for ProseStrategy {
    fn distill_system(&self) -> String {
        format!("{PROSE_SYSTEM}\n\n{}", self.policy)
    }

    fn distill_user(&self, content: &str) -> String {
        format!("CHAPTER TO DISTILL:\n\n{content}")
    }

    fn dedup_system(&self) -> String {
        DEDUP_SYSTEM.into()
    }

    fn dedup_user(&self, combined_distilled: &str) -> String {
        format!("DISTILLED CHAPTERS TO DEDUPLICATE:\n\n{combined_distilled}")
    }

    fn refinement_system(&self) -> String {
        REFINEMENT_SYSTEM.into()
    }

    fn refinement_user(&self, combined_distilled: &str) -> String {
        format!("DISTILLED CHAPTERS TO REFINE:\n\n{combined_distilled}")
    }

    fn supports_multi_pass(&self) -> bool {
        true
    }
}

/// Strategy for TLDR level: structured knowledge extraction, not prose compression.
pub struct TldrStrategy;

impl CompressionStrategy for TldrStrategy {
    fn distill_system(&self) -> String {
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

    fn distill_user(&self, content: &str) -> String {
        format!("TEXT TO EXTRACT FROM:\n\n{content}")
    }

    fn dedup_system(&self) -> String {
        self.distill_system()
    }

    fn dedup_user(&self, combined_distilled: &str) -> String {
        self.distill_user(combined_distilled)
    }

    fn refinement_system(&self) -> String {
        self.distill_system()
    }

    fn refinement_user(&self, combined_distilled: &str) -> String {
        self.distill_user(combined_distilled)
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

    #[test]
    fn prose_tight_contains_research_prompt() {
        let strategy = ProseStrategy::tight();
        let prompt = strategy.distill_system();
        assert!(prompt.contains("distilling a book chapter"));
        assert!(prompt.contains("REMOVE:"));
        assert!(prompt.contains("PRESERVE:"));
        assert!(prompt.contains("~80%"));
    }

    #[test]
    fn prose_dense_contains_policy() {
        let strategy = ProseStrategy::dense();
        let prompt = strategy.distill_system();
        assert!(prompt.contains("~50%"));
    }

    #[test]
    fn prose_refinement_prompt_exists() {
        let strategy = ProseStrategy::tight();
        let prompt = strategy.refinement_system();
        assert!(prompt.contains("final coherence refinement"));
        assert!(prompt.contains("dangling references"));
    }

    #[test]
    fn prose_dedup_prompt_exists() {
        let strategy = ProseStrategy::tight();
        let prompt = strategy.dedup_system();
        assert!(prompt.contains("global deduplication"));
        assert!(prompt.contains("cross-chapter redundancy"));
    }

    #[test]
    fn prose_supports_multi_pass() {
        let strategy = ProseStrategy::tight();
        assert!(strategy.supports_multi_pass());
    }

    #[test]
    fn tldr_does_not_support_multi_pass() {
        let strategy = TldrStrategy;
        assert!(!strategy.supports_multi_pass());
    }

    #[test]
    fn tldr_is_extraction_prompt() {
        let strategy = TldrStrategy;
        let prompt = strategy.distill_system();
        assert!(prompt.contains("knowledge extraction"));
        assert!(prompt.contains("Key Ideas"));
    }

    #[test]
    fn strategy_for_returns_correct_types() {
        let dense = strategy_for(&CompressionLevel::Dense);
        assert!(dense.supports_multi_pass());

        let tldr = strategy_for(&CompressionLevel::Tldr);
        assert!(!tldr.supports_multi_pass());
    }

    #[test]
    fn distill_user_wraps_content() {
        let strategy = ProseStrategy::tight();
        let prompt = strategy.distill_user("some chapter text");
        assert!(prompt.contains("some chapter text"));
    }
}
