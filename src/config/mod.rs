mod schema;

pub use schema::{should_launch, AppRule, Config, Shortcut};

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
        ["settings", "launch"] => {
            config.settings.launch = parse_bool(value)?;
        }
        ["settings", "animate"] => {
            config.settings.animate = parse_bool(value)?;
        }
        ["settings", "fuzzy_threshold"] => {
            config.settings.fuzzy_threshold = value
                .parse()
                .with_context(|| format!("Invalid number: {}", value))?;
        }
        ["settings", "delay_ms"] => {
            config.settings.delay_ms = value
                .parse()
                .with_context(|| format!("Invalid number: {}", value))?;
        }
        ["settings", "retry", "count"] => {
            config.settings.retry.count = value
                .parse()
                .with_context(|| format!("Invalid number: {}", value))?;
        }
        ["settings", "retry", "delay_ms"] => {
            config.settings.retry.delay_ms = value
                .parse()
                .with_context(|| format!("Invalid number: {}", value))?;
        }
        ["settings", "retry", "backoff"] => {
            config.settings.retry.backoff = value
                .parse()
                .with_context(|| format!("Invalid number: {}", value))?;
        }
        _ => {
            return Err(anyhow!(
                "Unknown config key: {}. Valid keys: settings.launch, settings.animate, settings.fuzzy_threshold, settings.delay_ms, settings.retry.count, settings.retry.delay_ms, settings.retry.backoff",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool_true_values() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("True").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("YES").unwrap());
        assert!(parse_bool("on").unwrap());
        assert!(parse_bool("ON").unwrap());
    }

    #[test]
    fn test_parse_bool_false_values() {
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(!parse_bool("False").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("NO").unwrap());
        assert!(!parse_bool("off").unwrap());
        assert!(!parse_bool("OFF").unwrap());
    }

    #[test]
    fn test_parse_bool_invalid() {
        assert!(parse_bool("").is_err());
        assert!(parse_bool("maybe").is_err());
        assert!(parse_bool("2").is_err());
        assert!(parse_bool("yep").is_err());
    }

    #[test]
    fn test_set_value_launch() {
        let mut config = Config::default();
        assert!(!config.settings.launch);

        set_value(&mut config, "settings.launch", "true").unwrap();
        assert!(config.settings.launch);

        set_value(&mut config, "settings.launch", "false").unwrap();
        assert!(!config.settings.launch);
    }

    #[test]
    fn test_set_value_animate() {
        let mut config = Config::default();
        assert!(!config.settings.animate);

        set_value(&mut config, "settings.animate", "yes").unwrap();
        assert!(config.settings.animate);

        set_value(&mut config, "settings.animate", "no").unwrap();
        assert!(!config.settings.animate);
    }

    #[test]
    fn test_set_value_fuzzy_threshold() {
        let mut config = Config::default();
        assert_eq!(config.settings.fuzzy_threshold, 2);

        set_value(&mut config, "settings.fuzzy_threshold", "5").unwrap();
        assert_eq!(config.settings.fuzzy_threshold, 5);

        set_value(&mut config, "settings.fuzzy_threshold", "0").unwrap();
        assert_eq!(config.settings.fuzzy_threshold, 0);
    }

    #[test]
    fn test_set_value_delay_ms() {
        let mut config = Config::default();
        assert_eq!(config.settings.delay_ms, 500);

        set_value(&mut config, "settings.delay_ms", "1000").unwrap();
        assert_eq!(config.settings.delay_ms, 1000);
    }

    #[test]
    fn test_set_value_retry_count() {
        let mut config = Config::default();
        assert_eq!(config.settings.retry.count, 10);

        set_value(&mut config, "settings.retry.count", "5").unwrap();
        assert_eq!(config.settings.retry.count, 5);
    }

    #[test]
    fn test_set_value_retry_delay_ms() {
        let mut config = Config::default();
        assert_eq!(config.settings.retry.delay_ms, 100);

        set_value(&mut config, "settings.retry.delay_ms", "200").unwrap();
        assert_eq!(config.settings.retry.delay_ms, 200);
    }

    #[test]
    fn test_set_value_retry_backoff() {
        let mut config = Config::default();
        assert_eq!(config.settings.retry.backoff, 1.5);

        set_value(&mut config, "settings.retry.backoff", "2.0").unwrap();
        assert_eq!(config.settings.retry.backoff, 2.0);
    }

    #[test]
    fn test_set_value_invalid_key() {
        let mut config = Config::default();
        assert!(set_value(&mut config, "invalid.key", "true").is_err());
        assert!(set_value(&mut config, "settings", "true").is_err());
        assert!(set_value(&mut config, "settings.unknown", "true").is_err());
    }

    #[test]
    fn test_set_value_invalid_value() {
        let mut config = Config::default();
        assert!(set_value(&mut config, "settings.animate", "maybe").is_err());
        assert!(set_value(&mut config, "settings.fuzzy_threshold", "abc").is_err());
    }
}
