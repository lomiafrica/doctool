use std::env;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_provider() -> String {
    "openai_compatible".into()
}

fn default_model() -> String {
    "gpt-4.1-mini".into()
}

fn default_base_url() -> String {
    "https://api.openai.com/v1".into()
}

fn default_temperature() -> f32 {
    0.2
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            base_url: default_base_url(),
            temperature: default_temperature(),
        }
    }
}

impl LlmConfig {
    pub fn resolve(&self) -> Self {
        let mut cfg = self.clone();
        if let Ok(model) = env::var("DOCTOOL_LLM_MODEL") {
            if !model.is_empty() {
                cfg.model = model;
            }
        }
        if let Ok(url) = env::var("DOCTOOL_LLM_BASE_URL") {
            if !url.is_empty() {
                cfg.base_url = url;
            }
        }
        cfg
    }

    pub fn api_key(&self) -> Option<String> {
        env::var("DOCTOOL_LLM_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .or_else(|| env::var("OPENAI_API_KEY").ok().filter(|k| !k.is_empty()))
    }

    pub fn use_mock(&self) -> bool {
        self.provider == "mock" || env::var("DOCTOOL_LLM_MOCK").ok().as_deref() == Some("1")
    }
}
