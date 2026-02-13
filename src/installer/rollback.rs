use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::TelemetrySettings;
use crate::installer::github::GitHubClient;

pub struct UpdateAttempt {
    pub new_version: String,
    pub previous_version: String,
}

#[allow(dead_code)]
pub async fn safe_update_with_rollback<F>(update_fn: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension("backup");

    // create backup
    fs::copy(&current_exe, &backup_path).context("Failed to create backup")?;

    // attempt update
    match update_fn() {
        Ok(_) => {
            // test new binary
            match test_updated_binary(&current_exe).await {
                Ok(_) => {
                    // success - remove backup
                    fs::remove_file(&backup_path).ok();
                    Ok(())
                }
                Err(e) => {
                    // test failed - rollback
                    eprintln!("Update test failed: {}", e);
                    eprintln!("Rolling back to previous version...");

                    fs::remove_file(&current_exe).ok();
                    fs::rename(&backup_path, &current_exe).context("Failed to rollback")?;

                    // restore man page from rolled-back binary
                    if let Err(man_err) = crate::installer::install::install_man_page(false) {
                        eprintln!("⚠️  Failed to restore man page: {}", man_err);
                    }

                    Err(anyhow!("Update rolled back due to test failure: {}", e))
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

#[allow(dead_code)]
async fn test_updated_binary(binary_path: &Path) -> Result<()> {
    use tokio::process::Command;

    // run version command
    let output = Command::new(binary_path).arg("version").output().await?;

    if !output.status.success() {
        return Err(anyhow!(
            "Binary failed version check: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // could add more tests here
    // - check permissions command
    // - verify config loads

    Ok(())
}

#[allow(dead_code)]
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
        report.insert("os_version".to_string(), get_macos_version()?);
        report.insert(
            "architecture".to_string(),
            std::env::consts::ARCH.to_string(),
        );
        report.insert("cpu_count".to_string(), num_cpus::get().to_string());

        // memory info
        if let Ok(mem) = get_memory_info() {
            report.insert("memory_total".to_string(), mem.0.to_string());
            report.insert("memory_available".to_string(), mem.1.to_string());
        }
    }

    // operation log (last N lines of output)
    if let Ok(log) = capture_recent_logs() {
        report.insert("operation_log".to_string(), log);
    }

    // create GitHub issue
    create_error_report_issue(&report)?;

    Ok(())
}

#[allow(dead_code)]
fn get_macos_version() -> Result<String> {
    use std::process::Command;

    let output = Command::new("sw_vers").arg("-productVersion").output()?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[allow(dead_code)]
fn get_memory_info() -> Result<(u64, u64)> {
    use std::process::Command;

    let output = Command::new("sysctl").args(["-n", "hw.memsize"]).output()?;

    let total = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .unwrap_or(0);

    // this is approximate - better would be to use vm_stat
    let available = total / 2; // rough estimate

    Ok((total, available))
}

#[allow(dead_code)]
fn capture_recent_logs() -> Result<String> {
    // in a real implementation, we'd capture stdout/stderr during the update
    // for now, return a placeholder
    Ok("Update operation log would be captured here".to_string())
}

#[allow(dead_code)]
fn create_error_report_issue(report: &HashMap<String, String>) -> Result<()> {
    let repo = env!("GITHUB_REPO");
    let client = GitHubClient::new(repo)?;

    // format issue title
    let title = format!(
        "Update failure: {} -> {}",
        report
            .get("previous_version")
            .unwrap_or(&"unknown".to_string()),
        report.get("new_version").unwrap_or(&"unknown".to_string())
    );

    // format issue body
    let mut body = String::from("## Automatic Error Report\n\n");
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
            report.get("os_version").unwrap()
        ));
        body.push_str(&format!(
            "**Architecture:** {}\n",
            report.get("architecture").unwrap()
        ));
        body.push_str(&format!("**CPUs:** {}\n", report.get("cpu_count").unwrap()));
        if let Some(mem) = report.get("memory_total") {
            let mem_gb = mem.parse::<u64>().unwrap_or(0) / 1_073_741_824;
            body.push_str(&format!("**Memory:** {} GB\n", mem_gb));
        }
        body.push('\n');
    }

    if let Some(log) = report.get("operation_log") {
        body.push_str("### Operation Log\n");
        body.push_str("```\n");
        body.push_str(log);
        body.push_str("\n```\n");
    }

    body.push_str("\n### Debug Information\n");
    body.push_str("<details>\n<summary>Full error chain</summary>\n\n");
    body.push_str("```\n");
    body.push_str(
        report
            .get("error_chain")
            .unwrap_or(&"unavailable".to_string()),
    );
    body.push_str("\n```\n</details>\n");

    // create issue with labels
    client.create_issue(
        &title,
        &body,
        vec!["bug", "auto-reported", "update-failure"],
    )?;

    println!("Error report submitted to GitHub");
    Ok(())
}
