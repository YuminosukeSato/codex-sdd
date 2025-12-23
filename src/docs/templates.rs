use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::util::{ensure_dir, write_file, write_file_if_missing};

pub const PROMPT_PLANS_FILENAME: &str = "plans.md";

pub fn render_agents_md() -> String {
    let contents = r#"<!-- OPENSPEC:START -->

# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:

- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:

- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# Project Agent Instructions (codex-sdd)

## 1) Language selection
- Start by asking the user which language to use (e.g., English / Japanese).
- After the user selects, respond only in that language.

## 2) Context7 usage
- If the task involves a library/framework and Context7 docs are available, resolve the
  library ID and use Context7 as the authoritative source before coding or explaining.

## 3) codex-sdd usage (detailed)

### Prerequisites
- Rust toolchain (cargo)
- A git repository (commands use `git` under the hood)
- Codex CLI available as `codex`
- Optional: `cargo llvm-cov` or `cargo tarpaulin` for coverage

### Setup
1. `codex-sdd install` to create `CODEX_HOME/prompts/plans.md`.
2. `codex-sdd init` to scaffold `docs/sdd` and `AGENTS.md`.

### End-to-end workflow
1. Start a change session:
   - `codex-sdd plans --name "<change-name>" [--agents N] [--include-untracked]`
   - Output: `docs/sdd/changes/<id>_<name>/10_repo_digest.md`
2. Generate review and tasks:
   - `codex-sdd review`
   - `codex-sdd tasks`
   - Outputs: `20_review.md`, `40_tasks.md`
3. Approve to unlock work:
   - `codex-sdd approve`
   - Output: `90_decision.md`
4. Create worktrees:
   - `codex-sdd worktrees --agents N`
   - Output: `.codex/sdd/worktrees/<change_id>/agentN`
5. Test plan and execution:
   - `codex-sdd test-plan [--coverage llvm-cov|tarpaulin|none]`
   - Output: `50_test_plan.md` + metrics in `.codex/sdd/runs/<change_id>/`
6. Compare variants:
   - `codex-sdd select`
   - Output: `80_selection.md`
7. Finalize:
   - `codex-sdd finalize --agent agent1 [--strategy merge|cherry-pick]`
   - Requires `docs/sdd/specs/<spec>.md` update when code changed.
   - Output: change archived to `docs/sdd/archive/<date>-<change_dir>`

### CI check rules
- `codex-sdd check` passes if only `docs/**` changed.
- For code changes, it requires:
  - `docs/sdd/specs/*.md` updated
  - `90_decision.md`, `40_tasks.md`, `50_test_plan.md` present under a change directory.

### Generated paths
- `docs/sdd/specs/` current specs
- `docs/sdd/changes/` active sessions
- `docs/sdd/archive/` completed changes
- `.codex/sdd/` runtime state (state.json, runs, worktrees, schemas)

### Environment variables
- `CODEX_HOME`: Base directory for Codex assets (default: `~/.codex`).
- `CODEX_SDD_PROMPT_FLAG`: Override the prompt flag (default: `--prompt-file`).
- `CODEX_SDD_EXEC_ARGS`: Extra args passed to `codex exec`.
"#;
    contents.to_string()
}

pub fn render_prompt_plans() -> String {
    let contents = r#"---
name: plans
argument-hint: change-id
---

# Codex SDD Plans

以下の手順でリポジトリの概要を把握し、`docs/sdd/changes/<change_id>_.../10_repo_digest.md` を更新してください。

1. `docs/sdd/changes/<change_id>_.../context/file_index.json` を参照して対象ファイルを把握する
2. `docs/sdd/changes/<change_id>_.../context/repo_tree.txt` を読み、ディレクトリ構造を理解する
3. 重要な領域・公開API・リスク・テスト観点を整理する

出力は日本語で簡潔にまとめてください。
"#;
    contents.to_string()
}

pub fn render_docs_readme() -> String {
    r#"# Spec-Driven Development (SDD)

このフォルダは `codex-sdd` の成果物を保持します。
- `specs/`: 現行仕様
- `changes/`: 変更提案と作業セッション
"#
    .to_string()
}

pub fn render_change_placeholders() -> Vec<(String, String)> {
    vec![
        (
            "10_repo_digest.md".to_string(),
            "# Repo Digest\n\n(自動生成)\n".to_string(),
        ),
        (
            "20_review.md".to_string(),
            "# Review\n\n(自動生成)\n".to_string(),
        ),
        (
            "40_tasks.md".to_string(),
            "# Tasks\n\n(自動生成)\n".to_string(),
        ),
        (
            "50_test_plan.md".to_string(),
            "# Test Plan\n\n(自動生成)\n".to_string(),
        ),
        (
            "90_decision.md".to_string(),
            "# Decision\n\n(承認後に作成)\n".to_string(),
        ),
    ]
}

pub fn render_context_placeholders() -> Vec<(String, String)> {
    vec![
        (
            "README.md".to_string(),
            "# Context\n\nこのフォルダにはインデックスや補助資料を置きます。\n".to_string(),
        ),
        (
            "repo_tree.txt".to_string(),
            "(自動生成)\n".to_string(),
        ),
        (
            "file_index.json".to_string(),
            "{}\n".to_string(),
        ),
    ]
}

pub fn ensure_repo_scaffold(repo_root: &Path) -> Result<()> {
    let docs_sdd = repo_root.join("docs/sdd");
    ensure_dir(&docs_sdd.join("specs"))?;
    ensure_dir(&docs_sdd.join("changes"))?;
    write_file_if_missing(&docs_sdd.join("README.md"), &render_docs_readme())?;
    Ok(())
}

pub fn ensure_agents_md(repo_root: &Path) -> Result<bool> {
    let path = repo_root.join("AGENTS.md");
    write_file_if_missing(&path, &render_agents_md())
}

pub fn write_prompt(codex_home: &Path) -> Result<PathBuf> {
    let prompts_dir = codex_home.join("prompts");
    ensure_dir(&prompts_dir)?;
    let prompt_path = prompts_dir.join(PROMPT_PLANS_FILENAME);
    write_file(&prompt_path, &render_prompt_plans())?;
    Ok(prompt_path)
}

pub fn ensure_change_scaffold(change_dir: &Path) -> Result<()> {
    ensure_dir(change_dir)?;
    for (name, contents) in render_change_placeholders() {
        let path = change_dir.join(name);
        if !path.exists() {
            write_file(&path, &contents)?;
        }
    }
    let context_dir = change_dir.join("context");
    ensure_dir(&context_dir)?;
    for (name, contents) in render_context_placeholders() {
        let path = context_dir.join(name);
        if !path.exists() {
            write_file(&path, &contents)?;
        }
    }
    Ok(())
}
