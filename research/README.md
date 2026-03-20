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

## Setup

```bash
export OPENROUTER_API_KEY=sk-or-v1-...   # worker models
export ANTHROPIC_API_KEY=sk-ant-...       # Opus 4.6 judge
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

# 3. Evaluate with Opus 4.6
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
