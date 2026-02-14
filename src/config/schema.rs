use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_SCHEMA_REF: &str = "./config.schema.json";

pub type DisplayAliases = HashMap<String, Vec<String>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(
        rename = "$schema",
        default = "default_schema",
        skip_serializing_if = "Option::is_none"
    )]
    pub schema: Option<String>,
    #[serde(default)]
    pub shortcuts: Vec<Shortcut>,
    #[serde(default)]
    pub app_rules: Vec<AppRule>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub spotlight: Vec<SpotlightShortcut>,
    #[serde(default)]
    pub display_aliases: DisplayAliases,
}

fn default_schema() -> Option<String> {
    Some(DEFAULT_SCHEMA_REF.to_string())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: Some(DEFAULT_SCHEMA_REF.to_string()),
            shortcuts: Vec::new(),
            app_rules: Vec::new(),
            settings: Settings::default(),
            spotlight: Vec::new(),
            display_aliases: DisplayAliases::new(),
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

/// spotlight shortcut that appears in macOS Spotlight search
/// uses the same action format as shortcuts: focus, maximize, move_display:next, resize:80
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotlightShortcut {
    /// name displayed in Spotlight (will be prefixed with "cwm: ")
    pub name: String,
    /// action in same format as shortcuts: focus, maximize, move_display:next, resize:80
    pub action: String,
    /// target application (required for focus, optional for others)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// launch app if not running
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<bool>,
    /// custom icon for the Spotlight shortcut
    /// can be: path to .icns file, path to .png file, or app name to extract icon from
    /// if not specified, uses target app's icon (if app is set) or default cwm icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl SpotlightShortcut {
    /// returns the full name with "cwm: " prefix
    pub fn display_name(&self) -> String {
        format!("cwm: {}", self.name)
    }

    /// returns a sanitized identifier for use in bundle IDs and filenames
    pub fn identifier(&self) -> String {
        self.name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
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
    #[serde(default)]
    pub update: UpdateSettings,
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
            update: UpdateSettings::default(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_daily")]
    pub check_frequency: UpdateFrequency,
    #[serde(default = "default_prompt")]
    pub auto_update: AutoUpdateMode,
    #[serde(default)]
    pub channels: UpdateChannels,
    #[serde(default)]
    pub telemetry: TelemetrySettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_check: Option<DateTime<Utc>>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            check_frequency: UpdateFrequency::Daily,
            auto_update: AutoUpdateMode::Prompt,
            channels: UpdateChannels::default(),
            telemetry: TelemetrySettings::default(),
            last_check: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChannels {
    #[serde(default)]
    pub dev: bool,
    #[serde(default)]
    pub beta: bool,
    #[serde(default = "default_true")]
    pub stable: bool,
}

impl Default for UpdateChannels {
    fn default() -> Self {
        Self {
            dev: false,
            beta: false,
            stable: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetrySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub include_system_info: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateFrequency {
    Daily,
    Weekly,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoUpdateMode {
    Always,
    Prompt,
    Never,
}

// default functions for serde
fn default_true() -> bool {
    true
}

fn default_daily() -> UpdateFrequency {
    UpdateFrequency::Daily
}

fn default_prompt() -> AutoUpdateMode {
    AutoUpdateMode::Prompt
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
        assert_eq!(config.schema, Some(DEFAULT_SCHEMA_REF.to_string()));
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

        assert_eq!(parsed.schema, Some(DEFAULT_SCHEMA_REF.to_string()));
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

    #[test]
    fn test_update_settings_defaults() {
        let settings = UpdateSettings::default();

        assert!(settings.enabled);
        assert!(matches!(settings.check_frequency, UpdateFrequency::Daily));
        assert!(matches!(settings.auto_update, AutoUpdateMode::Prompt));
        assert!(!settings.channels.dev);
        assert!(!settings.channels.beta);
        assert!(settings.channels.stable);
        assert!(!settings.telemetry.enabled);
        assert!(!settings.telemetry.include_system_info);
        assert!(settings.last_check.is_none());
    }

    #[test]
    fn test_update_settings_serialization() {
        let settings = UpdateSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let parsed: UpdateSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.enabled, settings.enabled);
        assert!(parsed.channels.stable);
    }

    #[test]
    fn test_update_channels_defaults() {
        let channels = UpdateChannels::default();

        assert!(!channels.dev);
        assert!(!channels.beta);
        assert!(channels.stable);
    }

    #[test]
    fn test_telemetry_settings_defaults() {
        let telemetry = TelemetrySettings::default();

        assert!(!telemetry.enabled);
        assert!(!telemetry.include_system_info);
    }

    #[test]
    fn test_update_frequency_serialization() {
        let daily = UpdateFrequency::Daily;
        let json = serde_json::to_string(&daily).unwrap();
        assert_eq!(json, "\"daily\"");

        let weekly = UpdateFrequency::Weekly;
        let json = serde_json::to_string(&weekly).unwrap();
        assert_eq!(json, "\"weekly\"");

        let manual = UpdateFrequency::Manual;
        let json = serde_json::to_string(&manual).unwrap();
        assert_eq!(json, "\"manual\"");
    }

    #[test]
    fn test_auto_update_mode_serialization() {
        let always = AutoUpdateMode::Always;
        let json = serde_json::to_string(&always).unwrap();
        assert_eq!(json, "\"always\"");

        let prompt = AutoUpdateMode::Prompt;
        let json = serde_json::to_string(&prompt).unwrap();
        assert_eq!(json, "\"prompt\"");

        let never = AutoUpdateMode::Never;
        let json = serde_json::to_string(&never).unwrap();
        assert_eq!(json, "\"never\"");
    }

    #[test]
    fn test_config_with_update_settings() {
        let json = r#"{
            "shortcuts": [],
            "app_rules": [],
            "settings": {
                "update": {
                    "enabled": true,
                    "check_frequency": "weekly",
                    "auto_update": "always",
                    "channels": {
                        "dev": true,
                        "beta": true,
                        "stable": true
                    },
                    "telemetry": {
                        "enabled": true,
                        "include_system_info": true
                    }
                }
            }
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();

        assert!(config.settings.update.enabled);
        assert!(matches!(
            config.settings.update.check_frequency,
            UpdateFrequency::Weekly
        ));
        assert!(matches!(
            config.settings.update.auto_update,
            AutoUpdateMode::Always
        ));
        assert!(config.settings.update.channels.dev);
        assert!(config.settings.update.channels.beta);
        assert!(config.settings.update.channels.stable);
        assert!(config.settings.update.telemetry.enabled);
        assert!(config.settings.update.telemetry.include_system_info);
    }

    #[test]
    fn test_partial_update_settings_uses_defaults() {
        let json = r#"{
            "shortcuts": [],
            "app_rules": [],
            "settings": {
                "update": {
                    "check_frequency": "manual"
                }
            }
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();

        // specified value
        assert!(matches!(
            config.settings.update.check_frequency,
            UpdateFrequency::Manual
        ));

        // defaults
        assert!(config.settings.update.enabled);
        assert!(matches!(
            config.settings.update.auto_update,
            AutoUpdateMode::Prompt
        ));
        assert!(config.settings.update.channels.stable);
        assert!(!config.settings.update.channels.dev);
    }

    #[test]
    fn test_spotlight_shortcut_display_name() {
        let shortcut = SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
            icon: None,
        };

        assert_eq!(shortcut.display_name(), "cwm: Focus Safari");
    }

    #[test]
    fn test_spotlight_shortcut_identifier() {
        let shortcut = SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
            icon: None,
        };

        assert_eq!(shortcut.identifier(), "focus-safari");

        let shortcut2 = SpotlightShortcut {
            name: "Move to Next Display".to_string(),
            action: "move_display:next".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        assert_eq!(shortcut2.identifier(), "move-to-next-display");
    }

    #[test]
    fn test_spotlight_shortcut_serialization() {
        let shortcut = SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
            icon: None,
        };

        let json = serde_json::to_string(&shortcut).unwrap();
        let parsed: SpotlightShortcut = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "Focus Safari");
        assert_eq!(parsed.action, "focus");
        assert_eq!(parsed.app, Some("Safari".to_string()));
        assert_eq!(parsed.launch, Some(true));
        assert_eq!(parsed.icon, None);
    }

    #[test]
    fn test_spotlight_shortcut_with_icon() {
        let shortcut = SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
            icon: Some("/path/to/icon.icns".to_string()),
        };

        let json = serde_json::to_string(&shortcut).unwrap();
        assert!(json.contains("icon"));

        let parsed: SpotlightShortcut = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.icon, Some("/path/to/icon.icns".to_string()));
    }

    #[test]
    fn test_spotlight_shortcut_icon_from_json() {
        let json = r#"{
            "name": "Focus Chrome",
            "action": "focus",
            "app": "Chrome",
            "icon": "Google Chrome"
        }"#;

        let shortcut: SpotlightShortcut = serde_json::from_str(json).unwrap();
        assert_eq!(shortcut.icon, Some("Google Chrome".to_string()));
    }

    #[test]
    fn test_config_with_spotlight() {
        let json = r#"{
            "shortcuts": [],
            "app_rules": [],
            "settings": {},
            "spotlight": [
                {
                    "name": "Focus Safari",
                    "action": "focus",
                    "app": "Safari",
                    "launch": true
                },
                {
                    "name": "Maximize Window",
                    "action": "maximize"
                }
            ]
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(config.spotlight.len(), 2);
        assert_eq!(config.spotlight[0].name, "Focus Safari");
        assert_eq!(config.spotlight[0].action, "focus");
        assert_eq!(config.spotlight[0].app, Some("Safari".to_string()));
        assert_eq!(config.spotlight[0].launch, Some(true));
        assert_eq!(config.spotlight[1].name, "Maximize Window");
        assert_eq!(config.spotlight[1].action, "maximize");
    }

    #[test]
    fn test_config_without_spotlight_defaults_to_empty() {
        let json = r#"{
            "shortcuts": [],
            "app_rules": [],
            "settings": {}
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.spotlight.is_empty());
    }
}
