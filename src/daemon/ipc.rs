#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SOCKET_PATH: &str = "/tmp/cwm.sock";
const PID_FILE: &str = "/tmp/cwm.pid";

pub fn get_socket_path() -> PathBuf {
    PathBuf::from(SOCKET_PATH)
}

pub fn get_pid_file_path() -> PathBuf {
    PathBuf::from(PID_FILE)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    Focus { app: String },
    Maximize { app: Option<String> },
    MoveDisplay { target: String, app: Option<String> },
    Stop,
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Response {
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
            data: None,
        }
    }

    pub fn ok_with_data(data: serde_json::Value) -> Self {
        Self {
            success: true,
            error: None,
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(message.into()),
            data: None,
        }
    }
}

/// Send a request to the daemon and get a response
pub async fn send_request(_request: &Request) -> Result<Response> {
    // TODO: implement IPC client
    Ok(Response::error("IPC not yet implemented"))
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

/// Write PID file
pub fn write_pid_file() -> Result<()> {
    let pid = std::process::id();
    std::fs::write(get_pid_file_path(), pid.to_string())?;
    Ok(())
}

/// Remove PID file
pub fn remove_pid_file() -> Result<()> {
    let path = get_pid_file_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
