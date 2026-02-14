mod generator;
pub(crate) mod icons;
pub(crate) mod signing;

pub use generator::{
    get_apps_directory, get_installed_shortcuts, install_all, install_shortcut, remove_all,
    remove_shortcut,
};

use crate::config::SpotlightShortcut;

/// prefix added to all spotlight shortcut names
pub const SHORTCUT_PREFIX: &str = "cwm: ";

/// bundle identifier prefix for generated apps
pub const BUNDLE_ID_PREFIX: &str = "com.cwm.spotlight";

/// returns the default directory for cwm spotlight apps
pub fn default_apps_directory() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join("Applications")
        .join("cwm")
}

/// returns example spotlight shortcuts for documentation
pub fn get_example_shortcuts() -> Vec<SpotlightShortcut> {
    vec![
        SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
            icon: None, // will auto-extract from Safari
        },
        SpotlightShortcut {
            name: "Focus Slack".to_string(),
            action: "focus".to_string(),
            app: Some("Slack".to_string()),
            launch: Some(true),
            icon: None, // will auto-extract from Slack
        },
        SpotlightShortcut {
            name: "Maximize Window".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
            icon: None, // will use default cwm icon
        },
        SpotlightShortcut {
            name: "Move to Next Display".to_string(),
            action: "move:next".to_string(),
            app: None,
            launch: None,
            icon: None,
        },
        SpotlightShortcut {
            name: "Move to Previous Display".to_string(),
            action: "move:prev".to_string(),
            app: None,
            launch: None,
            icon: None,
        },
        SpotlightShortcut {
            name: "Move to Top Left".to_string(),
            action: "move:top-left".to_string(),
            app: None,
            launch: None,
            icon: None,
        },
        SpotlightShortcut {
            name: "Center Window".to_string(),
            action: "move:50%,50%".to_string(),
            app: None,
            launch: None,
            icon: None,
        },
        SpotlightShortcut {
            name: "Top Left on Next Display".to_string(),
            action: "move:top-left;display=next".to_string(),
            app: None,
            launch: None,
            icon: None,
        },
        SpotlightShortcut {
            name: "Resize 80%".to_string(),
            action: "resize:80".to_string(),
            app: None,
            launch: None,
            icon: None,
        },
        SpotlightShortcut {
            name: "Resize Full".to_string(),
            action: "resize:full".to_string(),
            app: None,
            launch: None,
            icon: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_prefix_constant() {
        assert_eq!(SHORTCUT_PREFIX, "cwm: ");
    }

    #[test]
    fn test_bundle_id_prefix_constant() {
        assert_eq!(BUNDLE_ID_PREFIX, "com.cwm.spotlight");
    }

    #[test]
    fn test_default_apps_directory() {
        let dir = default_apps_directory();
        let path_str = dir.to_string_lossy();

        // should be in home directory
        assert!(path_str.contains("Applications"));
        assert!(path_str.contains("cwm"));
    }

    #[test]
    fn test_get_example_shortcuts_count() {
        let shortcuts = get_example_shortcuts();
        assert_eq!(shortcuts.len(), 10);
    }

    #[test]
    fn test_get_example_shortcuts_has_focus() {
        let shortcuts = get_example_shortcuts();
        let focus_count = shortcuts.iter().filter(|s| s.action == "focus").count();
        assert_eq!(focus_count, 2);
    }

    #[test]
    fn test_get_example_shortcuts_has_maximize() {
        let shortcuts = get_example_shortcuts();
        let has_maximize = shortcuts.iter().any(|s| s.action == "maximize");
        assert!(has_maximize);
    }

    #[test]
    fn test_get_example_shortcuts_has_move() {
        let shortcuts = get_example_shortcuts();
        let move_count = shortcuts
            .iter()
            .filter(|s| s.action.starts_with("move:"))
            .count();
        assert_eq!(move_count, 5);
    }

    #[test]
    fn test_get_example_shortcuts_has_resize() {
        let shortcuts = get_example_shortcuts();
        let resize_count = shortcuts
            .iter()
            .filter(|s| s.action.starts_with("resize"))
            .count();
        assert_eq!(resize_count, 2);
    }

    #[test]
    fn test_get_example_shortcuts_focus_has_app() {
        let shortcuts = get_example_shortcuts();
        for shortcut in shortcuts.iter().filter(|s| s.action == "focus") {
            assert!(shortcut.app.is_some(), "Focus shortcuts should have app");
            assert!(
                shortcut.launch.is_some(),
                "Focus shortcuts should have launch"
            );
        }
    }

    #[test]
    fn test_get_example_shortcuts_non_focus_no_app() {
        let shortcuts = get_example_shortcuts();
        for shortcut in shortcuts.iter().filter(|s| s.action != "focus") {
            assert!(
                shortcut.app.is_none(),
                "Non-focus shortcuts should not have app"
            );
        }
    }

    #[test]
    fn test_get_example_shortcuts_serializable() {
        let shortcuts = get_example_shortcuts();
        let json = serde_json::to_string(&shortcuts);
        assert!(json.is_ok());
    }
}
