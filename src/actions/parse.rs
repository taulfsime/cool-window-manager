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
use crate::window::manager::ResizeTarget;

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

            "move_display" => {
                let target_str = params.get_string("target")?;
                let target = DisplayTarget::parse(&target_str)
                    .map_err(|e| ActionError::invalid_args(e.to_string()))?;
                Ok(Command::MoveDisplay {
                    app: params.get_string_array_or_empty("app"),
                    target,
                    launch: params.get_optional_bool("launch")?,
                })
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

            // ==================== Interactive Commands ====================
            "record_shortcut" => Err(ActionError::not_supported(
                "record_shortcut is interactive and not available via IPC",
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
    fn test_parse_move_display() {
        let req = JsonRpcRequest::parse(r#"{"method":"move_display","params":{"target":"next"}}"#)
            .unwrap();
        let cmd = req.to_command().unwrap();

        match cmd {
            Command::MoveDisplay { app, target, .. } => {
                assert!(app.is_empty());
                assert!(matches!(target, DisplayTarget::Next));
            }
            _ => panic!("expected MoveDisplay command"),
        }
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
    fn test_parse_unknown_method() {
        let req = JsonRpcRequest::parse(r#"{"method":"unknown_method"}"#).unwrap();
        let result = req.to_command();

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unknown method"));
    }
}
