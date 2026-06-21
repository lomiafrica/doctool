mod support;

use support::{
    assert_fixture_root_exists, categories_in_report, issues_with_category, load_fixture_config,
    mini_monorepo_root,
};

use doctool_core::{build_drift_report, find_monorepo_root, merge_ts_errors};

#[test]
fn fixture_mini_monorepo_exists() {
    let root = mini_monorepo_root();
    assert_fixture_root_exists(&root);
}

#[test]
fn find_monorepo_root_from_fixture() {
    let root = mini_monorepo_root();
    let found = find_monorepo_root(&root).expect("fixture should resolve as monorepo root");
    assert_eq!(found, root);
}

#[test]
fn drift_detects_all_rust_categories_on_fixture() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = build_drift_report(&config, &root).expect("drift report");

    let cats: Vec<&str> = categories_in_report(&report);
    assert!(
        cats.contains(&"missing_endpoint"),
        "expected missing_endpoint (POST /refunds), got: {cats:?}"
    );
    assert!(
        cats.contains(&"orphan_doc"),
        "expected orphan_doc (GET /orphan/only-in-docs), got: {cats:?}"
    );
    assert!(
        cats.contains(&"locale_gap"),
        "expected locale_gap (getting-started.mdx), got: {cats:?}"
    );
    assert!(
        cats.contains(&"guide_dead_link"),
        "expected guide_dead_link, got: {cats:?}"
    );
    assert!(
        cats.contains(&"sdk_unmentioned"),
        "expected sdk_unmentioned, got: {cats:?}"
    );
}

#[test]
fn drift_missing_endpoint_names_post_refunds() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = build_drift_report(&config, &root).unwrap();

    let missing = issues_with_category(&report, "missing_endpoint");
    assert_eq!(missing.len(), 1);
    assert!(missing[0].message.contains("POST /refunds"));
}

#[test]
fn drift_orphan_doc_points_at_fixture_file() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = build_drift_report(&config, &root).unwrap();

    let orphan = issues_with_category(&report, "orphan_doc");
    assert_eq!(orphan.len(), 1);
    assert_eq!(
        orphan[0].file.as_deref(),
        Some("api/orphan/OrphanController_only.mdx")
    );
}

#[test]
fn drift_dead_link_targets_missing_slug() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = build_drift_report(&config, &root).unwrap();

    let dead = issues_with_category(&report, "guide_dead_link");
    assert_eq!(dead.len(), 1);
    assert!(dead[0].message.contains("api/missing/MissingController_action"));
}

#[test]
fn merge_ts_errors_splits_stderr_lines() {
    let stderr = "error: missing page\nwarning: stale slug\n";
    let merged = merge_ts_errors(stderr);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].category, "ts_drift");
    assert!(merged[0].message.contains("missing page"));
}

#[test]
fn drift_report_is_not_clean_on_fixture() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = build_drift_report(&config, &root).unwrap();
    assert!(!report.is_clean());
    assert!(report.issue_count >= 5);
}
