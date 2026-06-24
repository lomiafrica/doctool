use std::path::Path;

use anyhow::{bail, Result};
use colored::Colorize;
use doctool_core::{run_translate_i18n, DoctoolConfig, TranslateI18nOptions, TranslateI18nReport, TranslatePageResult};

const MAX_PAGE_LIST: usize = 15;

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
    } else if dry_run || check {
        let label = if dry_run {
            "dt translate-i18n --dry-run"
        } else {
            "dt translate-i18n --check"
        };
        println!("{}", label.bold());
        print_lock_notice(&report);
        println!("  Pending segments: {}", report.pending_segments);
        print_page_list(&report.pages);
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

fn print_lock_notice(report: &TranslateI18nReport) {
    if report.lock_exists {
        return;
    }
    println!();
    println!(
        "  {} No i18n.lock at {}",
        "!".yellow(),
        report.lock_path
    );
    println!(
        "    All EN segments are treated as pending ({} is not a count of real errors).",
        report.pending_segments
    );
    println!("    Create a baseline: {}", "dt sync-i18n --lock".cyan());
    println!();
}

fn print_page_list(pages: &[TranslatePageResult]) {
    if pages.is_empty() {
        return;
    }
    for page in pages.iter().take(MAX_PAGE_LIST) {
        println!(
            "  {} → {} ({} segment(s))",
            page.en_path, page.target_path, page.segments_translated
        );
    }
    if pages.len() > MAX_PAGE_LIST {
        println!("  … and {} more page(s)", pages.len() - MAX_PAGE_LIST);
    }
}
