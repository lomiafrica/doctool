//! Code relationship graphs — dependency, inheritance, call graphs.
//! Uses petgraph as the Rust equivalent of networkx.

use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::{Deserialize, Serialize};

use super::global_index::GlobalIndex;
use super::types::*;

/// All code relationship graphs.
#[derive(Debug)]
pub struct CodeGraphs {
    /// File A imports File B → edge A→B
    pub dependency_graph: DiGraph<String, String>,
    /// Class A extends Class B → edge A→B
    pub inheritance_graph: DiGraph<String, String>,
    /// Function A calls Function B → edge A→B
    pub call_graph: DiGraph<String, String>,
    /// Element ID → NodeIndex mapping for each graph
    dep_node_map: HashMap<String, NodeIndex>,
    inh_node_map: HashMap<String, NodeIndex>,
    call_node_map: HashMap<String, NodeIndex>,
}

impl CodeGraphs {
    pub fn new() -> Self {
        Self {
            dependency_graph: DiGraph::new(),
            inheritance_graph: DiGraph::new(),
            call_graph: DiGraph::new(),
            dep_node_map: HashMap::new(),
            inh_node_map: HashMap::new(),
            call_node_map: HashMap::new(),
        }
    }

    /// Build all graphs from indexed elements and global index.
    pub fn build(
        &mut self,
        elements: &[CodeElement],
        parse_results: &[FileParseResult],
        global_index: &GlobalIndex,
        repo_root: &str,
    ) {
        self.build_dependency_graph(elements, parse_results, global_index, repo_root);
        self.build_inheritance_graph(elements, global_index);
        self.build_call_graph(elements, parse_results, global_index, repo_root);
    }

    // ─── Dependency Graph ───────────────────────────────────────────────

    fn build_dependency_graph(
        &mut self,
        elements: &[CodeElement],
        parse_results: &[FileParseResult],
        global_index: &GlobalIndex,
        repo_root: &str,
    ) {
        // Ensure all file elements have nodes
        for elem in elements.iter().filter(|e| e.element_type == "file") {
            self.ensure_dep_node(&elem.id);
        }

        // Process imports from each file
        for pr in parse_results {
            let source_file_id = match global_index.file_map.get(&pr.file_path) {
                Some(id) => id.clone(),
                None => continue,
            };

            for imp in &pr.imports {
                if let Some(target_file_id) =
                    global_index.resolve_import(&pr.file_path, &imp.module, imp.level, repo_root)
                {
                    if target_file_id != source_file_id {
                        let source_idx = self.ensure_dep_node(&source_file_id);
                        let target_idx = self.ensure_dep_node(&target_file_id);
                        self.dependency_graph.add_edge(
                            source_idx,
                            target_idx,
                            "imports".to_string(),
                        );
                    }
                }
            }
        }
    }

    fn ensure_dep_node(&mut self, id: &str) -> NodeIndex {
        *self
            .dep_node_map
            .entry(id.to_string())
            .or_insert_with(|| self.dependency_graph.add_node(id.to_string()))
    }

    // ─── Inheritance Graph ──────────────────────────────────────────────

    fn build_inheritance_graph(&mut self, elements: &[CodeElement], _global_index: &GlobalIndex) {
        // Build a name→ID map for all classes
        let mut class_name_map: HashMap<String, String> = HashMap::new();
        for elem in elements.iter().filter(|e| e.element_type == "class") {
            class_name_map.insert(elem.name.clone(), elem.id.clone());
        }

        // Process class bases
        for elem in elements.iter().filter(|e| e.element_type == "class") {
            if let Some(bases) = elem.metadata.get("bases") {
                if let Some(base_array) = bases.as_array() {
                    for base_val in base_array {
                        if let Some(base_name) = base_val.as_str() {
                            // Try to find the base class
                            if let Some(base_id) = class_name_map.get(base_name) {
                                if *base_id != elem.id {
                                    let child_idx = self.ensure_inh_node(&elem.id);
                                    let parent_idx = self.ensure_inh_node(base_id);
                                    self.inheritance_graph.add_edge(
                                        child_idx,
                                        parent_idx,
                                        "extends".to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn ensure_inh_node(&mut self, id: &str) -> NodeIndex {
        *self
            .inh_node_map
            .entry(id.to_string())
            .or_insert_with(|| self.inheritance_graph.add_node(id.to_string()))
    }

    // ─── Call Graph ─────────────────────────────────────────────────────

    fn build_call_graph(
        &mut self,
        elements: &[CodeElement],
        parse_results: &[FileParseResult],
        global_index: &GlobalIndex,
        repo_root: &str,
    ) {
        // Build function name→ID map
        let mut func_name_map: HashMap<String, String> = HashMap::new();
        for elem in elements.iter().filter(|e| e.element_type == "function") {
            let simple_name = elem.name.rsplit('.').next().unwrap_or(&elem.name);
            func_name_map.insert(simple_name.to_string(), elem.id.clone());
        }

        // Process calls from each file
        for pr in parse_results {
            let file_id = match global_index.file_map.get(&pr.file_path) {
                Some(id) => id.clone(),
                None => continue,
            };

            for call in &pr.calls {
                // Find the caller (function containing this call)
                let caller_id = find_enclosing_function(elements, &pr.file_path, call.line);

                // Find the callee
                let callee_id = global_index
                    .resolve_symbol(&call.callee_name, &file_id, repo_root)
                    .or_else(|| func_name_map.get(&call.callee_name).cloned());

                if let (Some(caller), Some(callee)) = (caller_id, callee_id) {
                    if caller != callee {
                        let caller_idx = self.ensure_call_node(&caller);
                        let callee_idx = self.ensure_call_node(&callee);
                        self.call_graph
                            .add_edge(caller_idx, callee_idx, "calls".to_string());
                    }
                }
            }
        }
    }

    fn ensure_call_node(&mut self, id: &str) -> NodeIndex {
        *self
            .call_node_map
            .entry(id.to_string())
            .or_insert_with(|| self.call_graph.add_node(id.to_string()))
    }

    // ─── Query Methods ──────────────────────────────────────────────────

    /// Get direct dependencies of a file (files it imports).
    pub fn get_dependencies(&self, element_id: &str) -> Vec<String> {
        self.get_neighbors(
            &self.dependency_graph,
            &self.dep_node_map,
            element_id,
            Direction::Outgoing,
        )
    }

    /// Get files that depend on this file (files that import it).
    pub fn get_dependents(&self, element_id: &str) -> Vec<String> {
        self.get_neighbors(
            &self.dependency_graph,
            &self.dep_node_map,
            element_id,
            Direction::Incoming,
        )
    }

    /// Get file links (dependencies and dependents) for a given file path.
    pub fn get_file_links(&self, file_id: &str, global_index: &GlobalIndex) -> Vec<CodeLink> {
        let mut links = Vec::new();

        // Dependencies (outgoing)
        for dep_id in self.get_dependencies(file_id) {
            if let Some(elem) = global_index.get_element(&dep_id) {
                links.push(CodeLink {
                    source_id: file_id.to_string(),
                    target_id: dep_id.clone(),
                    link_type: "dependency".to_string(),
                    source_name: global_index
                        .get_element(file_id)
                        .map(|e| e.relative_path.clone())
                        .unwrap_or_default(),
                    target_name: elem.relative_path.clone(),
                });
            }
        }

        // Dependents (incoming)
        for dep_id in self.get_dependents(file_id) {
            if let Some(elem) = global_index.get_element(&dep_id) {
                links.push(CodeLink {
                    source_id: dep_id.clone(),
                    target_id: file_id.to_string(),
                    link_type: "dependent".to_string(),
                    source_name: elem.relative_path.clone(),
                    target_name: global_index
                        .get_element(file_id)
                        .map(|e| e.relative_path.clone())
                        .unwrap_or_default(),
                });
            }
        }

        links
    }

    /// Get graph statistics.
    pub fn get_stats(&self) -> GraphStats {
        GraphStats {
            dependency_nodes: self.dependency_graph.node_count(),
            dependency_edges: self.dependency_graph.edge_count(),
            inheritance_nodes: self.inheritance_graph.node_count(),
            inheritance_edges: self.inheritance_graph.edge_count(),
            call_nodes: self.call_graph.node_count(),
            call_edges: self.call_graph.edge_count(),
        }
    }

    fn get_neighbors(
        &self,
        graph: &DiGraph<String, String>,
        node_map: &HashMap<String, NodeIndex>,
        element_id: &str,
        direction: Direction,
    ) -> Vec<String> {
        if let Some(&idx) = node_map.get(element_id) {
            graph
                .neighbors_directed(idx, direction)
                .map(|n| graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStats {
    pub dependency_nodes: usize,
    pub dependency_edges: usize,
    pub inheritance_nodes: usize,
    pub inheritance_edges: usize,
    pub call_nodes: usize,
    pub call_edges: usize,
}

/// Find which function element encloses a given line in a file.
fn find_enclosing_function(
    elements: &[CodeElement],
    file_path: &str,
    line: usize,
) -> Option<String> {
    let mut best: Option<&CodeElement> = None;

    for elem in elements
        .iter()
        .filter(|e| e.element_type == "function" && e.file_path == file_path)
    {
        if elem.start_line <= line && line <= elem.end_line {
            // Prefer the most specific (smallest range) function
            if let Some(current_best) = best {
                let current_range = current_best.end_line - current_best.start_line;
                let new_range = elem.end_line - elem.start_line;
                if new_range < current_range {
                    best = Some(elem);
                }
            } else {
                best = Some(elem);
            }
        }
    }

    best.map(|e| e.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_graphs_new() {
        let graphs = CodeGraphs::new();
        assert_eq!(graphs.dependency_graph.node_count(), 0);
        assert_eq!(graphs.inheritance_graph.node_count(), 0);
        assert_eq!(graphs.call_graph.node_count(), 0);
    }
}
