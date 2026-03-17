# PROJECT SPECIFICATION: distill — Semantic Compression CLI

## Role

You are a **Staff Rust Engineer**.
Design and implement a **production-grade CLI tool** called `distill`.
This is NOT a summarizer.
This is a **structure-preserving semantic compression engine**.

---

## Core Objective

`distill` takes a **book (PDF/EPUB)** or **web article (URL)** and produces a **shorter version** that:

**Preserves:**
- Structure (chapters, sections, headers)
- Core ideas and insights
- Meaningful examples (never fully removed)
- Author's tone and vocabulary (as much as possible)

**Removes:**
- Repetition across chapters
- Filler text
- Long transitions
- Meta-text ("we will cover later", "as mentioned previously")

> The output must feel like the same book — but denser and faster to read.

---

## Modes

### Book Mode
- **Triggered when:** input is a file (PDF/EPUB) or URL pointing to a PDF/EPUB, AND estimated token count ≥ 30k
- **Default output:** EPUB
- **Also supports:** HTML, Markdown (via `--format`)
- **Default compression:** `dense`

### Article Mode
- **Triggered when:** input is a URL (to a web article) or text, AND estimated token count < 30k
- **Output:** Markdown to stdout
- **Default compression:** `tight`
- **Optimized for piping** (e.g., `distill https://example.com/post | glow`)

### Mode Detection Logic
1. If input is a URL → fetch and inspect content type
   - If URL points to PDF/EPUB → download, treat as file input
   - If URL points to HTML → extract article text
2. If input is a local file → detect by extension (`.pdf`, `.epub`)
3. Estimate token count of extracted text
4. If tokens ≥ 30k → **Book Mode**; else → **Article Mode**
5. Mode can be forced with `--mode book` or `--mode article`

---

## Compression Levels

| Level       | Target | Behavior                                      |
|-------------|--------|-----------------------------------------------|
| `tight`     | ~80%   | Remove fluff, preserve original wording       |
| `dense`     | ~50%   | Compress explanations, reduce repetition      |
| `distilled` | ~30%   | Keep only strongest insights, allow restructuring |

### Internal Policies per Level

**tight:**
- Remove repetition and filler only
- Minimal rewriting
- Preserve original phrasing wherever possible

**dense:**
- Compress explanations into fewer sentences
- Merge short paragraphs covering the same point
- Shorten repeated mentions of the same concept

**distilled:**
- Aggressive compression
- Keep only the strongest example per concept
- Allow intra-section restructuring for clarity

### Defaults
- **Book Mode** defaults to `dense`
- **Article Mode** defaults to `tight`
- Override with `--level tight|dense|distilled`

---

## CLI Interface

Use `clap` with derive API. No subcommands — single command with flags.

### Usage
```
distill [OPTIONS] <INPUT>
```

`<INPUT>` is a file path (PDF/EPUB) or URL.

### Flags

| Flag | Short | Type | Description |
|------|-------|------|-------------|
| `--output` | `-o` | `PathBuf` | Output file path. Default: `<input>-distilled.<ext>` (book mode) or stdout (article mode) |
| `--format` | `-f` | `epub\|md\|html` | Output format. Default: `epub` (book), `md` (article) |
| `--level` | `-l` | `tight\|dense\|distilled` | Compression level. Default: auto-selected by mode |
| `--mode` | `-m` | `book\|article` | Force mode. Default: auto-detected |
| `--model` | | `String` | LLM model name. Overrides `DISTILL_MODEL` |
| `--api-base` | | `String` | LLM API base URL. Overrides `DISTILL_API_BASE` |
| `--api-key` | | `String` | LLM API key. Overrides `DISTILL_API_KEY` |
| `--parallel` | | flag | Enable concurrent chunk processing in Pass 1 |
| `--jobs` | `-j` | `usize` | Concurrency limit when `--parallel` is set. Default: 4 |
| `--resume` | | flag | Resume from checkpoint if one exists for this input |
| `--clean` | | flag | Remove checkpoint file for this input and exit |
| `--verbose` | `-v` | flag | Increase log verbosity (repeatable: `-v`, `-vv`) |
| `--quiet` | `-q` | flag | Suppress progress bars, errors only |

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `DISTILL_API_KEY` | LLM API key | Yes (unless `--api-key`) |
| `DISTILL_API_BASE` | LLM API base URL (e.g., `https://api.deepseek.com/v1`) | Yes (unless `--api-base`) |
| `DISTILL_MODEL` | Model name (e.g., `deepseek-chat`) | Yes (unless `--model`) |

CLI flags take precedence over env vars.

### Output File Naming (Book Mode)
- If `-o` is provided → use it
- Otherwise → `<input_stem>-distilled.<format_ext>`
- Example: `thinking-fast.pdf` → `thinking-fast-distilled.epub`

### Article Mode Output
- Default: Markdown to stdout
- If `-o` is provided → write to file instead

---

## LLM Backend

Use the **OpenAI-compatible chat completions API** (`/v1/chat/completions`).

This covers: DeepSeek, Ollama, LM Studio, OpenRouter, OpenAI, Anthropic-compatible proxies, and any other provider exposing this protocol.

### Implementation Requirements
- Use `reqwest` for HTTP calls
- Send requests to `{api_base}/chat/completions`
- Support streaming responses (for progress indication on long chunks)
- All LLM calls must go through a single `LlmClient` abstraction with this interface:
  ```rust
  pub struct LlmClient { /* reqwest::Client, config */ }

  impl LlmClient {
      pub async fn complete(&self, system: &str, user: &str) -> Result<String>;
  }
  ```
- Retry logic: **3 retries with exponential backoff** (1s, 4s, 16s) on:
  - HTTP 429 (rate limit)
  - HTTP 5xx
  - Connection timeout
- On retry exhaustion: return error, checkpoint saves progress (see Caching section)
- Timeout: 120s per request (configurable later if needed)

---

## Architecture — Async Pipeline

Runtime: `tokio` (multi-thread runtime).

### Pipeline Stages

```
INPUT → Ingestion → Segmentation → Pass 1 (Local Compression)
      → Pass 2 (Global Deduplication) → Pass 3 (Refinement) → Export
```

---

### 1. Ingestion

Responsible for extracting structured text from input.

**PDF:**
- Use `pdf-extract` or `lopdf` for text extraction
- Attempt to recover structure (headers, paragraphs) from text layout
- Fall back to flat text if structure is unrecoverable

**EPUB:**
- Use `epub` crate to parse EPUB container
- Extract XHTML content from spine items in order
- Convert to Markdown preserving headers and paragraph structure

**URL (article):**
- Use `reqwest` to fetch content
- Inspect Content-Type and URL extension:
  - `application/pdf` or `.pdf` → download to temp file, process as PDF
  - `application/epub+zip` or `.epub` → download to temp file, process as EPUB
  - `text/html` → extract article body using `readability` crate (Rust port of Mozilla Readability)
- Output: Markdown string with structural markers

**Output of ingestion stage:** A `Document` struct:
```rust
pub struct Document {
    pub title: Option<String>,
    pub author: Option<String>,
    pub content: String,          // Markdown
    pub source: InputSource,
    pub estimated_tokens: usize,
}

pub enum InputSource {
    File(PathBuf),
    Url(String),
}
```

---

### 2. Segmentation

Split the document into processable chunks.

**Strategy:**
1. Split by Markdown headers (`#`, `##`, `###`)
2. If any resulting chunk > 5,000 tokens → sub-split by paragraphs
3. Enforce constraints:
   - Minimum chunk size: 500 tokens
   - Maximum chunk size: 5,000 tokens
4. Add **10% content overlap** between adjacent chunks (hardcoded)
5. Token estimation: use a simple heuristic (`word_count * 1.3`) — do NOT add a tokenizer dependency

**Output:** `Vec<Chunk>`:
```rust
pub struct Chunk {
    pub index: usize,
    pub header_path: Vec<String>,  // e.g., ["Chapter 3", "Section 2"]
    pub content: String,
    pub token_estimate: usize,
}
```

---

### 3. Pass 1 — Local Compression

Each chunk is independently compressed via LLM.

**Processing order:** Sequential by default. If `--parallel` is set, process up to `--jobs` chunks concurrently using `tokio::sync::Semaphore`.

**LLM prompt per chunk must include:**
- The compression level and its policy (from the table above)
- The current chunk content
- The current StateLedger (serialized as JSON)
- Instructions to return:
  1. Compressed Markdown
  2. New ledger entries (concepts/examples discovered in this chunk) as JSON

**LLM response parsing:**
- Expect a structured response with two clearly delimited sections:
  - Compressed text (Markdown)
  - Ledger updates (JSON)
- Use XML-style delimiters in the prompt (e.g., `<compressed>...</compressed>` and `<ledger>...</ledger>`) for reliable parsing
- If parsing fails → retry once with a stricter prompt; if still fails → keep original chunk uncompressed and warn

**Output per chunk:** `CompressedChunk`:
```rust
pub struct CompressedChunk {
    pub index: usize,
    pub header_path: Vec<String>,
    pub content: String,             // compressed Markdown
    pub ledger_updates: LedgerDelta, // new concepts/examples
}
```

---

### 4. Pass 2 — Global Deduplication

Operates on ALL compressed chunks together with the full StateLedger.

**Input:** All `CompressedChunk`s + aggregated `StateLedger`

**Behavior:**
- Send the full ledger to the LLM and ask it to identify:
  - Concepts repeated across multiple chunks
  - Examples that appear in multiple places
- For each repeated concept/example:
  - Keep the **strongest, most complete version** (usually first occurrence)
  - In later occurrences: compress to 1–2 sentences with a back-reference (e.g., "As discussed in Chapter 2, ...")
- This pass may need to process chunks in batches if the full content exceeds context window

**Chunking strategy for Pass 2:**
- If total compressed content fits in context (~80% of model's context window) → single LLM call
- Otherwise → process in groups of 5–10 chunks, carrying the ledger forward

---

### 5. Pass 3 — Refinement

Final polish pass.

**Input:** Deduplicated chunks in order

**Behavior (single LLM pass over the full text, or batched if too large):**
- Fix broken transitions between chunks (artifacts of chunk boundaries)
- Smooth tone to match original author's voice
- Ensure no dangling references to removed content
- Do NOT add new content or re-expand compressed sections

---

### 6. Export

**Article Mode:**
- Output GFM (GitHub Flavored Markdown) to stdout
- If `-o` is specified → write to file

**Book Mode — EPUB (default):**
- Use `epub-builder` crate
- Structure:
  - Generate TOC from header hierarchy
  - One XHTML file per top-level chapter
  - Include metadata: title, author (if extracted during ingestion)
  - Embed a minimal, readable CSS (serif font, comfortable line-height, modest margins)

**Book Mode — HTML:**
- Single self-contained HTML file
- Include the same CSS inline
- TOC as an internal link list at the top

**Book Mode — Markdown:**
- Single Markdown file with full header structure preserved

---

## State Management

### StateLedger

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateLedger {
    pub concepts: Vec<Concept>,
    pub examples: Vec<Example>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Concept {
    pub id: String,           // e.g., "concept-001"
    pub name: String,
    pub first_seen_chunk: usize,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Example {
    pub id: String,           // e.g., "example-001"
    pub related_concept: String,  // concept id
    pub first_seen_chunk: usize,
    pub summary: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LedgerDelta {
    pub new_concepts: Vec<Concept>,
    pub new_examples: Vec<Example>,
}
```

The ledger is built incrementally during Pass 1 and used in Pass 2 for deduplication decisions.

---

## Caching & Resume

### When to Cache
- **Book Mode only.** Article mode (< 30k tokens) does not cache.
- Cache is created automatically when processing begins in book mode.

### Cache File
- Location: next to the input file
- Naming: `<input_stem>.distill-cache`
- Example: `thinking-fast.pdf` → `thinking-fast.distill-cache`
- Format: single JSON file containing:
  ```rust
  #[derive(Serialize, Deserialize)]
  pub struct Checkpoint {
      pub input_hash: String,        // SHA-256 of input file (detect changes)
      pub level: CompressionLevel,
      pub model: String,
      pub completed_pass: u8,        // 0 = not started, 1 = pass1 done, etc.
      pub chunks: Vec<ChunkState>,
      pub ledger: StateLedger,
  }

  #[derive(Serialize, Deserialize)]
  pub struct ChunkState {
      pub index: usize,
      pub status: ChunkStatus,       // Pending | Compressed | Deduplicated | Refined
      pub original: String,
      pub compressed: Option<String>,
  }
  ```

### Resume Behavior
- On startup, if `--resume` is passed and a `.distill-cache` file exists:
  - Verify `input_hash` matches current file (if not → warn and start fresh)
  - Skip already-completed chunks/passes
  - Continue from where it left off
- Without `--resume`, existing cache is ignored (but not deleted)

### Cleanup
- On **successful completion**: delete the `.distill-cache` file automatically
- `distill --clean <INPUT>`: delete cache file for this input and exit
- If the process is interrupted (Ctrl+C): cache is preserved for later `--resume`

---

## Error Handling

### Style
Human-readable colored output to stderr, similar to `cargo`.

### Format
```
error: failed to extract text from PDF
  → caused by: unsupported PDF encryption (AES-256)
  → file: /home/user/books/locked-book.pdf

warning: chunk 14 could not be parsed after retry, keeping original
  → section: "Chapter 5 > Market Analysis"
```

### Rules
- Use `miette` or `color-eyre` for error reporting with context chains
- All errors must include:
  - What failed (action)
  - Why it failed (cause)
  - Where it failed (file, chunk, section — whatever is applicable)
- Warnings go to stderr but do not stop processing
- Fatal errors print the error and exit with code 1
- If a checkpoint exists when a fatal error occurs, print:
  ```
  hint: progress saved. run with --resume to continue
  ```

### Exit Codes
| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Runtime error (LLM failure, I/O error, parse failure) |
| 2 | Invalid arguments / configuration |

---

## Progress Reporting

Use `indicatif` crate for progress bars to stderr.

### Display
```
📖 distill · thinking-fast.pdf · dense

[1/3] Compressing    ████████░░░░░░░░  12/24 chunks  [chunk: "Ch.3 > Heuristics"]
[2/3] Deduplicating  ░░░░░░░░░░░░░░░░  waiting
[3/3] Refining       ░░░░░░░░░░░░░░░░  waiting
```

### Requirements
- Show a multi-progress bar with one bar per pass
- Current pass bar shows: chunk progress, current section name
- Waiting passes show "waiting"
- On completion:
  ```
  ✓ Done in 4m 32s · 24 chunks · 187k → 94k tokens (~50%)
  → thinking-fast-distilled.epub
  ```
- `--quiet` suppresses all progress, only errors shown
- `--verbose` / `-v` adds per-chunk timing and token counts
- `-vv` adds full LLM prompts and responses to stderr (debug mode)

---

## Project Structure

Single crate, modules organized by pipeline stage.

```
distill/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI parsing, orchestration
│   ├── cli.rs               # clap derive structs, arg validation
│   ├── config.rs            # Resolve env vars + CLI flags into Config
│   ├── mode.rs              # Mode detection logic
│   ├── ingest/
│   │   ├── mod.rs            # Ingestion trait + dispatch
│   │   ├── pdf.rs
│   │   ├── epub.rs
│   │   └── url.rs
│   ├── segment/
│   │   ├── mod.rs            # Segmentation logic
│   │   └── chunk.rs          # Chunk struct and helpers
│   ├── compress/
│   │   ├── mod.rs            # Pass orchestration (1 → 2 → 3)
│   │   ├── pass1.rs          # Local compression
│   │   ├── pass2.rs          # Global deduplication
│   │   └── pass3.rs          # Refinement
│   ├── llm/
│   │   ├── mod.rs            # LlmClient
│   │   ├── prompt.rs         # Prompt templates for each pass
│   │   └── parse.rs          # Response parsing (XML delimiters → structs)
│   ├── state/
│   │   ├── mod.rs            # StateLedger, LedgerDelta
│   │   └── checkpoint.rs     # Checkpoint serialization, resume logic
│   ├── export/
│   │   ├── mod.rs            # Export trait + dispatch
│   │   ├── markdown.rs
│   │   ├── html.rs
│   │   └── epub.rs
│   ├── progress.rs           # indicatif setup, progress bar management
│   └── error.rs              # Error types, miette/color-eyre setup
└── tests/
    ├── unit/
    │   ├── segment_test.rs    # Segmentation edge cases
    │   ├── parse_test.rs      # LLM response parsing
    │   ├── checkpoint_test.rs # Checkpoint save/load/resume
    │   ├── mode_test.rs       # Mode detection logic
    │   └── config_test.rs     # Config resolution (env + flags)
    ├── integration/
    │   ├── pdf_flow_test.rs   # PDF → ingest → segment → export
    │   ├── epub_flow_test.rs  # EPUB → ingest → segment → export
    │   ├── url_flow_test.rs   # URL → ingest → segment → export
    │   └── helpers/
    │       ├── mod.rs
    │       └── mock_llm.rs    # Mock HTTP server returning canned LLM responses
    └── fixtures/
        ├── sample.pdf
        ├── sample.epub
        └── expected/          # Expected outputs for snapshot comparison
```

---

## Testing

### Unit Tests

Test critical non-LLM logic in isolation:

- **`segment_test.rs`**: segmentation splits correctly by headers, respects min/max token limits, handles documents with no headers, verifies overlap is applied
- **`parse_test.rs`**: LLM response parsing handles well-formed XML delimiters, malformed responses, missing sections, edge cases (empty compressed text, empty ledger)
- **`checkpoint_test.rs`**: checkpoint round-trips (serialize → deserialize), resume skips completed chunks, hash mismatch triggers fresh start
- **`mode_test.rs`**: mode detection for various inputs (small file → article, large file → book, URL to HTML → article, URL to PDF → book, explicit `--mode` override)
- **`config_test.rs`**: env var resolution, CLI flag override precedence, missing required config produces clear error

### Integration Tests

Test end-to-end flows using a **mock LLM server** (`wiremock` or `httpmock`):

- **`mock_llm.rs`**: spins up a local HTTP server that:
  - Accepts OpenAI-compatible `/v1/chat/completions` requests
  - Returns canned compressed text + ledger JSON
  - Can simulate errors (429, 500, timeout) for retry testing
- **`pdf_flow_test.rs`**: PDF file → ingestion → segmentation → Pass 1 (mocked) → export to Markdown. Verify structure is preserved.
- **`epub_flow_test.rs`**: same for EPUB input
- **`url_flow_test.rs`**: spin up a local HTTP server serving a sample HTML page, verify article extraction + compression flow

### What NOT to Test
- Do not test actual LLM output quality (non-deterministic)
- Do not test external crate internals (PDF parsing, EPUB parsing)

### Running Tests
- `cargo test` runs all unit + integration tests
- Integration tests requiring the mock server use `#[tokio::test]`
- Fixture files are committed to the repo under `tests/fixtures/`

---

## Key Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indicatif = "0.17"
color-eyre = "0.6"
epub-builder = "0.7"
# PDF extraction — evaluate: pdf-extract, lopdf
# EPUB reading — evaluate: epub crate
# Article extraction — readability crate (Mozilla Readability port)
sha2 = "0.10"          # For input file hashing (checkpoint)

[dev-dependencies]
wiremock = "0.6"        # Mock HTTP server for integration tests
tempfile = "3"          # Temp dirs for test outputs
```

**Note:** Evaluate PDF/EPUB crates at implementation time. The exact crates may differ — prioritize ones that are actively maintained and extract structured text (not just raw bytes).

---

## Implementation Order

Build and test in this order:

1. **CLI + Config** — `cli.rs`, `config.rs`, `error.rs`. Verify arg parsing and config resolution.
2. **Ingestion** — `ingest/` module. Start with URL (simplest), then EPUB, then PDF.
3. **Segmentation** — `segment/` module. Test thoroughly with unit tests.
4. **LLM Client** — `llm/` module. Test against mock server.
5. **Pass 1** — `compress/pass1.rs` + `llm/prompt.rs` + `llm/parse.rs`. Test parsing.
6. **Checkpoint** — `state/checkpoint.rs`. Test round-trip and resume.
7. **Pass 2 + Pass 3** — `compress/pass2.rs`, `compress/pass3.rs`.
8. **Export** — `export/` module. Start with Markdown, then HTML, then EPUB.
9. **Progress** — `progress.rs`. Wire up indicatif bars.
10. **Integration tests** — Full flows with mock LLM.

---

## Constraints

- **Rust edition:** 2021
- **MSRV:** 1.75+
- **No unsafe code** unless absolutely required by a dependency
- **All public types** must derive `Debug`
- **All errors** must provide context (no bare `.unwrap()` in non-test code)
- **Clippy clean:** `cargo clippy -- -D warnings` must pass
- **No unnecessary dependencies** — evaluate each crate addition
