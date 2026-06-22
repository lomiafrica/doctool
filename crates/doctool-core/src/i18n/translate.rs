use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::config::DoctoolConfig;
use crate::i18n::lock::LockFileManager;
use crate::llm::{
    calculate_chunk_size, translate_segments_batched, LlmClient, TranslatePromptOptions,
};
use crate::provenance::GitProvenance;
use crate::sources::i18n::{locale_sibling_path, source_pages_matching};
use crate::sources::mdx::document::MdxDocument;
use crate::i18n::sync::scaffold_fr_stub;

#[derive(Debug, Clone, Default)]
pub struct TranslateI18nOptions {
    pub check_only: bool,
    pub dry_run: bool,
    pub force: bool,
    pub refresh_lock: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranslatePageResult {
    pub en_path: String,
    pub target_path: String,
    pub segments_translated: usize,
    pub written: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranslateI18nReport {
    pub pages: Vec<TranslatePageResult>,
    pub pending_segments: usize,
    pub provenance: GitProvenance,
    pub lock_updated: bool,
}

pub async fn run_translate_i18n(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    options: &TranslateI18nOptions,
) -> Result<TranslateI18nReport> {
    let i18n = config.i18n_config();
    let docs_content = config.resolve(monorepo_root, &config.docs_content);
    let llm_config = config.llm_config();

    let en_pages = source_pages_matching(
        monorepo_root,
        &config.docs_content,
        &i18n.mdx.include,
        &i18n.mdx.exclude,
    );

    let mut lock_mgr = LockFileManager::load(monorepo_root, &i18n.lock_cache)?;
    let provider = LlmClient::from_resolved(llm_config)?;
    let provenance = crate::provenance::collect_git_provenance(monorepo_root);

    let mut pages = Vec::new();
    let mut pending_segments = 0usize;
    let mut lock_entries: Vec<(String, HashMap<String, String>)> = Vec::new();

    for en_relative in &en_pages {
        let en_full = docs_content.join(en_relative);
        let en_raw = fs::read_to_string(&en_full)?;
        let en_doc = MdxDocument::parse(en_relative, &en_raw);
        let en_segments = en_doc.segment_values();
        lock_entries.push((en_relative.clone(), en_segments.clone()));

        for target in &i18n.targets {
            let fr_relative = locale_sibling_path(en_relative, target);
            let fr_full = docs_content.join(&fr_relative);

            let keys_to_translate: Vec<String> = if options.force {
                en_doc.translatable_segment_keys()
            } else {
                let changes = lock_mgr.get_changes(en_relative, &en_segments);
                changes
                    .added_keys
                    .into_iter()
                    .chain(changes.changed_keys)
                    .filter(|k| en_doc.is_translatable_segment_key(k))
                    .collect()
            };

            if keys_to_translate.is_empty() {
                continue;
            }

            pending_segments += keys_to_translate.len();

            if options.check_only {
                pages.push(TranslatePageResult {
                    en_path: en_relative.clone(),
                    target_path: fr_relative.clone(),
                    segments_translated: keys_to_translate.len(),
                    written: false,
                });
                continue;
            }

            if options.dry_run {
                pages.push(TranslatePageResult {
                    en_path: en_relative.clone(),
                    target_path: fr_relative.clone(),
                    segments_translated: keys_to_translate.len(),
                    written: false,
                });
                continue;
            }

            let batch: Vec<(String, String)> = keys_to_translate
                .iter()
                .filter_map(|k| en_segments.get(k).map(|v| (k.clone(), v.clone())))
                .collect();

            let prompt_opts = TranslatePromptOptions {
                source_locale: i18n.source.clone(),
                target_locale: target.clone(),
            };
            let chunk_size = calculate_chunk_size(&batch, Some(&prompt_opts));
            let translated =
                translate_segments_batched(provider.as_ref(), &batch, &prompt_opts, chunk_size)
                    .await?;

            let mut updates: HashMap<String, String> = translated
                .into_iter()
                .map(|t| (t.key, t.translated_text))
                .collect();

            let mut fr_doc = if fr_full.is_file() {
                let fr_raw = fs::read_to_string(&fr_full)?;
                MdxDocument::parse(&fr_relative, &fr_raw)
            } else {
                let stub = scaffold_fr_stub(&en_doc, target);
                MdxDocument::parse(&fr_relative, &stub)
            };

            fr_doc.apply_segment_updates(&updates, Some(&en_doc));

            if let Some(parent) = fr_full.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&fr_full, fr_doc.serialize())?;

            pages.push(TranslatePageResult {
                en_path: en_relative.clone(),
                target_path: fr_relative.clone(),
                segments_translated: updates.len(),
                written: true,
            });

            let _ = &mut updates;
        }
    }

    let mut lock_updated = false;
    if options.refresh_lock && !options.check_only && !options.dry_run {
        lock_mgr.refresh_from_corpus(lock_entries)?;
        lock_updated = true;
    }

    Ok(TranslateI18nReport {
        pages,
        pending_segments,
        provenance,
        lock_updated,
    })
}
