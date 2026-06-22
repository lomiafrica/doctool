mod support;

use std::fs;

use support::{copy_mini_fixture_to, load_fixture_config, mini_monorepo_root, categories_in_report, issues_with_category};

use doctool_core::{run_sync_i18n, run_translate_i18n, SyncI18nOptions, TranslateI18nOptions};

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

#[tokio::test]
async fn translate_i18n_mock_merges_fr_segments() {
    std::env::set_var("DOCTOOL_LLM_MOCK", "1");

    let tmp = tempfile::tempdir().expect("tempdir");
    copy_mini_fixture_to(tmp.path());

    let config =
        doctool_core::DoctoolConfig::load(Some(&tmp.path().join("doctool.config.toml")))
            .expect("fixture config");

    let report = run_translate_i18n(
        &config,
        tmp.path(),
        &TranslateI18nOptions {
            check_only: false,
            dry_run: false,
            force: true,
            refresh_lock: false,
        },
    )
    .await
    .expect("translate-i18n");

    assert!(
        report
            .pages
            .iter()
            .any(|p| p.en_path == "build/guides/stale-en.mdx" && p.written),
        "expected stale-en translation page"
    );

    let fr_path = tmp
        .path()
        .join("apps/docs/content/docs/build/guides/stale-en.fr.mdx");
    let fr = fs::read_to_string(&fr_path).expect("read fr mdx");
    assert!(
        fr.contains("[FR]"),
        "mock LLM should prefix translated prose: {fr}"
    );
}

#[tokio::test]
async fn translate_i18n_check_reports_pending_on_fixture() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let report = run_translate_i18n(
        &config,
        &root,
        &TranslateI18nOptions {
            check_only: true,
            dry_run: false,
            force: false,
            refresh_lock: false,
        },
    )
    .await
    .expect("translate-i18n check");

    assert!(
        report.pending_segments > 0,
        "fixture lock should have pending translations"
    );
}
