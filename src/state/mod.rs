pub mod checkpoint;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedChunk {
    pub index: usize,
    pub header_path: Vec<String>,
    pub content: String,
}
