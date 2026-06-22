use std::path::Path;
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct GitProvenance {
    pub branch: Option<String>,
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,
}

/// Best-effort git metadata for translate/improve JSON output.
pub fn collect_git_provenance(monorepo_root: &Path) -> GitProvenance {
    let branch = git_output(monorepo_root, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit_hash = git_output(monorepo_root, &["rev-parse", "--short", "HEAD"]);
    let commit_message = git_output(monorepo_root, &["log", "-1", "--pretty=%s"]);

    GitProvenance {
        branch,
        commit_hash,
        commit_message,
    }
}

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).current_dir(cwd).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}
