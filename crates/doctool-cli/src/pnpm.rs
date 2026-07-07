use std::process::Command;

use anyhow::{bail, Context, Result};

/// pnpm on Windows is installed as `pnpm.cmd`; the extensionless `pnpm` shim is a shell script.
fn pnpm_program() -> &'static str {
    if cfg!(windows) {
        "pnpm.cmd"
    } else {
        "pnpm"
    }
}

pub fn run_pnpm_in_docs(monorepo_root: &std::path::Path, script: &str) -> Result<()> {
    let (ok, _) = run_pnpm_in_docs_capture(monorepo_root, script)?;
    if !ok {
        bail!("pnpm {script} failed in apps/docs");
    }
    Ok(())
}

pub fn run_pnpm_in_docs_capture(monorepo_root: &std::path::Path, script: &str) -> Result<(bool, String)> {
    let docs_dir = monorepo_root.join("apps/docs");
    let output = Command::new(pnpm_program())
        .arg(script)
        .current_dir(&docs_dir)
        .output()
        .with_context(|| format!("Failed to run pnpm {script} in {}", docs_dir.display()))?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let combined = format!("{stdout}\n{stderr}");
    Ok((output.status.success(), combined))
}

pub fn run_scaffold(monorepo_root: &std::path::Path) -> Result<()> {
    let docs_dir = monorepo_root.join("apps/docs");
    let status = Command::new(pnpm_program())
        .arg("run")
        .arg("api:regenerate-rest-reference")
        .env("CONFIRM_BOOTSTRAP", "1")
        .current_dir(&docs_dir)
        .status()
        .with_context(|| format!("Failed to run scaffold in {}", docs_dir.display()))?;

    if !status.success() {
        bail!("scaffold command failed in apps/docs");
    }
    Ok(())
}

pub fn run_docs_drift_capture(monorepo_root: &std::path::Path) -> Result<(bool, String)> {
    let docs_dir = monorepo_root.join("apps/docs");
    let output = Command::new(pnpm_program())
        .args(["exec", "tsx", "lib/scripts/docs-drift.ts"])
        .current_dir(&docs_dir)
        .output()
        .with_context(|| "Failed to run docs:drift")?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let combined = format!("{stdout}\n{stderr}");
    Ok((output.status.success(), combined))
}
