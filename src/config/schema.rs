use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub shortcuts: Vec<Shortcut>,
    #[serde(default)]
    pub app_rules: Vec<AppRule>,
    pub matching: Matching,
    pub behavior: Behavior,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shortcuts: vec![],
            app_rules: vec![],
            matching: Matching::default(),
            behavior: Behavior::default(),
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
    pub launch_if_not_running: Option<bool>,
}

/// Rule to apply an action when an app launches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub app: String,
    pub action: String,
    /// delay in milliseconds before executing the action (overrides global)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matching {
    pub fuzzy_threshold: usize,
}

impl Default for Matching {
    fn default() -> Self {
        Self { fuzzy_threshold: 2 }
    }
}

pub const DEFAULT_APP_RULE_DELAY_MS: u64 = 500;
pub const DEFAULT_APP_RULE_RETRY_COUNT: u32 = 10;
pub const DEFAULT_APP_RULE_RETRY_DELAY_MS: u64 = 100;
pub const DEFAULT_APP_RULE_RETRY_BACKOFF: f64 = 1.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    pub launch_if_not_running: bool,
    pub animate: bool,
    /// default delay in milliseconds before executing app rule actions
    #[serde(default = "default_app_rule_delay")]
    pub app_rule_delay_ms: u64,
    /// number of retry attempts if window is not ready
    #[serde(default = "default_app_rule_retry_count")]
    pub app_rule_retry_count: u32,
    /// initial retry delay in milliseconds
    #[serde(default = "default_app_rule_retry_delay")]
    pub app_rule_retry_delay_ms: u64,
    /// backoff multiplier for each retry (e.g., 1.5 means each retry waits 1.5x longer)
    #[serde(default = "default_app_rule_retry_backoff")]
    pub app_rule_retry_backoff: f64,
}

fn default_app_rule_delay() -> u64 {
    DEFAULT_APP_RULE_DELAY_MS
}

fn default_app_rule_retry_count() -> u32 {
    DEFAULT_APP_RULE_RETRY_COUNT
}

fn default_app_rule_retry_delay() -> u64 {
    DEFAULT_APP_RULE_RETRY_DELAY_MS
}

fn default_app_rule_retry_backoff() -> f64 {
    DEFAULT_APP_RULE_RETRY_BACKOFF
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            launch_if_not_running: false,
            animate: false,
            app_rule_delay_ms: DEFAULT_APP_RULE_DELAY_MS,
            app_rule_retry_count: DEFAULT_APP_RULE_RETRY_COUNT,
            app_rule_retry_delay_ms: DEFAULT_APP_RULE_RETRY_DELAY_MS,
            app_rule_retry_backoff: DEFAULT_APP_RULE_RETRY_BACKOFF,
        }
    }
}

/// Determines if an app should be launched based on CLI flags, shortcut config, and global config
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
}
