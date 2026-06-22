use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::sources::mdx::document::hash_value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub version: u32,
    pub files: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
pub struct FileChanges {
    pub added_keys: Vec<String>,
    pub removed_keys: Vec<String>,
    pub changed_keys: Vec<String>,
}

pub struct LockFileManager {
    lock_file: LockFile,
    lock_file_path: PathBuf,
    monorepo_root: PathBuf,
}

impl LockFileManager {
    pub fn load(monorepo_root: &Path, lock_cache_relative: &str) -> Result<Self> {
        let lock_file_path = monorepo_root.join(lock_cache_relative);
        let lock_file = if lock_file_path.is_file() {
            let raw = fs::read_to_string(&lock_file_path)
                .with_context(|| format!("read {}", lock_file_path.display()))?;
            serde_yaml::from_str(&raw).unwrap_or_else(|_| LockFile {
                version: 1,
                files: HashMap::new(),
            })
        } else {
            LockFile {
                version: 1,
                files: HashMap::new(),
            }
        };

        Ok(Self {
            lock_file,
            lock_file_path,
            monorepo_root: monorepo_root.to_path_buf(),
        })
    }

    pub fn lock_path(&self) -> &Path {
        &self.lock_file_path
    }

    pub fn exists(&self) -> bool {
        self.lock_file_path.is_file()
    }

    fn relative_key(&self, file_path: &str) -> String {
        file_path.replace('\\', "/")
    }

    pub fn register_source(&mut self, file_path: &str, source_data: &HashMap<String, String>) {
        let relative = self.relative_key(file_path);
        let mut hashed = HashMap::new();
        for (key, value) in source_data {
            hashed.insert(key.clone(), hash_value(value));
        }
        self.lock_file.files.insert(relative, hashed);
    }

    pub fn get_changes(
        &self,
        file_path: &str,
        source_data: &HashMap<String, String>,
    ) -> FileChanges {
        let relative = self.relative_key(file_path);
        let previous = self.lock_file.files.get(&relative);

        let current_keys: HashSet<_> = source_data.keys().cloned().collect();

        let Some(previous_state) = previous else {
            return FileChanges {
                added_keys: source_data.keys().cloned().collect(),
                removed_keys: vec![],
                changed_keys: vec![],
            };
        };

        let previous_keys: HashSet<_> = previous_state.keys().cloned().collect();

        let added_keys: Vec<_> = current_keys
            .difference(&previous_keys)
            .cloned()
            .collect();
        let removed_keys: Vec<_> = previous_keys
            .difference(&current_keys)
            .cloned()
            .collect();
        let changed_keys: Vec<_> = current_keys
            .intersection(&previous_keys)
            .filter(|key| {
                hash_value(&source_data[*key]) != previous_state[*key]
            })
            .cloned()
            .collect();

        FileChanges {
            added_keys,
            removed_keys,
            changed_keys,
        }
    }

    pub fn has_file(&self, file_path: &str) -> bool {
        self.lock_file.files.contains_key(&self.relative_key(file_path))
    }

    pub fn has_stale_segments(
        &self,
        file_path: &str,
        source_data: &HashMap<String, String>,
    ) -> bool {
        let changes = self.get_changes(file_path, source_data);
        !changes.added_keys.is_empty()
            || !changes.removed_keys.is_empty()
            || !changes.changed_keys.is_empty()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.lock_file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(&self.lock_file)?;
        fs::write(&self.lock_file_path, yaml)
            .with_context(|| format!("write {}", self.lock_file_path.display()))?;
        Ok(())
    }

    pub fn refresh_from_corpus(
        &mut self,
        entries: impl IntoIterator<Item = (String, HashMap<String, String>)>,
    ) -> Result<()> {
        self.lock_file.version = 1;
        self.lock_file.files.clear();
        for (path, segments) in entries {
            self.register_source(&path, &segments);
        }
        self.save()
    }

    pub fn monorepo_root(&self) -> &Path {
        &self.monorepo_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_changed_segments() {
        let tmp = TempDir::new().unwrap();
        let lock_path = ".doctool/i18n.lock";
        let mut mgr = LockFileManager::load(tmp.path(), lock_path).unwrap();

        let mut initial = HashMap::new();
        initial.insert("frontmatter:title".into(), "Hello".into());
        mgr.register_source("build/a.mdx", &initial);
        mgr.save().unwrap();

        let mut mgr2 = LockFileManager::load(tmp.path(), lock_path).unwrap();
        let mut updated = initial.clone();
        updated.insert("frontmatter:title".into(), "Hello world".into());
        let changes = mgr2.get_changes("build/a.mdx", &updated);
        assert!(changes.changed_keys.contains(&"frontmatter:title".to_string()));
    }
}
