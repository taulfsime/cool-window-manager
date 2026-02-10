mod schema;

pub use schema::{should_launch, Config, Shortcut};

#[allow(unused_imports)]
pub use schema::{Behavior, Matching};

use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

const CONFIG_ENV_VAR: &str = "CWM_CONFIG";
const DEFAULT_CONFIG_FILE: &str = ".cwm.json";

pub fn get_config_path() -> PathBuf {
    if let Ok(path) = env::var(CONFIG_ENV_VAR) {
        return PathBuf::from(path);
    }

    dirs::home_dir()
        .map(|home| home.join(DEFAULT_CONFIG_FILE))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE))
}

pub fn load() -> Result<Config> {
    let path = get_config_path();

    if !path.exists() {
        let config = Config::default();
        save(&config)?;
        return Ok(config);
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = get_config_path();

    let content = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;

    fs::write(&path, content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

pub fn set_value(config: &mut Config, key: &str, value: &str) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();

    match parts.as_slice() {
        ["behavior", "launch_if_not_running"] => {
            config.behavior.launch_if_not_running = parse_bool(value)?;
        }
        ["behavior", "animate"] => {
            config.behavior.animate = parse_bool(value)?;
        }
        ["matching", "fuzzy_threshold"] => {
            config.matching.fuzzy_threshold = value
                .parse()
                .with_context(|| format!("Invalid number: {}", value))?;
        }
        _ => {
            return Err(anyhow!(
                "Unknown config key: {}. Valid keys: behavior.launch_if_not_running, behavior.animate, matching.fuzzy_threshold",
                key
            ));
        }
    }

    Ok(())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(anyhow!(
            "Invalid boolean value: {}. Use true/false, yes/no, 1/0, or on/off",
            value
        )),
    }
}


