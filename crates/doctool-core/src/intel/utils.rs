//! Utility functions for the code intelligence system.

use std::path::Path;

/// Map file extension to programming language name.
pub fn get_language_from_extension(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        "py" | "pyw" | "pyi" => Some("python"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "ts" | "mts" | "cts" => Some("typescript"),
        "tsx" => Some("tsx"),
        "jsx" => Some("jsx"),
        "rs" => Some("rust"),
        "go" => Some("go"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some("cpp"),
        "cs" => Some("csharp"),
        "rb" => Some("ruby"),
        "php" => Some("php"),
        "swift" => Some("swift"),
        "kt" | "kts" => Some("kotlin"),
        "scala" => Some("scala"),
        "r" => Some("r"),
        "lua" => Some("lua"),
        "sh" | "bash" | "zsh" => Some("shell"),
        "sql" => Some("sql"),
        "md" | "markdown" => Some("markdown"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        "xml" => Some("xml"),
        "html" | "htm" => Some("html"),
        "css" => Some("css"),
        "scss" | "sass" => Some("scss"),
        "vue" => Some("vue"),
        "svelte" => Some("svelte"),
        "dart" => Some("dart"),
        "ex" | "exs" => Some("elixir"),
        "erl" | "hrl" => Some("erlang"),
        "hs" => Some("haskell"),
        "ml" | "mli" => Some("ocaml"),
        "tf" | "tfvars" => Some("terraform"),
        "proto" => Some("protobuf"),
        "graphql" | "gql" => Some("graphql"),
        _ => None,
    }
}

/// Supported extensions that tree-sitter can parse for code intelligence.
pub fn is_parseable_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "py" | "pyw"
            | "pyi"
            | "js"
            | "mjs"
            | "cjs"
            | "jsx"
            | "ts"
            | "mts"
            | "cts"
            | "tsx"
            | "rs"
            | "go"
            | "java"
            | "c"
            | "h"
            | "cpp"
            | "cc"
            | "cxx"
            | "hpp"
            | "hxx"
            | "hh"
    )
}

/// Convert a file path to a dotted module path (for Python-like imports).
/// e.g., "/repo/app/services/auth.py" with root "/repo" → "app.services.auth"
pub fn file_path_to_module_path(file_path: &str, repo_root: &str) -> Option<String> {
    let file = Path::new(file_path);
    let root = Path::new(repo_root);

    let relative = file.strip_prefix(root).ok()?;
    let relative_str = relative.to_str()?;

    // Remove file extension
    let without_ext = if let Some(stem) = relative.file_stem() {
        let parent = relative.parent().unwrap_or(Path::new(""));
        if parent.as_os_str().is_empty() {
            stem.to_str()?.to_string()
        } else {
            format!("{}/{}", parent.to_str()?, stem.to_str()?)
        }
    } else {
        relative_str.to_string()
    };

    // Convert path separators to dots
    let module_path = without_ext.replace(['/', '\\'], ".");

    // Remove trailing __init__
    let module_path = module_path
        .strip_suffix(".__init__")
        .unwrap_or(&module_path)
        .to_string();

    if module_path.is_empty() {
        None
    } else {
        Some(module_path)
    }
}

/// Compute MD5 hash for deterministic ID generation.
pub fn compute_hash(parts: &[&str]) -> String {
    let combined = parts.join("::");
    format!("{:x}", md5::compute(combined.as_bytes()))
}

/// Get the file extension (lowercase, without dot).
pub fn get_extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(get_language_from_extension("py"), Some("python"));
        assert_eq!(get_language_from_extension("ts"), Some("typescript"));
        assert_eq!(get_language_from_extension("rs"), Some("rust"));
        assert_eq!(get_language_from_extension("xyz"), None);
    }

    #[test]
    fn test_module_path() {
        assert_eq!(
            file_path_to_module_path("/repo/app/services/auth.py", "/repo"),
            Some("app.services.auth".to_string())
        );
        assert_eq!(
            file_path_to_module_path("/repo/app/__init__.py", "/repo"),
            Some("app".to_string())
        );
    }
}
