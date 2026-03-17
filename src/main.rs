mod cli;
mod config;
mod error;
mod mode;

use clap::Parser;

fn main() -> error::Result<()> {
    color_eyre::install()?;
    let _cli = cli::Cli::parse();
    println!("distill");
    Ok(())
}
