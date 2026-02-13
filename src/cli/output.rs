//! output formatting utilities for scriptable CLI output
//!
//! uses JSON-RPC 2.0 format for machine-readable output:
//! - success: {"jsonrpc": "2.0", "result": {...}, "id": null}
//! - error: {"jsonrpc": "2.0", "error": {"code": N, "message": "...", "data": {...}}, "id": null}
//!
//! also provides format string templating for flexible scripting support

use serde::Serialize;
use std::io::IsTerminal;

/// JSON-RPC version constant
const JSONRPC_VERSION: &str = "2.0";

/// output mode determines how results are formatted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// human-readable text output
    Text,
    /// machine-readable JSON-RPC 2.0 output
    Json,
    /// no output on success (errors still go to stderr)
    Quiet,
    /// one item name per line, ideal for piping to fzf/xargs
    Names,
    /// custom format string with {field} placeholders
    Format,
}

impl OutputMode {
    /// determine output mode from CLI flags and environment
    ///
    /// priority: quiet > names > format > json > no_json > auto-detect
    pub fn from_flags(json: bool, no_json: bool, quiet: bool, names: bool, format: bool) -> Self {
        if quiet {
            return Self::Quiet;
        }
        if names {
            return Self::Names;
        }
        if format {
            return Self::Format;
        }
        if json {
            return Self::Json;
        }
        if no_json {
            return Self::Text;
        }
        // auto-detect: JSON when stdout is not a TTY (piped)
        if !std::io::stdout().is_terminal() {
            Self::Json
        } else {
            Self::Text
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }

    #[allow(dead_code)]
    pub fn is_quiet(&self) -> bool {
        matches!(self, Self::Quiet)
    }
}

/// JSON-RPC 2.0 success response
#[derive(Serialize)]
pub struct JsonRpcResponse<T: Serialize> {
    pub jsonrpc: &'static str,
    pub result: T,
    /// null for CLI responses (no request id)
    pub id: Option<String>,
}

impl<T: Serialize> JsonRpcResponse<T> {
    pub fn new(result: T) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            result,
            id: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_id(result: T, id: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            result,
            id: Some(id.into()),
        }
    }
}

/// JSON-RPC 2.0 error response
#[derive(Serialize)]
pub struct JsonRpcError {
    pub jsonrpc: &'static str,
    pub error: RpcError,
    pub id: Option<String>,
}

/// JSON-RPC 2.0 error object
#[derive(Serialize)]
pub struct RpcError {
    /// error code (using cwm exit codes, offset by -32000 for app-specific errors)
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ErrorData>,
}

/// additional error data
#[derive(Serialize)]
pub struct ErrorData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl JsonRpcError {
    /// create error with standard JSON-RPC error code range
    /// cwm uses -32000 to -32099 for application errors (per JSON-RPC spec)
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            error: RpcError {
                code: to_jsonrpc_code(code),
                message: message.into(),
                data: None,
            },
            id: None,
        }
    }

    pub fn with_suggestions(
        code: i32,
        message: impl Into<String>,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            error: RpcError {
                code: to_jsonrpc_code(code),
                message: message.into(),
                data: Some(ErrorData {
                    suggestions: if suggestions.is_empty() {
                        None
                    } else {
                        Some(suggestions)
                    },
                    details: None,
                }),
            },
            id: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// convert cwm exit code to JSON-RPC error code
/// JSON-RPC reserves -32000 to -32099 for server/application errors
fn to_jsonrpc_code(cwm_code: i32) -> i32 {
    -32000 - cwm_code
}

/// convert JSON-RPC error code back to cwm exit code
#[allow(dead_code)]
pub fn from_jsonrpc_code(rpc_code: i32) -> i32 {
    -(rpc_code + 32000)
}

// ============================================================================
// Result data structures for different actions
// ============================================================================

/// result data for focus action
#[derive(Serialize)]
pub struct FocusData {
    pub action: &'static str,
    pub app: AppData,
    #[serde(rename = "match")]
    pub match_info: MatchData,
}

/// result data for maximize action
#[derive(Serialize)]
pub struct MaximizeData {
    pub action: &'static str,
    pub app: AppData,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "match")]
    pub match_info: Option<MatchData>,
}

/// result data for resize action
#[derive(Serialize)]
pub struct ResizeData {
    pub action: &'static str,
    pub app: AppData,
    pub size: SizeData,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "match")]
    pub match_info: Option<MatchData>,
}

#[derive(Serialize)]
pub struct SizeData {
    pub width: u32,
    pub height: u32,
}

/// result data for move-display action
#[derive(Serialize)]
pub struct MoveDisplayData {
    pub action: &'static str,
    pub app: AppData,
    pub display: DisplayData,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "match")]
    pub match_info: Option<MatchData>,
}

#[derive(Serialize, Clone)]
pub struct DisplayData {
    pub index: usize,
    pub name: String,
}

/// basic app information for action results
#[derive(Serialize, Clone)]
pub struct AppData {
    pub name: String,
    pub pid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
}

/// match information showing how an app was found
#[derive(Serialize, Clone)]
pub struct MatchData {
    #[serde(rename = "type")]
    pub match_type: String,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<usize>,
}

// ============================================================================
// Output functions
// ============================================================================

/// format a string template with {field} placeholders
///
/// # example
/// ```ignore
/// let data = AppData { name: "Safari".into(), pid: 1234, bundle_id: None };
/// let result = format_template("{name} ({pid})", &data);
/// assert_eq!(result, "Safari (1234)");
/// ```
pub fn format_template<T: Serialize>(template: &str, data: &T) -> String {
    let value = match serde_json::to_value(data) {
        Ok(v) => v,
        Err(_) => return template.to_string(),
    };

    let mut result = template.to_string();

    if let serde_json::Value::Object(map) = value {
        for (key, val) in map {
            let placeholder = format!("{{{}}}", key);
            let replacement = match val {
                serde_json::Value::String(s) => s,
                serde_json::Value::Null => String::new(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Array(arr) => {
                    // join array elements with comma
                    arr.iter()
                        .filter_map(|v| match v {
                            serde_json::Value::String(s) => Some(s.clone()),
                            _ => Some(v.to_string()),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                }
                serde_json::Value::Object(_) => val.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
    }

    result
}

/// print JSON-RPC success response to stdout
pub fn print_json<T: Serialize>(data: &T) {
    let response = JsonRpcResponse::new(data);
    if let Ok(json) = serde_json::to_string(&response) {
        println!("{}", json);
    }
}

/// print JSON-RPC error to stdout
pub fn print_json_error(code: i32, message: &str) {
    let error = JsonRpcError::new(code, message);
    if let Ok(json) = serde_json::to_string(&error) {
        println!("{}", json);
    }
}

/// print JSON-RPC error with suggestions
pub fn print_json_error_with_suggestions(code: i32, message: &str, suggestions: Vec<String>) {
    let error = JsonRpcError::with_suggestions(code, message, suggestions);
    if let Ok(json) = serde_json::to_string(&error) {
        println!("{}", json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_mode_from_flags_quiet_wins() {
        assert_eq!(
            OutputMode::from_flags(true, false, true, false, false),
            OutputMode::Quiet
        );
    }

    #[test]
    fn test_output_mode_from_flags_names() {
        assert_eq!(
            OutputMode::from_flags(false, false, false, true, false),
            OutputMode::Names
        );
    }

    #[test]
    fn test_output_mode_from_flags_json() {
        assert_eq!(
            OutputMode::from_flags(true, false, false, false, false),
            OutputMode::Json
        );
    }

    #[test]
    fn test_output_mode_from_flags_no_json() {
        assert_eq!(
            OutputMode::from_flags(false, true, false, false, false),
            OutputMode::Text
        );
    }

    #[test]
    fn test_format_template_basic() {
        #[derive(Serialize)]
        struct Data {
            name: String,
            pid: i32,
        }

        let data = Data {
            name: "Safari".to_string(),
            pid: 1234,
        };

        assert_eq!(format_template("{name}", &data), "Safari");
        assert_eq!(format_template("{pid}", &data), "1234");
        assert_eq!(format_template("{name} ({pid})", &data), "Safari (1234)");
    }

    #[test]
    fn test_format_template_missing_field() {
        #[derive(Serialize)]
        struct Data {
            name: String,
        }

        let data = Data {
            name: "Test".to_string(),
        };

        // unknown placeholders are left as-is
        assert_eq!(format_template("{name} {unknown}", &data), "Test {unknown}");
    }

    #[test]
    fn test_format_template_with_none() {
        #[derive(Serialize)]
        struct Data {
            name: String,
            bundle_id: Option<String>,
        }

        let data = Data {
            name: "Test".to_string(),
            bundle_id: None,
        };

        assert_eq!(format_template("{name} {bundle_id}", &data), "Test ");
    }

    #[test]
    fn test_format_template_with_array() {
        #[derive(Serialize)]
        struct Data {
            titles: Vec<String>,
        }

        let data = Data {
            titles: vec!["Tab 1".to_string(), "Tab 2".to_string()],
        };

        assert_eq!(format_template("{titles}", &data), "Tab 1, Tab 2");
    }

    #[test]
    fn test_jsonrpc_response_format() {
        let data = FocusData {
            action: "focus",
            app: AppData {
                name: "Safari".to_string(),
                pid: 1234,
                bundle_id: Some("com.apple.Safari".to_string()),
            },
            match_info: MatchData {
                match_type: "exact".to_string(),
                query: "safari".to_string(),
                distance: None,
            },
        };

        let response = JsonRpcResponse::new(&data);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"result\":"));
        assert!(json.contains("\"id\":null"));
        assert!(json.contains("\"action\":\"focus\""));
        assert!(json.contains("\"name\":\"Safari\""));
    }

    #[test]
    fn test_jsonrpc_error_format() {
        let error = JsonRpcError::new(2, "App not found");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"error\":"));
        assert!(json.contains("\"code\":-32002")); // -32000 - 2
        assert!(json.contains("\"message\":\"App not found\""));
        assert!(json.contains("\"id\":null"));
    }

    #[test]
    fn test_jsonrpc_error_with_suggestions() {
        let error = JsonRpcError::with_suggestions(
            2,
            "App 'Safar' not found",
            vec!["Safari".to_string(), "Slack".to_string()],
        );
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"data\":"));
        assert!(json.contains("\"suggestions\":[\"Safari\",\"Slack\"]"));
    }

    #[test]
    fn test_jsonrpc_code_conversion() {
        // cwm code 0 -> JSON-RPC -32000
        assert_eq!(to_jsonrpc_code(0), -32000);
        // cwm code 2 -> JSON-RPC -32002
        assert_eq!(to_jsonrpc_code(2), -32002);
        // round-trip
        assert_eq!(from_jsonrpc_code(to_jsonrpc_code(5)), 5);
    }

    #[test]
    fn test_jsonrpc_response_with_id() {
        let data = AppData {
            name: "Test".to_string(),
            pid: 1,
            bundle_id: None,
        };
        let response = JsonRpcResponse::with_id(&data, "req-123");
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"id\":\"req-123\""));
    }
}
