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
export DISTILL_API_BASE="https://api.deepseek.com/v1"
export DISTILL_MODEL="deepseek-chat"

# Compress a web article → markdown to stdout
distill https://example.com/long-article

# Compress a book → EPUB
distill thinking-fast-and-slow.pdf
# => thinking-fast-and-slow-distilled.epub
```

---

## How It Works

### Article Mode (< 30k tokens)

**Single-pass** — each chunk gets one LLM call, then chunks are reassembled and output directly. Fast, no overhead.

### Book Mode (>= 30k tokens)

**Three-pass pipeline:**

```
Input → Ingest → Segment → Pass 1 → Pass 2 → Pass 3 → Export
                            │         │         │
                            ▼         ▼         ▼
                          Compress  Dedup     Refine
```

1. **Local Compression** — Each chunk is independently compressed. A semantic ledger tracks concepts, definitions, principles, examples, anti-patterns, and relationships across chunks.
2. **Global Deduplication** — The ledger identifies repeated elements across chapters. The strongest version is kept; later occurrences are compressed to back-references.
3. **Refinement** — Final polish to fix broken transitions, smooth tone, and remove dangling references.

---

## Modes

Auto-detected based on content size:

| Mode | Trigger | Default Output | Default Level | Strategy |
|------|---------|---------------|---------------|----------|
| **Article** | < 30k tokens | Markdown (stdout) | `tight` | Single pass |
| **Book** | >= 30k tokens | EPUB (file) | `dense` | Multi-pass |

Override with `--mode book` or `--mode article`.

---

## Compression Levels

| Level | Target | Behavior |
|-------|--------|----------|
| `tight` | ~80% | Remove fluff only, preserve original wording |
| `dense` | ~50% | Compress explanations, merge redundant paragraphs |
| `distilled` | ~30% | Keep strongest insights only, allow restructuring |

---

## Examples

**Web article to stdout:**

```bash
distill https://paulgraham.com/greatwork.html
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

**Resume after interruption:**

```bash
distill large-book.pdf --resume
```

**Different LLM providers:**

```bash
# Ollama (local)
distill article.pdf --api-base http://localhost:11434/v1 --model llama3

# OpenRouter
distill article.pdf \
  --api-base https://openrouter.ai/api/v1 \
  --api-key $OPENROUTER_KEY \
  --model anthropic/claude-3-haiku
```

---

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `DISTILL_API_KEY` | LLM API key |
| `DISTILL_API_BASE` | API base URL (e.g., `https://api.deepseek.com/v1`) |
| `DISTILL_MODEL` | Model name (e.g., `deepseek-chat`) |

CLI flags (`--api-key`, `--api-base`, `--model`) take precedence over environment variables.

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
  -l, --level <LEVEL>   Compression level [tight, dense, distilled]
  -m, --mode <MODE>     Force mode [book, article]
      --model <NAME>    LLM model (overrides DISTILL_MODEL)
      --api-base <URL>  API base URL (overrides DISTILL_API_BASE)
      --api-key <KEY>   API key (overrides DISTILL_API_KEY)
      --parallel        Concurrent chunk processing
  -j, --jobs <N>        Concurrency limit [default: 4]
      --resume          Resume from checkpoint
      --clean           Remove checkpoint and exit
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
