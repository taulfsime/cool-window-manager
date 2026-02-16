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
    "conditions": {
      "type": "object",
      "description": "Global condition definitions that can be referenced by $ref in shortcuts and app_rules",
      "additionalProperties": {
        "$ref": "#/$defs/Condition"
      },
      "default": {},
      "examples": [
        {
          "work_hours": { "time": "9:00AM-5:00PM", "time.day": "mon-fri" },
          "docked": { "display.count": { ">=": 2 } }
        }
      ]
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
        },
        "when": {
          "$ref": "#/$defs/Condition",
          "description": "Condition that must be true for this shortcut to execute"
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
        },
        "when": {
          "$ref": "#/$defs/Condition",
          "description": "Condition that must be true for this rule to execute"
        }
      }
    },
    "Condition": {
      "description": "A condition that evaluates to true or false. Supports logical operators (all, any, not), comparison operators, and field conditions.",
      "oneOf": [
        {
          "type": "object",
          "description": "Object with field conditions (implicit AND) or logical operators",
          "properties": {
            "all": {
              "type": "array",
              "description": "All conditions must be true (AND)",
              "items": { "$ref": "#/$defs/Condition" }
            },
            "any": {
              "type": "array",
              "description": "Any condition must be true (OR)",
              "items": { "$ref": "#/$defs/Condition" }
            },
            "not": {
              "$ref": "#/$defs/Condition",
              "description": "Negate a condition"
            },
            "$ref": {
              "type": "string",
              "description": "Reference to a named condition defined in 'conditions'"
            },
            "time": {
              "type": "string",
              "description": "Time range(s): '9:00-17:00', '9AM-5PM', '9:00-12:00,14:00-18:00'",
              "examples": ["9:00AM-5:00PM", "22:00-06:00", "9:00-12:00,14:00-18:00"]
            },
            "time.day": {
              "type": "string",
              "description": "Day(s) of week: 'mon', 'mon-fri', 'mon,wed,fri'",
              "examples": ["mon-fri", "sat,sun", "mon,wed,fri"]
            },
            "display.count": {
              "description": "Number of connected displays",
              "oneOf": [
                { "type": "integer" },
                { "$ref": "#/$defs/CompareOp" }
              ]
            },
            "display.connected": {
              "description": "Check if display alias is connected",
              "oneOf": [
                { "type": "string" },
                { "$ref": "#/$defs/InOp" }
              ]
            },
            "app": {
              "description": "Target app name or title match",
              "oneOf": [
                { "type": "string" },
                { "$ref": "#/$defs/InOp" }
              ]
            },
            "app.running": {
              "description": "Check if app is running",
              "oneOf": [
                { "type": "string" },
                { "$ref": "#/$defs/InOp" }
              ]
            },
            "app.focused": {
              "description": "Check if app has focus",
              "oneOf": [
                { "type": "string" },
                { "type": "boolean" },
                { "$ref": "#/$defs/InOp" }
              ]
            },
            "app.fullscreen": {
              "type": "boolean",
              "description": "Check if target window is fullscreen"
            },
            "app.minimized": {
              "type": "boolean",
              "description": "Check if target window is minimized"
            },
            "app.display": {
              "description": "Check which display target window is on",
              "oneOf": [
                { "type": "string" },
                { "$ref": "#/$defs/InOp" }
              ]
            }
          },
          "additionalProperties": true
        },
        {
          "type": "boolean",
          "description": "true = always, false = never"
        }
      ],
      "examples": [
        { "display.count": { ">=": 2 } },
        { "time": "9AM-5PM", "time.day": "mon-fri" },
        { "all": [{ "$ref": "work_hours" }, { "display.connected": "external" }] },
        { "not": { "app.fullscreen": true } }
      ]
    },
    "CompareOp": {
      "type": "object",
      "description": "Comparison operator object",
      "properties": {
        "==": { "type": "number" },
        "eq": { "type": "number" },
        "equals": { "type": "number" },
        "!=": { "type": "number" },
        "ne": { "type": "number" },
        "not_equals": { "type": "number" },
        ">": { "type": "number" },
        "gt": { "type": "number" },
        "greater_than": { "type": "number" },
        ">=": { "type": "number" },
        "gte": { "type": "number" },
        "greater_than_or_equal": { "type": "number" },
        "<": { "type": "number" },
        "lt": { "type": "number" },
        "less_than": { "type": "number" },
        "<=": { "type": "number" },
        "lte": { "type": "number" },
        "less_than_or_equal": { "type": "number" }
      },
      "additionalProperties": false
    },
    "InOp": {
      "type": "object",
      "description": "Set membership operator",
      "properties": {
        "in": {
          "type": "array",
          "items": { "type": "string" }
        }
      },
      "required": ["in"],
      "additionalProperties": false
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
           "pattern": "^move:.+$",
           "description": "Move window to a position and/or display. Positions: top-left, top-right, bottom-left, bottom-right, left, right, 50%,50%, 100,200px. Displays: next, prev, 0, display=next, or combined: top-left;display=2 (semicolon separates arguments)"
         },
         {
           "pattern": "^resize:(100|[1-9][0-9]?|full)$",
           "description": "Resize window to a percentage of the screen (1-100) or full. Window is centered."
         }
       ],
       "examples": ["focus", "maximize", "move:next", "move:top-left", "move:50%,50%", "move:display=external", "move:top-left;display=2", "resize:80", "resize:full"]
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
        },
        "history": {
          "$ref": "#/$defs/HistorySettings"
        }
      }
    },
    "HistorySettings": {
      "type": "object",
      "description": "Undo/redo history settings",
      "properties": {
        "enabled": {
          "type": "boolean",
          "default": true,
          "description": "Enable undo/redo history tracking"
        },
        "limit": {
          "type": "integer",
          "minimum": 1,
          "default": 50,
          "description": "Maximum number of entries in the undo stack"
        },
        "flush_delay_ms": {
          "type": "integer",
          "minimum": 0,
          "default": 2000,
          "description": "Delay in milliseconds before flushing history to disk"
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
        assert!(defs.get("HistorySettings").is_some());
        assert!(defs.get("Condition").is_some());
        assert!(defs.get("CompareOp").is_some());
        assert!(defs.get("InOp").is_some());
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

    // ========================================================================
    // Action schema tests
    // ========================================================================

    #[test]
    fn test_schema_action_has_all_types() {
        let parsed: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        let action = parsed.get("$defs").and_then(|d| d.get("Action")).unwrap();

        // action should have oneOf with multiple options
        let one_of = action.get("oneOf").unwrap().as_array().unwrap();

        // should have focus, maximize, move, resize
        let has_focus = one_of
            .iter()
            .any(|v| v.get("const") == Some(&serde_json::json!("focus")));
        let has_maximize = one_of
            .iter()
            .any(|v| v.get("const") == Some(&serde_json::json!("maximize")));
        let has_move = one_of.iter().any(|v| {
            v.get("pattern")
                .and_then(|p| p.as_str())
                .map(|s| s.contains("move:"))
                .unwrap_or(false)
        });
        let has_resize = one_of.iter().any(|v| {
            v.get("pattern")
                .and_then(|p| p.as_str())
                .map(|s| s.contains("resize:"))
                .unwrap_or(false)
        });

        assert!(has_focus, "schema should include focus action");
        assert!(has_maximize, "schema should include maximize action");
        assert!(has_move, "schema should include move action pattern");
        assert!(has_resize, "schema should include resize action pattern");
    }

    #[test]
    fn test_schema_action_examples_include_move_variants() {
        let parsed: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        let action = parsed.get("$defs").and_then(|d| d.get("Action")).unwrap();

        let examples = action.get("examples").unwrap().as_array().unwrap();
        let examples_str: Vec<&str> = examples.iter().filter_map(|v| v.as_str()).collect();

        // check for various move action examples
        assert!(
            examples_str.iter().any(|e| *e == "move:next"),
            "examples should include move:next"
        );
        assert!(
            examples_str.iter().any(|e| *e == "move:top-left"),
            "examples should include move:top-left"
        );
        assert!(
            examples_str.iter().any(|e| e.contains("50%")),
            "examples should include percentage move"
        );
        assert!(
            examples_str.iter().any(|e| e.contains(";display=")),
            "examples should include combined position;display format"
        );
    }

    #[test]
    fn test_schema_move_action_description_mentions_semicolon() {
        let parsed: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        let action = parsed.get("$defs").and_then(|d| d.get("Action")).unwrap();

        let one_of = action.get("oneOf").unwrap().as_array().unwrap();
        let move_action = one_of
            .iter()
            .find(|v| {
                v.get("pattern")
                    .and_then(|p| p.as_str())
                    .map(|s| s.contains("move:"))
                    .unwrap_or(false)
            })
            .unwrap();

        let description = move_action.get("description").unwrap().as_str().unwrap();

        // description should mention semicolon separator
        assert!(
            description.contains("semicolon"),
            "move action description should mention semicolon separator"
        );
        // description should mention various position types
        assert!(
            description.contains("top-left"),
            "move action description should mention anchors"
        );
        assert!(
            description.contains("%"),
            "move action description should mention percentages"
        );
        assert!(
            description.contains("px"),
            "move action description should mention pixels"
        );
    }

    #[test]
    fn test_schema_move_action_no_move_display() {
        // ensure old move_display action is not in schema
        let schema_str = JSON_SCHEMA;
        assert!(
            !schema_str.contains("move_display"),
            "schema should not contain old move_display action"
        );
        assert!(
            !schema_str.contains("move-display"),
            "schema should not contain old move-display action"
        );
    }

    // ========================================================================
    // display_aliases schema tests
    // ========================================================================

    #[test]
    fn test_schema_display_aliases_structure() {
        let parsed: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        let display_aliases = parsed
            .get("properties")
            .and_then(|p| p.get("display_aliases"))
            .unwrap();

        // should be an object type
        assert_eq!(
            display_aliases.get("type").and_then(|v| v.as_str()),
            Some("object")
        );

        // should have additionalProperties as array of strings
        let additional = display_aliases.get("additionalProperties").unwrap();
        assert_eq!(
            additional.get("type").and_then(|v| v.as_str()),
            Some("array")
        );
    }

    #[test]
    fn test_schema_display_aliases_examples() {
        let parsed: serde_json::Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        let display_aliases = parsed
            .get("properties")
            .and_then(|p| p.get("display_aliases"))
            .unwrap();

        let examples = display_aliases.get("examples").unwrap().as_array().unwrap();
        assert!(!examples.is_empty(), "display_aliases should have examples");

        // first example should have office and home aliases
        let first_example = &examples[0];
        assert!(first_example.get("office").is_some());
        assert!(first_example.get("home").is_some());
    }
}
