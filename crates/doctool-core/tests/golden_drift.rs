//! Golden shape checks for drift report JSON (fixture).
mod support;

use support::{load_fixture_config, mini_monorepo_root};

use doctool_core::build_drift_report;

#[test]
fn drift_report_json_shape_on_fixture() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = build_drift_report(&config, &root).unwrap();

    let json = serde_json::to_value(&report).unwrap();
    assert!(json.get("issues").and_then(|v| v.as_array()).is_some());
    assert!(json.get("issue_count").and_then(|v| v.as_u64()).is_some());
    assert!(json.get("next_steps").and_then(|v| v.as_array()).is_some());

    for issue in &report.issues {
        assert!(!issue.category.is_empty());
        assert!(!issue.message.is_empty());
    }
}
