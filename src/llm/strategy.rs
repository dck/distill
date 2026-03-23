use crate::cli::CompressionLevel;

pub trait CompressionStrategy: Send + Sync {
    fn distill_system(&self) -> String;
    fn distill_user(&self, content: &str) -> String;
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
You are performing a coherence refinement pass across all distilled chapters of a book.

You will receive the full set of distilled chapters. Your job:

1. Fix dangling references — if a chapter references something that was cut \
from an earlier chapter, either restore the minimal context or remove the reference.
2. Remove cross-chapter redundancy — if the same point is made in multiple \
chapters, keep the strongest version and compress or remove the others.
3. Smooth transitions between chapters where needed.
4. Ensure consistent terminology throughout.

Do NOT change the overall length significantly. Do NOT add new content. \
Do NOT re-expand distilled sections. This is a refinement pass, not a rewrite.

RESPONSE FORMAT:
<compressed>
[refined markdown here]
</compressed>";

impl CompressionStrategy for ProseStrategy {
    fn distill_system(&self) -> String {
        format!("{PROSE_SYSTEM}\n\n{}", self.policy)
    }

    fn distill_user(&self, content: &str) -> String {
        format!("CHAPTER TO DISTILL:\n\n{content}")
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
        "Extract the essence. Be brutally concise.\n\n\
         <compressed>\n\
         **[ONE sentence: what is this about and why it matters]**\n\n\
         - [key point — sentence fragment or short sentence]\n\
         - [key point]\n\
         - [key point]\n\
         </compressed>\n\n\
         Rules:\n\
         - 3-7 bullets total. That's the entire output.\n\
         - Each bullet: a sentence fragment or single short sentence. No fluff.\n\
         - Only include what someone would remember a week later.\n\
         - Skip anything obvious, generic, or introductory.\n\
         - No headers, no sections, no sub-bullets. Just the bold summary line + flat bullet list.\n\
         - You MUST wrap output in <compressed></compressed> tags."
            .into()
    }

    fn distill_user(&self, content: &str) -> String {
        format!("TEXT TO EXTRACT FROM:\n\n{content}")
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
        assert!(prompt.contains("coherence refinement"));
        assert!(prompt.contains("dangling references"));
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
        assert!(prompt.contains("brutally concise"));
        assert!(prompt.contains("3-7 bullets"));
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
