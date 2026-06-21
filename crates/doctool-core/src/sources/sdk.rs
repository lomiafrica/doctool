use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkMethod {
    pub service: String,
    pub method: String,
    pub qualified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkIndex {
    pub methods: Vec<SdkMethod>,
}

#[derive(Debug, Clone, Deserialize)]
struct SdkManifest {
    sdk: HashMap<String, Vec<String>>,
}

pub fn load_sdk_index(path: &Path) -> Result<SdkIndex> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read SDK manifest at {}", path.display()))?;
    let manifest: SdkManifest = serde_json::from_str(&raw)?;

    let mut methods = Vec::new();
    for (service, names) in manifest.sdk {
        for method in names {
            methods.push(SdkMethod {
                qualified: format!("lomi.{service}.{method}"),
                service: service.clone(),
                method,
            });
        }
    }

    methods.sort_by(|a, b| a.qualified.cmp(&b.qualified));
    Ok(SdkIndex { methods })
}

pub fn unmentioned_methods(index: &SdkIndex, mdx_blob: &str) -> Vec<SdkMethod> {
    index
        .methods
        .iter()
        .filter(|m| {
            !mdx_blob.contains(&m.method)
                && !mdx_blob.contains(&m.qualified)
                && !mdx_blob.contains(&format!("lomi.{}", m.service))
        })
        .cloned()
        .collect()
}
