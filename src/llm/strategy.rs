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

pub struct LevelStrategy {
    article: &'static str,
    book_distill: &'static str,
    book_refine: &'static str,
    user_prefix: &'static str,
    multi_pass: bool,
}

// ---------------------------------------------------------------------------
// Article prompts
// ---------------------------------------------------------------------------

const ARTICLE_LIGHT: &str = "\
You are distilling an article. Remove low-value content while keeping the \
substance intact.

REMOVE: filler phrases, redundant restatements, verbose transitions, rhetorical \
questions that add no information, unnecessary qualifications and hedging.

PRESERVE: all arguments, evidence, examples, data, quotes, definitions, \
cause-effect relationships. Keep the author's voice and the original structure.

This should read like the same article, just tighter — NOT a summary.
Output as markdown. Preserve any existing headings and sub-sections.

TARGET: Retain ~80% of original length.

RESPONSE FORMAT:
Return your response in exactly this format:
<compressed>
[distilled markdown here]
</compressed>";

const ARTICLE_MEDIUM: &str = "\
You are distilling an article. Compress it significantly while preserving \
core content.

REMOVE: filler phrases, redundant restatements, verbose transitions, weak \
examples that repeat a point already made, excessive background context, \
rhetorical padding, unnecessary qualifications.

PRESERVE: key arguments, strongest examples, data and evidence, frameworks, \
actionable insights, important quotes, definitions.

Merge paragraphs that cover the same point. Compress explanations to their \
essence. This should read like a tighter version of the same article — NOT \
a summary.
Output as markdown. Preserve headings and overall structure.

TARGET: Retain ~50% of original length.

RESPONSE FORMAT:
Return your response in exactly this format:
<compressed>
[distilled markdown here]
</compressed>";

const ARTICLE_HEAVY: &str = "\
You are creating an annotated outline of an article. Follow the article's \
structure but compress each section into a brief summary of its key idea.

For each section or major idea in the article:
- Use the original heading (or create a short one if missing)
- Write 1-3 sentences summarizing the key point
- Include concrete details: names, numbers, conclusions — not vague descriptions

End with a \"Takeaways\" section: a flat bullet list of the most important \
insights from the entire article.

Do NOT write long paragraphs. Every section should be scannable.
Output as markdown with ## headers for each section and ### for Takeaways.

RESPONSE FORMAT:
Return your response in exactly this format:
<compressed>
[annotated outline here]
</compressed>";

const ARTICLE_TLDR: &str = "\
Extract a brief card from this article.

Output format (inside <compressed> tags):

<compressed>
## [Descriptive title]

[One sentence: what this article is about — enough context to stand alone]

### Key Points

- [Specific insight with concrete detail]
- [Another key point — include the \"so what\"]
- [Non-obvious finding or practical implication]
</compressed>

Rules:
- One-sentence description, then 3-7 bullet points. No paragraphs beyond the \
one sentence.
- Total: 50-150 words. Dense, not verbose.
- Each bullet is a complete, self-contained thought.
- BAD: \"The new approach improves performance\" (what approach? what performance?)
- GOOD: \"Replacing attention with state-space models (Mamba) matches Transformer \
quality at 5x throughput because inference scales linearly instead of quadratically.\"
- No filler. No sub-bullets. No bold **Topic:** markers.
- You MUST wrap output in <compressed></compressed> tags.";

// ---------------------------------------------------------------------------
// Book prompts — pass 1 (distill)
// ---------------------------------------------------------------------------

const BOOK_LIGHT: &str = "\
You are distilling a book chapter into a shorter version that reads like the \
same chapter — same voice, same narrative flow, just less padding.

REMOVE: filler phrases, redundant restatements of the same idea, motivational \
padding that doesn't carry new information, verbose introductions, unnecessary \
transitions, rhetorical questions that add nothing.

PRESERVE: key arguments, frameworks, concrete examples (with names and data), \
research citations, actionable advice, important quotes, definitions, \
cause-effect relationships, narrative flow.

CRITICAL FORMAT RULES:
- Output flowing prose paragraphs. Maintain the author's writing style.
- Do NOT use bullet points or numbered lists unless the original chapter does.
- Do NOT convert prose into outlines or summaries.
- Use markdown headings that mirror the original chapter structure.

TARGET: Retain ~80% of original length.

RESPONSE FORMAT:
Return your response in exactly this format:
<compressed>
[distilled markdown here]
</compressed>";

const BOOK_MEDIUM: &str = "\
You are distilling a book chapter into a denser version that still reads as \
a coherent chapter — same voice, same ideas, fewer words.

REMOVE: filler phrases, redundant restatements, excessive anecdotes that repeat \
a point already made, verbose introductions, motivational padding, unnecessary \
transitions, rhetorical questions, weak examples when a stronger one already \
makes the same point.

PRESERVE: key arguments, frameworks, the strongest example per concept, research \
citations, actionable advice, important quotes, definitions, cause-effect \
relationships.

Merge paragraphs that cover the same point into one tighter paragraph. Compress \
explanations to their essence while keeping them readable.

CRITICAL FORMAT RULES:
- Output flowing prose paragraphs. Maintain the author's voice and style.
- Do NOT use bullet points or numbered lists unless the original chapter does.
- Do NOT convert prose into outlines, lists, or note-style summaries.
- The result must read like a shorter version of the same book, not study notes.
- Use markdown headings that mirror the original chapter structure.

TARGET: Retain ~50% of original length.

RESPONSE FORMAT:
Return your response in exactly this format:
<compressed>
[distilled markdown here]
</compressed>";

const BOOK_HEAVY: &str = "\
You are distilling a book chapter into a short, aggressively compressed version \
that is still readable prose — a chapter you could read in bed, not an outline \
or a set of notes.

Keep only the core argument of each section and one concrete example per concept. \
Drop all anecdotes unless they carry a unique insight not stated elsewhere. \
Compress multi-paragraph explanations into single tight paragraphs.

CRITICAL FORMAT RULES:
- Output MUST be flowing prose paragraphs. This is the most important rule.
- Do NOT use bullet points, numbered lists, or outline format.
- Do NOT write in note-taking style (\"Key idea: ...\", \"Example: ...\").
- If you feel tempted to list points, write them as connected sentences instead.
- The result must still read like a book chapter, just a very short one.
- Use markdown headings that mirror the original structure.

TARGET: Retain ~30% of original length.

RESPONSE FORMAT:
Return your response in exactly this format:
<compressed>
[distilled markdown here]
</compressed>";

const BOOK_TLDR: &str = "\
You extract the key takeaways from a book into a dense, structured reference \
document — like the book-notes repositories people publish on GitHub after \
reading a book.

The reader has NOT read the book. This document replaces reading it.

Output format (inside <compressed> tags):

<compressed>
## [Book Title]

[2-3 sentences: what this book is about, its core thesis, and why it matters]

### [Theme 1 — identified by you, not by chapter]

- Takeaway with enough context to stand alone
- Another takeaway — include the \"so what\"

### [Theme 2]

- ...

### [Theme 3]

- ...
</compressed>

Rules:
- Group takeaways by THEME, not by chapter. Identify 3-6 themes that capture \
the book's most important ideas.
- Each theme gets a ### heading and 2-5 bullet takeaways.
- Each bullet must be a complete, self-contained thought — no dangling references \
like \"their framework\" without saying whose framework or what it contains.
- Total length: 1-3 pages (roughly 500-1500 words).
- Include concrete details: names, numbers, frameworks, mechanisms.
- No filler phrases. No sub-bullets. No bold **Topic:** markers within bullets.
- You MUST wrap output in <compressed></compressed> tags.";

// ---------------------------------------------------------------------------
// Book prompts — pass 2 (refinement)
// ---------------------------------------------------------------------------

const REFINE_LIGHT: &str = "\
You are performing a coherence refinement pass across all distilled chapters \
of a book.

You will receive the full set of distilled chapters. Your job:

1. Fix dangling references — if a chapter references something that was cut \
from an earlier chapter, either restore the minimal context or remove the reference.
2. Remove cross-chapter redundancy — if the same point is made in multiple \
chapters, keep the strongest version and compress or remove the others.
3. Smooth transitions between chapters where needed.
4. Ensure consistent terminology throughout.
5. Verify the output reads as continuous prose — no bullet-point lists or \
outline fragments should remain.

Do NOT change the overall length significantly. Do NOT add new content. \
Do NOT re-expand distilled sections. This is a refinement pass, not a rewrite.

RESPONSE FORMAT:
<compressed>
[refined markdown here]
</compressed>";

const REFINE_MEDIUM: &str = "\
You are performing a coherence refinement pass across all distilled chapters \
of a book.

You will receive the full set of distilled chapters. Your job:

1. Fix dangling references — if a chapter references something that was cut \
from an earlier chapter, either restore the minimal context or remove the reference.
2. Remove cross-chapter redundancy — if the same point is made in multiple \
chapters, keep the strongest version and compress or remove the others.
3. Smooth transitions between chapters where needed.
4. Ensure consistent terminology throughout.
5. Check that compression has not made sections too terse to read naturally. \
If a paragraph has become a choppy sequence of disconnected sentences, smooth \
it into flowing prose.
6. The output must read like a shorter book, not like notes. No bullet-point \
lists or outline fragments.

Do NOT change the overall length significantly. Do NOT add new content. \
This is a refinement pass, not a rewrite.

RESPONSE FORMAT:
<compressed>
[refined markdown here]
</compressed>";

const REFINE_HEAVY: &str = "\
You are performing an aggressive coherence refinement pass across all distilled \
chapters of a book. The chapters have been heavily compressed and may have \
degraded into bullet points, outlines, or choppy fragments.

Your job — in order of priority:

1. ENFORCE PROSE — this is the most important task. If any section has collapsed \
into bullet points, numbered lists, outline format, or note-taking style \
(\"Key idea: ...\"), REWRITE those sections as flowing prose paragraphs. The \
result must read like a short book, not a collection of notes.
2. Fix dangling references — restore minimal context or remove the reference.
3. Remove cross-chapter redundancy.
4. Smooth transitions so chapters connect naturally.
5. Ensure consistent terminology throughout.

The result should be something you could hand to someone and say \"here, read \
this short version of the book.\" Every section must be readable prose.

Do NOT significantly expand the total length. Do NOT add new ideas. \
Convert format, don't add content.

RESPONSE FORMAT:
<compressed>
[refined markdown here]
</compressed>";

// ---------------------------------------------------------------------------
// Strategy constructors
// ---------------------------------------------------------------------------

impl LevelStrategy {
    pub fn light() -> Self {
        Self {
            article: ARTICLE_LIGHT,
            book_distill: BOOK_LIGHT,
            book_refine: REFINE_LIGHT,
            user_prefix: "CONTENT TO DISTILL:",
            multi_pass: true,
        }
    }

    pub fn medium() -> Self {
        Self {
            article: ARTICLE_MEDIUM,
            book_distill: BOOK_MEDIUM,
            book_refine: REFINE_MEDIUM,
            user_prefix: "CONTENT TO DISTILL:",
            multi_pass: true,
        }
    }

    pub fn heavy() -> Self {
        Self {
            article: ARTICLE_HEAVY,
            book_distill: BOOK_HEAVY,
            book_refine: REFINE_HEAVY,
            user_prefix: "CONTENT TO DISTILL:",
            multi_pass: true,
        }
    }

    pub fn tldr() -> Self {
        Self {
            article: ARTICLE_TLDR,
            book_distill: BOOK_TLDR,
            book_refine: "",
            user_prefix: "TEXT TO EXTRACT FROM:",
            multi_pass: false,
        }
    }
}

impl CompressionStrategy for LevelStrategy {
    fn distill_system(&self) -> String {
        self.book_distill.into()
    }

    fn distill_user(&self, content: &str) -> String {
        format!("{}\n\n{content}", self.user_prefix)
    }

    fn article_system(&self) -> String {
        self.article.into()
    }

    fn refinement_system(&self) -> String {
        self.book_refine.into()
    }

    fn refinement_user(&self, combined_distilled: &str) -> String {
        format!("DISTILLED CHAPTERS TO REFINE:\n\n{combined_distilled}")
    }

    fn supports_multi_pass(&self) -> bool {
        self.multi_pass
    }
}

/// Create the right strategy for a given compression level.
pub fn strategy_for(level: &CompressionLevel) -> Box<dyn CompressionStrategy> {
    match level {
        CompressionLevel::Light => Box::new(LevelStrategy::light()),
        CompressionLevel::Medium => Box::new(LevelStrategy::medium()),
        CompressionLevel::Heavy => Box::new(LevelStrategy::heavy()),
        CompressionLevel::Tldr => Box::new(LevelStrategy::tldr()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn article_light_is_article_specific() {
        let s = LevelStrategy::light();
        let prompt = s.article_system();
        assert!(prompt.contains("distilling an article"));
        assert!(!prompt.contains("book chapter"));
        assert!(prompt.contains("~80%"));
    }

    #[test]
    fn article_medium_targets_50_percent() {
        let s = LevelStrategy::medium();
        let prompt = s.article_system();
        assert!(prompt.contains("~50%"));
        assert!(prompt.contains("Merge paragraphs"));
    }

    #[test]
    fn article_heavy_is_annotated_outline() {
        let s = LevelStrategy::heavy();
        let prompt = s.article_system();
        assert!(prompt.contains("annotated outline"));
        assert!(prompt.contains("Takeaways"));
        assert!(!prompt.contains("book"));
    }

    #[test]
    fn article_tldr_is_card_format() {
        let s = LevelStrategy::tldr();
        let prompt = s.article_system();
        assert!(prompt.contains("card"));
        assert!(prompt.contains("50-150 words"));
        assert!(prompt.contains("Key Points"));
    }

    #[test]
    fn book_light_is_book_specific() {
        let s = LevelStrategy::light();
        let prompt = s.distill_system();
        assert!(prompt.contains("book chapter"));
        assert!(prompt.contains("~80%"));
    }

    #[test]
    fn book_refinement_prompt_exists() {
        let s = LevelStrategy::light();
        let prompt = s.refinement_system();
        assert!(prompt.contains("coherence refinement"));
        assert!(prompt.contains("dangling references"));
    }

    #[test]
    fn light_medium_heavy_support_multi_pass() {
        assert!(LevelStrategy::light().supports_multi_pass());
        assert!(LevelStrategy::medium().supports_multi_pass());
        assert!(LevelStrategy::heavy().supports_multi_pass());
    }

    #[test]
    fn tldr_does_not_support_multi_pass() {
        assert!(!LevelStrategy::tldr().supports_multi_pass());
    }

    #[test]
    fn book_tldr_is_theme_grouped() {
        let s = LevelStrategy::tldr();
        let prompt = s.distill_system();
        assert!(prompt.contains("THEME"));
        assert!(prompt.contains("1-3 pages"));
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
        let s = LevelStrategy::light();
        let prompt = s.distill_user("some chapter text");
        assert!(prompt.contains("some chapter text"));
        assert!(prompt.contains("CONTENT TO DISTILL:"));
    }

    #[test]
    fn tldr_user_prefix_is_extraction() {
        let s = LevelStrategy::tldr();
        let prompt = s.distill_user("some text");
        assert!(prompt.contains("TEXT TO EXTRACT FROM:"));
    }

    #[test]
    fn book_heavy_enforces_prose() {
        let s = LevelStrategy::heavy();
        let prompt = s.distill_system();
        assert!(prompt.contains("Do NOT use bullet points"));
        assert!(prompt.contains("~30%"));
    }

    #[test]
    fn book_heavy_refinement_rewrites_bullets_to_prose() {
        let s = LevelStrategy::heavy();
        let prompt = s.refinement_system();
        assert!(prompt.contains("ENFORCE PROSE"));
        assert!(prompt.contains("REWRITE"));
    }

    #[test]
    fn book_medium_refinement_smooths_terse_sections() {
        let s = LevelStrategy::medium();
        let prompt = s.refinement_system();
        assert!(prompt.contains("terse"));
    }

    #[test]
    fn article_prompts_never_mention_book() {
        for level in [
            LevelStrategy::light(),
            LevelStrategy::medium(),
            LevelStrategy::heavy(),
            LevelStrategy::tldr(),
        ] {
            let prompt = level.article_system();
            assert!(
                !prompt.contains("book chapter"),
                "article prompt should not mention 'book chapter'"
            );
        }
    }

    #[test]
    fn all_prompts_require_compressed_tags() {
        for level in [
            LevelStrategy::light(),
            LevelStrategy::medium(),
            LevelStrategy::heavy(),
            LevelStrategy::tldr(),
        ] {
            assert!(
                level.article_system().contains("<compressed>"),
                "article prompt must mention <compressed> tag"
            );
            assert!(
                level.distill_system().contains("<compressed>"),
                "book prompt must mention <compressed> tag"
            );
        }
    }

    #[test]
    fn each_level_has_distinct_article_prompt() {
        let prompts: Vec<String> = [
            LevelStrategy::light(),
            LevelStrategy::medium(),
            LevelStrategy::heavy(),
            LevelStrategy::tldr(),
        ]
        .iter()
        .map(|s| s.article_system())
        .collect();

        for i in 0..prompts.len() {
            for j in (i + 1)..prompts.len() {
                assert_ne!(
                    prompts[i], prompts[j],
                    "article prompts {i} and {j} must differ"
                );
            }
        }
    }

    #[test]
    fn each_level_has_distinct_book_prompt() {
        let prompts: Vec<String> = [
            LevelStrategy::light(),
            LevelStrategy::medium(),
            LevelStrategy::heavy(),
            LevelStrategy::tldr(),
        ]
        .iter()
        .map(|s| s.distill_system())
        .collect();

        for i in 0..prompts.len() {
            for j in (i + 1)..prompts.len() {
                assert_ne!(
                    prompts[i], prompts[j],
                    "book prompts {i} and {j} must differ"
                );
            }
        }
    }
}
