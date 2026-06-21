//! Shared data types for the code intelligence system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about a parsed function or method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub docstring: Option<String>,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_method: bool,
    pub class_name: Option<String>,
    pub decorators: Vec<String>,
    pub complexity: usize,
}

/// Information about a parsed class.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub docstring: Option<String>,
    pub bases: Vec<String>,
    pub methods: Vec<FunctionInfo>,
    pub decorators: Vec<String>,
}

/// Information about an import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    pub module: String,
    pub names: Vec<String>,
    pub alias: Option<String>,
    pub is_from: bool,
    pub line: usize,
    /// Relative import level (0 = absolute, 1 = ., 2 = .., etc.)
    pub level: usize,
}

/// Information about a function call site.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallInfo {
    pub callee_name: String,
    /// e.g. "self.method", "module.func"
    pub qualifier: Option<String>,
    pub line: usize,
    /// The scope (function/class) containing this call
    pub scope: Option<String>,
    pub scope_type: Option<String>,
}

/// Result of parsing a single source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileParseResult {
    pub file_path: String,
    pub relative_path: String,
    pub language: String,
    pub classes: Vec<ClassInfo>,
    pub functions: Vec<FunctionInfo>,
    pub imports: Vec<ImportInfo>,
    pub calls: Vec<CallInfo>,
    pub module_docstring: Option<String>,
    pub total_lines: usize,
    pub code_lines: usize,
}

/// A scanned file entry from the loader.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub relative_path: String,
    pub language: String,
    pub extension: String,
    pub size_bytes: u64,
}

/// A unified code element at any granularity level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeElement {
    pub id: String,
    /// "file", "class", "function", "documentation"
    pub element_type: String,
    pub name: String,
    pub file_path: String,
    pub relative_path: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    pub code: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub summary: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Statistics about the code intelligence scan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub total_files: usize,
    pub total_classes: usize,
    pub total_functions: usize,
    pub total_imports: usize,
    pub total_elements: usize,
    pub total_lines: usize,
    pub languages: HashMap<String, usize>,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub scan_duration_ms: u64,
}

/// A link/relationship between two code elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLink {
    pub source_id: String,
    pub target_id: String,
    /// "dependency", "inheritance", "call"
    pub link_type: String,
    pub source_name: String,
    pub target_name: String,
}
