use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::util::run_cmd_allow_fail;

#[derive(Debug, Clone)]
pub struct TestResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_tests(repo_root: &Path) -> Result<TestResult> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(repo_root).arg("test");
    let output = run_cmd_allow_fail(cmd)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(TestResult {
        success: output.status.success(),
        stdout,
        stderr,
    })
}

pub fn ensure_success(result: &TestResult) -> Result<()> {
    if result.success {
        Ok(())
    } else {
        Err(anyhow!("tests failed"))
    }
}
