use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::config::DoctoolConfig;
use crate::drift::{build_next_steps, DriftIssue, DriftReport};
use crate::drift::categories::DriftCategory;
use crate::i18n::LockFileManager;
use crate::sources::i18n::{
    detect_locale_from_path, locale_sibling_path, paired_source_path, source_pages_matching,
};
use crate::sources::mdx::document::MdxDocument;

pub struct SyncI18nOptions {
    pub check_only: bool,
    pub dry_run: bool,
    pub scaffold_missing: bool,
    pub refresh_lock: bool,
}

pub struct SyncI18nReport {
    pub drift: DriftReport,
    pub scaffolded: Vec<String>,
    pub lock_updated: bool,
}

pub fn run_sync_i18n(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    options: &SyncI18nOptions,
) -> Result<SyncI18nReport> {
    let i18n = config.i18n_config();
    let docs_content = config.resolve(monorepo_root, &config.docs_content);

    let en_pages = source_pages_matching(
        monorepo_root,
        &config.docs_content,
        &i18n.mdx.include,
        &i18n.mdx.exclude,
    );

    let mut lock_mgr = LockFileManager::load(monorepo_root, &i18n.lock_cache)?;
    let mut issues = Vec::new();
    let mut scaffolded = Vec::new();
    let mut lock_entries: Vec<(String, std::collections::HashMap<String, String>)> = Vec::new();

    // Orphan FR files (no EN sibling)
    if docs_content.is_dir() {
        for entry in walkdir::WalkDir::new(&docs_content)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("mdx") {
                continue;
            }
            let relative = path
                .strip_prefix(&docs_content)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let locale = detect_locale_from_path(&relative);
            if locale == i18n.source.as_str() {
                continue;
            }
            if !i18n.targets.iter().any(|t| locale == t.as_str()) {
                continue;
            }
            let en_path = paired_source_path(&relative, &i18n.source);
            let en_exists = en_path
                .as_ref()
                .map(|p| docs_content.join(p).is_file())
                .unwrap_or(false);
            if !en_exists {
                issues.push(DriftIssue {
                    category: DriftCategory::LocaleOrphan.as_str().to_string(),
                    message: format!("Orphan {locale} page with no EN source: {relative}"),
                    file: Some(relative),
                });
            }
        }
    }

    for en_relative in &en_pages {
        let en_full = docs_content.join(en_relative);
        let en_raw = fs::read_to_string(&en_full)?;
        let en_doc = MdxDocument::parse(en_relative, &en_raw);
        let en_segments = en_doc.segment_values();
        lock_entries.push((en_relative.clone(), en_segments.clone()));

        for target in &i18n.targets {
            let fr_relative = locale_sibling_path(en_relative, target);
            let fr_full = docs_content.join(&fr_relative);

            if !fr_full.is_file() {
                issues.push(DriftIssue {
                    category: DriftCategory::LocaleGap.as_str().to_string(),
                    message: format!("Missing {target} sibling for {en_relative}"),
                    file: Some(en_relative.clone()),
                });

                if options.scaffold_missing && !options.check_only && !options.dry_run {
                    let stub = scaffold_fr_stub(&en_doc, target);
                    if let Some(parent) = fr_full.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&fr_full, stub)?;
                    scaffolded.push(fr_relative);
                }
                continue;
            }

            if lock_mgr.exists()
                && lock_mgr.has_file(en_relative)
                && lock_mgr.has_stale_segments(en_relative, &en_segments)
            {
                issues.push(DriftIssue {
                    category: DriftCategory::LocaleStale.as_str().to_string(),
                    message: format!("EN segments changed since lock for {en_relative}"),
                    file: Some(en_relative.clone()),
                });
            }

            let fr_raw = fs::read_to_string(&fr_full)?;
            let fr_doc = MdxDocument::parse(&fr_relative, &fr_raw);

            if en_doc.heading_count() != fr_doc.heading_count() {
                issues.push(DriftIssue {
                    category: DriftCategory::LocaleStructure.as_str().to_string(),
                    message: format!(
                        "Heading count mismatch EN={} FR={} for {}",
                        en_doc.heading_count(),
                        fr_doc.heading_count(),
                        en_relative
                    ),
                    file: Some(en_relative.clone()),
                });
            }

            let en_fm_keys: HashSet<_> = translatable_frontmatter_keys(&en_doc).collect();
            let fr_fm_keys: HashSet<_> = translatable_frontmatter_keys(&fr_doc).collect();
            if en_fm_keys != fr_fm_keys {
                issues.push(DriftIssue {
                    category: DriftCategory::LocaleStructure.as_str().to_string(),
                    message: format!("Frontmatter key mismatch for {en_relative}"),
                    file: Some(en_relative.clone()),
                });
            }

            let en_links: HashSet<_> = en_doc.internal_links().into_iter().collect();
            let fr_links: HashSet<_> = fr_doc.internal_links().into_iter().collect();
            if en_links != fr_links {
                issues.push(DriftIssue {
                    category: DriftCategory::LocaleStructure.as_str().to_string(),
                    message: format!("Internal link set mismatch for {en_relative}"),
                    file: Some(en_relative.clone()),
                });
            }
        }
    }

    let issue_count = issues.len();
    let next_steps = build_i18n_next_steps(&issues);
    let drift = DriftReport {
        issues,
        issue_count,
        next_steps,
    };

    let mut lock_updated = false;
    if options.refresh_lock && !options.check_only {
        lock_mgr.refresh_from_corpus(lock_entries)?;
        lock_updated = true;
    }

    Ok(SyncI18nReport {
        drift,
        scaffolded,
        lock_updated,
    })
}

fn translatable_frontmatter_keys(doc: &MdxDocument) -> impl Iterator<Item = &String> {
    doc.frontmatter
        .keys()
        .filter(|k| *k == "title" || *k == "description")
}

fn scaffold_fr_stub(en_doc: &MdxDocument, target: &str) -> String {
    let mut fm = en_doc.frontmatter.clone();
    if let Some(title) = fm.get_mut("title") {
        *title = format!("[TODO] {title}");
    }
    if let Some(desc) = fm.get_mut("description") {
        *desc = format!("[TODO] {desc}");
    }

    let mut out = String::from("---\n");
    let mut keys: Vec<_> = fm.keys().collect();
    keys.sort();
    for key in keys {
        let value = &fm[key];
        out.push_str(key);
        out.push_str(": ");
        if value.contains(':') || value.contains('"') {
            out.push('"');
            out.push_str(value);
            out.push('"');
        } else {
            out.push_str(value);
        }
        out.push('\n');
    }
    out.push_str("---\n\n");
    out.push_str("{/* TODO: translate from EN */}\n\n");

    for block in &en_doc.body_blocks {
        match block.kind {
            crate::sources::mdx::document::BodyBlockKind::Prose => {
                out.push_str("<!-- TODO: translate -->\n\n");
            }
            crate::sources::mdx::document::BodyBlockKind::Heading => {
                out.push_str(&block.content);
                out.push_str("\n\n");
            }
            _ => {
                out.push_str(&block.content);
                if !block.content.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
        }
    }

    let _ = target;
    out
}

fn build_i18n_next_steps(issues: &[DriftIssue]) -> Vec<String> {
    let mut steps = build_next_steps(issues);
    let cats: HashSet<_> = issues.iter().map(|i| i.category.as_str()).collect();
    if cats.contains("locale_structure") {
        steps.push("Manually align FR structure with EN source".into());
    }
    if cats.contains("locale_orphan") {
        steps.push("Remove orphan FR pages or restore EN source".into());
    }
    steps.sort();
    steps.dedup();
    steps
}
