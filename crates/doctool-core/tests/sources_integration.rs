mod support;

use std::fs;

use support::{load_fixture_config, mini_monorepo_root};

use doctool_core::sources::mdx::load_mdx_index;
use doctool_core::sources::openapi::load_openapi;
use doctool_core::sources::sdk::load_sdk_index;
use doctool_core::{build_knowledge_graph, find_monorepo_root, DoctoolEngine};

#[test]
fn openapi_fixture_loads_three_operations() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let path = config.resolve(&root, &config.openapi);
    let index = load_openapi(&path).unwrap();
    assert_eq!(index.operations.len(), 3);
}

#[test]
fn mdx_fixture_indexes_pages_and_slugs() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let docs = config.resolve(&root, &config.docs_content);
    let index = load_mdx_index(&docs).unwrap();

    assert!(index.pages.len() >= 5);
    assert!(index.valid_slugs.contains("api/products/ProductsController_list"));
    assert!(index.valid_slugs.contains("start/sandbox-payments"));
}

#[test]
fn sdk_fixture_loads_qualified_methods() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let path = config.resolve(&root, &config.sdk_manifest);
    let index = load_sdk_index(&path).unwrap();
    assert!(index
        .methods
        .iter()
        .any(|m| m.qualified == "lomi.products.list"));
}

#[test]
fn knowledge_graph_links_api_doc_to_operation() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let openapi = load_openapi(&config.resolve(&root, &config.openapi)).unwrap();
    let mdx = load_mdx_index(&config.resolve(&root, &config.docs_content)).unwrap();
    let sdk = load_sdk_index(&config.resolve(&root, &config.sdk_manifest)).ok();

    let graph = build_knowledge_graph(&openapi, &mdx, sdk.as_ref());
    assert!(graph.nodes.iter().any(|n| n.kind == "operation"));
    assert!(graph.nodes.iter().any(|n| n.kind == "api_doc"));
    assert!(graph.edges.iter().any(|e| e.relation == "documents"));
}

#[test]
fn engine_scan_produces_snapshot_and_caches() {
    let root = mini_monorepo_root();
    let config = load_fixture_config();
    let mut engine = DoctoolEngine::new(config.clone(), &root);

    let snapshot = engine.scan().expect("scan fixture monorepo");
    assert!(snapshot.code_element_count > 0);
    assert!(!snapshot.openapi.operations.is_empty());
    assert!(!snapshot.mdx.pages.is_empty());
    assert!(!snapshot.competitors.documents.is_empty());
    assert!(!snapshot.drift_issues.is_empty());

    engine.save_snapshot(&snapshot).expect("save snapshot");

    let index_path = config.resolve(&root, &config.index_cache);
    let graph_path = config.resolve(&root, &config.graph_cache);
    assert!(index_path.is_file());
    assert!(graph_path.is_file());

    let mut reload = DoctoolEngine::new(config, &root);
    let loaded = reload.load_snapshot().expect("load snapshot");
    assert_eq!(loaded.code_element_count, snapshot.code_element_count);

    // Cleanup artifact so fixture tree stays clean in dev.
    let _ = fs::remove_file(index_path);
    let _ = fs::remove_file(graph_path);
    let _ = fs::remove_dir(root.join(".doctool"));
}

#[test]
fn find_monorepo_from_apps_doctool_crate_dir() {
    let doctool_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let apps_doctool = doctool_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("apps/doctool path");
    let root = find_monorepo_root(apps_doctool).expect("real monorepo root");
    assert!(root.join("apps/docs/package.json").is_file());
}
