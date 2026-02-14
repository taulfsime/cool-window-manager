// integration tests for the resize command

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

/// helper to run cwm resize command (text output)
fn run_resize(args: &[&str], config_path: &std::path::Path) -> std::process::Output {
    let binary = cwm_binary_path();
    // use --no-json to get text output (stdout is piped in tests, which auto-enables JSON)
    let mut cmd_args = vec![
        "--config",
        config_path.to_str().unwrap(),
        "--no-json",
        "resize",
    ];
    cmd_args.extend(args);

    Command::new(&binary)
        .args(&cmd_args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm resize")
}

// ============================================================================
// resize command argument parsing tests
// ============================================================================

#[test]
fn test_resize_requires_to_argument() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["resize"])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    assert!(!output.status.success(), "resize without --to should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--to") || stderr.contains("required"),
        "should mention --to is required"
    );
}

#[test]
fn test_resize_percent_format() {
    let test_dir = create_test_dir(&unique_test_name("resize_percent"));
    let config_path = create_test_config(&test_dir);

    // test various percent formats - these should parse correctly
    // (may fail due to no window, but parsing should succeed)
    for size in &["80", "80%", "50", "100"] {
        let output = run_resize(&["--to", size], &config_path);

        let stderr = String::from_utf8_lossy(&output.stderr);
        // should not fail due to parsing error
        assert!(
            !stderr.contains("Invalid size") && !stderr.contains("invalid"),
            "size '{}' should be valid: {}",
            size,
            stderr
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_decimal_format() {
    let test_dir = create_test_dir(&unique_test_name("resize_decimal"));
    let config_path = create_test_config(&test_dir);

    // test decimal formats (0.0-1.0)
    for size in &["0.8", "0.5", "1.0", "0.25"] {
        let output = run_resize(&["--to", size], &config_path);

        let stderr = String::from_utf8_lossy(&output.stderr);
        // should not fail due to parsing error
        assert!(
            !stderr.contains("Invalid") && !stderr.contains("invalid"),
            "decimal size '{}' should be valid: {}",
            size,
            stderr
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_full_keyword() {
    let test_dir = create_test_dir(&unique_test_name("resize_full"));
    let config_path = create_test_config(&test_dir);

    let output = run_resize(&["--to", "full"], &config_path);

    let stderr = String::from_utf8_lossy(&output.stderr);
    // should not fail due to parsing error
    assert!(
        !stderr.contains("Invalid size") && !stderr.contains("invalid"),
        "'full' should be valid: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_pixels_format() {
    let test_dir = create_test_dir(&unique_test_name("resize_pixels"));
    let config_path = create_test_config(&test_dir);

    // test pixel formats
    for size in &["1920px", "1920x1080px", "800px"] {
        let output = run_resize(&["--to", size], &config_path);

        let stderr = String::from_utf8_lossy(&output.stderr);
        // should not fail due to parsing error
        assert!(
            !stderr.contains("Invalid") && !stderr.contains("invalid"),
            "pixel size '{}' should be valid: {}",
            size,
            stderr
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_points_format() {
    let test_dir = create_test_dir(&unique_test_name("resize_points"));
    let config_path = create_test_config(&test_dir);

    // test point formats
    for size in &["800pt", "800x600pt", "1200pt"] {
        let output = run_resize(&["--to", size], &config_path);

        let stderr = String::from_utf8_lossy(&output.stderr);
        // should not fail due to parsing error
        assert!(
            !stderr.contains("Invalid") && !stderr.contains("invalid"),
            "point size '{}' should be valid: {}",
            size,
            stderr
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_invalid_percent_over_100() {
    let test_dir = create_test_dir(&unique_test_name("resize_invalid_over"));
    let config_path = create_test_config(&test_dir);

    let output = run_resize(&["--to", "150"], &config_path);

    assert!(
        !output.status.success(),
        "resize with percent > 100 should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("100") || stderr.contains("invalid") || stderr.contains("Invalid"),
        "should mention invalid percentage: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_invalid_percent_zero() {
    let test_dir = create_test_dir(&unique_test_name("resize_invalid_zero"));
    let config_path = create_test_config(&test_dir);

    let output = run_resize(&["--to", "0"], &config_path);

    assert!(
        !output.status.success(),
        "resize with percent 0 should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("0") || stderr.contains("invalid") || stderr.contains("Invalid"),
        "should mention invalid percentage: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_invalid_decimal_over_1() {
    let test_dir = create_test_dir(&unique_test_name("resize_invalid_decimal"));
    let config_path = create_test_config(&test_dir);

    let output = run_resize(&["--to", "1.5"], &config_path);

    assert!(
        !output.status.success(),
        "resize with decimal > 1.0 should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("1.0") || stderr.contains("invalid") || stderr.contains("Invalid"),
        "should mention invalid decimal: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_invalid_format() {
    let test_dir = create_test_dir(&unique_test_name("resize_invalid_format"));
    let config_path = create_test_config(&test_dir);

    for invalid in &["abc", "80em", "100vw", "large", "-50"] {
        let output = run_resize(&["--to", invalid], &config_path);

        assert!(
            !output.status.success(),
            "resize with invalid format '{}' should fail",
            invalid
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_invalid_dimensions() {
    let test_dir = create_test_dir(&unique_test_name("resize_invalid_dims"));
    let config_path = create_test_config(&test_dir);

    // invalid dimension formats
    for invalid in &["1920x1080x720px", "1920xpx", "x1080px", "0x0px"] {
        let output = run_resize(&["--to", invalid], &config_path);

        assert!(
            !output.status.success(),
            "resize with invalid dimensions '{}' should fail",
            invalid
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// resize with app argument
// ============================================================================

#[test]
fn test_resize_with_app_argument() {
    let test_dir = create_test_dir(&unique_test_name("resize_with_app"));
    let config_path = create_test_config(&test_dir);

    let output = run_resize(
        &["--to", "80", "--app", "NonExistentApp12345"],
        &config_path,
    );

    // should fail because app doesn't exist
    assert!(
        !output.status.success(),
        "resize with non-existent app should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("not running"),
        "should mention app not found: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_resize_with_finder() {
    let test_dir = create_test_dir(&unique_test_name("resize_finder"));
    let config_path = create_test_config(&test_dir);

    let output = run_resize(&["--to", "80", "--app", "Finder"], &config_path);

    // may succeed or fail depending on permissions
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        // should fail with reasonable error
        assert!(
            stderr.contains("permission")
                || stderr.contains("accessibility")
                || stderr.contains("window"),
            "should fail with reasonable error: {}",
            stderr
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// resize with overflow flag
// ============================================================================

#[test]
fn test_resize_overflow_flag_accepted() {
    let test_dir = create_test_dir(&unique_test_name("resize_overflow"));
    let config_path = create_test_config(&test_dir);

    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "resize",
            "--to",
            "80",
            "--overflow",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    // --overflow flag should be accepted
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "--overflow flag should be accepted"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// resize JSON output
// ============================================================================

#[test]
fn test_resize_json_output() {
    let test_dir = create_test_dir(&unique_test_name("resize_json"));
    let config_path = create_test_config(&test_dir);

    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--json",
            "resize",
            "--to",
            "80",
            "--app",
            "Finder",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // if there's output, it should be valid JSON
    if !stdout.trim().is_empty() {
        let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(json.is_ok(), "output should be valid JSON: {}", stdout);
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// resize case insensitivity
// ============================================================================

#[test]
fn test_resize_case_insensitive() {
    let test_dir = create_test_dir(&unique_test_name("resize_case"));
    let config_path = create_test_config(&test_dir);

    // test that size parsing is case insensitive
    for size in &["FULL", "Full", "80PX", "800PT"] {
        let output = run_resize(&["--to", size], &config_path);

        let stderr = String::from_utf8_lossy(&output.stderr);
        // should not fail due to case
        assert!(
            !stderr.contains("Invalid size"),
            "size '{}' should be valid (case insensitive): {}",
            size,
            stderr
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// resize with whitespace
// ============================================================================

#[test]
fn test_resize_with_whitespace() {
    let test_dir = create_test_dir(&unique_test_name("resize_whitespace"));
    let config_path = create_test_config(&test_dir);

    // test that whitespace is trimmed
    let output = run_resize(&["--to", " 80 "], &config_path);

    let stderr = String::from_utf8_lossy(&output.stderr);
    // should not fail due to whitespace
    assert!(
        !stderr.contains("Invalid size"),
        "size with whitespace should be valid: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// resize verbose mode
// ============================================================================

#[test]
fn test_resize_verbose_flag() {
    let test_dir = create_test_dir(&unique_test_name("resize_verbose"));
    let config_path = create_test_config(&test_dir);

    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "resize",
            "--to",
            "80",
            "-v",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    // -v flag should be accepted
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "-v flag should be accepted"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// resize launch flags
// ============================================================================

#[test]
fn test_resize_launch_flags() {
    let test_dir = create_test_dir(&unique_test_name("resize_launch"));
    let config_path = create_test_config(&test_dir);

    // test --launch flag
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "resize",
            "--to",
            "80",
            "--app",
            "NonExistent",
            "--launch",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "--launch flag should be accepted"
    );

    // test --no-launch flag
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "resize",
            "--to",
            "80",
            "--app",
            "NonExistent",
            "--no-launch",
        ])
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "--no-launch flag should be accepted"
    );

    cleanup_test_dir(&test_dir);
}
