pub mod document;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdxPage {
    pub relative_path: String,
    pub slug: String,
    pub locale: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub operation_id: Option<String>,
    pub body_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdxIndex {
    pub pages: Vec<MdxPage>,
    pub valid_slugs: HashSet<String>,
}

pub fn load_mdx_index(content_root: &Path) -> Result<MdxIndex> {
    let mut pages = Vec::new();
    let mut valid_slugs = HashSet::new();

    for entry in WalkDir::new(content_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext != "mdx" {
            continue;
        }

        let relative = path
            .strip_prefix(content_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read MDX {}", path.display()))?;

        let (frontmatter, body) = split_frontmatter(&raw);
        let meta = parse_frontmatter(&frontmatter);

        let locale = detect_locale(&relative);
        let slug = slug_from_path(&relative);

        valid_slugs.insert(slug.clone());
        if slug.ends_with("/index") {
            valid_slugs.insert(slug.trim_end_matches("/index").to_string());
        }

        pages.push(MdxPage {
            relative_path: relative,
            slug,
            locale,
            title: meta.get("title").cloned(),
            description: meta.get("description").cloned(),
            method: meta.get("method").map(|m| m.to_uppercase()),
            path: meta.get("path").cloned(),
            operation_id: meta.get("operationId").cloned(),
            body_preview: body.chars().take(500).collect(),
        });
    }

    add_directory_slugs(content_root, &mut valid_slugs);

    pages.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(MdxIndex { pages, valid_slugs })
}

pub(crate) fn split_frontmatter(raw: &str) -> (String, String) {
    if raw.starts_with("---") {
        if let Some(end) = raw[3..].find("\n---") {
            let fm = raw[3..3 + end].trim().to_string();
            let body = raw[3 + end + 4..].trim_start().to_string();
            return (fm, body);
        }
    }
    (String::new(), raw.to_string())
}

pub(crate) fn parse_frontmatter(fm: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in fm.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_string();
        let mut value = value.trim().to_string();
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value = value[1..value.len() - 1].to_string();
        }
        map.insert(key, value);
    }
    map
}

fn detect_locale(relative: &str) -> String {
    if relative.ends_with(".fr.mdx") {
        "fr".into()
    } else if relative.ends_with(".es.mdx") {
        "es".into()
    } else if relative.ends_with(".zh.mdx") {
        "zh".into()
    } else {
        "en".into()
    }
}

fn slug_from_path(relative: &str) -> String {
    let without_ext = relative.trim_end_matches(".mdx");
    let without_locale = without_ext
        .trim_end_matches(".fr")
        .trim_end_matches(".es")
        .trim_end_matches(".zh");
    without_locale.trim_end_matches("/index").to_string()
}

fn add_directory_slugs(content_root: &Path, slugs: &mut HashSet<String>) {
    for segment in ["start", "build", "api", "resources"] {
        let segment_path = content_root.join(segment);
        if !segment_path.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&segment_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    slugs.insert(format!("{segment}/{name}"));
                }
            }
        }
    }
}

pub fn documented_operations(pages: &[MdxPage]) -> HashMap<String, MdxPage> {
    let mut map = HashMap::new();
    for page in pages {
        if page.locale != "en" {
            continue;
        }
        if let (Some(method), Some(path)) = (&page.method, &page.path) {
            map.insert(format!("{method} {path}"), page.clone());
        }
    }
    map
}

pub fn find_internal_links(
    content_root: &Path,
    pages: &[MdxPage],
    valid_slugs: &HashSet<String>,
) -> Vec<(String, String)> {
    let re = Regex::new(r"\]\(/(start|build|api|resources)/([^)\s#]+)").unwrap();
    let mut dead = Vec::new();
    for page in pages {
        if page.locale != "en" {
            continue;
        }
        let full_path = content_root.join(&page.relative_path);
        let Ok(content) = fs::read_to_string(&full_path) else {
            continue;
        };
        for cap in re.captures_iter(&content) {
            let slug = format!("{}/{}", &cap[1], &cap[2]);
            if !valid_slugs.contains(&slug) {
                dead.push((page.relative_path.clone(), slug));
            }
        }
    }
    dead
}

pub fn read_page_content(content_root: &Path, relative: &str) -> Result<String> {
    let path = content_root.join(relative);
    fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))
}

pub fn english_pages(pages: &[MdxPage]) -> Vec<&MdxPage> {
    pages.iter().filter(|p| p.locale == "en").collect()
}

pub fn missing_french_siblings(pages: &[MdxPage]) -> Vec<String> {
    let fr_paths: HashSet<String> = pages
        .iter()
        .filter(|p| p.locale == "fr")
        .map(|p| p.relative_path.clone())
        .collect();

    pages
        .iter()
        .filter(|p| p.locale == "en")
        .filter(|p| {
            let fr = p.relative_path.replace(".mdx", ".fr.mdx");
            !fr_paths.contains(&fr)
        })
        .map(|p| p.relative_path.clone())
        .collect()
}

pub fn all_mdx_content(content_root: &Path, pages: &[MdxPage]) -> Result<String> {
    let mut combined = String::new();
    for page in pages {
        if let Ok(content) = read_page_content(content_root, &page.relative_path) {
            combined.push_str(&content);
            combined.push('\n');
        }
    }
    Ok(combined)
}
