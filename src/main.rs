mod analysis;
mod codex;
mod core;
mod docs;
mod git;
mod quality;
mod util;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::analysis::index::{build_index, shard_files, shard_hash, FileEntry};
use crate::codex::exec::{output_paths, ExecSpec};
use crate::core::paths::{GlobalPaths, RepoPaths};
use crate::core::state::State;
use crate::docs::templates::{
    ensure_agents_md, ensure_change_scaffold, ensure_repo_scaffold, write_prompt,
};
use crate::git::worktree::{
    cherry_pick, create_worktree, current_commit, git_diff_names, git_diff_numstat, merge_branch,
    move_dir,
};
use crate::quality::coverage::{run_llvm_cov, run_tarpaulin};
use crate::quality::tests::run_tests;
use crate::util::{
    ensure_dir, log_event, now_rfc3339, read_to_string, slugify, write_file, write_string,
};

#[derive(Parser)]
#[command(name = "codex-sdd", version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install,
    Init,
    Plans(PlansArgs),
    Review(ChangeArgs),
    Tasks(ChangeArgs),
    Approve(ApproveArgs),
    Check(CheckArgs),
    Worktrees(WorktreesArgs),
    TestPlan(TestPlanArgs),
    Select(ChangeArgs),
    Finalize(FinalizeArgs),
}

#[derive(Args)]
struct PlansArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    id: Option<String>,
    #[arg(long, default_value_t = 4)]
    agents: usize,
    #[arg(long)]
    include_untracked: bool,
}

#[derive(Args)]
struct ChangeArgs {
    #[arg(long)]
    id: Option<String>,
}

#[derive(Args)]
struct ApproveArgs {
    #[arg(long)]
    id: Option<String>,
    #[arg(long)]
    by: Option<String>,
}

#[derive(Args)]
struct CheckArgs {
    #[arg(long)]
    base: Option<String>,
}

#[derive(Args)]
struct WorktreesArgs {
    #[arg(long)]
    id: Option<String>,
    #[arg(long, default_value_t = 2)]
    agents: usize,
}

#[derive(Args)]
struct TestPlanArgs {
    #[arg(long)]
    id: Option<String>,
    #[arg(long, default_value = "llvm-cov")]
    coverage: String,
}

#[derive(Args)]
struct FinalizeArgs {
    #[arg(long)]
    id: Option<String>,
    #[arg(long)]
    agent: String,
    #[arg(long, default_value = "merge")]
    strategy: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VariantMetrics {
    agent: String,
    tests_passed: bool,
    coverage_percent: Option<f64>,
    coverage_tool: String,
    test_output: String,
    coverage_output: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SelectionVariant {
    agent: String,
    tests_passed: bool,
    coverage_percent: Option<f64>,
    lines_added: u64,
    lines_removed: u64,
    notes: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install => cmd_install(),
        Commands::Init => cmd_init(),
        Commands::Plans(args) => cmd_plans(args),
        Commands::Review(args) => cmd_review(args),
        Commands::Tasks(args) => cmd_tasks(args),
        Commands::Approve(args) => cmd_approve(args),
        Commands::Check(args) => cmd_check(args),
        Commands::Worktrees(args) => cmd_worktrees(args),
        Commands::TestPlan(args) => cmd_test_plan(args),
        Commands::Select(args) => cmd_select(args),
        Commands::Finalize(args) => cmd_finalize(args),
    }
}

fn cmd_install() -> Result<()> {
    log_event("info", "install prompt");
    let global = GlobalPaths::load()?;
    let prompt_path = write_prompt(&global.codex_home)?;
    println!(
        "prompts/plans.md を {} に作成しました。新しいCodexセッションを開いてください。",
        prompt_path.display()
    );
    Ok(())
}

fn cmd_init() -> Result<()> {
    log_event("info", "init repo scaffold");
    let paths = RepoPaths::load()?;
    ensure_repo_scaffold(&paths.repo_root)?;
    let created = ensure_agents_md(&paths.repo_root)?;
    if created {
        println!("AGENTS.md を作成しました。");
    } else {
        println!("AGENTS.md は既に存在します。");
    }
    println!(".codex/sdd/ を .gitignore に追加することを推奨します（.codex/skills は除外しないでください）。");
    Ok(())
}

fn cmd_plans(args: PlansArgs) -> Result<()> {
    log_event("info", "plans start");
    let paths = RepoPaths::load()?;
    ensure_repo_scaffold(&paths.repo_root)?;

    let mut state = State::load(&paths.state_path)?;
    let name_slug = slugify(&args.name);
    let base_id = args.id.unwrap_or_else(|| name_slug.clone());
    let change_id = ensure_unique_change_id(&paths, &base_id, &name_slug)?;
    let change_dir = paths.change_dir(&change_id, &name_slug);
    ensure_change_scaffold(&change_dir)?;

    let index_result = build_index(&paths.repo_root, args.include_untracked)?;
    let context_dir = paths.change_context_dir(&change_dir);
    let index_path = context_dir.join("file_index.json");
    let tree_path = context_dir.join("repo_tree.txt");
    crate::analysis::index::write_index(&index_path, &index_result.index)?;
    crate::analysis::index::write_repo_tree(&tree_path, &index_result.repo_tree)?;

    {
        let change_state = state.change_state_mut(&change_id);
        change_state
            .file_hashes
            .clone_from(&index_result.file_hashes);
        change_state.file_index_hash = Some(index_result.index_hash.clone());
        change_state.file_index_generated_at = Some(now_rfc3339());
    }
    state.active_change_id = Some(change_id.clone());
    let existing_shard_hashes = state
        .change_state(&change_id)
        .map(|c| c.reader_shard_hashes.clone())
        .unwrap_or_default();

    ensure_schemas(&paths)?;

    let shards = shard_files(&index_result.index, args.agents);
    ensure_dir(&paths.runs_dir.join(&change_id))?;

    let mut handles = Vec::new();
    for (idx, shard) in shards.iter().enumerate() {
        if shard.is_empty() {
            continue;
        }
        let shard_name = format!("reader_{idx}");
        let shard_hash_val = shard_hash(shard);
        let existing_hash = existing_shard_hashes.get(&shard_name).cloned();
        let (output_path, json_path) = output_paths(&paths.runs_dir, &change_id, &shard_name);

        if existing_hash == Some(shard_hash_val.clone()) && output_path.exists() {
            log_event("info", &format!("reuse shard {idx}"));
            continue;
        }

        let prompt_path = context_dir.join(format!("reader_prompt_{idx}.md"));
        let prompt = render_reader_prompt(&change_id, idx, shards.len(), shard);
        write_string(&prompt_path, &prompt)?;

        let schema_path = paths.schemas_dir.join("reader.json");
        let exec_spec = ExecSpec {
            cwd: paths.repo_root.clone(),
            prompt_path,
            output_path: output_path.clone(),
            json_output_path: Some(json_path),
            sandbox: "read-only".to_string(),
            schema_path: Some(schema_path),
        };

        let shard_key = shard_name.clone();
        handles.push(std::thread::spawn(
            move || -> Result<(String, String, bool)> {
                let result = crate::codex::exec::run(&exec_spec)?;
                Ok((shard_key, shard_hash_val, result.status_ok))
            },
        ));
    }

    for handle in handles {
        let (shard_key, shard_hash_val, ok) = handle
            .join()
            .map_err(|_| anyhow!("reader thread failed"))??;
        if !ok {
            return Err(anyhow!("reader agent failed"));
        }
        state.record_thread(&change_id, &shard_key, &shard_key);
        let change_state = state.change_state_mut(&change_id);
        change_state
            .reader_shard_hashes
            .insert(shard_key, shard_hash_val);
    }

    let repo_digest = compose_repo_digest(&paths, &change_id, shards.len())?;
    write_file(&change_dir.join("repo_digest.md"), &repo_digest)?;
    write_file(&change_dir.join("10_repo_digest.md"), &repo_digest)?;

    state.save(&paths.state_path)?;
    println!("plans 完了: {}", change_dir.display());
    Ok(())
}

fn cmd_review(args: ChangeArgs) -> Result<()> {
    log_event("info", "review start");
    let paths = RepoPaths::load()?;
    let mut state = State::load(&paths.state_path)?;
    let change_id = resolve_change_id(&state, args.id.as_deref())?;
    let change_dir = paths.find_change_dir(&change_id)?;
    ensure_schemas(&paths)?;
    ensure_dir(&paths.runs_dir.join(&change_id))?;

    let prompt = render_review_prompt(&change_dir, &change_id);
    let prompt_path = paths
        .change_context_dir(&change_dir)
        .join("review_prompt.md");
    write_string(&prompt_path, &prompt)?;

    let (output_path, json_path) = output_paths(&paths.runs_dir, &change_id, "review");
    let exec_spec = ExecSpec {
        cwd: paths.repo_root.clone(),
        prompt_path,
        output_path: output_path.clone(),
        json_output_path: Some(json_path),
        sandbox: "read-only".to_string(),
        schema_path: Some(paths.schemas_dir.join("review.json")),
    };

    let result = crate::codex::exec::run(&exec_spec)?;
    if !result.status_ok {
        return Err(anyhow!("review failed"));
    }
    state.record_thread(&change_id, "review", "review");
    state.save(&paths.state_path)?;

    let contents = read_to_string(&output_path)?;
    write_file(&change_dir.join("20_review.md"), &contents)?;
    println!("review 完了: {}", change_dir.display());
    Ok(())
}

fn cmd_tasks(args: ChangeArgs) -> Result<()> {
    log_event("info", "tasks start");
    let paths = RepoPaths::load()?;
    let mut state = State::load(&paths.state_path)?;
    let change_id = resolve_change_id(&state, args.id.as_deref())?;
    let change_dir = paths.find_change_dir(&change_id)?;
    ensure_schemas(&paths)?;
    ensure_dir(&paths.runs_dir.join(&change_id))?;

    let prompt = render_tasks_prompt(&change_dir, &change_id);
    let prompt_path = paths
        .change_context_dir(&change_dir)
        .join("tasks_prompt.md");
    write_string(&prompt_path, &prompt)?;

    let (output_path, json_path) = output_paths(&paths.runs_dir, &change_id, "tasks");
    let exec_spec = ExecSpec {
        cwd: paths.repo_root.clone(),
        prompt_path,
        output_path: output_path.clone(),
        json_output_path: Some(json_path),
        sandbox: "read-only".to_string(),
        schema_path: Some(paths.schemas_dir.join("tasks.json")),
    };

    let result = crate::codex::exec::run(&exec_spec)?;
    if !result.status_ok {
        return Err(anyhow!("tasks failed"));
    }
    state.record_thread(&change_id, "tasks", "tasks");
    state.save(&paths.state_path)?;

    let contents = read_to_string(&output_path)?;
    write_file(&change_dir.join("40_tasks.md"), &contents)?;
    println!("tasks 完了: {}", change_dir.display());
    Ok(())
}

fn cmd_approve(args: ApproveArgs) -> Result<()> {
    log_event("info", "approve change");
    let paths = RepoPaths::load()?;
    let mut state = State::load(&paths.state_path)?;
    let change_id = resolve_change_id(&state, args.id.as_deref())?;
    let change_dir = paths.find_change_dir(&change_id)?;

    let approved_by = args
        .by
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "unknown".to_string());
    state.approve_change(&change_id, &approved_by);
    state.save(&paths.state_path)?;

    let decision = format!(
        "# Decision\n\n- approved: true\n- approved_at: {}\n- approved_by: {}\n",
        now_rfc3339(),
        approved_by
    );
    write_file(&change_dir.join("90_decision.md"), &decision)?;
    println!("approve 完了: {}", change_dir.display());
    Ok(())
}

fn cmd_check(args: CheckArgs) -> Result<()> {
    log_event("info", "check start");
    let paths = RepoPaths::load()?;
    let base = resolve_base_ref(&paths.repo_root, args.base.as_deref())?;
    let changed = git_diff_names(&paths.repo_root, &base)?;

    if changed.is_empty() {
        println!("変更なし");
        return Ok(());
    }

    let docs_only = changed.iter().all(|p| p.starts_with("docs/"));
    if docs_only {
        println!("docs-only 変更のため check は成功扱いです。");
        return Ok(());
    }

    let code_changed = changed.iter().any(|p| {
        p.starts_with("src/") || p.starts_with("tests/") || p == "Cargo.toml" || p == "Cargo.lock"
    });

    if code_changed {
        let required_specs = changed
            .iter()
            .any(|p| p.starts_with("docs/sdd/specs/") && p.ends_with(".md"));
        if !required_specs {
            return Err(anyhow!(
                "code変更には docs/sdd/specs/<spec>.md の更新が必要です"
            ));
        }

        let (decision_ok, tasks_ok, test_plan_ok) = required_artifacts(&changed);
        if !(decision_ok && tasks_ok && test_plan_ok) {
            return Err(anyhow!("code変更には docs/sdd/changes/<id>_<name>/90_decision.md, 40_tasks.md, 50_test_plan.md が必要です"));
        }
    }

    println!("check 完了");
    Ok(())
}

fn cmd_worktrees(args: WorktreesArgs) -> Result<()> {
    log_event("info", "worktrees start");
    let paths = RepoPaths::load()?;
    let mut state = State::load(&paths.state_path)?;
    let change_id = resolve_change_id(&state, args.id.as_deref())?;
    state.require_approved(&change_id)?;

    let base_commit = current_commit(&paths.repo_root)?;
    let change_state = state.change_state_mut(&change_id);
    change_state.base_commit = Some(base_commit);
    state.save(&paths.state_path)?;

    let worktree_root = paths.worktrees_dir.join(&change_id);
    ensure_dir(&worktree_root)?;

    for idx in 1..=args.agents {
        let agent_name = format!("agent{idx}");
        let branch = format!("sdd/{change_id}/{agent_name}");
        let path = worktree_root.join(&agent_name);
        create_worktree(&paths.repo_root, &branch, &path)?;
    }

    println!("worktrees 完了: {}", worktree_root.display());
    Ok(())
}

fn cmd_test_plan(args: TestPlanArgs) -> Result<()> {
    log_event("info", "test-plan start");
    let paths = RepoPaths::load()?;
    let state = State::load(&paths.state_path)?;
    let change_id = resolve_change_id(&state, args.id.as_deref())?;
    state.require_approved(&change_id)?;
    state.save(&paths.state_path)?;

    let change_dir = paths.find_change_dir(&change_id)?;
    let worktree_root = paths.worktrees_dir.join(&change_id);
    if !worktree_root.exists() {
        return Err(anyhow!("worktrees が存在しません"));
    }
    ensure_schemas(&paths)?;
    ensure_dir(&paths.runs_dir.join(&change_id))?;

    let mut metrics = Vec::new();
    let mut plan_sections = Vec::new();

    for entry in fs::read_dir(&worktree_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let agent = entry.file_name().to_string_lossy().to_string();
        let worktree_path = entry.path();

        let prompt = render_test_plan_prompt(&change_id, &agent);
        let prompt_path = paths
            .change_context_dir(&change_dir)
            .join(format!("test_plan_prompt_{agent}.md"));
        write_string(&prompt_path, &prompt)?;

        let (output_path, json_path) =
            output_paths(&paths.runs_dir, &change_id, &format!("test_plan_{agent}"));
        let exec_spec = ExecSpec {
            cwd: worktree_path.clone(),
            prompt_path: prompt_path.clone(),
            output_path: output_path.clone(),
            json_output_path: Some(json_path),
            sandbox: "workspace-write".to_string(),
            schema_path: Some(paths.schemas_dir.join("tasks.json")),
        };
        let result = crate::codex::exec::run(&exec_spec)?;
        if !result.status_ok {
            return Err(anyhow!("test plan agent failed"));
        }

        let test_result = run_tests(&worktree_path)?;
        let test_output_path = paths
            .runs_dir
            .join(&change_id)
            .join(format!("test_results_{agent}.txt"));
        write_string(&test_output_path, &test_result.stdout)?;

        let (coverage_percent, coverage_output_path, coverage_tool) = match args.coverage.as_str() {
            "none" => (None, None, "none".to_string()),
            "tarpaulin" => {
                let cov = run_tarpaulin(&worktree_path)?;
                let out_path = paths
                    .runs_dir
                    .join(&change_id)
                    .join(format!("coverage_{agent}.txt"));
                write_string(&out_path, &cov.stdout)?;
                (
                    cov.percent,
                    Some(out_path.to_string_lossy().to_string()),
                    "tarpaulin".to_string(),
                )
            }
            _ => {
                let cov = run_llvm_cov(&worktree_path)?;
                let out_path = paths
                    .runs_dir
                    .join(&change_id)
                    .join(format!("coverage_{agent}.txt"));
                write_string(&out_path, &cov.stdout)?;
                (
                    cov.percent,
                    Some(out_path.to_string_lossy().to_string()),
                    "llvm-cov".to_string(),
                )
            }
        };

        let contents = read_to_string(&output_path)?;
        plan_sections.push(format!("## {agent}\n\n{contents}\n"));

        metrics.push(VariantMetrics {
            agent,
            tests_passed: test_result.success,
            coverage_percent,
            coverage_tool,
            test_output: test_output_path.to_string_lossy().to_string(),
            coverage_output: coverage_output_path,
        });
    }

    let summary = format!("# Test Plan\n\n{}", plan_sections.join("\n"));
    write_file(&change_dir.join("50_test_plan.md"), &summary)?;
    let metrics_path = paths.runs_dir.join(&change_id).join("metrics.json");
    write_string(&metrics_path, &serde_json::to_string_pretty(&metrics)?)?;

    println!("test-plan 完了: {}", change_dir.display());
    Ok(())
}

fn cmd_select(args: ChangeArgs) -> Result<()> {
    log_event("info", "select start");
    let paths = RepoPaths::load()?;
    let state = State::load(&paths.state_path)?;
    let change_id = resolve_change_id(&state, args.id.as_deref())?;
    let change_dir = paths.find_change_dir(&change_id)?;

    let metrics_path = paths.runs_dir.join(&change_id).join("metrics.json");
    if !metrics_path.exists() {
        return Err(anyhow!(
            "metrics が見つかりません。先に test-plan を実行してください"
        ));
    }
    let data = read_to_string(&metrics_path)?;
    let metrics: Vec<VariantMetrics> = serde_json::from_str(&data)?;

    let base_commit = state
        .change_state(&change_id)
        .and_then(|c| c.base_commit.clone())
        .unwrap_or_else(|| "HEAD~1".to_string());

    let mut variants = Vec::new();
    let worktree_root = paths.worktrees_dir.join(&change_id);
    for metric in metrics {
        let worktree_path = worktree_root.join(&metric.agent);
        let (added, removed) = git_diff_numstat(&worktree_path, &base_commit)?;
        let notes = format!("coverage: {:?}", metric.coverage_percent);
        variants.push(SelectionVariant {
            agent: metric.agent,
            tests_passed: metric.tests_passed,
            coverage_percent: metric.coverage_percent,
            lines_added: added,
            lines_removed: removed,
            notes,
        });
    }

    let tasks_completion = task_completion_ratio(&change_dir.join("40_tasks.md"));
    let risk_flag = detect_risk(&change_dir.join("20_review.md"));

    let mut summary = String::from("# Selection Summary\n\n");
    summary.push_str(&format!(
        "- tasks_completion: {:.1}%\n",
        tasks_completion * 100.0
    ));
    summary.push_str(&format!(
        "- risk_flag: {}\n\n",
        if risk_flag { "あり" } else { "なし" }
    ));
    summary.push_str("## Variants\n");
    for v in &variants {
        summary.push_str(&format!(
            "- {}: tests_passed={}, coverage={:?}, diff=+{} -{}\n",
            v.agent, v.tests_passed, v.coverage_percent, v.lines_added, v.lines_removed
        ));
    }

    write_file(&change_dir.join("80_selection.md"), &summary)?;
    let json_path = paths.runs_dir.join(&change_id).join("selection.json");
    write_string(&json_path, &serde_json::to_string_pretty(&variants)?)?;

    println!("select 完了: {}", change_dir.display());
    Ok(())
}

fn cmd_finalize(args: FinalizeArgs) -> Result<()> {
    log_event("info", "finalize start");
    let paths = RepoPaths::load()?;
    let state = State::load(&paths.state_path)?;
    let change_id = resolve_change_id(&state, args.id.as_deref())?;
    state.require_approved(&change_id)?;

    let change_dir = paths.find_change_dir(&change_id)?;
    let worktree_path = paths.worktrees_dir.join(&change_id).join(&args.agent);
    if worktree_path.exists() {
        if let Some(base_commit) = state
            .change_state(&change_id)
            .and_then(|c| c.base_commit.clone())
        {
            let changed = git_diff_names(&worktree_path, &base_commit)?;
            let spec_updated = changed
                .iter()
                .any(|p| p.starts_with("docs/sdd/specs/") && p.ends_with(".md"));
            if !spec_updated {
                return Err(anyhow!(
                    "finalize には docs/sdd/specs/<spec>.md の更新が必要です"
                ));
            }
        }
    }
    let branch = format!("sdd/{change_id}/{}", args.agent);

    match args.strategy.as_str() {
        "cherry-pick" => cherry_pick(&paths.repo_root, &branch)?,
        _ => merge_branch(&paths.repo_root, &branch, true)?,
    }

    let archive_name = format!(
        "{}-{}",
        chrono::Utc::now().format("%Y-%m-%d"),
        change_dir.file_name().unwrap().to_string_lossy()
    );
    let archive_dir = paths.docs_sdd.join("archive").join(archive_name);
    move_dir(&change_dir, &archive_dir)?;

    println!("finalize 完了: {}", archive_dir.display());
    Ok(())
}

fn resolve_change_id(state: &State, requested: Option<&str>) -> Result<String> {
    if let Some(id) = requested {
        return Ok(id.to_string());
    }
    state
        .active_change_id
        .clone()
        .ok_or_else(|| anyhow!("change id を指定してください"))
}

fn ensure_unique_change_id(paths: &RepoPaths, base_id: &str, name_slug: &str) -> Result<String> {
    let mut candidate = base_id.to_string();
    let mut counter = 2;
    loop {
        let dir = paths.change_dir(&candidate, name_slug);
        if !dir.exists() {
            return Ok(candidate);
        }
        candidate = format!("{}-{}", base_id, counter);
        counter += 1;
    }
}

fn ensure_schemas(paths: &RepoPaths) -> Result<()> {
    ensure_dir(&paths.schemas_dir)?;
    let reader_schema = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "files": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "path": {"type": "string"},
          "role": {"type": "string"},
          "public_api": {"type": "string"},
          "risks": {"type": "string"},
          "test_notes": {"type": "string"}
        },
        "required": ["path"]
      }
    }
  },
  "required": ["files"]
}"#;
    let review_schema = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "findings": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "severity": {"type": "string"},
          "file": {"type": "string"},
          "rationale": {"type": "string"},
          "suggestion": {"type": "string"}
        },
        "required": ["severity", "file"]
      }
    }
  },
  "required": ["findings"]
}"#;
    let tasks_schema = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "tasks": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "id": {"type": "string"},
          "summary": {"type": "string"},
          "files": {"type": "array", "items": {"type": "string"}},
          "acceptance_criteria": {"type": "array", "items": {"type": "string"}},
          "tests": {"type": "array", "items": {"type": "string"}},
          "deps": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["id", "summary"]
      }
    }
  },
  "required": ["tasks"]
}"#;
    let select_schema = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "variants": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "agent": {"type": "string"},
          "coverage": {"type": "number"},
          "tests_passed": {"type": "boolean"},
          "diff_stats": {"type": "string"},
          "notes": {"type": "string"}
        },
        "required": ["agent"]
      }
    }
  },
  "required": ["variants"]
}"#;

    write_schema_file(&paths.schemas_dir.join("reader.json"), reader_schema)?;
    write_schema_file(&paths.schemas_dir.join("review.json"), review_schema)?;
    write_schema_file(&paths.schemas_dir.join("tasks.json"), tasks_schema)?;
    write_schema_file(&paths.schemas_dir.join("select.json"), select_schema)?;
    Ok(())
}

fn write_schema_file(path: &Path, contents: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    write_string(path, contents)
}

fn compose_repo_digest(paths: &RepoPaths, change_id: &str, shards: usize) -> Result<String> {
    let mut out = String::from("# Repo Digest\n\n");
    for idx in 0..shards {
        let name = format!("reader_{idx}");
        let (output_path, _) = output_paths(&paths.runs_dir, change_id, &name);
        if output_path.exists() {
            let contents = read_to_string(&output_path)?;
            out.push_str(&format!("## Shard {idx}\n\n{contents}\n"));
        }
    }
    Ok(out)
}

fn render_reader_prompt(change_id: &str, idx: usize, total: usize, shard: &[FileEntry]) -> String {
    let mut out = String::new();
    out.push_str("# Reader\n\n");
    out.push_str(&format!("change_id: {change_id}\n"));
    out.push_str(&format!("shard: {}/{}\n\n", idx + 1, total));
    out.push_str("対象ファイル:\n");
    for entry in shard {
        out.push_str(&format!("- {}\n", entry.path));
    }
    out.push_str(
        "\n以下を日本語で簡潔にまとめてください:\n- 役割\n- 公開API\n- リスク\n- テスト観点\n",
    );
    out
}

fn render_review_prompt(change_dir: &Path, change_id: &str) -> String {
    format!(
        "# Review\n\nchange_id: {change_id}\n\n次のドキュメントを読み、レビュー観点を整理してください:\n- {}/10_repo_digest.md\n\n出力は JSON スキーマに沿って作成してください。\n",
        change_dir.display()
    )
}

fn render_tasks_prompt(change_dir: &Path, change_id: &str) -> String {
    format!(
        "# Tasks\n\nchange_id: {change_id}\n\n次のドキュメントを読み、実装タスクを整理してください:\n- {}/10_repo_digest.md\n- {}/20_review.md\n\n出力は JSON スキーマに沿って作成してください。\n",
        change_dir.display(),
        change_dir.display()
    )
}

fn render_test_plan_prompt(change_id: &str, agent: &str) -> String {
    format!(
        "# Test Plan\n\nchange_id: {change_id}\nagent: {agent}\n\n対象ブランチのテスト計画を日本語で整理してください。\n"
    )
}

fn required_artifacts(changed: &[String]) -> (bool, bool, bool) {
    let mut by_change: HashMap<String, (bool, bool, bool)> = HashMap::new();
    for path in changed {
        if let Some(rest) = path.strip_prefix("docs/sdd/changes/") {
            let mut parts = rest.split('/');
            if let Some(change_dir) = parts.next() {
                let entry = by_change
                    .entry(change_dir.to_string())
                    .or_insert((false, false, false));
                if path.ends_with("/90_decision.md") {
                    entry.0 = true;
                }
                if path.ends_with("/40_tasks.md") {
                    entry.1 = true;
                }
                if path.ends_with("/50_test_plan.md") {
                    entry.2 = true;
                }
            }
        }
    }
    let mut decision = false;
    let mut tasks = false;
    let mut test_plan = false;
    for (_, (d, t, tp)) in by_change {
        if d && t && tp {
            decision = true;
            tasks = true;
            test_plan = true;
            break;
        }
    }
    (decision, tasks, test_plan)
}

fn resolve_base_ref(repo_root: &Path, requested: Option<&str>) -> Result<String> {
    if let Some(base) = requested {
        return Ok(base.to_string());
    }
    let default = "origin/main";
    if crate::git::worktree::ensure_base_ref(repo_root, default).is_ok() {
        return Ok(default.to_string());
    }
    Ok("HEAD~1".to_string())
}

fn task_completion_ratio(path: &Path) -> f64 {
    if let Ok(contents) = read_to_string(path) {
        let total = contents.matches("- [").count();
        if total == 0 {
            return 0.0;
        }
        let done = contents.matches("- [x]").count();
        return done as f64 / total as f64;
    }
    0.0
}

fn detect_risk(path: &Path) -> bool {
    if let Ok(contents) = read_to_string(path) {
        let lower = contents.to_lowercase();
        return lower.contains("high") || lower.contains("重大") || lower.contains("critical");
    }
    false
}
