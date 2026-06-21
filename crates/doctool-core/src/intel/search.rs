use ignore::WalkBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastSearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
}

pub fn fast_workspace_search(
    root: &str,
    pattern: &str,
    is_regex: bool,
) -> Result<Vec<FastSearchResult>, String> {
    let regex = if is_regex {
        Regex::new(pattern).map_err(|e| e.to_string())?
    } else {
        Regex::new(&regex::escape(pattern)).map_err(|e| e.to_string())?
    };

    let results = Arc::new(Mutex::new(Vec::new()));

    let walker = WalkBuilder::new(root).build_parallel();
    walker.run(|| {
        let results = Arc::clone(&results);
        let regex = regex.clone();

        Box::new(move |result| {
            if let Ok(entry) = result {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        let mut local_matches = Vec::new();
                        for (i, line) in content.lines().enumerate() {
                            if regex.is_match(line) {
                                local_matches.push(FastSearchResult {
                                    file_path: entry.path().to_string_lossy().into_owned(),
                                    line_number: i + 1,
                                    line_content: line.trim().to_string(),
                                });
                            }
                        }
                        if !local_matches.is_empty() {
                            if let Ok(mut global_results) = results.lock() {
                                global_results.extend(local_matches);
                            }
                        }
                    }
                }
            }
            ignore::WalkState::Continue
        })
    });

    let final_results = matches_cloned(&results);
    Ok(final_results)
}

fn matches_cloned(results: &Arc<Mutex<Vec<FastSearchResult>>>) -> Vec<FastSearchResult> {
    results.lock().unwrap().clone()
}
