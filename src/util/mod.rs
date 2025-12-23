use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
struct LogEvent<'a> {
    ts: &'a str,
    level: &'a str,
    message: &'a str,
}

pub fn log_event(level: &str, message: &str) {
    let ts = Utc::now().to_rfc3339();
    let event = LogEvent {
        ts: &ts,
        level,
        message,
    };
    if let Ok(line) = serde_json::to_string(&event) {
        eprintln!("{line}");
    } else {
        eprintln!("{{\"ts\":\"{ts}\",\"level\":\"{level}\",\"message\":\"{message}\"}}");
    }
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("create dir {}", path.display()))
}

pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

pub fn write_file_if_missing(path: &Path, contents: &str) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    write_file(path, contents)?;
    Ok(true)
}

pub fn run_cmd_allow_fail(mut cmd: Command) -> Result<Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().with_context(|| "run command")?;
    Ok(output)
}

pub fn write_string(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let mut file = fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
    file.write_all(contents.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in name.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "change".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn normalize_path(path: &Path) -> Result<String> {
    let s = path
        .to_str()
        .ok_or_else(|| anyhow!("invalid utf-8 path: {}", path.display()))?;
    Ok(s.replace('\\', "/"))
}

pub fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}
