// integration tests for move-display command

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
    let mut cmd_args = vec!["--config", config_path.to_str().unwrap()];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm")
}

#[test]
fn test_move_display_requires_target() {
    let test_dir = create_test_dir(&unique_test_name("move_display_target"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["move-display"], &config_path);
    // should fail without target
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required") || stderr.contains("Usage"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_display_app_not_found() {
    let test_dir = create_test_dir(&unique_test_name("move_display_notfound"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move-display",
            "next",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    // should fail when app not found
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_display_app_not_found_json() {
    let test_dir = create_test_dir(&unique_test_name("move_display_notfound_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--json",
            "move-display",
            "next",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    // should fail when app not found
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON error response
    assert!(stdout.contains("error") || stdout.contains("not found"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_display_with_no_launch_flag() {
    let test_dir = create_test_dir(&unique_test_name("move_display_nolaunch"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move-display",
            "next",
            "--app",
            "NonExistentApp12345",
            "--no-launch",
        ],
        &config_path,
    );
    // should fail when app not found and --no-launch
    assert!(!output.status.success());

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_display_launch_and_no_launch_conflict() {
    let test_dir = create_test_dir(&unique_test_name("move_display_conflict"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &["move-display", "next", "--launch", "--no-launch"],
        &config_path,
    );
    // should fail - conflicting flags
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with") || stderr.contains("conflict"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_display_numeric_target() {
    let test_dir = create_test_dir(&unique_test_name("move_display_numeric"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move-display",
            "0",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    // numeric target should parse correctly
    // will fail because app not found, but target parsing should work
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // should fail due to app not found, not target parsing
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}
