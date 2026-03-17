# distill

Structure-preserving semantic compression engine for books and articles.

`distill` takes a book (PDF/EPUB) or web article (URL) and produces a shorter version that preserves structure, core ideas, and the author's voice — while removing repetition, filler, and meta-text.

> The output feels like the same book — but denser and faster to read.

## Install

```bash
cargo install --path .
```

Requires Rust 1.93+.

## Quick Start

```bash
# Set up your LLM provider (any OpenAI-compatible API)
export DISTILL_API_KEY="your-api-key"
export DISTILL_API_BASE="https://api.deepseek.com/v1"
export DISTILL_MODEL="deepseek-chat"

# Compress a web article (outputs markdown to stdout)
distill https://example.com/long-article

# Pipe to a markdown viewer
distill https://example.com/long-article | glow

# Compress a book (outputs EPUB by default)
distill thinking-fast-and-slow.pdf

# Compress a book to markdown
distill thinking-fast-and-slow.pdf -f md -o thinking-fast-compressed.md
```

## Modes

distill auto-detects the mode based on content size:

| Mode | Trigger | Default Output | Default Level | Strategy |
|------|---------|---------------|---------------|----------|
| **Article** | < 30k tokens | Markdown (stdout) | `tight` | Single pass |
| **Book** | >= 30k tokens | EPUB (file) | `dense` | Multi-pass (compress → deduplicate → refine) |

Override with `--mode book` or `--mode article`.

## Compression Levels

| Level | Target | Behavior |
|-------|--------|----------|
| `tight` | ~80% | Remove fluff only, preserve original wording |
| `dense` | ~50% | Compress explanations, merge redundant paragraphs |
| `distilled` | ~30% | Keep strongest insights only, allow restructuring |

```bash
# Light compression — keep almost everything
distill article.pdf -l tight

# Aggressive compression — just the key ideas
distill article.pdf -l distilled
```

## Examples

### Web article to stdout

```bash
distill https://paulgraham.com/greatwork.html
```

### Book to EPUB

```bash
distill sapiens.epub
# => sapiens-distilled.epub
```

### Book to HTML with custom output path

```bash
distill sapiens.epub -f html -o sapiens-short.html
```

### Parallel processing for large books

```bash
distill large-book.pdf --parallel -j 8
```

### Resume after interruption

```bash
# If processing is interrupted (Ctrl+C), progress is saved
distill large-book.pdf --resume
```

### Clean up cache

```bash
distill large-book.pdf --clean
```

### Use a different LLM provider

```bash
# Ollama (local)
distill article.pdf --api-base http://localhost:11434/v1 --model llama3

# OpenRouter
distill article.pdf --api-base https://openrouter.ai/api/v1 --api-key $OPENROUTER_KEY --model anthropic/claude-3-haiku
```

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `DISTILL_API_KEY` | LLM API key |
| `DISTILL_API_BASE` | API base URL (e.g., `https://api.deepseek.com/v1`) |
| `DISTILL_MODEL` | Model name (e.g., `deepseek-chat`) |

CLI flags (`--api-key`, `--api-base`, `--model`) override environment variables.

## How It Works

### Article Mode (< 30k tokens)

Single-pass compression: each chunk gets one LLM call to compress it, then chunks are reassembled and output directly.

### Book Mode (>= 30k tokens)

Three-pass pipeline:

1. **Local Compression** — Each chunk is independently compressed. A StateLedger tracks concepts and examples across chunks.
2. **Global Deduplication** — Using the ledger, repeated concepts/examples are identified. The strongest version is kept; later occurrences are compressed to back-references.
3. **Refinement** — Final polish pass to fix broken transitions, smooth tone, and remove dangling references.

### Supported Inputs

| Format | Source |
|--------|--------|
| PDF | Local file or URL |
| EPUB | Local file or URL |
| HTML | URL (article extracted via Mozilla Readability) |

### Supported Outputs

| Format | Flag | Notes |
|--------|------|-------|
| EPUB | `-f epub` | Default for books. Chapters, TOC, metadata. |
| Markdown | `-f md` | Default for articles. Stdout or file. |
| HTML | `-f html` | Self-contained with inline CSS. |

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

## License

MIT
