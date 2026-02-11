// integration tests for rollback functionality

use crate::common::*;
use std::fs;

#[test]
fn test_rollback_on_binary_test_failure() {
    require_docker!();

    let test_dir = create_test_dir("rollback_test_fail");
    let binary_path = test_dir.join("cwm");
    let backup_path = test_dir.join("cwm.backup");
    let new_binary_path = test_dir.join("cwm_new");

    // create original working binary
    create_old_binary(&binary_path, "working-version (dev, 2026-01-01)")
        .expect("Failed to create working binary");

    // create backup
    fs::copy(&binary_path, &backup_path).expect("Failed to create backup");

    // create a failing new binary
    create_failing_binary(&new_binary_path).expect("Failed to create failing binary");

    // simulate update attempt
    fs::remove_file(&binary_path).ok();
    fs::rename(&new_binary_path, &binary_path).expect("Failed to install new binary");

    // test the new binary (should fail)
    let test_output = std::process::Command::new(&binary_path)
        .arg("--version")
        .output()
        .expect("Failed to run binary");

    let test_failed = !test_output.status.success();
    assert!(test_failed, "New binary test should fail");

    // rollback: restore from backup
    if test_failed {
        fs::remove_file(&binary_path).ok();
        fs::rename(&backup_path, &binary_path).expect("Failed to restore backup");
    }

    // verify rollback succeeded
    let version = get_installed_version(&binary_path);
    assert!(
        version.is_some(),
        "Should be able to get version after rollback"
    );
    assert!(
        version.unwrap().contains("working-version"),
        "Should have original version after rollback"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_rollback_on_checksum_mismatch() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    // set scenario to return wrong checksum
    set_scenario("checksum_mismatch").expect("Failed to set scenario");

    let test_dir = create_test_dir("rollback_checksum");
    let binary_path = test_dir.join("cwm");

    // create original binary
    create_old_binary(&binary_path, "original-version (dev, 2026-01-01)")
        .expect("Failed to create original binary");

    // fetch releases
    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");
    let releases: Vec<serde_json::Value> = response.json().expect("Failed to parse releases");

    let first_release = &releases[0];
    let assets = first_release["assets"]
        .as_array()
        .expect("Should have assets");
    let tar_asset = assets
        .iter()
        .find(|a| a["name"].as_str().unwrap_or("").ends_with(".tar.gz"))
        .expect("Should have tar.gz asset");

    let download_url = tar_asset["browser_download_url"]
        .as_str()
        .expect("Should have download URL");
    let checksum_url = format!("{}.sha256", download_url);

    // download binary and checksum
    let binary_response = client.get(download_url).send().expect("Failed to download");
    let binary_data = binary_response.bytes().expect("Failed to read data");

    let checksum_response = client
        .get(&checksum_url)
        .send()
        .expect("Failed to download checksum");
    let checksum_content = checksum_response.text().expect("Failed to read checksum");
    let expected_checksum = checksum_content
        .split_whitespace()
        .next()
        .expect("Should have checksum");

    // calculate actual checksum
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let actual_checksum = hex::encode(hasher.finalize());

    // verify checksum mismatch
    assert_ne!(
        actual_checksum, expected_checksum,
        "Checksums should NOT match in this scenario"
    );

    // original binary should still be intact (we didn't proceed with update)
    let version = get_installed_version(&binary_path);
    assert!(version.is_some(), "Original binary should still work");
    assert!(
        version.unwrap().contains("original-version"),
        "Should still have original version"
    );

    // reset scenario
    reset_scenario().expect("Failed to reset scenario");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_rollback_on_corrupt_download() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    // set scenario to return corrupt download
    set_scenario("corrupt_download").expect("Failed to set scenario");

    let test_dir = create_test_dir("rollback_corrupt");

    // fetch releases
    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");
    let releases: Vec<serde_json::Value> = response.json().expect("Failed to parse releases");

    let first_release = &releases[0];
    let assets = first_release["assets"]
        .as_array()
        .expect("Should have assets");
    let tar_asset = assets
        .iter()
        .find(|a| a["name"].as_str().unwrap_or("").ends_with(".tar.gz"))
        .expect("Should have tar.gz asset");

    let download_url = tar_asset["browser_download_url"]
        .as_str()
        .expect("Should have download URL");

    // download corrupt archive
    let archive_path = test_dir.join("cwm.tar.gz");
    let binary_response = client.get(download_url).send().expect("Failed to download");
    let binary_data = binary_response.bytes().expect("Failed to read data");
    fs::write(&archive_path, &binary_data).expect("Failed to write archive");

    // try to extract (should fail)
    let output = std::process::Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap()])
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run tar");

    assert!(
        !output.status.success(),
        "tar extraction should fail on corrupt archive"
    );

    // reset scenario
    reset_scenario().expect("Failed to reset scenario");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_backup_is_removed_on_successful_update() {
    require_docker!();

    let test_dir = create_test_dir("rollback_cleanup");
    let binary_path = test_dir.join("cwm");
    let backup_path = test_dir.join("cwm.backup");
    let new_binary_path = test_dir.join("cwm_new");

    // create original binary
    create_old_binary(&binary_path, "old-version (dev, 2026-01-01)")
        .expect("Failed to create old binary");

    // create backup
    fs::copy(&binary_path, &backup_path).expect("Failed to create backup");

    // create new working binary
    create_old_binary(&new_binary_path, "new-version (dev, 2026-02-10)")
        .expect("Failed to create new binary");

    // simulate update
    fs::remove_file(&binary_path).ok();
    fs::rename(&new_binary_path, &binary_path).expect("Failed to install new binary");

    // test new binary (should succeed)
    let test_output = std::process::Command::new(&binary_path)
        .arg("--version")
        .output()
        .expect("Failed to run binary");

    assert!(
        test_output.status.success(),
        "New binary test should succeed"
    );

    // remove backup after successful update
    if test_output.status.success() {
        fs::remove_file(&backup_path).ok();
    }

    // verify backup is gone
    assert!(
        !backup_path.exists(),
        "Backup should be removed after successful update"
    );

    // verify new binary is installed
    let version = get_installed_version(&binary_path);
    assert!(version.is_some());
    assert!(version.unwrap().contains("new-version"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_rollback_preserves_permissions() {
    require_docker!();

    let test_dir = create_test_dir("rollback_perms");
    let binary_path = test_dir.join("cwm");
    let backup_path = test_dir.join("cwm.backup");

    // create original binary with specific permissions
    create_old_binary(&binary_path, "original (dev, 2026-01-01)").expect("Failed to create binary");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms).unwrap();
    }

    // create backup
    fs::copy(&binary_path, &backup_path).expect("Failed to create backup");

    // simulate failed update and rollback
    fs::remove_file(&binary_path).ok();
    fs::rename(&backup_path, &binary_path).expect("Failed to restore backup");

    // verify permissions are preserved
    assert!(
        is_executable(&binary_path),
        "Restored binary should be executable"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_rollback_handles_missing_backup_gracefully() {
    require_docker!();

    let test_dir = create_test_dir("rollback_no_backup");
    let binary_path = test_dir.join("cwm");
    let backup_path = test_dir.join("cwm.backup");

    // create binary but no backup
    create_old_binary(&binary_path, "current (dev, 2026-01-01)").expect("Failed to create binary");

    // verify backup doesn't exist
    assert!(!backup_path.exists(), "Backup should not exist");

    // attempting to restore from missing backup should fail gracefully
    let restore_result = fs::rename(&backup_path, &binary_path);
    assert!(
        restore_result.is_err(),
        "Restore from missing backup should fail"
    );

    // original binary should still be intact
    assert!(binary_path.exists(), "Original binary should still exist");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_binary_test_fails_scenario() {
    require_docker!();

    if !wait_for_mock_server(10) {
        eprintln!("Skipping test: mock server not available");
        return;
    }

    // set scenario where downloaded binary always fails
    set_scenario("binary_test_fails").expect("Failed to set scenario");

    let test_dir = create_test_dir("rollback_binary_fails");

    // fetch and download a release
    let url = format!("{}/repos/test/repo/releases", mock_server_url());
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send().expect("Failed to fetch releases");
    let releases: Vec<serde_json::Value> = response.json().expect("Failed to parse releases");

    let first_release = &releases[0];
    let assets = first_release["assets"]
        .as_array()
        .expect("Should have assets");
    let tar_asset = assets
        .iter()
        .find(|a| a["name"].as_str().unwrap_or("").ends_with(".tar.gz"))
        .expect("Should have tar.gz asset");

    let download_url = tar_asset["browser_download_url"]
        .as_str()
        .expect("Should have download URL");

    // download and extract
    let archive_path = test_dir.join("cwm.tar.gz");
    let binary_response = client.get(download_url).send().expect("Failed to download");
    let binary_data = binary_response.bytes().expect("Failed to read data");
    fs::write(&archive_path, &binary_data).expect("Failed to write archive");

    let output = std::process::Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap()])
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run tar");

    assert!(output.status.success(), "Extraction should succeed");

    // test the extracted binary (should fail)
    let extracted_binary = test_dir.join("cwm");
    let test_output = std::process::Command::new(&extracted_binary)
        .arg("--version")
        .output()
        .expect("Failed to run binary");

    assert!(
        !test_output.status.success(),
        "Binary test should fail in this scenario"
    );

    // reset scenario
    reset_scenario().expect("Failed to reset scenario");

    cleanup_test_dir(&test_dir);
}
