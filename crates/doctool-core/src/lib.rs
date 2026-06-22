pub mod config;
pub mod diff;
pub mod drift;
pub mod graph;
pub mod i18n;
pub mod improve;
pub mod index;
pub mod intel;
pub mod llm;
pub mod provenance;
pub mod rerank;
pub mod snapshot;
pub mod sources;

pub use config::{find_monorepo_root, DoctoolConfig, I18nConfig};
pub use drift::{build_drift_report, build_next_steps, merge_ts_errors, DriftIssue, DriftReport};
pub use graph::{build_knowledge_graph, KnowledgeGraph};
pub use diff::{diff_text, run_diff, DiffFormat, DiffReport};
pub use i18n::{
    run_sync_i18n, run_translate_i18n, LockFileManager, SyncI18nOptions, SyncI18nReport,
    TranslateI18nOptions, TranslateI18nReport,
};
pub use improve::{run_improve, ImproveOptions, ImproveReport};
pub use llm::LlmConfig;
pub use provenance::GitProvenance;
pub use index::CodeIndex;
pub use snapshot::{DoctoolEngine, DoctoolSnapshot};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn loads_default_config() {
        let config = DoctoolConfig::default();
        assert!(config.code_roots.contains(&"apps/api".to_string()));
    }

    #[test]
    fn openapi_operation_key_format() {
        use crate::sources::openapi::{operations_key, OpenApiOperation};
        let op = OpenApiOperation {
            method: "POST".into(),
            path: "/refunds".into(),
            operation_id: None,
            summary: None,
        };
        assert_eq!(operations_key(&op), "POST /refunds");
    }

    #[test]
    fn find_monorepo_from_apps_doctool() {
        let doctool_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let monorepo = doctool_dir
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .expect("doctool crate path");
        let root = find_monorepo_root(monorepo).expect("monorepo root");
        assert!(root.join("apps/docs/package.json").is_file());
    }
}
