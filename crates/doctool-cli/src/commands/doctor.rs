use std::path::Path;

use anyhow::{bail, Result};
use colored::Colorize;
use doctool_core::{run_doctor, DoctorStatus, DoctoolConfig};

pub fn run(config: &DoctoolConfig, monorepo_root: &Path, json: bool) -> Result<()> {
    let report = run_doctor(config, monorepo_root);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", "dt doctor".bold());
        println!(
            "  {} ok · {} warn · {} fail",
            report.ok_count, report.warn_count, report.fail_count
        );
        println!();

        for check in &report.checks {
            let icon = match check.status {
                DoctorStatus::Ok => "✓".green(),
                DoctorStatus::Warn => "!".yellow(),
                DoctorStatus::Fail => "✗".red(),
            };
            println!("  {icon} {} — {}", check.label.bold(), check.detail);
        }
    }

    if report.fail_count > 0 {
        bail!(
            "Doctor found {} failed check(s) — fix setup before using doctool",
            report.fail_count
        );
    }

    Ok(())
}
