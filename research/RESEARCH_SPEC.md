# distill Research: Finding the Best Model × Algorithm for Book Distillation

## Overview

This is a research sub-project inside the `distill` repo. The goal is to systematically compare different LLM models and distillation algorithms to find the best combination for compressing non-fiction books — removing filler while preserving key ideas, examples, frameworks, and the author's voice.

The research produces a data-driven report that will inform the default algorithm and recommended models in the `distill` Rust CLI.

## Project Location

```
distill/
├── src/                    # Existing Rust CLI
├── Cargo.toml
├── research/               # ← This sub-project
│   ├── RESEARCH_SPEC.md    # This file
│   ├── pyproject.toml      # uv-managed Python project
│   ├── config.toml         # Models, algorithms, settings
│   ├── run.py              # Main CLI entry point (argparse)
│   ├── fetch_book.py       # Downloads + splits test book
│   ├── algorithms.py       # All distillation algorithms
│   ├── prompts.py          # Prompt tiers (strong/weak/reasoning)
│   ├── eval_metrics.py     # DeepEval metric definitions
│   ├── judge.py            # Claude Opus 4.6 wrapper for DeepEval
│   ├── report.py           # Generate markdown report + charts
│   ├── data/
│   │   ├── originals/      # ← gitignored, created by fetch_book.py
│   │   └── results/        # ← gitignored, created by run.py
│   ├── reports/            # ← committed: eval_report.json, report.md, charts/
│   └── .gitignore
```

## Setup & Dependencies

### Python environment (uv)

```bash
cd research
uv init
uv add deepeval anthropic openai tomli matplotlib
```

### pyproject.toml

```toml
[project]
name = "distill-research"
version = "0.1.0"
description = "Research: finding best model × algorithm for book distillation"
requires-python = ">=3.12"
dependencies = [
    "deepeval",
    "anthropic",
    "openai",
    "tomli",
    "matplotlib",
]
```

### Environment variables

```bash
export OPENROUTER_API_KEY=sk-or-v1-...   # For worker models
export ANTHROPIC_API_KEY=sk-ant-...       # For Opus 4.6 judge
```

## Test Book

**"Think and Grow Rich"** by Napoleon Hill (1937 original edition).

- **License**: Public domain. The 1937 edition's copyright was not renewed in the 1950s.
- **Source**: Internet Archive (plain text version).
- **Do NOT commit the book text to git.** The `fetch_book.py` script downloads and splits it.

### Why this book

- Mix of high-value content (13 principles, named examples like Ford/Edison/Carnegie, concrete frameworks) and significant filler (motivational repetition, verbose introductions, rhetorical padding).
- Clear chapter structure — each chapter covers one principle.
- Well-known enough that spot-checking quality is feasible even without a full read.
- English language (Russian testing is out of scope for this phase).

### Chapters to use (5-6)

Select chapters that represent different content types:

| Chapter | Title | Content type |
|---------|-------|-------------|
| Ch 1 | Desire | Heavy on anecdotes + one core framework |
| Ch 2 | Faith | Abstract concepts + examples |
| Ch 3 | Auto-Suggestion | Practical instructions + repetition |
| Ch 4 | Specialized Knowledge | Data-heavy, examples, actionable |
| Ch 5 | Imagination | Mix of theory + concrete examples |
| Ch 6 | Organized Planning | Long, detailed, many sub-sections |

### fetch_book.py behavior

1. Download the plain text from Internet Archive (hardcoded URL).
2. Split into chapters using regex patterns on chapter headings.
3. Save each chapter as `data/originals/ch01_desire.txt`, etc.
4. Idempotent — if files exist and are non-empty, skip download.
5. Print chapter word counts after splitting.

## Models

Defined in `config.toml`. Easy to add/remove.

### Initial model set

```toml
[[models]]
id = "deepseek/deepseek-v3.2"
name = "DeepSeek V3.2"
tier = "strong"           # prompt tier
context_window = 164000
free = false
cost_input = 0.255        # per 1M tokens
cost_output = 0.40

[[models]]
id = "deepseek/deepseek-r1:free"
name = "DeepSeek R1 Free"
tier = "reasoning"
context_window = 164000
free = true

[[models]]
id = "openrouter/hunter-alpha"
name = "Hunter Alpha"
tier = "strong"
context_window = 1048576
free = true
note = "Prompts logged by provider"

[[models]]
id = "meta-llama/llama-3.3-70b-instruct:free"
name = "Llama 3.3 70B"
tier = "weak"
context_window = 131000
free = true

[[models]]
id = "qwen/qwen3-coder:free"
name = "Qwen3 Coder 480B"
tier = "strong"
context_window = 262000
free = true

[[models]]
id = "google/gemini-3-flash-preview"
name = "Gemini 3 Flash"
tier = "strong"
context_window = 1048576
free = false
cost_input = 0.50
cost_output = 3.00
note = "Paid quality ceiling reference"
```

The user plans to add more models later (NVIDIA, Mistral, etc.) by editing this file.

### Prompt tiers

Models are assigned a `tier` which determines which prompt variant they receive. Defined in `prompts.py`.

**Tier: "strong"** — Concise, high-level instructions. These models understand intent well. Less prescriptive.

**Tier: "weak"** — More explicit, structured instructions with DO/DON'T lists. These models need more hand-holding to avoid producing summaries instead of distillations.

**Tier: "reasoning"** — Adapted for models like DeepSeek R1 that have a thinking phase. Prompt guides the reasoning toward identifying filler vs substance, then instructs to output only the distilled text.

### All prompts share these core elements

- The distinction between what to REMOVE (filler, redundancy, motivational padding, verbose intros) and what to PRESERVE (key arguments, named examples with data, frameworks, actionable advice, definitions, cause-effect chains).
- The framing: "This should read like the same book, just shorter — not a summary."
- Output format: markdown with chapter headings and sub-sections.
- NO specific compression ratio target. Let each model decide naturally how much to cut. Actual compression ratio is measured as a metric.

## Algorithms (8)

All algorithms use the same prompt (per tier) for the actual distillation call. The only difference is **what context** is provided alongside each chapter.

### 1. `whole_book`
Send all chapters concatenated in one prompt. Model sees full book context.
- **Requires**: context window > total book tokens.
- **Skip automatically** if model context window is too small.
- Pros: model can deduplicate across chapters, sees full narrative arc.
- Cons: expensive input tokens, may lose detail in very long context.

### 2. `independent`
Each chapter processed independently. Zero cross-chapter context.
- Pros: simple, parallelizable, works with any context size.
- Cons: no continuity, cross-references may break.

### 3. `overlap_10`
Each chapter receives the last 10% of the **raw text** of the previous chapter as prefix context.
- Pros: local continuity.
- Cons: overlap is raw (verbose) text, adds input cost.

### 4. `overlap_20`
Same as overlap_10 but with 20% overlap. Tests whether more context helps.

### 5. `running_summary`
After distilling each chapter, generate a short summary (~200 words) of that chapter. Pass the cumulative summaries of all previous chapters as context to each subsequent chapter.
- Pros: cheap context (summaries are short), global awareness.
- Cons: summaries may lose nuance; sequential only.

### 6. `hierarchical`
Two-pass map-reduce approach.
- Pass 1: Distill each chapter independently (parallelizable).
- Pass 2: Model sees ALL distilled chapters together, refines for coherence — fixes dangling references, smooths transitions, removes cross-chapter redundancy. Does NOT change length.
- **Requires**: context window > total distilled output tokens for pass 2.
- Pros: best coherence (supported by BooookScore research).
- Cons: 2× LLM calls.

### 7. `incremental`
Carry forward the full distilled text. Each step: model sees all previously distilled chapters + new raw chapter, outputs the complete updated distillation.
- **Requires**: growing context window (final step sees all distilled + last raw chapter).
- Pros: highest information retention, model can revise earlier chapters.
- Cons: sequential, growing cost, may over-edit early chapters.

### 8. `extract_compress`
Two-phase hybrid approach.
- Phase 1 (extractive): LLM identifies key elements as a structured list (KEY_ARGUMENT, EXAMPLE, FRAMEWORK, INSIGHT, ACTIONABLE).
- Phase 2 (abstractive): LLM rewrites those elements into flowing prose matching the author's voice.
- Pros: explicit control over what's kept, less hallucination.
- Cons: 2× calls, prose may feel less like original author.

### Algorithm compatibility

The runner must check `model.context_window` against estimated token count before running an algorithm. Rules:
- `whole_book`: needs context > (sum of all chapter tokens + output buffer).
- `hierarchical` pass 2: needs context > (sum of distilled chapter tokens + output buffer).
- `incremental` final step: needs context > (sum of all distilled tokens + last raw chapter + output buffer).
- All other algorithms: need context > (single chapter tokens × 1.5 for overlap + output buffer).

If a model × algorithm combination exceeds the context window, **skip it** and log a message. Do not fail.

Estimate tokens as `len(text) / 4` (rough English approximation).

## Evaluation

### Judge model

**Claude Opus 4.6** via Anthropic API directly (not OpenRouter). The user has max-tier API access with high rate limits.

Implement a custom `DeepEvalBaseLLM` wrapper class (`judge.py`) that:
- Calls `anthropic.Anthropic().messages.create()` with `model="claude-opus-4-6"`.
- Handles DeepEval's expected interface (`generate`, `a_generate`, `get_model_name`).
- Extracts JSON from responses when DeepEval passes a schema.

### Metrics (4)

Ordered by importance (this ranking should be reflected in the final weighted score):

1. **Completeness / Coverage** (weight: 0.35) — `SummarizationMetric` from DeepEval.
   Uses QAG: generates questions from original, checks if distilled text can answer them.
   Also checks alignment (no hallucinations in distilled text).
   Final score = min(alignment, coverage).

2. **Structure Preservation** (weight: 0.25) — Custom `GEval` metric.
   Criteria: maintains original chapter organization, logical order of arguments,
   progression from intro to conclusion. Feels like the same chapter, just shorter.

3. **Coherence / Readability** (weight: 0.25) — Custom `GEval` metric.
   Criteria: continuous narrative, sentences follow logically, no dangling references,
   smooth transitions.

4. **Compression Quality** (weight: 0.15) — Custom `GEval` metric.
   Criteria: removed text was genuinely low-value (filler, padding, redundancy);
   retained text is high-value (arguments, examples, frameworks, advice).

### Additional non-LLM metrics (computed, no API calls)

- **Compression ratio**: `len(distilled) / len(original)` — measured per chapter.
- **Cost estimate**: based on model pricing from config × actual token counts.
- **Latency**: wall-clock time per chapter distillation.

### Weighted composite score

```
composite = (0.35 × completeness) + (0.25 × structure) + (0.25 × coherence) + (0.15 × compression_quality)
```

This is used for the final leaderboard ranking.

## Checkpointing & Resilience

### Distillation checkpoints

- Each distilled chapter saved immediately as: `data/results/{model_slug}__{algo}/ch01_desire.txt`
- Before each API call, check if file exists and is non-empty. If yes, skip.
- Model slug: take model id, replace `/` and `:` with `_` (e.g., `deepseek_deepseek-v3.2`).

### Evaluation checkpoints

- Each chapter's eval result saved as: `data/results/{experiment}/eval_ch01_desire.json`
- Contains all metric scores + reasons for that chapter.
- Eval runner skips chapters that already have eval JSON.

### API error handling

- Wrap every API call in retry logic: 3 attempts, exponential backoff (2s, 8s, 32s).
- On persistent failure (3 retries exhausted): log error, skip this chapter/experiment, continue.
- Free model rate limits (20 req/min, 200 req/day): add 3-second delay between calls to free models.
- If a model fails on 3+ consecutive chapters, mark it as "down" and skip remaining chapters for that model. Log warning.

## CLI Interface

Single entry point: `run.py` with argparse subcommands.

```bash
# Fetch and prepare the test book
uv run python fetch_book.py

# Run all distillation experiments
uv run python run.py distill

# Run specific model or algorithm only
uv run python run.py distill --model "deepseek/deepseek-v3.2"
uv run python run.py distill --algo independent
uv run python run.py distill --model "deepseek/deepseek-v3.2" --algo overlap_10

# Run evaluation on all completed experiments
uv run python run.py eval

# Run evaluation for a specific experiment
uv run python run.py eval --experiment "deepseek_deepseek-v3.2__overlap_10"

# Generate the final report (reads eval_report.json, produces report.md + charts)
uv run python run.py report

# Show status: which experiments are done, which need eval
uv run python run.py status
```

### `distill` subcommand behavior

1. Load `config.toml` — get model list and settings.
2. Load chapters from `data/originals/` (fail if not found, suggest running `fetch_book.py`).
3. For each model × algorithm combination:
   a. Check context window compatibility. Skip if incompatible.
   b. Check if all chapter results already exist (checkpoint). Skip if complete.
   c. Run the algorithm. Save each chapter result immediately.
   d. Log progress: `[3/48] deepseek-v3.2 × overlap_10 → ch03 ✓ (4.2s)`
4. Print summary: X experiments complete, Y skipped (context), Z failed.

### `eval` subcommand behavior

1. Discover all experiments in `data/results/` that have distilled chapter files.
2. For each experiment × chapter, check if `eval_*.json` exists. Skip if yes.
3. Run 4 DeepEval metrics using Opus 4.6 judge.
4. Save per-chapter eval JSON immediately.
5. After all evals, aggregate into `reports/eval_report.json`.

### `report` subcommand behavior

1. Read `reports/eval_report.json`.
2. Generate `reports/report.md` with:
   - Leaderboard table (ranked by weighted composite score).
   - Per-metric breakdown tables.
   - Per-algorithm analysis (which algorithm wins on average across models?).
   - Per-model analysis (which model wins on average across algorithms?).
   - Insights and observations (written by the report generator).
   - Cost comparison table.
3. Generate charts in `reports/charts/`:
   - `leaderboard.png` — horizontal bar chart of top 15 experiments by composite score.
   - `algo_comparison.png` — grouped bar chart: algorithms on x-axis, metrics as groups.
   - `model_comparison.png` — grouped bar chart: models on x-axis, metrics as groups.
   - `compression_vs_completeness.png` — scatter plot: compression ratio vs completeness score.
   - `cost_vs_quality.png` — scatter plot: estimated cost vs composite score.

### `status` subcommand behavior

Print a table showing:
```
Model               Algorithm        Chapters  Eval'd  Status
─────────────────────────────────────────────────────────────
deepseek-v3.2       independent      6/6       6/6     ✓ Complete
deepseek-v3.2       whole_book       -         -       ⊘ Skipped (context)
hunter-alpha        whole_book       6/6       0/6     ◐ Needs eval
llama-3.3-70b       overlap_10       3/6       0/6     ◑ Partial
...
```

## config.toml Structure

```toml
[settings]
target_book_url = "https://archive.org/download/..."
judge_model = "claude-opus-4-6"
temperature = 0.3
eval_temperature = 0.0
free_model_delay_seconds = 3
retry_attempts = 3
retry_backoff_base = 2

[metrics_weights]
completeness = 0.35
structure = 0.25
coherence = 0.25
compression_quality = 0.15

[[models]]
id = "deepseek/deepseek-v3.2"
name = "DeepSeek V3.2"
tier = "strong"
context_window = 164000
free = false
cost_input = 0.255
cost_output = 0.40

# ... more models ...

# Algorithms are NOT in config — they are code.
# But they could be toggled on/off:
[algorithms]
enabled = [
    "whole_book",
    "independent",
    "overlap_10",
    "overlap_20",
    "running_summary",
    "hierarchical",
    "incremental",
    "extract_compress",
]
```

## Prompt Design (prompts.py)

### Shared core instructions (all tiers)

```
You are distilling a book chapter. Your goal is to remove low-value content
while preserving the intellectual substance.

REMOVE: filler phrases, redundant restatements of the same idea, excessive
anecdotes that repeat a point already made, verbose introductions, motivational
padding, unnecessary transitions, rhetorical questions that add no information.

PRESERVE: key arguments, frameworks, concrete examples (with names and data),
research citations, actionable advice, important quotes, definitions,
cause-effect relationships.

This should read like the same book chapter, just shorter — NOT a summary.
Maintain the author's voice and writing style.
Output as markdown with appropriate headings and sub-sections.
Output ONLY the distilled text, no meta-commentary.
```

### Tier "strong" additions

Minimal additions. Just the core instructions above. Strong models handle implicit intent well.

### Tier "weak" additions

Add explicit DO/DON'T examples:

```
DO: Keep "Henry Ford started with nothing and built an empire by applying
the principle of definiteness of purpose."
DON'T: Keep "It is a well-known fact that many people, in many walks of life,
have found that there is great power in having a clear purpose."

DO: Keep specific steps, numbers, and frameworks.
DON'T: Keep paragraphs that only say "this is important" without explaining why.
```

### Tier "reasoning" additions

Guide the thinking phase:

```
Before writing the distilled text, analyze the chapter:
1. Identify the core thesis/argument of the chapter.
2. List the concrete examples and frameworks that support it.
3. Identify filler: paragraphs that restate already-made points,
   motivational padding, verbose transitions.
4. Now write the distilled version, keeping only items from step 1-2.
```

### Per-algorithm context prefixes

Each algorithm prepends its own context to the user message (previous chapter tail, running summary, etc.) as described in the Algorithms section. The system prompt stays the same — only the user message content changes.

## Expected Output

After running all phases, the `reports/` directory contains:

```
reports/
├── eval_report.json          # Raw data: all scores per experiment per chapter
├── report.md                 # Human-readable analysis with tables and insights
└── charts/
    ├── leaderboard.png
    ├── algo_comparison.png
    ├── model_comparison.png
    ├── compression_vs_completeness.png
    └── cost_vs_quality.png
```

The `report.md` should conclude with:
- **Recommended algorithm** for the distill CLI default.
- **Recommended model tiers** (best free, best budget, best quality).
- **Key insights** about what works and what doesn't.
- **Limitations** of the research (sample size, single book, etc.).

## Cost Estimate

With 6 models × 8 algorithms × 6 chapters = ~288 distillation calls (minus skipped combos, realistically ~200 calls). Most models are free. Gemini 3 Flash is the only significant cost (~$2-3 for all its runs). Evaluation with Opus 4.6 for ~200 chapter evals at ~2K output tokens each ≈ $10-15. **Total estimated cost: $12-18.**

## Implementation Order

1. `pyproject.toml` + `config.toml` + `.gitignore`
2. `fetch_book.py` — download and split the book
3. `prompts.py` — define the 3 prompt tiers
4. `algorithms.py` — implement all 8 algorithms
5. `judge.py` — Opus 4.6 DeepEval wrapper
6. `eval_metrics.py` — define the 4 metrics
7. `run.py` — CLI with `distill`, `eval`, `report`, `status` subcommands
8. `report.py` — report generation + matplotlib charts
9. Test with 1 model × 1 algorithm × 1 chapter end-to-end
10. Run full matrix
