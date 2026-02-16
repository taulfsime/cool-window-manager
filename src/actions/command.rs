//! unified command definitions - single source of truth for all transports
//!
//! this enum defines all commands supported by cwm. CLI, IPC, and HTTP
//! all parse their input into this type, ensuring consistent behavior
//! across all transports.

use std::path::PathBuf;

use crate::display::DisplayTarget;
use crate::window::manager::{MoveTarget, ResizeTarget};

/// all commands supported by cwm
///
/// adding a new field here automatically makes it available in CLI, IPC, and HTTP
/// once the corresponding parser and handler are updated.
#[derive(Debug, Clone)]
pub enum Command {
    // ==================== Window Commands ====================
    /// focus an application window
    Focus {
        /// target app name(s), tries each in order until one is found
        app: Vec<String>,
        /// launch behavior: Some(true) = force, Some(false) = never, None = config default
        launch: Option<bool>,
    },

    /// maximize a window
    Maximize {
        /// target app(s), empty = focused window
        app: Vec<String>,
        /// launch behavior override
        launch: Option<bool>,
    },

    /// resize a window
    Resize {
        /// target app(s), empty = focused window
        app: Vec<String>,
        /// target size (already parsed/validated)
        to: ResizeTarget,
        /// allow window to extend beyond screen bounds
        overflow: bool,
        /// launch behavior override
        launch: Option<bool>,
    },

    /// move a window to a specific position and/or display
    Move {
        /// target app(s), empty = focused window
        app: Vec<String>,
        /// target position (None = keep relative position when switching display, or center)
        to: Option<MoveTarget>,
        /// target display (None = current display)
        display: Option<DisplayTarget>,
        /// launch behavior override
        launch: Option<bool>,
    },

    /// kill (terminate) an application
    Kill {
        /// target app name(s) - required
        app: Vec<String>,
        /// force terminate without save dialogs
        force: bool,
        /// wait for app to terminate before returning
        wait: bool,
    },

    /// close window(s) of an application
    Close {
        /// target app name(s) - required (matches by name or title)
        app: Vec<String>,
    },

    // ==================== Query Commands ====================
    /// list resources
    List {
        /// resource type to list
        resource: ListResource,
        /// include detailed information
        detailed: bool,
    },

    /// get window information
    Get {
        /// what to get info about
        target: GetTarget,
    },

    // ==================== System Commands ====================
    /// health check (IPC/HTTP only)
    Ping,

    /// service status
    Status,

    /// check accessibility permissions
    CheckPermissions {
        /// prompt user to grant permissions if not granted
        prompt: bool,
    },

    /// show version information
    Version,

    /// record commands (shortcuts, layouts)
    /// note: these variants exist for IPC rejection and is_interactive checks
    /// the actual implementation is in CLI handlers
    #[allow(dead_code)]
    Record(RecordCommand),

    // ==================== Daemon Commands ====================
    /// daemon management
    Daemon(DaemonCommand),

    // ==================== Config Commands ====================
    /// configuration management
    Config(ConfigCommand),

    // ==================== Spotlight Commands ====================
    /// macOS Spotlight integration
    Spotlight(SpotlightCommand),

    // ==================== Events Commands ====================
    /// event subscription and waiting
    #[allow(dead_code)]
    Events(EventsCommand),

    // ==================== History Commands ====================
    /// undo the last window action
    Undo,

    /// redo the last undone action
    Redo,

    /// history management
    History(HistoryCommand),

    // ==================== Install Commands ====================
    /// install cwm to system PATH
    Install {
        /// installation directory
        path: Option<PathBuf>,
        /// force overwrite existing installation
        force: bool,
        /// don't use sudo even if needed
        no_sudo: bool,
        /// install shell completions: None = prompt, Some("auto") = detect, Some("zsh") = specific
        completions: Option<String>,
        /// skip shell completion installation
        no_completions: bool,
        /// only install completions (skip binary installation)
        completions_only: bool,
    },

    /// uninstall cwm from system
    Uninstall {
        /// remove from specific path
        path: Option<PathBuf>,
    },

    /// update cwm to latest version
    Update {
        /// only check for updates, don't install
        check: bool,
        /// force update even if on latest version
        force: bool,
        /// include pre-release versions
        prerelease: bool,
    },
}

/// resource types for list command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListResource {
    /// running applications
    Apps,
    /// available displays
    Displays,
    /// display aliases (system and user-defined)
    Aliases,
    /// available event types
    Events,
}

/// target for get command
#[derive(Debug, Clone)]
pub enum GetTarget {
    /// currently focused window
    Focused,
    /// specific app's window
    Window {
        /// app name(s) to get info for
        app: Vec<String>,
    },
}

/// daemon subcommands
#[derive(Debug, Clone)]
pub enum DaemonCommand {
    /// start the daemon
    Start {
        /// log file path
        log: Option<String>,
        /// run in foreground instead of daemonizing
        foreground: bool,
    },
    /// stop the daemon
    Stop,
    /// check daemon status
    Status,
    /// install daemon to run on login
    Install {
        /// path to cwm binary
        bin: Option<String>,
        /// log file path
        log: Option<String>,
    },
    /// uninstall daemon from login items
    Uninstall,
}

/// config subcommands
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    /// show current configuration
    Show,
    /// show configuration file path
    Path,
    /// set a configuration value
    Set {
        /// configuration key
        key: String,
        /// value to set
        value: String,
    },
    /// reset configuration to defaults
    Reset,
    /// show the default configuration with examples
    Default,
    /// verify configuration file for errors
    Verify,
}

/// spotlight subcommands
#[derive(Debug, Clone)]
pub enum SpotlightCommand {
    /// install spotlight shortcuts as macOS apps
    Install {
        /// install only a specific shortcut by name
        name: Option<String>,
        /// force overwrite existing shortcuts
        force: bool,
    },
    /// list installed spotlight shortcuts
    List,
    /// remove installed spotlight shortcuts
    Remove {
        /// remove specific shortcut by name
        name: Option<String>,
        /// remove all cwm spotlight shortcuts
        all: bool,
    },
    /// show example spotlight configuration
    Example,
}

/// events subcommands
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum EventsCommand {
    /// listen for events and stream to stdout
    Listen {
        /// event patterns to filter (empty = all events)
        event: Vec<String>,
        /// app name/title filters (empty = all apps)
        app: Vec<String>,
        /// custom output format
        format: Option<String>,
    },
    /// wait for specific event(s) then exit
    Wait {
        /// event type(s) to wait for (empty = any event)
        event: Vec<String>,
        /// app name/title filters (empty = all apps)
        app: Vec<String>,
        /// timeout in seconds (None = wait forever)
        timeout: Option<u64>,
    },
}

/// history subcommands
#[derive(Debug, Clone)]
pub enum HistoryCommand {
    /// list history entries
    List,
    /// clear all history
    Clear,
}

/// record subcommands
/// note: these variants exist for IPC rejection and is_interactive checks
/// the actual implementation is in CLI handlers
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RecordCommand {
    /// record a keyboard shortcut (interactive - CLI only)
    Shortcut {
        /// action to bind
        action: Option<String>,
        /// target app name
        app: Option<String>,
        /// launch behavior
        launch: Option<bool>,
        /// skip confirmation prompt
        yes: bool,
    },
    /// record current window layout
    Layout {
        /// target app name(s) to record (empty = all visible windows)
        app: Vec<String>,
        /// only record windows on this display
        display: Option<String>,
    },
}

impl Command {
    /// check if this command requires interactive input (CLI only)
    pub fn is_interactive(&self) -> bool {
        match self {
            // record shortcut without --yes requires user input
            Command::Record(RecordCommand::Shortcut { yes: false, .. }) => true,
            // record layout is not interactive (just reads window state)
            Command::Record(RecordCommand::Layout { .. }) => false,
            // install without path requires interactive selection (unless completions_only)
            Command::Install {
                path: None,
                completions_only: false,
                ..
            } => true,
            // events listen/wait are blocking but not interactive
            Command::Events(_) => false,
            _ => false,
        }
    }

    /// get the method name for this command (used in JSON-RPC)
    pub fn method_name(&self) -> &'static str {
        match self {
            Command::Focus { .. } => "focus",
            Command::Maximize { .. } => "maximize",
            Command::Resize { .. } => "resize",
            Command::Move { .. } => "move",
            Command::Kill { .. } => "kill",
            Command::Close { .. } => "close",
            Command::List { .. } => "list",
            Command::Get { .. } => "get",
            Command::Ping => "ping",
            Command::Status => "status",
            Command::CheckPermissions { .. } => "check_permissions",
            Command::Version => "version",
            Command::Record(RecordCommand::Shortcut { .. }) => "record_shortcut",
            Command::Record(RecordCommand::Layout { .. }) => "record_layout",
            Command::Daemon(_) => "daemon",
            Command::Config(_) => "config",
            Command::Spotlight(_) => "spotlight",
            Command::Events(EventsCommand::Listen { .. }) => "events_listen",
            Command::Events(EventsCommand::Wait { .. }) => "events_wait",
            Command::Undo => "undo",
            Command::Redo => "redo",
            Command::History(HistoryCommand::List) => "history_list",
            Command::History(HistoryCommand::Clear) => "history_clear",
            Command::Install { .. } => "install",
            Command::Uninstall { .. } => "uninstall",
            Command::Update { .. } => "update",
        }
    }
}

impl std::fmt::Display for ListResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListResource::Apps => write!(f, "apps"),
            ListResource::Displays => write!(f, "displays"),
            ListResource::Aliases => write!(f, "aliases"),
            ListResource::Events => write!(f, "events"),
        }
    }
}

impl std::str::FromStr for ListResource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "apps" => Ok(ListResource::Apps),
            "displays" => Ok(ListResource::Displays),
            "aliases" => Ok(ListResource::Aliases),
            "events" => Ok(ListResource::Events),
            _ => Err(format!(
                "invalid resource '{}', expected: apps, displays, aliases, events",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_is_interactive() {
        // record shortcut without yes is interactive
        assert!(Command::Record(RecordCommand::Shortcut {
            action: None,
            app: None,
            launch: None,
            yes: false,
        })
        .is_interactive());

        // record shortcut with yes is not interactive
        assert!(!Command::Record(RecordCommand::Shortcut {
            action: None,
            app: None,
            launch: None,
            yes: true,
        })
        .is_interactive());

        // record layout is not interactive
        assert!(!Command::Record(RecordCommand::Layout {
            app: vec![],
            display: None,
        })
        .is_interactive());

        // install without path is interactive (unless completions_only)
        assert!(Command::Install {
            path: None,
            force: false,
            no_sudo: false,
            completions: None,
            no_completions: false,
            completions_only: false,
        }
        .is_interactive());

        // install with path is not interactive
        assert!(!Command::Install {
            path: Some(PathBuf::from("/usr/local/bin")),
            force: false,
            no_sudo: false,
            completions: None,
            no_completions: false,
            completions_only: false,
        }
        .is_interactive());

        // completions_only is not interactive (doesn't need path selection)
        assert!(!Command::Install {
            path: None,
            force: false,
            no_sudo: false,
            completions: Some("zsh".to_string()),
            no_completions: false,
            completions_only: true,
        }
        .is_interactive());

        // other commands are not interactive
        assert!(!Command::Focus {
            app: vec!["Safari".to_string()],
            launch: None,
        }
        .is_interactive());
        assert!(!Command::Ping.is_interactive());

        // kill and close are not interactive
        assert!(!Command::Kill {
            app: vec!["Safari".to_string()],
            force: false,
            wait: false,
        }
        .is_interactive());
        assert!(!Command::Close {
            app: vec!["Safari".to_string()],
        }
        .is_interactive());
    }

    #[test]
    fn test_list_resource_from_str() {
        assert_eq!("apps".parse::<ListResource>().unwrap(), ListResource::Apps);
        assert_eq!(
            "displays".parse::<ListResource>().unwrap(),
            ListResource::Displays
        );
        assert_eq!(
            "aliases".parse::<ListResource>().unwrap(),
            ListResource::Aliases
        );
        assert_eq!(
            "events".parse::<ListResource>().unwrap(),
            ListResource::Events
        );

        // case insensitive
        assert_eq!("APPS".parse::<ListResource>().unwrap(), ListResource::Apps);
        assert_eq!(
            "Displays".parse::<ListResource>().unwrap(),
            ListResource::Displays
        );
        assert_eq!(
            "EVENTS".parse::<ListResource>().unwrap(),
            ListResource::Events
        );

        // invalid
        assert!("invalid".parse::<ListResource>().is_err());
    }

    #[test]
    fn test_command_method_name() {
        assert_eq!(
            Command::Focus {
                app: vec![],
                launch: None
            }
            .method_name(),
            "focus"
        );
        assert_eq!(Command::Ping.method_name(), "ping");
        assert_eq!(
            Command::Daemon(DaemonCommand::Status).method_name(),
            "daemon"
        );
        assert_eq!(
            Command::Kill {
                app: vec!["Safari".to_string()],
                force: false,
                wait: false,
            }
            .method_name(),
            "kill"
        );
        assert_eq!(
            Command::Close {
                app: vec!["Safari".to_string()],
            }
            .method_name(),
            "close"
        );
    }

    #[test]
    fn test_undo_redo_method_names() {
        assert_eq!(Command::Undo.method_name(), "undo");
        assert_eq!(Command::Redo.method_name(), "redo");
        assert_eq!(
            Command::History(HistoryCommand::List).method_name(),
            "history_list"
        );
        assert_eq!(
            Command::History(HistoryCommand::Clear).method_name(),
            "history_clear"
        );
    }

    #[test]
    fn test_undo_redo_not_interactive() {
        assert!(!Command::Undo.is_interactive());
        assert!(!Command::Redo.is_interactive());
        assert!(!Command::History(HistoryCommand::List).is_interactive());
        assert!(!Command::History(HistoryCommand::Clear).is_interactive());
    }
}
