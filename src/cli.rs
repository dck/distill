use std::path::PathBuf;

use clap::{Parser, ValueEnum};

fn parse_jobs(value: &str) -> Result<usize, String> {
    let jobs = value
        .parse::<usize>()
        .map_err(|_| format!("invalid concurrency limit: {value}"))?;
    if jobs == 0 {
        return Err("concurrency limit must be at least 1".into());
    }
    Ok(jobs)
}

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
    Tldr,
}

impl std::str::FromStr for CompressionLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tight" => Ok(Self::Tight),
            "dense" => Ok(Self::Dense),
            "distilled" => Ok(Self::Distilled),
            "tldr" => Ok(Self::Tldr),
            _ => Err(format!("unknown compression level: {s}")),
        }
    }
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum Mode {
    Book,
    Article,
}

#[derive(Debug, Parser)]
#[command(
    name = "distill",
    about = "Structure-preserving semantic compression engine"
)]
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

    /// Concurrency limit [default: 1, sequential]
    #[arg(short, long, default_value_t = 1, value_parser = parse_jobs)]
    pub jobs: usize,

    /// Increase log verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress progress bars, errors only
    #[arg(short, long)]
    pub quiet: bool,
}

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
            "distill",
            "-o",
            "out.epub",
            "-f",
            "epub",
            "-l",
            "dense",
            "-m",
            "book",
            "-j",
            "8",
            "-v",
            "input.pdf",
        ]);
        assert_eq!(args.output, Some("out.epub".into()));
        assert_eq!(args.format, Some(OutputFormat::Epub));
        assert_eq!(args.level, Some(CompressionLevel::Dense));
        assert_eq!(args.mode, Some(Mode::Book));
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
    fn test_jobs_must_be_positive() {
        let result = Cli::try_parse_from(["distill", "-j", "0", "input.pdf"]);
        assert!(result.is_err());
    }
}
