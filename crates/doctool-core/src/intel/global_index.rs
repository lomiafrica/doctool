//! Global index builder — file/module/symbol lookup maps.

use std::collections::HashMap;

use super::types::*;
use super::utils::file_path_to_module_path;

/// Global index for fast lookups of code elements.
#[derive(Debug, Default)]
pub struct GlobalIndex {
    /// abs_path → file element ID
    pub file_map: HashMap<String, String>,
    /// dotted.module.path → file element ID
    pub module_map: HashMap<String, String>,
    /// module_path → { symbol_name → element ID }
    pub export_map: HashMap<String, HashMap<String, String>>,
    /// element ID → CodeElement (for quick access)
    pub elements_by_id: HashMap<String, CodeElement>,
    /// file element ID → list of import infos
    pub file_imports: HashMap<String, Vec<ImportInfo>>,
}

impl GlobalIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build all lookup maps from a set of CodeElements.
    pub fn build(
        &mut self,
        elements: &[CodeElement],
        parse_results: &[FileParseResult],
        repo_root: &str,
    ) {
        self.file_map.clear();
        self.module_map.clear();
        self.export_map.clear();
        self.elements_by_id.clear();
        self.file_imports.clear();

        // Index all elements by ID
        for elem in elements {
            self.elements_by_id.insert(elem.id.clone(), elem.clone());
        }

        // Build file_map and module_map from file-level elements
        for elem in elements.iter().filter(|e| e.element_type == "file") {
            let abs_path = &elem.file_path;
            self.file_map.insert(abs_path.clone(), elem.id.clone());

            if let Some(module_path) = file_path_to_module_path(abs_path, repo_root) {
                self.module_map.insert(module_path, elem.id.clone());
            }
        }

        // Build export map from class/function/header/tag elements
        for elem in elements.iter().filter(|e| {
            e.element_type == "class"
                || e.element_type == "function"
                || e.element_type == "header"
                || e.element_type == "tag"
        }) {
            if let Some(module_path) = file_path_to_module_path(&elem.file_path, repo_root) {
                let exports = self.export_map.entry(module_path).or_default();
                // Use the simple name (without class prefix)
                let simple_name = elem.name.rsplit('.').next().unwrap_or(&elem.name);
                exports.insert(simple_name.to_string(), elem.id.clone());
                // For markdown, also insert the full name exactly
                if elem.element_type == "header" || elem.element_type == "tag" {
                    exports.insert(elem.name.clone(), elem.id.clone());
                }
            }
        }

        // Store imports per file
        for pr in parse_results {
            if let Some(file_id) = self.file_map.get(&pr.file_path) {
                self.file_imports
                    .insert(file_id.clone(), pr.imports.clone());
            }
        }
    }

    /// Resolve an import to a target file ID.
    pub fn resolve_import(
        &self,
        current_file_path: &str,
        module_name: &str,
        level: usize,
        repo_root: &str,
    ) -> Option<String> {
        if level > 0 {
            // Relative import
            self.resolve_relative_import(current_file_path, module_name, level, repo_root)
        } else {
            // Absolute import
            self.resolve_absolute_import(module_name)
        }
    }

    fn resolve_relative_import(
        &self,
        current_file_path: &str,
        import_name: &str,
        level: usize,
        repo_root: &str,
    ) -> Option<String> {
        let current_module = file_path_to_module_path(current_file_path, repo_root)?;
        let parts: Vec<&str> = current_module.split('.').collect();

        let is_init = current_file_path.ends_with("__init__.py");
        let strip_count = if is_init { level - 1 } else { level };

        if strip_count > parts.len() {
            return None;
        }

        let parent_parts = if strip_count > 0 {
            &parts[..parts.len() - strip_count]
        } else {
            &parts
        };

        let target_module = if import_name.is_empty() {
            parent_parts.join(".")
        } else {
            format!("{}.{}", parent_parts.join("."), import_name)
        };

        self.module_map.get(&target_module).cloned()
    }

    fn resolve_absolute_import(&self, import_name: &str) -> Option<String> {
        self.module_map.get(import_name).cloned()
    }

    /// Resolve a symbol name to its element ID.
    pub fn resolve_symbol(
        &self,
        symbol_name: &str,
        current_file_id: &str,
        repo_root: &str,
    ) -> Option<String> {
        // 1. Check local definitions in the same file
        for elem in self.elements_by_id.values() {
            if elem.file_path == self.get_file_path(current_file_id)
                && (elem.element_type == "class"
                    || elem.element_type == "function"
                    || elem.element_type == "header"
                    || elem.element_type == "tag")
            {
                // For markdown elements, exact match or simple name
                let simple_name = elem.name.rsplit('.').next().unwrap_or(&elem.name);
                if simple_name == symbol_name || elem.name == symbol_name {
                    return Some(elem.id.clone());
                }
            }
        }

        // 2. Check through imports
        if let Some(imports) = self.file_imports.get(current_file_id) {
            let file_path = self.get_file_path(current_file_id);
            for imp in imports {
                if imp.names.contains(&symbol_name.to_string())
                    || imp.names.contains(&"*".to_string())
                {
                    // Resolve the import module to a file ID
                    if let Some(target_file_id) =
                        self.resolve_import(&file_path, &imp.module, imp.level, repo_root)
                    {
                        // Look for the symbol in that file's exports
                        if let Some(module_path) = self.get_module_path_by_file_id(&target_file_id)
                        {
                            if let Some(exports) = self.export_map.get(&module_path) {
                                if let Some(elem_id) = exports.get(symbol_name) {
                                    return Some(elem_id.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn get_file_path(&self, file_id: &str) -> String {
        self.elements_by_id
            .get(file_id)
            .map(|e| e.file_path.clone())
            .unwrap_or_default()
    }

    fn get_module_path_by_file_id(&self, file_id: &str) -> Option<String> {
        for (module, fid) in &self.module_map {
            if fid == file_id {
                return Some(module.clone());
            }
        }
        None
    }

    /// Get element by ID.
    pub fn get_element(&self, id: &str) -> Option<&CodeElement> {
        self.elements_by_id.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_index_build() {
        let elements = vec![
            CodeElement {
                id: "file_abc".to_string(),
                element_type: "file".to_string(),
                name: "main.py".to_string(),
                file_path: "/repo/app/main.py".to_string(),
                relative_path: "app/main.py".to_string(),
                language: "python".to_string(),
                start_line: 1,
                end_line: 10,
                code: String::new(),
                signature: None,
                docstring: None,
                summary: None,
                metadata: HashMap::new(),
            },
            CodeElement {
                id: "func_def".to_string(),
                element_type: "function".to_string(),
                name: "main".to_string(),
                file_path: "/repo/app/main.py".to_string(),
                relative_path: "app/main.py".to_string(),
                language: "python".to_string(),
                start_line: 5,
                end_line: 10,
                code: String::new(),
                signature: None,
                docstring: None,
                summary: None,
                metadata: HashMap::new(),
            },
        ];

        let mut index = GlobalIndex::new();
        index.build(&elements, &[], "/repo");

        assert_eq!(index.file_map.len(), 1);
        assert!(index.module_map.contains_key("app.main"));
        assert!(index.export_map.contains_key("app.main"));
    }
}
