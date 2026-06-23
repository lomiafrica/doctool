use anyhow::Result;
use async_trait::async_trait;

use super::client::LlmProvider;
use super::prompt::TranslatePromptOptions;

/// Deterministic mock for CI — prefixes translatable text with [FR].
pub struct MockLlmClient;

#[async_trait]
impl LlmProvider for MockLlmClient {
    async fn translate_segments(
        &self,
        segments: &[(String, String)],
        _options: &TranslatePromptOptions,
    ) -> Result<Vec<String>> {
        Ok(segments
            .iter()
            .map(|(_, text)| format!("[FR] {text}"))
            .collect())
    }
}

/// Mock improve: appends a marker comment at the end.
pub fn mock_improve_content(content: &str) -> String {
    format!("{content}\n\n{{/* improved by mock LLM */}}\n")
}

pub fn mock_suggest_plan(input: &super::prompt::SuggestLlmInput) -> super::prompt::SuggestLlmPlan {
    use super::prompt::SuggestRecommendation;

    let recommendations = input
        .next_steps
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, step)| SuggestRecommendation {
            priority: (i + 1) as u8,
            title: format!("Run {step}"),
            command: Some(step.clone()),
            rationale: "Deterministic drift fix from scan".into(),
        })
        .collect();

    super::prompt::SuggestLlmPlan {
        summary: format!(
            "Mock plan: {} issue categories detected. Execute listed commands first.",
            input.drift_summary
        ),
        recommendations,
    }
}
