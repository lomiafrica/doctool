use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use doctool_core::{DoctoolConfig, DoctoolEngine};

pub async fn run(config: &DoctoolConfig, monorepo_root: &Path, json: bool) -> Result<()> {
    let mut engine = DoctoolEngine::new(config.clone(), monorepo_root);
    let snapshot = engine.scan()?;
    engine.save_snapshot(&snapshot)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
        return Ok(());
    }

    println!("{}", "dt scan".bold());
    println!("  Monorepo: {}", monorepo_root.display());
    println!("  Code elements: {}", snapshot.code_element_count);
    println!("  OpenAPI operations: {}", snapshot.openapi.operations.len());
    println!("  MDX pages: {}", snapshot.mdx.pages.len());
    println!(
        "  Competitor docs: {}",
        snapshot.competitors.documents.len()
    );
    println!(
        "  Drift issues (preview): {}",
        snapshot.drift_issues.len()
    );
    println!(
        "  {} Wrote {}",
        "✓".green(),
        config.resolve(monorepo_root, &config.index_cache).display()
    );
    Ok(())
}
