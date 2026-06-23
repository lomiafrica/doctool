//! Environment and monorepo readiness checks for `dt doctor`.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::config::DoctoolConfig;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DoctorStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub id: String,
    pub label: String,
    pub status: DoctorStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub ok_count: usize,
    pub warn_count: usize,
    pub fail_count: usize,
}

pub fn run_doctor(config: &DoctoolConfig, monorepo_root: &Path) -> DoctorReport {
    let mut checks = Vec::new();

    let docs_pkg = monorepo_root.join("apps/docs/package.json");
    push_check(
        &mut checks,
        "monorepo",
        "Monorepo root",
        if docs_pkg.is_file() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Fail
        },
        if docs_pkg.is_file() {
            format!("Found {}", docs_pkg.display())
        } else {
            "Missing apps/docs/package.json — pass --root or run from the lomi monorepo".into()
        },
    );

    let openapi = config.resolve(monorepo_root, &config.openapi);
    push_check(
        &mut checks,
        "openapi",
        "OpenAPI spec",
        if openapi.is_file() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Fail
        },
        if openapi.is_file() {
            openapi.display().to_string()
        } else {
            format!("Missing {}", openapi.display())
        },
    );

    let docs_content = config.resolve(monorepo_root, &config.docs_content);
    push_check(
        &mut checks,
        "docs_content",
        "Docs content",
        if docs_content.is_dir() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Fail
        },
        docs_content.display().to_string(),
    );

    let pnpm = command_version("pnpm", &["--version"]);
    push_check(
        &mut checks,
        "pnpm",
        "pnpm",
        if pnpm.is_some() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        pnpm.unwrap_or_else(|| "Not on PATH — required for `dt check` and `dt scaffold`".into()),
    );

    let i18n = config.i18n_config();
    let lock_path = config.resolve(monorepo_root, &i18n.lock_cache);
    let lock_exists = lock_path.is_file();
    push_check(
        &mut checks,
        "i18n_lock",
        "i18n.lock baseline",
        if lock_exists {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        if lock_exists {
            lock_path.display().to_string()
        } else {
            format!(
                "Missing {} — `dt translate-i18n` treats all EN segments as pending. Run `dt sync-i18n --lock` to create a baseline.",
                lock_path.display()
            )
        },
    );

    let index_path = config.resolve(monorepo_root, &config.index_cache);
    push_check(
        &mut checks,
        "scan_index",
        "Scan index cache",
        if index_path.is_file() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        if index_path.is_file() {
            index_path.display().to_string()
        } else {
            format!(
                "Missing {} — run `dt scan` to refresh the knowledge index",
                index_path.display()
            )
        },
    );

    let competitors = config.resolve(monorepo_root, &config.competitors);
    push_check(
        &mut checks,
        "competitors",
        "Competitor corpus",
        if competitors.is_dir() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        if competitors.is_dir() {
            competitors.display().to_string()
        } else {
            format!(
                "Missing {} — run `git submodule update --init apps/design` for RAG competitor context",
                competitors.display()
            )
        },
    );

    push_check(
        &mut checks,
        "reranker_model",
        "Reranker model (first run)",
        DoctorStatus::Warn,
        "First `dt suggest` or `dt improve` may download ~1 GB (BGE reranker via fastembed); cached afterward".into(),
    );

    let ok_count = checks
        .iter()
        .filter(|c| c.status == DoctorStatus::Ok)
        .count();
    let warn_count = checks
        .iter()
        .filter(|c| c.status == DoctorStatus::Warn)
        .count();
    let fail_count = checks
        .iter()
        .filter(|c| c.status == DoctorStatus::Fail)
        .count();

    DoctorReport {
        checks,
        ok_count,
        warn_count,
        fail_count,
    }
}

fn push_check(
    checks: &mut Vec<DoctorCheck>,
    id: &str,
    label: &str,
    status: DoctorStatus,
    detail: String,
) {
    checks.push(DoctorCheck {
        id: id.into(),
        label: label.into(),
        status,
        detail,
    });
}

fn command_version(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn doctor_fails_without_docs_package() {
        let tmp = tempfile::tempdir().unwrap();
        let config = DoctoolConfig::default();
        let report = run_doctor(&config, tmp.path());
        assert!(report.fail_count >= 1);
        assert!(report.checks.iter().any(|c| c.id == "monorepo" && c.status == DoctorStatus::Fail));
    }

    #[test]
    fn doctor_ok_on_fixture_layout() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture = manifest
            .join("tests/fixtures/mini-monorepo")
            .canonicalize()
            .expect("fixture");
        fs::create_dir_all(fixture.join(".doctool")).ok();
        let config = DoctoolConfig::load(Some(&fixture.join("doctool.config.toml"))).unwrap();
        let report = run_doctor(&config, &fixture);
        assert_eq!(report.checks.iter().find(|c| c.id == "monorepo").unwrap().status, DoctorStatus::Ok);
    }
}
