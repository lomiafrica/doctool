mod support;

use support::{categories_in_report, issues_with_category, load_fixture_config, mini_monorepo_root};

use doctool_core::{run_sync_i18n, SyncI18nOptions};

#[test]
fn sync_i18n_detects_gap_stale_structure_orphan_on_fixture() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = run_sync_i18n(
        &config,
        &root,
        &SyncI18nOptions {
            check_only: true,
            dry_run: true,
            scaffold_missing: false,
            refresh_lock: false,
        },
    )
    .expect("sync-i18n");

    let cats = categories_in_report(&report.drift);
    assert!(
        cats.contains(&"locale_gap"),
        "expected locale_gap, got {cats:?}"
    );
    assert!(
        cats.contains(&"locale_stale"),
        "expected locale_stale, got {cats:?}"
    );
    assert!(
        cats.contains(&"locale_structure"),
        "expected locale_structure, got {cats:?}"
    );
    assert!(
        cats.contains(&"locale_orphan"),
        "expected locale_orphan, got {cats:?}"
    );
}

#[test]
fn sync_i18n_stale_points_at_fixture_file() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = run_sync_i18n(
        &config,
        &root,
        &SyncI18nOptions {
            check_only: true,
            dry_run: true,
            scaffold_missing: false,
            refresh_lock: false,
        },
    )
    .unwrap();

    let stale = issues_with_category(&report.drift, "locale_stale");
    assert!(stale.iter().any(|i| {
        i.file.as_deref() == Some("build/guides/stale-en.mdx")
    }));
}

#[test]
fn drift_report_includes_next_steps() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = doctool_core::build_drift_report(&config, &root).unwrap();
    assert!(!report.next_steps.is_empty() || report.issues.is_empty());
    if !report.issues.is_empty() {
        assert!(report
            .next_steps
            .iter()
            .any(|s| s.contains("dt scaffold") || s.contains("sync-i18n")));
    }
}
