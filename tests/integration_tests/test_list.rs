// integration tests for the list command

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

/// helper to run cwm list command and get output
/// note: if args contains --json, JSON output is used; otherwise --no-json is added
/// to prevent auto-JSON when stdout is piped
fn run_list(args: &[&str]) -> std::process::Output {
    let test_dir = create_test_dir(&unique_test_name("list"));
    let config_path = create_test_config(&test_dir);

    let binary = cwm_binary_path();
    let has_json_flag = args.iter().any(|a| *a == "--json" || *a == "-j");

    let mut cmd_args = vec!["--config", config_path.to_str().unwrap()];
    // add --no-json for text output tests (stdout is piped, which auto-enables JSON)
    if !has_json_flag {
        cmd_args.push("--no-json");
    }
    cmd_args.push("list");
    cmd_args.extend(args);

    let output = Command::new(&binary)
        .args(&cmd_args)
        // set mock server URL to prevent any network calls
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm list");

    cleanup_test_dir(&test_dir);
    output
}

/// helper to parse JSON output (handles JSON-RPC wrapper)
fn parse_json_output(output: &std::process::Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Failed to parse JSON output: {}\nOutput was: {}", e, stdout));

    // if it's a JSON-RPC response, extract the result
    if json.get("jsonrpc").is_some() && json.get("result").is_some() {
        json["result"].clone()
    } else {
        json
    }
}

// ============================================================================
// list apps tests
// ============================================================================

#[test]
fn test_list_apps_text_output() {
    let output = run_list(&["apps"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "list apps should succeed.\nstdout: {}\nstderr: {}\nexit code: {:?}",
        stdout,
        stderr,
        output.status.code()
    );

    assert!(
        stdout.contains("Running applications:"),
        "should have header"
    );
    assert!(stdout.contains("Total:"), "should have total count");
}

#[test]
fn test_list_apps_json_output() {
    let output = run_list(&["apps", "--json"]);

    assert!(output.status.success(), "list apps --json should succeed");

    let json = parse_json_output(&output);
    assert!(json["items"].is_array(), "should have items array");

    // each item should have name and pid
    if let Some(items) = json["items"].as_array() {
        for item in items {
            assert!(item["name"].is_string(), "item should have name");
            assert!(item["pid"].is_number(), "item should have pid");
            // summary should NOT have bundle_id or titles
            assert!(
                item.get("bundle_id").is_none(),
                "summary should not have bundle_id"
            );
            assert!(
                item.get("titles").is_none(),
                "summary should not have titles"
            );
        }
    }
}

#[test]
fn test_list_apps_json_detailed_output() {
    let output = run_list(&["apps", "--json", "--detailed"]);

    assert!(
        output.status.success(),
        "list apps --json --detailed should succeed"
    );

    let json = parse_json_output(&output);
    assert!(json["items"].is_array(), "should have items array");

    // each item should have all fields
    if let Some(items) = json["items"].as_array() {
        for item in items {
            assert!(item["name"].is_string(), "item should have name");
            assert!(item["pid"].is_number(), "item should have pid");
            // detailed should have bundle_id and titles (even if null/empty)
            assert!(
                item.get("bundle_id").is_some(),
                "detailed should have bundle_id"
            );
            assert!(item.get("titles").is_some(), "detailed should have titles");
        }
    }
}

#[test]
fn test_list_apps_detailed_without_json_is_same_as_text() {
    // --detailed without --json should produce same output as text
    let text_output = run_list(&["apps"]);
    let detailed_output = run_list(&["apps", "--detailed"]);

    assert!(text_output.status.success());
    assert!(detailed_output.status.success());

    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    let detailed_stdout = String::from_utf8_lossy(&detailed_output.stdout);

    // both should be text format (not JSON)
    assert!(
        text_stdout.contains("Running applications:"),
        "text should have header"
    );
    assert!(
        detailed_stdout.contains("Running applications:"),
        "detailed text should have header"
    );
}

// ============================================================================
// list displays tests
// ============================================================================

#[test]
fn test_list_displays_text_output() {
    let output = run_list(&["displays"]);

    assert!(output.status.success(), "list displays should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // should have either "Available displays:" or "No displays found"
    assert!(
        stdout.contains("Available displays:") || stdout.contains("No displays found"),
        "should have appropriate header"
    );
}

#[test]
fn test_list_displays_json_output() {
    let output = run_list(&["displays", "--json"]);

    assert!(
        output.status.success(),
        "list displays --json should succeed"
    );

    let json = parse_json_output(&output);
    assert!(json["items"].is_array(), "should have items array");

    // each item should have summary fields
    if let Some(items) = json["items"].as_array() {
        for item in items {
            assert!(item["index"].is_number(), "item should have index");
            assert!(item["name"].is_string(), "item should have name");
            assert!(item["width"].is_number(), "item should have width");
            assert!(item["height"].is_number(), "item should have height");
            assert!(item["is_main"].is_boolean(), "item should have is_main");
            // summary should NOT have detailed fields
            assert!(
                item.get("vendor_id").is_none(),
                "summary should not have vendor_id"
            );
            assert!(
                item.get("unique_id").is_none(),
                "summary should not have unique_id"
            );
        }
    }
}

#[test]
fn test_list_displays_json_detailed_output() {
    let output = run_list(&["displays", "--json", "--detailed"]);

    assert!(
        output.status.success(),
        "list displays --json --detailed should succeed"
    );

    let json = parse_json_output(&output);
    assert!(json["items"].is_array(), "should have items array");

    // each item should have all fields
    if let Some(items) = json["items"].as_array() {
        for item in items {
            // basic fields
            assert!(item["index"].is_number(), "item should have index");
            assert!(item["name"].is_string(), "item should have name");
            assert!(item["width"].is_number(), "item should have width");
            assert!(item["height"].is_number(), "item should have height");
            assert!(item["is_main"].is_boolean(), "item should have is_main");
            // detailed fields
            assert!(item.get("x").is_some(), "detailed should have x");
            assert!(item.get("y").is_some(), "detailed should have y");
            assert!(
                item.get("is_builtin").is_some(),
                "detailed should have is_builtin"
            );
            assert!(
                item.get("display_id").is_some(),
                "detailed should have display_id"
            );
            assert!(
                item.get("unique_id").is_some(),
                "detailed should have unique_id"
            );
        }
    }
}

// ============================================================================
// list aliases tests
// ============================================================================

#[test]
fn test_list_aliases_text_output() {
    let output = run_list(&["aliases"]);

    assert!(output.status.success(), "list aliases should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("System Aliases:"),
        "should have system aliases header"
    );
    // should list system aliases
    assert!(
        stdout.contains("builtin") || stdout.contains("external"),
        "should list system aliases"
    );
}

#[test]
fn test_list_aliases_json_output() {
    let output = run_list(&["aliases", "--json"]);

    assert!(
        output.status.success(),
        "list aliases --json should succeed"
    );

    let json = parse_json_output(&output);
    assert!(json["items"].is_array(), "should have items array");

    // should have at least the 4 system aliases
    let items = json["items"].as_array().unwrap();
    assert!(items.len() >= 4, "should have at least 4 system aliases");

    // check structure of items
    for item in items {
        assert!(item["name"].is_string(), "item should have name");
        assert!(item["type"].is_string(), "item should have type");
        assert!(item["resolved"].is_boolean(), "item should have resolved");
        // display_index can be null or number
        assert!(
            item.get("display_index").is_some(),
            "item should have display_index"
        );
        // summary should NOT have detailed fields
        assert!(
            item.get("display_name").is_none(),
            "summary should not have display_name"
        );
    }

    // verify system aliases are present
    let names: Vec<&str> = items.iter().filter_map(|i| i["name"].as_str()).collect();
    assert!(names.contains(&"builtin"), "should have builtin alias");
    assert!(names.contains(&"external"), "should have external alias");
    assert!(names.contains(&"main"), "should have main alias");
    assert!(names.contains(&"secondary"), "should have secondary alias");
}

#[test]
fn test_list_aliases_json_detailed_output() {
    let output = run_list(&["aliases", "--json", "--detailed"]);

    assert!(
        output.status.success(),
        "list aliases --json --detailed should succeed"
    );

    let json = parse_json_output(&output);
    assert!(json["items"].is_array(), "should have items array");

    let items = json["items"].as_array().unwrap();

    // check structure of detailed items
    for item in items {
        assert!(item["name"].is_string(), "item should have name");
        assert!(item["type"].is_string(), "item should have type");
        assert!(item["resolved"].is_boolean(), "item should have resolved");
        // detailed fields (may be null if not resolved)
        assert!(
            item.get("display_index").is_some(),
            "detailed should have display_index"
        );
        assert!(
            item.get("display_name").is_some(),
            "detailed should have display_name"
        );
        assert!(
            item.get("display_unique_id").is_some(),
            "detailed should have display_unique_id"
        );

        // system aliases should have description
        if item["type"].as_str() == Some("system") {
            assert!(
                item.get("description").is_some(),
                "system alias should have description"
            );
        }
    }
}

#[test]
fn test_list_aliases_system_types() {
    let output = run_list(&["aliases", "--json"]);
    let json = parse_json_output(&output);
    let items = json["items"].as_array().unwrap();

    // all system aliases should have type "system"
    let system_aliases = ["builtin", "external", "main", "secondary"];
    for alias_name in &system_aliases {
        let alias = items
            .iter()
            .find(|i| i["name"].as_str() == Some(alias_name));
        assert!(alias.is_some(), "should have {} alias", alias_name);
        assert_eq!(
            alias.unwrap()["type"].as_str(),
            Some("system"),
            "{} should be system type",
            alias_name
        );
    }
}

// ============================================================================
// error handling tests
// ============================================================================

#[test]
fn test_list_invalid_resource() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["list", "invalid"])
        .output()
        .expect("Failed to run cwm");

    assert!(!output.status.success(), "list invalid should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("Invalid"),
        "should mention invalid value"
    );
}

#[test]
fn test_list_missing_resource_shows_help() {
    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args(["list"])
        .output()
        .expect("Failed to run cwm");

    // list without resource now shows help and succeeds
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Available resources") || stdout.contains("Usage"),
        "list without resource should show help: {}",
        stdout
    );
}

// ============================================================================
// JSON structure tests
// ============================================================================

#[test]
fn test_list_json_is_valid_json() {
    // test all three resources produce valid JSON
    for resource in &["apps", "displays", "aliases"] {
        let output = run_list(&[resource, "--json"]);
        assert!(
            output.status.success(),
            "list {} --json should succeed",
            resource
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(
            result.is_ok(),
            "list {} --json should produce valid JSON: {}",
            resource,
            stdout
        );
    }
}

#[test]
fn test_list_json_detailed_is_valid_json() {
    // test all three resources produce valid JSON with --detailed
    for resource in &["apps", "displays", "aliases"] {
        let output = run_list(&[resource, "--json", "--detailed"]);
        assert!(
            output.status.success(),
            "list {} --json --detailed should succeed",
            resource
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(
            result.is_ok(),
            "list {} --json --detailed should produce valid JSON: {}",
            resource,
            stdout
        );
    }
}

#[test]
fn test_list_json_has_items_array() {
    // all JSON outputs should have an "items" array at the root
    for resource in &["apps", "displays", "aliases"] {
        let output = run_list(&[resource, "--json"]);
        let json = parse_json_output(&output);

        assert!(
            json.is_object(),
            "list {} --json should return an object",
            resource
        );
        assert!(
            json.get("items").is_some(),
            "list {} --json should have 'items' field",
            resource
        );
        assert!(
            json["items"].is_array(),
            "list {} --json 'items' should be an array",
            resource
        );
    }
}

// ============================================================================
// flag combination tests
// ============================================================================

#[test]
fn test_list_flags_order_does_not_matter() {
    // --json --detailed should be same as --detailed --json
    let output1 = run_list(&["apps", "--json", "--detailed"]);
    let output2 = run_list(&["apps", "--detailed", "--json"]);

    assert!(output1.status.success());
    assert!(output2.status.success());

    let json1 = parse_json_output(&output1);
    let json2 = parse_json_output(&output2);

    // both should have the same structure
    assert!(json1["items"].is_array());
    assert!(json2["items"].is_array());

    // both should have detailed fields
    if let (Some(items1), Some(items2)) = (json1["items"].as_array(), json2["items"].as_array()) {
        if !items1.is_empty() && !items2.is_empty() {
            assert!(items1[0].get("bundle_id").is_some());
            assert!(items2[0].get("bundle_id").is_some());
        }
    }
}

#[test]
fn test_list_short_detailed_flag() {
    // -d should work same as --detailed
    let output_long = run_list(&["displays", "--json", "--detailed"]);
    let output_short = run_list(&["displays", "--json", "-d"]);

    assert!(output_long.status.success());
    assert!(output_short.status.success());

    let json_long = parse_json_output(&output_long);
    let json_short = parse_json_output(&output_short);

    // both should have detailed fields
    if let (Some(items_long), Some(items_short)) = (
        json_long["items"].as_array(),
        json_short["items"].as_array(),
    ) {
        if !items_long.is_empty() && !items_short.is_empty() {
            assert!(items_long[0].get("unique_id").is_some());
            assert!(items_short[0].get("unique_id").is_some());
        }
    }
}
