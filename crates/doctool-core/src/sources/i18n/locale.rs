use std::path::{Path, PathBuf};

use glob::glob;
use walkdir::WalkDir;

/// Detect locale from an MDX relative path (lomi suffix convention).
pub fn detect_locale_from_path(relative: &str) -> &'static str {
    if relative.ends_with(".fr.mdx") {
        "fr"
    } else if relative.ends_with(".es.mdx") {
        "es"
    } else if relative.ends_with(".zh.mdx") {
        "zh"
    } else {
        "en"
    }
}

/// Strip locale suffix and `.mdx` to get the canonical slug path segment.
pub fn slug_without_locale(relative: &str) -> String {
    let without_ext = relative.trim_end_matches(".mdx");
    without_ext
        .trim_end_matches(".fr")
        .trim_end_matches(".es")
        .trim_end_matches(".zh")
        .trim_end_matches("/index")
        .to_string()
}

/// `build/usage-billing.mdx` → `build/usage-billing.fr.mdx`
pub fn locale_sibling_path(en_relative: &str, target_locale: &str) -> String {
    if target_locale == "en" {
        return en_relative.to_string();
    }
    en_relative.replace(".mdx", &format!(".{target_locale}.mdx"))
}

/// `build/usage-billing.fr.mdx` → `build/usage-billing.mdx`
pub fn paired_source_path(locale_relative: &str, source_locale: &str) -> Option<String> {
    if detect_locale_from_path(locale_relative) == source_locale {
        return Some(locale_relative.to_string());
    }
    let locale = detect_locale_from_path(locale_relative);
    if locale == "en" {
        return None;
    }
    Some(locale_relative.replace(&format!(".{locale}.mdx"), ".mdx"))
}

/// Collect EN (source) MDX paths under `content_root` matching include/exclude globs.
/// Returns paths relative to `docs_content` (e.g. `build/guides/foo.mdx`).
pub fn source_pages_matching(
    monorepo_root: &Path,
    docs_content_relative: &str,
    include_globs: &[String],
    exclude_globs: &[String],
) -> Vec<String> {
    let docs_root = monorepo_root.join(docs_content_relative);
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let relativize = |path: &Path| -> Option<String> {
        path.strip_prefix(&docs_root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
    };

    for pattern in include_globs {
        let full_pattern = monorepo_root.join(pattern);
        let pattern_str = full_pattern.to_string_lossy().to_string();
        if let Ok(entries) = glob(&pattern_str) {
            for entry in entries.flatten() {
                if !entry.is_file() {
                    continue;
                }
                let Some(relative) = relativize(&entry) else {
                    continue;
                };
                if detect_locale_from_path(&relative) != "en" {
                    continue;
                }
                if exclude_globs.iter().any(|ex| {
                    let ex_full = monorepo_root.join(ex);
                    glob::Pattern::new(&ex_full.to_string_lossy())
                        .ok()
                        .is_some_and(|p| p.matches_path(&entry))
                }) {
                    continue;
                }
                if seen.insert(relative.clone()) {
                    paths.push(relative);
                }
            }
        }
    }

    if paths.is_empty() && docs_root.is_dir() {
        for entry in WalkDir::new(&docs_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("mdx") {
                continue;
            }
            let Some(relative) = relativize(path) else {
                continue;
            };
            if detect_locale_from_path(&relative) != "en" {
                continue;
            }
            if seen.insert(relative.clone()) {
                paths.push(relative);
            }
        }
    }

    paths.sort();
    paths
}

pub fn resolve_docs_relative(monorepo_root: &Path, docs_content_relative: &str, relative: &str) -> PathBuf {
    monorepo_root.join(docs_content_relative).join(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_sibling_fr_suffix() {
        assert_eq!(
            locale_sibling_path("build/usage-billing.mdx", "fr"),
            "build/usage-billing.fr.mdx"
        );
        assert_eq!(
            locale_sibling_path("api/foo/Bar.mdx", "fr"),
            "api/foo/Bar.fr.mdx"
        );
    }

    #[test]
    fn paired_source_inverts_sibling() {
        assert_eq!(
            paired_source_path("build/usage-billing.fr.mdx", "en").as_deref(),
            Some("build/usage-billing.mdx")
        );
    }

    #[test]
    fn detect_locale_suffixes() {
        assert_eq!(detect_locale_from_path("a/b.mdx"), "en");
        assert_eq!(detect_locale_from_path("a/b.fr.mdx"), "fr");
    }
}
