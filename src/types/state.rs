use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Sync state (.wald/state.yaml, gitignored)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncState {
    /// Last sync commit hash
    #[serde(default)]
    pub last_sync: Option<String>,
}

impl SyncState {
    /// Load state from a YAML file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read state: {}", path.display()))?;
        let state: SyncState = serde_yml::from_str(&content)
            .with_context(|| format!("failed to parse state: {}", path.display()))?;
        Ok(state)
    }

    /// Save state to a YAML file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yml::to_string(self).context("failed to serialize state")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write state: {}", path.display()))?;
        Ok(())
    }

    /// Update last sync to a new commit
    pub fn update_last_sync(&mut self, commit: &str) {
        self.last_sync = Some(commit.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = SyncState::default();
        assert!(state.last_sync.is_none());
    }

    #[test]
    fn test_update_last_sync() {
        let mut state = SyncState::default();
        state.update_last_sync("abc123");
        assert_eq!(state.last_sync, Some("abc123".to_string()));
    }

    #[test]
    fn test_state_roundtrip() {
        let state = SyncState {
            last_sync: Some("def456".to_string()),
        };

        let yaml = serde_yml::to_string(&state).unwrap();
        let parsed: SyncState = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(parsed.last_sync, Some("def456".to_string()));
    }
}
