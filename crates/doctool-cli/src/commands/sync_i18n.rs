use std::path::Path;

use anyhow::{bail, Result};
use colored::Colorize;
use doctool_core::{run_sync_i18n, DoctoolConfig, SyncI18nOptions};

pub async fn run(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    json: bool,
    check: bool,
    dry_run: bool,
    scaffold_missing: bool,
    lock: bool,
) -> Result<()> {
    let options = SyncI18nOptions {
        check_only: check || dry_run,
        dry_run,
        scaffold_missing,
        refresh_lock: lock,
    };

    let report = run_sync_i18n(config, monorepo_root, &options)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report.drift)?);
        if !report.scaffolded.is_empty() {
            eprintln!(
                "scaffolded: {}",
                serde_json::to_string(&report.scaffolded)?
            );
        }
    } else if dry_run {
        println!("{}", "dt sync-i18n --dry-run".bold());
        println!("  Issues: {}", report.drift.issue_count);
        for issue in &report.drift.issues {
            println!("  [{}] {}", issue.category, issue.message);
        }
    } else if lock {
        println!("{}", "dt sync-i18n lock".bold());
        if report.lock_updated {
            println!("  {} Updated i18n lock file", "✓".green());
        }
    } else {
        println!("{}", "dt sync-i18n".bold());
        println!("  Issues: {}", report.drift.issue_count);
        for issue in &report.drift.issues {
            println!("  [{}] {}", issue.category, issue.message);
        }
        if !report.scaffolded.is_empty() {
            println!("  Scaffolded {} file(s)", report.scaffolded.len());
        }
    }

    if check && !report.drift.is_clean() {
        bail!(
            "i18n sync check failed ({} issue(s))",
            report.drift.issue_count
        );
    }

    if !json && report.drift.is_clean() && check {
        println!("  {} No i18n drift detected", "✓".green());
    }

    Ok(())
}
