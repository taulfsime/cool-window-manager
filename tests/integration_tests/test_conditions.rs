// integration tests for conditions system

use std::fs;

use crate::common::{cleanup_test_dir, create_test_dir, run_cwm_with_env};

/// test that config with conditions field is valid
#[test]
fn test_config_with_conditions_is_valid() {
    let test_dir = create_test_dir("conditions_valid");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } },
            "work_hours": { "time": "9:00-17:00", "time.day": "mon-fri" }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test that shortcut with when condition is valid
#[test]
fn test_shortcut_with_when_condition_is_valid() {
    let test_dir = create_test_dir("conditions_shortcut_when");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Safari",
                "when": { "display.count": { ">=": 2 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test that app_rule with when condition is valid
#[test]
fn test_app_rule_with_when_condition_is_valid() {
    let test_dir = create_test_dir("conditions_app_rule_when");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [],
        "app_rules": [
            {
                "app": "Slack",
                "action": "move:external",
                "when": { "display.connected": "external" }
            }
        ],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test that $ref references work in when conditions
#[test]
fn test_condition_ref_in_when() {
    let test_dir = create_test_dir("conditions_ref");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "$ref": "docked" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test complex condition with all/any/not
#[test]
fn test_complex_condition_is_valid() {
    let test_dir = create_test_dir("conditions_complex");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "work_setup": {
                "all": [
                    { "display.count": { ">=": 2 } },
                    { "time.day": "mon-fri" },
                    { "not": { "app.running": "Slack" } }
                ]
            }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": { "$ref": "work_setup" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test condition with any (OR) operator
#[test]
fn test_any_condition_is_valid() {
    let test_dir = create_test_dir("conditions_any");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "focus",
                "app": "Safari",
                "when": {
                    "any": [
                        { "app.running": "Safari" },
                        { "app.running": "Chrome" },
                        { "app.running": "Firefox" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test condition with in operator
#[test]
fn test_in_operator_condition_is_valid() {
    let test_dir = create_test_dir("conditions_in");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "move:external",
                "when": {
                    "display.connected": { "in": ["external", "office"] }
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test config show includes conditions
#[test]
fn test_config_show_includes_conditions() {
    let test_dir = create_test_dir("conditions_show");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "show", "--json"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "config show failed");
    assert!(
        stdout.contains("conditions") || stdout.contains("docked"),
        "config show should include conditions: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test time condition format
#[test]
fn test_time_condition_format_is_valid() {
    let test_dir = create_test_dir("conditions_time");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": {
                    "time": "9:00AM-5:00PM",
                    "time.day": "mon-fri"
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test overnight time range format
#[test]
fn test_overnight_time_range_is_valid() {
    let test_dir = create_test_dir("conditions_overnight");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+n",
                "action": "maximize",
                "when": {
                    "time": "22:00-06:00"
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test app.fullscreen and app.minimized conditions
#[test]
fn test_window_state_conditions_are_valid() {
    let test_dir = create_test_dir("conditions_window_state");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+f",
                "action": "maximize",
                "when": {
                    "not": { "app.fullscreen": true }
                }
            },
            {
                "keys": "ctrl+alt+u",
                "action": "focus",
                "app": "Safari",
                "when": {
                    "app.minimized": false
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test comparison operators
#[test]
fn test_comparison_operators_are_valid() {
    let test_dir = create_test_dir("conditions_comparison");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "single_display": { "display.count": { "==": 1 } },
            "not_single": { "display.count": { "!=": 1 } },
            "multi_display": { "display.count": { ">": 1 } },
            "at_least_two": { "display.count": { ">=": 2 } },
            "less_than_three": { "display.count": { "<": 3 } },
            "at_most_two": { "display.count": { "<=": 2 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

/// test alternative operator names (eq, ne, gt, gte, lt, lte)
#[test]
fn test_alternative_operator_names_are_valid() {
    let test_dir = create_test_dir("conditions_alt_ops");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "single_display": { "display.count": { "eq": 1 } },
            "not_single": { "display.count": { "ne": 1 } },
            "multi_display": { "display.count": { "gt": 1 } },
            "at_least_two": { "display.count": { "gte": 2 } },
            "less_than_three": { "display.count": { "lt": 3 } },
            "at_most_two": { "display.count": { "lte": 2 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config verify failed: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 1: Time Conditions (~10 tests)
// ============================================================================

/// test 24-hour time format
#[test]
fn test_time_24h_format() {
    let test_dir = create_test_dir("time_24h");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": { "time": "09:00-17:00" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "24h time format should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test 12-hour AM/PM time format
#[test]
fn test_time_12h_ampm_format() {
    let test_dir = create_test_dir("time_12h");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": { "time": "9:00AM-5:00PM" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "12h AM/PM time format should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test short AM/PM format without minutes
#[test]
fn test_time_short_ampm_format() {
    let test_dir = create_test_dir("time_short_ampm");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": { "time": "9AM-5PM" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "short AM/PM time format should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test overnight time range (crosses midnight)
#[test]
fn test_time_overnight_range() {
    let test_dir = create_test_dir("time_overnight");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+n",
                "action": "maximize",
                "when": { "time": "22:00-06:00" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "overnight time range should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test multiple time ranges separated by commas
#[test]
fn test_time_multiple_ranges() {
    let test_dir = create_test_dir("time_multiple");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": { "time": "09:00-12:00,14:00-18:00" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "multiple time ranges should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test midnight boundary (00:00)
#[test]
fn test_time_midnight_boundary() {
    let test_dir = create_test_dir("time_midnight");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "time": "00:00-06:00" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "midnight boundary should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test end of day boundary (23:59)
#[test]
fn test_time_end_of_day_boundary() {
    let test_dir = create_test_dir("time_end_of_day");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+e",
                "action": "maximize",
                "when": { "time": "18:00-23:59" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "end of day boundary should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test 12AM and 12PM edge cases
#[test]
fn test_time_12_oclock_edge_cases() {
    let test_dir = create_test_dir("time_12_oclock");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "noon_hour": { "time": "12PM-1PM" },
            "midnight_hour": { "time": "12AM-1AM" }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "12AM/12PM edge cases should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test mixed format time ranges
#[test]
fn test_time_mixed_formats() {
    let test_dir = create_test_dir("time_mixed");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": { "time": "9AM-12PM, 2PM-6PM" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "mixed format time ranges should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test single minute precision
#[test]
fn test_time_minute_precision() {
    let test_dir = create_test_dir("time_minute");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": { "time": "09:30-17:45" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "minute precision should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 2: Day Conditions (~8 tests)
// ============================================================================

/// test single day abbreviation
#[test]
fn test_day_single_abbreviation() {
    let test_dir = create_test_dir("day_single");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "time.day": "mon" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "single day abbreviation should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test full day name
#[test]
fn test_day_full_name() {
    let test_dir = create_test_dir("day_full");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "time.day": "monday" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "full day name should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test day range (mon-fri)
#[test]
fn test_day_range_weekdays() {
    let test_dir = create_test_dir("day_range");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": { "time.day": "mon-fri" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "day range should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test weekend range (sat-sun)
#[test]
fn test_day_range_weekend() {
    let test_dir = create_test_dir("day_weekend");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": { "time.day": "sat-sun" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "weekend range should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test day list (mon,wed,fri)
#[test]
fn test_day_list() {
    let test_dir = create_test_dir("day_list");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "time.day": "mon,wed,fri" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "day list should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test mixed day range and list (mon-wed,fri,sun)
#[test]
fn test_day_mixed_range_and_list() {
    let test_dir = create_test_dir("day_mixed");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "time.day": "mon-wed,fri,sun" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "mixed day range and list should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test day range wrap around (fri-mon)
#[test]
fn test_day_range_wrap_around() {
    let test_dir = create_test_dir("day_wrap");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "time.day": "fri-mon" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "day range wrap around should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test case insensitivity for days
#[test]
fn test_day_case_insensitive() {
    let test_dir = create_test_dir("day_case");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "lower": { "time.day": "mon" },
            "upper": { "time.day": "MON" },
            "mixed": { "time.day": "Mon" },
            "full_upper": { "time.day": "MONDAY" }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "day case insensitivity should work: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 3: Time + Day Combined (~4 tests)
// ============================================================================

/// test time and day combined in same condition
#[test]
fn test_time_and_day_combined() {
    let test_dir = create_test_dir("time_day_combined");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": {
                    "time": "9:00-17:00",
                    "time.day": "mon-fri"
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "time and day combined should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test work hours condition definition
#[test]
fn test_work_hours_condition_definition() {
    let test_dir = create_test_dir("work_hours_def");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "work_hours": {
                "time": "9:00AM-5:00PM",
                "time.day": "mon-fri"
            }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": { "$ref": "work_hours" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "work hours condition definition should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test night shift hours (overnight + specific days)
#[test]
fn test_night_shift_hours() {
    let test_dir = create_test_dir("night_shift");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "night_shift": {
                "time": "22:00-06:00",
                "time.day": "mon-fri"
            }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+n",
                "action": "maximize",
                "when": { "$ref": "night_shift" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "night shift hours should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test multiple time windows on specific days
#[test]
fn test_multiple_time_windows_specific_days() {
    let test_dir = create_test_dir("multi_time_days");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": {
                    "time": "09:00-12:00,14:00-18:00",
                    "time.day": "mon,wed,fri"
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "multiple time windows on specific days should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 4: Display Count Operators (~8 tests)
// ============================================================================

/// test display.count equals
#[test]
fn test_display_count_equals() {
    let test_dir = create_test_dir("display_eq");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { "==": 1 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count equals should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.count not equals
#[test]
fn test_display_count_not_equals() {
    let test_dir = create_test_dir("display_ne");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { "!=": 1 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count not equals should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.count greater than
#[test]
fn test_display_count_greater_than() {
    let test_dir = create_test_dir("display_gt");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { ">": 1 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count greater than should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.count greater than or equal
#[test]
fn test_display_count_greater_than_or_equal() {
    let test_dir = create_test_dir("display_gte");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { ">=": 2 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count greater than or equal should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.count less than
#[test]
fn test_display_count_less_than() {
    let test_dir = create_test_dir("display_lt");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { "<": 3 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count less than should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.count less than or equal
#[test]
fn test_display_count_less_than_or_equal() {
    let test_dir = create_test_dir("display_lte");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { "<=": 2 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count less than or equal should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.count simple equality (shorthand)
#[test]
fn test_display_count_simple_equality() {
    let test_dir = create_test_dir("display_simple_eq");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": 2 }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count simple equality should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.count range (combined operators)
#[test]
fn test_display_count_range() {
    let test_dir = create_test_dir("display_range");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { ">=": 2, "<=": 4 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.count range should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 5: Display Connected (~6 tests)
// ============================================================================

/// test display.connected with system alias (external)
#[test]
fn test_display_connected_external() {
    let test_dir = create_test_dir("display_external");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+e",
                "action": "move:external",
                "when": { "display.connected": "external" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.connected external should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.connected with system alias (builtin)
#[test]
fn test_display_connected_builtin() {
    let test_dir = create_test_dir("display_builtin");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "move:builtin",
                "when": { "display.connected": "builtin" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.connected builtin should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.connected with user-defined alias
#[test]
fn test_display_connected_user_alias() {
    let test_dir = create_test_dir("display_user_alias");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "display_aliases": {
            "office_monitor": ["10AC_D0B3_67890"]
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+o",
                "action": "move:office_monitor",
                "when": { "display.connected": "office_monitor" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.connected user alias should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.connected with in operator (multiple aliases)
#[test]
fn test_display_connected_in_multiple() {
    let test_dir = create_test_dir("display_in_multiple");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "display_aliases": {
            "office": ["10AC_D0B3_67890"],
            "home": ["1E6D_5B11_12345"]
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "move:external",
                "when": { "display.connected": { "in": ["office", "home", "external"] } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.connected with in operator should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test display.connected array shorthand for in
#[test]
fn test_display_connected_array_shorthand() {
    let test_dir = create_test_dir("display_array");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "move:external",
                "when": { "display.connected": ["external", "main"] }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "display.connected array shorthand should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test negated display.connected
#[test]
fn test_display_connected_negated() {
    let test_dir = create_test_dir("display_negated");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "not": { "display.connected": "external" } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "negated display.connected should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 6: App Running (~6 tests)
// ============================================================================

/// test app.running with single app
#[test]
fn test_app_running_single() {
    let test_dir = create_test_dir("app_running_single");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Slack",
                "when": { "app.running": "Slack" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.running single should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.running with in operator (any of multiple apps)
#[test]
fn test_app_running_in_multiple() {
    let test_dir = create_test_dir("app_running_in");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "focus",
                "app": "Safari",
                "when": { "app.running": { "in": ["Safari", "Chrome", "Firefox"] } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.running with in operator should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.running array shorthand
#[test]
fn test_app_running_array_shorthand() {
    let test_dir = create_test_dir("app_running_array");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "focus",
                "app": "Safari",
                "when": { "app.running": ["Safari", "Chrome"] }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.running array shorthand should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test negated app.running
#[test]
fn test_app_running_negated() {
    let test_dir = create_test_dir("app_running_negated");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Slack",
                "launch": true,
                "when": { "not": { "app.running": "Slack" } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "negated app.running should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.running with any operator
#[test]
fn test_app_running_any() {
    let test_dir = create_test_dir("app_running_any");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "maximize",
                "when": {
                    "any": [
                        { "app.running": "Safari" },
                        { "app.running": "Chrome" },
                        { "app.running": "Firefox" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.running with any operator should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.running combined with other conditions
#[test]
fn test_app_running_combined() {
    let test_dir = create_test_dir("app_running_combined");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Slack",
                "when": {
                    "all": [
                        { "app.running": "Slack" },
                        { "display.count": { ">=": 2 } }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.running combined should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 7: App Focused (~5 tests)
// ============================================================================

/// test app.focused with string value
#[test]
fn test_app_focused_string() {
    let test_dir = create_test_dir("app_focused_string");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "app.focused": "Safari" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.focused string should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.focused with boolean (any app focused)
#[test]
fn test_app_focused_boolean() {
    let test_dir = create_test_dir("app_focused_bool");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "app.focused": true }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.focused boolean should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.focused with in operator
#[test]
fn test_app_focused_in() {
    let test_dir = create_test_dir("app_focused_in");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "app.focused": { "in": ["Safari", "Chrome", "Firefox"] } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.focused with in operator should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.focused array shorthand
#[test]
fn test_app_focused_array_shorthand() {
    let test_dir = create_test_dir("app_focused_array");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "app.focused": ["Safari", "Chrome"] }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.focused array shorthand should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test negated app.focused
#[test]
fn test_app_focused_negated() {
    let test_dir = create_test_dir("app_focused_negated");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "not": { "app.focused": "Finder" } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "negated app.focused should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 8: App Window State (~6 tests)
// ============================================================================

/// test app.fullscreen true
#[test]
fn test_app_fullscreen_true() {
    let test_dir = create_test_dir("app_fullscreen_true");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+f",
                "action": "maximize",
                "when": { "app.fullscreen": true }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.fullscreen true should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.fullscreen false
#[test]
fn test_app_fullscreen_false() {
    let test_dir = create_test_dir("app_fullscreen_false");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+f",
                "action": "maximize",
                "when": { "app.fullscreen": false }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.fullscreen false should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.minimized true
#[test]
fn test_app_minimized_true() {
    let test_dir = create_test_dir("app_minimized_true");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+u",
                "action": "focus",
                "app": "Safari",
                "when": { "app.minimized": true }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.minimized true should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.minimized false
#[test]
fn test_app_minimized_false() {
    let test_dir = create_test_dir("app_minimized_false");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+u",
                "action": "focus",
                "app": "Safari",
                "when": { "app.minimized": false }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.minimized false should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test negated app.fullscreen
#[test]
fn test_app_fullscreen_negated() {
    let test_dir = create_test_dir("app_fullscreen_negated");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+f",
                "action": "maximize",
                "when": { "not": { "app.fullscreen": true } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "negated app.fullscreen should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test combined window state conditions
#[test]
fn test_window_state_combined() {
    let test_dir = create_test_dir("window_state_combined");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": {
                    "all": [
                        { "app.fullscreen": false },
                        { "app.minimized": false }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "combined window state conditions should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 9: App Field Matching (~4 tests)
// ============================================================================

/// test app field with simple string (fuzzy match)
#[test]
fn test_app_field_simple_string() {
    let test_dir = create_test_dir("app_field_simple");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "maximize",
                "when": { "app": "Safari" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app field simple string should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app field with in operator
#[test]
fn test_app_field_in_operator() {
    let test_dir = create_test_dir("app_field_in");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "maximize",
                "when": { "app": { "in": ["Safari", "Chrome", "Firefox"] } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app field with in operator should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app field array shorthand
#[test]
fn test_app_field_array_shorthand() {
    let test_dir = create_test_dir("app_field_array");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "maximize",
                "when": { "app": ["Safari", "Chrome"] }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app field array shorthand should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test app.display field
#[test]
fn test_app_display_field() {
    let test_dir = create_test_dir("app_display");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "move:next",
                "when": { "app.display": "builtin" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "app.display field should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 10: Comparison Operators Long Forms (~6 tests)
// ============================================================================

/// test equals long form
#[test]
fn test_operator_equals_long_form() {
    let test_dir = create_test_dir("op_equals");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "single": { "display.count": { "equals": 1 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "equals long form should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test not_equals long form
#[test]
fn test_operator_not_equals_long_form() {
    let test_dir = create_test_dir("op_not_equals");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "not_single": { "display.count": { "not_equals": 1 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "not_equals long form should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test greater_than long form
#[test]
fn test_operator_greater_than_long_form() {
    let test_dir = create_test_dir("op_greater_than");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "multi": { "display.count": { "greater_than": 1 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "greater_than long form should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test greater_than_or_equal long form
#[test]
fn test_operator_greater_than_or_equal_long_form() {
    let test_dir = create_test_dir("op_gte_long");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "at_least_two": { "display.count": { "greater_than_or_equal": 2 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "greater_than_or_equal long form should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test less_than long form
#[test]
fn test_operator_less_than_long_form() {
    let test_dir = create_test_dir("op_less_than");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "few": { "display.count": { "less_than": 3 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "less_than long form should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test less_than_or_equal long form
#[test]
fn test_operator_less_than_or_equal_long_form() {
    let test_dir = create_test_dir("op_lte_long");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "at_most_two": { "display.count": { "less_than_or_equal": 2 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "less_than_or_equal long form should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 11: Logical Operators (~8 tests)
// ============================================================================

/// test all operator with multiple conditions
#[test]
fn test_logical_all_multiple() {
    let test_dir = create_test_dir("logical_all");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": {
                    "all": [
                        { "display.count": { ">=": 2 } },
                        { "app.running": "Safari" },
                        { "time.day": "mon-fri" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "all operator with multiple conditions should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test any operator with multiple conditions
#[test]
fn test_logical_any_multiple() {
    let test_dir = create_test_dir("logical_any");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "focus",
                "app": "Safari",
                "when": {
                    "any": [
                        { "app.running": "Safari" },
                        { "app.running": "Chrome" },
                        { "app.running": "Firefox" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "any operator with multiple conditions should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test not operator
#[test]
fn test_logical_not() {
    let test_dir = create_test_dir("logical_not");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+n",
                "action": "maximize",
                "when": {
                    "not": { "app.fullscreen": true }
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "not operator should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test nested all inside any
#[test]
fn test_logical_nested_all_in_any() {
    let test_dir = create_test_dir("logical_nested_all_any");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+c",
                "action": "maximize",
                "when": {
                    "any": [
                        {
                            "all": [
                                { "display.count": { ">=": 2 } },
                                { "app.running": "Safari" }
                            ]
                        },
                        { "time.day": "sat-sun" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "nested all inside any should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test nested any inside all
#[test]
fn test_logical_nested_any_in_all() {
    let test_dir = create_test_dir("logical_nested_any_all");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+c",
                "action": "maximize",
                "when": {
                    "all": [
                        { "display.count": { ">=": 2 } },
                        {
                            "any": [
                                { "app.running": "Safari" },
                                { "app.running": "Chrome" }
                            ]
                        }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "nested any inside all should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test not inside all
#[test]
fn test_logical_not_in_all() {
    let test_dir = create_test_dir("logical_not_all");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+c",
                "action": "maximize",
                "when": {
                    "all": [
                        { "display.count": { ">=": 2 } },
                        { "not": { "app.fullscreen": true } }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "not inside all should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test deeply nested logical operators
#[test]
fn test_logical_deeply_nested() {
    let test_dir = create_test_dir("logical_deep");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": {
                    "all": [
                        {
                            "any": [
                                {
                                    "all": [
                                        { "display.count": { ">=": 2 } },
                                        { "not": { "app.fullscreen": true } }
                                    ]
                                },
                                { "time.day": "sat-sun" }
                            ]
                        },
                        { "app.running": "Safari" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "deeply nested logical operators should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test empty all (always true)
#[test]
fn test_logical_empty_all() {
    let test_dir = create_test_dir("logical_empty_all");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+e",
                "action": "maximize",
                "when": { "all": [] }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "empty all should be valid (always true): {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 12: $ref References (~6 tests)
// ============================================================================

/// test simple $ref reference
#[test]
fn test_ref_simple() {
    let test_dir = create_test_dir("ref_simple");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "$ref": "docked" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "simple $ref should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test $ref inside all
#[test]
fn test_ref_inside_all() {
    let test_dir = create_test_dir("ref_in_all");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } },
            "work_hours": { "time": "9:00-17:00", "time.day": "mon-fri" }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": {
                    "all": [
                        { "$ref": "docked" },
                        { "$ref": "work_hours" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "$ref inside all should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test $ref inside any
#[test]
fn test_ref_inside_any() {
    let test_dir = create_test_dir("ref_in_any");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "work_hours": { "time": "9:00-17:00", "time.day": "mon-fri" },
            "weekend": { "time.day": "sat-sun" }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": {
                    "any": [
                        { "$ref": "work_hours" },
                        { "$ref": "weekend" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "$ref inside any should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test $ref with not
#[test]
fn test_ref_with_not() {
    let test_dir = create_test_dir("ref_with_not");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "fullscreen": { "app.fullscreen": true }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+f",
                "action": "maximize",
                "when": { "not": { "$ref": "fullscreen" } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "$ref with not should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test multiple conditions using same $ref
#[test]
fn test_ref_reused() {
    let test_dir = create_test_dir("ref_reused");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+1",
                "action": "maximize",
                "when": { "$ref": "docked" }
            },
            {
                "keys": "ctrl+alt+2",
                "action": "move:external",
                "when": { "$ref": "docked" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "reused $ref should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test $ref in app_rules
#[test]
fn test_ref_in_app_rules() {
    let test_dir = create_test_dir("ref_app_rules");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [],
        "app_rules": [
            {
                "app": "Slack",
                "action": "move:external",
                "when": { "$ref": "docked" }
            }
        ],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "$ref in app_rules should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 13: Shorthand Syntax (~5 tests)
// ============================================================================

/// test implicit AND with multiple fields
#[test]
fn test_shorthand_implicit_and() {
    let test_dir = create_test_dir("shorthand_implicit_and");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+w",
                "action": "maximize",
                "when": {
                    "display.count": { ">=": 2 },
                    "app.running": "Safari",
                    "time.day": "mon-fri"
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "implicit AND should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test simple equality shorthand (no operator object)
#[test]
fn test_shorthand_simple_equality() {
    let test_dir = create_test_dir("shorthand_simple_eq");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "maximize",
                "when": { "display.count": 1 }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "simple equality shorthand should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test array shorthand for in operator
#[test]
fn test_shorthand_array_for_in() {
    let test_dir = create_test_dir("shorthand_array_in");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+b",
                "action": "maximize",
                "when": { "app.running": ["Safari", "Chrome", "Firefox"] }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "array shorthand for in should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test boolean true as always-true condition
#[test]
fn test_shorthand_boolean_true() {
    let test_dir = create_test_dir("shorthand_bool_true");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+t",
                "action": "maximize",
                "when": true
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "boolean true shorthand should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test boolean false as always-false condition
#[test]
fn test_shorthand_boolean_false() {
    let test_dir = create_test_dir("shorthand_bool_false");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+f",
                "action": "maximize",
                "when": false
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "boolean false shorthand should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 14: Error Handling & Invalid Inputs (~8 tests)
// ============================================================================

// helper function to check if JSON output indicates validation failure
fn json_output_has_errors(stdout: &str) -> bool {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
        if let Some(result) = json.get("result").and_then(|r| r.get("result")) {
            if let Some(valid) = result.get("valid").and_then(|v| v.as_bool()) {
                return !valid;
            }
        }
    }
    false
}

fn json_output_error_contains(stdout: &str, pattern: &str) -> bool {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
        if let Some(result) = json.get("result").and_then(|r| r.get("result")) {
            if let Some(errors) = result.get("errors").and_then(|e| e.as_array()) {
                return errors.iter().any(|e| {
                    e.as_str()
                        .map(|s| s.to_lowercase().contains(&pattern.to_lowercase()))
                        .unwrap_or(false)
                });
            }
        }
    }
    false
}

/// test unknown operator returns error
#[test]
fn test_error_unknown_operator() {
    let test_dir = create_test_dir("error_unknown_op");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { "???": 2 } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "unknown operator should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "unknown operator"),
        "error should mention unknown operator: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test undefined $ref returns error
#[test]
fn test_error_undefined_ref() {
    let test_dir = create_test_dir("error_undefined_ref");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {},
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "$ref": "nonexistent" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "undefined $ref should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "undefined"),
        "error should mention undefined reference: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test null value returns error
#[test]
fn test_error_null_value() {
    let test_dir = create_test_dir("error_null");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": null }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "null value should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "null"),
        "error should mention null: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test all with non-array returns error
#[test]
fn test_error_all_non_array() {
    let test_dir = create_test_dir("error_all_non_array");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "all": "not an array" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "all with non-array should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "array"),
        "error should mention array requirement: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test any with non-array returns error
#[test]
fn test_error_any_non_array() {
    let test_dir = create_test_dir("error_any_non_array");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "any": 123 }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "any with non-array should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "array"),
        "error should mention array requirement: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test $ref with non-string returns error
#[test]
fn test_error_ref_non_string() {
    let test_dir = create_test_dir("error_ref_non_string");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "$ref": 123 }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "$ref with non-string should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "string"),
        "error should mention string requirement: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test empty operator object returns error
#[test]
fn test_error_empty_operator_object() {
    let test_dir = create_test_dir("error_empty_op");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": {} }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "empty operator object should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "empty"),
        "error should mention empty operator: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test nested object as value returns error
#[test]
fn test_error_nested_object_value() {
    let test_dir = create_test_dir("error_nested_obj");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+d",
                "action": "maximize",
                "when": { "display.count": { "==": { "nested": "object" } } }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        json_output_has_errors(&stdout),
        "nested object as value should fail validation: {}",
        stdout
    );
    assert!(
        json_output_error_contains(&stdout, "nested")
            || json_output_error_contains(&stdout, "object"),
        "error should mention nested objects: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 15: Multiple Rules First-Match-Wins (~4 tests)
// ============================================================================

/// test multiple shortcuts with same keys, different conditions
#[test]
fn test_multiple_shortcuts_same_keys() {
    let test_dir = create_test_dir("multi_shortcuts");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "move:external",
                "when": { "display.count": { ">=": 2 } }
            },
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "display.count": 1 }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "multiple shortcuts with same keys should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test multiple app_rules for same app, different conditions
#[test]
fn test_multiple_app_rules_same_app() {
    let test_dir = create_test_dir("multi_app_rules");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [],
        "app_rules": [
            {
                "app": "Slack",
                "action": "move:external",
                "when": { "display.count": { ">=": 2 } }
            },
            {
                "app": "Slack",
                "action": "maximize",
                "when": { "display.count": 1 }
            }
        ],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "multiple app_rules for same app should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test fallback rule without condition
#[test]
fn test_fallback_rule_no_condition() {
    let test_dir = create_test_dir("fallback_rule");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "move:external",
                "when": { "display.count": { ">=": 2 } }
            },
            {
                "keys": "ctrl+alt+m",
                "action": "maximize"
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "fallback rule without condition should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test time-based fallback rules
#[test]
fn test_time_based_fallback_rules() {
    let test_dir = create_test_dir("time_fallback");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Slack",
                "when": { "time": "9:00-17:00", "time.day": "mon-fri" }
            },
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Discord"
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "verify"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "time-based fallback rules should be valid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// GROUP 16: Real Command Execution (~6 tests)
// ============================================================================

/// test list apps command works with conditions config
#[test]
fn test_command_list_apps_with_conditions() {
    let test_dir = create_test_dir("cmd_list_apps");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Safari",
                "when": { "$ref": "docked" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["list", "apps"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "list apps should work with conditions config: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test list displays command works with conditions config
#[test]
fn test_command_list_displays_with_conditions() {
    let test_dir = create_test_dir("cmd_list_displays");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "display_aliases": {
            "office": ["10AC_D0B3_67890"]
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["list", "displays"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "list displays should work with conditions config: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test config show with conditions
#[test]
fn test_command_config_show_with_conditions() {
    let test_dir = create_test_dir("cmd_config_show");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } },
            "work_hours": { "time": "9:00-17:00", "time.day": "mon-fri" }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": {
                    "all": [
                        { "$ref": "docked" },
                        { "$ref": "work_hours" }
                    ]
                }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "show"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "config show should work: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("conditions") || stdout.contains("docked") || stdout.contains("work_hours"),
        "config show should display conditions: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test config show --json with conditions
#[test]
fn test_command_config_show_json_with_conditions() {
    let test_dir = create_test_dir("cmd_config_show_json");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["config", "show", "--json"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "config show --json should work: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // verify it's valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "config show --json should output valid JSON: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

/// test version command works with conditions config
#[test]
fn test_command_version_with_conditions() {
    let test_dir = create_test_dir("cmd_version");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["version"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "version command should work with conditions config: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    cleanup_test_dir(&test_dir);
}

/// test daemon status command works with conditions config
#[test]
fn test_command_daemon_status_with_conditions() {
    let test_dir = create_test_dir("cmd_daemon_status");
    let config_path = test_dir.join("config.json");

    let config = r#"{
        "conditions": {
            "docked": { "display.count": { ">=": 2 } }
        },
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "maximize",
                "when": { "$ref": "docked" }
            }
        ],
        "app_rules": [],
        "settings": {}
    }"#;

    fs::write(&config_path, config).expect("Failed to write config");

    let output = run_cwm_with_env(
        &["daemon", "status"],
        &[("CWM_CONFIG", config_path.to_str().unwrap())],
    );

    // daemon status may return non-zero if daemon isn't running, but it shouldn't crash
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // check that it didn't crash with a condition parsing error
    assert!(
        !stderr.contains("condition") || !stderr.contains("error"),
        "daemon status should not fail due to conditions: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}
