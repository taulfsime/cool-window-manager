use anyhow::Result;
use std::path::PathBuf;

const PID_FILE: &str = "/tmp/cwm.pid";

pub fn get_pid_file_path() -> PathBuf {
    PathBuf::from(PID_FILE)
}

/// Check if daemon is running by checking PID file
pub fn is_daemon_running() -> bool {
    use std::process::Command;

    let pid_path = get_pid_file_path();

    if !pid_path.exists() {
        return false;
    }

    // read PID and check if process is running
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // check if process exists using kill -0
            return Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        }
    }

    false
}

pub fn write_pid_file() -> Result<()> {
    let pid = std::process::id();
    std::fs::write(get_pid_file_path(), pid.to_string())?;
    Ok(())
}

pub fn remove_pid_file() -> Result<()> {
    let path = get_pid_file_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
