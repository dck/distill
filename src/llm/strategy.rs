use crate::cli::CompressionLevel;

pub trait CompressionStrategy: Send + Sync {
    fn distill_system(&self) -> String;
    fn distill_user(&self, content: &str) -> String;
    /// System prompt for article mode. Defaults to distill_system() if not overridden.
    fn article_system(&self) -> String {
        self.distill_system()
    }
    fn refinement_system(&self) -> String;
    fn refinement_user(&self, combined_distilled: &str) -> String;
    fn supports_multi_pass(&self) -> bool;
}

pub struct ProseStrategy {
    policy: &'static str,
    article_system: Option<&'static str>,
}

impl ProseStrategy {
    pub fn light() -> Self {
        Self {
            policy: "TARGET: Retain ~80% of original length.\n\
                     Remove only clear filler and redundancy. Preserve original phrasing.",
            article_system: None,
        }
    }
    pub fn medium() -> Self {
        Self {
            policy: "TARGET: Retain ~50% of original length.\n\
                     Compress explanations, merge paragraphs covering the same point.",
            article_system: None,
        }
    }
    pub fn heavy() -> Self {
        Self {
            policy: "TARGET: Retain ~30% of original length.\n\
                     Aggressive compression. Keep only the strongest example per concept.",
            article_system: Some(
                "Please summarize the selection using precise and concise language. \
                 Use headers and bulleted lists in the summary, to make it scannable. \
                 Maintain the meaning and factual accuracy.\n\n\
                 RESPONSE FORMAT:\n\
                 Return your response in exactly this format:\n\
                 <compressed>\n\
                 [summarized markdown here]\n\
                 </compressed>",
            ),
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

    fn article_system(&self) -> String {
        match self.article_system {
            Some(prompt) => prompt.into(),
            None => self.distill_system(),
        }
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
        "You extract knowledge from articles into dense, self-contained markdown summaries for a personal knowledge base.\n\n\
         The output must make sense to someone who hasn't read the article and is re-reading this note weeks later.\n\n\
         Output format (inside <compressed> tags):\n\n\
         <compressed>\n\
         # Title: Concise, descriptive title of the article\n\n\
         One-sentence summary with who/what/why — enough context that the note stands alone\n\n\n\n\
         2-3 sentence paragraph explaining the core idea, mechanism, or argument. Use concrete details: names, numbers, \
         how things work. Connect the dots — don't just list facts, show why they matter or how they relate. \
         This paragraph is the main knowledge extract.\n\n\
         ### Takeaways\n\n\
         - Specific insight or fact with enough surrounding context to be understood on its own\n\
         - Another takeaway — include the \"so what\" if it's not obvious\n\
         - Non-obvious finding, counterintuitive result, or practical implication\n\
         </compressed>\n\n\
         Rules:\n\
         - Output valid markdown. Use ## for the title, ### for the takeaways header.\n\
         - Total length: 150-300 words. Dense but readable.\n\
         - The paragraph does the heavy lifting. Takeaways are for things worth remembering separately.\n\
         - 2-5 takeaways. Each must be a complete thought — no dangling references like \"the system\" or \
         \"their approach\" without saying what system or whose approach.\n\
         - BAD takeaway: \"The new architecture improves performance\" (what architecture? what performance? compared to what?)\n\
         - GOOD takeaway: \"Replacing the attention layer with a state-space model (Mamba) matches Transformer quality \
         on language tasks at 5x throughput because inference scales linearly with sequence length instead of quadratically.\"\n\
         - No sub-bullets. No bold markers like **Topic:** — just use markdown headers.\n\
         - Write in a neutral, technical tone. No filler phrases like \"interestingly\" or \"it's worth noting.\"\n\
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
        CompressionLevel::Light => Box::new(ProseStrategy::light()),
        CompressionLevel::Medium => Box::new(ProseStrategy::medium()),
        CompressionLevel::Heavy => Box::new(ProseStrategy::heavy()),
        CompressionLevel::Tldr => Box::new(TldrStrategy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_light_contains_research_prompt() {
        let strategy = ProseStrategy::light();
        let prompt = strategy.distill_system();
        assert!(prompt.contains("distilling a book chapter"));
        assert!(prompt.contains("REMOVE:"));
        assert!(prompt.contains("PRESERVE:"));
        assert!(prompt.contains("~80%"));
    }

    #[test]
    fn prose_medium_contains_policy() {
        let strategy = ProseStrategy::medium();
        let prompt = strategy.distill_system();
        assert!(prompt.contains("~50%"));
    }

    #[test]
    fn prose_refinement_prompt_exists() {
        let strategy = ProseStrategy::light();
        let prompt = strategy.refinement_system();
        assert!(prompt.contains("coherence refinement"));
        assert!(prompt.contains("dangling references"));
    }

    #[test]
    fn prose_supports_multi_pass() {
        let strategy = ProseStrategy::light();
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
        assert!(prompt.contains("personal knowledge base"));
        assert!(prompt.contains("150-300 words"));
    }

    #[test]
    fn strategy_for_returns_correct_types() {
        let medium = strategy_for(&CompressionLevel::Medium);
        assert!(medium.supports_multi_pass());

        let tldr = strategy_for(&CompressionLevel::Tldr);
        assert!(!tldr.supports_multi_pass());
    }

    #[test]
    fn distill_user_wraps_content() {
        let strategy = ProseStrategy::light();
        let prompt = strategy.distill_user("some chapter text");
        assert!(prompt.contains("some chapter text"));
    }
}
