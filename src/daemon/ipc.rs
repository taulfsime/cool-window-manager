use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::exit_codes;
use crate::cli::output::{JsonRpcError, JsonRpcResponse};

const PID_FILE: &str = "/tmp/cwm.pid";
const SOCKET_FILE: &str = "cwm.sock";

// ============================================================================
// Input format detection and parsing
// ============================================================================

/// Detected input format for IPC messages
#[derive(Debug, Clone, PartialEq)]
pub enum InputFormat {
    /// JSON format: {"method": "...", "params": {...}, "id": ...}
    /// The "jsonrpc": "2.0" field is optional
    Json,
    /// Plain text format: "focus:Safari"
    Text,
}

/// Parsed IPC request (internal representation)
#[derive(Debug, Clone)]
pub struct IpcRequest {
    /// method to execute
    pub method: String,
    /// parameters
    pub params: HashMap<String, String>,
    /// original input format (for response formatting)
    pub format: InputFormat,
    /// request id (if provided) - when absent, treated as notification
    pub id: Option<serde_json::Value>,
}

/// JSON request structure (jsonrpc field is optional)
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
    /// Parse an IPC message into a request, detecting the format automatically
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();

        // try JSON format (starts with '{')
        if trimmed.starts_with('{') {
            if let Ok(req) = serde_json::from_str::<JsonRequest>(trimmed) {
                return Ok(Self {
                    method: req.method,
                    params: req.params,
                    format: InputFormat::Json,
                    id: req.id,
                });
            }
            // invalid JSON
            return Err(anyhow!("Invalid JSON request"));
        }

        // plain text format: "command" or "command:arg" or "command:arg1:arg2"
        Ok(Self::parse_text(trimmed))
    }

    /// Parse plain text format into a request
    fn parse_text(input: &str) -> Self {
        let parts: Vec<&str> = input.splitn(2, ':').collect();
        let method = parts[0].to_string();
        let mut params = HashMap::new();

        if parts.len() > 1 {
            // for text format, the argument after : is context-dependent
            // focus:Safari -> app=Safari
            // maximize:Safari -> app=Safari
            // resize:80 -> size=80
            // resize:80:Safari -> size=80, app=Safari
            // move_display:next -> target=next
            // move_display:next:Safari -> target=next, app=Safari
            let arg = parts[1];

            match method.as_str() {
                "focus" | "maximize" => {
                    params.insert("app".to_string(), arg.to_string());
                }
                "resize" => {
                    let resize_parts: Vec<&str> = arg.splitn(2, ':').collect();
                    params.insert("to".to_string(), resize_parts[0].to_string());
                    if resize_parts.len() > 1 {
                        params.insert("app".to_string(), resize_parts[1].to_string());
                    }
                }
                "move_display" => {
                    let move_parts: Vec<&str> = arg.splitn(2, ':').collect();
                    params.insert("target".to_string(), move_parts[0].to_string());
                    if move_parts.len() > 1 {
                        params.insert("app".to_string(), move_parts[1].to_string());
                    }
                }
                _ => {
                    // generic: treat as first positional argument
                    params.insert("arg".to_string(), arg.to_string());
                }
            }
        }

        Self {
            method,
            params,
            format: InputFormat::Text,
            id: None,
        }
    }

    /// Check if this is a notification (JSON request without id)
    pub fn is_notification(&self) -> bool {
        self.format == InputFormat::Json && self.id.is_none()
    }
}

// ============================================================================
// Response formatting
// ============================================================================

/// Format a successful response based on the input format
pub fn format_success_response(request: &IpcRequest, result: impl Serialize) -> Option<String> {
    // notifications don't get responses
    if request.is_notification() {
        return None;
    }

    match request.format {
        InputFormat::Text => Some("OK".to_string()),
        InputFormat::Json => {
            let response = if let Some(ref id) = request.id {
                // echo back the id
                let mut resp = JsonRpcResponse::new(&result);
                resp.id = match id {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                };
                resp
            } else {
                JsonRpcResponse::new(&result)
            };
            serde_json::to_string(&response).ok()
        }
    }
}

/// Format an error response based on the input format
pub fn format_error_response(request: &IpcRequest, code: i32, message: &str) -> Option<String> {
    // notifications don't get responses
    if request.is_notification() {
        return None;
    }

    match request.format {
        InputFormat::Text => Some(format!("ERROR: {}", message)),
        InputFormat::Json => {
            let mut error = JsonRpcError::new(code, message);
            if let Some(ref id) = request.id {
                error.id = match id {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                };
            }
            serde_json::to_string(&error).ok()
        }
    }
}

/// Format an error response with a default error code
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

/// Send a command to the daemon via Unix socket (plain text protocol)
/// Returns Ok(response) if successful, Err if daemon not running or command failed
#[allow(dead_code)]
pub fn send_text_command(command: &str) -> Result<String> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let socket_path = get_socket_path();

    if !socket_path.exists() {
        anyhow::bail!(
            "Daemon not running (socket not found at {}). Start with: cwm daemon start",
            socket_path.display()
        );
    }

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| anyhow!("Failed to connect to daemon: {}", e))?;

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
        assert_eq!(req.format, InputFormat::Json);
        assert!(req.id.is_some());
    }

    #[test]
    fn test_parse_json_request_without_jsonrpc_field() {
        // jsonrpc field is optional
        let input = r#"{"method":"focus","params":{"app":"Safari"},"id":1}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "focus");
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
        assert_eq!(req.format, InputFormat::Json);
        assert!(req.id.is_some());
    }

    #[test]
    fn test_parse_json_notification() {
        let input = r#"{"jsonrpc":"2.0","method":"focus","params":{"app":"Safari"}}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "focus");
        assert_eq!(req.format, InputFormat::Json);
        assert!(req.id.is_none());
        assert!(req.is_notification());
    }

    #[test]
    fn test_parse_json_notification_without_jsonrpc_field() {
        // notification without jsonrpc field
        let input = r#"{"method":"maximize","params":{"app":"Chrome"}}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "maximize");
        assert_eq!(req.format, InputFormat::Json);
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
    }

    #[test]
    fn test_parse_json_minimal() {
        // minimal JSON request - only method is required
        let input = r#"{"method":"ping"}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "ping");
        assert!(req.params.is_empty());
        assert_eq!(req.format, InputFormat::Json);
        assert!(req.id.is_none());
    }

    #[test]
    fn test_parse_text_simple() {
        let input = "ping";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "ping");
        assert!(req.params.is_empty());
        assert_eq!(req.format, InputFormat::Text);
    }

    #[test]
    fn test_parse_text_focus() {
        let input = "focus:Safari";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "focus");
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
        assert_eq!(req.format, InputFormat::Text);
    }

    #[test]
    fn test_parse_text_resize() {
        let input = "resize:80";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "resize");
        assert_eq!(req.params.get("to"), Some(&"80".to_string()));
    }

    #[test]
    fn test_parse_text_resize_with_app() {
        let input = "resize:80:Safari";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "resize");
        assert_eq!(req.params.get("to"), Some(&"80".to_string()));
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
    }

    #[test]
    fn test_parse_text_move_display() {
        let input = "move_display:next";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "move_display");
        assert_eq!(req.params.get("target"), Some(&"next".to_string()));
    }

    #[test]
    fn test_parse_text_move_display_with_app() {
        let input = "move_display:next:Safari";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "move_display");
        assert_eq!(req.params.get("target"), Some(&"next".to_string()));
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
    }

    #[test]
    fn test_format_success_text() {
        let req = IpcRequest::parse("ping").unwrap();
        let response = format_success_response(&req, "pong");

        assert_eq!(response, Some("OK".to_string()));
    }

    #[test]
    fn test_format_success_jsonrpc() {
        let input = r#"{"jsonrpc":"2.0","method":"ping","id":1}"#;
        let req = IpcRequest::parse(input).unwrap();
        let response = format_success_response(&req, "pong").unwrap();

        assert!(response.contains("\"jsonrpc\":\"2.0\""));
        assert!(response.contains("\"result\":\"pong\""));
        assert!(response.contains("\"id\":\"1\""));
    }

    #[test]
    fn test_format_success_jsonrpc_string_id() {
        let input = r#"{"jsonrpc":"2.0","method":"ping","id":"req-456"}"#;
        let req = IpcRequest::parse(input).unwrap();
        let response = format_success_response(&req, "pong").unwrap();

        assert!(response.contains("\"id\":\"req-456\""));
    }

    #[test]
    fn test_format_error_text() {
        let req = IpcRequest::parse("invalid").unwrap();
        let response = format_error(&req, "Unknown command");

        assert_eq!(response, Some("ERROR: Unknown command".to_string()));
    }

    #[test]
    fn test_format_error_jsonrpc() {
        let input = r#"{"jsonrpc":"2.0","method":"invalid","id":1}"#;
        let req = IpcRequest::parse(input).unwrap();
        let response =
            format_error_response(&req, exit_codes::APP_NOT_FOUND, "App not found").unwrap();

        assert!(response.contains("\"jsonrpc\":\"2.0\""));
        assert!(response.contains("\"error\":"));
        assert!(response.contains("\"code\":-32002")); // -32000 - 2
        assert!(response.contains("\"message\":\"App not found\""));
    }

    #[test]
    fn test_notification_no_response() {
        let input = r#"{"jsonrpc":"2.0","method":"focus","params":{"app":"Safari"}}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert!(req.is_notification());
        assert!(format_success_response(&req, "done").is_none());
        assert!(format_error(&req, "error").is_none());
    }

    // ========================================================================
    // Additional parsing edge cases
    // ========================================================================

    #[test]
    fn test_parse_json_with_whitespace() {
        let input = r#"  {  "method" : "ping"  }  "#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "ping");
        assert_eq!(req.format, InputFormat::Json);
    }

    #[test]
    fn test_parse_json_invalid() {
        let input = r#"{"method": }"#;
        let result = IpcRequest::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_json_missing_method() {
        // serde will fail if method is missing
        let input = r#"{"params": {"app": "Safari"}}"#;
        let result = IpcRequest::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_text_empty() {
        let input = "";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "");
        assert!(req.params.is_empty());
        assert_eq!(req.format, InputFormat::Text);
    }

    #[test]
    fn test_parse_text_whitespace_only() {
        let input = "   ";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "");
        assert!(req.params.is_empty());
    }

    #[test]
    fn test_parse_text_maximize() {
        let input = "maximize:Safari";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "maximize");
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
    }

    #[test]
    fn test_parse_text_generic_action() {
        let input = "custom_action:some_value";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "custom_action");
        // generic actions use "arg" as the key
        assert_eq!(req.params.get("arg"), Some(&"some_value".to_string()));
    }

    #[test]
    fn test_parse_text_colon_in_value() {
        // resize:80:Safari should parse correctly
        let input = "resize:80:Safari";
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "resize");
        assert_eq!(req.params.get("to"), Some(&"80".to_string()));
        assert_eq!(req.params.get("app"), Some(&"Safari".to_string()));
    }

    #[test]
    fn test_parse_json_null_id() {
        let input = r#"{"method":"ping","id":null}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "ping");
        // explicit null id is treated as None by serde's Option deserialization
        // this means it's a notification (no response expected)
        assert!(req.id.is_none());
    }

    #[test]
    fn test_parse_json_numeric_id() {
        let input = r#"{"method":"ping","id":42}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "ping");
        assert!(req.id.is_some());
        assert_eq!(req.id.unwrap(), serde_json::json!(42));
    }

    #[test]
    fn test_parse_json_empty_params() {
        let input = r#"{"method":"status","params":{},"id":1}"#;
        let req = IpcRequest::parse(input).unwrap();

        assert_eq!(req.method, "status");
        assert!(req.params.is_empty());
    }

    // ========================================================================
    // is_notification edge cases
    // ========================================================================

    #[test]
    fn test_text_request_not_notification() {
        // text requests are never notifications
        let req = IpcRequest::parse("ping").unwrap();
        assert!(!req.is_notification());
    }

    #[test]
    fn test_json_with_id_not_notification() {
        let input = r#"{"method":"ping","id":"abc"}"#;
        let req = IpcRequest::parse(input).unwrap();
        assert!(!req.is_notification());
    }

    // ========================================================================
    // Response formatting edge cases
    // ========================================================================

    #[test]
    fn test_format_success_json_complex_result() {
        let input = r#"{"method":"status","id":1}"#;
        let req = IpcRequest::parse(input).unwrap();

        let result = serde_json::json!({
            "running": true,
            "pid": 12345,
            "shortcuts": 5
        });

        let response = format_success_response(&req, result).unwrap();

        assert!(response.contains("\"running\":true"));
        assert!(response.contains("\"pid\":12345"));
        assert!(response.contains("\"shortcuts\":5"));
    }

    #[test]
    fn test_format_error_response_with_string_id() {
        let input = r#"{"method":"focus","id":"request-123"}"#;
        let req = IpcRequest::parse(input).unwrap();

        let response = format_error_response(&req, exit_codes::ERROR, "Something failed").unwrap();

        assert!(response.contains("\"id\":\"request-123\""));
        assert!(response.contains("\"message\":\"Something failed\""));
    }

    #[test]
    fn test_format_error_text_mode() {
        let req = IpcRequest::parse("invalid_command").unwrap();
        let response = format_error(&req, "Unknown command").unwrap();

        assert_eq!(response, "ERROR: Unknown command");
    }

    #[test]
    fn test_format_success_text_mode() {
        let req = IpcRequest::parse("ping").unwrap();
        let response = format_success_response(&req, "pong").unwrap();

        assert_eq!(response, "OK");
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

    // ========================================================================
    // InputFormat tests
    // ========================================================================

    #[test]
    fn test_input_format_equality() {
        assert_eq!(InputFormat::Json, InputFormat::Json);
        assert_eq!(InputFormat::Text, InputFormat::Text);
        assert_ne!(InputFormat::Json, InputFormat::Text);
    }

    #[test]
    fn test_input_format_clone() {
        let fmt = InputFormat::Json;
        let cloned = fmt.clone();
        assert_eq!(fmt, cloned);
    }

    #[test]
    fn test_input_format_debug() {
        let fmt = InputFormat::Json;
        let debug_str = format!("{:?}", fmt);
        assert!(debug_str.contains("Json"));
    }

    // ========================================================================
    // IpcRequest clone and debug
    // ========================================================================

    #[test]
    fn test_ipc_request_clone() {
        let req = IpcRequest::parse(r#"{"method":"ping","id":1}"#).unwrap();
        let cloned = req.clone();

        assert_eq!(req.method, cloned.method);
        assert_eq!(req.format, cloned.format);
    }

    #[test]
    fn test_ipc_request_debug() {
        let req = IpcRequest::parse("ping").unwrap();
        let debug_str = format!("{:?}", req);

        assert!(debug_str.contains("IpcRequest"));
        assert!(debug_str.contains("method"));
        assert!(debug_str.contains("ping"));
    }
}
