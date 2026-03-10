//! Post-generation voice filter — catches AI-sounding output.
//!
//! The voice filter is a multi-layer rejection system that detects common
//! LLM writing patterns and either auto-fixes them or flags for retry.

use regex::Regex;
use std::sync::LazyLock;

/// Result of a voice filter check.
#[derive(Debug, Clone)]
pub struct VoiceCheckResult {
    /// Whether the text passed the filter.
    pub passed: bool,
    /// Mechanically cleaned version of the text.
    pub cleaned: String,
    /// Human-readable violation descriptions.
    pub violations: Vec<String>,
    /// Severity score (0 = clean, 1 = minor auto-fixed, 2 = should retry, 3 = hard fail).
    pub severity: u8,
    /// Targeted retry hints for each violation (for LLM re-generation).
    pub retry_hints: Vec<String>,
}

/// Configuration for the voice filter.
#[derive(Debug, Clone)]
pub struct VoiceFilterConfig {
    /// Additional AI phrases to detect (appended to built-in list).
    pub extra_ai_phrases: Vec<String>,
    /// Maximum sentences allowed (0 = no limit).
    pub max_sentences: usize,
    /// Maximum characters allowed (0 = no limit).
    pub max_chars: usize,
    /// Whether to reject trailing questions.
    pub reject_trailing_questions: bool,
    /// Whether to replace em-dashes with periods.
    pub fix_em_dashes: bool,
    /// Whether to reject multi-paragraph output.
    pub reject_multi_paragraph: bool,
}

impl Default for VoiceFilterConfig {
    fn default() -> Self {
        Self {
            extra_ai_phrases: Vec::new(),
            max_sentences: 0,
            max_chars: 0,
            reject_trailing_questions: true,
            fix_em_dashes: true,
            reject_multi_paragraph: false,
        }
    }
}

/// Post-generation voice filter that catches AI-sounding output.
///
/// Runs 6 detection layers in sequence, auto-fixing where possible
/// and flagging severity for retry decisions.
pub struct VoiceFilter {
    config: VoiceFilterConfig,
}

impl VoiceFilter {
    /// Create a new voice filter with the given configuration.
    pub fn new(config: VoiceFilterConfig) -> Self {
        Self { config }
    }

    /// Check text for AI-sounding patterns.
    ///
    /// Returns a result with violations, cleaned text, and retry hints.
    pub fn check(&self, text: &str) -> VoiceCheckResult {
        let mut cleaned = text.to_string();
        let mut violations = Vec::new();
        let mut hints = Vec::new();
        let mut max_severity: u8 = 0;

        // Layer 1: AI vocabulary detection
        self.check_ai_vocabulary(&cleaned, &mut violations, &mut hints, &mut max_severity);

        // Layer 2: Meta-commentary openers
        cleaned =
            self.check_meta_commentary(&cleaned, &mut violations, &mut hints, &mut max_severity);

        // Layer 3: Multi-paragraph structure
        if self.config.reject_multi_paragraph {
            cleaned = self.check_multi_paragraph(
                &cleaned,
                &mut violations,
                &mut hints,
                &mut max_severity,
            );
        }

        // Layer 4: Trailing questions
        if self.config.reject_trailing_questions {
            cleaned = self.check_trailing_questions(
                &cleaned,
                &mut violations,
                &mut hints,
                &mut max_severity,
            );
        }

        // Layer 5: Em-dashes
        if self.config.fix_em_dashes {
            cleaned =
                self.check_em_dashes(&cleaned, &mut violations, &mut hints, &mut max_severity);
        }

        // Layer 6: Length violations
        cleaned = self.check_length(&cleaned, &mut violations, &mut hints, &mut max_severity);

        VoiceCheckResult {
            passed: max_severity < 2,
            cleaned,
            violations,
            severity: max_severity,
            retry_hints: hints,
        }
    }

    fn check_ai_vocabulary(
        &self,
        text: &str,
        violations: &mut Vec<String>,
        hints: &mut Vec<String>,
        severity: &mut u8,
    ) {
        let lower = text.to_lowercase();

        let mut found = Vec::new();
        for phrase in AI_VOCABULARY.iter() {
            if lower.contains(phrase) {
                found.push(*phrase);
            }
        }

        // Check extra phrases from config
        for phrase in &self.config.extra_ai_phrases {
            if lower.contains(&phrase.to_lowercase()) {
                found.push(phrase.as_str());
            }
        }

        if !found.is_empty() {
            *severity = (*severity).max(2);
            violations.push(format!("AI vocabulary detected: {}", found.join(", ")));
            hints.push(
                "DO NOT use formal/academic language. Write like a person texting, not an analyst."
                    .to_string(),
            );
        }
    }

    fn check_meta_commentary(
        &self,
        text: &str,
        violations: &mut Vec<String>,
        hints: &mut Vec<String>,
        severity: &mut u8,
    ) -> String {
        static META_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?i)^(the|this)\s+(tweet|post|article|comment|message|text|statement|piece|thread|response)\s+(highlights?|shows?|discusses?|suggests?|raises?|mentions?|points?\s+out|illustrates?|demonstrates?|reveals?|underscores?|emphasizes?|captures?)",
            )
            .unwrap()
        });

        if let Some(m) = META_RE.find(text) {
            *severity = (*severity).max(2);
            violations.push("Meta-commentary opener detected".to_string());
            hints.push(
                "Talk TO the person, not ABOUT their text. No 'The post shows...' — state your take directly."
                    .to_string(),
            );

            // Auto-strip the meta opener
            let rest = text[m.end()..].trim_start();
            let mut cleaned = rest.to_string();
            if let Some(first_char) = cleaned.chars().next() {
                cleaned = format!(
                    "{}{}",
                    first_char.to_uppercase(),
                    &cleaned[first_char.len_utf8()..]
                );
            }
            cleaned
        } else {
            text.to_string()
        }
    }

    fn check_multi_paragraph(
        &self,
        text: &str,
        violations: &mut Vec<String>,
        hints: &mut Vec<String>,
        severity: &mut u8,
    ) -> String {
        if text.contains("\n\n") {
            *severity = (*severity).max(2);
            violations.push("Multi-paragraph structure detected".to_string());
            hints.push("ONE paragraph only. 1-2 sentences. That's it.".to_string());
            text.replace("\n\n", " ")
        } else {
            text.to_string()
        }
    }

    fn check_trailing_questions(
        &self,
        text: &str,
        violations: &mut Vec<String>,
        hints: &mut Vec<String>,
        severity: &mut u8,
    ) -> String {
        let trimmed = text.trim();
        if !trimmed.ends_with('?') {
            return text.to_string();
        }

        // Check for generic AI questions
        static GENERIC_Q: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)(how will|what does|what do you|thoughts\??|wouldn't you|don't you think|isn't it|right\?)\s*$").unwrap()
        });

        if GENERIC_Q.is_match(trimmed) {
            *severity = (*severity).max(2);
        } else {
            *severity = (*severity).max(1);
        }

        violations.push("Trailing question detected".to_string());
        hints.push(
            "Do NOT end with ANY question. No 'How will...?'. Just state your take and STOP."
                .to_string(),
        );

        // Auto-strip trailing question sentence
        let sentences: Vec<&str> = trimmed
            .split(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .collect();

        if sentences.len() > 1 {
            // Keep all but the last sentence
            let last_q_start = trimmed.rfind(['.', '!']);
            if let Some(pos) = last_q_start {
                return trimmed[..=pos].trim().to_string();
            }
        }

        text.to_string()
    }

    fn check_em_dashes(
        &self,
        text: &str,
        violations: &mut Vec<String>,
        hints: &mut Vec<String>,
        severity: &mut u8,
    ) -> String {
        if text.contains('—') || text.contains(" -- ") {
            *severity = (*severity).max(1);
            violations.push("Em-dash usage detected".to_string());
            hints.push("No em-dashes. Use periods instead.".to_string());

            text.replace(" — ", ". ")
                .replace("— ", ". ")
                .replace(" —", ". ")
                .replace('—', ". ")
                .replace(" -- ", ". ")
        } else {
            text.to_string()
        }
    }

    fn check_length(
        &self,
        text: &str,
        violations: &mut Vec<String>,
        hints: &mut Vec<String>,
        severity: &mut u8,
    ) -> String {
        let mut result = text.to_string();

        // Check character limit
        if self.config.max_chars > 0 && result.len() > self.config.max_chars {
            *severity = (*severity).max(1);
            violations.push(format!(
                "Text exceeds {} char limit ({} chars)",
                self.config.max_chars,
                result.len()
            ));
            hints.push("SHORTER. Be more concise.".to_string());
        }

        // Check sentence limit
        if self.config.max_sentences > 0 {
            let sentences: Vec<&str> = result
                .split(['.', '!', '?'])
                .filter(|s| !s.trim().is_empty())
                .collect();

            if sentences.len() > self.config.max_sentences {
                *severity = (*severity).max(1);
                violations.push(format!(
                    "Text has {} sentences (max {})",
                    sentences.len(),
                    self.config.max_sentences
                ));
                hints.push(format!(
                    "Max {} sentences. Cut the rest.",
                    self.config.max_sentences
                ));

                // Auto-truncate to max sentences
                let mut end_pos = 0;
                let mut count = 0;
                for (i, c) in result.char_indices() {
                    if c == '.' || c == '!' || c == '?' {
                        count += 1;
                        end_pos = i + c.len_utf8();
                        if count >= self.config.max_sentences {
                            break;
                        }
                    }
                }
                if end_pos > 0 && end_pos < result.len() {
                    result = result[..end_pos].trim().to_string();
                }
            }
        }

        result
    }
}

/// Built-in AI vocabulary — phrases that betray LLM-generated text.
///
/// These are patterns consistently produced by ChatGPT, Claude, and similar
/// models that human writers almost never use.
static AI_VOCABULARY: &[&str] = &[
    // Academic hedging
    "it's important to note",
    "it's worth noting",
    "it is worth mentioning",
    "it bears mentioning",
    "it should be noted",
    "one could argue",
    "it remains to be seen",
    // Formal transitions
    "furthermore",
    "moreover",
    "in conclusion",
    "in summary",
    "to summarize",
    "overall",
    "that being said",
    "having said that",
    "with that in mind",
    "moving forward",
    "going forward",
    "at the end of the day",
    // AI-specific tells
    "underscores",
    "highlights the",
    "sends a clear message",
    "raises important questions",
    "sheds light on",
    "the significance of",
    "a testament to",
    "multifaceted",
    "resonates with",
    "navigating",
    "landscape",
    "tapestry",
    "delve",
    "delving",
    "leveraging",
    "fostering",
    "paradigm",
    "synergy",
    "holistic",
    "nuanced",
    "robust",
    "comprehensive",
    "encompasses",
    "facilitating",
    "pivotal",
    "imperative",
    "paramount",
    "intricate",
    "dynamic",
    "realm",
    // Engagement bait
    "let's break this down",
    "here's the thing",
    "here's why",
    "here's what you need to know",
    "the truth is",
    "the reality is",
    "the bottom line",
    "make no mistake",
    // Filler connectives
    "in essence",
    "in other words",
    "simply put",
    "to put it simply",
    "needless to say",
    "as a matter of fact",
    "by and large",
    "for all intents and purposes",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text_passes() {
        let filter = VoiceFilter::new(VoiceFilterConfig::default());
        let result = filter.check("Ship fast, break things. That's the only way.");
        assert!(result.passed);
        assert!(result.violations.is_empty());
        assert_eq!(result.severity, 0);
    }

    #[test]
    fn test_ai_vocabulary_detected() {
        let filter = VoiceFilter::new(VoiceFilterConfig::default());
        let result = filter.check("It's important to note that shipping fast is crucial. Furthermore, the landscape of development is multifaceted.");
        assert!(!result.passed);
        assert_eq!(result.severity, 2);
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn test_meta_commentary_stripped() {
        let filter = VoiceFilter::new(VoiceFilterConfig::default());
        let result = filter.check("The post highlights an important trend in tech.");
        assert_eq!(result.severity, 2);
        assert!(
            result.cleaned.starts_with("An important")
                || result.cleaned.starts_with("an important")
        );
    }

    #[test]
    fn test_trailing_question_detected() {
        let filter = VoiceFilter::new(VoiceFilterConfig::default());
        let result = filter.check("Strong point. What do you think?");
        assert!(result.violations.iter().any(|v| v.contains("question")));
    }

    #[test]
    fn test_em_dash_replaced() {
        let filter = VoiceFilter::new(VoiceFilterConfig::default());
        let result = filter.check("China is winning — and nobody sees it.");
        assert!(result.cleaned.contains(". "));
        assert!(!result.cleaned.contains('—'));
    }

    #[test]
    fn test_multi_paragraph_joined() {
        let config = VoiceFilterConfig {
            reject_multi_paragraph: true,
            ..Default::default()
        };
        let filter = VoiceFilter::new(config);
        let result = filter.check("Point one.\n\nPoint two.");
        assert!(!result.cleaned.contains("\n\n"));
    }

    #[test]
    fn test_sentence_limit() {
        let config = VoiceFilterConfig {
            max_sentences: 2,
            ..Default::default()
        };
        let filter = VoiceFilter::new(config);
        let result =
            filter.check("First sentence. Second sentence. Third sentence. Fourth sentence.");
        let sentence_count = result
            .cleaned
            .split(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .count();
        assert!(sentence_count <= 2);
    }

    #[test]
    fn test_char_limit() {
        let config = VoiceFilterConfig {
            max_chars: 50,
            ..Default::default()
        };
        let filter = VoiceFilter::new(config);
        let long_text = "This is a very long piece of text that definitely exceeds the fifty character limit we set.";
        let result = filter.check(long_text);
        assert!(result.violations.iter().any(|v| v.contains("char limit")));
    }

    #[test]
    fn test_custom_ai_phrases() {
        let config = VoiceFilterConfig {
            extra_ai_phrases: vec!["synergize the workflow".into()],
            ..Default::default()
        };
        let filter = VoiceFilter::new(config);
        let result = filter.check("We need to synergize the workflow to achieve results.");
        assert!(!result.passed);
    }

    #[test]
    fn test_retry_hints_provided() {
        let filter = VoiceFilter::new(VoiceFilterConfig::default());
        let result =
            filter.check("The article highlights why it's important to note the significance of this paradigm shift. What do you think?");
        assert!(!result.retry_hints.is_empty());
        // Should have hints for AI vocab AND trailing question
        assert!(result.retry_hints.len() >= 2);
    }
}
