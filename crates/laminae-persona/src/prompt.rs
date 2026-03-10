//! Compile a persona into a compact prompt block for any LLM.

use crate::model::Persona;

/// Compile a persona into a compact prompt string for LLM injection.
///
/// The output is designed for high fidelity in limited token budgets.
/// Place this in the system prompt, before any task-specific instructions.
///
/// # Example
///
/// ```rust
/// # use laminae_persona::*;
/// # use laminae_persona::compile_persona;
/// # use chrono::Utc;
/// let persona = Persona {
///     meta: PersonaMeta {
///         id: "test".into(),
///         name: "Test".into(),
///         version: 1,
///         created_at: Utc::now(),
///         updated_at: Utc::now(),
///         source: PersonaSource::Manual,
///         samples_analyzed: 0,
///     },
///     identity: PersonaIdentity {
///         bio: "A Rust developer".into(),
///         expertise: vec!["systems programming".into()],
///         perspective: "Pragmatic minimalist".into(),
///     },
///     voice: PersonaVoice {
///         tone_words: vec!["direct".into(), "blunt".into()],
///         writing_style: "Short punchy sentences".into(),
///         humor_style: "Dry sarcasm".into(),
///         emotional_range: "Controlled intensity".into(),
///         sentence_length: SentenceLength::Short,
///         punctuation_style: "Heavy periods".into(),
///         voice_summary: "You write like a telegram. Every word earns its place.".into(),
///         examples: vec!["Ship it.".into()],
///         voice_dna: vec![],
///     },
///     rules: PersonaRules::default(),
/// };
///
/// let prompt = compile_persona(&persona);
/// assert!(prompt.contains("VOICE DNA"));
/// assert!(prompt.contains("direct, blunt"));
/// ```
pub fn compile_persona(persona: &Persona) -> String {
    let mut sections = Vec::new();

    // Voice DNA section (highest priority — placed first)
    sections.push(format!(
        "--- VOICE DNA (highest priority) ---\n{}",
        persona.voice.voice_summary
    ));

    // Add confirmed DNA phrases if any
    if !persona.voice.voice_dna.is_empty() {
        let phrases: Vec<&str> = persona
            .voice
            .voice_dna
            .iter()
            .map(|d| d.phrase.as_str())
            .collect();
        sections.push(format!(
            "VOICE DNA (phrases confirmed by repeated use): \"{}\"",
            phrases.join("\", \"")
        ));
    }

    sections.push("---".to_string());

    // Identity
    if !persona.identity.bio.is_empty() {
        sections.push(format!("WHO YOU ARE: {}", persona.identity.bio));
    }
    if !persona.identity.perspective.is_empty() {
        sections.push(format!("PERSPECTIVE: {}", persona.identity.perspective));
    }

    // Voice characteristics
    sections.push(format!(
        "YOUR VOICE: {}. {}",
        persona.voice.tone_words.join(", "),
        persona.voice.writing_style
    ));

    if persona.voice.humor_style != "none detected" && !persona.voice.humor_style.is_empty() {
        sections.push(format!("HUMOR: {}", persona.voice.humor_style));
    }
    if !persona.voice.emotional_range.is_empty() {
        sections.push(format!("EMOTION: {}", persona.voice.emotional_range));
    }
    if !persona.voice.punctuation_style.is_empty() {
        sections.push(format!("PUNCTUATION: {}", persona.voice.punctuation_style));
    }

    // Expertise
    if !persona.identity.expertise.is_empty() {
        sections.push(format!(
            "EXPERTISE: {}",
            persona.identity.expertise.join(", ")
        ));
    }

    // Style reference examples
    if !persona.voice.examples.is_empty() {
        let examples: Vec<String> = persona
            .voice
            .examples
            .iter()
            .map(|e| format!("- {e}"))
            .collect();
        sections.push(format!(
            "STYLE REFERENCE (never copy, match this energy):\n{}",
            examples.join("\n")
        ));
    }

    // Rules
    if !persona.rules.banned_words.is_empty() {
        sections.push(format!(
            "BANNED WORDS (never use): {}",
            persona.rules.banned_words.join(", ")
        ));
    }
    if !persona.rules.banned_patterns.is_empty() {
        sections.push(format!(
            "BANNED PATTERNS (never write): {}",
            persona.rules.banned_patterns.join(", ")
        ));
    }

    if persona.rules.max_sentences > 0 && persona.rules.max_sentences < 100 {
        sections.push(format!(
            "MAX LENGTH: {} sentences, {} characters",
            persona.rules.max_sentences, persona.rules.max_chars
        ));
    }

    sections.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::Utc;

    fn test_persona() -> Persona {
        Persona {
            meta: PersonaMeta {
                id: "test-1".into(),
                name: "Test Persona".into(),
                version: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                source: PersonaSource::Extracted,
                samples_analyzed: 50,
            },
            identity: PersonaIdentity {
                bio: "A systems engineer who ships fast".into(),
                expertise: vec!["Rust".into(), "distributed systems".into()],
                perspective: "Pragmatic minimalist".into(),
            },
            voice: PersonaVoice {
                tone_words: vec!["sharp".into(), "direct".into(), "witty".into()],
                writing_style: "Short punchy sentences with no padding".into(),
                humor_style: "Dry sarcasm, never forced".into(),
                emotional_range: "Controlled intensity — calm until provoked".into(),
                sentence_length: SentenceLength::Short,
                punctuation_style: "Heavy periods, rare commas, occasional dashes".into(),
                voice_summary: "You write like a telegram operator with opinions. Every word earns its place. You never hedge.".into(),
                examples: vec![
                    "Ship it. Fix it later.".into(),
                    "Nobody reads your README.".into(),
                ],
                voice_dna: vec![DnaPhrase {
                    phrase: "ship it".into(),
                    confirmed_by: 5,
                    first_seen: Utc::now(),
                }],
            },
            rules: PersonaRules {
                banned_words: vec!["furthermore".into(), "moreover".into()],
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_compile_contains_voice_dna() {
        let prompt = compile_persona(&test_persona());
        assert!(prompt.contains("VOICE DNA"));
        assert!(prompt.contains("telegram operator"));
    }

    #[test]
    fn test_compile_contains_tone() {
        let prompt = compile_persona(&test_persona());
        assert!(prompt.contains("sharp, direct, witty"));
    }

    #[test]
    fn test_compile_contains_examples() {
        let prompt = compile_persona(&test_persona());
        assert!(prompt.contains("Ship it. Fix it later."));
        assert!(prompt.contains("STYLE REFERENCE"));
    }

    #[test]
    fn test_compile_contains_banned() {
        let prompt = compile_persona(&test_persona());
        assert!(prompt.contains("BANNED WORDS"));
        assert!(prompt.contains("furthermore"));
    }

    #[test]
    fn test_compile_contains_dna_phrases() {
        let prompt = compile_persona(&test_persona());
        assert!(prompt.contains("\"ship it\""));
    }

    #[test]
    fn test_compile_no_humor_when_none() {
        let mut persona = test_persona();
        persona.voice.humor_style = "none detected".into();
        let prompt = compile_persona(&persona);
        assert!(!prompt.contains("HUMOR:"));
    }

    #[test]
    fn test_compile_minimal_persona() {
        let persona = Persona {
            meta: PersonaMeta {
                id: "min".into(),
                name: "Minimal".into(),
                version: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                source: PersonaSource::Manual,
                samples_analyzed: 0,
            },
            identity: PersonaIdentity::default(),
            voice: PersonaVoice {
                tone_words: vec!["neutral".into()],
                writing_style: "Standard".into(),
                humor_style: String::new(),
                emotional_range: String::new(),
                sentence_length: SentenceLength::Medium,
                punctuation_style: String::new(),
                voice_summary: "You write plainly.".into(),
                examples: vec![],
                voice_dna: vec![],
            },
            rules: PersonaRules::default(),
        };
        let prompt = compile_persona(&persona);
        assert!(prompt.contains("You write plainly."));
        assert!(!prompt.contains("STYLE REFERENCE"));
    }
}
