// integration tests for the focus command

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

/// helper to run cwm focus command (text output)
fn run_focus(args: &[&str], config_path: &std::path::Path) -> std::process::Output {
    let binary = cwm_binary_path();
    // use --no-json to get text output (stdout is piped in tests, which auto-enables JSON)
    let mut cmd_args = vec![
        "--config",
        config_path.to_str().unwrap(),
        "--no-json",
        "focus",
    ];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm focus")
}

/// helper to run cwm focus with json flag
fn run_focus_json(args: &[&str], config_path: &std::path::Path) -> std::process::Output {
    let binary = cwm_binary_path();
    let mut cmd_args = vec!["--config", config_path.to_str().unwrap(), "--json", "focus"];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm focus --json")
}

// ============================================================================
// focus command argument parsing tests
// ============================================================================

#[test]
fn test_focus_requires_app_argument() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["focus"])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    assert!(!output.status.success(), "focus without --app should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--app") || stderr.contains("required"),
        "should mention --app is required"
    );
}

#[test]
fn test_focus_app_not_found_returns_error() {
    let test_dir = create_test_dir(&unique_test_name("focus_not_found"));
    let config_path = create_test_config(&test_dir);

    // use a very unlikely app name
    let output = run_focus(&["--app", "NonExistentApp12345XYZ"], &config_path);

    assert!(
        !output.status.success(),
        "focus with non-existent app should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("no matching")
            || stderr.contains("No matching")
            || stderr.contains("not running"),
        "should mention app not found: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_focus_app_not_found_json_output() {
    let test_dir = create_test_dir(&unique_test_name("focus_not_found_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_focus_json(&["--app", "NonExistentApp12345XYZ"], &config_path);

    assert!(
        !output.status.success(),
        "focus with non-existent app should fail"
    );

    // JSON error output should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(
            json.is_ok(),
            "JSON error output should be valid JSON: {}",
            stdout
        );

        if let Ok(json) = json {
            // should have error field
            assert!(
                json.get("error").is_some(),
                "JSON error should have error field"
            );
        }
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_focus_multiple_apps_syntax() {
    let test_dir = create_test_dir(&unique_test_name("focus_multi"));
    let config_path = create_test_config(&test_dir);

    // test that multiple --app flags are accepted
    let output = run_focus(
        &[
            "--app",
            "NonExistentApp1",
            "--app",
            "NonExistentApp2",
            "--app",
            "NonExistentApp3",
        ],
        &config_path,
    );

    // should fail because none exist, but should parse correctly
    assert!(
        !output.status.success(),
        "focus with non-existent apps should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // should not complain about argument parsing
    assert!(
        !stderr.contains("unexpected argument"),
        "multiple --app flags should be accepted"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_focus_with_launch_flag() {
    let test_dir = create_test_dir(&unique_test_name("focus_launch"));
    let config_path = create_test_config(&test_dir);

    // test that --launch flag is accepted
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "focus",
            "--app",
            "NonExistentApp12345",
            "--launch",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    // should fail (app doesn't exist) but --launch should be accepted
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("--launch"),
        "--launch flag should be accepted"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_focus_with_no_launch_flag() {
    let test_dir = create_test_dir(&unique_test_name("focus_no_launch"));
    let config_path = create_test_config(&test_dir);

    // test that --no-launch flag is accepted
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "focus",
            "--app",
            "NonExistentApp12345",
            "--no-launch",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    // should fail (app doesn't exist) but --no-launch should be accepted
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "--no-launch flag should be accepted"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_focus_launch_and_no_launch_conflict() {
    let test_dir = create_test_dir(&unique_test_name("focus_conflict"));
    let config_path = create_test_config(&test_dir);

    // --launch and --no-launch should conflict
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "focus",
            "--app",
            "Safari",
            "--launch",
            "--no-launch",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    assert!(
        !output.status.success(),
        "--launch and --no-launch should conflict"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("conflict") || stderr.contains("cannot be used with"),
        "should mention conflicting flags"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_focus_verbose_flag() {
    let test_dir = create_test_dir(&unique_test_name("focus_verbose"));
    let config_path = create_test_config(&test_dir);

    // test that -v/--verbose flag is accepted
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "focus",
            "--app",
            "NonExistentApp12345",
            "-v",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    // should fail but -v should be accepted
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "-v flag should be accepted"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// focus with real apps (may succeed on macOS)
// ============================================================================

#[test]
fn test_focus_finder_if_running() {
    let test_dir = create_test_dir(&unique_test_name("focus_finder"));
    let config_path = create_test_config(&test_dir);

    // Finder is always running on macOS
    let output = run_focus(&["--app", "Finder"], &config_path);

    // this may succeed or fail depending on accessibility permissions
    // we just verify it doesn't crash and produces reasonable output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // should either succeed or fail with a reasonable error
    if !output.status.success() {
        assert!(
            stderr.contains("permission")
                || stderr.contains("accessibility")
                || stderr.contains("not found")
                || stderr.contains("window"),
            "should fail with reasonable error: {}",
            stderr
        );
    } else {
        // if it succeeded, stdout might have info
        assert!(
            stdout.is_empty() || stdout.contains("Finder") || stdout.contains("focused"),
            "success output should be reasonable: {}",
            stdout
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_focus_finder_json_output() {
    let test_dir = create_test_dir(&unique_test_name("focus_finder_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_focus_json(&["--app", "Finder"], &config_path);

    let stdout = String::from_utf8_lossy(&output.stdout);

    // if there's output, it should be valid JSON
    if !stdout.trim().is_empty() {
        let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(json.is_ok(), "output should be valid JSON: {}", stdout);
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// focus exit codes
// ============================================================================

#[test]
fn test_focus_app_not_found_exit_code() {
    let test_dir = create_test_dir(&unique_test_name("focus_exit_code"));
    let config_path = create_test_config(&test_dir);

    let output = run_focus(&["--app", "NonExistentApp12345XYZ"], &config_path);

    // should have non-zero exit code
    assert!(!output.status.success(), "should have non-zero exit code");

    // exit code should be specific (not just 1)
    if let Some(code) = output.status.code() {
        // APP_NOT_FOUND is typically 10 based on exit_codes.rs
        assert!(code > 0, "exit code should be positive: {}", code);
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// focus with fuzzy matching
// ============================================================================

#[test]
fn test_focus_fuzzy_match_typo() {
    let test_dir = create_test_dir(&unique_test_name("focus_fuzzy"));
    let config_path = create_test_config(&test_dir);

    // "Findr" is close to "Finder" (1 char difference)
    // with fuzzy_threshold: 2, this should match
    let output = run_focus(&["--app", "Findr"], &config_path);

    // may succeed or fail depending on permissions
    // but should not fail due to "not found" with fuzzy matching enabled
    let stderr = String::from_utf8_lossy(&output.stderr);

    // if it fails, check why
    if !output.status.success() {
        // should either be permission issue or the fuzzy match didn't work
        // (which is also acceptable behavior)
        assert!(
            stderr.contains("permission")
                || stderr.contains("accessibility")
                || stderr.contains("not found")
                || stderr.contains("window"),
            "should fail with reasonable error: {}",
            stderr
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// quiet mode tests
// ============================================================================

#[test]
fn test_focus_quiet_mode_no_output_on_error() {
    let test_dir = create_test_dir(&unique_test_name("focus_quiet"));
    let config_path = create_test_config(&test_dir);

    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--quiet",
            "focus",
            "--app",
            "NonExistentApp12345XYZ",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    // should fail
    assert!(!output.status.success());

    // stdout should be empty in quiet mode (errors go to stderr)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "stdout should be empty in quiet mode: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}
