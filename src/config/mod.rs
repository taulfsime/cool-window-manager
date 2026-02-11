mod schema;

pub use schema::{
    should_launch, AppRule, AutoUpdateMode, Config, Settings, Shortcut, TelemetrySettings,
    UpdateFrequency, UpdateSettings,
};

use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::daemon::hotkeys::Hotkey;

const CONFIG_ENV_VAR: &str = "CWM_CONFIG";

pub fn get_config_path() -> PathBuf {
    if let Ok(path) = env::var(CONFIG_ENV_VAR) {
        return PathBuf::from(path);
    }

    // new location: ~/.cwm/config.json
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".cwm")
        .join("config.json")
}

pub fn ensure_cwm_dir() -> Result<PathBuf> {
    let cwm_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?
        .join(".cwm");

    if !cwm_dir.exists() {
        fs::create_dir_all(&cwm_dir)?;
    }

    Ok(cwm_dir)
}

pub fn load() -> Result<Config> {
    let path = get_config_path();

    if !path.exists() {
        // ensure directory exists
        ensure_cwm_dir()?;
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

    // ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&path, content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

/// Verify configuration file and return a list of errors
pub fn verify(path: &Path) -> Result<Vec<String>> {
    let mut errors = Vec::new();

    if !path.exists() {
        return Err(anyhow!("config file not found: {}", path.display()));
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;

    let config: Config = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            return Err(anyhow!("invalid JSON: {}", e));
        }
    };

    // validate shortcuts
    for (i, shortcut) in config.shortcuts.iter().enumerate() {
        let prefix = format!("shortcuts[{}]", i);

        // validate hotkey format
        if let Err(e) = Hotkey::parse(&shortcut.keys) {
            errors.push(format!(
                "{}: invalid keys '{}': {}",
                prefix, shortcut.keys, e
            ));
        }

        // validate action
        if let Err(e) = validate_action(&shortcut.action) {
            errors.push(format!("{}: {}", prefix, e));
        }

        // focus requires app
        if shortcut.action == "focus" && shortcut.app.is_none() {
            errors.push(format!("{}: action 'focus' requires 'app' field", prefix));
        }
    }

    // validate app_rules
    for (i, rule) in config.app_rules.iter().enumerate() {
        let prefix = format!("app_rules[{}]", i);

        // validate action
        if let Err(e) = validate_action(&rule.action) {
            errors.push(format!("{}: {}", prefix, e));
        }
    }

    Ok(errors)
}

fn validate_action(action: &str) -> Result<(), String> {
    let valid_base_actions = ["focus", "maximize"];

    if valid_base_actions.contains(&action) {
        return Ok(());
    }

    if let Some(arg) = action.strip_prefix("move_display:") {
        if arg.is_empty() {
            return Err(
                "action 'move_display' requires a target (next, prev, or number)".to_string(),
            );
        }
        // validate target
        if arg != "next" && arg != "prev" && arg.parse::<u32>().is_err() {
            return Err(format!(
                "invalid move_display target '{}': use 'next', 'prev', or a number",
                arg
            ));
        }
        return Ok(());
    }

    if let Some(arg) = action.strip_prefix("resize:") {
        if arg.is_empty() {
            return Err("action 'resize' requires a size (1-100 or 'full')".to_string());
        }
        if !arg.eq_ignore_ascii_case("full") {
            match arg.parse::<u32>() {
                Ok(n) if (1..=100).contains(&n) => {}
                Ok(n) => {
                    return Err(format!("resize size {} out of range (1-100)", n));
                }
                Err(_) => {
                    return Err(format!(
                        "invalid resize size '{}': use a number 1-100 or 'full'",
                        arg
                    ));
                }
            }
        }
        return Ok(());
    }

    Err(format!(
        "invalid action '{}': valid actions are focus, maximize, move_display:<target>, resize:<size>",
        action
    ))
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
        ["settings", "update", "enabled"] => {
            config.settings.update.enabled = parse_bool(value)?;
        }
        ["settings", "update", "check_frequency"] => {
            config.settings.update.check_frequency = match value.to_lowercase().as_str() {
                "daily" => UpdateFrequency::Daily,
                "weekly" => UpdateFrequency::Weekly,
                "manual" => UpdateFrequency::Manual,
                _ => {
                    return Err(anyhow!(
                        "Invalid check_frequency: {}. Use daily, weekly, or manual",
                        value
                    ))
                }
            };
        }
        ["settings", "update", "auto_update"] => {
            config.settings.update.auto_update = match value.to_lowercase().as_str() {
                "always" => AutoUpdateMode::Always,
                "prompt" => AutoUpdateMode::Prompt,
                "never" => AutoUpdateMode::Never,
                _ => {
                    return Err(anyhow!(
                        "Invalid auto_update: {}. Use always, prompt, or never",
                        value
                    ))
                }
            };
        }
        ["settings", "update", "channels", "dev"] => {
            config.settings.update.channels.dev = parse_bool(value)?;
        }
        ["settings", "update", "channels", "beta"] => {
            config.settings.update.channels.beta = parse_bool(value)?;
        }
        ["settings", "update", "channels", "stable"] => {
            config.settings.update.channels.stable = parse_bool(value)?;
        }
        ["settings", "update", "telemetry", "enabled"] => {
            config.settings.update.telemetry.enabled = parse_bool(value)?;
        }
        ["settings", "update", "telemetry", "include_system_info"] => {
            config.settings.update.telemetry.include_system_info = parse_bool(value)?;
        }
        _ => {
            return Err(anyhow!(
                "Unknown config key: {}. Valid keys include: settings.launch, settings.animate, settings.fuzzy_threshold, settings.update.enabled, settings.update.channels.stable, etc.",
                key
            ));
        }
    }

    Ok(())
}

/// generates a default config with example shortcuts and rules
pub fn default_with_examples() -> Config {
    Config {
        shortcuts: vec![
            Shortcut {
                keys: "ctrl+alt+s".to_string(),
                action: "focus".to_string(),
                app: Some("Slack".to_string()),
                launch: Some(true),
            },
            Shortcut {
                keys: "ctrl+alt+t".to_string(),
                action: "focus".to_string(),
                app: Some("Terminal".to_string()),
                launch: None,
            },
            // match by window title instead of app name
            Shortcut {
                keys: "ctrl+alt+g".to_string(),
                action: "focus".to_string(),
                app: Some("GitHub".to_string()),
                launch: None,
            },
            Shortcut {
                keys: "ctrl+alt+m".to_string(),
                action: "maximize".to_string(),
                app: None,
                launch: None,
            },
            Shortcut {
                keys: "ctrl+alt+right".to_string(),
                action: "move_display:next".to_string(),
                app: None,
                launch: None,
            },
            Shortcut {
                keys: "ctrl+alt+8".to_string(),
                action: "resize:80".to_string(),
                app: None,
                launch: None,
            },
        ],
        app_rules: vec![AppRule {
            app: "Terminal".to_string(),
            action: "maximize".to_string(),
            delay_ms: Some(500),
        }],
        settings: Settings::default(),
    }
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

    #[test]
    fn test_validate_action_valid() {
        assert!(validate_action("focus").is_ok());
        assert!(validate_action("maximize").is_ok());
        assert!(validate_action("move_display:next").is_ok());
        assert!(validate_action("move_display:prev").is_ok());
        assert!(validate_action("move_display:0").is_ok());
        assert!(validate_action("move_display:2").is_ok());
        assert!(validate_action("resize:50").is_ok());
        assert!(validate_action("resize:100").is_ok());
        assert!(validate_action("resize:1").is_ok());
        assert!(validate_action("resize:full").is_ok());
        assert!(validate_action("resize:FULL").is_ok());
    }

    #[test]
    fn test_validate_action_invalid() {
        assert!(validate_action("unknown").is_err());
        assert!(validate_action("move_display:").is_err());
        assert!(validate_action("move_display:invalid").is_err());
        assert!(validate_action("resize:").is_err());
        assert!(validate_action("resize:0").is_err());
        assert!(validate_action("resize:101").is_err());
        assert!(validate_action("resize:abc").is_err());
    }

    #[test]
    fn test_verify_valid_config() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_valid.json");

        let config = r#"{
            "shortcuts": [
                {"keys": "ctrl+alt+s", "action": "focus", "app": "Slack"},
                {"keys": "ctrl+alt+m", "action": "maximize"},
                {"keys": "ctrl+alt+r", "action": "resize:80"},
                {"keys": "ctrl+alt+d", "action": "move_display:next"}
            ],
            "app_rules": [
                {"app": "Terminal", "action": "maximize"}
            ],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_verify_invalid_hotkey() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_invalid_hotkey.json");

        let config = r#"{
            "shortcuts": [
                {"keys": "ctrl+alt", "action": "maximize"}
            ],
            "app_rules": [],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("invalid keys"));
        assert!(errors[0].contains("No key specified"));
    }

    #[test]
    fn test_verify_focus_without_app() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_focus_no_app.json");

        let config = r#"{
            "shortcuts": [
                {"keys": "ctrl+alt+f", "action": "focus"}
            ],
            "app_rules": [],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("focus"));
        assert!(errors[0].contains("requires 'app' field"));
    }

    #[test]
    fn test_verify_invalid_action() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_invalid_action.json");

        let config = r#"{
            "shortcuts": [
                {"keys": "ctrl+alt+x", "action": "unknown_action"}
            ],
            "app_rules": [],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("invalid action"));
    }

    #[test]
    fn test_verify_invalid_resize() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_invalid_resize.json");

        let config = r#"{
            "shortcuts": [
                {"keys": "ctrl+alt+1", "action": "resize:0"},
                {"keys": "ctrl+alt+2", "action": "resize:101"},
                {"keys": "ctrl+alt+3", "action": "resize:abc"}
            ],
            "app_rules": [],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(errors.len(), 3);
        assert!(errors[0].contains("out of range"));
        assert!(errors[1].contains("out of range"));
        assert!(errors[2].contains("invalid resize size"));
    }

    #[test]
    fn test_verify_invalid_move_display() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_invalid_move.json");

        let config = r#"{
            "shortcuts": [
                {"keys": "ctrl+alt+1", "action": "move_display:"},
                {"keys": "ctrl+alt+2", "action": "move_display:invalid"}
            ],
            "app_rules": [],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("requires a target"));
        assert!(errors[1].contains("invalid move_display target"));
    }

    #[test]
    fn test_verify_invalid_app_rule() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_invalid_rule.json");

        let config = r#"{
            "shortcuts": [],
            "app_rules": [
                {"app": "Terminal", "action": "bad_action"}
            ],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("app_rules[0]"));
        assert!(errors[0].contains("invalid action"));
    }

    #[test]
    fn test_verify_multiple_errors() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_multiple_errors.json");

        let config = r#"{
            "shortcuts": [
                {"keys": "ctrl+alt", "action": "focus", "app": "Slack"},
                {"keys": "ctrl+alt+f", "action": "focus"},
                {"keys": "ctrl+alt+x", "action": "bad"}
            ],
            "app_rules": [
                {"app": "Terminal", "action": "invalid"}
            ],
            "settings": {}
        }"#;

        std::fs::write(&path, config).unwrap();
        let errors = verify(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(errors.len(), 4);
    }

    #[test]
    fn test_verify_file_not_found() {
        let path = PathBuf::from("/nonexistent/path/config.json");
        let result = verify(&path);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_verify_invalid_json() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_invalid_json.json");

        std::fs::write(&path, "{ invalid json }").unwrap();
        let result = verify(&path);
        std::fs::remove_file(&path).ok();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid JSON"));
    }
}
