//! Code search context for RAG-backed LLM commands.

use std::path::Path;

use anyhow::Result;

use crate::config::DoctoolConfig;
use crate::drift::DriftIssue;
use crate::index::CodeIndex;
use crate::sources::mdx::document::MdxDocument;

/// Scan configured code roots and optionally embed elements for hybrid search.
pub async fn build_code_index(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    embed: bool,
) -> Result<CodeIndex> {
    let mut index = CodeIndex::new();
    let roots = config.code_root_paths(monorepo_root);
    index
        .scan_roots(&roots)
        .map_err(|e| anyhow::anyhow!(e))?;
    if embed {
        index.populate_vectors().await;
    }
    Ok(index)
}

/// Build search queries from an MDX page path and parsed document.
pub fn queries_for_page(page_path: &str, doc: &MdxDocument) -> Vec<String> {
    let mut queries = Vec::new();

    if let Some(method) = doc.frontmatter.get("method") {
        if let Some(path) = doc.frontmatter.get("path") {
            queries.push(format!("{method} {path}"));
            queries.push(path.clone());
        }
        queries.push(method.clone());
    }

    if let Some(operation_id) = doc.frontmatter.get("operationId") {
        queries.push(operation_id.clone());
    }

    for segment in page_path.split('/') {
        if segment.len() > 3 && !segment.ends_with(".mdx") {
            queries.push(segment.replace(".mdx", ""));
        }
    }

    if let Some(title) = doc.frontmatter.get("title") {
        queries.push(title.clone());
    }

    queries.sort();
    queries.dedup();
    queries
}

/// Derive code-search terms from drift issues.
pub fn queries_for_drift_issues(issues: &[DriftIssue]) -> Vec<String> {
    let mut queries = Vec::new();

    for issue in issues.iter().take(40) {
        if let Some(file) = &issue.file {
            for segment in file.split('/') {
                if segment.contains("Controller") {
                    queries.push(segment.replace(".mdx", "").to_string());
                }
            }
            queries.push(file.replace(".mdx", "").replace('/', " "));
        }

        if issue.message.contains("OpenAPI operation missing MDX:") {
            if let Some(op) = issue.message.split("OpenAPI operation missing MDX: ").nth(1) {
                queries.push(op.to_string());
            }
        }

        if issue.message.contains("SDK method not mentioned") {
            if let Some(method) = issue.message.split(": ").nth(1) {
                queries.push(method.to_string());
            }
        }
    }

    queries.sort();
    queries.dedup();
    queries
}

/// Hybrid search across the code index; returns formatted snippets for LLM prompts.
pub async fn format_code_context(
    index: &CodeIndex,
    queries: &[String],
    max_snippets: usize,
) -> String {
    if queries.is_empty() {
        return "(no code search queries)".into();
    }

    let mut seen = std::collections::HashSet::new();
    let mut blocks = Vec::new();

    for query in queries {
        if blocks.len() >= max_snippets {
            break;
        }
        let Ok(hits) = index.search_elements(query).await else {
            continue;
        };
        for element in hits {
            if !seen.insert(element.id.clone()) {
                continue;
            }
            let preview: String = element.code.chars().take(800).collect();
            blocks.push(format!(
                "### {} ({}) — {}:{}\n{}\n",
                element.name,
                element.element_type,
                element.relative_path,
                element.start_line,
                preview
            ));
            if blocks.len() >= max_snippets {
                break;
            }
        }
    }

    if blocks.is_empty() {
        "(no matching code elements — run `dt scan` after API changes)".into()
    } else {
        blocks.join("\n")
    }
}
