use std::path::Path;

use anyhow::{bail, Result};
use colored::Colorize;

use crate::pnpm::{run_pnpm_in_docs, run_pnpm_in_docs_capture};

#[derive(serde::Serialize)]
struct CheckStepResult {
    step: &'static str,
    status: &'static str,
}

#[derive(serde::Serialize)]
struct CheckReport {
    status: &'static str,
    steps: Vec<CheckStepResult>,
}

pub fn run(monorepo_root: &Path, json: bool) -> Result<()> {
    if !json {
        println!("{}", "dt check".bold());
        println!("  Monorepo: {}", monorepo_root.display());
    }

    let mut steps = Vec::new();

    // Step 1: lint
    if json {
        run_pnpm_in_docs(monorepo_root, "lint")?;
        steps.push(CheckStepResult {
            step: "lint",
            status: "passed",
        });
    } else {
        print!("  lint ... ");
        match run_pnpm_in_docs_capture(monorepo_root, "lint") {
            Ok((true, _)) => {
                println!("{}", "ok".green());
                steps.push(CheckStepResult {
                    step: "lint",
                    status: "passed",
                });
            }
            Ok((false, output)) => {
                println!("{}", "failed".red());
                eprintln!("{output}");
                bail!("Docs lint failed");
            }
            Err(e) => return Err(e),
        }
    }

    // Step 2: docs:drift (TypeScript)
    if json {
        run_pnpm_in_docs(monorepo_root, "docs:drift")?;
        steps.push(CheckStepResult {
            step: "docs_drift",
            status: "passed",
        });
    } else {
        print!("  docs:drift ... ");
        match run_pnpm_in_docs_capture(monorepo_root, "docs:drift") {
            Ok((true, _)) => {
                println!("{}", "ok".green());
                steps.push(CheckStepResult {
                    step: "docs_drift",
                    status: "passed",
                });
            }
            Ok((false, output)) => {
                println!("{}", "failed".red());
                eprintln!("{output}");
                bail!("Docs drift script failed");
            }
            Err(e) => return Err(e),
        }
    }

    // Step 3: docs screenshots (1280×720 WebP manifest)
    if json {
        run_pnpm_in_docs(monorepo_root, "screenshots:verify")?;
        steps.push(CheckStepResult {
            step: "screenshots_verify",
            status: "passed",
        });
    } else {
        print!("  screenshots:verify ... ");
        match run_pnpm_in_docs_capture(monorepo_root, "screenshots:verify") {
            Ok((true, _)) => {
                println!("{}", "ok".green());
                steps.push(CheckStepResult {
                    step: "screenshots_verify",
                    status: "passed",
                });
            }
            Ok((false, output)) => {
                println!("{}", "failed".red());
                eprintln!("{output}");
                bail!("Docs screenshot verification failed");
            }
            Err(e) => return Err(e),
        }
    }

    if json {
        let report = CheckReport {
            status: "ok",
            steps,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("  {} Docs lint and drift checks passed", "✓".green());
    }

    Ok(())
}
