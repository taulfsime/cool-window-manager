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

    /// record a keyboard shortcut (interactive - CLI only)
    /// note: this variant exists for IPC rejection, CLI handles it directly
    #[allow(dead_code)]
    RecordShortcut {
        /// action to bind
        action: Option<String>,
        /// target app name
        app: Option<String>,
        /// launch behavior
        launch: Option<bool>,
        /// skip confirmation prompt
        yes: bool,
    },

    // ==================== Daemon Commands ====================
    /// daemon management
    Daemon(DaemonCommand),

    // ==================== Config Commands ====================
    /// configuration management
    Config(ConfigCommand),

    // ==================== Spotlight Commands ====================
    /// macOS Spotlight integration
    Spotlight(SpotlightCommand),

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

impl Command {
    /// check if this command requires interactive input (CLI only)
    pub fn is_interactive(&self) -> bool {
        match self {
            // record-shortcut without --yes requires user input
            Command::RecordShortcut { yes: false, .. } => true,
            // install without path requires interactive selection (unless completions_only)
            Command::Install {
                path: None,
                completions_only: false,
                ..
            } => true,
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
            Command::List { .. } => "list",
            Command::Get { .. } => "get",
            Command::Ping => "ping",
            Command::Status => "status",
            Command::CheckPermissions { .. } => "check_permissions",
            Command::Version => "version",
            Command::RecordShortcut { .. } => "record_shortcut",
            Command::Daemon(_) => "daemon",
            Command::Config(_) => "config",
            Command::Spotlight(_) => "spotlight",
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
            _ => Err(format!(
                "invalid resource '{}', expected: apps, displays, aliases",
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
        // record_shortcut without yes is interactive
        assert!(Command::RecordShortcut {
            action: None,
            app: None,
            launch: None,
            yes: false,
        }
        .is_interactive());

        // record_shortcut with yes is not interactive
        assert!(!Command::RecordShortcut {
            action: None,
            app: None,
            launch: None,
            yes: true,
        }
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

        // case insensitive
        assert_eq!("APPS".parse::<ListResource>().unwrap(), ListResource::Apps);
        assert_eq!(
            "Displays".parse::<ListResource>().unwrap(),
            ListResource::Displays
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
    }
}
