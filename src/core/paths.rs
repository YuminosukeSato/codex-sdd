use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

use crate::util::run_cmd_allow_fail;

#[derive(Clone, Debug)]
pub struct GlobalPaths {
    pub codex_home: PathBuf,
}

#[derive(Clone, Debug)]
pub struct RepoPaths {
    pub repo_root: PathBuf,
    pub docs_sdd: PathBuf,
    pub docs_changes: PathBuf,
    pub state_path: PathBuf,
    pub runs_dir: PathBuf,
    pub worktrees_dir: PathBuf,
    pub schemas_dir: PathBuf,
}

pub fn resolve_codex_home() -> Result<PathBuf> {
    if let Ok(path) = env::var("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow!("home directory not found"))?;
    Ok(home.join(".codex"))
}

pub fn git_repo_root() -> Result<PathBuf> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        return Err(anyhow!("Gitリポジトリが必要です"));
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        return Err(anyhow!("Gitリポジトリが必要です"));
    }
    Ok(PathBuf::from(root))
}

impl GlobalPaths {
    pub fn load() -> Result<Self> {
        Ok(Self {
            codex_home: resolve_codex_home()?,
        })
    }
}

impl RepoPaths {
    pub fn load() -> Result<Self> {
        let repo_root = git_repo_root()?;
        let docs_sdd = repo_root.join("docs/sdd");
        let docs_changes = docs_sdd.join("changes");
        let codex_sdd_dir = repo_root.join(".codex/sdd");
        let state_path = codex_sdd_dir.join("state.json");
        let runs_dir = codex_sdd_dir.join("runs");
        let worktrees_dir = codex_sdd_dir.join("worktrees");
        let schemas_dir = codex_sdd_dir.join("schemas");
        Ok(Self {
            repo_root,
            docs_sdd,
            docs_changes,
            state_path,
            runs_dir,
            worktrees_dir,
            schemas_dir,
        })
    }

    pub fn change_dir(&self, change_id: &str, name: &str) -> PathBuf {
        let dir_name = format!("{}_{}", change_id, name);
        self.docs_changes.join(dir_name)
    }

    pub fn find_change_dir(&self, change_id: &str) -> Result<PathBuf> {
        let entries = std::fs::read_dir(&self.docs_changes)
            .with_context(|| format!("read {}", self.docs_changes.display()))?;
        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&format!("{change_id}_")) {
                return Ok(entry.path());
            }
        }
        Err(anyhow!("change workspace not found for {change_id}"))
    }

    pub fn change_context_dir(&self, change_dir: &Path) -> PathBuf {
        change_dir.join("context")
    }
}
