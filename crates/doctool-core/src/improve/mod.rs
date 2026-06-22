use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::config::DoctoolConfig;
use crate::diff::{diff_text, DiffFormat};
use crate::llm::{
    improve_mdx_content, ImprovePromptInput,
};
use crate::provenance::{collect_git_provenance, GitProvenance};
use crate::sources::competitors::load_competitor_index;
use crate::sources::mdx::document::MdxDocument;
use crate::sources::openapi::load_openapi;

const STYLE_GUIDE_REL: &str = "apps/docs/lib/scripts/manual-api/docs-style-guide.md";

#[derive(Debug, Clone, Default)]
pub struct ImproveOptions {
    pub path: String,
    pub stdout: bool,
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImproveReport {
    pub path: String,
    pub improved_content: String,
    pub diff_unified: String,
    pub written_to: Option<String>,
    pub provenance: GitProvenance,
}

pub async fn run_improve(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    options: &ImproveOptions,
) -> Result<ImproveReport> {
    let docs_content = config.resolve(monorepo_root, &config.docs_content);
    let page_path = options.path.trim_start_matches('/');
    let full_path = docs_content.join(page_path);

    let current = fs::read_to_string(&full_path)
        .with_context(|| format!("read MDX {}", full_path.display()))?;

    let doc = MdxDocument::parse(page_path, &current);
    let style_guide = load_style_guide(monorepo_root);
    let openapi_context = build_openapi_context(config, monorepo_root, &doc);
    let competitor_context = build_competitor_context(config, monorepo_root, page_path);
    let code_context = String::from("(run `dt scan` for code RAG context)");

    let prompt_input = ImprovePromptInput {
        page_path: page_path.to_string(),
        current_content: current.clone(),
        style_guide,
        openapi_context,
        code_context,
        competitor_context,
    };

    let llm_config = config.llm_config();
    let improved = improve_mdx_content(&llm_config, &prompt_input).await?;
    let diff = diff_text(page_path, &current, &improved, DiffFormat::Unified);

    let mut written_to = None;
    if let Some(out_dir) = &options.output_dir {
        let dest = out_dir.join(page_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, &improved)?;
        written_to = Some(dest.to_string_lossy().into_owned());
    }

    Ok(ImproveReport {
        path: page_path.to_string(),
        improved_content: if options.stdout {
            improved
        } else {
            String::new()
        },
        diff_unified: diff.patch,
        written_to,
        provenance: collect_git_provenance(monorepo_root),
    })
}

fn load_style_guide(monorepo_root: &Path) -> String {
    let path = monorepo_root.join(STYLE_GUIDE_REL);
    fs::read_to_string(&path).unwrap_or_else(|_| {
        "Use clear, concise technical prose. Preserve API accuracy.".into()
    })
}

fn build_openapi_context(config: &DoctoolConfig, monorepo_root: &Path, doc: &MdxDocument) -> String {
    let method = doc.frontmatter.get("method").map(String::as_str);
    let path = doc.frontmatter.get("path").map(String::as_str);
    if method.is_none() && path.is_none() {
        return String::from("(no OpenAPI operation on this page)");
    }

    let openapi_path = config.resolve(monorepo_root, &config.openapi);
    let Ok(index) = load_openapi(&openapi_path) else {
        return String::from("(OpenAPI index unavailable)");
    };

    let mut out = String::new();
    for op in &index.operations {
        let matches_method = method.is_none_or(|m| op.method.eq_ignore_ascii_case(m));
        let matches_path = path.is_none_or(|p| op.path == p);
        if matches_method && matches_path {
            out.push_str(&format!("{} {}\n", op.method, op.path));
            if let Some(s) = &op.summary {
                out.push_str(s);
                out.push('\n');
            }
            out.push('\n');
        }
    }
    if out.is_empty() {
        "(operation not found in OpenAPI)".into()
    } else {
        out
    }
}

fn build_competitor_context(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    page_path: &str,
) -> String {
    let competitors_path = config.resolve(monorepo_root, &config.competitors);
    let Ok(index) = load_competitor_index(&competitors_path) else {
        return String::from("(competitor corpus unavailable)");
    };

    let topic = page_path.split('/').nth(1).unwrap_or("docs");
    let mut snippets = Vec::new();
    for entry in index.documents.iter().take(50) {
        let title = entry.title.as_deref().unwrap_or(&entry.relative_path);
        if entry.relative_path.contains(topic) || title.to_lowercase().contains(topic) {
            snippets.push(format!("- {}: {}", title, entry.relative_path));
        }
        if snippets.len() >= 3 {
            break;
        }
    }
    if snippets.is_empty() {
        "(no competitor snippets for this topic)".into()
    } else {
        snippets.join("\n")
    }
}
