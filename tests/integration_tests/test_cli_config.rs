// integration tests for the config command

use crate::common::*;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

// counter for unique test directory names
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// create a unique test directory name
fn unique_test_name(prefix: &str) -> String {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();
    format!("{}_{:?}_{}", prefix, thread_id, count)
}

/// create a test config file with updates disabled
fn create_test_config(test_dir: &std::path::Path) -> std::path::PathBuf {
    let config_path = test_dir.join("config.json");
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .expect("Failed to write test config");
    config_path
}

/// create a test config with specific content
fn create_config_with_content(
    test_dir: &std::path::Path,
    content: &serde_json::Value,
) -> std::path::PathBuf {
    let config_path = test_dir.join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(content).unwrap())
        .expect("Failed to write test config");
    config_path
}

/// helper to run cwm config command (text output)
fn run_config(args: &[&str], config_path: &std::path::Path) -> std::process::Output {
    let binary = cwm_binary_path();
    // use --no-json to get text output (stdout is piped in tests, which auto-enables JSON)
    let mut cmd_args = vec![
        "--config",
        config_path.to_str().unwrap(),
        "--no-json",
        "config",
    ];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm config")
}

// ============================================================================
// config show tests
// ============================================================================

#[test]
fn test_config_show_displays_config() {
    let test_dir = create_test_dir(&unique_test_name("config_show"));
    let config_path = create_test_config(&test_dir);

    let output = run_config(&["show"], &config_path);

    assert!(output.status.success(), "config show should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should contain JSON config
    assert!(stdout.contains("shortcuts"), "should show shortcuts field");
    assert!(stdout.contains("settings"), "should show settings field");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_show_json_output() {
    let test_dir = create_test_dir(&unique_test_name("config_show_json"));
    let config_path = create_test_config(&test_dir);

    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "config",
            "show",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "config show --json should succeed.\nstdout: {}\nstderr: {}\nexit code: {:?}",
        stdout,
        stderr,
        output.status.code()
    );

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("config show --json should produce valid JSON");

    // output is JSON-RPC wrapped, extract the result
    let result = json
        .get("result")
        .and_then(|r| r.get("result"))
        .expect("should have result.result");

    // should have config fields
    assert!(result.get("shortcuts").is_some(), "should have shortcuts");
    assert!(result.get("settings").is_some(), "should have settings");

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config path tests
// ============================================================================

#[test]
fn test_config_path_shows_path() {
    let test_dir = create_test_dir(&unique_test_name("config_path"));
    let config_path = create_test_config(&test_dir);

    let output = run_config(&["path"], &config_path);

    assert!(output.status.success(), "config path should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should contain the config path
    assert!(
        stdout.contains(config_path.to_str().unwrap()),
        "should show the config path"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config set tests
// ============================================================================

#[test]
fn test_config_set_fuzzy_threshold() {
    let test_dir = create_test_dir(&unique_test_name("config_set_fuzzy"));
    let config_path = create_test_config(&test_dir);

    // use full path: settings.fuzzy_threshold
    let output = run_config(&["set", "settings.fuzzy_threshold", "5"], &config_path);

    assert!(
        output.status.success(),
        "config set settings.fuzzy_threshold should succeed"
    );

    // verify the config was updated
    let config_content = fs::read_to_string(&config_path).expect("Failed to read config");
    let config: serde_json::Value =
        serde_json::from_str(&config_content).expect("Failed to parse config");

    assert_eq!(
        config["settings"]["fuzzy_threshold"].as_u64(),
        Some(5),
        "fuzzy_threshold should be updated to 5"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_set_launch_true() {
    let test_dir = create_test_dir(&unique_test_name("config_set_launch"));
    let config_path = create_test_config(&test_dir);

    // use full path: settings.launch
    let output = run_config(&["set", "settings.launch", "true"], &config_path);

    assert!(
        output.status.success(),
        "config set settings.launch should succeed"
    );

    // verify the config was updated
    let config_content = fs::read_to_string(&config_path).expect("Failed to read config");
    let config: serde_json::Value =
        serde_json::from_str(&config_content).expect("Failed to parse config");

    assert_eq!(
        config["settings"]["launch"].as_bool(),
        Some(true),
        "launch should be updated to true"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_set_animate() {
    let test_dir = create_test_dir(&unique_test_name("config_set_animate"));
    let config_path = create_test_config(&test_dir);

    // use full path: settings.animate
    let output = run_config(&["set", "settings.animate", "true"], &config_path);

    assert!(
        output.status.success(),
        "config set settings.animate should succeed"
    );

    // verify the config was updated
    let config_content = fs::read_to_string(&config_path).expect("Failed to read config");
    let config: serde_json::Value =
        serde_json::from_str(&config_content).expect("Failed to parse config");

    assert_eq!(
        config["settings"]["animate"].as_bool(),
        Some(true),
        "animate should be updated to true"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_set_delay_ms() {
    let test_dir = create_test_dir(&unique_test_name("config_set_delay"));
    let config_path = create_test_config(&test_dir);

    // use full path: settings.delay_ms
    let output = run_config(&["set", "settings.delay_ms", "1000"], &config_path);

    assert!(
        output.status.success(),
        "config set settings.delay_ms should succeed"
    );

    // verify the config was updated
    let config_content = fs::read_to_string(&config_path).expect("Failed to read config");
    let config: serde_json::Value =
        serde_json::from_str(&config_content).expect("Failed to parse config");

    assert_eq!(
        config["settings"]["delay_ms"].as_u64(),
        Some(1000),
        "delay_ms should be updated to 1000"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_set_invalid_value() {
    let test_dir = create_test_dir(&unique_test_name("config_set_invalid_val"));
    let config_path = create_test_config(&test_dir);

    // fuzzy_threshold should be a number, not a string
    let output = run_config(&["set", "fuzzy_threshold", "not_a_number"], &config_path);

    assert!(
        !output.status.success(),
        "config set with invalid value should fail"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config verify tests
// ============================================================================

#[test]
fn test_config_verify_valid_config() {
    let test_dir = create_test_dir(&unique_test_name("config_verify_valid"));
    let config_path = create_test_config(&test_dir);

    let output = run_config(&["verify"], &config_path);

    assert!(
        output.status.success(),
        "config verify with valid config should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("valid")
            || stdout.contains("Valid")
            || stdout.contains("OK")
            || stdout.contains("✓"),
        "should indicate config is valid"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_verify_invalid_shortcut_action() {
    let test_dir = create_test_dir(&unique_test_name("config_verify_invalid"));
    let config = serde_json::json!({
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "invalid_action"
            }
        ],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with invalid action should fail"
    );

    // error message goes to stdout for verify command
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("invalid") || combined.contains("Invalid") || combined.contains("action"),
        "should mention invalid action: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_verify_focus_without_app() {
    let test_dir = create_test_dir(&unique_test_name("config_verify_focus"));
    let config = serde_json::json!({
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus"
                // missing app field
            }
        ],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with focus action without app should fail"
    );

    // error message may go to stdout for verify command
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("app") || combined.contains("focus"),
        "should mention missing app for focus action: stdout={}, stderr={}",
        stdout,
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_verify_invalid_hotkey() {
    let test_dir = create_test_dir(&unique_test_name("config_verify_hotkey"));
    let config = serde_json::json!({
        "shortcuts": [
            {
                "keys": "ctrl+alt+",  // invalid - no key after +
                "action": "maximize"
            }
        ],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with invalid hotkey should fail"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_verify_invalid_move_display_target() {
    let test_dir = create_test_dir(&unique_test_name("config_verify_move"));
    let config = serde_json::json!({
        "shortcuts": [
            {
                "keys": "ctrl+alt+m",
                "action": "move_display:invalid target!"  // invalid characters
            }
        ],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with invalid move_display target should fail"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_verify_invalid_resize_value() {
    let test_dir = create_test_dir(&unique_test_name("config_verify_resize"));
    let config = serde_json::json!({
        "shortcuts": [
            {
                "keys": "ctrl+alt+r",
                "action": "resize:150"  // invalid - over 100%
            }
        ],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with invalid resize value should fail"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config default tests
// ============================================================================

#[test]
fn test_config_default_shows_default() {
    let test_dir = create_test_dir(&unique_test_name("config_default"));
    let config_path = create_test_config(&test_dir);

    let output = run_config(&["default"], &config_path);

    assert!(output.status.success(), "config default should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should contain default config JSON
    assert!(
        stdout.contains("shortcuts") && stdout.contains("settings"),
        "should show default config structure"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config with --config flag tests
// ============================================================================

#[test]
fn test_config_override_nonexistent_file_fails() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            "/nonexistent/path/config.json",
            "config",
            "show",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    assert!(
        !output.status.success(),
        "config with nonexistent file should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("No such file")
            || stderr.contains("does not exist"),
        "should mention file not found"
    );
}

#[test]
fn test_config_override_invalid_json_fails() {
    let test_dir = create_test_dir(&unique_test_name("config_invalid_json"));
    let config_path = test_dir.join("config.json");
    fs::write(&config_path, "{ invalid json }").expect("Failed to write invalid config");

    let output = run_config(&["show"], &config_path);

    assert!(
        !output.status.success(),
        "config with invalid JSON should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("parse") || stderr.contains("JSON") || stderr.contains("invalid"),
        "should mention parse error"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config with shortcuts tests
// ============================================================================

#[test]
fn test_config_with_valid_shortcuts() {
    let test_dir = create_test_dir(&unique_test_name("config_shortcuts"));
    let config = serde_json::json!({
        "shortcuts": [
            {
                "keys": "ctrl+alt+s",
                "action": "focus",
                "app": "Safari",
                "launch": true
            },
            {
                "keys": "ctrl+alt+m",
                "action": "maximize"
            },
            {
                "keys": "ctrl+alt+n",
                "action": "move:next"
            },
            {
                "keys": "ctrl+alt+r",
                "action": "resize:80"
            }
        ],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        output.status.success(),
        "config verify with valid shortcuts should succeed"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config with display aliases tests
// ============================================================================

#[test]
fn test_config_with_valid_display_aliases() {
    let test_dir = create_test_dir(&unique_test_name("config_aliases"));
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {
            "office": ["ABC123"],
            "home": ["DEF456", "GHI789"]
        },
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        output.status.success(),
        "config verify with valid display aliases should succeed"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_with_invalid_display_alias_name() {
    let test_dir = create_test_dir(&unique_test_name("config_alias_invalid"));
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {
            "invalid name!": ["ABC123"]  // spaces and special chars not allowed
        },
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with invalid alias name should fail"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config with app rules tests
// ============================================================================

#[test]
fn test_config_with_valid_app_rules() {
    let test_dir = create_test_dir(&unique_test_name("config_app_rules"));
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [
            {
                "app": "Terminal",
                "action": "maximize",
                "delay_ms": 500
            },
            {
                "app": "Safari",
                "action": "resize:80"
            }
        ],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        output.status.success(),
        "config verify with valid app rules should succeed"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_with_invalid_app_rule_action() {
    let test_dir = create_test_dir(&unique_test_name("config_app_rule_invalid"));
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [
            {
                "app": "Terminal",
                "action": "invalid_action"
            }
        ],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with invalid app rule action should fail"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config with spotlight tests
// ============================================================================

#[test]
fn test_config_with_valid_spotlight() {
    let test_dir = create_test_dir(&unique_test_name("config_spotlight"));
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
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
        ],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        output.status.success(),
        "config verify with valid spotlight should succeed"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_with_spotlight_focus_without_app() {
    let test_dir = create_test_dir(&unique_test_name("config_spotlight_no_app"));
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [
            {
                "name": "Focus Something",
                "action": "focus"
                // missing app
            }
        ],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    let config_path = create_config_with_content(&test_dir, &config);

    let output = run_config(&["verify"], &config_path);

    assert!(
        !output.status.success(),
        "config verify with spotlight focus without app should fail"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// JSONC support tests
// ============================================================================

#[test]
fn test_config_jsonc_with_comments() {
    let test_dir = create_test_dir(&unique_test_name("config_jsonc"));
    let config_path = test_dir.join("config.jsonc");

    let jsonc_content = r#"{
        // this is a comment
        "shortcuts": [],
        "app_rules": [],
        /* multi-line
           comment */
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    }"#;

    fs::write(&config_path, jsonc_content).expect("Failed to write JSONC config");

    let output = run_config(&["verify"], &config_path);

    assert!(
        output.status.success(),
        "config verify with JSONC comments should succeed"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_jsonc_with_trailing_commas() {
    let test_dir = create_test_dir(&unique_test_name("config_jsonc_trailing"));
    let config_path = test_dir.join("config.jsonc");

    let jsonc_content = r#"{
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false,
            },
        },
    }"#;

    fs::write(&config_path, jsonc_content).expect("Failed to write JSONC config");

    let output = run_config(&["verify"], &config_path);

    assert!(
        output.status.success(),
        "config verify with JSONC trailing commas should succeed"
    );

    cleanup_test_dir(&test_dir);
}
