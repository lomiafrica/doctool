use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::DoctoolConfig;
use crate::sources::mdx::{
    all_mdx_content, documented_operations, find_internal_links, load_mdx_index,
    missing_french_siblings,
};
use crate::sources::openapi::{
    filter_public_operations, load_expected_public_operations, load_openapi, operations_key,
};
use crate::sources::sdk::{load_sdk_index, unmentioned_methods};
use crate::snapshot::DoctoolSnapshot;

use super::categories::DriftCategory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftIssue {
    pub category: String,
    pub message: String,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub issues: Vec<DriftIssue>,
    pub issue_count: usize,
    #[serde(default)]
    pub next_steps: Vec<String>,
}

impl DriftReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

pub fn build_next_steps(issues: &[DriftIssue]) -> Vec<String> {
    let mut steps = Vec::new();
    let mut seen = HashSet::new();

    for issue in issues {
        let cmd = match issue.category.as_str() {
            "missing_endpoint" => Some("dt scaffold"),
            "locale_gap" => Some("dt sync-i18n --scaffold-missing"),
            "locale_stale" => Some("dt sync-i18n lock"),
            "guide_dead_link" => Some("Fix dead internal links in MDX"),
            _ => None,
        };
        if let Some(step) = cmd {
            if seen.insert(step.to_string()) {
                steps.push(step.to_string());
            }
        }
    }
    steps
}

pub fn build_drift_report(config: &DoctoolConfig, monorepo_root: &Path) -> Result<DriftReport> {
    let openapi_path = config.resolve(monorepo_root, &config.openapi);
    let expected_path = config.resolve(monorepo_root, &config.expected_public_operations);
    let docs_path = config.resolve(monorepo_root, &config.docs_content);
    let sdk_path = config.resolve(monorepo_root, &config.sdk_manifest);

    let openapi = load_openapi(&openapi_path)?;
    let expected = load_expected_public_operations(&expected_path)?;
    let public_ops = filter_public_operations(&openapi, &expected);
    let mdx = load_mdx_index(&docs_path)?;
    let documented = documented_operations(&mdx.pages);

    let mut issues = Vec::new();

    for op in &public_ops {
        let key = operations_key(op);
        if !documented.contains_key(&key) {
            issues.push(DriftIssue {
                category: DriftCategory::MissingEndpoint.as_str().to_string(),
                message: format!("OpenAPI operation missing MDX: {key}"),
                file: None,
            });
        }
    }

    for (key, page) in &documented {
        if !public_ops.iter().any(|op| operations_key(op) == *key) {
            issues.push(DriftIssue {
                category: DriftCategory::OrphanDoc.as_str().to_string(),
                message: format!("MDX documents unknown or non-public OpenAPI operation: {key}"),
                file: Some(page.relative_path.clone()),
            });
        }
    }

    for path in missing_french_siblings(&mdx.pages) {
        issues.push(DriftIssue {
            category: DriftCategory::LocaleGap.as_str().to_string(),
            message: format!("Missing French sibling for {path}"),
            file: Some(path),
        });
    }

    if sdk_path.is_file() {
        let sdk = load_sdk_index(&sdk_path)?;
        let mdx_blob = all_mdx_content(&docs_path, &mdx.pages).unwrap_or_default();
        for method in unmentioned_methods(&sdk, &mdx_blob) {
            issues.push(DriftIssue {
                category: DriftCategory::SdkUnmentioned.as_str().to_string(),
                message: format!("SDK method not mentioned in docs: {}", method.qualified),
                file: None,
            });
        }
    }

    for (file, slug) in find_internal_links(&docs_path, &mdx.pages, &mdx.valid_slugs) {
        issues.push(DriftIssue {
            category: DriftCategory::GuideDeadLink.as_str().to_string(),
            message: format!("Dead internal link in {file}: /{slug}"),
            file: Some(file),
        });
    }

    let issue_count = issues.len();
    let next_steps = build_next_steps(&issues);
    Ok(DriftReport {
        issues,
        issue_count,
        next_steps,
    })
}

pub fn drift_from_snapshot(snapshot: &DoctoolSnapshot) -> DriftReport {
    let issues = snapshot.drift_issues.clone();
    let issue_count = issues.len();
    let next_steps = build_next_steps(&issues);
    DriftReport {
        issues,
        issue_count,
        next_steps,
    }
}

pub fn merge_ts_errors(ts_stderr: &str) -> Vec<DriftIssue> {
    ts_stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| DriftIssue {
            category: "ts_drift".to_string(),
            message: line.to_string(),
            file: None,
        })
        .collect()
}
