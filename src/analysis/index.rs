use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::util::{log_event, normalize_path, run_cmd_allow_fail};

const DEFAULT_MAX_BYTES: u64 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndex {
    pub files: Vec<FileEntry>,
}

pub struct IndexResult {
    pub index: FileIndex,
    pub repo_tree: String,
    pub file_hashes: HashMap<String, String>,
    pub index_hash: String,
}

pub fn build_index(repo_root: &Path, include_untracked: bool) -> Result<IndexResult> {
    let mut files = list_git_files(repo_root, include_untracked)?;
    files.sort();

    let mut entries = Vec::new();
    let mut file_hashes = HashMap::new();

    for rel in files {
        let full = repo_root.join(&rel);
        if should_exclude(&rel) {
            continue;
        }
        if let Ok(metadata) = std::fs::metadata(&full) {
            if metadata.len() > DEFAULT_MAX_BYTES {
                continue;
            }
        }
        if is_binary(&full)? {
            continue;
        }
        let hash = hash_file(&full)?;
        let size = std::fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
        let path = match normalize_path(Path::new(&rel)) {
            Ok(path) => path,
            Err(err) => {
                log_event("warn", &format!("skip invalid path {rel}: {err}"));
                continue;
            }
        };
        file_hashes.insert(path.clone(), hash.clone());
        entries.push(FileEntry { path, hash, size });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let index = FileIndex { files: entries };
    let index_hash = hash_index(&index);
    let repo_tree = build_repo_tree(&index);
    Ok(IndexResult {
        index,
        repo_tree,
        file_hashes,
        index_hash,
    })
}

fn list_git_files(repo_root: &Path, include_untracked: bool) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let mut tracked_cmd = std::process::Command::new("git");
    tracked_cmd.current_dir(repo_root).args(["ls-files", "-z"]);
    let tracked = run_cmd_allow_fail(tracked_cmd)?;
    if !tracked.status.success() {
        return Err(anyhow!("failed to list git files"));
    }
    files.extend(split_nul(&tracked.stdout));

    if include_untracked {
        let mut untracked_cmd = std::process::Command::new("git");
        untracked_cmd
            .current_dir(repo_root)
            .args(["ls-files", "--others", "--exclude-standard", "-z"]);
        let untracked = run_cmd_allow_fail(untracked_cmd)?;
        if untracked.status.success() {
            files.extend(split_nul(&untracked.stdout));
        }
    }

    Ok(files)
}

fn split_nul(data: &[u8]) -> Vec<String> {
    data.split(|b| *b == 0)
        .filter_map(|chunk| {
            if chunk.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(chunk).to_string())
            }
        })
        .collect()
}

fn should_exclude(rel: &str) -> bool {
    rel.starts_with(".git/")
        || rel.starts_with("target/")
        || rel.starts_with("node_modules/")
        || rel.starts_with(".codex/sdd/")
}

fn is_binary(path: &Path) -> Result<bool> {
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut buf = [0u8; 1024];
    let read = file.read(&mut buf).unwrap_or(0);
    Ok(buf[..read].iter().any(|b| *b == 0))
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).with_context(|| format!("read {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn hash_index(index: &FileIndex) -> String {
    let mut hasher = Hasher::new();
    for entry in &index.files {
        hasher.update(entry.path.as_bytes());
        hasher.update(entry.hash.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn build_repo_tree(index: &FileIndex) -> String {
    let mut out = String::new();
    for entry in &index.files {
        out.push_str(&entry.path);
        out.push('\n');
    }
    out
}

pub fn write_index(path: &Path, index: &FileIndex) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(index).with_context(|| "serialize index")?;
    let mut file = File::create(path).with_context(|| format!("write {}", path.display()))?;
    file.write_all(data.as_bytes())?;
    Ok(())
}

pub fn write_repo_tree(path: &Path, tree: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut file = File::create(path).with_context(|| format!("write {}", path.display()))?;
    file.write_all(tree.as_bytes())?;
    Ok(())
}

pub fn shard_files(index: &FileIndex, shards: usize) -> Vec<Vec<FileEntry>> {
    if shards == 0 {
        return vec![];
    }
    let total = index.files.len();
    let chunk = (total + shards - 1) / shards;
    let mut out = Vec::new();
    for i in 0..shards {
        let start = i * chunk;
        let end = std::cmp::min(start + chunk, total);
        if start >= end {
            out.push(Vec::new());
            continue;
        }
        out.push(index.files[start..end].to_vec());
    }
    out
}

pub fn shard_hash(entries: &[FileEntry]) -> String {
    let mut hasher = Hasher::new();
    for entry in entries {
        hasher.update(entry.path.as_bytes());
        hasher.update(entry.hash.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}
