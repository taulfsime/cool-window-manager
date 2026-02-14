//! record action handlers

use anyhow::Result;
use serde::Serialize;

use crate::cli::output::OutputMode;
use crate::config::Config;
use crate::display::{self, DisplayInfo, DisplayTarget};
use crate::window::{manager, matching};

/// recorded window layout for a single app
#[derive(Debug, Clone, Serialize)]
struct RecordedWindow {
    app_name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    display_index: usize,
    display_name: String,
    display_unique_id: String,
    is_builtin: bool,
}

/// execute record layout command
pub fn execute_record_layout(
    app_filter: &[String],
    display_filter: Option<&str>,
    config: &Config,
    output_mode: OutputMode,
) -> Result<()> {
    let running_apps = matching::get_running_apps()?;
    let displays = display::get_displays()?;

    // resolve display filter if provided
    let target_display: Option<&DisplayInfo> = if let Some(d) = display_filter {
        let target = DisplayTarget::parse(d)?;
        Some(display::resolve_target_display_with_aliases(
            0,
            &target,
            &displays,
            &config.display_aliases,
        )?)
    } else {
        None
    };

    // collect window info for matching apps
    let mut recorded_windows: Vec<RecordedWindow> = Vec::new();

    for app in &running_apps {
        // filter by app name if specified
        if !app_filter.is_empty() {
            let matches_filter = app_filter.iter().any(|filter| {
                let filter_lower = filter.to_lowercase();
                let app_lower = app.name.to_lowercase();
                app_lower == filter_lower || app_lower.starts_with(&filter_lower)
            });
            if !matches_filter {
                continue;
            }
        }

        // try to get window info for this app
        let window_info = match manager::get_window_info_for_app(app) {
            Ok(info) => info,
            Err(_) => continue, // skip apps without accessible windows
        };

        let (_, window_data, display_data) = window_info;

        // find the display this window is on
        let display = displays
            .iter()
            .find(|d| d.index == display_data.index)
            .unwrap_or(&displays[0]);

        // filter by display if specified
        if let Some(target) = target_display {
            if display.index != target.index {
                continue;
            }
        }

        recorded_windows.push(RecordedWindow {
            app_name: app.name.clone(),
            x: window_data.x,
            y: window_data.y,
            width: window_data.width,
            height: window_data.height,
            display_index: display.index,
            display_name: display.name.clone(),
            display_unique_id: display.unique_id(),
            is_builtin: display.is_builtin,
        });
    }

    if recorded_windows.is_empty() {
        if app_filter.is_empty() && display_filter.is_none() {
            println!("No windows found to record.");
        } else {
            println!("No matching windows found.");
            if !app_filter.is_empty() {
                println!("  App filter: {}", app_filter.join(", "));
            }
            if let Some(d) = display_filter {
                println!("  Display filter: {}", d);
            }
        }
        return Ok(());
    }

    // output based on mode
    if output_mode.is_json() {
        print_json_output(&recorded_windows, &displays);
    } else {
        print_text_output(&recorded_windows, &displays);
    }

    Ok(())
}

fn print_text_output(windows: &[RecordedWindow], _displays: &[DisplayInfo]) {
    println!("Recording window layout...\n");

    // print window info with action snippets
    for w in windows {
        println!(
            "{} (Display {}: {})",
            w.app_name, w.display_index, w.display_name
        );
        println!("  Position: {}, {}", w.x, w.y);
        println!("  Size: {}x{}", w.width, w.height);

        // show action snippets for easy copy-paste
        let move_action = format!("\"move:{},{}px\"", w.x, w.y);
        let resize_action = format!("\"resize:{}x{}px\"", w.width, w.height);
        let display_action = if w.is_builtin {
            "\"move:builtin\"".to_string()
        } else {
            format!("\"move:{}\"", w.display_unique_id)
        };

        println!(
            "  Actions: {} {} {}",
            resize_action, move_action, display_action
        );
        println!();
    }
}

fn print_json_output(windows: &[RecordedWindow], displays: &[DisplayInfo]) {
    let mut variants = serde_json::Map::new();

    // full variant
    variants.insert(
        "full".to_string(),
        generate_variant_json(windows, true, true, false, displays),
    );

    // position_only variant
    variants.insert(
        "position_only".to_string(),
        generate_variant_json(windows, true, false, false, displays),
    );

    // display_only variant
    variants.insert(
        "display_only".to_string(),
        generate_variant_json(windows, false, false, true, displays),
    );

    // size_only variant
    variants.insert(
        "size_only".to_string(),
        generate_variant_json(windows, false, true, false, displays),
    );

    let output = serde_json::json!({
        "windows": windows,
        "variants": variants,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn generate_variant_json(
    windows: &[RecordedWindow],
    include_position: bool,
    include_size: bool,
    include_display: bool,
    _displays: &[DisplayInfo],
) -> serde_json::Value {
    let mut rules: Vec<serde_json::Value> = Vec::new();
    let mut aliases_needed: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for w in windows {
        if include_display {
            let display_target = if w.is_builtin {
                "builtin".to_string()
            } else {
                let alias_name = format!("display_{}", w.display_index);
                aliases_needed.insert(alias_name.clone(), w.display_unique_id.clone());
                alias_name
            };

            rules.push(serde_json::json!({
                "app": w.app_name,
                "action": format!("move:{}", display_target),
            }));
        }

        if include_position && !include_display {
            rules.push(serde_json::json!({
                "app": w.app_name,
                "action": format!("move:{},{}px", w.x, w.y),
                "delay_ms": 500,
            }));
        }

        if include_size {
            let mut rule = serde_json::json!({
                "app": w.app_name,
                "action": format!("resize:{}x{}px", w.width, w.height),
            });
            if include_position {
                rule["delay_ms"] = serde_json::json!(600);
            }
            rules.push(rule);
        }
    }

    let mut output = serde_json::json!({
        "app_rules": rules,
    });

    if !aliases_needed.is_empty() {
        let mut aliases_obj = serde_json::Map::new();
        for (alias, unique_id) in &aliases_needed {
            aliases_obj.insert(alias.clone(), serde_json::json!([unique_id]));
        }
        output["display_aliases"] = serde_json::Value::Object(aliases_obj);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorded_window_serialization() {
        let window = RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        };

        let json = serde_json::to_string(&window).unwrap();
        assert!(json.contains("Safari"));
        assert!(json.contains("1920"));
        assert!(json.contains("Built-in Display"));
    }

    #[test]
    fn test_generate_variant_json_position_only() {
        let windows = vec![RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        }];

        let result = generate_variant_json(&windows, true, false, false, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        assert_eq!(rules.len(), 1);
        let action = rules[0].get("action").unwrap().as_str().unwrap();
        assert!(action.starts_with("move:"));
        assert!(action.contains("100,200px"));
    }

    #[test]
    fn test_generate_variant_json_size_only() {
        let windows = vec![RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        }];

        let result = generate_variant_json(&windows, false, true, false, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        assert_eq!(rules.len(), 1);
        let action = rules[0].get("action").unwrap().as_str().unwrap();
        assert!(action.starts_with("resize:"));
        assert!(action.contains("1920x1080px"));
    }

    #[test]
    fn test_generate_variant_json_display_only_builtin() {
        let windows = vec![RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        }];

        let result = generate_variant_json(&windows, false, false, true, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        assert_eq!(rules.len(), 1);
        let action = rules[0].get("action").unwrap().as_str().unwrap();
        assert_eq!(action, "move:builtin");
    }

    #[test]
    fn test_generate_variant_json_display_only_external() {
        let windows = vec![RecordedWindow {
            app_name: "Chrome".to_string(),
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
            display_index: 1,
            display_name: "LG Display".to_string(),
            display_unique_id: "1E6D_5B11_12345".to_string(),
            is_builtin: false,
        }];

        let result = generate_variant_json(&windows, false, false, true, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        assert_eq!(rules.len(), 1);
        let action = rules[0].get("action").unwrap().as_str().unwrap();
        assert_eq!(action, "move:display_1");

        // should include display_aliases
        let aliases = result.get("display_aliases").unwrap();
        assert!(aliases.get("display_1").is_some());
    }

    #[test]
    fn test_generate_variant_json_full() {
        let windows = vec![RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        }];

        let result = generate_variant_json(&windows, true, true, false, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        // should have 2 rules: move and resize
        assert_eq!(rules.len(), 2);

        let move_action = rules[0].get("action").unwrap().as_str().unwrap();
        assert!(move_action.starts_with("move:"));

        let resize_action = rules[1].get("action").unwrap().as_str().unwrap();
        assert!(resize_action.starts_with("resize:"));
    }

    #[test]
    fn test_generate_variant_json_multiple_windows() {
        let windows = vec![
            RecordedWindow {
                app_name: "Safari".to_string(),
                x: 0,
                y: 0,
                width: 960,
                height: 1080,
                display_index: 0,
                display_name: "Built-in Display".to_string(),
                display_unique_id: "0610_A034_12345".to_string(),
                is_builtin: true,
            },
            RecordedWindow {
                app_name: "Chrome".to_string(),
                x: 960,
                y: 0,
                width: 960,
                height: 1080,
                display_index: 0,
                display_name: "Built-in Display".to_string(),
                display_unique_id: "0610_A034_12345".to_string(),
                is_builtin: true,
            },
        ];

        let result = generate_variant_json(&windows, true, true, false, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        // should have 4 rules: move+resize for each app
        assert_eq!(rules.len(), 4);

        // check Safari rules
        assert_eq!(rules[0].get("app").unwrap().as_str().unwrap(), "Safari");
        assert!(rules[0]
            .get("action")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("move:"));
        assert_eq!(rules[1].get("app").unwrap().as_str().unwrap(), "Safari");
        assert!(rules[1]
            .get("action")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("resize:"));

        // check Chrome rules
        assert_eq!(rules[2].get("app").unwrap().as_str().unwrap(), "Chrome");
        assert_eq!(rules[3].get("app").unwrap().as_str().unwrap(), "Chrome");
    }

    #[test]
    fn test_generate_variant_json_delay_ms_values() {
        let windows = vec![RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        }];

        // full variant (position + size) should have delay_ms
        let result = generate_variant_json(&windows, true, true, false, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        // move rule should have delay_ms: 500
        assert_eq!(rules[0].get("delay_ms").unwrap().as_u64().unwrap(), 500);
        // resize rule should have delay_ms: 600
        assert_eq!(rules[1].get("delay_ms").unwrap().as_u64().unwrap(), 600);
    }

    #[test]
    fn test_generate_variant_json_size_only_no_delay() {
        let windows = vec![RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        }];

        // size-only variant should not have delay_ms
        let result = generate_variant_json(&windows, false, true, false, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        assert!(rules[0].get("delay_ms").is_none());
    }

    #[test]
    fn test_generate_variant_json_display_only_no_delay() {
        let windows = vec![RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        }];

        // display-only variant should not have delay_ms
        let result = generate_variant_json(&windows, false, false, true, &[]);
        let rules = result.get("app_rules").unwrap().as_array().unwrap();

        assert!(rules[0].get("delay_ms").is_none());
    }

    #[test]
    fn test_generate_variant_json_external_display_alias() {
        let windows = vec![
            RecordedWindow {
                app_name: "Safari".to_string(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                display_index: 1,
                display_name: "Dell Monitor".to_string(),
                display_unique_id: "10AC_D0B3_67890".to_string(),
                is_builtin: false,
            },
            RecordedWindow {
                app_name: "Chrome".to_string(),
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
                display_index: 2,
                display_name: "LG Monitor".to_string(),
                display_unique_id: "1E6D_5B11_12345".to_string(),
                is_builtin: false,
            },
        ];

        let result = generate_variant_json(&windows, false, false, true, &[]);

        // check display_aliases contains both displays
        let aliases = result.get("display_aliases").unwrap();
        assert!(aliases.get("display_1").is_some());
        assert!(aliases.get("display_2").is_some());

        // check the unique IDs are correct
        let display_1_ids = aliases.get("display_1").unwrap().as_array().unwrap();
        assert_eq!(display_1_ids[0].as_str().unwrap(), "10AC_D0B3_67890");

        let display_2_ids = aliases.get("display_2").unwrap().as_array().unwrap();
        assert_eq!(display_2_ids[0].as_str().unwrap(), "1E6D_5B11_12345");
    }

    #[test]
    fn test_recorded_window_action_strings() {
        let window = RecordedWindow {
            app_name: "Safari".to_string(),
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        };

        // verify the action string formats
        let move_action = format!("move:{},{}px", window.x, window.y);
        assert_eq!(move_action, "move:100,200px");

        let resize_action = format!("resize:{}x{}px", window.width, window.height);
        assert_eq!(resize_action, "resize:1920x1080px");

        let display_action = if window.is_builtin {
            "move:builtin".to_string()
        } else {
            format!("move:{}", window.display_unique_id)
        };
        assert_eq!(display_action, "move:builtin");
    }

    #[test]
    fn test_recorded_window_external_display_action() {
        let window = RecordedWindow {
            app_name: "Chrome".to_string(),
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
            display_index: 1,
            display_name: "LG Display".to_string(),
            display_unique_id: "1E6D_5B11_12345".to_string(),
            is_builtin: false,
        };

        let display_action = if window.is_builtin {
            "move:builtin".to_string()
        } else {
            format!("move:{}", window.display_unique_id)
        };
        assert_eq!(display_action, "move:1E6D_5B11_12345");
    }

    #[test]
    fn test_recorded_window_negative_coordinates() {
        let window = RecordedWindow {
            app_name: "Safari".to_string(),
            x: -100,
            y: -50,
            width: 800,
            height: 600,
            display_index: 0,
            display_name: "Built-in Display".to_string(),
            display_unique_id: "0610_A034_12345".to_string(),
            is_builtin: true,
        };

        let move_action = format!("move:{},{}px", window.x, window.y);
        assert_eq!(move_action, "move:-100,-50px");
    }
}
