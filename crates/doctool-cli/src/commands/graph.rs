use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use doctool_core::{build_knowledge_graph, DoctoolConfig, DoctoolEngine};
use doctool_core::sources::mdx::load_mdx_index;
use doctool_core::sources::openapi::load_openapi;
use doctool_core::sources::sdk::load_sdk_index;

pub async fn run(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    output: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let graph = if monorepo_root
        .join(&config.index_cache)
        .is_file()
    {
        let mut engine = DoctoolEngine::new(config.clone(), monorepo_root);
        engine.load_snapshot()?.knowledge_graph
    } else {
        let openapi = load_openapi(&config.resolve(monorepo_root, &config.openapi))?;
        let mdx = load_mdx_index(&config.resolve(monorepo_root, &config.docs_content))?;
        let sdk_path = config.resolve(monorepo_root, &config.sdk_manifest);
        let sdk = if sdk_path.is_file() {
            Some(load_sdk_index(&sdk_path)?)
        } else {
            None
        };
        build_knowledge_graph(&openapi, &mdx, sdk.as_ref())
    };

    let payload = serde_json::to_string_pretty(&graph)?;

    if let Some(path) = output {
        fs::write(&path, &payload)?;
        if !json {
            println!("Wrote knowledge graph to {}", path.display());
        }
    } else {
        println!("{payload}");
    }

    Ok(())
}
