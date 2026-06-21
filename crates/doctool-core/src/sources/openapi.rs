use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiOperation {
    pub method: String,
    pub path: String,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiIndex {
    pub operations: Vec<OpenApiOperation>,
}

pub fn load_openapi(path: &Path) -> Result<OpenApiIndex> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read OpenAPI spec at {}", path.display()))?;
    let spec: serde_json::Value =
        serde_json::from_str(&raw).context("Failed to parse OpenAPI JSON")?;

    let mut operations = Vec::new();
    let paths = spec
        .get("paths")
        .and_then(|p| p.as_object())
        .context("OpenAPI missing paths")?;

    for (path, item) in paths {
        let Some(item_obj) = item.as_object() else {
            continue;
        };
        for method in ["get", "post", "put", "patch", "delete"] {
            let Some(op) = item_obj.get(method) else {
                continue;
            };
            operations.push(OpenApiOperation {
                method: method.to_uppercase(),
                path: path.clone(),
                operation_id: op
                    .get("operationId")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                summary: op
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            });
        }
    }

    operations.sort_by(|a, b| a.path.cmp(&b.path).then(a.method.cmp(&b.method)));
    Ok(OpenApiIndex { operations })
}

pub fn load_expected_public_operations(path: &Path) -> Result<HashSet<String>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read expected operations at {}", path.display()))?;
    let list: Vec<String> = serde_json::from_str(&raw)?;
    Ok(list.into_iter().collect())
}

pub fn filter_public_operations(
    index: &OpenApiIndex,
    expected: &HashSet<String>,
) -> Vec<OpenApiOperation> {
    index
        .operations
        .iter()
        .filter(|op| {
            let key = format!("{} {}", op.method, op.path);
            expected.contains(&key)
        })
        .cloned()
        .collect()
}

pub fn operations_key(op: &OpenApiOperation) -> String {
    format!("{} {}", op.method, op.path)
}

pub fn operations_map(ops: &[OpenApiOperation]) -> HashMap<String, OpenApiOperation> {
    ops.iter()
        .map(|op| (operations_key(op), op.clone()))
        .collect()
}
