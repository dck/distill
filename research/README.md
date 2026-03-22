# distill research

Compares LLM models x distillation algorithms to find the best combo for the distill CLI.

## Test Book: Atomic Habits by James Clear

This book was selected for maximum discriminative power between algorithms:

- **Cross-chapter dependencies** -- The "4 Laws of Behavior Change" framework introduced in Part 2 is referenced throughout all subsequent chapters. Context-aware algorithms (running_summary, hierarchical, incremental) should outperform independent processing here, giving us measurable signal.
- **High filler-to-substance ratio** -- Each chapter buries 3-4 key ideas in extended stories, analogies, and motivational padding. This is the exact content profile distillation targets.
- **Diverse content types** -- Framework-introduction chapters, story-heavy chapters, actionable-advice chapters, and philosophical chapters test whether algorithms handle variety.
- **Named examples with data** -- British cycling team, Jerry Seinfeld's chain, specific percentages and studies. These produce reliable QAG questions for the Completeness metric.
- **Modern writing style** -- Representative of the non-fiction books that distill CLI users will actually process, unlike older public domain alternatives.

The book text is never committed to the repo (results are gitignored). Purchase the EPUB through any legal channel.

### Chapter selection

Six chapters chosen for content-type diversity and cross-chapter reference density:

| ch | Book Chapter | Tests |
|----|-------------|-------|
| 1  | Ch 1: The Surprising Power of Atomic Habits | Core framework introduction -- later chapters reference it |
| 2  | Ch 4: The Man Who Didn't Look Right | Research studies + data-heavy |
| 3  | Ch 7: The Secret to Self-Control | Short, story-driven -- tests over-cutting |
| 4  | Ch 11: Walk Slowly but Never Backward | Actionable steps + frameworks |
| 5  | Ch 14: How to Make Good Habits Inevitable | Cross-references earlier Laws extensively |
| 6  | Ch 20: The Downside of Creating Good Habits | Philosophical + nuanced |

## Methodology

### Pipeline

Three sequential phases, each checkpointed and independently re-runnable:

1. **Distill** -- Each (model x algorithm) combination processes the selected chapters, producing distilled text. Outputs are saved per-chapter so partial runs resume where they left off.
2. **Eval** -- A judge model (GPT-5 Mini via GitHub Copilot) scores each distilled chapter on 4 metrics. Eval JSONs are saved per-chapter; failed evals (all-None scores) are automatically re-evaluated on the next run.
3. **Report** -- Aggregates eval results into a leaderboard, per-model analysis, charts, and cost breakdown.

### Judge Model

**GPT-5 Mini** (OpenAI) -- a reasoning model with 128K prompt window.

### Evaluation Framework

We use [DeepEval](https://github.com/confident-ai/deepeval), an open-source LLM evaluation framework. Two metric types are used:

**SummarizationMetric** -- DeepEval's built-in factual consistency metric. It works in three steps:
1. **Extract truths** -- The judge extracts atomic factual claims from the original text (e.g., "The British cycling team improved by 1% in many small areas").
2. **Extract claims** -- The judge extracts claims from the distilled text.
3. **Verify alignment** -- Each original truth is checked against the distilled claims to determine if it was preserved. The score is the fraction of original truths retained. With `n=5`, this verification runs 5 times and results are averaged to reduce variance.

**GEval** ([Liu et al., 2023](https://arxiv.org/abs/2303.16634)) -- A framework for LLM-based evaluation using custom criteria. For each metric, GEval:
1. **Generates evaluation steps** -- Given the criteria text, the judge produces a rubric (chain-of-thought evaluation steps specific to the task).
2. **Scores the output** -- The judge applies those steps to the actual text, producing a 1-10 score that is normalized to 0-1.

GEval's advantage over simple "rate this 1-10" prompts is that the auto-generated rubric makes scoring more consistent and explainable.

### Evaluation Metrics

All metrics produce scores on a 0-1 scale. The composite score is a weighted average.

| Metric | Weight | Type | Inputs | What it measures |
|--------|--------|------|--------|-----------------|
| **Completeness** | 35% | SummarizationMetric | original + distilled | Extracts factual claims from the original and checks how many survive in the distilled version. Uses 5 verification rounds for reliability. The most mechanical metric: did you keep the facts? |
| **Structure Preservation** | 25% | GEval | original + distilled | Does the distilled text maintain the chapter's logical flow -- intro-to-conclusion ordering, argument progression, section organization? A good distillation reads like the same chapter, just shorter. Penalizes if content was reshuffled. |
| **Coherence** | 25% | GEval | distilled only | Does the output read well as standalone text? Catches dangling references ("as mentioned above" where "above" was cut), abrupt transitions, and sentence fragments that lost their context. This is where aggressive cutting shows up. |
| **Compression Quality** | 15% | GEval | original + distilled | Did the model cut the right things (filler, motivational padding, redundancy) and keep the right things (frameworks, concrete examples, actionable advice)? Lower weight because it partially overlaps with Completeness. |

**Composite**: `0.35 * Completeness + 0.25 * Structure + 0.25 * Coherence + 0.15 * Compression Quality`

### Distillation Algorithms

| Algorithm | Strategy | LLM calls per chapter | Context awareness |
|-----------|----------|----------------------|-------------------|
| `whole_book` | All chapters in one prompt | 1 total | Full book |
| `independent` | Each chapter separately | 1 | None |
| `overlap_10` | Chapter + 10% of previous chapter's raw text | 1 | Adjacent |
| `overlap_20` | Chapter + 20% of previous chapter's raw text | 1 | Adjacent |
| `running_summary` | Chapter + summaries of all previous chapters | 2 (distill + summarize) | Cumulative (compressed) |
| `hierarchical` | Pass 1: independent distill, Pass 2: coherence refinement over all | 1 + 1 refinement | Two-pass |
| `incremental` | Chapter + all previous distilled outputs | 1 | Cumulative (full) |
| `extract_compress` | Pass 1: extract key facts, Pass 2: rewrite into prose | 2 (extract + rewrite) | None |

## Setup

```bash
export OPENROUTER_API_KEY=sk-or-v1-...   # worker models
```

## Run

```bash
# 1. Extract chapters from EPUB (purchase separately)
uv run python fetch_book.py atomic_habits.epub

# List all chapters to pick your own selection:
uv run python fetch_book.py atomic_habits.epub --list

# Override chapter selection from config.toml:
uv run python fetch_book.py atomic_habits.epub --chapters 1,4,7,11,14,20

# 2. Run all distillation experiments
uv run python run.py distill

# 3. Evaluate with judge model
uv run python run.py eval

# 4. Generate report + charts -> reports/
uv run python run.py report
```

**Spot-run a single combo:**
```bash
uv run python run.py distill --model "deepseek/deepseek-v3.2" --algo independent
```

**Check progress:**
```bash
uv run python run.py status
```

Results land in `data/results/` (gitignored) and `reports/` (committed). All steps are checkpointed -- safe to rerun.
