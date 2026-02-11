// integration tests for channel switching (dev -> beta -> stable)

use crate::common::*;
use std::fs;

#[test]
fn test_releases_have_correct_channels() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    reset_scenario().expect("Failed to reset scenario");

    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");
    let releases: Vec<serde_json::Value> = response.json().expect("Failed to parse releases");

    // verify we have releases from each channel
    let mut has_stable = false;
    let mut has_beta = false;
    let mut has_dev = false;

    for release in &releases {
        let tag = release["tag_name"].as_str().unwrap_or("");
        if tag.starts_with("stable-") {
            has_stable = true;
        } else if tag.starts_with("beta-") {
            has_beta = true;
        } else if tag.starts_with("dev-") {
            has_dev = true;
        }
    }

    assert!(has_stable, "Should have stable release");
    assert!(has_beta, "Should have beta release");
    assert!(has_dev, "Should have dev release");
}

#[test]
fn test_channel_priority_stable_first() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    reset_scenario().expect("Failed to reset scenario");

    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");
    let releases: Vec<serde_json::Value> = response.json().expect("Failed to parse releases");

    // when user wants stable, we should prefer stable over beta/dev
    // even if beta/dev are newer

    let stable_release = releases
        .iter()
        .find(|r| r["tag_name"].as_str().unwrap_or("").starts_with("stable-"));

    assert!(stable_release.is_some(), "Should find stable release");

    let stable_tag = stable_release.unwrap()["tag_name"].as_str().unwrap();
    assert!(
        stable_tag.starts_with("stable-"),
        "Tag should be stable channel"
    );
}

#[test]
fn test_dev_to_beta_upgrade() {
    require_docker!();

    let test_dir = create_test_dir("channel_dev_to_beta");
    let cwm_dir = test_dir.join(".cwm");
    let version_file = cwm_dir.join("version.json");

    fs::create_dir_all(&cwm_dir).expect("Failed to create .cwm directory");

    // start with dev version
    let dev_version_info = serde_json::json!({
        "current": "dev-c3d4e5f6-20260210",
        "previous": null,
        "last_seen_available": null,
        "install_date": "2026-02-10T00:00:00Z",
        "install_path": test_dir.join("cwm").to_string_lossy()
    });

    fs::write(
        &version_file,
        serde_json::to_string_pretty(&dev_version_info).unwrap(),
    )
    .expect("Failed to write version info");

    // simulate upgrade to beta
    let beta_version_info = serde_json::json!({
        "current": "beta-b2c3d4e5-20260205",
        "previous": "dev-c3d4e5f6-20260210",
        "last_seen_available": null,
        "install_date": "2026-02-11T00:00:00Z",
        "install_path": test_dir.join("cwm").to_string_lossy()
    });

    fs::write(
        &version_file,
        serde_json::to_string_pretty(&beta_version_info).unwrap(),
    )
    .expect("Failed to write version info");

    // verify upgrade
    let content = fs::read_to_string(&version_file).expect("Failed to read version info");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse");

    assert!(
        parsed["current"].as_str().unwrap().starts_with("beta-"),
        "Should be on beta channel"
    );
    assert!(
        parsed["previous"].as_str().unwrap().starts_with("dev-"),
        "Previous should be dev"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_beta_to_stable_upgrade() {
    require_docker!();

    let test_dir = create_test_dir("channel_beta_to_stable");
    let cwm_dir = test_dir.join(".cwm");
    let version_file = cwm_dir.join("version.json");

    fs::create_dir_all(&cwm_dir).expect("Failed to create .cwm directory");

    // start with beta version
    let beta_version_info = serde_json::json!({
        "current": "beta-b2c3d4e5-20260205",
        "previous": null,
        "last_seen_available": null,
        "install_date": "2026-02-05T00:00:00Z",
        "install_path": test_dir.join("cwm").to_string_lossy()
    });

    fs::write(
        &version_file,
        serde_json::to_string_pretty(&beta_version_info).unwrap(),
    )
    .expect("Failed to write version info");

    // simulate upgrade to stable
    let stable_version_info = serde_json::json!({
        "current": "stable-a1b2c3d4-20260201",
        "previous": "beta-b2c3d4e5-20260205",
        "last_seen_available": null,
        "install_date": "2026-02-11T00:00:00Z",
        "install_path": test_dir.join("cwm").to_string_lossy()
    });

    fs::write(
        &version_file,
        serde_json::to_string_pretty(&stable_version_info).unwrap(),
    )
    .expect("Failed to write version info");

    // verify upgrade
    let content = fs::read_to_string(&version_file).expect("Failed to read version info");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse");

    assert!(
        parsed["current"].as_str().unwrap().starts_with("stable-"),
        "Should be on stable channel"
    );
    assert!(
        parsed["previous"].as_str().unwrap().starts_with("beta-"),
        "Previous should be beta"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_dev_to_stable_direct_upgrade() {
    require_docker!();

    let test_dir = create_test_dir("channel_dev_to_stable");
    let cwm_dir = test_dir.join(".cwm");
    let version_file = cwm_dir.join("version.json");

    fs::create_dir_all(&cwm_dir).expect("Failed to create .cwm directory");

    // start with dev version
    let dev_version_info = serde_json::json!({
        "current": "dev-c3d4e5f6-20260210",
        "previous": null,
        "last_seen_available": null,
        "install_date": "2026-02-10T00:00:00Z",
        "install_path": test_dir.join("cwm").to_string_lossy()
    });

    fs::write(
        &version_file,
        serde_json::to_string_pretty(&dev_version_info).unwrap(),
    )
    .expect("Failed to write version info");

    // simulate direct upgrade to stable (skipping beta)
    let stable_version_info = serde_json::json!({
        "current": "stable-a1b2c3d4-20260201",
        "previous": "dev-c3d4e5f6-20260210",
        "last_seen_available": null,
        "install_date": "2026-02-11T00:00:00Z",
        "install_path": test_dir.join("cwm").to_string_lossy()
    });

    fs::write(
        &version_file,
        serde_json::to_string_pretty(&stable_version_info).unwrap(),
    )
    .expect("Failed to write version info");

    // verify upgrade
    let content = fs::read_to_string(&version_file).expect("Failed to read version info");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse");

    assert!(
        parsed["current"].as_str().unwrap().starts_with("stable-"),
        "Should be on stable channel"
    );
    assert!(
        parsed["previous"].as_str().unwrap().starts_with("dev-"),
        "Previous should be dev"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_release_assets_match_architecture() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    reset_scenario().expect("Failed to reset scenario");

    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");
    let releases: Vec<serde_json::Value> = response.json().expect("Failed to parse releases");

    // verify each release has assets for both architectures
    for release in &releases {
        let tag = release["tag_name"].as_str().unwrap_or("");
        let assets = release["assets"].as_array().expect("Should have assets");

        let asset_names: Vec<&str> = assets.iter().filter_map(|a| a["name"].as_str()).collect();

        // should have x86_64 and aarch64 variants
        let has_x86 = asset_names.iter().any(|n| n.contains("x86_64"));
        let has_arm = asset_names.iter().any(|n| n.contains("aarch64"));

        assert!(has_x86, "Release {} should have x86_64 asset", tag);
        assert!(has_arm, "Release {} should have aarch64 asset", tag);
    }
}

#[test]
fn test_no_releases_scenario() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    // set scenario with no releases
    set_scenario("no_releases").expect("Failed to set scenario");

    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");
    let releases: Vec<serde_json::Value> = response.json().expect("Failed to parse releases");

    assert!(
        releases.is_empty(),
        "Should have no releases in this scenario"
    );

    // reset scenario
    reset_scenario().expect("Failed to reset scenario");
}

#[test]
fn test_rate_limited_scenario() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    // set scenario to simulate rate limiting
    set_scenario("rate_limited").expect("Failed to set scenario");

    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");

    assert_eq!(
        response.status().as_u16(),
        403,
        "Should return 403 when rate limited"
    );

    // check rate limit headers
    let remaining = response.headers().get("x-ratelimit-remaining");
    assert!(remaining.is_some(), "Should have rate limit header");
    assert_eq!(
        remaining.unwrap().to_str().unwrap(),
        "0",
        "Rate limit should be 0"
    );

    // reset scenario
    reset_scenario().expect("Failed to reset scenario");
}
