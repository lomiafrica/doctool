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

/// Copy the mini-monorepo fixture into a temp directory for mutating tests.
pub fn copy_mini_fixture_to(dest: &Path) {
    copy_dir_recursive(&mini_monorepo_root(), dest).expect("copy mini-monorepo fixture");
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dest.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
