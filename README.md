# distill

[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

**Structure-preserving semantic compression engine for books and articles.**

Takes a book (PDF/EPUB) or web article (URL) and produces a shorter version that preserves structure, core ideas, and the author's voice — while removing repetition, filler, and meta-text. Powered by any OpenAI-compatible LLM.

> The output feels like the same book — but denser and faster to read.

---

## Table of Contents

- [Quick Start](#quick-start)
- [How It Works](#how-it-works)
- [Modes](#modes)
- [Compression Levels](#compression-levels)
- [Examples](#examples)
- [Research](#research)
- [Configuration](#configuration)
- [Supported Formats](#supported-formats)
- [CLI Reference](#cli-reference)
- [Building](#building)
- [License](#license)

---

## Quick Start

```bash
# Install
cargo install --path .

# Configure your LLM provider (any OpenAI-compatible API)
export DISTILL_API_KEY="your-api-key"
export DISTILL_API_BASE="https://api.openrouter.ai/api/v1"
export DISTILL_MODEL="stepfun/step-3.5-flash"

# Compress a web article → markdown to stdout
distill https://example.com/long-article

# Compress a book → EPUB
distill thinking-fast-and-slow.pdf
# => thinking-fast-and-slow-distilled.epub

# Quick takeaways from an article → structured bullet points
distill -l tldr https://example.com/news-article
```

---

## How It Works

### Article Mode (< 30k tokens)

**Single-pass** — each chunk gets one LLM call, then chunks are reassembled and output directly. Fast, no overhead.

### Book Mode (>= 30k tokens)

**Hierarchical two-pass pipeline:**

```
Input → Ingest → Segment → Pass 1 → Pass 2 → Export
                            │         │
                            ▼         ▼
                         Distill   Refine
                       (per chapter) (coherence)
```

1. **Independent Distillation** — Each chapter is distilled independently. The LLM removes filler, redundancy, and padding while preserving key arguments, frameworks, examples, and the author's voice.
2. **Coherence Refinement** — A second pass sees all distilled chapters together. It fixes dangling references to cut content, removes cross-chapter redundancy, smooths transitions, and ensures consistent terminology.

This approach was selected based on [systematic research](#research) evaluating 8 algorithms across 11 models.

### TLDR Mode

**Single-pass extraction** — produces structured bullet points (key ideas, insights, takeaways) instead of compressed prose. Designed for quick knowledge capture from articles and news.

---

## Modes

Auto-detected based on content size:

| Mode | Trigger | Default Output | Default Level | Pipeline |
|------|---------|---------------|---------------|----------|
| **Article** | < 30k tokens | Markdown (stdout) | `tight` | Single pass |
| **Book** | >= 30k tokens | EPUB (file) | `dense` | Hierarchical (2 pass) |

Override with `--mode book` or `--mode article`.

---

## Compression Levels

| Level | Target | Behavior |
|-------|--------|----------|
| `tight` | ~80% | Remove fluff only, preserve original wording |
| `dense` | ~50% | Compress explanations, merge redundant paragraphs |
| `distilled` | ~30% | Keep strongest insights only, allow restructuring |
| `tldr` | ~5% | Structured extraction: key ideas, insights, takeaways as bullet points |

---

## Examples

**Web article to stdout:**

```bash
distill https://paulgraham.com/greatwork.html
```

**Quick takeaways from a news article:**

```bash
distill -l tldr https://blog.example.com/ai-trends-2026
```

**Pipe to a markdown viewer:**

```bash
distill https://example.com/long-article | glow
```

**Book to EPUB (default):**

```bash
distill sapiens.epub
# => sapiens-distilled.epub
```

**Book to HTML:**

```bash
distill sapiens.epub -f html -o sapiens-short.html
```

**Aggressive compression:**

```bash
distill thinking-fast.pdf -l distilled
```

**Parallel processing for large books:**

```bash
distill large-book.pdf --parallel -j 8
```

**Interrupted book runs resume automatically:**

If a book run is interrupted, re-running the same command will continue from the saved checkpoint automatically. Successful runs clean up the temporary checkpoint file.

**Different LLM providers:**

```bash
# Ollama (local)
distill article.pdf --api-base http://localhost:11434/v1 --model llama3

# OpenRouter
distill article.pdf \
  --api-base https://openrouter.ai/api/v1 \
  --api-key $OPENROUTER_KEY \
  --model stepfun/step-3.5-flash
```

---

## Research

The distillation algorithm was selected through systematic evaluation of **8 algorithms across 11 LLM models** (88 experiments) using "Atomic Habits" by James Clear as the test corpus.

### Algorithm Comparison

| Algorithm | Composite Score | Completeness | Structure | Coherence |
|-----------|:-:|:-:|:-:|:-:|
| **hierarchical** | **0.88** | 0.90 | 0.94 | 0.83 |
| whole_book | 0.80 | 0.73 | 0.88 | 0.85 |
| running_summary | 0.78 | 0.61 | 0.92 | 0.87 |
| independent | 0.78 | 0.61 | 0.91 | 0.88 |
| overlap_10 | 0.77 | 0.59 | 0.87 | 0.88 |
| overlap_20 | 0.76 | 0.56 | 0.89 | 0.89 |
| incremental | 0.76 | 0.57 | 0.89 | 0.88 |
| extract_compress | 0.71 | 0.51 | 0.84 | 0.82 |

The **hierarchical** algorithm won decisively (+0.08 over second place), with the highest completeness (0.90) and structure preservation (0.94) of any algorithm.

### Top Model + Algorithm Combinations

| Model | Algorithm | Composite | Cost/Chapter |
|-------|-----------|:-:|:-:|
| Gemini 2.5 Pro | whole_book | 0.94 | $0.128 |
| GPT-5 Mini | hierarchical | 0.94 | $0.000 |
| GPT-4.1 | hierarchical | 0.92 | $0.000 |
| StepFun Flash | hierarchical | 0.88 | $0.000 |
| DeepSeek V3.2 | hierarchical | 0.92 | $0.006 |

Free models (StepFun Flash, GPT-4.1) perform on par with paid alternatives. See [`research/`](research/) for the full evaluation framework, data, and report.

---

## Configuration

CLI flags take precedence over environment variables, which take precedence over config file values.

### Environment Variables

| Variable | Description |
|----------|-------------|
| `DISTILL_API_KEY` | LLM API key |
| `DISTILL_API_BASE` | API base URL (e.g., `https://openrouter.ai/api/v1`) |
| `DISTILL_MODEL` | Model name (e.g., `stepfun/step-3.5-flash`) |

### Config File

Settings are stored in `~/.config/distill/config.toml`:

```toml
api_key = "sk-..."
api_base = "https://openrouter.ai/api/v1"
model = "stepfun/step-3.5-flash"
level = "dense"
parallel = true
jobs = 4
```

### Managing Config

```bash
# Show current config with sources
distill config

# Set a value
distill config set api_key sk-your-key
distill config set api_base https://openrouter.ai/api/v1
distill config set model stepfun/step-3.5-flash

# Show config file path
distill config path
```

---

## Supported Formats

### Input

| Format | Source |
|--------|--------|
| PDF | Local file or URL |
| EPUB | Local file or URL |
| HTML | URL (article extracted via Mozilla Readability) |

### Output

| Format | Flag | Notes |
|--------|------|-------|
| EPUB | `-f epub` | Default for books. Chapters, TOC, metadata. |
| Markdown | `-f md` | Default for articles. Stdout or file. |
| HTML | `-f html` | Self-contained with inline CSS. |

---

## CLI Reference

```
distill [OPTIONS] <INPUT>

Arguments:
  <INPUT>               File path (PDF/EPUB) or URL

Options:
  -o, --output <PATH>   Output file path
  -f, --format <FMT>    Output format [epub, md, html]
  -l, --level <LEVEL>   Compression level [tight, dense, distilled, tldr]
  -m, --mode <MODE>     Force mode [book, article]
      --model <NAME>    LLM model (overrides DISTILL_MODEL)
      --api-base <URL>  API base URL (overrides DISTILL_API_BASE)
      --api-key <KEY>   API key (overrides DISTILL_API_KEY)
      --parallel        Concurrent chunk processing
  -j, --jobs <N>        Concurrency limit [default: 1]
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
make install       # Build and install to ~/.cargo/bin
make test          # Run all tests
make lint          # Run clippy
make fmt           # Format code
make check         # fmt + lint + test
make clean         # Remove build artifacts
```

---

## License

MIT
