# distill — Semantic Compression CLI Design

## Goal

CLI tool that takes books (PDF/EPUB) or web articles (URL) and produces shorter versions preserving structure, core ideas, examples, and author voice — while removing repetition, filler, and meta-text.

## Architecture

Async pipeline (tokio): Ingest → Segment → Compress (strategy-selected) → Export

Two modes auto-detected by estimated token count (30k threshold):
- **Article mode** (<30k tokens): SinglePass strategy — compress chunks, output markdown to stdout
- **Book mode** (>=30k tokens): MultiPass strategy — Pass 1 (local compress) → Pass 2 (global dedup via StateLedger) → Pass 3 (refinement) → export EPUB/HTML/MD

LLM calls go through a single `LlmClient` abstraction using OpenAI-compatible `/v1/chat/completions` API.

## Compression Strategy (Strategy Pattern)

```rust
trait CompressionStrategy {
    async fn compress(&self, chunks: Vec<Chunk>, client: &LlmClient, ...) -> Result<Vec<CompressedChunk>>;
}

struct SinglePassStrategy;   // articles, short content
struct MultiPassStrategy;    // books (>=30k tokens)
```

SinglePass: each chunk gets one LLM call, no ledger tracking, no dedup, no refinement.
MultiPass: full 3-pass pipeline with StateLedger for cross-chunk deduplication.

## Compression Levels

| Level | Target | Behavior |
|-------|--------|----------|
| tight (~80%) | Remove fluff only, preserve original wording |
| dense (~50%) | Compress explanations, merge redundant paragraphs |
| distilled (~30%) | Keep strongest insights only, allow restructuring |

Defaults: Article → tight, Book → dense. Override with `--level`.

## Crate Decisions

| Concern | Crate |
|---------|-------|
| CLI | clap (derive) |
| Async runtime | tokio |
| HTTP | reqwest |
| Error reporting | color-eyre |
| PDF extraction | pdf-extract |
| EPUB reading | epub |
| EPUB writing | epub-builder |
| Article extraction | readability |
| Progress bars | indicatif |
| Serialization | serde + serde_json |
| Hashing | sha2 |
| Test mocking | wiremock |

## Key Data Types

- `Document` — extracted text + metadata from ingestion
- `Chunk` — segmented piece with header path and token estimate
- `CompressedChunk` — compressed output with ledger delta
- `StateLedger` — tracks concepts/examples across chunks (MultiPass only)
- `Checkpoint` — serialized progress for resume (Book mode only)

## Project Structure

Single crate, modules by pipeline stage: cli, config, mode, ingest/, segment/, compress/, llm/, state/, export/, progress, error.

## Deviations from PRD

1. **Strategy pattern for compression** — PRD always runs 3 passes. Design uses SinglePass for articles (<30k tokens) and MultiPass for books (>=30k tokens). The 30k threshold aligns with the existing article/book mode boundary.

Everything else follows the PRD as written.
