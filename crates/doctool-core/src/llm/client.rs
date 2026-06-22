use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

use super::config::LlmConfig;
use super::prompt::{create_improve_prompt, create_translate_prompt, ImprovePromptInput, TranslatePromptOptions};

#[derive(Debug, Clone)]
pub struct SegmentTranslation {
    pub key: String,
    pub translated_text: String,
}

#[derive(Debug, Clone)]
pub struct TranslateBatchResult {
    pub translations: Vec<SegmentTranslation>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn translate_segments(
        &self,
        segments: &[(String, String)],
        options: &TranslatePromptOptions,
    ) -> Result<Vec<String>>;
}

pub struct LlmClient {
    config: LlmConfig,
    http: reqwest::Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config: config.resolve(),
            http: reqwest::Client::new(),
        }
    }

    pub fn from_resolved(config: LlmConfig) -> Result<Box<dyn LlmProvider>> {
        if config.use_mock() {
            return Ok(Box::new(super::mock::MockLlmClient));
        }
        Ok(Box::new(Self::new(config)))
    }

    async fn chat_json(&self, prompt: &str) -> Result<String> {
        let api_key = self
            .config
            .api_key()
            .context("DOCTOOL_LLM_API_KEY or OPENAI_API_KEY required for LLM commands")?;

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let response = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .context("LLM API request failed")?;

        let status = response.status();
        let text = response.text().await.context("read LLM response body")?;
        if !status.is_success() {
            anyhow::bail!("LLM API error {status}: {text}");
        }

        let parsed: ChatCompletionResponse =
            serde_json::from_str(&text).context("parse LLM chat response")?;
        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .context("empty LLM response")?;
        Ok(content)
    }

    pub async fn improve_mdx(&self, input: &ImprovePromptInput) -> Result<String> {
        let api_key = self
            .config
            .api_key()
            .context("DOCTOOL_LLM_API_KEY or OPENAI_API_KEY required for LLM commands")?;

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let prompt = create_improve_prompt(input);
        let body = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let response = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .context("LLM improve request failed")?;

        let status = response.status();
        let text = response.text().await.context("read LLM response body")?;
        if !status.is_success() {
            anyhow::bail!("LLM API error {status}: {text}");
        }

        let parsed: ChatCompletionResponse =
            serde_json::from_str(&text).context("parse LLM chat response")?;
        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .context("empty LLM improve response")?;
        Ok(content.trim().to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranslateResponse {
    #[serde(alias = "translatedKeys", alias = "translated_keys")]
    translated_keys: Vec<String>,
}

#[async_trait]
impl LlmProvider for LlmClient {
    async fn translate_segments(
        &self,
        segments: &[(String, String)],
        options: &TranslatePromptOptions,
    ) -> Result<Vec<String>> {
        if segments.is_empty() {
            return Ok(vec![]);
        }

        let prompt = create_translate_prompt(segments, options);
        let raw = self.chat_json(&prompt).await?;
        let parsed: TranslateResponse =
            serde_json::from_str(&raw).context("parse translate JSON from LLM")?;

        if parsed.translated_keys.len() != segments.len() {
            anyhow::bail!(
                "LLM returned {} translations for {} segments",
                parsed.translated_keys.len(),
                segments.len()
            );
        }

        Ok(parsed.translated_keys)
    }
}

pub async fn translate_segments_batched(
    provider: &dyn LlmProvider,
    segments: &[(String, String)],
    options: &TranslatePromptOptions,
    chunk_size: usize,
) -> Result<Vec<SegmentTranslation>> {
    let mut out = Vec::new();
    for chunk in segments.chunks(chunk_size.max(1)) {
        let keys: Vec<String> = chunk.iter().map(|(k, _)| k.clone()).collect();
        let texts = provider.translate_segments(chunk, options).await?;
        for (key, text) in keys.into_iter().zip(texts) {
            out.push(SegmentTranslation {
                key,
                translated_text: text,
            });
        }
    }
    Ok(out)
}

pub async fn improve_mdx_content(
    config: &LlmConfig,
    input: &super::prompt::ImprovePromptInput,
) -> Result<String> {
    if config.use_mock() {
        return Ok(super::mock::mock_improve_content(&input.current_content));
    }

    let client = LlmClient::new(config.clone());
    client.improve_mdx(input).await
}
