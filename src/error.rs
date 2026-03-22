use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, DistillError>;

#[derive(Debug)]
pub enum DistillError {
    Ingestion {
        source: String,
        cause: String,
    },
    UnsupportedInput {
        source: String,
        extension: String,
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
    MissingConfig {
        field: &'static str,
        env_var: &'static str,
        flag: &'static str,
    },
    MissingCheckpoint {
        path: PathBuf,
    },
    CheckpointIo {
        path: PathBuf,
        cause: String,
    },
    CheckpointParse {
        path: PathBuf,
        cause: String,
    },
    CheckpointMismatch {
        field: &'static str,
        expected: String,
        found: String,
    },
    HttpStatus {
        source: String,
        status: String,
        body: String,
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
            Self::UnsupportedInput { source, extension } => {
                write!(
                    f,
                    "unsupported input\n  -> source: {source}\n  -> extension: .{extension}"
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
            Self::MissingConfig {
                field,
                env_var,
                flag,
            } => {
                write!(
                    f,
                    "configuration error\n  -> missing {field}. Set {env_var} or pass {flag}"
                )
            }
            Self::MissingCheckpoint { path } => {
                write!(f, "checkpoint not found\n  -> file: {}", path.display())
            }
            Self::CheckpointIo { path, cause } => {
                write!(
                    f,
                    "checkpoint I/O error\n  -> file: {}\n  -> caused by: {cause}",
                    path.display()
                )
            }
            Self::CheckpointParse { path, cause } => {
                write!(
                    f,
                    "checkpoint parse error\n  -> file: {}\n  -> caused by: {cause}",
                    path.display()
                )
            }
            Self::CheckpointMismatch {
                field,
                expected,
                found,
            } => {
                write!(
                    f,
                    "checkpoint mismatch\n  -> {field}: expected {expected}, found {found}"
                )
            }
            Self::HttpStatus {
                source,
                status,
                body,
            } => {
                write!(
                    f,
                    "HTTP request failed\n  -> source: {source}\n  -> status: {status}\n  -> body: {body}"
                )
            }
        }
    }
}

impl std::error::Error for DistillError {}
