use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use doctool_core::{run_suggest, DoctoolConfig, SuggestOptions};

pub async fn run(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    json: bool,
    skip_ts: bool,
    skip_ai: bool,
    no_i18n: bool,
) -> Result<()> {
    let options = SuggestOptions {
        skip_ts,
        skip_ai,
        include_i18n: !no_i18n,
    };

    let report = run_suggest(config, monorepo_root, &options).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("{}", "dt suggest".bold());
    println!();

    if report.blocking_issue_count == 0 {
        println!(
            "  {} Blocking (drift): {} — API, links, and SDK alignment look good",
            "✓".green(),
            report.blocking_issue_count
        );
    } else {
        println!(
            "  {} Blocking (drift): {} — fix before merge",
            "✗".red(),
            report.blocking_issue_count
        );
    }

    if no_i18n {
        println!("  (i18n checks skipped with --no-i18n)");
    } else if report.i18n_warning_count == 0 {
        println!(
            "  {} Warnings (i18n): {} — EN/FR structure aligned",
            "✓".green(),
            report.i18n_warning_count
        );
    } else {
        println!(
            "  {} Warnings (i18n): {} — non-blocking; mostly heading/link parity",
            "!".yellow(),
            report.i18n_warning_count
        );
        for (category, count) in &report.i18n_by_category {
            let hint = if *category == "locale_structure" && report.i18n_api_structure_count > 0 {
                format!(
                    " ({} under api/* — often missing « See also » / « Voir aussi » in FR)",
                    report.i18n_api_structure_count
                )
            } else {
                String::new()
            };
            println!("      {category}: {count}{hint}");
        }
        println!(
            "    Details: {} · baseline: {}",
            "dt sync-i18n --check".cyan(),
            "dt sync-i18n --lock".cyan()
        );
    }

    if let Some(summary) = &report.ai_summary {
        println!("\n{}", "Summary".bold());
        println!("  {summary}");
    }

    if !report.executable_commands.is_empty() {
        println!("\n{}", "Executable commands".bold());
        for cmd in &report.executable_commands {
            println!("  $ {cmd}");
        }
    }

    if !report.actions.is_empty() {
        println!("\n{}", "Prioritized actions".bold());
        for action in report.actions.iter().take(20) {
            let cmd = action
                .command
                .as_deref()
                .map(|c| format!(" → {c}"))
                .unwrap_or_default();
            println!(
                "  [{}] P{} {}{} — {}",
                action.source, action.priority, action.title, cmd, action.detail
            );
        }
        if report.actions.len() > 20 {
            println!("  … and {} more", report.actions.len() - 20);
        }
    }

    println!(
        "\n  Code context: {} chars from hybrid search",
        report.code_context_chars
    );

    Ok(())
}
