//! Shared helpers for doctool-core integration tests.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use doctool_core::DoctoolConfig;

/// Root of the checked-in mini-monorepo fixture tree.
pub fn mini_monorepo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini-monorepo")
}

pub fn load_fixture_config() -> DoctoolConfig {
    let root = mini_monorepo_root();
    DoctoolConfig::load(Some(&root.join("doctool.config.toml"))).expect("fixture config")
}

pub fn categories_in_report(report: &doctool_core::DriftReport) -> Vec<&str> {
    report
        .issues
        .iter()
        .map(|i| i.category.as_str())
        .collect()
}

pub fn issues_with_category<'a>(
    report: &'a doctool_core::DriftReport,
    category: &str,
) -> Vec<&'a doctool_core::DriftIssue> {
    report
        .issues
        .iter()
        .filter(|i| i.category == category)
        .collect()
}

pub fn assert_fixture_root_exists(root: &Path) {
    assert!(
        root.join("apps/docs/package.json").is_file(),
        "fixture root missing apps/docs/package.json: {}",
        root.display()
    );
}
