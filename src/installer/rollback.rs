//! rollback support for safe updates with automatic recovery
//!
//! this module provides functions for performing updates with automatic rollback
//! on failure, and browser-based error reporting for failed updates

#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::TelemetrySettings;

pub struct UpdateAttempt {
    pub new_version: String,
    pub previous_version: String,
}

/// perform update with automatic rollback on failure
pub fn safe_update_with_rollback<F>(update_fn: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension("backup");

    // create backup
    fs::copy(&current_exe, &backup_path).context("failed to create backup")?;

    // attempt update
    match update_fn() {
        Ok(_) => {
            // test new binary
            match test_updated_binary(&current_exe) {
                Ok(_) => {
                    // success - remove backup
                    fs::remove_file(&backup_path).ok();
                    Ok(())
                }
                Err(e) => {
                    // test failed - rollback
                    eprintln!("update test failed: {}", e);
                    eprintln!("rolling back to previous version...");

                    fs::remove_file(&current_exe).ok();
                    fs::rename(&backup_path, &current_exe).context("failed to rollback")?;

                    // restore man page from rolled-back binary
                    if let Err(man_err) = crate::installer::install::install_man_page(false) {
                        eprintln!("warning: failed to restore man page: {}", man_err);
                    }

                    Err(anyhow!("update rolled back due to test failure: {}", e))
                }
            }
        }
        Err(e) => {
            // update failed - restore backup
            fs::rename(&backup_path, &current_exe).ok();
            Err(e)
        }
    }
}

fn test_updated_binary(binary_path: &Path) -> Result<()> {
    // run version command to verify binary works
    let output = Command::new(binary_path).arg("version").output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "binary failed version check: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // verify output contains expected version format
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains("cwm") {
        return Err(anyhow!("binary version output invalid"));
    }

    Ok(())
}

/// report update failure by opening browser with pre-filled GitHub issue
pub fn report_update_failure(
    attempt: &UpdateAttempt,
    error: &anyhow::Error,
    telemetry: &TelemetrySettings,
) -> Result<()> {
    if !telemetry.enabled {
        return Ok(());
    }

    let mut report = HashMap::new();

    // basic error info
    report.insert("error_message".to_string(), error.to_string());
    report.insert("error_chain".to_string(), format!("{:?}", error));
    report.insert("new_version".to_string(), attempt.new_version.clone());
    report.insert(
        "previous_version".to_string(),
        attempt.previous_version.clone(),
    );
    report.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());

    // system info if enabled
    if telemetry.include_system_info {
        if let Ok(os_version) = get_macos_version() {
            report.insert("os_version".to_string(), os_version);
        }
        report.insert(
            "architecture".to_string(),
            std::env::consts::ARCH.to_string(),
        );
        report.insert("cpu_count".to_string(), num_cpus::get().to_string());

        if let Ok((total, available)) = get_memory_info() {
            report.insert("memory_total".to_string(), total.to_string());
            report.insert("memory_available".to_string(), available.to_string());
        }
    }

    // open browser with pre-filled issue
    open_error_report_in_browser(&report)?;

    Ok(())
}

fn get_macos_version() -> Result<String> {
    let output = Command::new("sw_vers").arg("-productVersion").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_memory_info() -> Result<(u64, u64)> {
    let output = Command::new("sysctl").args(["-n", "hw.memsize"]).output()?;

    let total = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .unwrap_or(0);

    // approximate available memory
    let available = total / 2;

    Ok((total, available))
}

fn open_error_report_in_browser(report: &HashMap<String, String>) -> Result<()> {
    let repo = env!("GITHUB_REPO");

    // build issue title
    let title = format!(
        "Update failure: {} -> {}",
        report
            .get("previous_version")
            .unwrap_or(&"unknown".to_string()),
        report.get("new_version").unwrap_or(&"unknown".to_string())
    );

    // build issue body
    let mut body = String::from("## Update Error Report\n\n");
    body.push_str("An update failed and was automatically rolled back.\n\n");

    body.push_str("### Error Details\n");
    body.push_str(&format!(
        "**Error:** {}\n",
        report
            .get("error_message")
            .unwrap_or(&"unknown".to_string())
    ));
    body.push_str(&format!(
        "**Timestamp:** {}\n",
        report.get("timestamp").unwrap_or(&"unknown".to_string())
    ));
    body.push('\n');

    body.push_str("### Version Information\n");
    body.push_str(&format!(
        "**Previous:** {}\n",
        report
            .get("previous_version")
            .unwrap_or(&"unknown".to_string())
    ));
    body.push_str(&format!(
        "**Attempted:** {}\n",
        report.get("new_version").unwrap_or(&"unknown".to_string())
    ));
    body.push('\n');

    if report.contains_key("os_version") {
        body.push_str("### System Information\n");
        body.push_str(&format!(
            "**OS:** macOS {}\n",
            report.get("os_version").unwrap_or(&"unknown".to_string())
        ));
        body.push_str(&format!(
            "**Architecture:** {}\n",
            report.get("architecture").unwrap_or(&"unknown".to_string())
        ));
        body.push_str(&format!(
            "**CPUs:** {}\n",
            report.get("cpu_count").unwrap_or(&"unknown".to_string())
        ));
        if let Some(mem) = report.get("memory_total") {
            let mem_gb = mem.parse::<u64>().unwrap_or(0) / 1_073_741_824;
            body.push_str(&format!("**Memory:** {} GB\n", mem_gb));
        }
        body.push('\n');
    }

    body.push_str("### Debug Information\n");
    body.push_str("<details>\n<summary>Full error chain</summary>\n\n");
    body.push_str("```\n");
    body.push_str(
        report
            .get("error_chain")
            .unwrap_or(&"unavailable".to_string()),
    );
    body.push_str("\n```\n</details>\n");

    // url-encode title and body
    let encoded_title = urlencoding::encode(&title);
    let encoded_body = urlencoding::encode(&body);

    // build GitHub new issue URL with pre-filled template
    let url = format!(
        "https://github.com/{}/issues/new?title={}&body={}&labels=bug,update-failure",
        repo, encoded_title, encoded_body
    );

    // open in default browser
    Command::new("open").arg(&url).spawn()?;

    eprintln!("opening browser to report this issue...");
    eprintln!(
        "if the browser doesn't open, visit: https://github.com/{}/issues/new",
        repo
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_macos_version() {
        let version = get_macos_version();
        assert!(version.is_ok());
        let v = version.unwrap();
        // should be something like "14.2.1" or "13.5"
        assert!(!v.is_empty());
        assert!(v.contains('.'));
    }

    #[test]
    fn test_get_memory_info() {
        let mem = get_memory_info();
        assert!(mem.is_ok());
        let (total, available) = mem.unwrap();
        // should have at least 1GB
        assert!(total > 1_000_000_000);
        assert!(available > 0);
    }

    #[test]
    fn test_update_attempt_fields() {
        let attempt = UpdateAttempt {
            new_version: "stable-abc12345".to_string(),
            previous_version: "stable-def67890".to_string(),
        };
        assert_eq!(attempt.new_version, "stable-abc12345");
        assert_eq!(attempt.previous_version, "stable-def67890");
    }
}
