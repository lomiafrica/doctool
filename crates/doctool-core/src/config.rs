use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

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
        }
    }
}

impl DoctoolConfig {
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
