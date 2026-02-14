use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub const JSON_SCHEMA: &str = r##"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "CWM Configuration",
  "description": "Configuration file for cwm (Cool Window Manager)",
  "type": "object",
  "properties": {
    "$schema": {
      "type": "string",
      "description": "JSON schema reference"
    },
    "shortcuts": {
      "type": "array",
      "description": "Global hotkey shortcuts",
      "items": {
        "$ref": "#/$defs/Shortcut"
      },
      "default": []
    },
    "app_rules": {
      "type": "array",
      "description": "Rules to apply when applications launch",
      "items": {
        "$ref": "#/$defs/AppRule"
      },
      "default": []
    },
    "spotlight": {
      "type": "array",
      "description": "Spotlight shortcuts that appear in macOS Spotlight search",
      "items": {
        "$ref": "#/$defs/SpotlightShortcut"
      },
      "default": []
    },
     "settings": {
       "$ref": "#/$defs/Settings",
       "description": "Global settings"
     },
     "display_aliases": {
       "type": "object",
       "description": "Map display aliases to unique IDs for multi-location setups",
       "additionalProperties": {
         "type": "array",
         "items": {
           "type": "string",
           "description": "Unique display ID (vendor_model_serial) or display name"
         }
       },
       "default": {},
       "examples": [
         {
           "office": ["10AC_D0B3_67890"],
           "home": ["1E6D_5B11_12345", "10AC_D0B3_67890"]
         }
       ]
     }
   },
   "$defs": {
    "Shortcut": {
      "type": "object",
      "description": "A global hotkey shortcut binding",
      "required": ["keys", "action"],
      "properties": {
        "keys": {
          "type": "string",
          "description": "Hotkey combination (e.g., ctrl+alt+s, cmd+shift+m)",
          "examples": ["ctrl+alt+s", "cmd+shift+m", "ctrl+alt+right"]
        },
        "action": {
          "$ref": "#/$defs/Action"
        },
        "app": {
          "type": "string",
          "description": "Target application name (fuzzy matched). Required for focus action."
        },
        "launch": {
          "type": "boolean",
          "description": "Launch the app if not running. Overrides global settings.launch."
        }
      },
      "if": {
        "properties": {
          "action": { "const": "focus" }
        }
      },
      "then": {
        "required": ["app"]
      }
    },
    "AppRule": {
      "type": "object",
      "description": "A rule that applies an action when an application launches",
      "required": ["app", "action"],
      "properties": {
        "app": {
          "type": "string",
          "description": "Application name (exact match)"
        },
        "action": {
          "$ref": "#/$defs/Action"
        },
        "delay_ms": {
          "type": "integer",
          "minimum": 0,
          "description": "Delay in milliseconds before executing the action. Overrides global settings.delay_ms."
        }
      }
    },
    "SpotlightShortcut": {
      "type": "object",
      "description": "A shortcut that appears in macOS Spotlight search",
      "required": ["name", "action"],
      "properties": {
        "name": {
          "type": "string",
          "description": "Name displayed in Spotlight (will be prefixed with cwm: )",
          "minLength": 1
        },
        "action": {
          "$ref": "#/$defs/Action"
        },
        "app": {
          "type": "string",
          "description": "Target application name (fuzzy matched). Required for focus action."
        },
        "launch": {
          "type": "boolean",
          "description": "Launch the app if not running"
        },
        "icon": {
          "type": "string",
          "description": "Custom icon for the Spotlight shortcut. Can be: path to .icns file, path to .png file, or app name to extract icon from. If not specified, uses target app's icon (if app is set) or default cwm icon.",
          "examples": ["/path/to/icon.icns", "Safari", "~/icons/custom.png"]
        }
      },
      "if": {
        "properties": {
          "action": { "const": "focus" }
        }
      },
      "then": {
        "required": ["app"]
      }
    },
     "Action": {
       "type": "string",
       "description": "Window action to perform",
       "oneOf": [
         {
           "const": "focus",
           "description": "Focus the application window (requires app field)"
         },
         {
           "const": "maximize",
           "description": "Maximize the current or specified window"
         },
         {
           "pattern": "^move_display:(next|prev|[0-9]+|[a-zA-Z_][a-zA-Z0-9_]*)$",
           "description": "Move window to another display. Use next, prev, a display number, or an alias name (builtin, external, main, secondary, or custom)."
         },
         {
           "pattern": "^resize:(100|[1-9][0-9]?|full)$",
           "description": "Resize window to a percentage of the screen (1-100) or full. Window is centered."
         }
       ],
       "examples": ["focus", "maximize", "move_display:next", "move_display:prev", "move_display:0", "move_display:external", "move_display:office_main", "resize:80", "resize:full"]
     },
    "Settings": {
      "type": "object",
      "description": "Global settings",
      "properties": {
        "fuzzy_threshold": {
          "type": "integer",
          "minimum": 0,
          "default": 2,
          "description": "Maximum Levenshtein distance for fuzzy app name matching"
        },
        "launch": {
          "type": "boolean",
          "default": false,
          "description": "Launch apps if not running (can be overridden per shortcut)"
        },
        "animate": {
          "type": "boolean",
          "default": false,
          "description": "Animate window movements"
        },
        "delay_ms": {
          "type": "integer",
          "minimum": 0,
          "default": 500,
          "description": "Default delay in milliseconds before executing app rule actions"
        },
        "retry": {
          "$ref": "#/$defs/Retry"
        },
        "update": {
          "$ref": "#/$defs/UpdateSettings"
        }
      }
    },
    "Retry": {
      "type": "object",
      "description": "Retry settings for window operations",
      "properties": {
        "count": {
          "type": "integer",
          "minimum": 0,
          "default": 10,
          "description": "Number of retry attempts"
        },
        "delay_ms": {
          "type": "integer",
          "minimum": 0,
          "default": 100,
          "description": "Initial delay between retries in milliseconds"
        },
        "backoff": {
          "type": "number",
          "minimum": 1.0,
          "default": 1.5,
          "description": "Backoff multiplier for retry delays"
        }
      }
    },
    "UpdateSettings": {
      "type": "object",
      "description": "Update checking and auto-update settings",
      "properties": {
        "enabled": {
          "type": "boolean",
          "default": true,
          "description": "Enable update checking"
        },
        "check_frequency": {
          "type": "string",
          "enum": ["daily", "weekly", "manual"],
          "default": "daily",
          "description": "How often to check for updates"
        },
        "auto_update": {
          "type": "string",
          "enum": ["always", "prompt", "never"],
          "default": "prompt",
          "description": "Auto-update behavior: always install, prompt user, or never auto-update"
        },
        "channels": {
          "$ref": "#/$defs/UpdateChannels"
        },
        "telemetry": {
          "$ref": "#/$defs/TelemetrySettings"
        },
        "last_check": {
          "type": "string",
          "format": "date-time",
          "description": "Timestamp of last update check (managed automatically)"
        }
      }
    },
    "UpdateChannels": {
      "type": "object",
      "description": "Which release channels to consider for updates",
      "properties": {
        "dev": {
          "type": "boolean",
          "default": false,
          "description": "Include development releases"
        },
        "beta": {
          "type": "boolean",
          "default": false,
          "description": "Include beta releases"
        },
        "stable": {
          "type": "boolean",
          "default": true,
          "description": "Include stable releases"
        }
      }
    },
    "TelemetrySettings": {
      "type": "object",
      "description": "Error reporting and telemetry settings",
      "properties": {
        "enabled": {
          "type": "boolean",
          "default": false,
          "description": "Enable error reporting"
        },
        "include_system_info": {
          "type": "boolean",
          "default": false,
          "description": "Include system information in error reports"
        }
      }
    }
  }
}"##;

/// writes the JSON schema to the specified path
pub fn write_schema_file(path: &Path) -> Result<()> {
    fs::write(path, JSON_SCHEMA)
        .with_context(|| format!("failed to write schema file: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        assert!(parsed.is_object());
        assert_eq!(
            parsed.get("$schema").and_then(|v| v.as_str()),
            Some("http://json-schema.org/draft-07/schema#")
        );
    }

    #[test]
    fn test_schema_has_required_definitions() {
        let parsed: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        let defs = parsed.get("$defs").unwrap();

        assert!(defs.get("Shortcut").is_some());
        assert!(defs.get("AppRule").is_some());
        assert!(defs.get("SpotlightShortcut").is_some());
        assert!(defs.get("Action").is_some());
        assert!(defs.get("Settings").is_some());
        assert!(defs.get("Retry").is_some());
        assert!(defs.get("UpdateSettings").is_some());
        assert!(defs.get("UpdateChannels").is_some());
        assert!(defs.get("TelemetrySettings").is_some());
    }

    #[test]
    fn test_write_schema_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("cwm_test_schema.json");

        write_schema_file(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, JSON_SCHEMA);

        std::fs::remove_file(&path).ok();
    }
}
