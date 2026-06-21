use std::path::Path;

use anyhow::{bail, Result};
use colored::Colorize;
use doctool_core::{build_drift_report, merge_ts_errors, DoctoolConfig};

use crate::pnpm::run_docs_drift_capture;

pub async fn run(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    json: bool,
    skip_ts: bool,
) -> Result<()> {
    let mut report = build_drift_report(config, monorepo_root)?;

    if !skip_ts {
        let (ok, output) = run_docs_drift_capture(monorepo_root)?;
        if !ok {
            report.issues.extend(merge_ts_errors(&output));
            report.issue_count = report.issues.len();
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", "dt drift".bold());
        println!("  Issues: {}", report.issue_count);
        for issue in &report.issues {
            println!("  [{}] {}", issue.category, issue.message);
        }
    }

    if !report.is_clean() {
        bail!("Docs drift check failed ({} issue(s))", report.issue_count);
    }

    if !json {
        println!("  {} No drift detected", "✓".green());
    }

    Ok(())
}
