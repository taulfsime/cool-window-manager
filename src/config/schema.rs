use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub shortcuts: Vec<Shortcut>,
    pub matching: Matching,
    pub behavior: Behavior,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shortcuts: vec![],
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



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matching {
    pub fuzzy_threshold: usize,
}

impl Default for Matching {
    fn default() -> Self {
        Self { fuzzy_threshold: 2 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    pub launch_if_not_running: bool,
    pub animate: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            launch_if_not_running: false,
            animate: false,
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
