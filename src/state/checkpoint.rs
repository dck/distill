use crate::cli::CompressionLevel;
use crate::error::{DistillError, Result};
use crate::segment::Chunk;
use crate::state::CompressedChunk;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkStatus {
    Pending,
    Compressed,
    Finalized,
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
    pub pass2_output: Option<String>,
    pub final_output: Option<String>,
    pub chunks: Vec<ChunkState>,
}

impl Checkpoint {
    pub fn cache_path(input_path: &Path) -> PathBuf {
        let stem = input_path.file_stem().unwrap_or_default();
        input_path.with_file_name(format!("{}.distill-cache", stem.to_string_lossy()))
    }

    pub fn cache_path_for_input(input: &str) -> PathBuf {
        if crate::mode::is_url(input) {
            let digest = hash_text(input);
            std::env::temp_dir().join(format!("distill-{}.distill-cache", &digest[..16]))
        } else {
            Self::cache_path(Path::new(input))
        }
    }

    pub fn input_hash(content: &str) -> String {
        hash_text(content)
    }

    pub fn new(
        input_hash: String,
        level: CompressionLevel,
        model: String,
        chunks: &[Chunk],
    ) -> Self {
        Self {
            input_hash,
            level,
            model,
            completed_pass: 0,
            pass2_output: None,
            final_output: None,
            chunks: chunks
                .iter()
                .map(|chunk| ChunkState {
                    index: chunk.index,
                    status: ChunkStatus::Pending,
                    original: chunk.content.clone(),
                    compressed: None,
                })
                .collect(),
        }
    }

    pub fn validate_resume(
        &self,
        input_hash: &str,
        level: &CompressionLevel,
        model: &str,
        chunks: &[Chunk],
    ) -> Result<()> {
        if self.input_hash != input_hash {
            return Err(DistillError::CheckpointMismatch {
                field: "input hash",
                expected: self.input_hash.clone(),
                found: input_hash.to_string(),
            });
        }
        if &self.level != level {
            return Err(DistillError::CheckpointMismatch {
                field: "compression level",
                expected: format!("{:?}", self.level),
                found: format!("{level:?}"),
            });
        }
        if self.model != model {
            return Err(DistillError::CheckpointMismatch {
                field: "model",
                expected: self.model.clone(),
                found: model.to_string(),
            });
        }
        if self.chunks.len() != chunks.len() {
            return Err(DistillError::CheckpointMismatch {
                field: "chunk count",
                expected: self.chunks.len().to_string(),
                found: chunks.len().to_string(),
            });
        }

        for (saved, chunk) in self.chunks.iter().zip(chunks) {
            if saved.index != chunk.index {
                return Err(DistillError::CheckpointMismatch {
                    field: "chunk index",
                    expected: saved.index.to_string(),
                    found: chunk.index.to_string(),
                });
            }
            if saved.original != chunk.content {
                return Err(DistillError::CheckpointMismatch {
                    field: "chunk content",
                    expected: format!("chunk {}", saved.index),
                    found: format!("chunk {}", chunk.index),
                });
            }
        }

        Ok(())
    }

    pub fn compressed_chunks(&self, chunks: &[Chunk]) -> Result<Vec<CompressedChunk>> {
        if self.chunks.len() != chunks.len() {
            return Err(DistillError::CheckpointMismatch {
                field: "chunk count",
                expected: self.chunks.len().to_string(),
                found: chunks.len().to_string(),
            });
        }

        self.chunks
            .iter()
            .zip(chunks)
            .map(|(saved, chunk)| {
                let content = saved
                    .compressed
                    .clone()
                    .ok_or(DistillError::CheckpointMismatch {
                        field: "compressed chunk",
                        expected: "present".into(),
                        found: format!("missing for chunk {}", saved.index),
                    })?;
                Ok(CompressedChunk {
                    index: chunk.index,
                    header_path: chunk.header_path.clone(),
                    content,
                })
            })
            .collect()
    }

    pub fn update_chunk(&mut self, chunk: &CompressedChunk) {
        if let Some(state) = self.chunks.get_mut(chunk.index) {
            state.status = ChunkStatus::Compressed;
            state.compressed = Some(chunk.content.clone());
        }
    }

    pub fn mark_pass2(&mut self, output: String) {
        self.completed_pass = self.completed_pass.max(2);
        self.pass2_output = Some(output);
    }

    pub fn mark_finished(&mut self, completed_pass: u8, output: String) {
        self.completed_pass = completed_pass;
        self.final_output = Some(output);
        for chunk in &mut self.chunks {
            if chunk.compressed.is_some() {
                chunk.status = ChunkStatus::Finalized;
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| DistillError::CheckpointParse {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        std::fs::write(path, json).map_err(|e| DistillError::CheckpointIo {
            path: path.to_path_buf(),
            cause: e.to_string(),
        })?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DistillError::MissingCheckpoint {
                    path: path.to_path_buf(),
                }
            } else {
                DistillError::CheckpointIo {
                    path: path.to_path_buf(),
                    cause: e.to_string(),
                }
            }
        })?;
        let checkpoint: Self =
            serde_json::from_str(&json).map_err(|e| DistillError::CheckpointParse {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        Ok(checkpoint)
    }

    pub fn delete(path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| DistillError::CheckpointIo {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
        }
        Ok(())
    }
}

fn hash_text(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_chunks() -> Vec<Chunk> {
        vec![
            Chunk {
                index: 0,
                header_path: vec!["Chapter 1".into()],
                content: "original text".into(),
                token_estimate: 10,
            },
            Chunk {
                index: 1,
                header_path: vec!["Chapter 2".into()],
                content: "more text".into(),
                token_estimate: 10,
            },
        ]
    }

    fn sample_checkpoint() -> Checkpoint {
        let chunks = sample_chunks();
        let mut checkpoint = Checkpoint::new(
            "abc123".into(),
            CompressionLevel::Dense,
            "test-model".into(),
            &chunks,
        );
        checkpoint.update_chunk(&CompressedChunk {
            index: 0,
            header_path: chunks[0].header_path.clone(),
            content: "compressed text".into(),
        });
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
        assert_eq!(loaded.completed_pass, 0);
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
    fn test_cache_path_for_url_uses_hash() {
        let cache = Checkpoint::cache_path_for_input("https://example.com/book");
        let file_name = cache.file_name().unwrap().to_string_lossy();
        assert!(file_name.starts_with("distill-"));
        assert!(file_name.ends_with(".distill-cache"));
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

    #[test]
    fn test_validate_resume_rejects_changed_content() {
        let checkpoint = sample_checkpoint();
        let mut chunks = sample_chunks();
        chunks[0].content = "changed".into();

        let err = checkpoint
            .validate_resume("abc123", &CompressionLevel::Dense, "test-model", &chunks)
            .unwrap_err();

        assert!(matches!(err, DistillError::CheckpointMismatch { .. }));
    }

    #[test]
    fn test_compressed_chunks_restores_saved_content() {
        let checkpoint = sample_checkpoint();
        let chunks = sample_chunks();
        let restored = checkpoint.compressed_chunks(&chunks).unwrap_err();
        assert!(matches!(restored, DistillError::CheckpointMismatch { .. }));
    }

    #[test]
    fn test_input_hash_is_stable() {
        let first = Checkpoint::input_hash("same content");
        let second = Checkpoint::input_hash("same content");
        assert_eq!(first, second);
    }
}
