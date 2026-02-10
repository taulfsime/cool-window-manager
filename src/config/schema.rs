use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub shortcuts: Vec<Shortcut>,
    #[serde(default)]
    pub app_rules: Vec<AppRule>,
    pub settings: Settings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shortcuts: vec![],
            app_rules: vec![],
            settings: Settings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub keys: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<bool>,
}

/// rule to apply an action when an app launches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub app: String,
    pub action: String,
    /// delay in milliseconds before executing the action (overrides global)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
}

pub const DEFAULT_DELAY_MS: u64 = 500;
pub const DEFAULT_RETRY_COUNT: u32 = 10;
pub const DEFAULT_RETRY_DELAY_MS: u64 = 100;
pub const DEFAULT_RETRY_BACKOFF: f64 = 1.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_fuzzy_threshold")]
    pub fuzzy_threshold: usize,
    #[serde(default)]
    pub launch: bool,
    #[serde(default)]
    pub animate: bool,
    /// default delay in milliseconds before executing app rule actions
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
    #[serde(default)]
    pub retry: Retry,
}

fn default_fuzzy_threshold() -> usize {
    2
}

fn default_delay_ms() -> u64 {
    DEFAULT_DELAY_MS
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            fuzzy_threshold: 2,
            launch: false,
            animate: false,
            delay_ms: DEFAULT_DELAY_MS,
            retry: Retry::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retry {
    #[serde(default = "default_retry_count")]
    pub count: u32,
    #[serde(default = "default_retry_delay_ms")]
    pub delay_ms: u64,
    #[serde(default = "default_retry_backoff")]
    pub backoff: f64,
}

fn default_retry_count() -> u32 {
    DEFAULT_RETRY_COUNT
}

fn default_retry_delay_ms() -> u64 {
    DEFAULT_RETRY_DELAY_MS
}

fn default_retry_backoff() -> f64 {
    DEFAULT_RETRY_BACKOFF
}

impl Default for Retry {
    fn default() -> Self {
        Self {
            count: DEFAULT_RETRY_COUNT,
            delay_ms: DEFAULT_RETRY_DELAY_MS,
            backoff: DEFAULT_RETRY_BACKOFF,
        }
    }
}

/// determines if an app should be launched based on CLI flags, shortcut config, and global config
pub fn should_launch(
    cli_launch: bool,
    cli_no_launch: bool,
    shortcut_launch: Option<bool>,
    global_launch: bool,
) -> bool {
    if cli_launch {
        return true;
    }
    if cli_no_launch {
        return false;
    }
    shortcut_launch.unwrap_or(global_launch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_launch_cli_override() {
        // CLI --launch always wins
        assert!(should_launch(true, false, Some(false), false));
        assert!(should_launch(true, false, None, false));

        // CLI --no-launch always wins
        assert!(!should_launch(false, true, Some(true), true));
        assert!(!should_launch(false, true, None, true));
    }

    #[test]
    fn test_should_launch_shortcut_override() {
        // shortcut setting overrides global
        assert!(should_launch(false, false, Some(true), false));
        assert!(!should_launch(false, false, Some(false), true));
    }

    #[test]
    fn test_should_launch_global_fallback() {
        // falls back to global when no overrides
        assert!(should_launch(false, false, None, true));
        assert!(!should_launch(false, false, None, false));
    }

    #[test]
    fn test_retry_default() {
        let retry = Retry::default();
        assert_eq!(retry.count, DEFAULT_RETRY_COUNT);
        assert_eq!(retry.delay_ms, DEFAULT_RETRY_DELAY_MS);
        assert_eq!(retry.backoff, DEFAULT_RETRY_BACKOFF);
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.fuzzy_threshold, 2);
        assert!(!settings.launch);
        assert!(!settings.animate);
        assert_eq!(settings.delay_ms, DEFAULT_DELAY_MS);
        assert_eq!(settings.retry.count, DEFAULT_RETRY_COUNT);
        assert_eq!(settings.retry.delay_ms, DEFAULT_RETRY_DELAY_MS);
        assert_eq!(settings.retry.backoff, DEFAULT_RETRY_BACKOFF);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.shortcuts.is_empty());
        assert!(config.app_rules.is_empty());
        assert_eq!(config.settings.fuzzy_threshold, 2);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut config = Config::default();
        config.shortcuts.push(Shortcut {
            keys: "ctrl+alt+s".to_string(),
            action: "focus".to_string(),
            app: Some("Slack".to_string()),
            launch: Some(true),
        });
        config.app_rules.push(AppRule {
            app: "Terminal".to_string(),
            action: "maximize".to_string(),
            delay_ms: Some(1000),
        });
        config.settings.launch = true;
        config.settings.retry.count = 5;

        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.shortcuts.len(), 1);
        assert_eq!(parsed.shortcuts[0].keys, "ctrl+alt+s");
        assert_eq!(parsed.shortcuts[0].launch, Some(true));
        assert_eq!(parsed.app_rules.len(), 1);
        assert_eq!(parsed.app_rules[0].delay_ms, Some(1000));
        assert!(parsed.settings.launch);
        assert_eq!(parsed.settings.retry.count, 5);
    }

    #[test]
    fn test_partial_settings_uses_defaults() {
        // settings with only some fields should use defaults for others
        let json = r#"{
            "shortcuts": [],
            "app_rules": [],
            "settings": {
                "launch": true
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert!(config.settings.launch);
        assert_eq!(config.settings.fuzzy_threshold, 2);
        assert!(!config.settings.animate);
        assert_eq!(config.settings.delay_ms, DEFAULT_DELAY_MS);
        assert_eq!(config.settings.retry.count, DEFAULT_RETRY_COUNT);
    }

    #[test]
    fn test_partial_retry_uses_defaults() {
        // retry with only some fields should use defaults for others
        let json = r#"{
            "shortcuts": [],
            "app_rules": [],
            "settings": {
                "retry": {
                    "count": 5
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(config.settings.retry.count, 5);
        assert_eq!(config.settings.retry.delay_ms, DEFAULT_RETRY_DELAY_MS);
        assert_eq!(config.settings.retry.backoff, DEFAULT_RETRY_BACKOFF);
    }

    #[test]
    fn test_shortcut_without_optional_fields() {
        let json = r#"{
            "shortcuts": [
                {
                    "keys": "ctrl+alt+m",
                    "action": "maximize"
                }
            ],
            "app_rules": [],
            "settings": {}
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(config.shortcuts.len(), 1);
        assert_eq!(config.shortcuts[0].keys, "ctrl+alt+m");
        assert_eq!(config.shortcuts[0].action, "maximize");
        assert!(config.shortcuts[0].app.is_none());
        assert!(config.shortcuts[0].launch.is_none());
    }

    #[test]
    fn test_app_rule_without_optional_fields() {
        let json = r#"{
            "shortcuts": [],
            "app_rules": [
                {
                    "app": "Safari",
                    "action": "maximize"
                }
            ],
            "settings": {}
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(config.app_rules.len(), 1);
        assert_eq!(config.app_rules[0].app, "Safari");
        assert_eq!(config.app_rules[0].action, "maximize");
        assert!(config.app_rules[0].delay_ms.is_none());
    }
}
