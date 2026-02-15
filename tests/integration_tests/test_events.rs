// integration tests for the events command

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

/// helper to run cwm command
fn run_cwm(args: &[&str]) -> std::process::Output {
    let test_dir = create_test_dir(&unique_test_name("events"));
    let config_path = create_test_config(&test_dir);

    let binary = cwm_binary_path();

    let mut cmd_args = vec!["--config", config_path.to_str().unwrap()];
    cmd_args.extend(args);

    let output = Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    cleanup_test_dir(&test_dir);
    output
}

// ============================================================================
// list events tests
// ============================================================================

#[test]
fn test_list_events_succeeds() {
    let output = run_cwm(&["list", "events", "--no-json"]);
    assert!(
        output.status.success(),
        "list events should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_list_events_shows_all_event_types() {
    let output = run_cwm(&["list", "events", "--no-json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // check that all event types are listed
    assert!(stdout.contains("app.launched"), "should list app.launched");
    assert!(stdout.contains("app.focused"), "should list app.focused");
    assert!(
        stdout.contains("app.terminated"),
        "should list app.terminated"
    );
    assert!(
        stdout.contains("window.maximized"),
        "should list window.maximized"
    );
    assert!(
        stdout.contains("window.resized"),
        "should list window.resized"
    );
    assert!(stdout.contains("window.moved"), "should list window.moved");
    assert!(
        stdout.contains("window.closed"),
        "should list window.closed"
    );
    assert!(
        stdout.contains("display.connected"),
        "should list display.connected"
    );
    assert!(
        stdout.contains("display.disconnected"),
        "should list display.disconnected"
    );
}

#[test]
fn test_list_events_shows_patterns() {
    let output = run_cwm(&["list", "events", "--no-json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // check that patterns are shown
    assert!(stdout.contains("Patterns:"), "should show patterns section");
    assert!(stdout.contains("app.*"), "should show app.* pattern");
    assert!(stdout.contains("window.*"), "should show window.* pattern");
    assert!(stdout.contains("daemon.*"), "should show daemon.* pattern");
}

#[test]
fn test_list_events_detailed() {
    let output = run_cwm(&["list", "events", "--detailed", "--no-json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // detailed output should include descriptions
    assert!(
        stdout.contains("Display was connected"),
        "should show description for display.connected"
    );
    assert!(
        stdout.contains("Application was focused"),
        "should show description for app.focused"
    );
}

#[test]
fn test_list_events_json() {
    let output = run_cwm(&["list", "events", "--json"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // extract result from JSON-RPC wrapper
    let result = json.get("result").expect("should have result field");

    // check JSON structure
    assert!(result.get("items").is_some(), "should have items array");
    let items = result["items"].as_array().unwrap();
    assert_eq!(items.len(), 9, "should have 9 event types");

    // check that each item has a name
    for item in items {
        assert!(item.get("name").is_some(), "each item should have name");
    }
}

#[test]
fn test_list_events_names_output() {
    let output = run_cwm(&["list", "events", "--names"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // should have one event per line
    assert_eq!(lines.len(), 9, "should have 9 event types");
    assert!(lines.contains(&"display.connected"));
    assert!(lines.contains(&"app.launched"));
    assert!(lines.contains(&"window.resized"));
}

// ============================================================================
// events listen tests (without daemon running)
// ============================================================================

#[test]
fn test_events_listen_requires_daemon() {
    let output = run_cwm(&["events", "listen"]);

    // should fail because daemon is not running
    assert!(!output.status.success(), "should fail without daemon");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not running") || stderr.contains("daemon"),
        "should mention daemon not running: {}",
        stderr
    );
}

#[test]
fn test_events_wait_requires_daemon() {
    let output = run_cwm(&[
        "events",
        "wait",
        "--event",
        "app.launched",
        "--timeout",
        "1",
    ]);

    // should fail because daemon is not running
    assert!(!output.status.success(), "should fail without daemon");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not running") || stderr.contains("daemon"),
        "should mention daemon not running: {}",
        stderr
    );
}

// ============================================================================
// events command help tests
// ============================================================================

#[test]
fn test_events_help() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["events", "--help"])
        .output()
        .expect("Failed to run cwm events --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("listen"), "should show listen subcommand");
    assert!(stdout.contains("wait"), "should show wait subcommand");
}

#[test]
fn test_events_listen_help() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["events", "listen", "--help"])
        .output()
        .expect("Failed to run cwm events listen --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--event"), "should show --event flag");
    assert!(stdout.contains("--app"), "should show --app flag");
    assert!(stdout.contains("--format"), "should show --format flag");
}

#[test]
fn test_events_wait_help() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["events", "wait", "--help"])
        .output()
        .expect("Failed to run cwm events wait --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--event"), "should show --event flag");
    assert!(stdout.contains("--app"), "should show --app flag");
    assert!(stdout.contains("--timeout"), "should show --timeout flag");
}
