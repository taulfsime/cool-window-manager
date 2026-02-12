use anyhow::Result;
use std::path::PathBuf;

const PID_FILE: &str = "/tmp/cwm.pid";
const SOCKET_FILE: &str = "/tmp/cwm.sock";

pub fn get_pid_file_path() -> PathBuf {
    PathBuf::from(PID_FILE)
}

pub fn get_socket_path() -> PathBuf {
    PathBuf::from(SOCKET_FILE)
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

pub fn remove_socket_file() -> Result<()> {
    let path = get_socket_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Send a command to the daemon via Unix socket
/// Returns Ok(response) if successful, Err if daemon not running or command failed
#[allow(dead_code)]
#[cfg(target_os = "macos")]
pub fn send_command(command: &str) -> Result<String> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let socket_path = get_socket_path();

    if !socket_path.exists() {
        anyhow::bail!("Daemon not running (socket not found). Start with: cwm daemon start");
    }

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| anyhow::anyhow!("Failed to connect to daemon: {}", e))?;

    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    // send command
    stream.write_all(command.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // shutdown write side to signal we're done sending
    stream.shutdown(std::net::Shutdown::Write)?;

    // read response
    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    Ok(response)
}

#[allow(dead_code)]
#[cfg(not(target_os = "macos"))]
pub fn send_command(_command: &str) -> Result<String> {
    anyhow::bail!("IPC is only supported on macOS")
}
