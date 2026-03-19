"""Prompt functions for the distillation research pipeline."""

_CORE_INSTRUCTIONS = """\
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
Output ONLY the distilled text, no meta-commentary."""

_WEAK_ADDITIONS = """

Examples to guide your judgment:

DO: Keep "Henry Ford started with nothing and built an empire by applying \
the principle of definiteness of purpose."
DON'T: Keep "It is a well-known fact that many people, in many walks of life, \
have found that there is great power in having a clear purpose."

DO: Keep specific steps, numbers, and frameworks.
DON'T: Keep paragraphs that only say "this is important" without explaining why."""

_REASONING_ADDITIONS = """

Before writing the distilled text, analyze the chapter:
1. Identify the core thesis/argument of the chapter.
2. List the concrete examples and frameworks that support it.
3. Identify filler: paragraphs that restate already-made points, \
motivational padding, verbose transitions.
4. Now write the distilled version, keeping only items from step 1-2."""


def get_system_prompt(tier: str) -> str:
    """Return system prompt for the given tier: 'strong', 'weak', or 'reasoning'."""
    if tier == "strong":
        return _CORE_INSTRUCTIONS
    elif tier == "weak":
        return _CORE_INSTRUCTIONS + _WEAK_ADDITIONS
    elif tier == "reasoning":
        return _CORE_INSTRUCTIONS + _REASONING_ADDITIONS
    else:
        raise ValueError(f"Unknown tier: {tier!r}. Expected 'strong', 'weak', or 'reasoning'.")


def get_distill_user_message(chapter_text: str, context: str | None = None) -> str:
    """Wrap chapter text + optional context prefix into a user message."""
    if context:
        return f"{context}\n\n---\n\nCHAPTER TO DISTILL:\n\n{chapter_text}"
    return f"CHAPTER TO DISTILL:\n\n{chapter_text}"


def get_extract_prompt() -> str:
    """System prompt for phase 1 of extract_compress: structured extraction."""
    return """\
You are extracting the key intellectual content from a book chapter.

Read the chapter and produce a structured list of elements worth preserving. \
For each element, use one of these tags:

- KEY_ARGUMENT: A core claim, thesis, or principle the author is making.
- EXAMPLE: A concrete, named example with specific details (people, companies, numbers, dates).
- FRAMEWORK: A step-by-step method, numbered list, or structured approach the author presents.
- INSIGHT: A non-obvious observation, cause-effect relationship, or counterintuitive point.
- ACTIONABLE: Specific advice the reader can act on — instructions, exercises, or recommendations.

Format each element as:

[TAG] Brief title
Content: The essential substance in 1-3 sentences, preserving specific names, numbers, and details.

Extract ALL elements worth keeping. Do NOT summarize — capture the specifics. \
Do NOT add your own commentary or analysis. \
If an element fits multiple tags, use the most specific one."""


def get_rewrite_prompt() -> str:
    """System prompt for phase 2 of extract_compress: rewrite into flowing prose."""
    return """\
You are rewriting extracted elements from a book chapter into flowing prose \
that reads like the original author wrote it.

You will receive a structured list of key elements (KEY_ARGUMENT, EXAMPLE, \
FRAMEWORK, INSIGHT, ACTIONABLE). Your job:

1. Weave these elements into a coherent chapter that maintains the author's \
voice and writing style.
2. Preserve ALL specific details — names, numbers, dates, steps.
3. Use appropriate markdown headings and sub-sections.
4. Maintain the logical order from the original chapter.
5. Add only the minimal connective tissue needed for readability — \
do NOT add filler, motivation, or padding.

Output ONLY the rewritten chapter text, no meta-commentary."""


def get_summary_prompt() -> str:
    """System prompt for running_summary algorithm: generate ~200 word chapter summary."""
    return """\
Summarize this distilled chapter in approximately 200 words. \
Focus on the core thesis, key frameworks, and most important examples. \
This summary will be used as context when distilling subsequent chapters, \
so emphasize concepts and terminology that may be referenced later.

Output ONLY the summary, no meta-commentary."""


def get_refinement_prompt() -> str:
    """System prompt for hierarchical pass 2: coherence refinement across chapters."""
    return """\
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

Output the complete refined text with all chapters, using the same markdown \
heading structure as the input.
Output ONLY the refined text, no meta-commentary."""
