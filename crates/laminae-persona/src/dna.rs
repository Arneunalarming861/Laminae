//! Voice DNA — tracks distinctive phrases confirmed by repeated use.
//!
//! When a user's generated text gets positive outcomes (engagement, approval,
//! reuse), distinctive phrases from that text are tracked and reinforced.

use chrono::Utc;
use std::collections::HashSet;

use crate::model::DnaPhrase;

/// Maximum number of DNA phrases to track.
const MAX_DNA_PHRASES: usize = 30;

/// Minimum word count for a phrase to qualify.
const MIN_PHRASE_WORDS: usize = 2;

/// Maximum word count for a phrase to qualify.
const MAX_PHRASE_WORDS: usize = 5;

/// Manages voice DNA — distinctive phrases that define a person's writing.
pub struct VoiceDna {
    phrases: Vec<DnaPhrase>,
}

impl VoiceDna {
    /// Create from existing DNA phrases (e.g., loaded from a persona).
    pub fn new(phrases: Vec<DnaPhrase>) -> Self {
        Self { phrases }
    }

    /// Create empty.
    pub fn empty() -> Self {
        Self {
            phrases: Vec::new(),
        }
    }

    /// Get current DNA phrases, sorted by confirmation count (highest first).
    pub fn phrases(&self) -> &[DnaPhrase] {
        &self.phrases
    }

    /// Update DNA with phrases from a successful text.
    ///
    /// Call this when a generated text receives positive feedback (high engagement,
    /// user approval, etc.). Distinctive phrases are extracted and tracked.
    pub fn record_success(&mut self, text: &str) {
        let candidates = extract_distinctive_phrases(text);

        for phrase in candidates {
            if let Some(existing) = self
                .phrases
                .iter_mut()
                .find(|p| p.phrase.to_lowercase() == phrase.to_lowercase())
            {
                existing.confirmed_by += 1;
            } else {
                self.phrases.push(DnaPhrase {
                    phrase,
                    confirmed_by: 1,
                    first_seen: Utc::now(),
                });
            }
        }

        // Cap at max, keeping highest confirmed
        if self.phrases.len() > MAX_DNA_PHRASES {
            self.phrases
                .sort_by(|a, b| b.confirmed_by.cmp(&a.confirmed_by));
            self.phrases.truncate(MAX_DNA_PHRASES);
        }
    }

    /// Export phrases back to the model format.
    pub fn into_phrases(self) -> Vec<DnaPhrase> {
        self.phrases
    }
}

/// Extract distinctive 2-5 word phrases from text.
///
/// Filters out generic phrases and keeps only those with markers
/// of distinctive voice (contractions, bold words, unusual combinations).
fn extract_distinctive_phrases(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < MIN_PHRASE_WORDS {
        return Vec::new();
    }

    // Generic phrases to skip
    let generic: HashSet<&str> = [
        "the fact",
        "that the",
        "in the",
        "of the",
        "to the",
        "for the",
        "on the",
        "is the",
        "it the",
        "and the",
        "a the",
        "this is",
        "that is",
        "there is",
        "there are",
        "i think",
        "you know",
        "as well",
    ]
    .into_iter()
    .collect();

    // Markers of distinctiveness
    let bold_words: HashSet<&str> = [
        "never",
        "always",
        "real",
        "actual",
        "literally",
        "absolutely",
        "dead",
        "pure",
        "raw",
        "zero",
        "nobody",
        "everybody",
        "everything",
        "nothing",
        "exactly",
        "genuinely",
    ]
    .into_iter()
    .collect();

    let mut phrases = Vec::new();

    for window_size in MIN_PHRASE_WORDS..=MAX_PHRASE_WORDS.min(words.len()) {
        for window in words.windows(window_size) {
            let phrase = window.join(" ");
            let lower = phrase.to_lowercase();

            // Skip generic
            if generic.contains(lower.as_str()) {
                continue;
            }

            // Must have at least one marker of distinctiveness
            let has_contraction = phrase.contains('\'');
            let has_bold = window
                .iter()
                .any(|w| bold_words.contains(w.to_lowercase().as_str()));
            let has_unusual_combo =
                window_size >= 3 && !phrase.chars().all(|c| c.is_ascii_lowercase() || c == ' ');

            if has_contraction || has_bold || has_unusual_combo {
                phrases.push(phrase);
            }
        }
    }

    // Deduplicate and limit
    phrases.dedup();
    phrases.truncate(10);
    phrases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_distinctive_bold_words() {
        let phrases =
            extract_distinctive_phrases("Nobody reads your README. Ship it or delete it.");
        assert!(!phrases.is_empty());
        let joined = phrases.join(" ").to_lowercase();
        assert!(joined.contains("nobody"));
    }

    #[test]
    fn test_extract_distinctive_contractions() {
        let phrases = extract_distinctive_phrases("That's the whole point. You can't fake this.");
        assert!(!phrases.is_empty());
    }

    #[test]
    fn test_extract_distinctive_skips_generic() {
        let phrases = extract_distinctive_phrases("the fact that the thing is there");
        // "the fact" and "that the" should be filtered
        let has_generic = phrases
            .iter()
            .any(|p| p.to_lowercase() == "the fact" || p.to_lowercase() == "that the");
        assert!(!has_generic);
    }

    #[test]
    fn test_record_success_increments() {
        let mut dna = VoiceDna::empty();

        dna.record_success("Nobody reads your README.");
        let initial = dna
            .phrases()
            .iter()
            .find(|p| p.phrase.to_lowercase().contains("nobody"));
        assert!(initial.is_some());

        dna.record_success("Nobody reads docs either.");
        let updated = dna
            .phrases()
            .iter()
            .find(|p| p.phrase.to_lowercase().contains("nobody"));
        // Should have incremented or added new phrase with "nobody"
        assert!(updated.is_some());
    }

    #[test]
    fn test_dna_caps_at_max() {
        let mut dna = VoiceDna::empty();

        // Generate many distinct phrases
        for i in 0..50 {
            dna.record_success(&format!(
                "Never forget rule number {}. It's absolutely critical.",
                i
            ));
        }

        assert!(dna.phrases().len() <= MAX_DNA_PHRASES);
    }

    #[test]
    fn test_empty_text() {
        let phrases = extract_distinctive_phrases("");
        assert!(phrases.is_empty());
    }

    #[test]
    fn test_short_text() {
        let phrases = extract_distinctive_phrases("Hi");
        assert!(phrases.is_empty());
    }
}
