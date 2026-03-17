pub mod checkpoint;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateLedger {
    pub concepts: Vec<Concept>,
    pub examples: Vec<Example>,
}

impl StateLedger {
    pub fn apply_delta(&mut self, delta: &LedgerDelta) {
        self.concepts.extend(delta.new_concepts.iter().cloned());
        self.examples.extend(delta.new_examples.iter().cloned());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub first_seen_chunk: usize,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub id: String,
    pub related_concept: String,
    pub first_seen_chunk: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LedgerDelta {
    pub new_concepts: Vec<Concept>,
    pub new_examples: Vec<Example>,
}

#[derive(Debug, Clone)]
pub struct CompressedChunk {
    pub index: usize,
    pub header_path: Vec<String>,
    pub content: String,
    pub ledger_updates: LedgerDelta,
}
