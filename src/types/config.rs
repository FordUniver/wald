use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{DepthPolicy, LfsPolicy};

/// Workspace configuration (.wald/config.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default LFS policy for new repos
    #[serde(default)]
    pub default_lfs: LfsPolicy,

    /// Default clone depth for new repos
    #[serde(default)]
    pub default_depth: DepthPolicy,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_lfs: LfsPolicy::Minimal,
            default_depth: DepthPolicy::Depth(100),
        }
    }
}

impl Config {
    /// Load config from a YAML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let config: Config = serde_yml::from_str(&content)
            .with_context(|| format!("failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    /// Save config to a YAML file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yml::to_string(self).context("failed to serialize config")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write config: {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.default_lfs, LfsPolicy::Minimal);
        assert_eq!(config.default_depth, DepthPolicy::Depth(100));
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config {
            default_lfs: LfsPolicy::Full,
            default_depth: DepthPolicy::Depth(50),
        };

        let yaml = serde_yml::to_string(&config).unwrap();
        let parsed: Config = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(parsed.default_lfs, LfsPolicy::Full);
        assert_eq!(parsed.default_depth, DepthPolicy::Depth(50));
    }
}
