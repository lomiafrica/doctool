use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::DoctoolConfig;
use crate::drift::{build_drift_report, DriftIssue};
use crate::graph::{build_knowledge_graph, KnowledgeGraph};
use crate::index::CodeIndex;
use crate::intel::ScanStats;
use crate::sources::competitors::{load_competitor_index, CompetitorIndex};
use crate::sources::mdx::load_mdx_index;
use crate::sources::mdx::MdxIndex;
use crate::sources::openapi::load_openapi;
use crate::sources::openapi::OpenApiIndex;
use crate::sources::sdk::load_sdk_index;
use crate::sources::sdk::SdkIndex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctoolSnapshot {
    pub scanned_at: String,
    pub monorepo_root: String,
    pub code_stats: Vec<ScanStats>,
    pub code_element_count: usize,
    pub openapi: OpenApiIndex,
    pub mdx: MdxIndex,
    pub sdk: Option<SdkIndex>,
    pub competitors: CompetitorIndex,
    pub knowledge_graph: KnowledgeGraph,
    pub drift_issues: Vec<DriftIssue>,
}

pub struct DoctoolEngine {
    pub config: DoctoolConfig,
    pub monorepo_root: std::path::PathBuf,
    pub code_index: CodeIndex,
    pub snapshot: Option<DoctoolSnapshot>,
}

impl DoctoolEngine {
    pub fn new(config: DoctoolConfig, monorepo_root: impl AsRef<Path>) -> Self {
        Self {
            config,
            monorepo_root: monorepo_root.as_ref().to_path_buf(),
            code_index: CodeIndex::new(),
            snapshot: None,
        }
    }

    pub fn scan(&mut self) -> Result<DoctoolSnapshot> {
        let roots = self.config.code_root_paths(&self.monorepo_root);
        let code_stats = self
            .code_index
            .scan_roots(&roots)
            .map_err(|e| anyhow::anyhow!(e))?;

        let openapi_path = self.config.resolve(&self.monorepo_root, &self.config.openapi);
        let docs_path = self.config.resolve(&self.monorepo_root, &self.config.docs_content);
        let sdk_path = self.config.resolve(&self.monorepo_root, &self.config.sdk_manifest);
        let competitors_path = self.config.resolve(&self.monorepo_root, &self.config.competitors);

        let openapi = load_openapi(&openapi_path)?;
        let mdx = load_mdx_index(&docs_path)?;
        let sdk = if sdk_path.is_file() {
            Some(load_sdk_index(&sdk_path)?)
        } else {
            None
        };
        let competitors = load_competitor_index(&competitors_path)?;
        let knowledge_graph = build_knowledge_graph(&openapi, &mdx, sdk.as_ref());
        let drift = build_drift_report(&self.config, &self.monorepo_root)?;

        let snapshot = DoctoolSnapshot {
            scanned_at: Utc::now().to_rfc3339(),
            monorepo_root: self.monorepo_root.to_string_lossy().to_string(),
            code_element_count: self.code_index.elements.len(),
            code_stats,
            openapi,
            mdx,
            sdk,
            competitors,
            knowledge_graph,
            drift_issues: drift.issues,
        };

        self.snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }

    pub fn save_snapshot(&self, snapshot: &DoctoolSnapshot) -> Result<()> {
        let index_path = self.config.resolve(&self.monorepo_root, &self.config.index_cache);
        let graph_path = self.config.resolve(&self.monorepo_root, &self.config.graph_cache);

        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let index_json = serde_json::to_string_pretty(snapshot)?;
        fs::write(&index_path, index_json)
            .with_context(|| format!("Failed to write {}", index_path.display()))?;

        let graph_json = serde_json::to_string_pretty(&snapshot.knowledge_graph)?;
        fs::write(&graph_path, graph_json)
            .with_context(|| format!("Failed to write {}", graph_path.display()))?;

        Ok(())
    }

    pub fn load_snapshot(&mut self) -> Result<DoctoolSnapshot> {
        let index_path = self.config.resolve(&self.monorepo_root, &self.config.index_cache);
        let raw = fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read snapshot at {}", index_path.display()))?;
        let snapshot: DoctoolSnapshot = serde_json::from_str(&raw)?;
        self.snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }
}
