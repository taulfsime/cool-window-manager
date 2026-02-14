//! conversion helpers for CLI commands

use crate::actions::{Command, ConfigCommand, DaemonCommand, SpotlightCommand};

use super::commands::{ConfigCommands, DaemonCommands, SpotlightCommands};

/// convert launch/no_launch flags to Option<bool>
pub fn resolve_launch_flags(launch: bool, no_launch: bool) -> Option<bool> {
    if launch {
        Some(true)
    } else if no_launch {
        Some(false)
    } else {
        None
    }
}

impl DaemonCommands {
    /// convert CLI daemon command to unified Command enum
    pub fn to_command(&self) -> Command {
        match self {
            DaemonCommands::Start { log, foreground } => Command::Daemon(DaemonCommand::Start {
                log: log.clone(),
                foreground: *foreground,
            }),
            DaemonCommands::Stop => Command::Daemon(DaemonCommand::Stop),
            DaemonCommands::Status => Command::Daemon(DaemonCommand::Status),
            DaemonCommands::Install { bin, log } => Command::Daemon(DaemonCommand::Install {
                bin: bin.clone(),
                log: log.clone(),
            }),
            DaemonCommands::Uninstall => Command::Daemon(DaemonCommand::Uninstall),
            DaemonCommands::RunForeground { log } => Command::Daemon(DaemonCommand::Start {
                log: log.clone(),
                foreground: true,
            }),
        }
    }
}

impl ConfigCommands {
    /// convert CLI config command to unified Command enum
    pub fn to_command(&self) -> Command {
        match self {
            ConfigCommands::Show => Command::Config(ConfigCommand::Show),
            ConfigCommands::Path => Command::Config(ConfigCommand::Path),
            ConfigCommands::Set { key, value } => Command::Config(ConfigCommand::Set {
                key: key.clone(),
                value: value.clone(),
            }),
            ConfigCommands::Reset => Command::Config(ConfigCommand::Reset),
            ConfigCommands::Default => Command::Config(ConfigCommand::Default),
            ConfigCommands::Verify => Command::Config(ConfigCommand::Verify),
        }
    }
}

impl SpotlightCommands {
    /// convert CLI spotlight command to unified Command enum
    pub fn to_command(&self) -> Command {
        match self {
            SpotlightCommands::Install { name, force } => {
                Command::Spotlight(SpotlightCommand::Install {
                    name: name.clone(),
                    force: *force,
                })
            }
            SpotlightCommands::List => Command::Spotlight(SpotlightCommand::List),
            SpotlightCommands::Remove { name, all } => {
                Command::Spotlight(SpotlightCommand::Remove {
                    name: name.clone(),
                    all: *all,
                })
            }
            SpotlightCommands::Example => Command::Spotlight(SpotlightCommand::Example),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_launch_flags() {
        assert_eq!(resolve_launch_flags(true, false), Some(true));
        assert_eq!(resolve_launch_flags(false, true), Some(false));
        assert_eq!(resolve_launch_flags(false, false), None);
    }

    #[test]
    fn test_daemon_commands_to_command() {
        let cmd = DaemonCommands::Start {
            log: Some("/tmp/log".to_string()),
            foreground: true,
        };
        match cmd.to_command() {
            Command::Daemon(DaemonCommand::Start { log, foreground }) => {
                assert_eq!(log, Some("/tmp/log".to_string()));
                assert!(foreground);
            }
            _ => panic!("unexpected command type"),
        }

        let cmd = DaemonCommands::Stop;
        assert!(matches!(
            cmd.to_command(),
            Command::Daemon(DaemonCommand::Stop)
        ));

        let cmd = DaemonCommands::Status;
        assert!(matches!(
            cmd.to_command(),
            Command::Daemon(DaemonCommand::Status)
        ));
    }

    #[test]
    fn test_config_commands_to_command() {
        let cmd = ConfigCommands::Show;
        assert!(matches!(
            cmd.to_command(),
            Command::Config(ConfigCommand::Show)
        ));

        let cmd = ConfigCommands::Set {
            key: "settings.launch".to_string(),
            value: "true".to_string(),
        };
        match cmd.to_command() {
            Command::Config(ConfigCommand::Set { key, value }) => {
                assert_eq!(key, "settings.launch");
                assert_eq!(value, "true");
            }
            _ => panic!("unexpected command type"),
        }
    }

    #[test]
    fn test_spotlight_commands_to_command() {
        let cmd = SpotlightCommands::List;
        assert!(matches!(
            cmd.to_command(),
            Command::Spotlight(SpotlightCommand::List)
        ));

        let cmd = SpotlightCommands::Install {
            name: Some("Focus Safari".to_string()),
            force: true,
        };
        match cmd.to_command() {
            Command::Spotlight(SpotlightCommand::Install { name, force }) => {
                assert_eq!(name, Some("Focus Safari".to_string()));
                assert!(force);
            }
            _ => panic!("unexpected command type"),
        }
    }
}
