//! Core data model for a writing persona.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A complete writing persona extracted from text samples.
///
/// Contains everything needed to make an LLM write like a specific person:
/// voice characteristics, identity context, style rules, and example text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// Metadata about this persona.
    pub meta: PersonaMeta,
    /// Who this person is (context, not style).
    pub identity: PersonaIdentity,
    /// How this person writes (the core of the persona).
    pub voice: PersonaVoice,
    /// Hard rules for generation.
    pub rules: PersonaRules,
}

/// Persona metadata — versioning, source, timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaMeta {
    /// Unique identifier.
    pub id: String,
    /// Display name for this persona.
    pub name: String,
    /// Schema version (for migration).
    pub version: u32,
    /// When this persona was first created.
    pub created_at: DateTime<Utc>,
    /// When this persona was last updated.
    pub updated_at: DateTime<Utc>,
    /// How the persona was created.
    pub source: PersonaSource,
    /// Number of text samples analyzed.
    pub samples_analyzed: usize,
}

/// How a persona was created.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PersonaSource {
    /// Automatically extracted from text samples.
    Extracted,
    /// Manually defined by a user.
    Manual,
    /// Incrementally updated from new samples.
    Refreshed,
}

/// Identity context — who the person is, not how they write.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonaIdentity {
    /// One-sentence description.
    pub bio: String,
    /// Topics they demonstrate expertise in (2-4).
    #[serde(default)]
    pub expertise: Vec<String>,
    /// Their unique lens on the world.
    #[serde(default)]
    pub perspective: String,
}

/// The 7 dimensions of writing voice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaVoice {
    /// 3-5 adjectives describing their tone (e.g., "sharp", "witty", "blunt").
    pub tone_words: Vec<String>,
    /// Free-text description of their writing style.
    pub writing_style: String,
    /// How they use humor (or "none detected").
    pub humor_style: String,
    /// How they express emotions in writing.
    pub emotional_range: String,
    /// Typical sentence length.
    pub sentence_length: SentenceLength,
    /// Punctuation habits.
    pub punctuation_style: String,
    /// 4-5 sentence coaching profile in 2nd person.
    /// This is the single most important field — it drives LLM style matching.
    pub voice_summary: String,
    /// Representative example texts (exact copies from samples).
    #[serde(default)]
    pub examples: Vec<String>,
    /// Distinctive phrases confirmed by reuse.
    #[serde(default)]
    pub voice_dna: Vec<DnaPhrase>,
}

/// Typical sentence length bucket.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SentenceLength {
    VeryShort,
    Short,
    #[default]
    Medium,
    Long,
}

/// A distinctive phrase confirmed by repeated appearance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnaPhrase {
    /// The phrase itself (2-5 words).
    pub phrase: String,
    /// How many times this phrase has been confirmed.
    pub confirmed_by: u32,
    /// When first detected.
    pub first_seen: DateTime<Utc>,
}

/// Hard rules that constrain generation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRules {
    /// Words to never use in generated output.
    #[serde(default)]
    pub banned_words: Vec<String>,
    /// Sentence patterns to avoid (regex-compatible).
    #[serde(default)]
    pub banned_patterns: Vec<String>,
    /// Maximum sentences per output.
    #[serde(default = "default_max_sentences")]
    pub max_sentences: usize,
    /// Maximum characters per output.
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
    /// Whether emoji are allowed.
    #[serde(default)]
    pub emoji_allowed: bool,
    /// Whether profanity is allowed.
    #[serde(default)]
    pub profanity_allowed: bool,
}

fn default_max_sentences() -> usize {
    4
}

fn default_max_chars() -> usize {
    500
}

impl Default for PersonaRules {
    fn default() -> Self {
        Self {
            banned_words: Vec::new(),
            banned_patterns: Vec::new(),
            max_sentences: default_max_sentences(),
            max_chars: default_max_chars(),
            emoji_allowed: false,
            profanity_allowed: false,
        }
    }
}

/// A weighted text sample for extraction.
///
/// If you have engagement data (likes, shares, etc.), set a weight > 1.0
/// to prioritize high-performing samples during extraction.
#[derive(Debug, Clone)]
pub struct WeightedSample {
    /// The text content.
    pub text: String,
    /// Engagement weight (default 1.0). Higher = more influence on persona.
    pub weight: f64,
}

impl From<String> for WeightedSample {
    fn from(text: String) -> Self {
        Self { text, weight: 1.0 }
    }
}

impl From<&str> for WeightedSample {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_string(),
            weight: 1.0,
        }
    }
}

impl WeightedSample {
    /// Create a weighted sample with custom engagement weight.
    pub fn with_weight(text: impl Into<String>, weight: f64) -> Self {
        Self {
            text: text.into(),
            weight,
        }
    }
}
