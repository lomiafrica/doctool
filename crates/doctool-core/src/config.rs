use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::llm::LlmConfig;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct I18nMdxConfig {
    #[serde(default = "default_i18n_include")]
    pub include: Vec<String>,
    #[serde(default = "default_i18n_exclude")]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct I18nConfig {
    #[serde(default = "default_i18n_source")]
    pub source: String,
    #[serde(default = "default_i18n_targets")]
    pub targets: Vec<String>,
    #[serde(default = "default_i18n_lock_cache")]
    pub lock_cache: String,
    #[serde(default)]
    pub mdx: I18nMdxConfig,
}

fn default_i18n_source() -> String {
    "en".into()
}

fn default_i18n_targets() -> Vec<String> {
    vec!["fr".into()]
}

fn default_i18n_lock_cache() -> String {
    ".doctool/i18n.lock".into()
}

fn default_i18n_include() -> Vec<String> {
    vec!["apps/docs/content/docs/**/*.mdx".into()]
}

fn default_i18n_exclude() -> Vec<String> {
    vec![
        "**/*.fr.mdx".into(),
        "**/*.es.mdx".into(),
        "**/*.zh.mdx".into(),
    ]
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            source: default_i18n_source(),
            targets: default_i18n_targets(),
            lock_cache: default_i18n_lock_cache(),
            mdx: I18nMdxConfig {
                include: default_i18n_include(),
                exclude: default_i18n_exclude(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DoctoolConfig {
    pub openapi: String,
    pub agent_openapi: String,
    pub docs_content: String,
    pub sdk_manifest: String,
    pub mcp_manifest: String,
    pub expected_public_operations: String,
    pub code_roots: Vec<String>,
    pub competitors: String,
    pub index_cache: String,
    pub graph_cache: String,
    #[serde(default)]
    pub i18n: Option<I18nConfig>,
    #[serde(default)]
    pub llm: Option<LlmConfig>,
}

impl Default for DoctoolConfig {
    fn default() -> Self {
        Self {
            openapi: "apps/docs/openapi.json".into(),
            agent_openapi: "apps/docs/agent-openapi.json".into(),
            docs_content: "apps/docs/content/docs".into(),
            sdk_manifest: "apps/sdks/ts/src/generated/sdk-public-methods.json".into(),
            mcp_manifest: "apps/mcp/src/generated/tools-manifest.json".into(),
            expected_public_operations:
                "apps/docs/lib/scripts/manual-api/_expected-public-operations.json".into(),
            code_roots: vec![
                "apps/api".into(),
                "apps/cli".into(),
                "apps/mcp".into(),
                "apps/sdks".into(),
            ],
            competitors: "apps/design/docs/competitors".into(),
            index_cache: ".doctool/index.json".into(),
            graph_cache: ".doctool/graph.json".into(),
            i18n: Some(I18nConfig::default()),
            llm: Some(LlmConfig::default()),
        }
    }
}

impl DoctoolConfig {
    pub fn i18n_config(&self) -> I18nConfig {
        self.i18n.clone().unwrap_or_default()
    }

    pub fn llm_config(&self) -> LlmConfig {
        self.llm.clone().unwrap_or_default().resolve()
    }

    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("Failed to read config {}", path.display()))?;
            return toml::from_str(&raw).context("Failed to parse doctool config TOML");
        }

        let candidates = [
            "doctool.config.toml",
            "apps/doctool/doctool.config.toml",
        ];

        for candidate in candidates {
            let path = PathBuf::from(candidate);
            if path.is_file() {
                let raw = fs::read_to_string(&path)?;
                return Ok(toml::from_str(&raw)?);
            }
        }

        Ok(Self::default())
    }

    pub fn resolve(&self, monorepo_root: &Path, relative: &str) -> PathBuf {
        monorepo_root.join(relative)
    }

    pub fn code_root_paths(&self, monorepo_root: &Path) -> Vec<PathBuf> {
        self.code_roots
            .iter()
            .map(|r| monorepo_root.join(r))
            .collect()
    }
}

pub fn find_monorepo_root(start: &Path) -> Result<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if dir.join("apps/docs/package.json").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!(
                "Could not find monorepo root (apps/docs/package.json) from {}",
                start.display()
            );
        }
    }
}
