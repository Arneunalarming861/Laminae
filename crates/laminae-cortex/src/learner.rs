//! LLM-powered edit analysis — converts edit diffs into reusable instructions.

use anyhow::{Context, Result};

use crate::store::LearnedInstruction;
use laminae_ollama::OllamaClient;

/// Analyzes edit diffs using a local LLM to generate reusable instructions.
pub struct EditLearner {
    client: OllamaClient,
    model: String,
}

impl EditLearner {
    /// Create a new learner with the specified Ollama model.
    ///
    /// Use a fast model (e.g., `qwen2.5:3b`) for low-latency analysis.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            client: OllamaClient::new(),
            model: model.into(),
        }
    }

    /// Create with a custom Ollama client.
    pub fn with_client(client: OllamaClient, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }

    /// Analyze an edit and generate a reusable instruction.
    ///
    /// Returns `None` if the edit is too minor or the LLM can't identify
    /// a clear pattern.
    pub async fn analyze(
        &self,
        original: &str,
        edited: &str,
    ) -> Result<Option<LearnedInstruction>> {
        if original.trim() == edited.trim() {
            return Ok(None);
        }

        let prompt = format!(
            "AI OUTPUT:\n{}\n\nUSER'S EDITED VERSION:\n{}\n\n\
             Compare these two texts. What specific change did the user make?\n\
             Write ONE short instruction (max 15 words) that captures this preference.\n\
             Examples: \"Never start replies with I think\", \"Keep under 2 sentences\", \
             \"Don't use em-dashes\", \"Be more direct, less hedging\"\n\n\
             If the change is trivial (typo fix, minor rewording), respond with: SKIP\n\n\
             Respond with ONLY the instruction or SKIP. No explanation.",
            truncate(original, 300),
            truncate(edited, 300),
        );

        let response = self
            .client
            .complete(
                &self.model,
                "You are a writing preference analyst. Extract one clear instruction from how a user edited AI output.",
                &prompt,
                0.3,
                100,
            )
            .await
            .context("LLM edit analysis failed")?;

        let instruction = response.trim().to_string();

        // Skip trivial edits
        if instruction.is_empty()
            || instruction.to_uppercase() == "SKIP"
            || instruction.len() < 5
            || instruction.len() > 200
        {
            return Ok(None);
        }

        Ok(Some(LearnedInstruction {
            text: instruction,
            source_count: 1,
            added: chrono::Utc::now(),
        }))
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello");
    }

    #[test]
    fn test_identical_returns_none() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let learner = EditLearner::new("test-model");
            let result = learner.analyze("same text", "same text").await;
            // Should return Ok(None) for identical text (no LLM call needed)
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        });
    }
}
