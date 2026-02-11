// shared utilities for integration tests

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// get the mock server URL from environment
pub fn mock_server_url() -> String {
    env::var("CWM_GITHUB_API_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// check if we're running in Docker (mock server available)
pub fn is_docker_environment() -> bool {
    env::var("CWM_GITHUB_API_URL").is_ok()
}

/// skip test if not in Docker environment
#[macro_export]
macro_rules! require_docker {
    () => {
        if !crate::common::is_docker_environment() {
            eprintln!("Skipping test: not in Docker environment");
            return;
        }
    };
}

/// create a temporary directory for test installations
pub fn create_test_dir(name: &str) -> PathBuf {
    let base = env::temp_dir().join("cwm_integration_tests");
    let dir = base.join(name);

    // clean up if exists
    if dir.exists() {
        fs::remove_dir_all(&dir).ok();
    }

    fs::create_dir_all(&dir).expect("Failed to create test directory");
    dir
}

/// clean up a test directory
pub fn cleanup_test_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).ok();
    }
}

/// set mock server scenario via HTTP
pub fn set_scenario(mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/control/set-scenario", mock_server_url());
    let client = reqwest::blocking::Client::new();

    let response = client
        .post(&url)
        .json(&serde_json::json!({ "mode": mode }))
        .send()?;

    if !response.status().is_success() {
        return Err(format!("Failed to set scenario: {}", response.status()).into());
    }

    Ok(())
}

/// reset mock server to default scenario
pub fn reset_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/control/reset", mock_server_url());
    let client = reqwest::blocking::Client::new();

    let response = client.post(&url).send()?;

    if !response.status().is_success() {
        return Err(format!("Failed to reset scenario: {}", response.status()).into());
    }

    Ok(())
}

/// get path to the built cwm binary
pub fn cwm_binary_path() -> PathBuf {
    // in Docker, the binary is built at /app/target/release/cwm
    let docker_path = PathBuf::from("/app/target/release/cwm");
    if docker_path.exists() {
        return docker_path;
    }

    // locally, check target/release or target/debug
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    let release_path = manifest_dir.join("target/release/cwm");
    if release_path.exists() {
        return release_path;
    }

    let debug_path = manifest_dir.join("target/debug/cwm");
    if debug_path.exists() {
        return debug_path;
    }

    panic!("Could not find cwm binary");
}

/// run cwm command and capture output
#[allow(dead_code)]
pub fn run_cwm(args: &[&str]) -> std::process::Output {
    let binary = cwm_binary_path();

    Command::new(&binary)
        .args(args)
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm")
}

/// run cwm command with custom environment
#[allow(dead_code)]
pub fn run_cwm_with_env(args: &[&str], env_vars: &[(&str, &str)]) -> std::process::Output {
    let binary = cwm_binary_path();

    let mut cmd = Command::new(&binary);
    cmd.args(args);
    cmd.env("CWM_GITHUB_API_URL", mock_server_url());

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    cmd.output().expect("Failed to run cwm")
}

/// check if a file is executable
#[cfg(unix)]
pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = fs::metadata(path) {
        let mode = metadata.permissions().mode();
        return mode & 0o111 != 0;
    }
    false
}

#[cfg(not(unix))]
pub fn is_executable(path: &Path) -> bool {
    path.exists()
}

/// read version from installed binary
pub fn get_installed_version(binary_path: &Path) -> Option<String> {
    let output = Command::new(binary_path).arg("--version").output().ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// create a fake "old" cwm binary for update testing
pub fn create_old_binary(path: &Path, version: &str) -> std::io::Result<()> {
    let script = format!(
        r#"#!/bin/sh
case "$1" in
    --version|version)
        echo "cwm {}"
        exit 0
        ;;
    *)
        echo "cwm old test binary"
        exit 0
        ;;
esac
"#,
        version
    );

    fs::write(path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// create a binary that fails when run (for rollback testing)
pub fn create_failing_binary(path: &Path) -> std::io::Result<()> {
    let script = r#"#!/bin/sh
exit 1
"#;

    fs::write(path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// wait for mock server to be ready
pub fn wait_for_mock_server(timeout_secs: u64) -> bool {
    let url = format!("{}/health", mock_server_url());
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if let Ok(response) = client.get(&url).send() {
            if response.status().is_success() {
                return true;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    false
}
