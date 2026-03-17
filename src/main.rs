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

fn main() -> error::Result<()> {
    color_eyre::install()?;
    let _cli = cli::Cli::parse();
    println!("distill");
    Ok(())
}
