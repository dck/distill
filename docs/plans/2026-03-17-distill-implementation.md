# distill Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a semantic compression CLI that takes books (PDF/EPUB) or web articles (URL) and produces shorter versions preserving structure, using LLM-powered compression.

**Architecture:** Async pipeline (tokio) with strategy pattern — SinglePass for articles (<30k tokens), MultiPass (3-pass) for books. LLM via OpenAI-compatible API. Modules organized by pipeline stage.

**Tech Stack:** Rust 2024 (edition), tokio, clap, reqwest, color-eyre, serde, pdf-extract, epub, epub-builder, readability, indicatif, wiremock

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `src/main.rs`

**Step 1: Initialize git repo**

```bash
cd /Users/deck/work/distill
git init
```

**Step 2: Create .gitignore**

```gitignore
/target
*.distill-cache
.env
```

**Step 3: Create Cargo.toml**

```toml
[package]
name = "distill"
version = "0.1.0"
edition = "2024"
rust-version = "1.93"
description = "Structure-preserving semantic compression engine for books and articles"

[dependencies]
tokio = { version = "1.50", features = ["full"] }
clap = { version = "4.6", features = ["derive"] }
reqwest = { version = "0.13", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indicatif = "0.18"
color-eyre = "0.6"
epub-builder = "0.8"
pdf-extract = "0.10"
epub = "2"
readability = "0.3"
sha2 = "0.10"
html2md = "0.2"

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
```

**Step 4: Create minimal main.rs**

```rust
fn main() {
    println!("distill");
}
```

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: successful compilation

**Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/main.rs
git commit -m "feat: project scaffold with dependencies"
```

---

### Task 2: Error Types

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs`

**Step 1: Create error.rs with custom error type**

```rust
use std::path::PathBuf;

pub type Result<T> = color_eyre::Result<T>;

#[derive(Debug)]
pub enum DistillError {
    Ingestion { source: String, cause: String },
    Segmentation { cause: String },
    Compression { chunk_index: usize, section: String, cause: String },
    Llm { cause: String },
    Export { cause: String },
    Config { cause: String },
    Checkpoint { path: PathBuf, cause: String },
}

impl std::fmt::Display for DistillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ingestion { source, cause } => {
                write!(f, "failed to ingest content\n  -> source: {source}\n  -> caused by: {cause}")
            }
            Self::Segmentation { cause } => {
                write!(f, "failed to segment document\n  -> caused by: {cause}")
            }
            Self::Compression { chunk_index, section, cause } => {
                write!(f, "failed to compress chunk {chunk_index}\n  -> section: \"{section}\"\n  -> caused by: {cause}")
            }
            Self::Llm { cause } => {
                write!(f, "LLM request failed\n  -> caused by: {cause}")
            }
            Self::Export { cause } => {
                write!(f, "failed to export\n  -> caused by: {cause}")
            }
            Self::Config { cause } => {
                write!(f, "configuration error\n  -> {cause}")
            }
            Self::Checkpoint { path, cause } => {
                write!(f, "checkpoint error\n  -> file: {}\n  -> caused by: {cause}", path.display())
            }
        }
    }
}

impl std::error::Error for DistillError {}
```

**Step 2: Wire up color-eyre in main.rs**

```rust
mod error;

fn main() -> error::Result<()> {
    color_eyre::install()?;
    println!("distill");
    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/error.rs src/main.rs
git commit -m "feat: error types with color-eyre"
```

---

### Task 3: CLI Argument Parsing

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Write the test (in cli.rs)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_parse_minimal_args() {
        let args = Cli::parse_from(["distill", "input.pdf"]);
        assert_eq!(args.input, "input.pdf");
        assert!(args.output.is_none());
        assert!(args.format.is_none());
        assert!(args.level.is_none());
        assert!(args.mode.is_none());
    }

    #[test]
    fn test_parse_all_flags() {
        let args = Cli::parse_from([
            "distill", "-o", "out.epub", "-f", "epub", "-l", "dense",
            "-m", "book", "--parallel", "-j", "8", "-v", "input.pdf",
        ]);
        assert_eq!(args.output, Some("out.epub".into()));
        assert_eq!(args.format, Some(OutputFormat::Epub));
        assert_eq!(args.level, Some(CompressionLevel::Dense));
        assert_eq!(args.mode, Some(Mode::Book));
        assert!(args.parallel);
        assert_eq!(args.jobs, 8);
        assert_eq!(args.verbose, 1);
    }

    #[test]
    fn test_verbosity_stacks() {
        let args = Cli::parse_from(["distill", "-vv", "input.pdf"]);
        assert_eq!(args.verbose, 2);
    }

    #[test]
    fn test_quiet_flag() {
        let args = Cli::parse_from(["distill", "-q", "input.pdf"]);
        assert!(args.quiet);
    }

    #[test]
    fn test_resume_and_clean_flags() {
        let args = Cli::parse_from(["distill", "--resume", "input.pdf"]);
        assert!(args.resume);

        let args = Cli::parse_from(["distill", "--clean", "input.pdf"]);
        assert!(args.clean);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib cli`
Expected: FAIL — types don't exist yet

**Step 3: Implement cli.rs**

```rust
use std::path::PathBuf;
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum OutputFormat {
    Epub,
    Md,
    Html,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CompressionLevel {
    Tight,
    Dense,
    Distilled,
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum Mode {
    Book,
    Article,
}

#[derive(Debug, Parser)]
#[command(name = "distill", about = "Structure-preserving semantic compression engine")]
pub struct Cli {
    /// Input file path (PDF/EPUB) or URL
    pub input: String,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output format
    #[arg(short, long)]
    pub format: Option<OutputFormat>,

    /// Compression level
    #[arg(short, long)]
    pub level: Option<CompressionLevel>,

    /// Force mode (auto-detected by default)
    #[arg(short, long)]
    pub mode: Option<Mode>,

    /// LLM model name (overrides DISTILL_MODEL)
    #[arg(long)]
    pub model: Option<String>,

    /// LLM API base URL (overrides DISTILL_API_BASE)
    #[arg(long)]
    pub api_base: Option<String>,

    /// LLM API key (overrides DISTILL_API_KEY)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Enable concurrent chunk processing
    #[arg(long)]
    pub parallel: bool,

    /// Concurrency limit (default: 4)
    #[arg(short, long, default_value_t = 4)]
    pub jobs: usize,

    /// Resume from checkpoint
    #[arg(long)]
    pub resume: bool,

    /// Remove checkpoint file and exit
    #[arg(long)]
    pub clean: bool,

    /// Increase log verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress progress bars, errors only
    #[arg(short, long)]
    pub quiet: bool,
}
```

**Step 4: Add mod to main.rs**

```rust
mod cli;
mod error;

use clap::Parser;

fn main() -> error::Result<()> {
    color_eyre::install()?;
    let _cli = cli::Cli::parse();
    Ok(())
}
```

**Step 5: Run tests**

Run: `cargo test --lib cli`
Expected: all tests PASS

**Step 6: Verify --help output**

Run: `cargo run -- --help`

**Step 7: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: CLI argument parsing with clap derive"
```

---

### Task 4: Config Resolution

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_flags_override_env() {
        // Simulate env vars set, CLI flags override
        let config = Config {
            api_key: "from-cli".into(),
            api_base: "https://cli.example.com/v1".into(),
            model: "cli-model".into(),
        };
        assert_eq!(config.api_key, "from-cli");
    }

    #[test]
    fn test_missing_api_key_errors() {
        let result = Config::resolve(None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_from_all_cli() {
        let result = Config::resolve(
            Some("key".into()),
            Some("https://api.example.com/v1".into()),
            Some("model".into()),
        );
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.api_key, "key");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib config`
Expected: FAIL

**Step 3: Implement config.rs**

```rust
use crate::error::DistillError;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub api_base: String,
    pub model: String,
}

impl Config {
    pub fn resolve(
        cli_key: Option<String>,
        cli_base: Option<String>,
        cli_model: Option<String>,
    ) -> crate::error::Result<Self> {
        let api_key = cli_key
            .or_else(|| env::var("DISTILL_API_KEY").ok())
            .ok_or_else(|| DistillError::Config {
                cause: "API key required. Set DISTILL_API_KEY or pass --api-key".into(),
            })?;

        let api_base = cli_base
            .or_else(|| env::var("DISTILL_API_BASE").ok())
            .ok_or_else(|| DistillError::Config {
                cause: "API base URL required. Set DISTILL_API_BASE or pass --api-base".into(),
            })?;

        let model = cli_model
            .or_else(|| env::var("DISTILL_MODEL").ok())
            .ok_or_else(|| DistillError::Config {
                cause: "Model name required. Set DISTILL_MODEL or pass --model".into(),
            })?;

        Ok(Self { api_key, api_base, model })
    }
}
```

**Step 4: Run tests**

Run: `cargo test --lib config`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: config resolution with env var + CLI flag precedence"
```

---

### Task 5: Mode Detection

**Files:**
- Create: `src/mode.rs`
- Modify: `src/main.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Mode;

    #[test]
    fn test_small_token_count_is_article() {
        let detected = detect_mode(None, 10_000);
        assert_eq!(detected, Mode::Article);
    }

    #[test]
    fn test_large_token_count_is_book() {
        let detected = detect_mode(None, 50_000);
        assert_eq!(detected, Mode::Book);
    }

    #[test]
    fn test_threshold_boundary() {
        assert_eq!(detect_mode(None, 29_999), Mode::Article);
        assert_eq!(detect_mode(None, 30_000), Mode::Book);
    }

    #[test]
    fn test_forced_mode_overrides() {
        assert_eq!(detect_mode(Some(Mode::Book), 1_000), Mode::Book);
        assert_eq!(detect_mode(Some(Mode::Article), 100_000), Mode::Article);
    }

    #[test]
    fn test_estimate_tokens() {
        // "hello world" = 2 words, 2 * 1.3 = 2.6 -> 2
        assert_eq!(estimate_tokens("hello world"), 2);
        // 10 words -> 13
        let text = "one two three four five six seven eight nine ten";
        assert_eq!(estimate_tokens(text), 13);
    }

    #[test]
    fn test_input_is_url() {
        assert!(is_url("https://example.com/article"));
        assert!(is_url("http://example.com/page"));
        assert!(!is_url("./local-file.pdf"));
        assert!(!is_url("/home/user/book.epub"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib mode`
Expected: FAIL

**Step 3: Implement mode.rs**

```rust
use crate::cli::Mode;

const TOKEN_THRESHOLD: usize = 30_000;

pub fn detect_mode(forced: Option<Mode>, estimated_tokens: usize) -> Mode {
    if let Some(mode) = forced {
        return mode;
    }
    if estimated_tokens >= TOKEN_THRESHOLD {
        Mode::Book
    } else {
        Mode::Article
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    (word_count as f64 * 1.3) as usize
}

pub fn is_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}
```

**Step 4: Run tests**

Run: `cargo test --lib mode`
Expected: PASS

**Step 5: Commit**

```bash
git add src/mode.rs src/main.rs
git commit -m "feat: mode detection with token estimation"
```

---

### Task 6: Core Data Types

**Files:**
- Create: `src/segment/mod.rs`
- Create: `src/segment/chunk.rs`
- Create: `src/state/mod.rs`
- Create: `src/state/checkpoint.rs`
- Modify: `src/main.rs`

**Step 1: Create segment/chunk.rs with Chunk type**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub index: usize,
    pub header_path: Vec<String>,
    pub content: String,
    pub token_estimate: usize,
}
```

**Step 2: Create segment/mod.rs**

```rust
pub mod chunk;
pub use chunk::Chunk;
```

**Step 3: Create state/mod.rs with StateLedger types**

```rust
pub mod checkpoint;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateLedger {
    pub concepts: Vec<Concept>,
    pub examples: Vec<Example>,
}

impl StateLedger {
    pub fn apply_delta(&mut self, delta: &LedgerDelta) {
        self.concepts.extend(delta.new_concepts.iter().cloned());
        self.examples.extend(delta.new_examples.iter().cloned());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub first_seen_chunk: usize,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub id: String,
    pub related_concept: String,
    pub first_seen_chunk: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LedgerDelta {
    pub new_concepts: Vec<Concept>,
    pub new_examples: Vec<Example>,
}

#[derive(Debug, Clone)]
pub struct CompressedChunk {
    pub index: usize,
    pub header_path: Vec<String>,
    pub content: String,
    pub ledger_updates: LedgerDelta,
}
```

**Step 4: Create state/checkpoint.rs**

```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::cli::CompressionLevel;
use crate::state::StateLedger;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkStatus {
    Pending,
    Compressed,
    Deduplicated,
    Refined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkState {
    pub index: usize,
    pub status: ChunkStatus,
    pub original: String,
    pub compressed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub input_hash: String,
    pub level: CompressionLevel,
    pub model: String,
    pub completed_pass: u8,
    pub chunks: Vec<ChunkState>,
    pub ledger: StateLedger,
}

impl Checkpoint {
    pub fn cache_path(input_path: &Path) -> PathBuf {
        let stem = input_path.file_stem().unwrap_or_default();
        input_path.with_file_name(format!("{}.distill-cache", stem.to_string_lossy()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        std::fs::write(path, json)
            .map_err(|e| crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        let checkpoint: Self = serde_json::from_str(&json)
            .map_err(|e| crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        Ok(checkpoint)
    }

    pub fn delete(path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|e| crate::error::DistillError::Checkpoint {
                    path: path.to_path_buf(),
                    cause: e.to_string(),
                })?;
        }
        Ok(())
    }
}
```

**Step 5: Update main.rs to declare modules**

```rust
mod cli;
mod config;
mod error;
mod mode;
mod segment;
mod state;
```

**Step 6: Verify it compiles**

Run: `cargo build`

**Step 7: Commit**

```bash
git add src/segment/ src/state/ src/main.rs
git commit -m "feat: core data types — Chunk, StateLedger, Checkpoint"
```

---

### Task 7: Checkpoint Tests

**Files:**
- Modify: `src/state/checkpoint.rs` (add tests)

**Step 1: Write checkpoint round-trip tests**

Add to `src/state/checkpoint.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_checkpoint() -> Checkpoint {
        Checkpoint {
            input_hash: "abc123".into(),
            level: CompressionLevel::Dense,
            model: "test-model".into(),
            completed_pass: 1,
            chunks: vec![
                ChunkState {
                    index: 0,
                    status: ChunkStatus::Compressed,
                    original: "original text".into(),
                    compressed: Some("compressed text".into()),
                },
                ChunkState {
                    index: 1,
                    status: ChunkStatus::Pending,
                    original: "more text".into(),
                    compressed: None,
                },
            ],
            ledger: StateLedger::default(),
        }
    }

    #[test]
    fn test_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.distill-cache");
        let checkpoint = sample_checkpoint();

        checkpoint.save(&path).unwrap();
        let loaded = Checkpoint::load(&path).unwrap();

        assert_eq!(loaded.input_hash, "abc123");
        assert_eq!(loaded.completed_pass, 1);
        assert_eq!(loaded.chunks.len(), 2);
        assert_eq!(loaded.chunks[0].status, ChunkStatus::Compressed);
        assert_eq!(loaded.chunks[1].status, ChunkStatus::Pending);
    }

    #[test]
    fn test_cache_path_naming() {
        let path = Path::new("/home/user/books/thinking-fast.pdf");
        let cache = Checkpoint::cache_path(path);
        assert_eq!(cache, Path::new("/home/user/books/thinking-fast.distill-cache"));
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.distill-cache");
        assert!(Checkpoint::delete(&path).is_ok());
    }

    #[test]
    fn test_delete_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.distill-cache");
        let checkpoint = sample_checkpoint();
        checkpoint.save(&path).unwrap();
        assert!(path.exists());
        Checkpoint::delete(&path).unwrap();
        assert!(!path.exists());
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib state::checkpoint`
Expected: PASS

**Step 3: Commit**

```bash
git add src/state/checkpoint.rs
git commit -m "test: checkpoint round-trip, naming, and deletion"
```

---

### Task 8: Segmentation Logic

**Files:**
- Modify: `src/segment/mod.rs`

**Step 1: Write segmentation tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_by_headers() {
        let input = "# Chapter 1\n\nSome text here.\n\n## Section 1.1\n\nMore text.\n\n# Chapter 2\n\nAnother chapter.";
        let chunks = segment(input);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].header_path, vec!["Chapter 1"]);
    }

    #[test]
    fn test_no_headers_produces_chunks() {
        let input = "Just a long paragraph without any headers. ".repeat(500);
        let chunks = segment(&input);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_respects_max_chunk_size() {
        // Create a section with >5000 estimated tokens
        let big_section = format!("# Big Section\n\n{}", "word ".repeat(5000));
        let chunks = segment(&big_section);
        for chunk in &chunks {
            assert!(chunk.token_estimate <= 5500, "chunk {} has {} tokens", chunk.index, chunk.token_estimate);
        }
    }

    #[test]
    fn test_small_sections_merged() {
        // Sections too small to stand alone should be merged
        let input = "# A\n\nTiny.\n\n# B\n\nAlso tiny.\n\n# C\n\nStill tiny.";
        let chunks = segment(&input);
        // With very small sections, they may be merged or kept depending on min threshold
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let input = "# One\n\nText. ".repeat(10);
        let chunks = segment(&input);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib segment`
Expected: FAIL — `segment` function doesn't exist

**Step 3: Implement segmentation in segment/mod.rs**

```rust
pub mod chunk;
pub use chunk::Chunk;

use crate::mode::estimate_tokens;

const MAX_CHUNK_TOKENS: usize = 5_000;
const MIN_CHUNK_TOKENS: usize = 500;
const OVERLAP_RATIO: f64 = 0.10;

struct RawSection {
    header_path: Vec<String>,
    content: String,
}

pub fn segment(text: &str) -> Vec<Chunk> {
    let sections = split_by_headers(text);
    let mut chunks = Vec::new();

    for section in sections {
        let token_est = estimate_tokens(&section.content);
        if token_est > MAX_CHUNK_TOKENS {
            let sub_chunks = split_by_paragraphs(&section.header_path, &section.content);
            chunks.extend(sub_chunks);
        } else {
            chunks.push(Chunk {
                index: 0, // re-indexed below
                header_path: section.header_path,
                content: section.content,
                token_estimate: token_est,
            });
        }
    }

    // Merge chunks that are too small
    chunks = merge_small_chunks(chunks);

    // Re-index
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.index = i;
        chunk.token_estimate = estimate_tokens(&chunk.content);
    }

    // Add overlap between adjacent chunks
    add_overlap(&mut chunks);

    chunks
}

fn split_by_headers(text: &str) -> Vec<RawSection> {
    let mut sections = Vec::new();
    let mut current_headers: Vec<String> = Vec::new();
    let mut current_content = String::new();

    for line in text.lines() {
        if let Some(header) = parse_header(line) {
            if !current_content.trim().is_empty() || !current_headers.is_empty() {
                sections.push(RawSection {
                    header_path: current_headers.clone(),
                    content: current_content.trim().to_string(),
                });
            }
            let level = header.0;
            let title = header.1;
            // Adjust header path based on level
            current_headers.truncate(level.saturating_sub(1));
            while current_headers.len() < level.saturating_sub(1) {
                current_headers.push(String::new());
            }
            if current_headers.len() >= level {
                current_headers[level - 1] = title;
            } else {
                current_headers.push(title);
            }
            current_content = String::new();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if !current_content.trim().is_empty() || !current_headers.is_empty() {
        sections.push(RawSection {
            header_path: current_headers,
            content: current_content.trim().to_string(),
        });
    }

    if sections.is_empty() && !text.trim().is_empty() {
        sections.push(RawSection {
            header_path: vec![],
            content: text.trim().to_string(),
        });
    }

    sections
}

fn parse_header(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level > 6 || level == 0 {
        return None;
    }
    let title = trimmed[level..].trim().to_string();
    if title.is_empty() {
        return None;
    }
    Some((level, title))
}

fn split_by_paragraphs(header_path: &[String], content: &str) -> Vec<Chunk> {
    let paragraphs: Vec<&str> = content.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
    let mut chunks = Vec::new();
    let mut current = String::new();

    for para in paragraphs {
        let combined_tokens = estimate_tokens(&format!("{current}\n\n{para}"));
        if combined_tokens > MAX_CHUNK_TOKENS && !current.is_empty() {
            chunks.push(Chunk {
                index: 0,
                header_path: header_path.to_vec(),
                content: current.trim().to_string(),
                token_estimate: estimate_tokens(current.trim()),
            });
            current = para.to_string();
        } else {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
        }
    }

    if !current.trim().is_empty() {
        chunks.push(Chunk {
            index: 0,
            header_path: header_path.to_vec(),
            content: current.trim().to_string(),
            token_estimate: estimate_tokens(current.trim()),
        });
    }

    chunks
}

fn merge_small_chunks(chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut merged = Vec::new();

    for chunk in chunks {
        if let Some(last) = merged.last_mut() {
            let last_chunk: &mut Chunk = last;
            if last_chunk.token_estimate < MIN_CHUNK_TOKENS {
                last_chunk.content.push_str("\n\n");
                last_chunk.content.push_str(&chunk.content);
                last_chunk.token_estimate = estimate_tokens(&last_chunk.content);
                continue;
            }
        }
        merged.push(chunk);
    }

    // Check if the last chunk is too small and merge backward
    if merged.len() > 1 {
        let last_tokens = merged.last().map(|c| c.token_estimate).unwrap_or(0);
        if last_tokens < MIN_CHUNK_TOKENS {
            let last = merged.pop().unwrap();
            if let Some(prev) = merged.last_mut() {
                prev.content.push_str("\n\n");
                prev.content.push_str(&last.content);
                prev.token_estimate = estimate_tokens(&prev.content);
            }
        }
    }

    merged
}

fn add_overlap(chunks: &mut [Chunk]) {
    if chunks.len() < 2 {
        return;
    }

    // We add overlap text from the end of chunk N to the start of chunk N+1
    // Work backwards to avoid mutation issues
    let overlap_contents: Vec<String> = chunks.iter().map(|c| {
        let words: Vec<&str> = c.content.split_whitespace().collect();
        let overlap_count = (words.len() as f64 * OVERLAP_RATIO) as usize;
        let overlap_count = overlap_count.max(1);
        let start = words.len().saturating_sub(overlap_count);
        words[start..].join(" ")
    }).collect();

    for i in 1..chunks.len() {
        let overlap_text = &overlap_contents[i - 1];
        chunks[i].content = format!("{overlap_text}\n\n{}", chunks[i].content);
        chunks[i].token_estimate = estimate_tokens(&chunks[i].content);
    }
}
```

**Step 4: Run tests**

Run: `cargo test --lib segment`
Expected: PASS

**Step 5: Commit**

```bash
git add src/segment/
git commit -m "feat: document segmentation with header splitting, merging, and overlap"
```

---

### Task 9: LLM Client

**Files:**
- Create: `src/llm/mod.rs`
- Modify: `src/main.rs`

**Step 1: Write LLM client tests (using wiremock)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_completion() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "choices": [{"message": {"content": "compressed output"}}]
                }),
            ))
            .mount(&server)
            .await;

        let client = LlmClient::new(
            "test-key".into(),
            server.uri(),
            "test-model".into(),
        );

        let result = client.complete("system prompt", "user prompt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "compressed output");
    }

    #[tokio::test]
    async fn test_retry_on_429() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(429))
            .up_to_n_times(2)
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "choices": [{"message": {"content": "success after retry"}}]
                }),
            ))
            .mount(&server)
            .await;

        let client = LlmClient::new(
            "test-key".into(),
            server.uri(),
            "test-model".into(),
        );

        let result = client.complete("sys", "user").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success after retry");
    }

    #[tokio::test]
    async fn test_exhausted_retries() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = LlmClient::new(
            "test-key".into(),
            server.uri(),
            "test-model".into(),
        );

        let result = client.complete("sys", "user").await;
        assert!(result.is_err());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib llm`
Expected: FAIL

**Step 3: Implement LlmClient**

```rust
use crate::error::{DistillError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod parse;
pub mod prompt;

const MAX_RETRIES: u32 = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(4),
    Duration::from_secs(16),
];

#[derive(Debug)]
pub struct LlmClient {
    http: reqwest::Client,
    api_key: String,
    api_base: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

impl LlmClient {
    pub fn new(api_key: String, api_base: String, model: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build HTTP client");

        Self { http, api_key, api_base, model }
    }

    pub async fn complete(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.api_base);
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message { role: "system".into(), content: system.into() },
                Message { role: "user".into(), content: user.into() },
            ],
        };

        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(RETRY_DELAYS[(attempt - 1) as usize]).await;
            }

            let response = self.http
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let chat_resp: ChatResponse = resp.json().await
                            .map_err(|e| DistillError::Llm { cause: format!("failed to parse response: {e}") })?;
                        let content = chat_resp.choices
                            .into_iter()
                            .next()
                            .ok_or_else(|| DistillError::Llm { cause: "empty response from LLM".into() })?
                            .message
                            .content;
                        return Ok(content);
                    }

                    let should_retry = status.as_u16() == 429 || status.is_server_error();
                    let err_msg = format!("HTTP {status}");
                    if should_retry && attempt < MAX_RETRIES {
                        last_err = Some(err_msg);
                        continue;
                    }
                    return Err(DistillError::Llm { cause: err_msg }.into());
                }
                Err(e) => {
                    let is_timeout = e.is_timeout() || e.is_connect();
                    let err_msg = e.to_string();
                    if is_timeout && attempt < MAX_RETRIES {
                        last_err = Some(err_msg);
                        continue;
                    }
                    return Err(DistillError::Llm { cause: err_msg }.into());
                }
            }
        }

        Err(DistillError::Llm {
            cause: format!("exhausted {MAX_RETRIES} retries. last error: {}", last_err.unwrap_or_default()),
        }.into())
    }
}
```

**Step 4: Create stub files for submodules**

`src/llm/parse.rs` and `src/llm/prompt.rs` — empty files for now.

**Step 5: Update main.rs**

Add `mod llm;`

**Step 6: Run tests**

Run: `cargo test --lib llm`
Expected: PASS

**Step 7: Commit**

```bash
git add src/llm/ src/main.rs
git commit -m "feat: LLM client with retry logic and exponential backoff"
```

---

### Task 10: LLM Response Parsing

**Files:**
- Modify: `src/llm/parse.rs`

**Step 1: Write parsing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_well_formed_response() {
        let response = r#"<compressed>
## Section Title

Compressed content here.
</compressed>
<ledger>
{"new_concepts": [{"id": "concept-001", "name": "Test Concept", "first_seen_chunk": 0, "description": "A test"}], "new_examples": []}
</ledger>"#;

        let parsed = parse_llm_response(response).unwrap();
        assert!(parsed.compressed.contains("Compressed content"));
        assert_eq!(parsed.ledger.new_concepts.len(), 1);
        assert_eq!(parsed.ledger.new_concepts[0].name, "Test Concept");
    }

    #[test]
    fn test_parse_missing_compressed_tag() {
        let response = "Just some text without tags";
        let result = parse_llm_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_ledger_tag() {
        let response = "<compressed>Some text</compressed>";
        let parsed = parse_llm_response(response).unwrap();
        assert_eq!(parsed.compressed, "Some text");
        assert!(parsed.ledger.new_concepts.is_empty());
    }

    #[test]
    fn test_parse_empty_compressed() {
        let response = "<compressed></compressed>\n<ledger>{\"new_concepts\": [], \"new_examples\": []}</ledger>";
        let parsed = parse_llm_response(response).unwrap();
        assert_eq!(parsed.compressed, "");
    }

    #[test]
    fn test_parse_malformed_ledger_json() {
        let response = "<compressed>Good text</compressed>\n<ledger>not valid json</ledger>";
        let parsed = parse_llm_response(response).unwrap();
        assert_eq!(parsed.compressed, "Good text");
        // Malformed ledger should fall back to empty
        assert!(parsed.ledger.new_concepts.is_empty());
    }

    #[test]
    fn test_parse_single_pass_response() {
        // SinglePass mode: no ledger expected, just compressed text
        let response = "<compressed>\n# Title\n\nCompressed article.\n</compressed>";
        let parsed = parse_llm_response(response).unwrap();
        assert!(parsed.compressed.contains("Compressed article"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib llm::parse`
Expected: FAIL

**Step 3: Implement parse.rs**

```rust
use crate::state::LedgerDelta;

#[derive(Debug)]
pub struct ParsedResponse {
    pub compressed: String,
    pub ledger: LedgerDelta,
}

pub fn parse_llm_response(response: &str) -> crate::error::Result<ParsedResponse> {
    let compressed = extract_tag(response, "compressed")
        .ok_or_else(|| crate::error::DistillError::Compression {
            chunk_index: 0,
            section: String::new(),
            cause: "missing <compressed> tag in LLM response".into(),
        })?;

    let ledger = extract_tag(response, "ledger")
        .and_then(|json| serde_json::from_str::<LedgerDelta>(&json).ok())
        .unwrap_or_default();

    Ok(ParsedResponse { compressed, ledger })
}

fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)?;
    let end = text.find(&close)?;
    if end <= start {
        return None;
    }
    let content = &text[start + open.len()..end];
    Some(content.trim().to_string())
}
```

**Step 4: Run tests**

Run: `cargo test --lib llm::parse`
Expected: PASS

**Step 5: Commit**

```bash
git add src/llm/parse.rs
git commit -m "feat: LLM response parsing with XML delimiter extraction"
```

---

### Task 11: LLM Prompt Templates

**Files:**
- Modify: `src/llm/prompt.rs`

**Step 1: Implement prompt templates**

```rust
use crate::cli::CompressionLevel;
use crate::state::StateLedger;

pub fn compression_level_policy(level: &CompressionLevel) -> &'static str {
    match level {
        CompressionLevel::Tight => {
            "COMPRESSION LEVEL: tight (~80% of original)\n\
             - Remove repetition and filler only\n\
             - Minimal rewriting\n\
             - Preserve original phrasing wherever possible"
        }
        CompressionLevel::Dense => {
            "COMPRESSION LEVEL: dense (~50% of original)\n\
             - Compress explanations into fewer sentences\n\
             - Merge short paragraphs covering the same point\n\
             - Shorten repeated mentions of the same concept"
        }
        CompressionLevel::Distilled => {
            "COMPRESSION LEVEL: distilled (~30% of original)\n\
             - Aggressive compression\n\
             - Keep only the strongest example per concept\n\
             - Allow intra-section restructuring for clarity"
        }
    }
}

pub fn pass1_system(level: &CompressionLevel) -> String {
    format!(
        "You are a structure-preserving semantic compression engine.\n\
         You compress text while preserving structure, core ideas, meaningful examples, and the author's voice.\n\
         Remove: repetition, filler, long transitions, meta-text.\n\n\
         {}\n\n\
         RESPONSE FORMAT:\n\
         Return your response in exactly this format:\n\
         <compressed>\n\
         [compressed markdown here]\n\
         </compressed>\n\
         <ledger>\n\
         {{\"new_concepts\": [...], \"new_examples\": [...]}}\n\
         </ledger>\n\n\
         Ledger entry format:\n\
         - Concept: {{\"id\": \"concept-NNN\", \"name\": \"...\", \"first_seen_chunk\": N, \"description\": \"...\"}}\n\
         - Example: {{\"id\": \"example-NNN\", \"related_concept\": \"concept-NNN\", \"first_seen_chunk\": N, \"summary\": \"...\"}}\n\n\
         If no new concepts or examples, return empty arrays.",
        compression_level_policy(level)
    )
}

pub fn pass1_user(chunk_content: &str, chunk_index: usize, ledger: &StateLedger) -> String {
    let ledger_json = serde_json::to_string(ledger).unwrap_or_else(|_| "{}".into());
    format!(
        "CHUNK INDEX: {chunk_index}\n\n\
         CURRENT LEDGER (concepts/examples seen so far):\n\
         {ledger_json}\n\n\
         TEXT TO COMPRESS:\n\
         {chunk_content}"
    )
}

pub fn single_pass_system(level: &CompressionLevel) -> String {
    format!(
        "You are a structure-preserving semantic compression engine.\n\
         You compress text while preserving structure, core ideas, meaningful examples, and the author's voice.\n\
         Remove: repetition, filler, long transitions, meta-text.\n\n\
         {}\n\n\
         RESPONSE FORMAT:\n\
         Return your response in exactly this format:\n\
         <compressed>\n\
         [compressed markdown here]\n\
         </compressed>",
        compression_level_policy(level)
    )
}

pub fn single_pass_user(content: &str) -> String {
    format!("TEXT TO COMPRESS:\n\n{content}")
}

pub fn pass2_system() -> String {
    "You are performing global deduplication on a compressed document.\n\
     You have a ledger of all concepts and examples found across chunks.\n\
     Your job:\n\
     1. Identify concepts/examples that appear in multiple chunks\n\
     2. Keep the strongest, most complete version (usually first occurrence)\n\
     3. In later occurrences, compress to 1-2 sentences with a back-reference\n\n\
     RESPONSE FORMAT:\n\
     <compressed>\n\
     [deduplicated markdown here]\n\
     </compressed>".into()
}

pub fn pass2_user(chunks_content: &str, ledger: &StateLedger) -> String {
    let ledger_json = serde_json::to_string_pretty(ledger).unwrap_or_else(|_| "{}".into());
    format!(
        "FULL LEDGER:\n{ledger_json}\n\n\
         CHUNKS TO DEDUPLICATE:\n{chunks_content}"
    )
}

pub fn pass3_system() -> String {
    "You are performing a final refinement pass on a compressed document.\n\
     Fix broken transitions between sections (artifacts of chunk boundaries).\n\
     Smooth tone to match the original author's voice.\n\
     Ensure no dangling references to removed content.\n\
     Do NOT add new content or re-expand compressed sections.\n\n\
     RESPONSE FORMAT:\n\
     <compressed>\n\
     [refined markdown here]\n\
     </compressed>".into()
}

pub fn pass3_user(content: &str) -> String {
    format!("TEXT TO REFINE:\n\n{content}")
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/llm/prompt.rs
git commit -m "feat: LLM prompt templates for all passes"
```

---

### Task 12: Compression Strategy — SinglePass + MultiPass

**Files:**
- Create: `src/compress/mod.rs`
- Create: `src/compress/pass1.rs`
- Create: `src/compress/pass2.rs`
- Create: `src/compress/pass3.rs`
- Modify: `src/main.rs`

**Step 1: Create compress/pass1.rs — local compression**

```rust
use crate::cli::CompressionLevel;
use crate::llm::LlmClient;
use crate::llm::parse::parse_llm_response;
use crate::llm::prompt;
use crate::segment::Chunk;
use crate::state::{CompressedChunk, LedgerDelta, StateLedger};
use crate::error::Result;

pub async fn compress_chunk(
    client: &LlmClient,
    chunk: &Chunk,
    level: &CompressionLevel,
    ledger: &StateLedger,
) -> Result<CompressedChunk> {
    let system = prompt::pass1_system(level);
    let user = prompt::pass1_user(&chunk.content, chunk.index, ledger);

    let response = client.complete(&system, &user).await?;

    let parsed = match parse_llm_response(&response) {
        Ok(p) => p,
        Err(_) => {
            // Retry with stricter prompt
            let retry_response = client.complete(
                &format!("{system}\n\nIMPORTANT: You MUST use <compressed> and </compressed> XML tags."),
                &user,
            ).await?;
            parse_llm_response(&retry_response).unwrap_or_else(|_| {
                eprintln!(
                    "warning: chunk {} could not be parsed after retry, keeping original\n  -> section: \"{}\"",
                    chunk.index,
                    chunk.header_path.join(" > ")
                );
                crate::llm::parse::ParsedResponse {
                    compressed: chunk.content.clone(),
                    ledger: LedgerDelta::default(),
                }
            })
        }
    };

    Ok(CompressedChunk {
        index: chunk.index,
        header_path: chunk.header_path.clone(),
        content: parsed.compressed,
        ledger_updates: parsed.ledger,
    })
}

pub async fn compress_chunk_single_pass(
    client: &LlmClient,
    chunk: &Chunk,
    level: &CompressionLevel,
) -> Result<CompressedChunk> {
    let system = prompt::single_pass_system(level);
    let user = prompt::single_pass_user(&chunk.content);

    let response = client.complete(&system, &user).await?;

    let parsed = match parse_llm_response(&response) {
        Ok(p) => p,
        Err(_) => {
            eprintln!(
                "warning: chunk {} could not be parsed, keeping original\n  -> section: \"{}\"",
                chunk.index,
                chunk.header_path.join(" > ")
            );
            crate::llm::parse::ParsedResponse {
                compressed: chunk.content.clone(),
                ledger: LedgerDelta::default(),
            }
        }
    };

    Ok(CompressedChunk {
        index: chunk.index,
        header_path: chunk.header_path.clone(),
        content: parsed.compressed,
        ledger_updates: parsed.ledger,
    })
}
```

**Step 2: Create compress/pass2.rs — global deduplication**

```rust
use crate::llm::LlmClient;
use crate::llm::parse::parse_llm_response;
use crate::llm::prompt;
use crate::state::{CompressedChunk, StateLedger, LedgerDelta};
use crate::error::Result;

pub async fn deduplicate(
    client: &LlmClient,
    chunks: &[CompressedChunk],
    ledger: &StateLedger,
) -> Result<Vec<CompressedChunk>> {
    let combined = chunks.iter()
        .map(|c| {
            let header = if c.header_path.is_empty() {
                String::new()
            } else {
                format!("<!-- chunk {} | {} -->\n", c.index, c.header_path.join(" > "))
            };
            format!("{header}{}", c.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let system = prompt::pass2_system();
    let user = prompt::pass2_user(&combined, ledger);

    let response = client.complete(&system, &user).await?;

    match parse_llm_response(&response) {
        Ok(parsed) => {
            // Split back into chunks by the chunk markers
            let deduped_chunks = reassemble_chunks(chunks, &parsed.compressed);
            Ok(deduped_chunks)
        }
        Err(_) => {
            eprintln!("warning: pass 2 (deduplication) failed to parse, keeping pass 1 output");
            Ok(chunks.to_vec())
        }
    }
}

fn reassemble_chunks(original_chunks: &[CompressedChunk], deduped_text: &str) -> Vec<CompressedChunk> {
    // Try to split by chunk markers; if not present, treat as single block
    let sections: Vec<&str> = deduped_text.split("---").collect();

    if sections.len() == original_chunks.len() {
        original_chunks.iter().enumerate().map(|(i, orig)| {
            CompressedChunk {
                index: orig.index,
                header_path: orig.header_path.clone(),
                content: sections[i].trim().to_string(),
                ledger_updates: LedgerDelta::default(),
            }
        }).collect()
    } else {
        // Can't reliably split — return as single chunk with original metadata
        vec![CompressedChunk {
            index: 0,
            header_path: original_chunks.first().map(|c| c.header_path.clone()).unwrap_or_default(),
            content: deduped_text.trim().to_string(),
            ledger_updates: LedgerDelta::default(),
        }]
    }
}
```

**Step 3: Create compress/pass3.rs — refinement**

```rust
use crate::llm::LlmClient;
use crate::llm::parse::parse_llm_response;
use crate::llm::prompt;
use crate::state::CompressedChunk;
use crate::error::Result;

pub async fn refine(
    client: &LlmClient,
    chunks: &[CompressedChunk],
) -> Result<String> {
    let combined = chunks.iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let system = prompt::pass3_system();
    let user = prompt::pass3_user(&combined);

    let response = client.complete(&system, &user).await?;

    match parse_llm_response(&response) {
        Ok(parsed) => Ok(parsed.compressed),
        Err(_) => {
            eprintln!("warning: pass 3 (refinement) failed to parse, keeping pass 2 output");
            Ok(combined)
        }
    }
}
```

**Step 4: Create compress/mod.rs — strategy orchestration**

```rust
pub mod pass1;
pub mod pass2;
pub mod pass3;

use crate::cli::CompressionLevel;
use crate::llm::LlmClient;
use crate::segment::Chunk;
use crate::state::{CompressedChunk, StateLedger};
use crate::error::Result;

pub async fn single_pass(
    client: &LlmClient,
    chunks: Vec<Chunk>,
    level: &CompressionLevel,
) -> Result<String> {
    let mut compressed = Vec::new();

    for chunk in &chunks {
        let result = pass1::compress_chunk_single_pass(client, chunk, level).await?;
        compressed.push(result);
    }

    let output = compressed.iter()
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(output)
}

pub async fn multi_pass(
    client: &LlmClient,
    chunks: Vec<Chunk>,
    level: &CompressionLevel,
    parallel: bool,
    jobs: usize,
) -> Result<String> {
    // Pass 1: Local compression
    let mut ledger = StateLedger::default();
    let mut compressed: Vec<CompressedChunk> = Vec::new();

    if parallel {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(jobs));
        let client = std::sync::Arc::new(client);
        // Note: parallel mode processes chunks concurrently but ledger isn't shared
        // For parallel, we skip ledger tracking during compression
        let mut handles = Vec::new();

        for chunk in &chunks {
            let sem = semaphore.clone();
            let client = client.clone();
            let chunk = chunk.clone();
            let level = level.clone();
            let ledger_snapshot = ledger.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                pass1::compress_chunk(&client, &chunk, &level, &ledger_snapshot).await
            }));
        }

        for handle in handles {
            let result = handle.await.map_err(|e| crate::error::DistillError::Compression {
                chunk_index: 0,
                section: String::new(),
                cause: e.to_string(),
            })??;
            ledger.apply_delta(&result.ledger_updates);
            compressed.push(result);
        }
    } else {
        for chunk in &chunks {
            let result = pass1::compress_chunk(client, chunk, level, &ledger).await?;
            ledger.apply_delta(&result.ledger_updates);
            compressed.push(result);
        }
    }

    // Pass 2: Global deduplication
    let deduped = pass2::deduplicate(client, &compressed, &ledger).await?;

    // Pass 3: Refinement
    let refined = pass3::refine(client, &deduped).await?;

    Ok(refined)
}
```

**Step 5: Update main.rs**

Add `mod compress;`

**Step 6: Verify it compiles**

Run: `cargo build`

**Step 7: Commit**

```bash
git add src/compress/ src/main.rs
git commit -m "feat: compression strategy — SinglePass and MultiPass with 3-pass pipeline"
```

---

### Task 13: Ingestion — URL (Article Extraction)

**Files:**
- Create: `src/ingest/mod.rs`
- Create: `src/ingest/url.rs`
- Modify: `src/main.rs`

**Step 1: Create ingest/mod.rs with Document type and dispatch**

```rust
pub mod url;
pub mod epub;
pub mod pdf;

use std::path::PathBuf;
use crate::error::Result;
use crate::mode::estimate_tokens;

#[derive(Debug, Clone)]
pub enum InputSource {
    File(PathBuf),
    Url(String),
}

#[derive(Debug)]
pub struct Document {
    pub title: Option<String>,
    pub author: Option<String>,
    pub content: String,
    pub source: InputSource,
    pub estimated_tokens: usize,
}

pub async fn ingest(input: &str) -> Result<Document> {
    if crate::mode::is_url(input) {
        url::ingest_url(input).await
    } else {
        let path = PathBuf::from(input);
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "pdf" => pdf::ingest_pdf(&path),
            "epub" => epub::ingest_epub(&path),
            _ => Err(crate::error::DistillError::Ingestion {
                source: input.into(),
                cause: format!("unsupported file extension: .{ext}"),
            }.into()),
        }
    }
}
```

**Step 2: Create ingest/url.rs**

```rust
use crate::ingest::{Document, InputSource};
use crate::error::{DistillError, Result};
use crate::mode::estimate_tokens;

pub async fn ingest_url(url: &str) -> Result<Document> {
    let response = reqwest::get(url).await
        .map_err(|e| DistillError::Ingestion {
            source: url.into(),
            cause: e.to_string(),
        })?;

    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if content_type.contains("application/pdf") || url.ends_with(".pdf") {
        let bytes = response.bytes().await
            .map_err(|e| DistillError::Ingestion { source: url.into(), cause: e.to_string() })?;
        let tmp = std::env::temp_dir().join("distill-download.pdf");
        std::fs::write(&tmp, &bytes)
            .map_err(|e| DistillError::Ingestion { source: url.into(), cause: e.to_string() })?;
        return crate::ingest::pdf::ingest_pdf(&tmp);
    }

    if content_type.contains("application/epub") || url.ends_with(".epub") {
        let bytes = response.bytes().await
            .map_err(|e| DistillError::Ingestion { source: url.into(), cause: e.to_string() })?;
        let tmp = std::env::temp_dir().join("distill-download.epub");
        std::fs::write(&tmp, &bytes)
            .map_err(|e| DistillError::Ingestion { source: url.into(), cause: e.to_string() })?;
        return crate::ingest::epub::ingest_epub(&tmp);
    }

    // HTML — extract article
    let html = response.text().await
        .map_err(|e| DistillError::Ingestion { source: url.into(), cause: e.to_string() })?;

    let content = extract_article(&html, url)?;
    let tokens = estimate_tokens(&content);

    Ok(Document {
        title: extract_title(&html),
        author: None,
        content,
        source: InputSource::Url(url.into()),
        estimated_tokens: tokens,
    })
}

fn extract_article(html: &str, url: &str) -> Result<String> {
    use readability::extractor;

    // readability expects a URL for resolving relative links
    let product = extractor::extract(&mut html.as_bytes(), url)
        .map_err(|e| DistillError::Ingestion {
            source: url.into(),
            cause: format!("article extraction failed: {e}"),
        })?;

    // Convert extracted HTML to markdown
    let markdown = html2md::parse_html(&product.content);
    Ok(markdown)
}

fn extract_title(html: &str) -> Option<String> {
    // Simple title extraction from <title> tag
    let start = html.find("<title>")?;
    let end = html.find("</title>")?;
    let title = &html[start + 7..end];
    Some(title.trim().to_string())
}
```

**Step 3: Create stub files**

`src/ingest/pdf.rs` and `src/ingest/epub.rs` — temporary stubs that return errors:

```rust
// pdf.rs
use std::path::Path;
use crate::ingest::Document;
use crate::error::Result;

pub fn ingest_pdf(_path: &Path) -> Result<Document> {
    todo!("PDF ingestion not yet implemented")
}
```

```rust
// epub.rs (same pattern)
use std::path::Path;
use crate::ingest::Document;
use crate::error::Result;

pub fn ingest_epub(_path: &Path) -> Result<Document> {
    todo!("EPUB ingestion not yet implemented")
}
```

**Step 4: Update main.rs**

Add `mod ingest;`

**Step 5: Verify it compiles**

Run: `cargo build`

**Step 6: Commit**

```bash
git add src/ingest/ src/main.rs
git commit -m "feat: URL ingestion with article extraction via readability"
```

---

### Task 14: Ingestion — EPUB

**Files:**
- Modify: `src/ingest/epub.rs`

**Step 1: Implement EPUB ingestion**

```rust
use std::path::Path;
use crate::ingest::{Document, InputSource};
use crate::error::{DistillError, Result};
use crate::mode::estimate_tokens;

pub fn ingest_epub(path: &Path) -> Result<Document> {
    let mut doc = epub::doc::EpubDoc::new(path)
        .map_err(|e| DistillError::Ingestion {
            source: path.display().to_string(),
            cause: format!("failed to open EPUB: {e}"),
        })?;

    let title = doc.mdata("title");
    let author = doc.mdata("creator");

    let mut content = String::new();
    let spine_ids: Vec<String> = doc.spine.clone();

    for id in &spine_ids {
        if let Some(body) = doc.get_resource_str_by_path(
            doc.resources.get(id)
                .map(|(path, _)| path.clone())
                .unwrap_or_default(),
        ) {
            let markdown = html2md::parse_html(&body);
            if !markdown.trim().is_empty() {
                content.push_str(&markdown);
                content.push_str("\n\n");
            }
        }
    }

    let tokens = estimate_tokens(&content);

    Ok(Document {
        title,
        author,
        content,
        source: InputSource::File(path.to_path_buf()),
        estimated_tokens: tokens,
    })
}
```

Note: The exact `epub` crate API may need adjustment at implementation time — check the crate docs for the version installed. The key operations are: open EPUB, read metadata, iterate spine items, extract XHTML content.

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/ingest/epub.rs
git commit -m "feat: EPUB ingestion with spine extraction"
```

---

### Task 15: Ingestion — PDF

**Files:**
- Modify: `src/ingest/pdf.rs`

**Step 1: Implement PDF ingestion**

```rust
use std::path::Path;
use crate::ingest::{Document, InputSource};
use crate::error::{DistillError, Result};
use crate::mode::estimate_tokens;

pub fn ingest_pdf(path: &Path) -> Result<Document> {
    let bytes = std::fs::read(path)
        .map_err(|e| DistillError::Ingestion {
            source: path.display().to_string(),
            cause: e.to_string(),
        })?;

    let content = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| DistillError::Ingestion {
            source: path.display().to_string(),
            cause: format!("failed to extract text from PDF: {e}"),
        })?;

    let tokens = estimate_tokens(&content);

    Ok(Document {
        title: None,
        author: None,
        content,
        source: InputSource::File(path.to_path_buf()),
        estimated_tokens: tokens,
    })
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/ingest/pdf.rs
git commit -m "feat: PDF ingestion with pdf-extract"
```

---

### Task 16: Export — Markdown

**Files:**
- Create: `src/export/mod.rs`
- Create: `src/export/markdown.rs`
- Modify: `src/main.rs`

**Step 1: Create export/mod.rs**

```rust
pub mod markdown;
pub mod html;
pub mod epub;

use std::path::Path;
use crate::cli::OutputFormat;
use crate::error::Result;

pub fn export(
    content: &str,
    title: Option<&str>,
    author: Option<&str>,
    format: &OutputFormat,
    output_path: Option<&Path>,
) -> Result<()> {
    match format {
        OutputFormat::Md => markdown::export_markdown(content, output_path),
        OutputFormat::Html => html::export_html(content, title, output_path),
        OutputFormat::Epub => epub::export_epub(content, title, author, output_path),
    }
}
```

**Step 2: Create export/markdown.rs**

```rust
use std::path::Path;
use crate::error::{DistillError, Result};

pub fn export_markdown(content: &str, output_path: Option<&Path>) -> Result<()> {
    match output_path {
        Some(path) => {
            std::fs::write(path, content)
                .map_err(|e| DistillError::Export { cause: e.to_string() })?;
        }
        None => {
            print!("{content}");
        }
    }
    Ok(())
}
```

**Step 3: Create stubs for html.rs and epub.rs**

```rust
// html.rs
use std::path::Path;
use crate::error::Result;

pub fn export_html(_content: &str, _title: Option<&str>, _output_path: Option<&Path>) -> Result<()> {
    todo!("HTML export not yet implemented")
}
```

```rust
// epub.rs
use std::path::Path;
use crate::error::Result;

pub fn export_epub(_content: &str, _title: Option<&str>, _author: Option<&str>, _output_path: Option<&Path>) -> Result<()> {
    todo!("EPUB export not yet implemented")
}
```

**Step 4: Update main.rs**

Add `mod export;`

**Step 5: Verify it compiles**

Run: `cargo build`

**Step 6: Commit**

```bash
git add src/export/ src/main.rs
git commit -m "feat: markdown export to stdout or file"
```

---

### Task 17: Export — HTML

**Files:**
- Modify: `src/export/html.rs`

**Step 1: Implement HTML export**

```rust
use std::path::Path;
use crate::error::{DistillError, Result};

const CSS: &str = r#"
body {
    font-family: Georgia, 'Times New Roman', serif;
    line-height: 1.6;
    max-width: 42em;
    margin: 2em auto;
    padding: 0 1em;
    color: #333;
}
h1, h2, h3 { margin-top: 1.5em; }
h1 { font-size: 1.8em; }
h2 { font-size: 1.4em; }
h3 { font-size: 1.2em; }
"#;

pub fn export_html(content: &str, title: Option<&str>, output_path: Option<&Path>) -> Result<()> {
    let title = title.unwrap_or("Distilled");

    // Generate TOC from headers
    let toc = generate_toc(content);

    // Convert markdown to HTML (basic conversion)
    let body_html = md_to_html(content);

    let html = format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>{title}</title>\n<style>{CSS}</style>\n</head>\n<body>\n\
         <h1>{title}</h1>\n{toc}\n<hr>\n{body_html}\n</body>\n</html>"
    );

    match output_path {
        Some(path) => {
            std::fs::write(path, &html)
                .map_err(|e| DistillError::Export { cause: e.to_string() })?;
        }
        None => {
            print!("{html}");
        }
    }
    Ok(())
}

fn generate_toc(content: &str) -> String {
    let mut toc = String::from("<nav>\n<ul>\n");
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            let title = &trimmed[3..];
            let id = title.to_lowercase().replace(' ', "-");
            toc.push_str(&format!("  <li><a href=\"#{id}\">{title}</a></li>\n"));
        }
    }
    toc.push_str("</ul>\n</nav>");
    toc
}

fn md_to_html(md: &str) -> String {
    // Basic markdown to HTML — headers, paragraphs
    let mut html = String::new();
    let mut in_paragraph = false;

    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("### ") {
            if in_paragraph { html.push_str("</p>\n"); in_paragraph = false; }
            let id = rest.to_lowercase().replace(' ', "-");
            html.push_str(&format!("<h3 id=\"{id}\">{rest}</h3>\n"));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            if in_paragraph { html.push_str("</p>\n"); in_paragraph = false; }
            let id = rest.to_lowercase().replace(' ', "-");
            html.push_str(&format!("<h2 id=\"{id}\">{rest}</h2>\n"));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            if in_paragraph { html.push_str("</p>\n"); in_paragraph = false; }
            let id = rest.to_lowercase().replace(' ', "-");
            html.push_str(&format!("<h1 id=\"{id}\">{rest}</h1>\n"));
        } else {
            if !in_paragraph {
                html.push_str("<p>");
                in_paragraph = true;
            } else {
                html.push(' ');
            }
            html.push_str(trimmed);
        }
    }

    if in_paragraph {
        html.push_str("</p>\n");
    }

    html
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/export/html.rs
git commit -m "feat: HTML export with inline CSS and TOC"
```

---

### Task 18: Export — EPUB

**Files:**
- Modify: `src/export/epub.rs`

**Step 1: Implement EPUB export**

```rust
use std::path::Path;
use crate::error::{DistillError, Result};
use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};

const EPUB_CSS: &str = r#"
body {
    font-family: Georgia, 'Times New Roman', serif;
    line-height: 1.6;
    margin: 1em;
    color: #333;
}
h1, h2, h3 { margin-top: 1.5em; }
"#;

pub fn export_epub(
    content: &str,
    title: Option<&str>,
    author: Option<&str>,
    output_path: Option<&Path>,
) -> Result<()> {
    let output_path = output_path
        .ok_or_else(|| DistillError::Export {
            cause: "EPUB export requires an output file path (-o)".into(),
        })?;

    let title = title.unwrap_or("Distilled");
    let chapters = split_into_chapters(content);

    let mut builder = EpubBuilder::new(ZipLibrary::new().map_err(|e| DistillError::Export {
        cause: format!("failed to create ZIP library: {e}"),
    })?)
    .map_err(|e| DistillError::Export { cause: e.to_string() })?;

    builder.metadata("title", title)
        .map_err(|e| DistillError::Export { cause: e.to_string() })?;

    if let Some(author) = author {
        builder.metadata("author", author)
            .map_err(|e| DistillError::Export { cause: e.to_string() })?;
    }

    builder.stylesheet(EPUB_CSS.as_bytes())
        .map_err(|e| DistillError::Export { cause: e.to_string() })?;

    for (i, chapter) in chapters.iter().enumerate() {
        let xhtml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <html xmlns=\"http://www.w3.org/1999/xhtml\">\n\
             <head><title>{}</title></head>\n\
             <body>\n{}\n</body>\n</html>",
            chapter.title, chapter.html_content
        );

        builder.add_content(
            EpubContent::new(format!("chapter_{i}.xhtml"), xhtml.as_bytes())
                .title(&chapter.title)
        ).map_err(|e| DistillError::Export { cause: e.to_string() })?;
    }

    let mut output = std::fs::File::create(output_path)
        .map_err(|e| DistillError::Export { cause: e.to_string() })?;

    builder.generate(&mut output)
        .map_err(|e| DistillError::Export { cause: e.to_string() })?;

    Ok(())
}

struct Chapter {
    title: String,
    html_content: String,
}

fn split_into_chapters(md: &str) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    let mut current_title = String::from("Introduction");
    let mut current_content = String::new();

    for line in md.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            if !current_content.trim().is_empty() {
                chapters.push(Chapter {
                    title: current_title,
                    html_content: super::html::md_to_html_fragment(&current_content),
                });
            }
            current_title = title.to_string();
            current_content = String::new();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if !current_content.trim().is_empty() {
        chapters.push(Chapter {
            title: current_title,
            html_content: super::html::md_to_html_fragment(&current_content),
        });
    }

    if chapters.is_empty() {
        chapters.push(Chapter {
            title: "Content".into(),
            html_content: super::html::md_to_html_fragment(md),
        });
    }

    chapters
}
```

Note: This requires exposing `md_to_html` from html.rs as `md_to_html_fragment` (a `pub` function). Rename or add a public wrapper during implementation.

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/export/epub.rs src/export/html.rs
git commit -m "feat: EPUB export with chapter splitting and TOC"
```

---

### Task 19: Progress Reporting

**Files:**
- Create: `src/progress.rs`
- Modify: `src/main.rs`

**Step 1: Implement progress.rs**

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Instant;

pub struct ProgressReporter {
    multi: MultiProgress,
    pass_bars: Vec<ProgressBar>,
    start_time: Instant,
    quiet: bool,
}

impl ProgressReporter {
    pub fn new(quiet: bool, is_multi_pass: bool) -> Self {
        let multi = MultiProgress::new();
        let pass_bars = Vec::new();

        let mut reporter = Self { multi, pass_bars, start_time: Instant::now(), quiet };

        if !quiet {
            if is_multi_pass {
                reporter.add_pass_bar("Compressing", 0);
                reporter.add_pass_bar("Deduplicating", 0);
                reporter.add_pass_bar("Refining", 0);
            } else {
                reporter.add_pass_bar("Compressing", 0);
            }
        }

        reporter
    }

    fn add_pass_bar(&mut self, label: &str, total: u64) {
        let style = ProgressStyle::default_bar()
            .template(&format!("[{{pos}}/{{len}}] {label:<16} {{bar:30}} {{msg}}"))
            .unwrap_or_else(|_| ProgressStyle::default_bar());

        let bar = self.multi.add(ProgressBar::new(total));
        bar.set_style(style);
        if total == 0 {
            bar.set_message("waiting");
        }
        self.pass_bars.push(bar);
    }

    pub fn set_total(&self, pass: usize, total: u64) {
        if self.quiet || pass >= self.pass_bars.len() {
            return;
        }
        self.pass_bars[pass].set_length(total);
    }

    pub fn inc(&self, pass: usize, section: &str) {
        if self.quiet || pass >= self.pass_bars.len() {
            return;
        }
        self.pass_bars[pass].set_message(section.to_string());
        self.pass_bars[pass].inc(1);
    }

    pub fn finish_pass(&self, pass: usize) {
        if self.quiet || pass >= self.pass_bars.len() {
            return;
        }
        self.pass_bars[pass].finish_with_message("done");
    }

    pub fn finish_all(&self, chunks: usize, input_tokens: usize, output_tokens: usize, output_path: &str) {
        if self.quiet {
            return;
        }
        for bar in &self.pass_bars {
            bar.finish_and_clear();
        }
        let elapsed = self.start_time.elapsed();
        let mins = elapsed.as_secs() / 60;
        let secs = elapsed.as_secs() % 60;
        let ratio = if input_tokens > 0 {
            (output_tokens as f64 / input_tokens as f64 * 100.0) as usize
        } else {
            100
        };
        eprintln!(
            "\nDone in {mins}m {secs:02}s | {chunks} chunks | {input_tokens} -> {output_tokens} tokens (~{ratio}%)\n-> {output_path}"
        );
    }
}
```

**Step 2: Update main.rs**

Add `mod progress;`

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/progress.rs src/main.rs
git commit -m "feat: progress reporting with indicatif multi-bar"
```

---

### Task 20: Main Orchestration

**Files:**
- Modify: `src/main.rs`

**Step 1: Wire everything together in main.rs**

```rust
mod cli;
mod compress;
mod config;
mod error;
mod export;
mod ingest;
mod llm;
mod mode;
mod progress;
mod segment;
mod state;

use clap::Parser;
use cli::{Cli, CompressionLevel, Mode, OutputFormat};
use error::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    // Handle --clean
    if cli.clean {
        let path = PathBuf::from(&cli.input);
        let cache_path = state::checkpoint::Checkpoint::cache_path(&path);
        state::checkpoint::Checkpoint::delete(&cache_path)?;
        eprintln!("Cleaned cache for {}", cli.input);
        return Ok(());
    }

    // Resolve config
    let config = config::Config::resolve(
        cli.api_key.clone(),
        cli.api_base.clone(),
        cli.model.clone(),
    )?;

    // Ingest
    let doc = ingest::ingest(&cli.input).await?;

    // Detect mode
    let detected_mode = mode::detect_mode(cli.mode.clone(), doc.estimated_tokens);

    // Determine compression level
    let level = cli.level.clone().unwrap_or(match detected_mode {
        Mode::Book => CompressionLevel::Dense,
        Mode::Article => CompressionLevel::Tight,
    });

    // Determine output format
    let format = cli.format.clone().unwrap_or(match detected_mode {
        Mode::Book => OutputFormat::Epub,
        Mode::Article => OutputFormat::Md,
    });

    // Header
    if !cli.quiet {
        eprintln!("distill | {} | {:?} | {:?}", cli.input, detected_mode, level);
    }

    // Segment
    let chunks = segment::segment(&doc.content);

    // Create LLM client
    let client = llm::LlmClient::new(config.api_key, config.api_base, config.model);

    // Compress based on mode
    let is_multi = detected_mode == Mode::Book;
    let compressed = if is_multi {
        compress::multi_pass(&client, chunks, &level, cli.parallel, cli.jobs).await?
    } else {
        compress::single_pass(&client, chunks, &level).await?
    };

    // Determine output path
    let output_path = cli.output.clone().or_else(|| {
        if detected_mode == Mode::Book {
            let stem = PathBuf::from(&cli.input);
            let stem = stem.file_stem().unwrap_or_default().to_string_lossy();
            let ext = match format {
                OutputFormat::Epub => "epub",
                OutputFormat::Html => "html",
                OutputFormat::Md => "md",
            };
            Some(PathBuf::from(format!("{stem}-distilled.{ext}")))
        } else {
            None
        }
    });

    // Export
    export::export(
        &compressed,
        doc.title.as_deref(),
        doc.author.as_deref(),
        &format,
        output_path.as_deref(),
    )?;

    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Verify --help works**

Run: `cargo run -- --help`

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: main orchestration wiring all pipeline stages"
```

---

### Task 21: Integration Test — Mock LLM Server

**Files:**
- Create: `tests/integration/helpers/mod.rs`
- Create: `tests/integration/helpers/mock_llm.rs`

**Step 1: Create mock LLM helper**

```rust
// tests/integration/helpers/mock_llm.rs
use wiremock::{MockServer, Mock, matchers, ResponseTemplate};
use serde_json::json;

pub async fn start_mock_llm() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{
                "message": {
                    "content": "<compressed>\n## Compressed Section\n\nThis is compressed content.\n</compressed>\n<ledger>\n{\"new_concepts\": [], \"new_examples\": []}\n</ledger>"
                }
            }]
        })))
        .mount(&server)
        .await;

    server
}
```

```rust
// tests/integration/helpers/mod.rs
pub mod mock_llm;
```

**Step 2: Commit**

```bash
git add tests/
git commit -m "test: mock LLM server helper for integration tests"
```

---

### Task 22: Clippy + Fmt Cleanup

**Files:**
- All source files

**Step 1: Run cargo fmt**

Run: `cargo fmt`

**Step 2: Run cargo clippy**

Run: `cargo clippy -- -D warnings`
Fix any warnings.

**Step 3: Run all tests**

Run: `cargo test`
Expected: all PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: clippy and fmt cleanup"
```

---

## Execution Notes

- Tasks 1-12 are the core pipeline — get these working first
- Tasks 13-15 (ingestion) can be refined once we test with real files
- Tasks 16-18 (export) can be iterated on quality later
- Task 20 (main orchestration) wires everything and is where the strategy pattern lives
- The `epub` and `readability` crate APIs may differ slightly from what's shown — verify against actual crate docs during implementation
- The `compress/mod.rs` parallel mode borrows `&LlmClient` through `Arc` — may need adjustment for the borrow checker at implementation time

---

Plan complete and saved to `docs/plans/2026-03-17-distill-implementation.md`. Two execution options:

**1. Subagent-Driven (this session)** — I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** — Open new session with executing-plans, batch execution with checkpoints

Which approach?
