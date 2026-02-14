// integration tests for the spotlight command

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

/// create a test config with spotlight shortcuts
fn create_config_with_spotlight(test_dir: &std::path::Path) -> std::path::PathBuf {
    let config_path = test_dir.join("config.json");
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
            },
            {
                "name": "Move to Next Display",
                "action": "move_display:next"
            },
            {
                "name": "Resize 80%",
                "action": "resize:80"
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
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .expect("Failed to write test config");
    config_path
}

/// helper to run cwm spotlight command (text output)
fn run_spotlight(args: &[&str], config_path: &std::path::Path) -> std::process::Output {
    let binary = cwm_binary_path();
    // use --no-json to get text output (stdout is piped in tests, which auto-enables JSON)
    let mut cmd_args = vec![
        "--config",
        config_path.to_str().unwrap(),
        "--no-json",
        "spotlight",
    ];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm spotlight")
}

// ============================================================================
// spotlight example tests
// ============================================================================

#[test]
fn test_spotlight_example_produces_output() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_example"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["example"], &config_path);

    assert!(output.status.success(), "spotlight example should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "spotlight example should produce output"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_spotlight_example_contains_json() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_example_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["example"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // the output contains instructions and JSON, not pure JSON
    // verify it contains the expected JSON structure
    assert!(
        stdout.contains("\"spotlight\"") && stdout.contains("\"action\""),
        "spotlight example should contain JSON config: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_spotlight_example_has_spotlight_array() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_example_array"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["example"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // the output contains instructions and JSON
    // verify it contains the spotlight array structure
    assert!(
        stdout.contains("\"spotlight\":") || stdout.contains("\"spotlight\": "),
        "example should contain 'spotlight' field"
    );
    assert!(
        stdout.contains("[") && stdout.contains("]"),
        "example should contain an array"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_spotlight_example_has_valid_shortcuts() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_example_valid"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["example"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // the output contains instructions and JSON
    // verify it contains expected shortcut fields
    assert!(
        stdout.contains("\"name\""),
        "example should contain 'name' field"
    );
    assert!(
        stdout.contains("\"action\""),
        "example should contain 'action' field"
    );
    // focus shortcuts should have app
    assert!(
        stdout.contains("\"app\""),
        "example should contain 'app' field for focus actions"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// spotlight list tests
// ============================================================================

#[test]
fn test_spotlight_list_succeeds() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_list_empty"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["list"], &config_path);

    assert!(output.status.success(), "spotlight list should succeed");

    // spotlight list shows installed shortcuts (from ~/Applications/cwm/)
    // it may show shortcuts installed by the user, or be empty
    // just verify it doesn't crash and produces some output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("spotlight")
            || stdout.contains("Installed")
            || stdout.contains("No")
            || stdout.contains("Total"),
        "should produce reasonable output: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_spotlight_list_with_shortcuts() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_list_shortcuts"));
    let config_path = create_config_with_spotlight(&test_dir);

    let output = run_spotlight(&["list"], &config_path);

    assert!(output.status.success(), "spotlight list should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should list the configured shortcuts
    // note: this lists installed shortcuts, not configured ones
    // so it may be empty if nothing is installed

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// spotlight subcommand tests
// ============================================================================

#[test]
fn test_spotlight_requires_subcommand() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["spotlight"])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    assert!(
        !output.status.success(),
        "spotlight without subcommand should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("subcommand") || stderr.contains("required"),
        "should mention subcommand required"
    );
}

#[test]
fn test_spotlight_invalid_subcommand() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_invalid"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["invalid"], &config_path);

    assert!(
        !output.status.success(),
        "spotlight with invalid subcommand should fail"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// spotlight install tests (may require permissions)
// ============================================================================

#[test]
fn test_spotlight_install_with_empty_config() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_install_empty"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["install"], &config_path);

    // should succeed but indicate nothing to install
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // either succeeds with "no shortcuts" message or fails gracefully
    if output.status.success() {
        assert!(
            stdout.contains("No") || stdout.contains("0") || stdout.trim().is_empty(),
            "should indicate no shortcuts to install"
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// spotlight remove tests
// ============================================================================

#[test]
fn test_spotlight_remove_nonexistent() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_remove"));
    let config_path = create_test_config(&test_dir);

    let output = run_spotlight(&["remove", "NonExistentShortcut"], &config_path);

    // should handle gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // either succeeds (nothing to remove) or fails with reasonable error
    if !output.status.success() {
        assert!(
            stderr.contains("not found") || stderr.contains("does not exist"),
            "should mention shortcut not found"
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// spotlight JSON output tests
// ============================================================================

#[test]
fn test_spotlight_list_output() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_list_json"));
    let config_path = create_test_config(&test_dir);

    // spotlight list currently only supports text output
    let output = run_spotlight(&["list"], &config_path);

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // should contain some output about shortcuts
        assert!(
            stdout.contains("spotlight")
                || stdout.contains("shortcut")
                || stdout.contains("Total")
                || stdout.contains("No"),
            "spotlight list should produce reasonable output: {}",
            stdout
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// spotlight shortcut name validation
// ============================================================================

#[test]
fn test_spotlight_config_with_empty_name() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_empty_name"));
    let config_path = test_dir.join("config.json");
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [
            {
                "name": "",  // empty name
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
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .expect("Failed to write config");

    // verify should fail
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "verify",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    assert!(
        !output.status.success(),
        "config with empty spotlight name should fail verification"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_spotlight_config_with_invalid_action() {
    let test_dir = create_test_dir(&unique_test_name("spotlight_invalid_action"));
    let config_path = test_dir.join("config.json");
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [
            {
                "name": "Invalid Action",
                "action": "invalid_action"
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
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .expect("Failed to write config");

    // verify should fail
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "verify",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    assert!(
        !output.status.success(),
        "config with invalid spotlight action should fail verification"
    );

    cleanup_test_dir(&test_dir);
}
