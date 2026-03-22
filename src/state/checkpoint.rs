use crate::cli::CompressionLevel;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkStatus {
    Pending,
    Compressed,
    Refined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkState {
    pub index: usize,
    pub status: ChunkStatus,
    pub original: String,
    pub compressed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub input_hash: String,
    pub level: CompressionLevel,
    pub model: String,
    pub completed_pass: u8,
    pub chunks: Vec<ChunkState>,
}

impl Checkpoint {
    pub fn cache_path(input_path: &Path) -> PathBuf {
        let stem = input_path.file_stem().unwrap_or_default();
        input_path.with_file_name(format!("{}.distill-cache", stem.to_string_lossy()))
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            }
        })?;
        std::fs::write(path, json).map_err(|e| crate::error::DistillError::Checkpoint {
            path: path.to_path_buf(),
            cause: e.to_string(),
        })?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load(path: &Path) -> Result<Self> {
        let json =
            std::fs::read_to_string(path).map_err(|e| crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        let checkpoint: Self =
            serde_json::from_str(&json).map_err(|e| crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        Ok(checkpoint)
    }

    pub fn delete(path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| crate::error::DistillError::Checkpoint {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_checkpoint() -> Checkpoint {
        Checkpoint {
            input_hash: "abc123".into(),
            level: CompressionLevel::Dense,
            model: "test-model".into(),
            completed_pass: 1,
            chunks: vec![
                ChunkState {
                    index: 0,
                    status: ChunkStatus::Compressed,
                    original: "original text".into(),
                    compressed: Some("compressed text".into()),
                },
                ChunkState {
                    index: 1,
                    status: ChunkStatus::Pending,
                    original: "more text".into(),
                    compressed: None,
                },
            ],
        }
    }

    #[test]
    fn test_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.distill-cache");
        let checkpoint = sample_checkpoint();

        checkpoint.save(&path).unwrap();
        let loaded = Checkpoint::load(&path).unwrap();

        assert_eq!(loaded.input_hash, "abc123");
        assert_eq!(loaded.completed_pass, 1);
        assert_eq!(loaded.chunks.len(), 2);
        assert_eq!(loaded.chunks[0].status, ChunkStatus::Compressed);
        assert_eq!(loaded.chunks[1].status, ChunkStatus::Pending);
    }

    #[test]
    fn test_cache_path_naming() {
        let path = Path::new("/home/user/books/thinking-fast.pdf");
        let cache = Checkpoint::cache_path(path);
        assert_eq!(
            cache,
            Path::new("/home/user/books/thinking-fast.distill-cache")
        );
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.distill-cache");
        assert!(Checkpoint::delete(&path).is_ok());
    }

    #[test]
    fn test_delete_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.distill-cache");
        let checkpoint = sample_checkpoint();
        checkpoint.save(&path).unwrap();
        assert!(path.exists());
        Checkpoint::delete(&path).unwrap();
        assert!(!path.exists());
    }
}
