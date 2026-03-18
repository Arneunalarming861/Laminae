//! Edit pattern detection — identifies recurring edit behaviors.
//!
//! Analyzes a history of edit records to find patterns like:
//! "user always shortens output", "user removes trailing questions",
//! "user strips AI-sounding phrases".

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::tracker::EditRecord;

/// A detected edit pattern with frequency data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPattern {
    /// What kind of edit this is.
    pub pattern_type: PatternType,
    /// What percentage of edits exhibit this pattern.
    pub frequency_pct: f64,
    /// How many edits matched this pattern.
    pub count: usize,
    /// Up to 3 example (original, edited) pairs.
    pub examples: Vec<(String, String)>,
}

/// Categories of edit patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// User removes 20%+ of the text.
    Shortened,
    /// User removes a trailing question.
    RemovedQuestion,
    /// User completely rewrites the opener (first sentence, <30% overlap).
    RemovedOpener,
    /// User strips AI-sounding phrases.
    RemovedAiPhrases,
    /// User adds 30%+ more content.
    AddedContent,
    /// User replaces complex words with simpler ones.
    SimplifiedLanguage,
    /// User softens the tone.
    ChangedToneSofter,
    /// User makes the tone more aggressive/direct.
    ChangedToneStronger,
}

/// AI-sounding phrases that users commonly strip.
const AI_TELL_PHRASES: &[&str] = &[
    "it's worth noting",
    "it's important to note",
    "it should be noted",
    "moving forward",
    "at the end of the day",
    "the implications",
    "in this context",
    "it bears mentioning",
    "one could argue",
    "from this perspective",
    "underscores the",
    "highlights the importance",
    "a testament to",
    "needless to say",
    "in essence",
];

/// Words that indicate softer tone.
const SOFTER_WORDS: &[&str] = &[
    "perhaps", "maybe", "might", "could", "possibly", "somewhat", "slightly", "tend to",
    "it seems", "in a way",
];

/// Words that indicate stronger/more direct tone.
const STRONGER_WORDS: &[&str] = &[
    "never",
    "always",
    "absolutely",
    "definitely",
    "clearly",
    "obviously",
    "must",
    "need to",
    "wrong",
    "right",
    "exactly",
    "period",
];

/// Detect edit patterns from a collection of edit records.
///
/// Returns patterns that meet the minimum frequency threshold.
pub fn detect_patterns(edits: &[&EditRecord], min_frequency_pct: f64) -> Vec<EditPattern> {
    let total = edits.len();
    if total == 0 {
        return Vec::new();
    }

    let mut patterns = Vec::new();

    collect_pattern(
        edits,
        total,
        min_frequency_pct,
        PatternType::Shortened,
        |e| e.removal_pct() >= 20.0,
        &mut patterns,
    );

    collect_pattern(
        edits,
        total,
        min_frequency_pct,
        PatternType::RemovedQuestion,
        |e| e.original.trim().ends_with('?') && !e.edited.trim().ends_with('?'),
        &mut patterns,
    );

    collect_pattern(
        edits,
        total,
        min_frequency_pct,
        PatternType::RemovedOpener,
        |e| {
            let orig_first = first_sentence(&e.original);
            let edit_first = first_sentence(&e.edited);
            word_overlap(orig_first, edit_first) < 0.3
        },
        &mut patterns,
    );

    collect_pattern(
        edits,
        total,
        min_frequency_pct,
        PatternType::RemovedAiPhrases,
        |e| {
            let orig_lower = e.original.to_lowercase();
            let edit_lower = e.edited.to_lowercase();
            AI_TELL_PHRASES
                .iter()
                .any(|phrase| orig_lower.contains(phrase) && !edit_lower.contains(phrase))
        },
        &mut patterns,
    );

    collect_pattern(
        edits,
        total,
        min_frequency_pct,
        PatternType::AddedContent,
        |e| e.addition_pct() >= 30.0,
        &mut patterns,
    );

    collect_pattern(
        edits,
        total,
        min_frequency_pct,
        PatternType::ChangedToneSofter,
        |e| tone_shift(&e.original, &e.edited) == ToneShift::Softer,
        &mut patterns,
    );

    collect_pattern(
        edits,
        total,
        min_frequency_pct,
        PatternType::ChangedToneStronger,
        |e| tone_shift(&e.original, &e.edited) == ToneShift::Stronger,
        &mut patterns,
    );

    patterns
}

/// Build an [`EditPattern`] from matching edits and push it onto `out` if the
/// computed frequency meets `min_frequency_pct`.
fn collect_pattern<F>(
    edits: &[&EditRecord],
    total: usize,
    min_frequency_pct: f64,
    pattern_type: PatternType,
    predicate: F,
    out: &mut Vec<EditPattern>,
) where
    F: Fn(&&EditRecord) -> bool,
{
    let matched: Vec<_> = edits.iter().filter(|e| predicate(e)).collect();
    if matched.is_empty() {
        return;
    }
    let freq = matched.len() as f64 / total as f64 * 100.0;
    if freq < min_frequency_pct {
        return;
    }
    out.push(EditPattern {
        pattern_type,
        frequency_pct: freq,
        count: matched.len(),
        examples: matched
            .iter()
            .take(3)
            .map(|e| (truncate(&e.original, 80), truncate(&e.edited, 80)))
            .collect(),
    });
}

#[derive(PartialEq)]
enum ToneShift {
    Softer,
    Stronger,
    Neutral,
}

fn tone_shift(original: &str, edited: &str) -> ToneShift {
    let orig_lower = original.to_lowercase();
    let edit_lower = edited.to_lowercase();

    let orig_soft = SOFTER_WORDS
        .iter()
        .filter(|w| orig_lower.contains(**w))
        .count();
    let edit_soft = SOFTER_WORDS
        .iter()
        .filter(|w| edit_lower.contains(**w))
        .count();
    let orig_strong = STRONGER_WORDS
        .iter()
        .filter(|w| orig_lower.contains(**w))
        .count();
    let edit_strong = STRONGER_WORDS
        .iter()
        .filter(|w| edit_lower.contains(**w))
        .count();

    let soft_delta = edit_soft as i32 - orig_soft as i32;
    let strong_delta = edit_strong as i32 - orig_strong as i32;

    if soft_delta > 0 && strong_delta <= 0 {
        ToneShift::Softer
    } else if strong_delta > 0 && soft_delta <= 0 {
        ToneShift::Stronger
    } else {
        ToneShift::Neutral
    }
}

fn first_sentence(text: &str) -> &str {
    text.split(['.', '!', '?']).next().unwrap_or(text).trim()
}

fn word_overlap(a: &str, b: &str) -> f64 {
    let a_words: HashSet<&str> = a.split_whitespace().collect();
    let b_words: HashSet<&str> = b.split_whitespace().collect();
    if a_words.is_empty() && b_words.is_empty() {
        return 1.0;
    }
    let max_len = a_words.len().max(b_words.len()) as f64;
    if max_len == 0.0 {
        return 1.0;
    }
    a_words.intersection(&b_words).count() as f64 / max_len
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::EditRecord;

    fn make_edits(pairs: &[(&str, &str)]) -> Vec<EditRecord> {
        pairs.iter().map(|(o, e)| EditRecord::new(o, e)).collect()
    }

    #[test]
    fn test_detect_shortened() {
        let edits = make_edits(&[
            ("This is a very long sentence with many unnecessary words that nobody needs to read.", "Short and direct."),
            ("Another extremely verbose output that goes on and on without saying much at all.", "Concise version."),
            ("Yet another unnecessarily long AI generated response with padding everywhere.", "Third short one."),
        ]);
        let refs: Vec<&EditRecord> = edits.iter().collect();
        let patterns = detect_patterns(&refs, 10.0);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::Shortened));
    }

    #[test]
    fn test_detect_removed_question() {
        let edits = make_edits(&[
            ("Good point. What do you think?", "Good point."),
            (
                "Interesting take. How will this play out?",
                "Interesting take.",
            ),
            ("Normal text without questions.", "Normal text."),
        ]);
        let refs: Vec<&EditRecord> = edits.iter().collect();
        let patterns = detect_patterns(&refs, 10.0);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::RemovedQuestion));
    }

    #[test]
    fn test_detect_removed_ai_phrases() {
        let edits = make_edits(&[
            ("It's worth noting that Rust is fast.", "Rust is fast."),
            ("Moving forward, we should use Rust.", "We should use Rust."),
            ("At the end of the day, types matter.", "Types matter."),
        ]);
        let refs: Vec<&EditRecord> = edits.iter().collect();
        let patterns = detect_patterns(&refs, 10.0);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::RemovedAiPhrases));
    }

    #[test]
    fn test_detect_tone_stronger() {
        let edits = make_edits(&[
            ("This could perhaps work.", "This definitely works."),
            ("It might be useful.", "It's absolutely essential."),
            ("Maybe consider this approach.", "Always use this approach."),
        ]);
        let refs: Vec<&EditRecord> = edits.iter().collect();
        let patterns = detect_patterns(&refs, 10.0);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::ChangedToneStronger));
    }

    #[test]
    fn test_detect_added_content() {
        let edits = make_edits(&[
            ("Short.", "Short. But actually there's way more to say about this topic and here's my full take on why it matters for everyone."),
            ("Brief.", "Brief. However I want to expand significantly on this point because it deserves a thorough explanation."),
        ]);
        let refs: Vec<&EditRecord> = edits.iter().collect();
        let patterns = detect_patterns(&refs, 10.0);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::AddedContent));
    }

    #[test]
    fn test_empty_edits() {
        let patterns = detect_patterns(&[], 10.0);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_below_frequency_threshold() {
        let edits = make_edits(&[
            ("Original one.", "Edited one."),
            ("Original two.", "Original two."),     // not edited
            ("Original three.", "Original three."), // not edited
            ("Original four.", "Original four."),   // not edited
            ("Original five.", "Original five."),   // not edited
        ]);
        // Only 1 out of 1 edited — but asking for 50% min frequency on specific patterns
        let refs: Vec<&EditRecord> = edits.iter().collect();
        let patterns = detect_patterns(&refs, 50.0);
        // The one edit is just a simple text change, probably won't match 50% on any category
        // This verifies the threshold filtering works
        assert!(patterns.is_empty() || patterns.iter().all(|p| p.frequency_pct >= 50.0));
    }
}
