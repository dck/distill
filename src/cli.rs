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
    Tldr,
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
            "--parallel",
            "-j",
            "8",
            "-v",
            "input.pdf",
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
