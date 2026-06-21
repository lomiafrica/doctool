//! File scanner/loader for the code intelligence system.

use std::path::Path;
use walkdir::WalkDir;

use super::types::FileEntry;
use super::utils::{get_extension, get_language_from_extension};

/// Scan a directory and collect metadata for all supported source files.
pub fn scan_directory(root: &str) -> Vec<FileEntry> {
    let root_path = Path::new(root);
    let mut entries = Vec::new();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip ignored directories
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                // Allow the root itself
                if e.depth() == 0 {
                    return true;
                }
                // Skip hidden and ignored dirs
                if name.starts_with('.') || is_ignored_dir(name) {
                    return false;
                }
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = get_extension(path.to_str().unwrap_or(""));

        // Skip files without a recognized language
        let language = match get_language_from_extension(&ext) {
            Some(lang) => lang.to_string(),
            None => continue,
        };

        let abs_path = path.to_str().unwrap_or("").to_string();
        let relative_path = path
            .strip_prefix(root_path)
            .unwrap_or(path)
            .to_str()
            .unwrap_or("")
            .to_string();

        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);

        entries.push(FileEntry {
            path: abs_path,
            relative_path,
            language,
            extension: ext,
            size_bytes,
        });
    }

    entries
}

/// Read file content with UTF-8 handling (skip binary files).
pub fn read_file_content(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | "target"
            | "build"
            | "dist"
            | ".next"
            | ".nuxt"
            | ".output"
            | "out"
            | ".turbo"
            | ".cache"
            | ".venv"
            | "venv"
            | "env"
            | ".tox"
            | "vendor"
            | "Pods"
            | ".gradle"
            | ".idea"
            | ".vscode"
            | ".vs"
            | "coverage"
            | ".nyc_output"
            | ".composer"
            | ".git"
            | ".svn"
            | ".hg"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_directory() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create test files
        fs::write(root.join("main.py"), "print('hello')").unwrap();
        fs::write(root.join("app.ts"), "console.log('hi')").unwrap();
        fs::write(root.join("readme.txt"), "just text").unwrap();

        // Create ignored directory
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/pkg.js"), "module").unwrap();

        let entries = scan_directory(root.to_str().unwrap());

        // Should find main.py and app.ts, not readme.txt or node_modules/pkg.js
        assert_eq!(entries.len(), 2);
        let names: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
        assert!(names.contains(&"main.py"));
        assert!(names.contains(&"app.ts"));
    }
}
