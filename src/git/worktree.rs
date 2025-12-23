use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

use crate::util::run_cmd_allow_fail;

pub fn current_commit(repo_root: &Path) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root).args(["rev-parse", "HEAD"]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        return Err(anyhow!("failed to resolve HEAD"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn create_worktree(repo_root: &Path, branch: &str, path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root)
        .args(["worktree", "add", "-b", branch, path.to_str().unwrap()]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git worktree failed: {stderr}"));
    }
    Ok(())
}

pub fn list_worktrees(repo_root: &Path) -> Result<Vec<String>> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root)
        .args(["worktree", "list", "--porcelain"]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        return Err(anyhow!("git worktree list failed"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut paths = Vec::new();
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            paths.push(path.to_string());
        }
    }
    Ok(paths)
}

pub fn git_diff_numstat(repo_root: &Path, base: &str) -> Result<(u64, u64)> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root).args(["diff", "--numstat", base]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        return Err(anyhow!("git diff failed"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut added = 0u64;
    let mut removed = 0u64;
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let add = parts.next().unwrap_or("0");
        let del = parts.next().unwrap_or("0");
        added += add.parse::<u64>().unwrap_or(0);
        removed += del.parse::<u64>().unwrap_or(0);
    }
    Ok((added, removed))
}

pub fn ensure_base_ref(repo_root: &Path, base_ref: &str) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root)
        .args(["rev-parse", "--verify", base_ref]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        return Err(anyhow!("base ref not found: {base_ref}"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

pub fn git_diff_names(repo_root: &Path, base_ref: &str) -> Result<Vec<String>> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root)
        .args(["diff", "--name-only", base_ref]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        return Err(anyhow!("git diff failed"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

pub fn merge_branch(repo_root: &Path, branch: &str, no_ff: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root).arg("merge");
    if no_ff {
        cmd.arg("--no-ff");
    }
    cmd.arg(branch);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git merge failed: {stderr}"));
    }
    Ok(())
}

pub fn cherry_pick(repo_root: &Path, branch: &str) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root)
        .args(["cherry-pick", "-x", branch]);
    let output = run_cmd_allow_fail(cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git cherry-pick failed: {stderr}"));
    }
    Ok(())
}

pub fn move_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to.parent().unwrap()).with_context(|| "create archive dir")?;
    std::fs::rename(from, to).with_context(|| "move change dir")?;
    Ok(())
}
