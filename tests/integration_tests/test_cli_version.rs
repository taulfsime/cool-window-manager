// integration tests for version command

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
fn test_version_command_shows_version() {
    let test_dir = create_test_dir(&unique_test_name("version_show"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["version"], &config_path);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should contain cwm and version info
    assert!(stdout.contains("cwm"));
    assert!(stdout.contains("Built:"));
    assert!(stdout.contains("Repository:"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_version_command_shows_github_repo() {
    let test_dir = create_test_dir(&unique_test_name("version_repo"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["version"], &config_path);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("github.com"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_version_command_with_json_flag() {
    let test_dir = create_test_dir(&unique_test_name("version_json"));
    let config_path = create_test_config(&test_dir);

    // version command doesn't support JSON output, but should still work
    let output = run_cwm_with_config(&["--json", "version"], &config_path);
    // should succeed (JSON flag is ignored for version)
    assert!(output.status.success());

    cleanup_test_dir(&test_dir);
}
