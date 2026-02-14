// integration tests for move command

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

// display target tests (replacing move-display functionality)

#[test]
fn test_move_display_next() {
    let test_dir = create_test_dir(&unique_test_name("move_next"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--display",
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
fn test_move_display_prev() {
    let test_dir = create_test_dir(&unique_test_name("move_prev"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--display",
            "prev",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_display_numeric() {
    let test_dir = create_test_dir(&unique_test_name("move_numeric"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--display",
            "1",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    // numeric target should parse correctly
    // will fail because app not found, but target parsing should work
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

// anchor position tests

#[test]
fn test_move_anchor_top_left() {
    let test_dir = create_test_dir(&unique_test_name("move_top_left"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "top-left",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_anchor_top_right() {
    let test_dir = create_test_dir(&unique_test_name("move_top_right"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "top-right",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_anchor_bottom_left() {
    let test_dir = create_test_dir(&unique_test_name("move_bottom_left"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "bottom-left",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_anchor_bottom_right() {
    let test_dir = create_test_dir(&unique_test_name("move_bottom_right"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "bottom-right",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_anchor_left() {
    let test_dir = create_test_dir(&unique_test_name("move_left"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "left",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_anchor_right() {
    let test_dir = create_test_dir(&unique_test_name("move_right"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "right",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

// percentage position tests

#[test]
fn test_move_percentage_center() {
    let test_dir = create_test_dir(&unique_test_name("move_center"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "50%,50%",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_percentage_corner() {
    let test_dir = create_test_dir(&unique_test_name("move_pct_corner"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "0%,0%",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

// absolute position tests

#[test]
fn test_move_absolute_pixels() {
    let test_dir = create_test_dir(&unique_test_name("move_pixels"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "100,200px",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_absolute_points() {
    let test_dir = create_test_dir(&unique_test_name("move_points"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "100,200pt",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

// relative position tests

#[test]
fn test_move_relative_positive() {
    let test_dir = create_test_dir(&unique_test_name("move_rel_pos"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "+100,+50",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_relative_negative() {
    let test_dir = create_test_dir(&unique_test_name("move_rel_neg"));
    let config_path = create_test_config(&test_dir);

    // use --to=value format to avoid shell interpreting - as flag
    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to=-50,-100",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_relative_mixed() {
    let test_dir = create_test_dir(&unique_test_name("move_rel_mixed"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "+100,-50",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

// combined display + position tests

#[test]
fn test_move_with_display_and_position() {
    let test_dir = create_test_dir(&unique_test_name("move_display_pos"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "top-left",
            "--display",
            "1",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_display_only_keeps_position() {
    let test_dir = create_test_dir(&unique_test_name("move_display_only"));
    let config_path = create_test_config(&test_dir);

    // when only --display is specified (no --to), should keep relative position
    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--display",
            "next",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("NonExistentApp12345"));

    cleanup_test_dir(&test_dir);
}

// flag tests

#[test]
fn test_move_app_not_found_json() {
    let test_dir = create_test_dir(&unique_test_name("move_notfound_json"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--json",
            "move",
            "--display",
            "next",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON error response
    assert!(stdout.contains("error") || stdout.contains("not found"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_with_no_launch_flag() {
    let test_dir = create_test_dir(&unique_test_name("move_nolaunch"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--display",
            "next",
            "--app",
            "NonExistentApp12345",
            "--no-launch",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_launch_and_no_launch_conflict() {
    let test_dir = create_test_dir(&unique_test_name("move_conflict"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &["move", "--display", "next", "--launch", "--no-launch"],
        &config_path,
    );
    // should fail - conflicting flags
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with") || stderr.contains("conflict"));

    cleanup_test_dir(&test_dir);
}

// error cases

#[test]
fn test_move_requires_to_or_display() {
    let test_dir = create_test_dir(&unique_test_name("move_requires_target"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(&["move"], &config_path);
    // should fail without --to or --display
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required") || stderr.contains("--to") || stderr.contains("--display"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_invalid_target() {
    let test_dir = create_test_dir(&unique_test_name("move_invalid"));
    let config_path = create_test_config(&test_dir);

    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "invalid_target",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    cleanup_test_dir(&test_dir);
}

// tests for invalid --to values (display targets should use --display flag)

#[test]
fn test_move_to_next_is_invalid() {
    let test_dir = create_test_dir(&unique_test_name("move_to_next"));
    let config_path = create_test_config(&test_dir);

    // "next" is a display target, not a position - should fail
    // users should use --display next instead
    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "next",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // should fail due to invalid position
    assert!(stderr.contains("invalid") || stderr.contains("position"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_move_to_prev_is_invalid() {
    let test_dir = create_test_dir(&unique_test_name("move_to_prev"));
    let config_path = create_test_config(&test_dir);

    // "prev" is a display target, not a position - should fail
    // users should use --display prev instead
    let output = run_cwm_with_config(
        &[
            "--no-json",
            "move",
            "--to",
            "prev",
            "--app",
            "NonExistentApp12345",
        ],
        &config_path,
    );
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // should fail due to invalid position
    assert!(stderr.contains("invalid") || stderr.contains("position"));

    cleanup_test_dir(&test_dir);
}
