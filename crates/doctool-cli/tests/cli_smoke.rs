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

    // Keep fixture tree clean (scan writes .doctool/).
    let doctool_dir = root.join(".doctool");
    let _ = std::fs::remove_dir_all(&doctool_dir);
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
