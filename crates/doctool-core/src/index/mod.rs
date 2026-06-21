//! Lean code index — refactored from Composer `code_intel::mod.rs` without Tauri/agent state.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use reqwest::Client;

use crate::intel::{
    embedder, global_index::GlobalIndex, graph::CodeGraphs, indexer, language_detect, loader,
    parser, types::*, utils, vector_store::VectorStore, CodeElement,
};
use crate::rerank;

pub struct CodeIndex {
    pub elements: Vec<CodeElement>,
    pub global_index: GlobalIndex,
    pub graphs: CodeGraphs,
    pub vector_store: VectorStore,
    pub scan_stats: Option<ScanStats>,
    client: Client,
}

impl Default for CodeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeIndex {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            global_index: GlobalIndex::new(),
            graphs: CodeGraphs::new(),
            vector_store: VectorStore::new(),
            scan_stats: None,
            client: Client::new(),
        }
    }

    /// Scan one folder: load files, parse, index, build graphs.
    pub fn scan_folder(&mut self, root: &Path) -> Result<ScanStats, String> {
        let root_str = root.to_string_lossy();
        let start = Instant::now();

        let file_entries = loader::scan_directory(&root_str);
        if file_entries.is_empty() {
            return Err(format!(
                "No supported source files found in {}",
                root.display()
            ));
        }

        let mut all_elements = Vec::new();
        let mut all_parse_results = Vec::new();
        let mut language_counts: HashMap<String, usize> = HashMap::new();
        let mut total_classes = 0usize;
        let mut total_functions = 0usize;
        let mut total_imports = 0usize;
        let mut total_lines = 0usize;

        for entry in &file_entries {
            if !utils::is_parseable_extension(&entry.extension) {
                continue;
            }

            let content = match loader::read_file_content(&entry.path) {
                Some(c) => c,
                None => continue,
            };

            if let Some(parse_result) =
                parser::parse_file(&entry.path, &content, &entry.language, &root_str)
            {
                total_classes += parse_result.classes.len();
                total_functions += parse_result.functions.len()
                    + parse_result
                        .classes
                        .iter()
                        .map(|c| c.methods.len())
                        .sum::<usize>();
                total_imports += parse_result.imports.len();
                total_lines += parse_result.total_lines;

                *language_counts.entry(entry.language.clone()).or_insert(0) += 1;

                let elements = indexer::index_file(&parse_result, &content);
                all_elements.extend(elements);
                all_parse_results.push(parse_result);
            }
        }

        let mut global_index = GlobalIndex::new();
        global_index.build(&all_elements, &all_parse_results, &root_str);

        let mut graphs = CodeGraphs::new();
        graphs.build(&all_elements, &all_parse_results, &global_index, &root_str);

        let graph_stats = graphs.get_stats();
        let stats = ScanStats {
            total_files: file_entries.len(),
            total_classes,
            total_functions,
            total_imports,
            total_elements: all_elements.len(),
            total_lines,
            languages: language_counts,
            graph_nodes: graph_stats.dependency_nodes
                + graph_stats.inheritance_nodes
                + graph_stats.call_nodes,
            graph_edges: graph_stats.dependency_edges
                + graph_stats.inheritance_edges
                + graph_stats.call_edges,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        };

        self.elements.extend(all_elements);
        self.global_index = global_index;
        self.graphs = graphs;
        self.scan_stats = Some(stats.clone());

        Ok(stats)
    }

    /// Scan multiple code roots and merge element lists.
    pub fn scan_roots(&mut self, roots: &[impl AsRef<Path>]) -> Result<Vec<ScanStats>, String> {
        *self = Self::new();
        let mut all_stats = Vec::new();
        for root in roots {
            if root.as_ref().is_dir() {
                all_stats.push(self.scan_folder(root.as_ref())?);
            }
        }
        Ok(all_stats)
    }

    pub fn get_elements(
        &self,
        element_type: Option<&str>,
        file_path: Option<&str>,
    ) -> Vec<CodeElement> {
        let mut results: Vec<CodeElement> = self
            .elements
            .iter()
            .filter(|e| {
                let type_match = element_type.is_none_or(|t| e.element_type == t);
                let file_match = file_path.is_none_or(|f| e.file_path == f || e.relative_path == f);
                type_match && file_match
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| {
            a.file_path
                .cmp(&b.file_path)
                .then(a.start_line.cmp(&b.start_line))
        });

        results
    }

    /// Hybrid keyword + semantic search with optional cross-encoder rerank.
    pub async fn search_elements(&self, query: &str) -> Result<Vec<CodeElement>, String> {
        let query_embedding = embedder::get_embedding(&self.client, query)
            .await
            .unwrap_or_default();

        let has_query_vec = !query_embedding.is_empty();
        let lang = language_detect::detect_language(query);
        let expanded = language_detect::expand_query_for_fts(query, &lang);
        let terms: Vec<&str> = expanded
            .split(" OR ")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut results: Vec<(f32, CodeElement)> = self
            .elements
            .iter()
            .filter_map(|e| {
                let mut keyword_score = 0.0;
                let name_lower = e.name.to_lowercase();
                let code_lower = e.code.to_lowercase();

                for term in &terms {
                    if name_lower.contains(term) {
                        keyword_score += 10.0;
                    }
                    if e.signature
                        .as_ref()
                        .is_some_and(|s| s.to_lowercase().contains(term))
                    {
                        keyword_score += 5.0;
                    }
                    if e.docstring
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(term))
                    {
                        keyword_score += 3.0;
                    }
                    if code_lower.contains(term) {
                        keyword_score += 1.0;
                    }
                }

                let mut semantic_score = 0.0;
                if has_query_vec {
                    if let Some(vec) = self.vector_store.get_vector(&e.id) {
                        semantic_score = cosine_similarity(&query_embedding, vec) * 30.0;
                    }
                }

                let total_score = keyword_score + semantic_score;
                if total_score > 0.1 {
                    Some((total_score, e.clone()))
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let top_initial: Vec<CodeElement> = results.into_iter().take(50).map(|(_, e)| e).collect();

        let documents: Vec<String> = top_initial.iter().map(|e| e.code.clone()).collect();
        let query_owned = query.to_string();
        let rerank_outcome = tokio::task::spawn_blocking(move || {
            rerank::rerank_documents(&query_owned, documents, 15)
        })
        .await;

        match rerank_outcome {
            Ok(Ok(reranked_indices)) => Ok(reranked_indices
                .into_iter()
                .filter_map(|(idx, _)| top_initial.get(idx).cloned())
                .collect()),
            _ => Ok(top_initial.into_iter().take(15).collect()),
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len().min(b.len()) {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}
