use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;

/// Configuration loaded from the TOML config file.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub api_key: String,
}

/// Returns the path to the config file, using `dirs::config_dir()` for cross-platform support.
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
            PathBuf::from(home).join(".config")
        })
        .join("mcp-server-grok-chat")
        .join("config.toml")
}

/// Load and validate the config file.
pub fn load() -> Result<Config> {
    let path = config_path();
    let content = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "Failed to read config file: {}\n\
             Create it with your xAI API key.\n\
             Example:\n\n\
             api_key = \"xai-...\"",
            path.display()
        )
    })?;
    let config: Config =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;

    if config.api_key.trim().is_empty() {
        bail!(
            "api_key in {} is empty — set it to your xAI API key",
            path.display()
        );
    }

    Ok(config)
}
