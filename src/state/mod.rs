pub mod checkpoint;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateLedger {
    pub concepts: Vec<Concept>,
    pub definitions: Vec<Definition>,
    pub principles: Vec<Principle>,
    pub examples: Vec<Example>,
    pub anti_patterns: Vec<AntiPattern>,
    pub relationships: Vec<Relationship>,
}

impl StateLedger {
    pub fn apply_delta(&mut self, delta: &LedgerDelta) {
        self.concepts.extend(delta.new_concepts.iter().cloned());
        self.definitions
            .extend(delta.new_definitions.iter().cloned());
        self.principles.extend(delta.new_principles.iter().cloned());
        self.examples.extend(delta.new_examples.iter().cloned());
        self.anti_patterns
            .extend(delta.new_anti_patterns.iter().cloned());
        self.relationships
            .extend(delta.new_relationships.iter().cloned());
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
pub struct Definition {
    pub id: String,
    pub term: String,
    pub meaning: String,
    pub first_seen_chunk: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub id: String,
    pub name: String,
    pub statement: String,
    pub related_concept: String,
    pub first_seen_chunk: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub id: String,
    pub related_concept: String,
    pub first_seen_chunk: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub related_concept: String,
    pub first_seen_chunk: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub from_concept: String,
    pub to_concept: String,
    pub relation_type: String,
    pub first_seen_chunk: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LedgerDelta {
    pub new_concepts: Vec<Concept>,
    pub new_definitions: Vec<Definition>,
    pub new_principles: Vec<Principle>,
    pub new_examples: Vec<Example>,
    pub new_anti_patterns: Vec<AntiPattern>,
    pub new_relationships: Vec<Relationship>,
}

#[derive(Debug, Clone)]
pub struct CompressedChunk {
    pub index: usize,
    pub header_path: Vec<String>,
    pub content: String,
    pub ledger_updates: LedgerDelta,
}
