# distill

[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Structure-preserving semantic compression for books and articles.**

Takes a book (PDF/EPUB) or web article (URL) and produces a shorter version that preserves structure, core ideas, and the author's voice. Powered by any OpenAI-compatible LLM.

> The output reads like the same book — just denser.

---

## Quick Start

```bash
cargo install --path .

# One-time setup
distill config set api_key sk-your-key
distill config set api_base https://openrouter.ai/api/v1
distill config set model stepfun/step-3.5-flash

# Distill an article
distill https://paulgraham.com/greatwork.html

# Distill a book
distill atomic-habits.epub
```

---

## How It Works

**Articles** (< 30k tokens) use a **single-pass** pipeline — one LLM call per chunk, output directly.

**Books** (>= 30k tokens) use a **[research-validated](research/)** hierarchical two-pass pipeline:

```
Pass 1: Distill each chapter independently
Pass 2: Coherence refinement across all chapters
```

1. **Distill** — Each chapter is compressed independently. Filler, redundancy, and padding are removed while keeping arguments, frameworks, examples, and the author's voice.
2. **Refine** — A second pass sees all distilled chapters together. Fixes dangling references, removes cross-chapter redundancy, smooths transitions, and ensures consistent terminology.

**TLDR** — Single-pass structured extraction. Produces bullet points (key ideas + insights) instead of prose.

---

## Compression Levels

| Level | Retention | Use case |
|-------|-----------|----------|
| `tight` | ~80% | Light cleanup — remove fluff, keep original wording |
| `dense` | ~50% | Default for books — compress explanations, merge paragraphs |
| `distilled` | ~30% | Aggressive — strongest insights only |
| `tldr` | ~5% | Bullet points — key ideas, insights, one sentence each |

Set per-run with `-l` or as default via `distill config set level dense`.

---

## Examples

```bash
# Article to stdout
distill https://paulgraham.com/greatwork.html

# Quick takeaways from a news article
distill -l tldr https://blog.bytebytego.com/p/how-openai-codex-works

# Pipe to a markdown viewer
distill https://example.com/long-article | glow

# Book to EPUB (default for books)
distill sapiens.epub
# => sapiens-distilled.epub

# Book to markdown
distill sapiens.epub -f md -o sapiens-short.md

# Aggressive compression
distill thinking-fast.pdf -l distilled

# Parallel processing (4 chapters at once)
distill large-book.pdf -j 4

# Different LLM provider
distill article.pdf --api-base http://localhost:11434/v1 --model llama3
```

---

## Research

The algorithm was chosen through systematic evaluation of **8 distillation algorithms across 11 LLM models** (88 experiments). Full methodology, data, and report: [`research/`](research/).

| Algorithm | Score | Completeness | Structure | Coherence |
|-----------|:-----:|:------------:|:---------:|:---------:|
| **hierarchical** | **0.88** | **0.90** | **0.94** | 0.83 |
| whole_book | 0.80 | 0.73 | 0.88 | 0.85 |
| running_summary | 0.78 | 0.61 | 0.92 | 0.87 |
| independent | 0.78 | 0.61 | 0.91 | 0.88 |
| extract_compress | 0.71 | 0.51 | 0.84 | 0.82 |

Top model+algorithm combinations (free models match or beat paid):

| Model | Algorithm | Score | Cost/chapter |
|-------|-----------|:-----:|:------------:|
| GPT-5 Mini | hierarchical | 0.94 | $0.00 |
| GPT-4.1 | hierarchical | 0.92 | $0.00 |
| StepFun Flash | hierarchical | 0.88 | $0.00 |
| DeepSeek V3.2 | hierarchical | 0.92 | $0.006 |

---

## Configuration

Resolution order: **CLI flags > env vars > config file**.

```bash
# Set up config (stored in ~/.config/distill/config.toml)
distill config set api_key sk-your-key
distill config set api_base https://openrouter.ai/api/v1
distill config set model stepfun/step-3.5-flash

# Optional defaults
distill config set level dense
distill config set jobs 4

# View current config with sources
distill config

# Show config file path
distill config path
```

Environment variables (`DISTILL_API_KEY`, `DISTILL_API_BASE`, `DISTILL_MODEL`) also work and override the config file.

---

## Supported Formats

| | Input | Output |
|---|---|---|
| **PDF** | Local file or URL | -- |
| **EPUB** | Local file or URL | `-f epub` (default for books) |
| **HTML** | URL (Readability extraction) | `-f html` |
| **Markdown** | -- | `-f md` (default for articles, stdout) |

---

## CLI Reference

```
distill [OPTIONS] <INPUT>
distill config [set <key> <value> | path]

Arguments:
  <INPUT>               File path (PDF/EPUB) or URL

Options:
  -o, --output <PATH>   Output file path
  -f, --format <FMT>    Output format [epub, md, html]
  -l, --level <LEVEL>   Compression level [tight, dense, distilled, tldr]
  -m, --mode <MODE>     Force mode [book, article]
      --model <NAME>    LLM model
      --api-base <URL>  API base URL
      --api-key <KEY>   API key
  -j, --jobs <N>        Concurrency [default: 1]
  -v, --verbose         Increase verbosity (-v, -vv)
  -q, --quiet           Errors only
  -h, --help            Print help
```

---

## Building

Requires Rust 1.93+.

```bash
make build         # Debug build
make release       # Release build
make install       # Install to ~/.cargo/bin
make test          # Run tests
make check         # fmt + lint + test
```

---

## License

MIT
