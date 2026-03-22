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
use owo_colors::OwoColorize;
use std::process::ExitCode;
use std::sync::Arc;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            ui::print_error(&e);
            ExitCode::FAILURE
        }
    }
}

async fn run() -> error::Result<()> {
    let cli = Cli::parse();
    let console = ui::Console::new(cli.quiet);

    // Handle --clean
    if cli.clean {
        let cache_path = state::checkpoint::Checkpoint::cache_path_for_input(&cli.input);
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
    console.ingested(
        doc.estimated_tokens,
        &format!("{detected_mode:?}"),
        &format!("{level:?}"),
    );

    // Segment
    let chunks = segment::segment(&doc.content);
    let chunk_count = chunks.len();

    let input_hash = state::checkpoint::Checkpoint::input_hash(&doc.content);

    // Create LLM client and compression strategy
    let model_name = config.model.clone();
    let client = Arc::new(llm::LlmClient::new(
        config.api_key,
        config.api_base,
        model_name.clone(),
        cli.verbose,
    ));
    let strategy: Arc<dyn llm::strategy::CompressionStrategy> =
        llm::strategy::strategy_for(&level).into();

    let checkpoint = if cli.resume {
        let cache_path = state::checkpoint::Checkpoint::cache_path_for_input(&cli.input);
        match state::checkpoint::Checkpoint::load(&cache_path) {
            Ok(existing) => {
                existing.validate_resume(&input_hash, &level, &model_name, &chunks)?;
                if cli.verbose >= 1 {
                    eprintln!(
                        "{} resuming from {} (completed_pass={})",
                        "[checkpoint]".dimmed(),
                        cache_path.display(),
                        existing.completed_pass
                    );
                }
                Some((cache_path, existing))
            }
            Err(error::DistillError::MissingCheckpoint { .. }) => {
                let fresh = state::checkpoint::Checkpoint::new(
                    input_hash,
                    level.clone(),
                    model_name.clone(),
                    &chunks,
                );
                fresh.save(&cache_path)?;
                if cli.verbose >= 1 {
                    eprintln!(
                        "{} no checkpoint found, starting fresh at {}",
                        "[checkpoint]".dimmed(),
                        cache_path.display()
                    );
                }
                Some((cache_path, fresh))
            }
            Err(e) => return Err(e),
        }
    } else {
        None
    };

    // Compress
    let is_multi = detected_mode == Mode::Book && strategy.supports_multi_pass();
    let pipeline = if is_multi {
        "hierarchical (distill → dedup → refine)"
    } else {
        "single-pass"
    };

    if cli.verbose >= 1 {
        eprintln!(
            "{} level={level:?} pipeline={pipeline} chunks={chunk_count}",
            "[pipeline]".dimmed()
        );
    }
    if cli.verbose >= 2 {
        eprintln!(
            "{}\n{}",
            "[pipeline] system prompt:".dimmed(),
            strategy.distill_system()
        );
    }

    let compressed = if is_multi {
        compress::hierarchical(
            client,
            chunks,
            strategy,
            cli.parallel,
            cli.jobs,
            &console,
            checkpoint,
        )
        .await?
    } else {
        let sp = console.spinner(&format!("Compressing {chunk_count} chunks..."));
        let result = compress::single_pass(client, chunks, strategy, checkpoint).await?;
        sp.finish();
        console.compressed(chunk_count);
        result
    };

    // Determine output path
    let output_path = cli.output.clone().or_else(|| {
        if detected_mode == Mode::Book {
            let stem = std::path::PathBuf::from(&cli.input);
            let stem = stem.file_stem().unwrap_or_default().to_string_lossy();
            let ext = match format {
                OutputFormat::Epub => "epub",
                OutputFormat::Html => "html",
                OutputFormat::Md => "md",
            };
            Some(std::path::PathBuf::from(format!("{stem}-distilled.{ext}")))
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
    console.done(
        chunk_count,
        doc.estimated_tokens,
        output_tokens,
        &output_display,
    );

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
