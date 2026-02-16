//! JSON-RPC request parsing into Command
//!
//! this module provides parsing from JSON-RPC requests to the unified Command enum.
//! it uses the same validation logic as CLI (e.g., ResizeTarget::parse) to ensure
//! consistent behavior across all transports.

use std::path::PathBuf;

use serde::Deserialize;

use crate::actions::command::*;
use crate::actions::error::ActionError;
use crate::display::DisplayTarget;
use crate::window::manager::{MoveTarget, ResizeTarget};

/// JSON-RPC 2.0 request structure
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    /// optional jsonrpc version (accepted for compatibility, not used)
    #[serde(default, rename = "jsonrpc")]
    _jsonrpc: Option<String>,
    /// method name
    pub method: String,
    /// parameters (can be object or omitted)
    #[serde(default)]
    pub params: serde_json::Value,
    /// request id (accepted for protocol compliance, response handling is in daemon/ipc.rs)
    #[serde(default, rename = "id")]
    _id: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// parse JSON string into JsonRpcRequest
    pub fn parse(input: &str) -> Result<Self, ActionError> {
        serde_json::from_str(input)
            .map_err(|e| ActionError::invalid_args(format!("invalid JSON-RPC request: {}", e)))
    }

    /// convert JSON-RPC request to unified Command
    pub fn to_command(&self) -> Result<Command, ActionError> {
        let params = Params::new(&self.params);

        match self.method.as_str() {
            // ==================== Window Commands ====================
            "focus" => {
                let app = params.get_string_array("app")?;
                if app.is_empty() {
                    return Err(ActionError::invalid_args("focus requires 'app' parameter"));
                }
                Ok(Command::Focus {
                    app,
                    launch: params.get_optional_bool("launch")?,
                })
            }

            "maximize" => Ok(Command::Maximize {
                app: params.get_string_array_or_empty("app"),
                launch: params.get_optional_bool("launch")?,
            }),

            "resize" => {
                let to_str = params.get_string("to")?;
                let to = ResizeTarget::parse(&to_str)
                    .map_err(|e| ActionError::invalid_args(e.to_string()))?;
                Ok(Command::Resize {
                    app: params.get_string_array_or_empty("app"),
                    to,
                    overflow: params.get_bool_or("overflow", false),
                    launch: params.get_optional_bool("launch")?,
                })
            }

            "move" => {
                let to_str = params.get_optional_string("to")?;
                let display_str = params.get_optional_string("display")?;

                // at least one of to or display must be specified
                if to_str.is_none() && display_str.is_none() {
                    return Err(ActionError::invalid_args(
                        "move requires at least one of 'to' or 'display' parameter",
                    ));
                }

                let to = to_str
                    .map(|s| MoveTarget::parse(&s))
                    .transpose()
                    .map_err(|e| ActionError::invalid_args(e.to_string()))?;

                let display = display_str
                    .map(|s| DisplayTarget::parse(&s))
                    .transpose()
                    .map_err(|e| ActionError::invalid_args(e.to_string()))?;

                Ok(Command::Move {
                    app: params.get_string_array_or_empty("app"),
                    to,
                    display,
                    launch: params.get_optional_bool("launch")?,
                })
            }

            "kill" => {
                let app = params.get_string_array("app")?;
                if app.is_empty() {
                    return Err(ActionError::invalid_args("kill requires 'app' parameter"));
                }
                Ok(Command::Kill {
                    app,
                    force: params.get_bool_or("force", false),
                    wait: params.get_bool_or("wait", false),
                })
            }

            "close" => {
                let app = params.get_string_array("app")?;
                if app.is_empty() {
                    return Err(ActionError::invalid_args("close requires 'app' parameter"));
                }
                Ok(Command::Close { app })
            }

            // ==================== Query Commands ====================
            "list" => {
                let resource_str = params.get_string("resource")?;
                let resource = resource_str
                    .parse::<ListResource>()
                    .map_err(ActionError::invalid_args)?;
                Ok(Command::List {
                    resource,
                    detailed: params.get_bool_or("detailed", false),
                })
            }

            "get" => {
                let target_str = params.get_string("target")?;
                let target = match target_str.as_str() {
                    "focused" => GetTarget::Focused,
                    "window" => {
                        let app = params.get_string_array("app")?;
                        if app.is_empty() {
                            return Err(ActionError::invalid_args(
                                "get window requires 'app' parameter",
                            ));
                        }
                        GetTarget::Window { app }
                    }
                    _ => {
                        return Err(ActionError::invalid_args(format!(
                            "invalid target '{}', expected: focused, window",
                            target_str
                        )))
                    }
                };
                Ok(Command::Get { target })
            }

            // ==================== System Commands ====================
            "ping" => Ok(Command::Ping),

            "status" => Ok(Command::Status),

            "version" => Ok(Command::Version),

            "check_permissions" => Ok(Command::CheckPermissions {
                prompt: params.get_bool_or("prompt", false),
            }),

            // ==================== Daemon Commands ====================
            "daemon" => {
                let cmd = params.get_string("command")?;
                match cmd.as_str() {
                    "start" => Ok(Command::Daemon(DaemonCommand::Start {
                        log: params.get_optional_string("log")?,
                        foreground: params.get_bool_or("foreground", false),
                    })),
                    "stop" => Ok(Command::Daemon(DaemonCommand::Stop)),
                    "status" => Ok(Command::Daemon(DaemonCommand::Status)),
                    "install" => Ok(Command::Daemon(DaemonCommand::Install {
                        bin: params.get_optional_string("bin")?,
                        log: params.get_optional_string("log")?,
                    })),
                    "uninstall" => Ok(Command::Daemon(DaemonCommand::Uninstall)),
                    _ => Err(ActionError::invalid_args(format!(
                        "unknown daemon command '{}', expected: start, stop, status, install, uninstall",
                        cmd
                    ))),
                }
            }

            // ==================== Config Commands ====================
            "config" => {
                let cmd = params.get_string("command")?;
                match cmd.as_str() {
                    "show" => Ok(Command::Config(ConfigCommand::Show)),
                    "path" => Ok(Command::Config(ConfigCommand::Path)),
                    "set" => Ok(Command::Config(ConfigCommand::Set {
                        key: params.get_string("key")?,
                        value: params.get_string("value")?,
                    })),
                    "reset" => Ok(Command::Config(ConfigCommand::Reset)),
                    "default" => Ok(Command::Config(ConfigCommand::Default)),
                    "verify" => Ok(Command::Config(ConfigCommand::Verify)),
                    _ => Err(ActionError::invalid_args(format!(
                        "unknown config command '{}', expected: show, path, set, reset, default, verify",
                        cmd
                    ))),
                }
            }

            // ==================== Spotlight Commands ====================
            "spotlight" => {
                let cmd = params.get_string("command")?;
                match cmd.as_str() {
                    "install" => Ok(Command::Spotlight(SpotlightCommand::Install {
                        name: params.get_optional_string("name")?,
                        force: params.get_bool_or("force", false),
                    })),
                    "list" => Ok(Command::Spotlight(SpotlightCommand::List)),
                    "remove" => Ok(Command::Spotlight(SpotlightCommand::Remove {
                        name: params.get_optional_string("name")?,
                        all: params.get_bool_or("all", false),
                    })),
                    "example" => Ok(Command::Spotlight(SpotlightCommand::Example)),
                    _ => Err(ActionError::invalid_args(format!(
                        "unknown spotlight command '{}', expected: install, list, remove, example",
                        cmd
                    ))),
                }
            }

            // ==================== Install Commands ====================
            // note: install via IPC is limited - completions_only is the main use case
            "install" => Ok(Command::Install {
                path: params.get_optional_string("path")?.map(PathBuf::from),
                force: params.get_bool_or("force", false),
                no_sudo: params.get_bool_or("no_sudo", false),
                completions: params.get_optional_string("completions")?,
                no_completions: params.get_bool_or("no_completions", false),
                completions_only: params.get_bool_or("completions_only", false),
            }),

            "uninstall" => Ok(Command::Uninstall {
                path: params.get_optional_string("path")?.map(PathBuf::from),
            }),

            "update" => Ok(Command::Update {
                check: params.get_bool_or("check", false),
                force: params.get_bool_or("force", false),
                prerelease: params.get_bool_or("prerelease", false),
            }),

            // ==================== History Commands ====================
            "undo" => Ok(Command::Undo),

            "redo" => Ok(Command::Redo),

            "history" | "history_list" | "history_clear" => {
                // handle both "history" with command param and direct method names
                if self.method == "history_list" {
                    Ok(Command::History(HistoryCommand::List))
                } else if self.method == "history_clear" {
                    Ok(Command::History(HistoryCommand::Clear))
                } else {
                    let cmd = params.get_string("command")?;
                    match cmd.as_str() {
                        "list" => Ok(Command::History(HistoryCommand::List)),
                        "clear" => Ok(Command::History(HistoryCommand::Clear)),
                        _ => Err(ActionError::invalid_args(format!(
                            "unknown history command '{}', expected: list, clear",
                            cmd
                        ))),
                    }
                }
            }

            // ==================== Events Commands ====================
            // subscribe is handled specially by the daemon for persistent connections
            "subscribe" => Err(ActionError::not_supported(
                "subscribe is handled directly by the daemon socket listener",
            )),

            // ==================== Interactive/CLI-only Commands ====================
            "record_shortcut" => Err(ActionError::not_supported(
                "record_shortcut is interactive and not available via IPC",
            )),
            "record_layout" => Err(ActionError::not_supported(
                "record_layout is a CLI-only command",
            )),

            // ==================== Unknown ====================
            _ => Err(ActionError::invalid_args(format!(
                "unknown method '{}'",
                self.method
            ))),
        }
    }
}

/// helper for extracting typed values from JSON params
struct Params<'a> {
    value: &'a serde_json::Value,
}

impl<'a> Params<'a> {
    fn new(value: &'a serde_json::Value) -> Self {
        Self { value }
    }

    /// get required string parameter
    fn get_string(&self, key: &str) -> Result<String, ActionError> {
        self.value
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                ActionError::invalid_args(format!("missing required parameter: {}", key))
            })
    }

    /// get optional string parameter
    fn get_optional_string(&self, key: &str) -> Result<Option<String>, ActionError> {
        match self.value.get(key) {
            Some(v) if v.is_null() => Ok(None),
            Some(v) => v
                .as_str()
                .map(|s| Some(s.to_string()))
                .ok_or_else(|| ActionError::invalid_args(format!("{} must be a string", key))),
            None => Ok(None),
        }
    }

    /// get required string array parameter
    /// accepts both array and single string (converted to single-element array)
    fn get_string_array(&self, key: &str) -> Result<Vec<String>, ActionError> {
        let value = match self.value.get(key) {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        if value.is_null() {
            return Ok(Vec::new());
        }

        if let Some(arr) = value.as_array() {
            arr.iter()
                .map(|v| {
                    v.as_str().map(|s| s.to_string()).ok_or_else(|| {
                        ActionError::invalid_args(format!("{} must be array of strings", key))
                    })
                })
                .collect()
        } else if let Some(s) = value.as_str() {
            // single string → single-element array
            Ok(vec![s.to_string()])
        } else {
            Err(ActionError::invalid_args(format!(
                "{} must be a string or array of strings",
                key
            )))
        }
    }

    /// get string array or empty vec if not present
    fn get_string_array_or_empty(&self, key: &str) -> Vec<String> {
        self.get_string_array(key).unwrap_or_default()
    }

    /// get boolean with default value
    fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.value
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    /// get optional boolean parameter
    fn get_optional_bool(&self, key: &str) -> Result<Option<bool>, ActionError> {
        match self.value.get(key) {
            Some(v) if v.is_null() => Ok(None),
            Some(v) => v
                .as_bool()
                .map(Some)
                .ok_or_else(|| ActionError::invalid_args(format!("{} must be a boolean", key))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_focus() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"focus","params":{"app":["Safari"]}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Focus { app, launch } => {
                assert_eq!(app, vec!["Safari"]);
                assert_eq!(launch, None);
            }
            _ => panic!("expected Focus command"),
        }
    }

    #[test]
    fn test_parse_focus_single_string() {
        // single string should be converted to array
        let req = JsonRpcRequest::parse(r#"{"method":"focus","params":{"app":"Safari"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Focus { app, .. } => {
                assert_eq!(app, vec!["Safari"]);
            }
            _ => panic!("expected Focus command"),
        }
    }

    #[test]
    fn test_parse_focus_multiple_apps() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"focus","params":{"app":["Safari","Chrome","Firefox"]}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Focus { app, .. } => {
                assert_eq!(app, vec!["Safari", "Chrome", "Firefox"]);
            }
            _ => panic!("expected Focus command"),
        }
    }

    #[test]
    fn test_parse_focus_with_launch() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"focus","params":{"app":["Safari"],"launch":true}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Focus { app, launch } => {
                assert_eq!(app, vec!["Safari"]);
                assert_eq!(launch, Some(true));
            }
            _ => panic!("expected Focus command"),
        }
    }

    #[test]
    fn test_parse_focus_missing_app() {
        let req = JsonRpcRequest::parse(r#"{"method":"focus","params":{}}"#).unwrap();
        let result = req.to_command();

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("app"));
    }

    #[test]
    fn test_parse_maximize() {
        let req = JsonRpcRequest::parse(r#"{"method":"maximize","params":{}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Maximize { app, launch } => {
                assert!(app.is_empty());
                assert_eq!(launch, None);
            }
            _ => panic!("expected Maximize command"),
        }
    }

    #[test]
    fn test_parse_maximize_with_app() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"maximize","params":{"app":["Safari"]}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Maximize { app, .. } => {
                assert_eq!(app, vec!["Safari"]);
            }
            _ => panic!("expected Maximize command"),
        }
    }

    #[test]
    fn test_parse_resize() {
        let req = JsonRpcRequest::parse(r#"{"method":"resize","params":{"to":"80%"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Resize {
                app,
                to,
                overflow,
                launch,
            } => {
                assert!(app.is_empty());
                assert_eq!(to, ResizeTarget::Percent(80));
                assert!(!overflow);
                assert_eq!(launch, None);
            }
            _ => panic!("expected Resize command"),
        }
    }

    #[test]
    fn test_parse_resize_pixels() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"resize","params":{"to":"1920x1080px"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Resize { to, .. } => {
                assert_eq!(
                    to,
                    ResizeTarget::Pixels {
                        width: 1920,
                        height: Some(1080)
                    }
                );
            }
            _ => panic!("expected Resize command"),
        }
    }

    #[test]
    fn test_parse_resize_with_overflow() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"resize","params":{"to":"80","overflow":true,"app":["Safari"]}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Resize { app, overflow, .. } => {
                assert_eq!(app, vec!["Safari"]);
                assert!(overflow);
            }
            _ => panic!("expected Resize command"),
        }
    }

    #[test]
    fn test_parse_move_to_anchor() {
        let req = JsonRpcRequest::parse(r#"{"method":"move","params":{"to":"top-left"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Move {
                app, to, display, ..
            } => {
                assert!(app.is_empty());
                assert!(to.is_some());
                assert!(display.is_none());
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_parse_move_display_only() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"move","params":{"display":"next"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Move {
                app, to, display, ..
            } => {
                assert!(app.is_empty());
                assert!(to.is_none());
                assert!(matches!(display, Some(DisplayTarget::Next)));
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_parse_move_combined() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"move","params":{"to":"top-left","display":"2","app":["Safari"]}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Move {
                app, to, display, ..
            } => {
                assert_eq!(app, vec!["Safari"]);
                assert!(to.is_some());
                assert!(display.is_some());
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_parse_move_requires_to_or_display() {
        let req = JsonRpcRequest::parse(r#"{"method":"move","params":{}}"#).unwrap();
        let result = req.to_command();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("to") || err.message.contains("display"));
    }

    #[test]
    fn test_parse_move_to_percent() {
        let req = JsonRpcRequest::parse(r#"{"method":"move","params":{"to":"50%,50%"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Move { to, display, .. } => {
                assert!(to.is_some());
                assert!(display.is_none());
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_parse_move_to_relative() {
        let req = JsonRpcRequest::parse(r#"{"method":"move","params":{"to":"+100,-50"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Move { to, display, .. } => {
                assert!(to.is_some());
                assert!(display.is_none());
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_parse_move_to_center() {
        let req = JsonRpcRequest::parse(r#"{"method":"move","params":{"to":"center"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Move { to, display, .. } => {
                assert!(to.is_some());
                assert!(display.is_none());
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_parse_move_to_pixels() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"move","params":{"to":"100,200px"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Move { to, display, .. } => {
                assert!(to.is_some());
                assert!(display.is_none());
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_parse_move_invalid_to() {
        let req = JsonRpcRequest::parse(r#"{"method":"move","params":{"to":"invalid-position"}}"#)
            .unwrap();
        let result = req.to_command();

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_move_invalid_percent_range() {
        let req = JsonRpcRequest::parse(r#"{"method":"move","params":{"to":"150%"}}"#).unwrap();
        let result = req.to_command();

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_list() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"list","params":{"resource":"apps"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::List { resource, detailed } => {
                assert_eq!(resource, ListResource::Apps);
                assert!(!detailed);
            }
            _ => panic!("expected List command"),
        }
    }

    #[test]
    fn test_parse_list_detailed() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"list","params":{"resource":"displays","detailed":true}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::List { resource, detailed } => {
                assert_eq!(resource, ListResource::Displays);
                assert!(detailed);
            }
            _ => panic!("expected List command"),
        }
    }

    #[test]
    fn test_parse_get_focused() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"get","params":{"target":"focused"}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Get { target } => {
                assert!(matches!(target, GetTarget::Focused));
            }
            _ => panic!("expected Get command"),
        }
    }

    #[test]
    fn test_parse_get_window() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"get","params":{"target":"window","app":["Safari"]}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Get { target } => match target {
                GetTarget::Window { app } => {
                    assert_eq!(app, vec!["Safari"]);
                }
                _ => panic!("expected Window target"),
            },
            _ => panic!("expected Get command"),
        }
    }

    #[test]
    fn test_parse_ping() {
        let req = JsonRpcRequest::parse(r#"{"method":"ping"}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::Ping));
    }

    #[test]
    fn test_parse_status() {
        let req = JsonRpcRequest::parse(r#"{"method":"status"}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::Status));
    }

    #[test]
    fn test_parse_version() {
        let req = JsonRpcRequest::parse(r#"{"method":"version"}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::Version));
    }

    #[test]
    fn test_parse_daemon_start() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"daemon","params":{"command":"start","foreground":true}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Daemon(DaemonCommand::Start { log, foreground }) => {
                assert!(log.is_none());
                assert!(foreground);
            }
            _ => panic!("expected Daemon Start command"),
        }
    }

    #[test]
    fn test_parse_daemon_stop() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"daemon","params":{"command":"stop"}}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::Daemon(DaemonCommand::Stop)));
    }

    #[test]
    fn test_parse_config_show() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"config","params":{"command":"show"}}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::Config(ConfigCommand::Show)));
    }

    #[test]
    fn test_parse_config_set() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"config","params":{"command":"set","key":"settings.launch","value":"true"}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Config(ConfigCommand::Set { key, value }) => {
                assert_eq!(key, "settings.launch");
                assert_eq!(value, "true");
            }
            _ => panic!("expected Config Set command"),
        }
    }

    #[test]
    fn test_parse_spotlight_install() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"spotlight","params":{"command":"install","force":true}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Spotlight(SpotlightCommand::Install { name, force }) => {
                assert!(name.is_none());
                assert!(force);
            }
            _ => panic!("expected Spotlight Install command"),
        }
    }

    #[test]
    fn test_parse_install() {
        let req = JsonRpcRequest::parse(r#"{"method":"install","params":{"force":true}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Install {
                path,
                force,
                no_sudo,
                completions,
                no_completions,
                completions_only,
            } => {
                assert!(path.is_none());
                assert!(force);
                assert!(!no_sudo);
                assert!(completions.is_none());
                assert!(!no_completions);
                assert!(!completions_only);
            }
            _ => panic!("expected Install command"),
        }
    }

    #[test]
    fn test_parse_install_with_completions() {
        let req = JsonRpcRequest::parse(
            r#"{"method":"install","params":{"completions":"zsh","completions_only":true}}"#,
        )
        .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Install {
                completions,
                completions_only,
                ..
            } => {
                assert_eq!(completions, Some("zsh".to_string()));
                assert!(completions_only);
            }
            _ => panic!("expected Install command"),
        }
    }

    #[test]
    fn test_parse_update() {
        let req = JsonRpcRequest::parse(r#"{"method":"update","params":{"check":true}}"#).unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::Update {
                check,
                force,
                prerelease,
            } => {
                assert!(check);
                assert!(!force);
                assert!(!prerelease);
            }
            _ => panic!("expected Update command"),
        }
    }

    #[test]
    fn test_parse_record_shortcut_rejected() {
        let req = JsonRpcRequest::parse(r#"{"method":"record_shortcut"}"#).unwrap();
        let result = req.to_command();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("interactive"));
    }

    #[test]
    fn test_parse_record_layout_rejected() {
        let req = JsonRpcRequest::parse(r#"{"method":"record_layout"}"#).unwrap();
        let result = req.to_command();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("CLI-only"));
    }

    #[test]
    fn test_parse_unknown_method() {
        let req = JsonRpcRequest::parse(r#"{"method":"unknown_method"}"#).unwrap();
        let result = req.to_command();

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unknown method"));
    }

    #[test]
    fn test_parse_undo() {
        let req = JsonRpcRequest::parse(r#"{"method":"undo"}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::Undo));
    }

    #[test]
    fn test_parse_redo() {
        let req = JsonRpcRequest::parse(r#"{"method":"redo"}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::Redo));
    }

    #[test]
    fn test_parse_history_list() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"history","params":{"command":"list"}}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::History(HistoryCommand::List)));
    }

    #[test]
    fn test_parse_history_list_direct() {
        let req = JsonRpcRequest::parse(r#"{"method":"history_list"}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::History(HistoryCommand::List)));
    }

    #[test]
    fn test_parse_history_clear() {
        let req =
            JsonRpcRequest::parse(r#"{"method":"history","params":{"command":"clear"}}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::History(HistoryCommand::Clear)));
    }

    #[test]
    fn test_parse_history_clear_direct() {
        let req = JsonRpcRequest::parse(r#"{"method":"history_clear"}"#).unwrap();
        let cmd = req.to_command().unwrap();
        assert!(matches!(cmd, Command::History(HistoryCommand::Clear)));
    }
}
