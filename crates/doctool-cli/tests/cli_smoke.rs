use std::process::Command;

fn dt_bin() -> &'static str {
    env!("CARGO_BIN_EXE_dt")
}

fn fixture_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("doctool-core/tests/fixtures/mini-monorepo")
}

#[test]
fn help_exits_zero() {
    let output = Command::new(dt_bin())
        .arg("--help")
        .output()
        .expect("spawn dt");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("scan"));
    assert!(stdout.contains("drift"));
    assert!(stdout.contains("translate-i18n"));
}

#[test]
fn translate_i18n_fixture_dry_run_exits_zero() {
    let root = fixture_root();
    let config = root.join("doctool.config.toml");
    let output = Command::new(dt_bin())
        .env("DOCTOOL_LLM_MOCK", "1")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "translate-i18n",
            "--dry-run",
        ])
        .output()
        .expect("spawn dt translate-i18n --dry-run");

    assert!(
        output.status.success(),
        "translate dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn scan_fixture_monorepo_json() {
    let root = fixture_root();
    let config = root.join("doctool.config.toml");
    let output = Command::new(dt_bin())
        .args([
            "--config",
            config.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--json",
            "scan",
        ])
        .output()
        .expect("spawn dt scan");

    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("code_element_count"));

    // Keep fixture tree clean (scan writes index/graph); preserve i18n.lock.
    let index_path = root.join(".doctool/index.json");
    let graph_path = root.join(".doctool/graph.json");
    let _ = std::fs::remove_file(index_path);
    let _ = std::fs::remove_file(graph_path);
}

#[test]
fn drift_fixture_exits_nonzero() {
    let root = fixture_root();
    let config = root.join("doctool.config.toml");
    let output = Command::new(dt_bin())
        .args([
            "--config",
            config.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--skip-ts",
            "drift",
        ])
        .output()
        .expect("spawn dt drift");

    assert!(
        !output.status.success(),
        "fixture drift should report issues"
    );
}

#[test]
fn sync_i18n_fixture_check_exits_nonzero() {
    let root = fixture_root();
    let config = root.join("doctool.config.toml");
    let output = Command::new(dt_bin())
        .args([
            "--config",
            config.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "sync-i18n",
            "--check",
        ])
        .output()
        .expect("spawn dt sync-i18n --check");

    assert!(
        !output.status.success(),
        "fixture i18n check should report issues"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("locale_gap") || stdout.contains("locale_stale"));
}

#[test]
fn graph_fixture_writes_json() {
    let root = fixture_root();
    let config = root.join("doctool.config.toml");
    let out = tempfile::NamedTempFile::new().expect("temp file");
    let output = Command::new(dt_bin())
        .args([
            "--config",
            config.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "graph",
            "--output",
            out.path().to_str().unwrap(),
        ])
        .output()
        .expect("spawn dt graph");

    assert!(output.status.success());
    let written = std::fs::read_to_string(out.path()).expect("graph output");
    assert!(written.contains("\"nodes\""));
}
