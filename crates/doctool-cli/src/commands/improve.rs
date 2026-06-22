use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use colored::Colorize;
use doctool_core::{run_improve, DoctoolConfig, ImproveOptions};

pub async fn run(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    json: bool,
    path: String,
    stdout: bool,
    output: Option<PathBuf>,
) -> Result<()> {
    if !stdout && output.is_none() {
        bail!("specify --stdout and/or --output <dir>");
    }

    let options = ImproveOptions {
        path,
        stdout,
        output_dir: output,
    };

    let report = run_improve(config, monorepo_root, &options).await?;

    if stdout {
        print!("{}", report.improved_content);
    } else if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", "dt improve".bold());
        println!("  Path: {}", report.path);
        if let Some(dest) = &report.written_to {
            println!("  {} Wrote {}", "✓".green(), dest);
        }
        if !report.diff_unified.is_empty() {
            println!("{}", report.diff_unified);
        }
    }

    Ok(())
}
