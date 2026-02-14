//! IPC (Inter-Process Communication) for daemon
//!
//! provides Unix socket communication using JSON-RPC 2.0 protocol.
//! plain text protocol has been removed - use JSON-RPC for all requests.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::exit_codes;
use crate::cli::output::{JsonRpcError, JsonRpcResponse};

const PID_FILE: &str = "/tmp/cwm.pid";
const SOCKET_FILE: &str = "cwm.sock";

// ============================================================================
// IPC Request parsing
// ============================================================================

/// Parsed IPC request
#[derive(Debug, Clone)]
pub struct IpcRequest {
    /// method to execute
    pub method: String,
    /// parameters as key-value pairs
    pub params: HashMap<String, String>,
    /// request id (if provided) - when absent, treated as notification
    pub id: Option<serde_json::Value>,
}

/// JSON request structure (jsonrpc field is optional for convenience)
#[derive(Debug, Clone, Deserialize)]
struct JsonRequest {
    /// optional jsonrpc version (ignored, for compatibility)
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    method: String,
    #[serde(default)]
    params: HashMap<String, String>,
    /// id can be string, number, or null
    id: Option<serde_json::Value>,
}

impl IpcRequest {
    /// Parse a JSON-RPC message into a request
    ///
    /// expects JSON format: `{"method": "...", "params": {...}, "id": ...}`
    /// the `jsonrpc` field is optional for convenience
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();

        if !trimmed.starts_with('{') {
            return Err(anyhow!(
                "Invalid request format. Expected JSON-RPC: {{\"method\": \"...\", \"params\": {{}}, \"id\": 1}}"
            ));
        }

        let req: JsonRequest = serde_json::from_str(trimmed)
            .map_err(|e| anyhow!("Invalid JSON-RPC request: {}", e))?;

        Ok(Self {
            method: req.method,
            params: req.params,
            id: req.id,
        })
    }

    /// Check if this is a notification (request without id = no response expected)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Get the id as a string for response
    pub fn id_string(&self) -> Option<String> {
        self.id.as_ref().map(|id| match id {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => id.to_string(),
        })
    }
}

// ============================================================================
// Response formatting (JSON-RPC 2.0)
// ============================================================================

/// Format a successful JSON-RPC response
pub fn format_success_response<T: serde::Serialize>(
    request: &IpcRequest,
    result: T,
) -> Option<String> {
    // notifications don't get responses
    if request.is_notification() {
        return None;
    }

    let mut response = JsonRpcResponse::new(&result);
    response.id = request.id_string();
    serde_json::to_string(&response).ok()
}

/// Format a JSON-RPC error response
pub fn format_error_response(request: &IpcRequest, code: i32, message: &str) -> Option<String> {
    // notifications don't get responses
    if request.is_notification() {
        return None;
    }

    let mut error = JsonRpcError::new(code, message);
    error.id = request.id_string();
    serde_json::to_string(&error).ok()
}

/// Format an error response with default error code
#[allow(dead_code)]
pub fn format_error(request: &IpcRequest, message: &str) -> Option<String> {
    format_error_response(request, exit_codes::ERROR, message)
}

// ============================================================================
// PID and socket file management
// ============================================================================

pub fn get_pid_file_path() -> PathBuf {
    PathBuf::from(PID_FILE)
}

/// returns the socket path in the cwm config directory (~/.cwm/cwm.sock)
pub fn get_socket_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".cwm").join(SOCKET_FILE))
        .unwrap_or_else(|| PathBuf::from("/tmp").join(SOCKET_FILE))
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

// ============================================================================
// Client functions for sending requests to the daemon
// ============================================================================

/// Send a JSON-RPC 2.0 request to the daemon
/// Returns the raw JSON response string
#[allow(dead_code)]
pub fn send_jsonrpc(
    method: &str,
    params: HashMap<String, String>,
    id: Option<&str>,
) -> Result<String> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let socket_path = get_socket_path();

    if !socket_path.exists() {
        return Err(anyhow!(
            "Daemon not running (socket not found at {}). Start with: cwm daemon start",
            socket_path.display()
        ));
    }

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| anyhow!("Failed to connect to daemon: {}", e))?;

    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    // build JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    });

    let json = serde_json::to_string(&request)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // for notifications (no id), don't wait for response
    if id.is_none() {
        stream.shutdown(std::net::Shutdown::Write)?;
        return Ok(String::new());
    }

    // shutdown write side to signal we're done sending
    stream.shutdown(std::net::Shutdown::Write)?;

    // read JSON response
    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    Ok(response_line)
}

/// Helper to send a simple JSON-RPC command without parameters
#[allow(dead_code)]
pub fn send_jsonrpc_simple(method: &str) -> Result<String> {
    send_jsonrpc(method, HashMap::new(), Some("1"))
}

/// Helper to send a JSON-RPC command with a single parameter
#[allow(dead_code)]
pub fn send_jsonrpc_with_param(method: &str, key: &str, value: &str) -> Result<String> {
    let mut params = HashMap::new();
    params.insert(key.to_string(), value.to_string());
    send_jsonrpc(method, params, Some("1"))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_request_with_jsonrpc_field() {
        let input = r#"{"jsonrpc":"2.0","method":"focus","params":{"app":"Safari"},"id":1}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "focus");
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
        assert!(req.id.is_some());
    }

    #[test]
    fn test_parse_json_request_without_jsonrpc_field() {
        // jsonrpc field is optional
        let input = r#"{"method":"focus","params":{"app":"Safari"},"id":1}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "focus");
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
        assert!(req.id.is_some());
    }

    #[test]
    fn test_parse_json_notification() {
        let input = r#"{"jsonrpc":"2.0","method":"focus","params":{"app":"Safari"}}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "focus");
        assert!(req.id.is_none());
        assert!(req.is_notification());
    }

    #[test]
    fn test_parse_json_string_id() {
        let input = r#"{"jsonrpc":"2.0","method":"ping","id":"req-123"}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "ping");
        assert_eq!(
            req.id,
            Some(serde_json::Value::String("req-123".to_string()))
        );
        assert_eq!(req.id_string(), Some("req-123".to_string()));
    }

    #[test]
    fn test_parse_json_numeric_id_string() {
        let input = r#"{"method":"ping","id":456}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.id_string(), Some("456".to_string()));
    }

    #[test]
    fn test_parse_json_minimal() {
        // minimal JSON request - only method is required
        let input = r#"{"method":"ping"}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "ping");
        assert!(req.params.is_empty());
        assert!(req.id.is_none());
    }

    #[test]
    fn test_parse_plain_text_rejected() {
        let input = "focus:Safari";
        let result = IpcRequest::parse(input);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON-RPC"));
    }

    #[test]
    fn test_parse_invalid_json() {
        let input = r#"{"method": "focus", invalid}"#;
        let result = IpcRequest::parse(input);

        assert!(result.is_err());
    }

    #[test]
    fn test_format_success_response() {
        let input = r#"{"method":"ping","id":1}"#;
        let req = IpcRequest::parse(input).unwrap();
        let response = format_success_response(&req, "pong").unwrap();

        assert!(response.contains("\"jsonrpc\":\"2.0\""));
        assert!(response.contains("\"result\":\"pong\""));
        assert!(response.contains("\"id\":\"1\""));
    }

    #[test]
    fn test_format_success_response_string_id() {
        let input = r#"{"method":"ping","id":"req-456"}"#;
        let req = IpcRequest::parse(input).unwrap();
        let response = format_success_response(&req, "pong").unwrap();

        assert!(response.contains("\"id\":\"req-456\""));
    }

    #[test]
    fn test_format_error_response() {
        let input = r#"{"method":"invalid","id":1}"#;
        let req = IpcRequest::parse(input).unwrap();
        let response =
            format_error_response(&req, exit_codes::APP_NOT_FOUND, "App not found").unwrap();

        assert!(response.contains("\"jsonrpc\":\"2.0\""));
        assert!(response.contains("\"error\":"));
        assert!(response.contains("\"message\":\"App not found\""));
    }

    #[test]
    fn test_notification_no_response() {
        let input = r#"{"method":"focus","params":{"app":"Safari"}}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert!(req.is_notification());
        assert!(format_success_response(&req, "done").is_none());
        assert!(format_error(&req, "error").is_none());
    }

    // ========================================================================
    // Path functions
    // ========================================================================

    #[test]
    fn test_get_pid_file_path() {
        let path = get_pid_file_path();
        assert_eq!(path.to_string_lossy(), "/tmp/cwm.pid");
    }

    #[test]
    fn test_get_socket_path() {
        let path = get_socket_path();
        // should be in ~/.cwm/cwm.sock
        assert!(path.to_string_lossy().contains(".cwm"));
        assert!(path.to_string_lossy().ends_with("cwm.sock"));
    }
}
