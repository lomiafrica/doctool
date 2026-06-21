use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorDoc {
    pub provider: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorIndex {
    pub documents: Vec<CompetitorDoc>,
}

pub fn load_competitor_index(root: &Path) -> Result<CompetitorIndex> {
    if !root.is_dir() {
        return Ok(CompetitorIndex {
            documents: Vec::new(),
        });
    }

    let mut documents = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext != "md" && ext != "mdx" && ext != "txt" {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let provider = relative.split('/').next().unwrap_or("unknown").to_string();
        let title = fs::read_to_string(path)
            .ok()
            .and_then(|c| c.lines().find(|l| l.starts_with("# ")).map(|l| l[2..].trim().to_string()));

        documents.push(CompetitorDoc {
            provider,
            relative_path: relative,
            title,
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
        });
    }

    documents.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(CompetitorIndex { documents })
}
