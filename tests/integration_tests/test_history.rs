// integration tests for undo/redo/history commands

use crate::common::*;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_test_name(prefix: &str) -> String {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();
    format!("{}_{:?}_{}", prefix, thread_id, count)
}

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
            },
            "history": {
                "enabled": true,
                "limit": 50,
                "flush_delay_ms": 2000
            }
        }
    });
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .expect("Failed to write test config");
    config_path
}

fn create_test_config_history_disabled(test_dir: &std::path::Path) -> std::path::PathBuf {
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
            },
            "history": {
                "enabled": false,
                "limit": 50,
                "flush_delay_ms": 2000
            }
        }
    });
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .expect("Failed to write test config");
    config_path
}

fn run_cwm_with_config(args: &[&str], config_path: &std::path::Path) -> std::process::Output {
    let binary = cwm_binary_path();
    // use --no-json to get text output (stdout is piped in tests, which auto-enables JSON)
    let mut cmd_args = vec!["--config", config_path.to_str().unwrap(), "--no-json"];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm")
}

fn run_cwm_with_config_json(args: &[&str], config_path: &std::path::Path) -> std::process::Output {
    let binary = cwm_binary_path();
    let mut cmd_args = vec!["--config", config_path.to_str().unwrap(), "--json"];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm")
}

// ============================================================================
// undo command tests
// ============================================================================

#[test]
fn test_undo_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("undo_daemon"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["undo"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Daemon not running") || stderr.contains("daemon"),
        "Expected daemon not running error, got: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_undo_json_output_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("undo_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config_json(&["undo"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should be valid JSON error response
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(json.is_ok(), "Expected valid JSON, got: {}", stdout);

    let json = json.unwrap();
    assert!(json.get("error").is_some(), "Expected error field in JSON");

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// redo command tests
// ============================================================================

#[test]
fn test_redo_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("redo_daemon"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["redo"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Daemon not running") || stderr.contains("daemon"),
        "Expected daemon not running error, got: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_redo_json_output_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("redo_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config_json(&["redo"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should be valid JSON error response
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(json.is_ok(), "Expected valid JSON, got: {}", stdout);

    let json = json.unwrap();
    assert!(json.get("error").is_some(), "Expected error field in JSON");

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// history command tests
// ============================================================================

#[test]
fn test_history_requires_subcommand() {
    let test_dir = create_test_dir(&unique_test_name("history_subcommand"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["history"], &config_path);

    // should fail without subcommand
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("subcommand") || stderr.contains("Usage") || stderr.contains("required"),
        "Expected subcommand required error, got: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_invalid_subcommand() {
    let test_dir = create_test_dir(&unique_test_name("history_invalid"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["history", "invalid"], &config_path);

    // should fail with invalid subcommand
    assert!(!output.status.success());

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_list_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("history_list_daemon"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["history", "list"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Daemon not running") || stderr.contains("daemon"),
        "Expected daemon not running error, got: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_list_json_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("history_list_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config_json(&["history", "list"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should be valid JSON error response
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(json.is_ok(), "Expected valid JSON, got: {}", stdout);

    let json = json.unwrap();
    assert!(json.get("error").is_some(), "Expected error field in JSON");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_clear_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("history_clear_daemon"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["history", "clear"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Daemon not running") || stderr.contains("daemon"),
        "Expected daemon not running error, got: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_clear_json_requires_daemon() {
    let test_dir = create_test_dir(&unique_test_name("history_clear_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config_json(&["history", "clear"], &config_path);

    // should fail when daemon is not running
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should be valid JSON error response
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(json.is_ok(), "Expected valid JSON, got: {}", stdout);

    let json = json.unwrap();
    assert!(json.get("error").is_some(), "Expected error field in JSON");

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// config tests for history settings
// ============================================================================

#[test]
fn test_config_has_history_settings() {
    let test_dir = create_test_dir(&unique_test_name("config_history"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config_json(&["config", "show"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    // navigate to result.result.settings.history (config show has nested result)
    let result = json.get("result").expect("Missing result field");
    let inner_result = result.get("result").expect("Missing inner result field");
    let settings = inner_result
        .get("settings")
        .expect("Missing settings field");
    let history = settings.get("history").expect("Missing history field");

    assert!(history.get("enabled").is_some(), "Missing history.enabled");
    assert!(history.get("limit").is_some(), "Missing history.limit");
    assert!(
        history.get("flush_delay_ms").is_some(),
        "Missing history.flush_delay_ms"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_history_default_values() {
    let test_dir = create_test_dir(&unique_test_name("config_history_defaults"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config_json(&["config", "show"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    let result = json.get("result").expect("Missing result field");
    let inner_result = result.get("result").expect("Missing inner result field");
    let settings = inner_result
        .get("settings")
        .expect("Missing settings field");
    let history = settings.get("history").expect("Missing history field");

    // check default values match what we set in config
    assert_eq!(history.get("enabled").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(history.get("limit").and_then(|v| v.as_u64()), Some(50));
    assert_eq!(
        history.get("flush_delay_ms").and_then(|v| v.as_u64()),
        Some(2000)
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_config_history_disabled() {
    let test_dir = create_test_dir(&unique_test_name("config_history_disabled"));
    let config_path = create_test_config_history_disabled(&test_dir);

    let output = run_cwm_with_config_json(&["config", "show"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    let result = json.get("result").expect("Missing result field");
    let inner_result = result.get("result").expect("Missing inner result field");
    let settings = inner_result
        .get("settings")
        .expect("Missing settings field");
    let history = settings.get("history").expect("Missing history field");

    assert_eq!(
        history.get("enabled").and_then(|v| v.as_bool()),
        Some(false)
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// help text tests
// ============================================================================

#[test]
fn test_undo_help() {
    let test_dir = create_test_dir(&unique_test_name("undo_help"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["undo", "--help"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("undo") || stdout.contains("Undo"),
        "Expected undo in help text, got: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_redo_help() {
    let test_dir = create_test_dir(&unique_test_name("redo_help"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["redo", "--help"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("redo") || stdout.contains("Redo"),
        "Expected redo in help text, got: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_help() {
    let test_dir = create_test_dir(&unique_test_name("history_help"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["history", "--help"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("history") || stdout.contains("History"),
        "Expected history in help text, got: {}",
        stdout
    );
    assert!(
        stdout.contains("list") && stdout.contains("clear"),
        "Expected list and clear subcommands in help, got: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_list_help() {
    let test_dir = create_test_dir(&unique_test_name("history_list_help"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["history", "list", "--help"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("list") || stdout.contains("List"),
        "Expected list in help text, got: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_history_clear_help() {
    let test_dir = create_test_dir(&unique_test_name("history_clear_help"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["history", "clear", "--help"], &config_path);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("clear") || stdout.contains("Clear"),
        "Expected clear in help text, got: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}
