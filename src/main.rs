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
use std::sync::Arc;

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
    let chunk_count = chunks.len();

    // Create LLM client
    let client = Arc::new(llm::LlmClient::new(config.api_key, config.api_base, config.model));

    // Compress based on mode
    let is_multi = detected_mode == Mode::Book;
    let compressed = if is_multi {
        compress::multi_pass(client, chunks, &level, cli.parallel, cli.jobs).await?
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

    // Summary
    if !cli.quiet {
        let output_tokens = mode::estimate_tokens(&compressed);
        let output_display = output_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "stdout".into());
        eprintln!(
            "\nDone | {} chunks | {} -> {} tokens (~{}%)\n-> {}",
            chunk_count,
            doc.estimated_tokens,
            output_tokens,
            if doc.estimated_tokens > 0 {
                (output_tokens as f64 / doc.estimated_tokens as f64 * 100.0) as usize
            } else {
                100
            },
            output_display,
        );
    }

    Ok(())
}
