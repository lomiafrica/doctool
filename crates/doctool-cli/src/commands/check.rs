use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use crate::pnpm::{run_pnpm_in_docs};

pub fn run(monorepo_root: &Path, json: bool) -> Result<()> {
    if !json {
        println!("{}", "dt check".bold());
        println!("  Monorepo: {}", monorepo_root.display());
    }

    run_pnpm_in_docs(monorepo_root, "lint")?;
    run_pnpm_in_docs(monorepo_root, "docs:drift")?;

    if json {
        println!(r#"{{"status":"ok","lint":"passed","docs_drift":"passed"}}"#);
    } else {
        println!("  {} Docs lint and drift checks passed", "✓".green());
    }

    Ok(())
}
