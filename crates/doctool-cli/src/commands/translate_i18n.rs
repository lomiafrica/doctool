use std::path::Path;

use anyhow::{bail, Result};
use colored::Colorize;
use doctool_core::{run_translate_i18n, DoctoolConfig, TranslateI18nOptions};

pub async fn run(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    json: bool,
    check: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let options = TranslateI18nOptions {
        check_only: check,
        dry_run,
        force,
        refresh_lock: !check && !dry_run,
    };

    let report = run_translate_i18n(config, monorepo_root, &options).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if dry_run {
        println!("{}", "dt translate-i18n --dry-run".bold());
        println!("  Pending segments: {}", report.pending_segments);
        for page in &report.pages {
            println!(
                "  {} → {} ({} segment(s))",
                page.en_path, page.target_path, page.segments_translated
            );
        }
    } else if check {
        println!("{}", "dt translate-i18n --check".bold());
        println!("  Pending segments: {}", report.pending_segments);
        for page in &report.pages {
            println!(
                "  {} → {} ({} segment(s))",
                page.en_path, page.target_path, page.segments_translated
            );
        }
    } else {
        println!("{}", "dt translate-i18n".bold());
        println!("  Translated {} segment(s)", report.pending_segments);
        for page in &report.pages {
            if page.written {
                println!("  {} → {}", page.en_path.green(), page.target_path);
            }
        }
        if report.lock_updated {
            println!("  {} Updated i18n lock file", "✓".green());
        }
    }

    if check && report.pending_segments > 0 {
        bail!(
            "translate-i18n check failed ({} pending segment(s))",
            report.pending_segments
        );
    }

    if check && report.pending_segments == 0 && !json {
        println!("  {} No pending translations", "✓".green());
    }

    Ok(())
}
