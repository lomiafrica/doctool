//! Multi-level code element indexer.
//! Creates CodeElements at file, class, and function levels.

use std::collections::HashMap;

use super::types::*;
use super::utils::compute_hash;

/// Index a parsed file into multiple CodeElement entries.
pub fn index_file(parse_result: &FileParseResult, content: &str) -> Vec<CodeElement> {
    let mut elements = Vec::new();

    // ─── File-level element ─────────────────────────────────────────────
    let file_summary = generate_file_summary(parse_result);
    let file_id = generate_id("file", &[&parse_result.file_path]);

    elements.push(CodeElement {
        id: file_id.clone(),
        element_type: "file".to_string(),
        name: parse_result
            .relative_path
            .rsplit('/')
            .next()
            .unwrap_or(&parse_result.relative_path)
            .to_string(),
        file_path: parse_result.file_path.clone(),
        relative_path: parse_result.relative_path.clone(),
        language: parse_result.language.clone(),
        start_line: 1,
        end_line: parse_result.total_lines,
        code: content.to_string(),
        signature: None,
        docstring: parse_result.module_docstring.clone(),
        summary: Some(file_summary),
        metadata: build_file_metadata(parse_result),
    });

    // ─── Class-level elements ───────────────────────────────────────────
    for class in &parse_result.classes {
        let is_md = parse_result.language == "md" || parse_result.language == "markdown";
        let elem_type = if is_md { "header" } else { "class" };
        let class_id = generate_id(elem_type, &[&parse_result.file_path, &class.name]);
        let class_code = extract_lines(content, class.start_line, class.end_line);

        let mut metadata = HashMap::new();
        metadata.insert(
            "bases".to_string(),
            serde_json::Value::Array(
                class
                    .bases
                    .iter()
                    .map(|b| serde_json::Value::String(b.clone()))
                    .collect(),
            ),
        );
        metadata.insert(
            "method_count".to_string(),
            serde_json::json!(class.methods.len()),
        );
        metadata.insert(
            "decorators".to_string(),
            serde_json::Value::Array(
                class
                    .decorators
                    .iter()
                    .map(|d| serde_json::Value::String(d.clone()))
                    .collect(),
            ),
        );

        let signature = if is_md {
            class.name.clone()
        } else {
            format!(
                "class {}{}",
                class.name,
                if class.bases.is_empty() {
                    String::new()
                } else {
                    format!("({})", class.bases.join(", "))
                }
            )
        };

        elements.push(CodeElement {
            id: class_id,
            element_type: elem_type.to_string(),
            name: class.name.clone(),
            file_path: parse_result.file_path.clone(),
            relative_path: parse_result.relative_path.clone(),
            language: parse_result.language.clone(),
            start_line: class.start_line,
            end_line: class.end_line,
            code: class_code,
            signature: Some(signature),
            docstring: class.docstring.clone(),
            summary: None,
            metadata,
        });

        // ─── Method-level elements ──────────────────────────────────────
        for method in &class.methods {
            let method_id = generate_id(
                "function",
                &[&parse_result.file_path, &class.name, &method.name],
            );
            let method_code = extract_lines(content, method.start_line, method.end_line);

            let signature = format!(
                "{}def {}({})",
                if method.is_async { "async " } else { "" },
                method.name,
                method.parameters.join(", ")
            );

            let mut metadata = HashMap::new();
            metadata.insert(
                "class_name".to_string(),
                serde_json::Value::String(class.name.clone()),
            );
            metadata.insert("is_method".to_string(), serde_json::json!(true));

            elements.push(CodeElement {
                id: method_id,
                element_type: "function".to_string(),
                name: format!("{}.{}", class.name, method.name),
                file_path: parse_result.file_path.clone(),
                relative_path: parse_result.relative_path.clone(),
                language: parse_result.language.clone(),
                start_line: method.start_line,
                end_line: method.end_line,
                code: method_code,
                signature: Some(signature),
                docstring: method.docstring.clone(),
                summary: None,
                metadata,
            });
        }
    }

    // ─── Top-level function elements ────────────────────────────────────
    for func in &parse_result.functions {
        let is_md = parse_result.language == "md" || parse_result.language == "markdown";
        let elem_type = if is_md { "tag" } else { "function" };
        let func_id = generate_id(elem_type, &[&parse_result.file_path, &func.name]);
        let func_code = extract_lines(content, func.start_line, func.end_line);

        let signature = if is_md {
            func.name.clone()
        } else {
            format!(
                "{}def {}({})",
                if func.is_async { "async " } else { "" },
                func.name,
                func.parameters.join(", ")
            )
        };

        elements.push(CodeElement {
            id: func_id,
            element_type: elem_type.to_string(),
            name: func.name.clone(),
            file_path: parse_result.file_path.clone(),
            relative_path: parse_result.relative_path.clone(),
            language: parse_result.language.clone(),
            start_line: func.start_line,
            end_line: func.end_line,
            code: func_code,
            signature: Some(signature),
            docstring: func.docstring.clone(),
            summary: None,
            metadata: HashMap::new(),
        });
    }

    elements
}

/// Generate a deterministic unique ID for a code element.
pub fn generate_id(element_type: &str, parts: &[&str]) -> String {
    let hash = compute_hash(parts);
    format!("{}_{}", element_type, &hash[..12])
}

/// Extract a range of lines from content (1-indexed, inclusive).
fn extract_lines(content: &str, start: usize, end: usize) -> String {
    content
        .lines()
        .skip(start.saturating_sub(1))
        .take(end.saturating_sub(start.saturating_sub(1)))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Generate a summary string for a file.
fn generate_file_summary(pr: &FileParseResult) -> String {
    let mut parts = Vec::new();
    parts.push(format!("{} file", pr.language));

    if !pr.classes.is_empty() {
        let label = if pr.language == "md" || pr.language == "markdown" {
            "header"
        } else {
            "class"
        };
        let class_names: Vec<&str> = pr.classes.iter().map(|c| c.name.as_str()).collect();
        parts.push(format!(
            "{} {}{}: {}",
            pr.classes.len(),
            label,
            if pr.classes.len() == 1 {
                ""
            } else {
                if label == "class" {
                    "es"
                } else {
                    "s"
                }
            },
            class_names.join(", ")
        ));
    }

    if !pr.functions.is_empty() {
        let label = if pr.language == "md" || pr.language == "markdown" {
            "tag"
        } else {
            "function"
        };
        let func_names: Vec<&str> = pr.functions.iter().map(|f| f.name.as_str()).collect();
        parts.push(format!(
            "{} {}{}: {}",
            pr.functions.len(),
            label,
            if pr.functions.len() == 1 { "" } else { "s" },
            func_names.join(", ")
        ));
    }

    parts.push(format!("{} lines", pr.total_lines));

    parts.join(", ")
}

/// Build metadata for a file-level element.
fn build_file_metadata(pr: &FileParseResult) -> HashMap<String, serde_json::Value> {
    let mut metadata = HashMap::new();
    metadata.insert("total_lines".to_string(), serde_json::json!(pr.total_lines));
    metadata.insert("code_lines".to_string(), serde_json::json!(pr.code_lines));

    let is_md = pr.language == "md" || pr.language == "markdown";
    let class_key = if is_md { "header_count" } else { "class_count" };
    let func_key = if is_md { "tag_count" } else { "function_count" };

    metadata.insert(class_key.to_string(), serde_json::json!(pr.classes.len()));
    metadata.insert(func_key.to_string(), serde_json::json!(pr.functions.len()));
    metadata.insert(
        "import_count".to_string(),
        serde_json::json!(pr.imports.len()),
    );

    let class_names: Vec<String> = pr.classes.iter().map(|c| c.name.clone()).collect();
    metadata.insert(
        if is_md {
            "headers".to_string()
        } else {
            "classes".to_string()
        },
        serde_json::Value::Array(
            class_names
                .iter()
                .map(|n| serde_json::Value::String(n.clone()))
                .collect(),
        ),
    );

    let func_names: Vec<String> = pr.functions.iter().map(|f| f.name.clone()).collect();
    metadata.insert(
        if is_md {
            "tags".to_string()
        } else {
            "functions".to_string()
        },
        serde_json::Value::Array(
            func_names
                .iter()
                .map(|n| serde_json::Value::String(n.clone()))
                .collect(),
        ),
    );

    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_file() {
        let pr = FileParseResult {
            file_path: "/test/main.py".to_string(),
            relative_path: "main.py".to_string(),
            language: "python".to_string(),
            classes: vec![ClassInfo {
                name: "Foo".to_string(),
                start_line: 1,
                end_line: 5,
                docstring: None,
                bases: vec!["Base".to_string()],
                methods: vec![FunctionInfo {
                    name: "bar".to_string(),
                    start_line: 3,
                    end_line: 5,
                    docstring: None,
                    parameters: vec!["self".to_string()],
                    return_type: None,
                    is_async: false,
                    is_method: true,
                    class_name: Some("Foo".to_string()),
                    decorators: Vec::new(),
                    complexity: 1,
                }],
                decorators: Vec::new(),
            }],
            functions: vec![FunctionInfo {
                name: "main".to_string(),
                start_line: 7,
                end_line: 8,
                docstring: None,
                parameters: Vec::new(),
                return_type: None,
                is_async: false,
                is_method: false,
                class_name: None,
                decorators: Vec::new(),
                complexity: 1,
            }],
            imports: Vec::new(),
            calls: Vec::new(),
            module_docstring: None,
            total_lines: 8,
            code_lines: 6,
        };

        let content = "class Foo(Base):\n    pass\n    def bar(self):\n        pass\n\ndef main():\n    pass\n";
        let elements = index_file(&pr, content);

        // Should have: 1 file + 1 class + 1 method + 1 function = 4
        assert_eq!(elements.len(), 4);

        let types: Vec<&str> = elements.iter().map(|e| e.element_type.as_str()).collect();
        assert!(types.contains(&"file"));
        assert!(types.contains(&"class"));
        assert_eq!(types.iter().filter(|&&t| t == "function").count(), 2);
    }
}
