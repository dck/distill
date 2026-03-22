use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
pub enum DistillError {
    Ingestion {
        source: String,
        cause: String,
    },
    Compression {
        chunk_index: usize,
        section: String,
        cause: String,
    },
    Llm {
        cause: String,
    },
    Export {
        cause: String,
    },
    Config {
        cause: String,
    },
    Checkpoint {
        path: PathBuf,
        cause: String,
    },
}

impl std::fmt::Display for DistillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ingestion { source, cause } => {
                write!(
                    f,
                    "failed to ingest content\n  -> source: {source}\n  -> caused by: {cause}"
                )
            }
            Self::Compression {
                chunk_index,
                section,
                cause,
            } => {
                write!(
                    f,
                    "failed to compress chunk {chunk_index}\n  -> section: \"{section}\"\n  -> caused by: {cause}"
                )
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
                write!(
                    f,
                    "checkpoint error\n  -> file: {}\n  -> caused by: {cause}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DistillError {}
