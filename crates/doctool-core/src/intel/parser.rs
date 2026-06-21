//! Tree-sitter based code parser for multiple languages.

use regex::Regex;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

use super::types::*;

/// Get the tree-sitter Language for a given language name.
fn get_ts_language(lang: &str) -> Option<Language> {
    match lang {
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "javascript" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "c" => Some(tree_sitter_c::LANGUAGE.into()),
        "cpp" => Some(tree_sitter_cpp::LANGUAGE.into()),
        _ => None,
    }
}

/// Parse a single source file and extract structured information.
pub fn parse_file(
    file_path: &str,
    content: &str,
    language: &str,
    root: &str,
) -> Option<FileParseResult> {
    let ts_lang = get_ts_language(language)?;

    let mut parser = Parser::new();
    parser.set_language(&ts_lang).ok()?;

    let tree = parser.parse(content.as_bytes(), None)?;
    let root_node = tree.root_node();
    let code_bytes = content.as_bytes();

    let relative_path = Path::new(file_path)
        .strip_prefix(root)
        .unwrap_or(Path::new(file_path))
        .to_str()
        .unwrap_or(file_path)
        .to_string();

    let total_lines = content.lines().count();
    let code_lines = content
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//")
        })
        .count();

    let mut classes = Vec::new();
    let mut functions = Vec::new();
    let mut imports = Vec::new();
    let mut calls = Vec::new();
    let module_docstring;

    match language {
        "python" => {
            module_docstring = extract_python_module_docstring(root_node, code_bytes);
            extract_python_classes(root_node, code_bytes, &mut classes, content);
            extract_python_functions(root_node, code_bytes, &mut functions, None, content);
            extract_python_imports(root_node, code_bytes, &mut imports);
            extract_python_calls(root_node, code_bytes, &mut calls);
        }
        "javascript" | "jsx" | "typescript" | "tsx" => {
            module_docstring = extract_js_module_docstring(root_node, code_bytes);
            extract_js_classes(root_node, code_bytes, &mut classes, content, &ts_lang);
            extract_js_functions(
                root_node,
                code_bytes,
                &mut functions,
                None,
                content,
                &ts_lang,
            );
            extract_js_imports(root_node, code_bytes, &mut imports);
            extract_js_calls(root_node, code_bytes, &mut calls);
        }
        "rust" => {
            module_docstring = extract_rust_module_docstring(root_node, code_bytes);
            extract_rust_structs(root_node, code_bytes, &mut classes, content);
            extract_rust_functions(root_node, code_bytes, &mut functions, None, content);
            extract_rust_imports(root_node, code_bytes, &mut imports);
            extract_rust_calls(root_node, code_bytes, &mut calls);
        }
        "go" => {
            module_docstring = None;
            extract_go_structs(root_node, code_bytes, &mut classes, content);
            extract_go_functions(root_node, code_bytes, &mut functions, content);
            extract_go_imports(root_node, code_bytes, &mut imports);
            calls = Vec::new(); // Go call extraction can be added later
        }
        "java" => {
            module_docstring = None;
            extract_java_classes(root_node, code_bytes, &mut classes, content);
            extract_java_functions(root_node, code_bytes, &mut functions, None, content);
            extract_java_imports(root_node, code_bytes, &mut imports);
            calls = Vec::new();
        }
        "c" | "cpp" => {
            module_docstring = None;
            extract_c_structs(root_node, code_bytes, &mut classes, content);
            extract_c_functions(root_node, code_bytes, &mut functions, content);
            extract_c_includes(root_node, code_bytes, &mut imports);
            calls = Vec::new();
        }
        "md" | "markdown" => {
            module_docstring = None;
            extract_markdown_elements(content, &mut classes, &mut imports, &mut functions);
            calls = Vec::new();
        }
        _ => {
            module_docstring = None;
        }
    }

    Some(FileParseResult {
        file_path: file_path.to_string(),
        relative_path,
        language: language.to_string(),
        classes,
        functions,
        imports,
        calls,
        module_docstring,
        total_lines,
        code_lines,
    })
}

// ─── Markdown extraction (Regex-based) ───────────────────────────────────────

fn extract_markdown_elements(
    content: &str,
    classes: &mut Vec<ClassInfo>,
    imports: &mut Vec<ImportInfo>,
    functions: &mut Vec<FunctionInfo>,
) {
    // We treat markdown headers as "classes" so they can be jumped to
    let header_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
    // We treat wiki-links [[Note Name]] as "imports"
    let link_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    // We treat tags #tag (but not # header) as "functions" for indexing purposes
    // or we could just append them to the class's tags. For now, let's just make functions.
    let tag_re = Regex::new(r"(?:\s|^)(#[a-zA-Z0-9_\-]+)").unwrap();

    let mut current_header: Option<ClassInfo> = None;

    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;
        let line_trim = line.trim();

        if let Some(caps) = header_re.captures(line_trim) {
            // Check if it's a real header, not just a line that starts with # inside a code block
            // (A fully robust parser would track code block state, but a simple regex is fine for now)
            if let Some(mut prev_header) = current_header.take() {
                prev_header.end_line = line_num.saturating_sub(1);
                classes.push(prev_header);
            }

            let name = caps.get(2).unwrap().as_str().to_string();
            current_header = Some(ClassInfo {
                name,
                start_line: line_num,
                end_line: line_num, // will be updated when the next header is found
                docstring: None,
                bases: Vec::new(),
                methods: Vec::new(),
                decorators: Vec::new(),
            });
        }

        // Extract wiki-links -> Imports
        for caps in link_re.captures_iter(line) {
            let note_name = caps.get(1).unwrap().as_str().to_string();
            imports.push(ImportInfo {
                module: note_name.clone(), // We link to the note name directly
                names: vec![note_name],
                alias: None,
                is_from: false,
                line: line_num,
                level: 0,
            });
        }

        // Extract tags -> Functions (for now, simply mapping them to indexable items)
        for caps in tag_re.captures_iter(line) {
            let tag = caps.get(1).unwrap().as_str().to_string();
            functions.push(FunctionInfo {
                name: tag,
                start_line: line_num,
                end_line: line_num,
                docstring: None,
                parameters: Vec::new(),
                return_type: None,
                is_async: false,
                is_method: false,
                class_name: None,
                decorators: Vec::new(),
                complexity: 1,
            });
        }
    }

    if let Some(mut last_header) = current_header.take() {
        last_header.end_line = content.lines().count();
        classes.push(last_header);
    }
}

// ─── Helper: get node text ───────────────────────────────────────────────────

fn node_text<'a>(node: Node<'a>, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

fn node_text_owned(node: Node, src: &[u8]) -> String {
    node_text(node, src).to_string()
}

// ─── Python extraction ──────────────────────────────────────────────────────

fn extract_python_module_docstring(root: Node, src: &[u8]) -> Option<String> {
    // First expression_statement child that is a string
    for i in 0..root.child_count() {
        let child = root.child(i)?;
        if child.kind() == "expression_statement" {
            if let Some(expr) = child.child(0) {
                if expr.kind() == "string" || expr.kind() == "concatenated_string" {
                    let text = node_text_owned(expr, src);
                    return Some(clean_docstring(&text));
                }
            }
        }
        // Skip comments and newlines
        if child.kind() != "comment" && child.kind() != "\n" {
            break;
        }
    }
    None
}

fn extract_python_classes(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>, content: &str) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_definition" {
            if let Some(class_info) = parse_python_class(child, src, content) {
                classes.push(class_info);
            }
        } else if child.kind() == "decorated_definition" {
            if let Some(inner) = child.child_by_field_name("definition") {
                if inner.kind() == "class_definition" {
                    if let Some(mut class_info) = parse_python_class(inner, src, content) {
                        // Extract decorators
                        let mut dcursor = child.walk();
                        for dchild in child.children(&mut dcursor) {
                            if dchild.kind() == "decorator" {
                                class_info.decorators.push(node_text_owned(dchild, src));
                            }
                        }
                        classes.push(class_info);
                    }
                }
            }
        }
        // Recurse into if/try/with blocks
        if matches!(
            child.kind(),
            "if_statement" | "try_statement" | "with_statement"
        ) {
            extract_python_classes(child, src, classes, content);
        }
    }
}

fn parse_python_class(node: Node, src: &[u8], content: &str) -> Option<ClassInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text_owned(name_node, src);

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    // Extract bases
    let mut bases = Vec::new();
    if let Some(args) = node.child_by_field_name("superclasses") {
        let mut cursor = args.walk();
        for child in args.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "attribute" {
                bases.push(node_text_owned(child, src));
            }
        }
    }

    // Extract docstring
    let docstring = extract_body_docstring(node, src);

    // Extract methods
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_python_functions(body, src, &mut methods, Some(&name), content);
    }

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        docstring,
        bases,
        methods,
        decorators: Vec::new(),
    })
}

fn extract_python_functions(
    node: Node,
    src: &[u8],
    functions: &mut Vec<FunctionInfo>,
    class_name: Option<&str>,
    _content: &str,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let (target_node, decorators) = if child.kind() == "decorated_definition" {
            let mut decos = Vec::new();
            let mut dcursor = child.walk();
            for dchild in child.children(&mut dcursor) {
                if dchild.kind() == "decorator" {
                    decos.push(node_text_owned(dchild, src));
                }
            }
            if let Some(inner) = child.child_by_field_name("definition") {
                (inner, decos)
            } else {
                continue;
            }
        } else if child.kind() == "function_definition" {
            (child, Vec::new())
        } else {
            // Recurse into if/try/with blocks when at module level
            if class_name.is_none()
                && matches!(
                    child.kind(),
                    "if_statement" | "try_statement" | "with_statement"
                )
            {
                extract_python_functions(child, src, functions, class_name, _content);
            }
            continue;
        };

        if target_node.kind() != "function_definition" {
            continue;
        }

        if let Some(mut func) = parse_python_function(target_node, src, class_name) {
            func.decorators = decorators;
            functions.push(func);
        }
    }
}

fn parse_python_function(node: Node, src: &[u8], class_name: Option<&str>) -> Option<FunctionInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text_owned(name_node, src);

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    // Check if async
    let is_async = node
        .parent()
        .is_some_and(|p| p.kind() == "decorated_definition")
        || node_text(node, src).starts_with("async ");

    // Extract parameters
    let mut parameters = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        let mut pcursor = params.walk();
        for param in params.children(&mut pcursor) {
            match param.kind() {
                "identifier" => parameters.push(node_text_owned(param, src)),
                "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                    if let Some(p_name) = param.child_by_field_name("name") {
                        parameters.push(node_text_owned(p_name, src));
                    } else {
                        // fallback: first child
                        if let Some(first) = param.child(0) {
                            parameters.push(node_text_owned(first, src));
                        }
                    }
                }
                "list_splat_pattern" | "dictionary_splat_pattern" => {
                    parameters.push(node_text_owned(param, src));
                }
                _ => {}
            }
        }
    }

    // Extract return type
    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| node_text_owned(n, src));

    // Extract docstring
    let docstring = extract_body_docstring(node, src);

    let is_method = class_name.is_some();

    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        docstring,
        parameters,
        return_type,
        is_async,
        is_method,
        class_name: class_name.map(|s| s.to_string()),
        decorators: Vec::new(),
        complexity: 1,
    })
}

fn extract_body_docstring(node: Node, src: &[u8]) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let first_stmt = body.child(0)?;
    if first_stmt.kind() == "expression_statement" {
        if let Some(expr) = first_stmt.child(0) {
            if expr.kind() == "string" || expr.kind() == "concatenated_string" {
                return Some(clean_docstring(&node_text_owned(expr, src)));
            }
        }
    }
    None
}

fn extract_python_imports(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_statement" => {
                // import foo / import foo as bar
                let mut icursor = child.walk();
                for item in child.children(&mut icursor) {
                    if item.kind() == "dotted_name" {
                        let name = node_text_owned(item, src);
                        imports.push(ImportInfo {
                            module: name.clone(),
                            names: vec![name],
                            alias: None,
                            is_from: false,
                            line: child.start_position().row + 1,
                            level: 0,
                        });
                    } else if item.kind() == "aliased_import" {
                        let name = item
                            .child_by_field_name("name")
                            .map(|n| node_text_owned(n, src))
                            .unwrap_or_default();
                        let alias = item
                            .child_by_field_name("alias")
                            .map(|n| node_text_owned(n, src));
                        imports.push(ImportInfo {
                            module: name.clone(),
                            names: vec![name],
                            alias,
                            is_from: false,
                            line: child.start_position().row + 1,
                            level: 0,
                        });
                    }
                }
            }
            "import_from_statement" => {
                let (module, level) = parse_python_from_import_context(child, src);
                let mut names = Vec::new();
                let mut icursor = child.walk();
                for item in child.children(&mut icursor) {
                    match item.kind() {
                        // Skip the module part (appears before "import" keyword); after "from "
                        "dotted_name" | "identifier"
                            if item.start_byte() > child.start_byte() + 5 =>
                        {
                            let text = node_text_owned(item, src);
                            if text != module {
                                names.push(text);
                            }
                        }
                        "aliased_import" => {
                            if let Some(name_node) = item.child_by_field_name("name") {
                                names.push(node_text_owned(name_node, src));
                            }
                        }
                        "wildcard_import" => {
                            names.push("*".to_string());
                        }
                        _ => {}
                    }
                }
                if names.is_empty() {
                    // from module import name — sometimes the layout differs
                    names.push(module.clone());
                }
                imports.push(ImportInfo {
                    module: module.clone(),
                    names,
                    alias: None,
                    is_from: true,
                    line: child.start_position().row + 1,
                    level,
                });
            }
            _ => {
                // Recurse
                extract_python_imports(child, src, imports);
            }
        }
    }
}

fn parse_python_from_import_context(node: Node, src: &[u8]) -> (String, usize) {
    let mut level = 0usize;
    let mut module = String::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "relative_import" => {
                let text = node_text(child, src);
                level = text.chars().take_while(|&c| c == '.').count();
                module = text[level..].to_string();
            }
            "dotted_name" if level == 0 && module.is_empty() => {
                module = node_text_owned(child, src);
            }
            "import" => break,
            _ => {}
        }
    }

    (module, level)
}

fn extract_python_calls(node: Node, src: &[u8], calls: &mut Vec<CallInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call" {
            if let Some(func) = child.child_by_field_name("function") {
                let (callee, qualifier) = match func.kind() {
                    "identifier" => (node_text_owned(func, src), None),
                    "attribute" => {
                        let full = node_text_owned(func, src);
                        let parts: Vec<&str> = full.rsplitn(2, '.').collect();
                        if parts.len() == 2 {
                            (parts[0].to_string(), Some(parts[1].to_string()))
                        } else {
                            (full, None)
                        }
                    }
                    _ => (node_text_owned(func, src), None),
                };
                calls.push(CallInfo {
                    callee_name: callee,
                    qualifier,
                    line: child.start_position().row + 1,
                    scope: None,
                    scope_type: None,
                });
            }
        }
        extract_python_calls(child, src, calls);
    }
}

// ─── JavaScript/TypeScript extraction ───────────────────────────────────────

fn extract_js_module_docstring(root: Node, src: &[u8]) -> Option<String> {
    let first = root.child(0)?;
    if first.kind() == "comment" {
        let text = node_text_owned(first, src);
        if text.starts_with("/**") || text.starts_with("/*") {
            return Some(clean_docstring(&text));
        }
    }
    None
}

fn extract_js_classes(
    node: Node,
    src: &[u8],
    classes: &mut Vec<ClassInfo>,
    content: &str,
    lang: &Language,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_declaration" || child.kind() == "class" {
            if let Some(class_info) = parse_js_class(child, src, content, lang) {
                classes.push(class_info);
            }
        }
        // Also check export_statement
        if child.kind() == "export_statement" {
            extract_js_classes(child, src, classes, content, lang);
        }
    }
}

fn parse_js_class(node: Node, src: &[u8], content: &str, lang: &Language) -> Option<ClassInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text_owned(name_node, src);

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    // Extract bases (heritage/extends)
    let mut bases = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            let mut hcursor = child.walk();
            for hchild in child.children(&mut hcursor) {
                if hchild.kind() == "identifier" || hchild.kind() == "member_expression" {
                    bases.push(node_text_owned(hchild, src));
                }
            }
        }
    }

    // Extract methods from body
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_js_functions(body, src, &mut methods, Some(&name), content, lang);
    }

    Some(ClassInfo {
        name,
        start_line,
        end_line,
        docstring: None,
        bases,
        methods,
        decorators: Vec::new(),
    })
}

fn extract_js_functions(
    node: Node,
    src: &[u8],
    functions: &mut Vec<FunctionInfo>,
    class_name: Option<&str>,
    _content: &str,
    _lang: &Language,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let target = match child.kind() {
            "function_declaration" | "generator_function_declaration" => Some(child),
            "method_definition" => Some(child),
            "export_statement" => {
                // Recurse into export
                extract_js_functions(child, src, functions, class_name, _content, _lang);
                continue;
            }
            "lexical_declaration" | "variable_declaration" => {
                // Arrow functions: const foo = () => {}
                extract_js_arrow_functions(child, src, functions);
                continue;
            }
            _ => None,
        };

        if let Some(func_node) = target {
            if let Some(func) = parse_js_function(func_node, src, class_name) {
                functions.push(func);
            }
        }
    }
}

fn parse_js_function(node: Node, src: &[u8], class_name: Option<&str>) -> Option<FunctionInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text_owned(name_node, src);

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let is_async = node_text(node, src).starts_with("async ");

    // Extract parameters
    let mut parameters = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        let mut pcursor = params.walk();
        for param in params.children(&mut pcursor) {
            match param.kind() {
                "identifier" | "required_parameter" | "optional_parameter" => {
                    let pname = param
                        .child_by_field_name("pattern")
                        .or_else(|| param.child_by_field_name("name"))
                        .unwrap_or(param);
                    parameters.push(node_text_owned(pname, src));
                }
                "rest_pattern" => parameters.push(node_text_owned(param, src)),
                _ => {}
            }
        }
    }

    Some(FunctionInfo {
        name,
        start_line,
        end_line,
        docstring: None,
        parameters,
        return_type: None,
        is_async,
        is_method: class_name.is_some(),
        class_name: class_name.map(|s| s.to_string()),
        decorators: Vec::new(),
        complexity: 1,
    })
}

fn extract_js_arrow_functions(node: Node, src: &[u8], functions: &mut Vec<FunctionInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let name_node = child.child_by_field_name("name");
            let value_node = child.child_by_field_name("value");
            if let (Some(name), Some(value)) = (name_node, value_node) {
                if value.kind() == "arrow_function" {
                    let fn_name = node_text_owned(name, src);
                    let start_line = child.start_position().row + 1;
                    let end_line = child.end_position().row + 1;
                    let is_async = node_text(value, src).starts_with("async ");

                    let mut parameters = Vec::new();
                    if let Some(params) = value.child_by_field_name("parameters") {
                        let mut pcursor = params.walk();
                        for param in params.children(&mut pcursor) {
                            if param.kind() == "identifier" {
                                parameters.push(node_text_owned(param, src));
                            }
                        }
                    } else if let Some(param) = value.child_by_field_name("parameter") {
                        parameters.push(node_text_owned(param, src));
                    }

                    functions.push(FunctionInfo {
                        name: fn_name,
                        start_line,
                        end_line,
                        docstring: None,
                        parameters,
                        return_type: None,
                        is_async,
                        is_method: false,
                        class_name: None,
                        decorators: Vec::new(),
                        complexity: 1,
                    });
                }
            }
        }
    }
}

fn extract_js_imports(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_statement" {
            let mut names = Vec::new();
            let mut module = String::new();

            let mut icursor = child.walk();
            for item in child.children(&mut icursor) {
                match item.kind() {
                    "import_clause" => {
                        let mut ccursor = item.walk();
                        for clause_child in item.children(&mut ccursor) {
                            match clause_child.kind() {
                                "identifier" => names.push(node_text_owned(clause_child, src)),
                                "named_imports" => {
                                    let mut ncursor = clause_child.walk();
                                    for named in clause_child.children(&mut ncursor) {
                                        if named.kind() == "import_specifier" {
                                            let n =
                                                named.child_by_field_name("name").unwrap_or(named);
                                            names.push(node_text_owned(n, src));
                                        }
                                    }
                                }
                                "namespace_import" => {
                                    names.push(node_text_owned(clause_child, src));
                                }
                                _ => {}
                            }
                        }
                    }
                    "string" | "string_fragment" => {
                        module = node_text(item, src)
                            .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                            .to_string();
                    }
                    _ => {}
                }
            }

            if !module.is_empty() {
                if names.is_empty() {
                    names.push(module.clone());
                }
                let is_relative = module.starts_with('.');
                let level = if is_relative {
                    module.chars().take_while(|&c| c == '.').count()
                } else {
                    0
                };
                imports.push(ImportInfo {
                    module,
                    names,
                    alias: None,
                    is_from: true,
                    line: child.start_position().row + 1,
                    level,
                });
            }
        }
        // Recurse
        if child.kind() != "import_statement" {
            extract_js_imports(child, src, imports);
        }
    }
}

fn extract_js_calls(node: Node, src: &[u8], calls: &mut Vec<CallInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call_expression" {
            if let Some(func) = child.child_by_field_name("function") {
                let (callee, qualifier) = match func.kind() {
                    "identifier" => (node_text_owned(func, src), None),
                    "member_expression" => {
                        let full = node_text_owned(func, src);
                        let parts: Vec<&str> = full.rsplitn(2, '.').collect();
                        if parts.len() == 2 {
                            (parts[0].to_string(), Some(parts[1].to_string()))
                        } else {
                            (full, None)
                        }
                    }
                    _ => (node_text_owned(func, src), None),
                };
                calls.push(CallInfo {
                    callee_name: callee,
                    qualifier,
                    line: child.start_position().row + 1,
                    scope: None,
                    scope_type: None,
                });
            }
        }
        extract_js_calls(child, src, calls);
    }
}

// ─── Rust extraction ────────────────────────────────────────────────────────

fn extract_rust_module_docstring(root: Node, src: &[u8]) -> Option<String> {
    // Look for //! or /*! comments at the top
    let mut doc_lines = Vec::new();
    for i in 0..root.child_count() {
        if let Some(child) = root.child(i) {
            let text = node_text(child, src).trim();
            if child.kind() == "line_comment" && text.starts_with("//!") {
                doc_lines.push(text.trim_start_matches("//!").trim().to_string());
            } else if child.kind() == "block_comment" && text.starts_with("/*!") {
                return Some(clean_docstring(text));
            } else if !doc_lines.is_empty() || (child.kind() != "line_comment") {
                break;
            }
        }
    }
    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

fn extract_rust_structs(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>, content: &str) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "struct_item" | "enum_item" | "trait_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = node_text_owned(name_node, src);
                    let docstring = extract_rust_doc_comment(child, src, node);
                    let start_line = child.start_position().row + 1;
                    let end_line = child.end_position().row + 1;

                    classes.push(ClassInfo {
                        name,
                        start_line,
                        end_line,
                        docstring,
                        bases: Vec::new(),
                        methods: Vec::new(),
                        decorators: Vec::new(),
                    });
                }
            }
            "impl_item" => {
                // Extract methods from impl blocks
                if let Some(type_node) = child.child_by_field_name("type") {
                    let impl_name = node_text_owned(type_node, src);
                    if let Some(body) = child.child_by_field_name("body") {
                        let mut methods = Vec::new();
                        extract_rust_functions(body, src, &mut methods, Some(&impl_name), content);
                        // Find or create the class entry
                        if let Some(class) = classes.iter_mut().find(|c| c.name == impl_name) {
                            class.methods.extend(methods);
                        } else {
                            classes.push(ClassInfo {
                                name: impl_name,
                                start_line: child.start_position().row + 1,
                                end_line: child.end_position().row + 1,
                                docstring: None,
                                bases: Vec::new(),
                                methods,
                                decorators: Vec::new(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_rust_functions(
    node: Node,
    src: &[u8],
    functions: &mut Vec<FunctionInfo>,
    class_name: Option<&str>,
    _content: &str,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_item" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text_owned(name_node, src);
                let start_line = child.start_position().row + 1;
                let end_line = child.end_position().row + 1;
                let is_async = node_text(child, src).contains("async fn ");
                let docstring = extract_rust_doc_comment(child, src, node);

                // Extract parameters
                let mut parameters = Vec::new();
                if let Some(params) = child.child_by_field_name("parameters") {
                    let mut pcursor = params.walk();
                    for param in params.children(&mut pcursor) {
                        if param.kind() == "parameter" || param.kind() == "self_parameter" {
                            if let Some(pat) = param.child_by_field_name("pattern") {
                                parameters.push(node_text_owned(pat, src));
                            } else {
                                parameters.push(node_text_owned(param, src));
                            }
                        }
                    }
                }

                // Extract return type
                let return_type = child
                    .child_by_field_name("return_type")
                    .map(|n| node_text_owned(n, src));

                let is_method =
                    class_name.is_some() || parameters.iter().any(|p| p.contains("self"));

                functions.push(FunctionInfo {
                    name,
                    start_line,
                    end_line,
                    docstring,
                    parameters,
                    return_type,
                    is_async,
                    is_method,
                    class_name: class_name.map(|s| s.to_string()),
                    decorators: Vec::new(),
                    complexity: 1,
                });
            }
        }
    }
}

fn extract_rust_doc_comment(node: Node, src: &[u8], parent: Node) -> Option<String> {
    // Look at preceding siblings for /// comments
    let mut doc_lines = Vec::new();
    let node_start = node.start_position().row;

    let mut cursor = parent.walk();
    for child in parent.children(&mut cursor) {
        if child.end_position().row + 1 == node_start && child.kind() == "line_comment" {
            let text = node_text(child, src).trim();
            if text.starts_with("///") {
                doc_lines.push(text.trim_start_matches("///").trim().to_string());
            }
        }
        if child.id() == node.id() {
            break;
        }
        // Keep collecting consecutive doc comments
        if child.kind() == "line_comment" {
            let text = node_text(child, src).trim();
            if text.starts_with("///") {
                if child.end_position().row + 2 > node_start {
                    doc_lines.push(text.trim_start_matches("///").trim().to_string());
                }
            } else {
                doc_lines.clear();
            }
        } else {
            doc_lines.clear();
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

fn extract_rust_imports(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "use_declaration" {
            let text = node_text(child, src);
            // Simple parsing: extract module path from "use foo::bar::baz;"
            let trimmed = text.trim_start_matches("use ").trim_end_matches(';').trim();
            let module = trimmed.replace("::", ".");
            imports.push(ImportInfo {
                module: module.clone(),
                names: vec![module],
                alias: None,
                is_from: true,
                line: child.start_position().row + 1,
                level: 0,
            });
        }
    }
}

fn extract_rust_calls(node: Node, src: &[u8], calls: &mut Vec<CallInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call_expression" {
            if let Some(func) = child.child_by_field_name("function") {
                let full = node_text_owned(func, src);
                let (callee, qualifier) = if full.contains("::") {
                    let parts: Vec<&str> = full.rsplitn(2, "::").collect();
                    (parts[0].to_string(), Some(parts[1].to_string()))
                } else if full.contains('.') {
                    let parts: Vec<&str> = full.rsplitn(2, '.').collect();
                    (parts[0].to_string(), Some(parts[1].to_string()))
                } else {
                    (full, None)
                };
                calls.push(CallInfo {
                    callee_name: callee,
                    qualifier,
                    line: child.start_position().row + 1,
                    scope: None,
                    scope_type: None,
                });
            }
        }
        extract_rust_calls(child, src, calls);
    }
}

// ─── Go extraction ──────────────────────────────────────────────────────────

fn extract_go_structs(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>, _content: &str) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_declaration" {
            let mut tcursor = child.walk();
            for spec in child.children(&mut tcursor) {
                if spec.kind() == "type_spec" {
                    if let Some(name_node) = spec.child_by_field_name("name") {
                        let name = node_text_owned(name_node, src);
                        classes.push(ClassInfo {
                            name,
                            start_line: spec.start_position().row + 1,
                            end_line: spec.end_position().row + 1,
                            docstring: None,
                            bases: Vec::new(),
                            methods: Vec::new(),
                            decorators: Vec::new(),
                        });
                    }
                }
            }
        }
    }
}

fn extract_go_functions(node: Node, src: &[u8], functions: &mut Vec<FunctionInfo>, _content: &str) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declaration" || child.kind() == "method_declaration" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text_owned(name_node, src);
                let is_method = child.kind() == "method_declaration";

                let mut parameters = Vec::new();
                if let Some(params) = child.child_by_field_name("parameters") {
                    let mut pcursor = params.walk();
                    for param in params.children(&mut pcursor) {
                        if param.kind() == "parameter_declaration" {
                            parameters.push(node_text_owned(param, src));
                        }
                    }
                }

                functions.push(FunctionInfo {
                    name,
                    start_line: child.start_position().row + 1,
                    end_line: child.end_position().row + 1,
                    docstring: None,
                    parameters,
                    return_type: child
                        .child_by_field_name("result")
                        .map(|n| node_text_owned(n, src)),
                    is_async: false,
                    is_method,
                    class_name: None,
                    decorators: Vec::new(),
                    complexity: 1,
                });
            }
        }
    }
}

fn extract_go_imports(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_declaration" {
            let mut icursor = child.walk();
            for spec in child.children(&mut icursor) {
                if spec.kind() == "import_spec" || spec.kind() == "import_spec_list" {
                    let text = node_text(spec, src)
                        .trim_matches('"')
                        .trim_matches('(')
                        .trim_matches(')')
                        .trim()
                        .to_string();
                    if !text.is_empty() {
                        imports.push(ImportInfo {
                            module: text.clone(),
                            names: vec![text],
                            alias: None,
                            is_from: true,
                            line: spec.start_position().row + 1,
                            level: 0,
                        });
                    }
                }
            }
        }
    }
}

// ─── Java extraction ────────────────────────────────────────────────────────

fn extract_java_classes(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>, content: &str) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_declaration" || child.kind() == "interface_declaration" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text_owned(name_node, src);

                // Extract superclass/interfaces
                let mut bases = Vec::new();
                if let Some(extends) = child.child_by_field_name("superclass") {
                    bases.push(node_text_owned(extends, src));
                }

                let mut methods = Vec::new();
                if let Some(body) = child.child_by_field_name("body") {
                    extract_java_functions(body, src, &mut methods, Some(&name), content);
                }

                classes.push(ClassInfo {
                    name,
                    start_line: child.start_position().row + 1,
                    end_line: child.end_position().row + 1,
                    docstring: None,
                    bases,
                    methods,
                    decorators: Vec::new(),
                });
            }
        }
    }
}

fn extract_java_functions(
    node: Node,
    src: &[u8],
    functions: &mut Vec<FunctionInfo>,
    class_name: Option<&str>,
    _content: &str,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "method_declaration" || child.kind() == "constructor_declaration" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text_owned(name_node, src);
                let mut parameters = Vec::new();
                if let Some(params) = child.child_by_field_name("parameters") {
                    let mut pcursor = params.walk();
                    for param in params.children(&mut pcursor) {
                        if param.kind() == "formal_parameter" {
                            if let Some(n) = param.child_by_field_name("name") {
                                parameters.push(node_text_owned(n, src));
                            }
                        }
                    }
                }

                functions.push(FunctionInfo {
                    name,
                    start_line: child.start_position().row + 1,
                    end_line: child.end_position().row + 1,
                    docstring: None,
                    parameters,
                    return_type: child
                        .child_by_field_name("type")
                        .map(|n| node_text_owned(n, src)),
                    is_async: false,
                    is_method: true,
                    class_name: class_name.map(|s| s.to_string()),
                    decorators: Vec::new(),
                    complexity: 1,
                });
            }
        }
    }
}

fn extract_java_imports(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_declaration" {
            let text = node_text(child, src);
            let module = text
                .trim_start_matches("import ")
                .trim_start_matches("static ")
                .trim_end_matches(';')
                .trim()
                .to_string();
            imports.push(ImportInfo {
                module: module.clone(),
                names: vec![module],
                alias: None,
                is_from: true,
                line: child.start_position().row + 1,
                level: 0,
            });
        }
    }
}

// ─── C/C++ extraction ───────────────────────────────────────────────────────

fn extract_c_structs(node: Node, src: &[u8], classes: &mut Vec<ClassInfo>, _content: &str) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "struct_specifier" || child.kind() == "class_specifier" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text_owned(name_node, src);
                classes.push(ClassInfo {
                    name,
                    start_line: child.start_position().row + 1,
                    end_line: child.end_position().row + 1,
                    docstring: None,
                    bases: Vec::new(),
                    methods: Vec::new(),
                    decorators: Vec::new(),
                });
            }
        }
    }
}

fn extract_c_functions(node: Node, src: &[u8], functions: &mut Vec<FunctionInfo>, _content: &str) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(declarator) = child.child_by_field_name("declarator") {
                let name = find_identifier_in_declarator(declarator, src);
                if let Some(name) = name {
                    let mut parameters = Vec::new();
                    // Find parameter_list in declarator
                    if let Some(params) = find_node_by_kind(declarator, "parameter_list") {
                        let mut pcursor = params.walk();
                        for param in params.children(&mut pcursor) {
                            if param.kind() == "parameter_declaration" {
                                if let Some(decl) = param.child_by_field_name("declarator") {
                                    parameters.push(node_text_owned(decl, src));
                                }
                            }
                        }
                    }

                    functions.push(FunctionInfo {
                        name,
                        start_line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        docstring: None,
                        parameters,
                        return_type: child
                            .child_by_field_name("type")
                            .map(|n| node_text_owned(n, src)),
                        is_async: false,
                        is_method: false,
                        class_name: None,
                        decorators: Vec::new(),
                        complexity: 1,
                    });
                }
            }
        }
    }
}

fn find_identifier_in_declarator(node: Node, src: &[u8]) -> Option<String> {
    if node.kind() == "identifier" || node.kind() == "field_identifier" {
        return Some(node_text_owned(node, src));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(name) = find_identifier_in_declarator(child, src) {
            return Some(name);
        }
    }
    None
}

fn find_node_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_node_by_kind(child, kind) {
            return Some(found);
        }
    }
    None
}

fn extract_c_includes(node: Node, src: &[u8], imports: &mut Vec<ImportInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "preproc_include" {
            let text = node_text(child, src);
            let module = text
                .trim_start_matches("#include")
                .trim()
                .trim_matches(|c| c == '"' || c == '<' || c == '>')
                .to_string();
            imports.push(ImportInfo {
                module: module.clone(),
                names: vec![module],
                alias: None,
                is_from: true,
                line: child.start_position().row + 1,
                level: 0,
            });
        }
    }
}

// ─── Utility ────────────────────────────────────────────────────────────────

fn clean_docstring(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'')
        .trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_start_matches("'''")
        .trim_end_matches("'''")
        .trim_start_matches("/**")
        .trim_end_matches("*/")
        .trim_start_matches("/*!")
        .trim_end_matches("*/")
        .trim_start_matches("/*")
        .trim_end_matches("*/")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python_file() {
        let code = r#"
"""Module docstring"""

import os
from typing import List

class MyClass(Base):
    """A class."""
    def __init__(self, x):
        self.x = x

    def method(self) -> int:
        return self.x

def top_level(a, b):
    """A function."""
    return a + b
"#;
        let result = parse_file("/test/main.py", code, "python", "/test").unwrap();
        assert_eq!(result.language, "python");
        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].name, "MyClass");
        assert_eq!(result.classes[0].bases, vec!["Base"]);
        assert_eq!(result.classes[0].methods.len(), 2);
        assert!(!result.functions.is_empty());
        assert!(result.imports.len() >= 2);
    }

    #[test]
    fn test_parse_typescript_file() {
        let code = r#"
import { useState } from 'react';
import axios from 'axios';

class UserService {
    async getUser(id: string) {
        return axios.get(`/users/${id}`);
    }
}

const handler = (req, res) => {
    res.send('ok');
};

export function main() {
    console.log('hello');
}
"#;
        let result = parse_file("/test/app.ts", code, "typescript", "/test").unwrap();
        assert_eq!(result.language, "typescript");
        assert!(result.imports.len() >= 2);
        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].name, "UserService");
    }

    #[test]
    fn test_parse_rust_file() {
        let code = r#"
//! Module documentation

use std::collections::HashMap;

/// A point in 2D space.
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

fn main() {
    let p = Point::new(1.0, 2.0);
}
"#;
        let result = parse_file("/test/main.rs", code, "rust", "/test").unwrap();
        assert_eq!(result.language, "rust");
        assert!(!result.imports.is_empty());
        assert!(!result.classes.is_empty());
    }
}
