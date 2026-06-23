//! Merge deterministic drift findings with code RAG + LLM recommendations.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::config::DoctoolConfig;
use crate::context::{build_code_index, format_code_context, queries_for_drift_issues};
use crate::drift::{build_drift_report, build_next_steps, DriftIssue, DriftReport};
use crate::i18n::{run_sync_i18n, SyncI18nOptions};
use crate::llm::{suggest_plan, SuggestLlmInput};

#[derive(Debug, Clone, Default)]
pub struct SuggestOptions {
    pub skip_ts: bool,
    pub skip_ai: bool,
    pub include_i18n: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestAction {
    pub source: String,
    pub priority: u8,
    pub command: Option<String>,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestReport {
    pub blocking_issue_count: usize,
    pub i18n_warning_count: usize,
    pub i18n_by_category: BTreeMap<String, usize>,
    /// `locale_structure` issues under `api/` (heading EN ≠ FR).
    pub i18n_api_structure_count: usize,
    /// Deprecated alias — same as `blocking_issue_count`.
    pub drift_issue_count: usize,
    /// Deprecated alias — same as `i18n_warning_count`.
    pub i18n_issue_count: usize,
    pub actions: Vec<SuggestAction>,
    pub executable_commands: Vec<String>,
    pub ai_summary: Option<String>,
    pub code_context_chars: usize,
}

pub async fn run_suggest(
    config: &DoctoolConfig,
    monorepo_root: &Path,
    options: &SuggestOptions,
) -> Result<SuggestReport> {
    let code_index = build_code_index(config, monorepo_root, false).await?;

    let mut drift = build_drift_report(config, monorepo_root)?;

    if !options.skip_ts {
        if let Ok((ok, output)) = run_ts_drift_capture(monorepo_root) {
            if !ok {
                drift
                    .issues
                    .extend(crate::drift::merge_ts_errors(&output));
                drift.issue_count = drift.issues.len();
            }
        }
    }

    let mut i18n_issues = Vec::new();
    if options.include_i18n {
        let i18n_report = run_sync_i18n(
            config,
            monorepo_root,
            &SyncI18nOptions {
                check_only: true,
                dry_run: false,
                scaffold_missing: false,
                refresh_lock: false,
            },
        )?;
        i18n_issues = i18n_report.drift.issues;
    }

    let all_issues: Vec<DriftIssue> = drift
        .issues
        .iter()
        .chain(i18n_issues.iter())
        .cloned()
        .collect();

    let mut actions = deterministic_actions(&drift, &i18n_issues);
    let executable_commands: Vec<String> = build_next_steps(&all_issues);

    let queries = queries_for_drift_issues(&all_issues);
    let code_context = format_code_context(&code_index, &queries, 15).await;
    let code_context_chars = code_context.len();

    let ai_summary = if options.skip_ai {
        None
    } else {
        let llm_input = SuggestLlmInput {
            drift_summary: summarize_issues(&all_issues),
            next_steps: executable_commands.clone(),
            code_context: code_context.clone(),
            knowledge_graph_stats: graph_stats(config, monorepo_root),
        };
        let plan = suggest_plan(&config.llm_config(), &llm_input).await?;
        for rec in plan.recommendations {
            if actions.iter().any(|a| a.title == rec.title) {
                continue;
            }
            actions.push(SuggestAction {
                source: "ai".into(),
                priority: rec.priority,
                command: rec.command,
                title: rec.title,
                detail: rec.rationale,
            });
        }
        Some(plan.summary)
    };

    actions.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.title.cmp(&b.title)));

    let i18n_by_category = summarize_i18n_categories(&i18n_issues);
    let i18n_api_structure_count = count_api_structure_issues(&i18n_issues);

    Ok(SuggestReport {
        blocking_issue_count: drift.issue_count,
        i18n_warning_count: i18n_issues.len(),
        i18n_by_category,
        i18n_api_structure_count,
        drift_issue_count: drift.issue_count,
        i18n_issue_count: i18n_issues.len(),
        actions,
        executable_commands,
        ai_summary,
        code_context_chars,
    })
}

fn deterministic_actions(drift: &DriftReport, i18n_issues: &[DriftIssue]) -> Vec<SuggestAction> {
    let mut actions = Vec::new();

    for issue in drift.issues.iter().take(30) {
        let (priority, command) = match issue.category.as_str() {
            "missing_endpoint" => (1, Some("dt scaffold".into())),
            "orphan_doc" => (2, None),
            "guide_dead_link" => (2, None),
            "sdk_unmentioned" => (3, None),
            "ts_drift" if issue.message.contains("pnpm run generate") => {
                (1, Some("cd apps/mcp && pnpm run generate".into()))
            }
            "ts_drift" => (2, Some("cd apps/docs && pnpm docs:drift".into())),
            _ => (4, None),
        };

        actions.push(SuggestAction {
            source: "drift".into(),
            priority,
            command,
            title: issue.category.clone(),
            detail: issue.message.clone(),
        });
    }

    if !i18n_issues.is_empty() {
        let structure_count = i18n_issues
            .iter()
            .filter(|i| i.category == "locale_structure")
            .count();
        let gap_count = i18n_issues
            .iter()
            .filter(|i| i.category == "locale_gap")
            .count();

        if gap_count > 0 {
            actions.push(SuggestAction {
                source: "i18n".into(),
                priority: 2,
                command: Some("dt sync-i18n --scaffold-missing".into()),
                title: "locale_gap".into(),
                detail: format!("{gap_count} EN pages missing .fr.mdx siblings"),
            });
        }

        if structure_count > 0 {
            actions.push(SuggestAction {
                source: "i18n".into(),
                priority: 3,
                command: Some("dt translate-i18n --dry-run".into()),
                title: "locale_structure".into(),
                detail: format!(
                    "{structure_count} EN/FR pages have heading or link structure mismatches"
                ),
            });
        }
    }

    actions
}

fn summarize_issues(issues: &[DriftIssue]) -> String {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for issue in issues {
        *counts.entry(issue.category.clone()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(cat, n)| format!("{cat}: {n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn summarize_i18n_categories(issues: &[DriftIssue]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for issue in issues {
        *counts.entry(issue.category.clone()).or_insert(0) += 1;
    }
    counts
}

fn count_api_structure_issues(issues: &[DriftIssue]) -> usize {
    issues
        .iter()
        .filter(|issue| {
            issue.category == "locale_structure"
                && issue
                    .file
                    .as_deref()
                    .is_some_and(|path| path.starts_with("api/"))
        })
        .count()
}

fn graph_stats(config: &DoctoolConfig, monorepo_root: &Path) -> String {
    let graph_path = config.resolve(monorepo_root, &config.graph_cache);
    if let Ok(raw) = std::fs::read_to_string(graph_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            let nodes = v.get("nodes").and_then(|n| n.as_array()).map(|a| a.len());
            let edges = v.get("edges").and_then(|e| e.as_array()).map(|a| a.len());
            return format!("graph nodes={nodes:?} edges={edges:?}");
        }
    }
    "(run `dt scan` to refresh knowledge graph)".into()
}

fn run_ts_drift_capture(monorepo_root: &Path) -> Result<(bool, String)> {
    let docs_dir = monorepo_root.join("apps/docs");
    let output = std::process::Command::new("pnpm")
        .args(["docs:drift"])
        .current_dir(&docs_dir)
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let combined = format!("{stderr}{stdout}");
    Ok((output.status.success(), combined))
}
