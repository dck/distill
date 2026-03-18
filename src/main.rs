mod cli;
mod compress;
mod config;
mod error;
mod export;
mod ingest;
mod llm;
mod mode;
mod segment;
mod state;
mod ui;

use clap::Parser;
use cli::{Cli, CompressionLevel, Mode, OutputFormat};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            ui::print_error(e.as_ref());
            ExitCode::FAILURE
        }
    }
}

async fn run() -> error::Result<()> {
    let cli = Cli::parse();
    let console = ui::Console::new(cli.quiet);

    // Handle --clean
    if cli.clean {
        let path = PathBuf::from(&cli.input);
        let cache_path = state::checkpoint::Checkpoint::cache_path(&path);
        state::checkpoint::Checkpoint::delete(&cache_path)?;
        console.cleaned(&cli.input);
        return Ok(());
    }

    // Resolve config
    let config =
        config::Config::resolve(cli.api_key.clone(), cli.api_base.clone(), cli.model.clone())?;

    // Ingest
    let sp = console.spinner(&format!("Ingesting {}", cli.input));
    let doc = ingest::ingest(&cli.input).await?;

    // Detect mode and settings
    let detected_mode = mode::detect_mode(cli.mode.clone(), doc.estimated_tokens);
    let level = cli.level.clone().unwrap_or(match detected_mode {
        Mode::Book => CompressionLevel::Dense,
        Mode::Article => CompressionLevel::Tight,
    });
    let format = cli.format.clone().unwrap_or(match detected_mode {
        Mode::Book => OutputFormat::Epub,
        Mode::Article => OutputFormat::Md,
    });

    sp.finish();
    console.ingested(doc.estimated_tokens, &format!("{detected_mode:?}"), &format!("{level:?}"));

    // Segment
    let chunks = segment::segment(&doc.content);
    let chunk_count = chunks.len();

    // Create LLM client
    let client = Arc::new(llm::LlmClient::new(
        config.api_key,
        config.api_base,
        config.model,
        cli.verbose,
    ));

    // Compress
    let is_multi = detected_mode == Mode::Book;
    let compressed = if is_multi {
        compress::multi_pass(client, chunks, &level, cli.parallel, cli.jobs, &console).await?
    } else {
        let sp = console.spinner(&format!("Compressing {chunk_count} chunks..."));
        let result = compress::single_pass(&client, chunks, &level).await?;
        sp.finish();
        console.compressed(chunk_count);
        result
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

    let output_display = output_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "stdout".into());

    // Summary (stderr, before export so markdown appears last in terminal)
    let output_tokens = mode::estimate_tokens(&compressed);
    console.done(chunk_count, doc.estimated_tokens, output_tokens, &output_display);

    // Export (stdout for articles without -o, file otherwise)
    export::export(
        &compressed,
        doc.title.as_deref(),
        doc.author.as_deref(),
        &format,
        output_path.as_deref(),
    )?;

    Ok(())
}
