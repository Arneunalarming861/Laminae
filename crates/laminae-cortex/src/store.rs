//! Instruction storage with deduplication and ranking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A learned instruction derived from user edit patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedInstruction {
    /// The instruction text (e.g., "Never start with I think").
    pub text: String,
    /// How many times this instruction has been reinforced by edits.
    pub source_count: u32,
    /// When this instruction was first added.
    pub added: DateTime<Utc>,
}

/// Deduplicated, ranked store of learned instructions.
pub struct InstructionStore {
    instructions: Vec<LearnedInstruction>,
    max_size: usize,
    dedup_threshold: f64,
}

impl InstructionStore {
    /// Create a new store with maximum capacity and dedup threshold.
    ///
    /// - `max_size`: Maximum instructions to keep (FIFO oldest dropped).
    /// - `dedup_threshold`: Word overlap threshold (0.0-1.0) for deduplication.
    ///   Instructions with overlap above this are considered duplicates.
    pub fn new(max_size: usize, dedup_threshold: f64) -> Self {
        Self {
            instructions: Vec::new(),
            max_size,
            dedup_threshold,
        }
    }

    /// Add an instruction, deduplicating against existing ones.
    ///
    /// If a similar instruction exists (word overlap > threshold),
    /// increments its `source_count` instead of adding a duplicate.
    pub fn add(&mut self, instruction: LearnedInstruction) {
        let new_words: HashSet<&str> = instruction.text.split_whitespace().collect();

        // Check for duplicates
        for existing in &mut self.instructions {
            let existing_words: HashSet<&str> = existing.text.split_whitespace().collect();
            let overlap = word_overlap(&new_words, &existing_words);

            if overlap > self.dedup_threshold {
                existing.source_count += instruction.source_count;
                return;
            }
        }

        // Not a duplicate — add it
        self.instructions.push(instruction);

        // Cap at max size (drop oldest)
        if self.instructions.len() > self.max_size {
            // Sort by source_count (most reinforced first), then by age
            self.instructions.sort_by(|a, b| {
                b.source_count
                    .cmp(&a.source_count)
                    .then(b.added.cmp(&a.added))
            });
            self.instructions.truncate(self.max_size);
        }
    }

    /// Get the top N instructions, ranked by reinforcement count.
    pub fn top(&self, n: usize) -> Vec<&LearnedInstruction> {
        let mut sorted: Vec<&LearnedInstruction> = self.instructions.iter().collect();
        sorted.sort_by(|a, b| b.source_count.cmp(&a.source_count));
        sorted.truncate(n);
        sorted
    }

    /// Get all instructions.
    pub fn all(&self) -> Vec<LearnedInstruction> {
        self.instructions.clone()
    }

    /// Number of stored instructions.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

fn word_overlap(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let max_len = a.len().max(b.len()) as f64;
    if max_len == 0.0 {
        return 1.0;
    }
    a.intersection(b).count() as f64 / max_len
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_instruction(text: &str, count: u32) -> LearnedInstruction {
        LearnedInstruction {
            text: text.to_string(),
            source_count: count,
            added: Utc::now(),
        }
    }

    #[test]
    fn test_add_unique() {
        let mut store = InstructionStore::new(50, 0.8);
        store.add(make_instruction("Never start with I think", 1));
        store.add(make_instruction("Keep under 2 sentences", 1));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_dedup_similar() {
        let mut store = InstructionStore::new(50, 0.8);
        store.add(make_instruction("Never start with I think", 1));
        store.add(make_instruction("Never start with I think please", 1));
        // Should deduplicate — "Never start with I think" overlaps > 80%
        assert_eq!(store.len(), 1);
        assert_eq!(store.instructions[0].source_count, 2);
    }

    #[test]
    fn test_dedup_different() {
        let mut store = InstructionStore::new(50, 0.8);
        store.add(make_instruction("Never start with I think", 1));
        store.add(make_instruction("Be more direct and concise", 1));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_max_size_cap() {
        let mut store = InstructionStore::new(3, 1.0); // no dedup (threshold=1.0), max 3

        store.add(make_instruction("First instruction", 1));
        store.add(make_instruction("Second instruction", 5));
        store.add(make_instruction("Third instruction", 2));
        store.add(make_instruction("Fourth instruction", 10));

        assert!(store.len() <= 3);
        // Should keep the highest source_count ones
        let top = store.top(3);
        assert!(top.iter().any(|i| i.text == "Fourth instruction"));
        assert!(top.iter().any(|i| i.text == "Second instruction"));
    }

    #[test]
    fn test_top_ranking() {
        let mut store = InstructionStore::new(50, 1.0);
        store.add(make_instruction("Low priority", 1));
        store.add(make_instruction("High priority", 10));
        store.add(make_instruction("Medium priority", 5));

        let top = store.top(2);
        assert_eq!(top[0].text, "High priority");
        assert_eq!(top[1].text, "Medium priority");
    }

    #[test]
    fn test_empty_store() {
        let store = InstructionStore::new(50, 0.8);
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.top(5).is_empty());
    }

    #[test]
    fn test_export() {
        let mut store = InstructionStore::new(50, 0.8);
        store.add(make_instruction("Test instruction", 3));
        let exported = store.all();
        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].source_count, 3);
    }
}
