mod commands;
mod output;
mod pnpm;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use doctool_core::{find_monorepo_root, DoctoolConfig};

use output::use_json;

#[derive(Parser, Debug)]
#[command(
    name = "dt",
    bin_name = "dt",
    about = "Documentation tooling for the lomi. monorepo (doctool)",
    version
)]
struct Cli {
    /// Path to doctool.config.toml
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Monorepo root override
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Emit JSON output
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Index code, OpenAPI, MDX, SDK, and competitor docs
    Scan,
    /// Run apps/docs lint and docs:drift (CI gate)
    Check,
    /// Unified drift report (Rust + optional TS drift)
    Drift {
        /// Skip running the TypeScript docs-drift script
        #[arg(long)]
        skip_ts: bool,
    },
    /// Export knowledge graph JSON
    Graph {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Scaffold missing REST reference MDX pages
    Scaffold,
    /// Deterministic i18n sync (no LLM): gap/stale/structure checks and lock file
    SyncI18n {
        /// CI mode: exit non-zero when i18n drift is detected
        #[arg(long)]
        check: bool,
        /// Print files/segments that would need work without writing
        #[arg(long)]
        dry_run: bool,
        /// Create minimal locale sibling stubs for missing pages
        #[arg(long)]
        scaffold_missing: bool,
        /// Refresh i18n.lock from current EN segment hashes
        #[arg(long)]
        lock: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let json = use_json(cli.json);
    let cwd = std::env::current_dir()?;
    let monorepo_root = match cli.root.clone() {
        Some(root) => root,
        None => find_monorepo_root(&cwd).context("Could not find monorepo root")?,
    };
    let config = DoctoolConfig::load(cli.config.as_deref())?;

    match cli.command {
        Commands::Scan => commands::scan::run(&config, &monorepo_root, json).await,
        Commands::Check => commands::check::run(&monorepo_root, json),
        Commands::Drift { skip_ts } => {
            commands::drift::run(&config, &monorepo_root, json, skip_ts).await
        }
        Commands::Graph { output } => {
            commands::graph::run(&config, &monorepo_root, output, json).await
        }
        Commands::Scaffold => commands::scaffold::run(&monorepo_root, json),
        Commands::SyncI18n {
            check,
            dry_run,
            scaffold_missing,
            lock,
        } => {
            commands::sync_i18n::run(
                &config,
                &monorepo_root,
                json,
                check,
                dry_run,
                scaffold_missing,
                lock,
            )
            .await
        }
    }
}
