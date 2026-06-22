use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy, Default)]
pub enum DiffFormat {
    #[default]
    Unified,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    pub path: String,
    pub format: String,
    pub patch: String,
    pub changed: bool,
}

/// Produce a unified diff between `original` and `proposed`.
pub fn diff_text(
    path: &str,
    original: &str,
    proposed: &str,
    format: DiffFormat,
) -> DiffReport {
    let patch = match format {
        DiffFormat::Unified => unified_diff(path, original, proposed),
    };
    let changed = original != proposed;
    DiffReport {
        path: path.to_string(),
        format: "unified".into(),
        patch,
        changed,
    }
}

fn unified_diff(path: &str, original: &str, proposed: &str) -> String {
    let diff = TextDiff::from_lines(original, proposed);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        out.push_str(sign);
        out.push_str(change.value());
        if !change.value().ends_with('\n') {
            out.push('\n');
        }
    }
    if out.is_empty() && original != proposed {
        format!("--- a/{path}\n+++ b/{path}\n")
    } else if !out.is_empty() {
        format!("--- a/{path}\n+++ b/{path}\n{out}")
    } else {
        String::new()
    }
}

/// Compare on-disk canonical MDX with optional proposed file content.
pub fn run_diff(
    docs_content: &Path,
    path: &str,
    proposed_path: Option<&Path>,
    format: DiffFormat,
) -> Result<DiffReport> {
    let page_path = path.trim_start_matches('/');
    let canonical_path = docs_content.join(page_path);
    let original = fs::read_to_string(&canonical_path)
        .with_context(|| format!("read canonical MDX {}", canonical_path.display()))?;

    let proposed = if let Some(prop) = proposed_path {
        fs::read_to_string(prop).with_context(|| format!("read proposed MDX {}", prop.display()))?
    } else {
        original.clone()
    };

    Ok(diff_text(page_path, &original, &proposed, format))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_line_change() {
        let report = diff_text("a.mdx", "hello\n", "hello world\n", DiffFormat::Unified);
        assert!(report.changed);
        assert!(report.patch.contains('+'));
    }
}
