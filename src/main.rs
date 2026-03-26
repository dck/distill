mod cli;
mod compress;
mod config;
mod config_cmd;
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
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("config") {
        config_cmd::handle(&args[2..]);
        return ExitCode::SUCCESS;
    }

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

    // Resolve config
    let config =
        config::Config::resolve(cli.api_key.clone(), cli.api_base.clone(), cli.model.clone())?;

    // Ingest
    let sp = console.spinner(&format!("Ingesting {}", cli.input));
    let doc = ingest::ingest(&cli.input).await?;

    // Detect mode and settings
    let detected_mode = mode::detect_mode(cli.mode.clone(), doc.estimated_tokens);
    let file = config::load_config_file();
    let file_level = file
        .level
        .as_deref()
        .and_then(|s| s.parse::<CompressionLevel>().ok());
    let level = cli
        .level
        .clone()
        .or(file_level)
        .unwrap_or(match detected_mode {
            Mode::Book => CompressionLevel::Medium,
            Mode::Article => CompressionLevel::Light,
        });
    let jobs = if cli.jobs != 1 {
        cli.jobs
    } else {
        file.jobs.unwrap_or(1)
    };
    let format = cli.format.clone().unwrap_or(match detected_mode {
        Mode::Book => OutputFormat::Epub,
        Mode::Article => OutputFormat::Md,
    });

    sp.finish();

    // Create LLM client and compression strategy
    let model_name = config.model.clone();

    console.ingested(
        doc.estimated_tokens,
        &format!("{detected_mode:?}"),
        &format!("{level:?}"),
        &model_name,
    );
    let client = Arc::new(llm::LlmClient::new(
        config.api_key,
        config.api_base,
        config.model,
        cli.verbose,
    ));
    let strategy = llm::strategy::strategy_for(&level);
    let is_multi = detected_mode == Mode::Book && strategy.supports_multi_pass();

    // Segment (books only — articles go as one chunk)
    let chunks = if is_multi {
        segment::segment(&doc.content)
    } else {
        vec![segment::Chunk {
            index: 0,
            header_path: vec![],
            content: doc.content.clone(),
            token_estimate: doc.estimated_tokens,
        }]
    };
    let chunk_count = chunks.len();

    // Compress
    let pipeline = if is_multi {
        "hierarchical (distill → refine)"
    } else {
        "single-pass"
    };
    let checkpoint_path =
        is_multi.then(|| state::checkpoint::Checkpoint::cache_path_for_input(&cli.input));

    if cli.verbose >= 1 {
        eprintln!(
            "{} level={level:?} pipeline={pipeline} chapters={chunk_count}",
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
        let strategy: Arc<dyn llm::strategy::CompressionStrategy> = strategy.into();
        let originals = chunks
            .iter()
            .map(|chunk| chunk.content.clone())
            .collect::<Vec<_>>();
        let input_hash = state::checkpoint::Checkpoint::input_hash(&doc.content);
        let checkpoint = checkpoint_path
            .as_ref()
            .map(|path| prepare_checkpoint(path, &input_hash, &level, &model_name, &originals));
        let checkpoint = match checkpoint {
            Some(Ok(state)) => Some(state),
            Some(Err(e)) => return Err(e),
            None => None,
        };
        compress::hierarchical(client, chunks, strategy, jobs, &console, checkpoint).await?
    } else {
        let sp = console.spinner("Distilling...");
        let result = compress::single_pass(&client, chunks, strategy.as_ref()).await?;
        sp.finish();
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

    if let Some(path) = checkpoint_path {
        state::checkpoint::Checkpoint::delete(&path)?;
    }

    Ok(())
}

fn prepare_checkpoint(
    path: &std::path::Path,
    input_hash: &str,
    level: &CompressionLevel,
    model_name: &str,
    originals: &[String],
) -> error::Result<(PathBuf, state::checkpoint::Checkpoint)> {
    let checkpoint = match state::checkpoint::Checkpoint::load(path) {
        Ok(existing) if existing.matches_run(input_hash, level, model_name, originals) => existing,
        Ok(_) => {
            ui::warning("existing checkpoint did not match current input, starting over");
            state::checkpoint::Checkpoint::new(
                input_hash.to_string(),
                level.clone(),
                model_name.to_string(),
                originals,
            )
        }
        Err(_) => state::checkpoint::Checkpoint::new(
            input_hash.to_string(),
            level.clone(),
            model_name.to_string(),
            originals,
        ),
    };

    checkpoint.save(path)?;
    Ok((path.to_path_buf(), checkpoint))
}
