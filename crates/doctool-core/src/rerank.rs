//! COMPOSER_SOURCE: composer/src-tauri/src/db/rerank.rs
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use std::sync::OnceLock;

static RERANKER: OnceLock<TextRerank> = OnceLock::new();

pub fn get_reranker() -> Result<&'static TextRerank, String> {
    if let Some(reranker) = RERANKER.get() {
        return Ok(reranker);
    }

    let model = TextRerank::try_new(RerankInitOptions::new(RerankerModel::BGERerankerBase))
        .map_err(|e| format!("Failed to initialize reranker: {e}"))?;
    let _ = RERANKER.set(model);

    RERANKER
        .get()
        .ok_or_else(|| "Failed to initialize reranker".to_string())
}

pub fn rerank_documents(
    query: &str,
    documents: Vec<String>,
    top_k: usize,
) -> Result<Vec<(usize, f32)>, String> {
    if documents.is_empty() {
        return Ok(Vec::new());
    }

    let model = get_reranker()?;
    let doc_refs: Vec<&str> = documents.iter().map(|d| d.as_str()).collect();
    let results = model
        .rerank(query, doc_refs, true, None)
        .map_err(|e| format!("Failed to score documents: {e}"))?;

    let mut scored: Vec<(usize, f32)> = results
        .into_iter()
        .map(|res| (res.index, res.score))
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if top_k > 0 && scored.len() > top_k {
        scored.truncate(top_k);
    }

    Ok(scored)
}
