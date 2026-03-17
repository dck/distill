mod cli;
mod config;
mod error;
mod llm;
mod mode;
mod segment;
mod state;

use clap::Parser;

fn main() -> error::Result<()> {
    color_eyre::install()?;
    let _cli = cli::Cli::parse();
    println!("distill");
    Ok(())
}
