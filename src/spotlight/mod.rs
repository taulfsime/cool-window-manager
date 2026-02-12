mod generator;

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

/// prints example spotlight configuration
pub fn print_example_config() {
    let examples = vec![
        SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
        },
        SpotlightShortcut {
            name: "Focus Slack".to_string(),
            action: "focus".to_string(),
            app: Some("Slack".to_string()),
            launch: Some(true),
        },
        SpotlightShortcut {
            name: "Maximize Window".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
        },
        SpotlightShortcut {
            name: "Move to Next Display".to_string(),
            action: "move_display:next".to_string(),
            app: None,
            launch: None,
        },
        SpotlightShortcut {
            name: "Move to Previous Display".to_string(),
            action: "move_display:prev".to_string(),
            app: None,
            launch: None,
        },
        SpotlightShortcut {
            name: "Resize 80%".to_string(),
            action: "resize:80".to_string(),
            app: None,
            launch: None,
        },
        SpotlightShortcut {
            name: "Resize Full".to_string(),
            action: "resize:full".to_string(),
            app: None,
            launch: None,
        },
    ];

    println!("Add the following to your ~/.cwm/config.json:\n");
    println!(r#""spotlight": "#);

    let json = serde_json::to_string_pretty(&examples).expect("Failed to serialize examples");
    println!("{}", json);

    println!("\nAfter adding, run: cwm spotlight install");
    println!("\nShortcuts will appear in Spotlight with the \"cwm: \" prefix.");
    println!("For example, search for \"cwm: Focus Safari\" in Spotlight.");
}
