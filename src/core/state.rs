use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::util::{now_rfc3339, write_string};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub schema_version: u32,
    pub tool_version: String,
    pub active_change_id: Option<String>,
    #[serde(default)]
    pub changes: HashMap<String, ChangeState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeState {
    pub approved: bool,
    pub approved_at: Option<String>,
    pub approved_by: Option<String>,
    pub file_index_hash: Option<String>,
    pub file_index_generated_at: Option<String>,
    #[serde(default)]
    pub codex_threads: Vec<CodexThread>,
    #[serde(default)]
    pub file_hashes: HashMap<String, String>,
    #[serde(default)]
    pub reader_shard_hashes: HashMap<String, String>,
    #[serde(default)]
    pub base_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexThread {
    pub purpose: String,
    pub thread_id: String,
    pub started_at: String,
}

impl State {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut state: State = serde_json::from_str(&data).with_context(|| "parse state.json")?;
        if state.schema_version == 0 {
            state.schema_version = SCHEMA_VERSION;
        }
        if state.schema_version != SCHEMA_VERSION {
            return Err(anyhow!(
                "unsupported state schema version {}",
                state.schema_version
            ));
        }
        if state.tool_version.is_empty() {
            state.tool_version = env!("CARGO_PKG_VERSION").to_string();
        }
        Ok(state)
    }

    pub fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            active_change_id: None,
            changes: HashMap::new(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let data = serde_json::to_string_pretty(self).with_context(|| "serialize state")?;
        write_string(path, &data)
    }

    pub fn change_state_mut(&mut self, change_id: &str) -> &mut ChangeState {
        self.changes
            .entry(change_id.to_string())
            .or_insert_with(ChangeState::default)
    }

    pub fn change_state(&self, change_id: &str) -> Option<&ChangeState> {
        self.changes.get(change_id)
    }

    pub fn require_approved(&self, change_id: &str) -> Result<()> {
        let state = self
            .changes
            .get(change_id)
            .ok_or_else(|| anyhow!("change {change_id} not found"))?;
        if !state.approved {
            return Err(anyhow!("approval required for change {change_id}"));
        }
        Ok(())
    }

    pub fn approve_change(&mut self, change_id: &str, approved_by: &str) {
        let state = self.change_state_mut(change_id);
        state.approved = true;
        state.approved_at = Some(now_rfc3339());
        state.approved_by = Some(approved_by.to_string());
    }

    pub fn record_thread(&mut self, change_id: &str, purpose: &str, thread_id: &str) {
        let state = self.change_state_mut(change_id);
        state.codex_threads.push(CodexThread {
            purpose: purpose.to_string(),
            thread_id: thread_id.to_string(),
            started_at: now_rfc3339(),
        });
    }
}
