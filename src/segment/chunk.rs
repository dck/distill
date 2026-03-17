use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub index: usize,
    pub header_path: Vec<String>,
    pub content: String,
    pub token_estimate: usize,
}
