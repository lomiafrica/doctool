use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;
use doctool_core::{run_diff, DiffFormat, DoctoolConfig};

pub async fn run(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    json: bool,
    path: String,
    proposed: Option<PathBuf>,
    format: String,
) -> Result<()> {
    let docs_content = config.resolve(monorepo_root, &config.docs_content);
    let diff_format = match format.as_str() {
        "unified" => DiffFormat::Unified,
        _ => DiffFormat::Unified,
    };

    let report = run_diff(&docs_content, &path, proposed.as_deref(), diff_format)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", "dt diff".bold());
        println!("  Path: {}", report.path);
        if report.changed {
            println!("{}", report.patch);
        } else {
            println!("  {} No differences", "✓".green());
        }
    }

    Ok(())
}
