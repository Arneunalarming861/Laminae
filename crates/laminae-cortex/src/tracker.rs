//! Edit tracking — records pairs of (AI output, user's version).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single edit record: what the AI generated vs what the user posted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditRecord {
    /// The original AI-generated text.
    pub original: String,
    /// The text the user actually used (possibly edited).
    pub edited: String,
    /// Whether the user modified the original.
    pub was_edited: bool,
    /// When this edit was recorded.
    pub timestamp: DateTime<Utc>,
    /// Length difference (negative = shortened, positive = expanded).
    pub length_delta: i64,
    /// Word count difference.
    pub word_delta: i64,
}

impl EditRecord {
    /// Create a new edit record, auto-detecting whether editing occurred.
    pub fn new(original: &str, edited: &str) -> Self {
        let was_edited = original.trim() != edited.trim();
        let orig_words = original.split_whitespace().count() as i64;
        let edit_words = edited.split_whitespace().count() as i64;

        Self {
            original: original.to_string(),
            edited: edited.to_string(),
            was_edited,
            timestamp: Utc::now(),
            length_delta: edited.len() as i64 - original.len() as i64,
            word_delta: edit_words - orig_words,
        }
    }

    /// Percentage of text that was removed (0.0 if expanded or unchanged).
    pub fn removal_pct(&self) -> f64 {
        if !self.was_edited || self.original.is_empty() {
            return 0.0;
        }
        let orig_len = self.original.len() as f64;
        let edit_len = self.edited.len() as f64;
        if edit_len >= orig_len {
            0.0
        } else {
            (orig_len - edit_len) / orig_len * 100.0
        }
    }

    /// Percentage of text that was added (0.0 if shortened or unchanged).
    pub fn addition_pct(&self) -> f64 {
        if !self.was_edited || self.original.is_empty() {
            return 0.0;
        }
        let orig_len = self.original.len() as f64;
        let edit_len = self.edited.len() as f64;
        if edit_len <= orig_len {
            0.0
        } else {
            (edit_len - orig_len) / orig_len * 100.0
        }
    }

    /// Word-level overlap between original and edited (0.0-1.0).
    pub fn word_overlap(&self) -> f64 {
        let orig_words: std::collections::HashSet<&str> =
            self.original.split_whitespace().collect();
        let edit_words: std::collections::HashSet<&str> = self.edited.split_whitespace().collect();

        if orig_words.is_empty() && edit_words.is_empty() {
            return 1.0;
        }

        let intersection = orig_words.intersection(&edit_words).count() as f64;
        let max_len = orig_words.len().max(edit_words.len()) as f64;
        if max_len == 0.0 {
            1.0
        } else {
            intersection / max_len
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_detected() {
        let record = EditRecord::new("Hello world", "Hello");
        assert!(record.was_edited);
        assert!(record.length_delta < 0);
    }

    #[test]
    fn test_no_edit_detected() {
        let record = EditRecord::new("Hello world", "Hello world");
        assert!(!record.was_edited);
    }

    #[test]
    fn test_removal_pct() {
        let record = EditRecord::new("This is a long sentence with many words", "Short");
        assert!(record.removal_pct() > 50.0);
    }

    #[test]
    fn test_addition_pct() {
        let record = EditRecord::new("Short", "Short text with many more words added here");
        assert!(record.addition_pct() > 100.0);
    }

    #[test]
    fn test_word_overlap() {
        let record = EditRecord::new("hello world foo bar", "hello world baz qux");
        let overlap = record.word_overlap();
        assert!(overlap > 0.3 && overlap < 0.7);
    }

    #[test]
    fn test_identical_overlap() {
        let record = EditRecord::new("same text", "same text");
        assert!((record.word_overlap() - 1.0).abs() < 0.01);
    }
}
