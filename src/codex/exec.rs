use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::util::{run_cmd_allow_fail, write_string};

#[derive(Clone, Debug)]
pub struct ExecSpec {
    pub cwd: PathBuf,
    pub prompt_path: PathBuf,
    pub output_path: PathBuf,
    pub json_output_path: Option<PathBuf>,
    pub sandbox: String,
    pub schema_path: Option<PathBuf>,
}

pub struct ExecResult {
    pub status_ok: bool,
}

pub fn run(spec: &ExecSpec) -> Result<ExecResult> {
    let prompt_flag =
        env::var("CODEX_SDD_PROMPT_FLAG").unwrap_or_else(|_| "--prompt-file".to_string());
    let extra_args = env::var("CODEX_SDD_EXEC_ARGS").unwrap_or_default();

    let mut cmd = Command::new("codex");
    cmd.arg("exec")
        .arg("--sandbox")
        .arg(&spec.sandbox)
        .arg("--cd")
        .arg(&spec.cwd)
        .arg("--output-last-message")
        .arg(&spec.output_path)
        .arg(&prompt_flag)
        .arg(&spec.prompt_path);

    if let Some(schema) = &spec.schema_path {
        cmd.arg("--output-schema").arg(schema);
    }

    if spec.json_output_path.is_some() {
        cmd.arg("--json");
    }

    if !extra_args.trim().is_empty() {
        for part in extra_args.split_whitespace() {
            cmd.arg(part);
        }
    }

    let output = run_cmd_allow_fail(cmd).with_context(|| "codex exec")?;
    if let Some(json_path) = &spec.json_output_path {
        let jsonl = String::from_utf8_lossy(&output.stdout).to_string();
        if !jsonl.is_empty() {
            write_string(json_path, &jsonl)?;
        }
    }

    Ok(ExecResult {
        status_ok: output.status.success(),
    })
}

pub fn output_paths(runs_dir: &Path, change_id: &str, name: &str) -> (PathBuf, PathBuf) {
    let change_dir = runs_dir.join(change_id);
    let output_path = change_dir.join(format!("{name}.md"));
    let json_path = change_dir.join(format!("{name}.jsonl"));
    (output_path, json_path)
}
