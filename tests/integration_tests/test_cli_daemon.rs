// integration tests for daemon commands

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

#[test]
fn test_daemon_status_when_not_running() {
    let test_dir = create_test_dir(&unique_test_name("daemon_status"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["daemon", "status"], &config_path);
    // should succeed even when daemon is not running
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not running"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_daemon_stop_when_not_running() {
    let test_dir = create_test_dir(&unique_test_name("daemon_stop"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["daemon", "stop"], &config_path);
    // should fail when daemon is not running
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not running"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_daemon_requires_subcommand() {
    let test_dir = create_test_dir(&unique_test_name("daemon_subcommand"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["daemon"], &config_path);
    // should fail without subcommand
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("subcommand") || stderr.contains("Usage"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_daemon_invalid_subcommand() {
    let test_dir = create_test_dir(&unique_test_name("daemon_invalid"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["daemon", "invalid"], &config_path);
    // should fail with invalid subcommand
    assert!(!output.status.success());

    cleanup_test_dir(&test_dir);
}
