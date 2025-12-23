use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::util::run_cmd_allow_fail;

#[derive(Debug, Clone)]
pub struct CoverageResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub percent: Option<f64>,
}

pub fn run_llvm_cov(repo_root: &Path) -> Result<CoverageResult> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(repo_root).args(["llvm-cov", "--summary"]);
    let output = run_cmd_allow_fail(cmd)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let percent = parse_percent(&stdout);
    Ok(CoverageResult {
        success: output.status.success(),
        stdout,
        stderr,
        percent,
    })
}

pub fn run_tarpaulin(repo_root: &Path) -> Result<CoverageResult> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(repo_root).args(["tarpaulin", "--quiet"]);
    let output = run_cmd_allow_fail(cmd)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let percent = parse_percent(&stdout);
    Ok(CoverageResult {
        success: output.status.success(),
        stdout,
        stderr,
        percent,
    })
}

fn parse_percent(output: &str) -> Option<f64> {
    for token in output.split_whitespace() {
        if let Some(stripped) = token.strip_suffix('%') {
            if let Ok(val) = stripped.parse::<f64>() {
                return Some(val);
            }
        }
    }
    None
}

pub fn ensure_success(result: &CoverageResult) -> Result<()> {
    if result.success {
        Ok(())
    } else {
        Err(anyhow!("coverage failed"))
    }
}
