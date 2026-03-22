use crate::cli::CompressionLevel;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

    pub fn cache_path_for_input(input: &str) -> PathBuf {
        if crate::mode::is_url(input) {
            let digest = Self::stable_hash(input);
            std::env::temp_dir().join(format!("distill-{}.distill-cache", &digest[..16]))
        } else {
            Self::cache_path(Path::new(input))
        }
    }

    pub fn input_hash(content: &str) -> String {
        Self::stable_hash(content)
    }

    pub fn new(
        input_hash: String,
        level: CompressionLevel,
        model: String,
        originals: &[String],
    ) -> Self {
        Self {
            input_hash,
            level,
            model,
            completed_pass: 0,
            chunks: originals
                .iter()
                .enumerate()
                .map(|(index, original)| ChunkState {
                    index,
                    status: ChunkStatus::Pending,
                    original: original.clone(),
                    compressed: None,
                })
                .collect(),
        }
    }

    pub fn matches_run(
        &self,
        input_hash: &str,
        level: &CompressionLevel,
        model: &str,
        originals: &[String],
    ) -> bool {
        self.input_hash == input_hash
            && &self.level == level
            && self.model == model
            && self.chunks.len() == originals.len()
            && self
                .chunks
                .iter()
                .zip(originals)
                .all(|(saved, original)| saved.original == *original)
    }

    pub fn update_chunk(&mut self, index: usize, compressed: String) {
        if let Some(chunk) = self.chunks.get_mut(index) {
            chunk.compressed = Some(compressed);
            chunk.status = ChunkStatus::Compressed;
        }
    }

    pub fn compressed_for(&self, index: usize) -> Option<&str> {
        self.chunks.get(index)?.compressed.as_deref()
    }

    pub fn all_chunks_compressed(&self) -> bool {
        self.chunks.iter().all(|chunk| chunk.compressed.is_some())
    }

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

    fn stable_hash(value: &str) -> String {
        let digest = Sha256::digest(value.as_bytes());
        format!("{digest:x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_checkpoint() -> Checkpoint {
        let mut checkpoint = Checkpoint::new(
            "abc123".into(),
            CompressionLevel::Dense,
            "test-model".into(),
            &["original text".into(), "more text".into()],
        );
        checkpoint.update_chunk(0, "compressed text".into());
        checkpoint.completed_pass = 1;
        checkpoint
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
    fn test_cache_path_for_url_uses_temp_hash() {
        let cache = Checkpoint::cache_path_for_input("https://example.com/book");
        let file_name = cache.file_name().unwrap().to_string_lossy();
        assert!(cache.starts_with(std::env::temp_dir()));
        assert!(file_name.starts_with("distill-"));
        assert!(file_name.ends_with(".distill-cache"));
    }

    #[test]
    fn test_matches_run_rejects_changed_input() {
        let checkpoint = sample_checkpoint();
        let originals = vec!["changed".to_string(), "more text".to_string()];
        assert!(!checkpoint.matches_run(
            "abc123",
            &CompressionLevel::Dense,
            "test-model",
            &originals
        ));
    }

    #[test]
    fn test_all_chunks_compressed() {
        let checkpoint = sample_checkpoint();
        assert!(!checkpoint.all_chunks_compressed());
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
