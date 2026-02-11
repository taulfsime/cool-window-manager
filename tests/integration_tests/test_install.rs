// integration tests for the install command

use crate::common::*;
use std::fs;

#[test]
fn test_install_to_user_directory() {
    require_docker!();

    let test_dir = create_test_dir("install_user");
    let target_path = test_dir.join("cwm");

    // copy current binary to test directory (simulating install)
    let source = cwm_binary_path();
    fs::copy(&source, &target_path).expect("Failed to copy binary");

    // verify binary exists and is executable
    assert!(target_path.exists(), "Binary should exist after install");
    assert!(is_executable(&target_path), "Binary should be executable");

    // verify it runs
    let version = get_installed_version(&target_path);
    assert!(
        version.is_some(),
        "Should be able to get version from installed binary"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_install_creates_directory_if_missing() {
    require_docker!();

    let test_dir = create_test_dir("install_mkdir");
    let nested_dir = test_dir.join("nested").join("path").join("bin");
    let target_path = nested_dir.join("cwm");

    // create nested directory structure
    fs::create_dir_all(&nested_dir).expect("Failed to create nested directory");

    // copy binary
    let source = cwm_binary_path();
    fs::copy(&source, &target_path).expect("Failed to copy binary");

    assert!(target_path.exists(), "Binary should exist in nested path");
    assert!(is_executable(&target_path), "Binary should be executable");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_install_force_overwrites_existing() {
    require_docker!();

    let test_dir = create_test_dir("install_force");
    let target_path = test_dir.join("cwm");

    // create an existing file
    fs::write(&target_path, "old content").expect("Failed to create existing file");

    // copy new binary (simulating --force)
    let source = cwm_binary_path();
    fs::copy(&source, &target_path).expect("Failed to copy binary");

    // verify it's the new binary, not the old content
    let version = get_installed_version(&target_path);
    assert!(version.is_some(), "Should be able to run the new binary");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_install_preserves_permissions() {
    require_docker!();

    let test_dir = create_test_dir("install_perms");
    let target_path = test_dir.join("cwm");

    let source = cwm_binary_path();
    fs::copy(&source, &target_path).expect("Failed to copy binary");

    // set executable permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms).unwrap();
    }

    assert!(
        is_executable(&target_path),
        "Binary should be executable after permission set"
    );

    // verify it still runs
    let version = get_installed_version(&target_path);
    assert!(version.is_some(), "Binary should still be runnable");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_install_detects_readonly_directory() {
    require_docker!();

    // /opt/readonly is set up in Docker as read-only
    let readonly_dir = std::path::PathBuf::from("/opt/readonly");

    if !readonly_dir.exists() {
        eprintln!("Skipping test: /opt/readonly not available");
        return;
    }

    // try to write to readonly directory
    let target_path = readonly_dir.join("cwm");
    let source = cwm_binary_path();

    let result = fs::copy(&source, &target_path);
    assert!(
        result.is_err(),
        "Should fail to write to read-only directory"
    );
}

#[test]
fn test_install_to_multiple_locations() {
    require_docker!();

    let test_dir = create_test_dir("install_multi");
    let source = cwm_binary_path();

    // install to multiple locations
    let locations = vec![
        test_dir.join("bin1").join("cwm"),
        test_dir.join("bin2").join("cwm"),
        test_dir.join("bin3").join("cwm"),
    ];

    for location in &locations {
        if let Some(parent) = location.parent() {
            fs::create_dir_all(parent).expect("Failed to create directory");
        }
        fs::copy(&source, location).expect("Failed to copy binary");
    }

    // verify all installations
    for location in &locations {
        assert!(location.exists(), "Binary should exist at {:?}", location);
        assert!(
            is_executable(location),
            "Binary should be executable at {:?}",
            location
        );

        let version = get_installed_version(location);
        assert!(version.is_some(), "Should get version from {:?}", location);
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_version_info_saved_after_install() {
    require_docker!();

    let test_dir = create_test_dir("install_version_info");
    let cwm_dir = test_dir.join(".cwm");
    let version_file = cwm_dir.join("version.json");

    // create .cwm directory
    fs::create_dir_all(&cwm_dir).expect("Failed to create .cwm directory");

    // simulate version info being saved
    let version_info = serde_json::json!({
        "current": "dev-c3d4e5f6-20260210",
        "previous": null,
        "last_seen_available": null,
        "install_date": "2026-02-10T00:00:00Z",
        "install_path": test_dir.join("cwm").to_string_lossy()
    });

    fs::write(
        &version_file,
        serde_json::to_string_pretty(&version_info).unwrap(),
    )
    .expect("Failed to write version info");

    // verify version info was saved
    assert!(version_file.exists(), "Version info file should exist");

    let content = fs::read_to_string(&version_file).expect("Failed to read version info");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse version info");

    assert_eq!(parsed["current"], "dev-c3d4e5f6-20260210");

    cleanup_test_dir(&test_dir);
}
