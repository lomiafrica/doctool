use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use crate::pnpm::run_scaffold;

pub fn run(monorepo_root: &Path, json: bool) -> Result<()> {
    if !json {
        println!("{}", "dt scaffold".bold());
        println!("  Running CONFIRM_BOOTSTRAP=1 api:regenerate-rest-reference");
    }

    run_scaffold(monorepo_root)?;

    if json {
        println!(r#"{{"status":"ok","action":"scaffold"}}"#);
    } else {
        println!("  {} Scaffold complete", "✓".green());
    }

    Ok(())
}
