use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use std::path::PathBuf;

use crate::actions::{self, Command, ExecutionContext};
use crate::config::{self, Shortcut};
use crate::daemon::hotkeys;
use crate::display;

use super::convert::resolve_launch_flags;
use super::output::{self, OutputMode};

#[derive(Parser)]
#[command(name = "cwm")]
#[command(about = "A macOS window manager with CLI and global hotkeys")]
#[command(version = env!("SEMVER"))]
pub struct Cli {
    /// Path to config file (overrides CWM_CONFIG env var and default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Output in JSON format (auto-enabled when stdout is piped)
    #[arg(short, long, global = true)]
    pub json: bool,

    /// Force text output even when stdout is piped
    #[arg(long, global = true, conflicts_with = "json")]
    pub no_json: bool,

    /// Suppress all output on success (errors still go to stderr)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Focus an application window
    Focus {
        /// Target app name(s) (fuzzy matched), tries each in order until one is found
        #[arg(short, long, required = true, action = clap::ArgAction::Append)]
        app: Vec<String>,

        /// Force launch app if not running (launches first app in list)
        #[arg(long, conflicts_with = "no_launch")]
        launch: bool,

        /// Never launch app even if configured to
        #[arg(long, conflicts_with = "launch")]
        no_launch: bool,

        /// Show verbose output including match details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Maximize a window
    Maximize {
        /// Target app name (fuzzy matched), uses focused window if not specified
        #[arg(short, long)]
        app: Option<String>,

        /// Force launch app if not running
        #[arg(long, conflicts_with = "no_launch")]
        launch: bool,

        /// Never launch app even if configured to
        #[arg(long, conflicts_with = "launch")]
        no_launch: bool,

        /// Show verbose output including match details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Move a window to a specific position and/or display
    Move {
        /// Target position: anchor (top-left, right), absolute (100,200px), percent (50%,25%), or relative (+100,-50)
        #[arg(short = 't', long = "to")]
        to: Option<String>,

        /// Target display: "next", "prev", display index (0-based), or alias name
        #[arg(short = 'd', long)]
        display: Option<String>,

        /// Target app name (fuzzy matched), uses focused window if not specified
        #[arg(short, long)]
        app: Option<String>,

        /// Force launch app if not running
        #[arg(long, conflicts_with = "no_launch")]
        launch: bool,

        /// Never launch app even if configured to
        #[arg(long, conflicts_with = "launch")]
        no_launch: bool,

        /// Show verbose output including match details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Resize a window to a target size (centered)
    Resize {
        /// Target size: percentage (80, 80%, 0.8), "full", pixels (1920px, 1920x1080px), or points (800pt, 800x600pt)
        #[arg(short = 't', long = "to")]
        to: String,

        /// Target app name (fuzzy matched), uses focused window if not specified
        #[arg(short, long)]
        app: Option<String>,

        /// Allow window to extend beyond screen bounds
        #[arg(long)]
        overflow: bool,

        /// Force launch app if not running
        #[arg(long, conflicts_with = "no_launch")]
        launch: bool,

        /// Never launch app even if configured to
        #[arg(long, conflicts_with = "launch")]
        no_launch: bool,

        /// Show verbose output including match details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Kill (terminate) an application
    Kill {
        /// Target app name(s) (fuzzy matched), tries each in order until one is found
        #[arg(short, long, required = true, action = clap::ArgAction::Append)]
        app: Vec<String>,

        /// Force terminate without save dialogs (like Force Quit)
        #[arg(long, short)]
        force: bool,

        /// Wait for app to terminate before returning
        #[arg(long, short)]
        wait: bool,

        /// Show verbose output including match details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Close an application's window(s) (app stays running)
    Close {
        /// Target app name(s) (fuzzy matched), tries each in order until one is found
        #[arg(short, long, required = true, action = clap::ArgAction::Append)]
        app: Vec<String>,

        /// Show verbose output including match details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Record keyboard shortcuts or window layouts
    Record {
        #[command(subcommand)]
        command: RecordCommands,
    },

    /// Daemon management
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// List resources (apps, displays, aliases)
    List {
        /// Resource type to list (shows available resources if omitted)
        #[arg(value_enum)]
        resource: Option<ListResource>,

        /// Output in JSON format (overrides global --json for this command)
        #[arg(long)]
        json: bool,

        /// Output one name per line (ideal for piping to fzf/xargs)
        #[arg(long, conflicts_with = "json")]
        names: bool,

        /// Custom output format using {field} placeholders (e.g., "{name} ({pid})")
        #[arg(long, conflicts_with_all = ["json", "names"])]
        format: Option<String>,

        /// Include additional fields in output
        #[arg(short, long)]
        detailed: bool,
    },

    /// Get information about windows
    Get {
        #[command(subcommand)]
        command: GetCommands,
    },

    /// Check accessibility permissions
    CheckPermissions {
        /// Prompt to grant permissions if not already granted
        #[arg(long)]
        prompt: bool,
    },

    /// Display version information
    Version,

    /// Install cwm to system PATH
    Install {
        /// Installation directory
        #[arg(long)]
        path: Option<PathBuf>,

        /// Force overwrite existing installation
        #[arg(long)]
        force: bool,

        /// Don't use sudo even if needed
        #[arg(long)]
        no_sudo: bool,

        /// Install shell completions (auto-detect shell, or specify: bash, zsh, fish, all)
        #[arg(long, value_name = "SHELL", num_args = 0..=1, default_missing_value = "auto")]
        completions: Option<String>,

        /// Skip shell completion installation (don't prompt)
        #[arg(long, conflicts_with = "completions")]
        no_completions: bool,

        /// Only install completions (skip binary installation)
        #[arg(long, requires = "completions")]
        completions_only: bool,
    },

    /// Uninstall cwm from system
    Uninstall {
        /// Remove from specific path
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Update cwm to latest version
    Update {
        /// Only check for updates, don't install
        #[arg(long)]
        check: bool,

        /// Force update even if on latest version
        #[arg(long)]
        force: bool,

        /// Include pre-release versions
        #[arg(long)]
        prerelease: bool,
    },

    /// Manage macOS Spotlight integration
    Spotlight {
        #[command(subcommand)]
        command: SpotlightCommands,
    },

    /// Subscribe to window manager events
    Events {
        #[command(subcommand)]
        command: EventsCommands,
    },

    /// Undo the last window action (requires daemon)
    Undo,

    /// Redo the last undone action (requires daemon)
    Redo,

    /// Manage undo/redo history
    History {
        #[command(subcommand)]
        command: HistoryCommands,
    },
}

#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon in the background
    Start {
        /// Log file path (default: no logging when in background)
        #[arg(long)]
        log: Option<String>,

        /// Run in foreground instead of daemonizing
        #[arg(long, short)]
        foreground: bool,
    },
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
    /// Install daemon to run on login
    Install {
        /// Path to cwm binary (defaults to current executable)
        #[arg(long)]
        bin: Option<String>,

        /// Log file path for the daemon
        #[arg(long)]
        log: Option<String>,
    },
    /// Uninstall daemon from login items
    Uninstall,
    /// Run the daemon in the foreground (used internally)
    #[command(hide = true)]
    RunForeground {
        /// Log file path
        #[arg(long)]
        log: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
    /// Set a configuration value
    Set {
        /// Configuration key (e.g., "behavior.launch_if_not_running")
        key: String,
        /// Value to set
        value: String,
    },
    /// Reset configuration to defaults
    Reset,
    /// Show the default configuration with example shortcuts and rules
    Default,
    /// Verify configuration file for errors
    Verify,
}

#[derive(Subcommand)]
pub enum SpotlightCommands {
    /// Install spotlight shortcuts as macOS apps
    Install {
        /// Install only a specific shortcut by name
        #[arg(long)]
        name: Option<String>,

        /// Force overwrite existing shortcuts
        #[arg(long, short)]
        force: bool,
    },
    /// List installed spotlight shortcuts
    List,
    /// Remove installed spotlight shortcuts
    Remove {
        /// Remove specific shortcut by name (without "cwm: " prefix)
        name: Option<String>,

        /// Remove all cwm spotlight shortcuts
        #[arg(long)]
        all: bool,
    },
    /// Show example spotlight configuration
    Example,
}

#[derive(Subcommand)]
pub enum EventsCommands {
    /// Listen for events and stream to stdout
    Listen {
        /// Event patterns to filter (e.g., "app.*", "window.resized")
        #[arg(short, long, action = clap::ArgAction::Append)]
        event: Vec<String>,

        /// Filter by app name/title (supports regex /pattern/)
        #[arg(short, long, action = clap::ArgAction::Append)]
        app: Vec<String>,

        /// Custom output format using {field} placeholders
        #[arg(long)]
        format: Option<String>,
    },

    /// Wait for specific event(s) then exit
    Wait {
        /// Event type(s) to wait for (e.g., "app.launched", "app.focused")
        #[arg(short, long, action = clap::ArgAction::Append)]
        event: Vec<String>,

        /// Filter by app name/title (supports regex /pattern/)
        #[arg(short, long, action = clap::ArgAction::Append)]
        app: Vec<String>,

        /// Timeout in seconds (exit code 1 on timeout)
        #[arg(short, long)]
        timeout: Option<u64>,
    },
}

#[derive(Subcommand)]
pub enum HistoryCommands {
    /// List history entries
    List,
    /// Clear all history
    Clear,
}

#[derive(Subcommand)]
pub enum GetCommands {
    /// Get info about the currently focused window
    Focused {
        /// Custom output format using {field} placeholders
        #[arg(long)]
        format: Option<String>,
    },

    /// Get info about a specific app's window
    Window {
        /// Target app name(s) (fuzzy/regex matched), tries each in order
        #[arg(short, long, required = true, action = clap::ArgAction::Append)]
        app: Vec<String>,

        /// Custom output format using {field} placeholders
        #[arg(long)]
        format: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum RecordCommands {
    /// Record a keyboard shortcut
    Shortcut {
        /// Action to bind (focus, maximize, move_display:next, etc.)
        #[arg(long)]
        action: Option<String>,

        /// Target app name
        #[arg(long)]
        app: Option<String>,

        /// Set launch_if_not_running to true
        #[arg(long, conflicts_with = "no_launch")]
        launch: bool,

        /// Set launch_if_not_running to false
        #[arg(long, conflicts_with = "launch")]
        no_launch: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Record current window layout for selected apps
    Layout {
        /// Target app name(s) to record (can be specified multiple times)
        #[arg(short, long, action = clap::ArgAction::Append)]
        app: Vec<String>,

        /// Only record windows on this display (index, alias, or unique ID)
        #[arg(short = 'd', long)]
        display: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ListResource {
    /// Running applications
    Apps,
    /// Available displays
    Displays,
    /// Display aliases (system and user-defined)
    Aliases,
    /// Available event types
    Events,
}

// JSON output structs for list command (used in tests to verify serialization format)

#[cfg(test)]
use serde::Serialize;

#[cfg(test)]
#[derive(Serialize)]
struct ListResponse<T: Serialize> {
    items: Vec<T>,
}

#[cfg(test)]
#[derive(Serialize)]
struct AppSummary {
    name: String,
    pid: i32,
}

#[cfg(test)]
#[derive(Serialize)]
struct AppDetailed {
    name: String,
    pid: i32,
    bundle_id: Option<String>,
    titles: Vec<String>,
}

#[cfg(test)]
#[derive(Serialize)]
struct DisplaySummary {
    index: usize,
    name: String,
    width: u32,
    height: u32,
    is_main: bool,
}

#[cfg(test)]
#[derive(Serialize)]
struct DisplayDetailed {
    index: usize,
    name: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    is_main: bool,
    is_builtin: bool,
    display_id: u32,
    vendor_id: Option<u32>,
    model_id: Option<u32>,
    serial_number: Option<u32>,
    unit_number: u32,
    unique_id: String,
}

#[cfg(test)]
#[derive(Serialize)]
struct AliasSummary {
    name: String,
    #[serde(rename = "type")]
    alias_type: String,
    resolved: bool,
    display_index: Option<usize>,
}

#[cfg(test)]
#[derive(Serialize)]
struct AliasDetailed {
    name: String,
    #[serde(rename = "type")]
    alias_type: String,
    resolved: bool,
    display_index: Option<usize>,
    display_name: Option<String>,
    display_unique_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mapped_ids: Option<Vec<String>>,
}

/// resolve app name, reading from stdin if "-" is passed
fn resolve_app_name(app: &str) -> Result<String> {
    if app == "-" {
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("failed to read app name from stdin")?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("empty app name from stdin"));
        }
        Ok(trimmed.to_string())
    } else {
        Ok(app.to_string())
    }
}

/// resolve multiple app names, reading from stdin if any is "-"
fn resolve_app_names(apps: &[String]) -> Result<Vec<String>> {
    apps.iter().map(|a| resolve_app_name(a)).collect()
}

pub fn execute(cli: Cli) -> Result<()> {
    let config_path = cli.config.as_deref();
    let output_mode = OutputMode::from_flags(cli.json, cli.no_json, cli.quiet, false, false);

    match cli.command {
        Commands::Focus {
            app: apps,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load_with_override(config_path)?;
            let apps = resolve_app_names(&apps)?;

            let cmd = Command::Focus {
                app: apps,
                launch: resolve_launch_flags(launch, no_launch),
            };
            let ctx = ExecutionContext::new_with_verbose(&config, true, verbose);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    // silent on success in text/quiet mode (Unix convention)
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error_with_suggestions(
                            err.code,
                            &err.message,
                            err.suggestions.clone(),
                        );
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Maximize {
            app,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load_with_override(config_path)?;
            let app = app.map(|a| resolve_app_name(&a)).transpose()?;

            let cmd = Command::Maximize {
                app: app.map(|a| vec![a]).unwrap_or_default(),
                launch: resolve_launch_flags(launch, no_launch),
            };
            let ctx = ExecutionContext::new_with_verbose(&config, true, verbose);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Move {
            to,
            display,
            app,
            launch,
            no_launch,
            verbose,
        } => {
            use crate::window::manager::MoveTarget;

            // at least one of --to or --display must be specified
            if to.is_none() && display.is_none() {
                return Err(anyhow!(
                    "at least one of --to or --display must be specified"
                ));
            }

            let config = config::load_with_override(config_path)?;
            let app = app.map(|a| resolve_app_name(&a)).transpose()?;

            let move_target = to.map(|t| MoveTarget::parse(&t)).transpose()?;
            let display_target = display
                .map(|d| display::DisplayTarget::parse(&d))
                .transpose()?;

            let cmd = Command::Move {
                app: app.map(|a| vec![a]).unwrap_or_default(),
                to: move_target,
                display: display_target,
                launch: resolve_launch_flags(launch, no_launch),
            };
            let ctx = ExecutionContext::new_with_verbose(&config, true, verbose);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Resize {
            to,
            app,
            overflow,
            launch,
            no_launch,
            verbose,
        } => {
            use crate::window::ResizeTarget;

            let config = config::load_with_override(config_path)?;
            let app = app.map(|a| resolve_app_name(&a)).transpose()?;
            let resize_target = ResizeTarget::parse(&to)?;

            let cmd = Command::Resize {
                app: app.map(|a| vec![a]).unwrap_or_default(),
                to: resize_target,
                overflow,
                launch: resolve_launch_flags(launch, no_launch),
            };
            let ctx = ExecutionContext::new_with_verbose(&config, true, verbose);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Kill {
            app: apps,
            force,
            wait,
            verbose,
        } => {
            let config = config::load_with_override(config_path)?;
            let apps = resolve_app_names(&apps)?;

            let cmd = Command::Kill {
                app: apps,
                force,
                wait,
            };
            let ctx = ExecutionContext::new_with_verbose(&config, true, verbose);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Close { app: apps, verbose } => {
            let config = config::load_with_override(config_path)?;
            let apps = resolve_app_names(&apps)?;

            let cmd = Command::Close { app: apps };
            let ctx = ExecutionContext::new_with_verbose(&config, true, verbose);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Record { command } => match command {
            RecordCommands::Shortcut {
                action,
                app,
                launch,
                no_launch,
                yes,
            } => {
                // record the hotkey
                let keys = hotkeys::record_hotkey()?;
                println!("\nDetected: {}", keys);

                // if no action specified, just print the keys and exit
                if action.is_none() {
                    println!("\nTo save this shortcut, run with --action:");
                    println!("  cwm record shortcut --action focus --app \"AppName\"");
                    return Ok(());
                }

                let action = action.unwrap();

                // validate action
                let valid_actions = ["focus", "maximize"];
                let is_valid = valid_actions.contains(&action.as_str())
                    || action.starts_with("move_display:")
                    || action.starts_with("resize:");

                if !is_valid {
                    return Err(anyhow!(
                        "Invalid action: '{}'. Valid actions: focus, maximize, move_display:next, move_display:prev, move_display:N, resize:N, resize:full",
                        action
                    ));
                }

                // focus requires app
                if action == "focus" && app.is_none() {
                    return Err(anyhow!("Action 'focus' requires --app to be specified"));
                }

                // build the shortcut
                let mut shortcut = Shortcut {
                    keys: keys.clone(),
                    action: action.clone(),
                    app: app.clone(),
                    launch: None,
                    when: None,
                };

                if launch {
                    shortcut.launch = Some(true);
                } else if no_launch {
                    shortcut.launch = Some(false);
                }

                // show what will be added
                let json = serde_json::to_string_pretty(&shortcut)
                    .context("Failed to serialize shortcut")?;
                println!("\nShortcut to add:\n{}", json);

                // load config and check for duplicates
                let mut config = config::load_with_override(config_path)?;
                let existing = config
                    .shortcuts
                    .iter()
                    .position(|s| s.keys.to_lowercase() == keys.to_lowercase());

                if let Some(idx) = existing {
                    let existing_shortcut = &config.shortcuts[idx];
                    println!(
                        "\nWarning: '{}' is already bound to '{}'",
                        keys, existing_shortcut.action
                    );

                    if !yes {
                        print!("Overwrite? [y/N]: ");
                        use std::io::{self, Write};
                        io::stdout().flush()?;

                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;

                        if !input.trim().eq_ignore_ascii_case("y") {
                            println!("Cancelled.");
                            return Ok(());
                        }
                    }

                    config.shortcuts[idx] = shortcut;
                } else {
                    config.shortcuts.push(shortcut);
                }

                // save config
                config::save_with_override(&config, config_path)?;
                println!(
                    "\nSaved to {}",
                    config::get_config_path_with_override(config_path)?.display()
                );

                Ok(())
            }

            RecordCommands::Layout { app, display } => {
                use crate::actions::handlers::record;

                let config = config::load_with_override(config_path)?;
                let apps = resolve_app_names(&app)?;

                record::execute_record_layout(&apps, display.as_deref(), &config, output_mode)?;
                Ok(())
            }
        },

        Commands::Daemon { command } => {
            let config = config::load_with_override(config_path)?;
            let cmd = command.to_command();
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        // text output based on action
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        if let Some(res) = value.get("result") {
                            if let Some(status) = res.get("status").and_then(|v| v.as_str()) {
                                match result.action {
                                    "daemon_status" => {
                                        let running = res
                                            .get("running")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                        if running {
                                            let pid = res
                                                .get("pid")
                                                .and_then(|v| v.as_u64())
                                                .unwrap_or(0);
                                            println!("Daemon is running (PID: {})", pid);
                                        } else {
                                            println!("Daemon is not running");
                                        }
                                    }
                                    "daemon_start" => {
                                        if status == "started" {
                                            println!("Daemon started");
                                        }
                                    }
                                    "daemon_stop" => println!("Daemon stopped"),
                                    "daemon_install" => {
                                        println!("Daemon installed to run on login")
                                    }
                                    "daemon_uninstall" => {
                                        println!("Daemon uninstalled from login items")
                                    }
                                    _ => {}
                                }
                            } else if result.action == "daemon_status" {
                                // status doesn't have "status" field
                                let running = res
                                    .get("running")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                if running {
                                    let pid = res.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
                                    println!("Daemon is running (PID: {})", pid);
                                } else {
                                    println!("Daemon is not running");
                                }
                            }
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Config { command } => {
            let config = config::load_with_override(config_path)?;
            let cmd = command.to_command();
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        match result.action {
                            "config_show" | "config_default" => {
                                // pretty print the config JSON
                                if let Some(res) = value.get("result") {
                                    let json = serde_json::to_string_pretty(res)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    println!("{}", json);
                                }
                            }
                            "config_path" => {
                                if let Some(res) = value.get("result") {
                                    if let Some(path) = res.get("path").and_then(|v| v.as_str()) {
                                        println!("{}", path);
                                    }
                                }
                            }
                            "config_set" => {
                                if let Some(res) = value.get("result") {
                                    let key = res.get("key").and_then(|v| v.as_str()).unwrap_or("");
                                    let val =
                                        res.get("value").and_then(|v| v.as_str()).unwrap_or("");
                                    println!("Set {} = {}", key, val);
                                }
                            }
                            "config_reset" => {
                                println!("Configuration reset to defaults");
                            }
                            "config_verify" => {
                                if let Some(res) = value.get("result") {
                                    let valid =
                                        res.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
                                    let path =
                                        res.get("path").and_then(|v| v.as_str()).unwrap_or("");

                                    if valid {
                                        println!("✓ Configuration is valid: {}", path);
                                    } else {
                                        let errors = res
                                            .get("errors")
                                            .and_then(|v| v.as_array())
                                            .map(|arr| arr.len())
                                            .unwrap_or(0);
                                        println!(
                                            "✗ Configuration has {} error(s): {}",
                                            errors, path
                                        );
                                        println!();
                                        if let Some(errs) =
                                            res.get("errors").and_then(|v| v.as_array())
                                        {
                                            for err in errs {
                                                if let Some(e) = err.as_str() {
                                                    println!("  - {}", e);
                                                }
                                            }
                                        }
                                        return Err(anyhow!("configuration validation failed"));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::List {
            resource,
            json,
            names,
            format,
            detailed,
        } => {
            use crate::actions::ListResource as ActionListResource;

            // if no resource specified, show available resources and exit with error
            let Some(resource) = resource else {
                eprintln!("error: missing required argument <RESOURCE>");
                eprintln!();
                eprintln!("Available resources:");
                eprintln!("  apps      Running applications");
                eprintln!("  displays  Available displays");
                eprintln!("  aliases   Display aliases (system and user-defined)");
                eprintln!("  events    Available event types");
                eprintln!();
                eprintln!("Usage: cwm list <RESOURCE> [OPTIONS]");
                eprintln!();
                eprintln!("Examples:");
                eprintln!("  cwm list apps");
                eprintln!("  cwm list apps --json");
                eprintln!("  cwm list apps --names");
                eprintln!("  cwm list displays --format '{{index}}: {{name}}'");
                eprintln!("  cwm list events --detailed");
                std::process::exit(super::exit_codes::INVALID_ARGS);
            };

            // determine list output mode (list command has its own json flag)
            let list_mode = OutputMode::from_flags(
                json || cli.json,
                cli.no_json,
                cli.quiet,
                names,
                format.is_some(),
            );

            let config = config::load_with_override(config_path)?;
            let action_resource = match resource {
                ListResource::Apps => ActionListResource::Apps,
                ListResource::Displays => ActionListResource::Displays,
                ListResource::Aliases => ActionListResource::Aliases,
                ListResource::Events => ActionListResource::Events,
            };

            let cmd = Command::List {
                resource: action_resource,
                detailed,
            };
            let ctx = ExecutionContext::new_with_verbose(&config, true, false);

            let result = actions::execute(cmd, &ctx).map_err(|e| anyhow!("{}", e.message))?;

            // extract items from result
            let result_value = serde_json::to_value(&result).unwrap_or_default();
            let items = result_value
                .get("items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            // format output based on mode
            match list_mode {
                OutputMode::Names => {
                    for item in &items {
                        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                            println!("{}", name);
                        }
                    }
                }
                OutputMode::Format => {
                    let fmt = format.as_ref().unwrap();
                    for item in &items {
                        println!("{}", output::format_template(fmt, item));
                    }
                }
                OutputMode::Json => {
                    output::print_json(&result);
                }
                OutputMode::Text | OutputMode::Quiet => match resource {
                    ListResource::Apps => {
                        println!("Running applications:");
                        for item in &items {
                            let name = item
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let pid = item.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
                            let bundle = item
                                .get("bundle_id")
                                .and_then(|v| v.as_str())
                                .map(|b| format!(" ({})", b))
                                .unwrap_or_default();
                            println!("  {} [PID: {}]{}", name, pid, bundle);
                            if let Some(titles) = item.get("titles").and_then(|v| v.as_array()) {
                                for title in titles {
                                    if let Some(t) = title.as_str() {
                                        println!("    - {}", t);
                                    }
                                }
                            }
                        }
                        println!("\nTotal: {} applications", items.len());
                    }
                    ListResource::Displays => {
                        if items.is_empty() {
                            println!("No displays found");
                        } else {
                            println!("Available displays:");
                            for item in &items {
                                let index = item.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                                let name = item
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let width = item.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                                let height =
                                    item.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                                let is_main = item
                                    .get("is_main")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                let main_str = if is_main { " (main)" } else { "" };
                                println!(
                                    "  {}: {} ({}x{}){}",
                                    index, name, width, height, main_str
                                );
                            }
                        }
                    }
                    ListResource::Aliases => {
                        println!("Display Aliases:");
                        for item in &items {
                            let name = item
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let alias_type = item
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let resolved = item
                                .get("resolved")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            if resolved {
                                let display_index = item
                                    .get("display_index")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let display_name = item
                                    .get("display_name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                println!(
                                    "  {:<20} ({}) → Display {}: {}",
                                    name, alias_type, display_index, display_name
                                );
                            } else {
                                println!("  {:<20} ({}) → not resolved", name, alias_type);
                            }
                        }
                    }
                    ListResource::Events => {
                        println!("Available events:\n");
                        for item in &items {
                            let name = item
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            if detailed {
                                let desc = item
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                println!("  {:<20} - {}", name, desc);
                            } else {
                                println!("  {}", name);
                            }
                        }
                        println!("\nPatterns:");
                        println!("  *          - All events");
                        println!("  app.*      - All app events");
                        println!("  window.*   - All window events");
                        println!("  daemon.*   - All daemon events");
                    }
                },
            }

            Ok(())
        }

        Commands::Get { command } => {
            use crate::actions::GetTarget;

            let config = config::load_with_override(config_path)?;

            let (cmd, fmt) = match command {
                GetCommands::Focused { format: fmt } => (
                    Command::Get {
                        target: GetTarget::Focused,
                    },
                    fmt,
                ),
                GetCommands::Window { app, format: fmt } => {
                    let apps: Vec<String> = app
                        .iter()
                        .map(|a| resolve_app_name(a))
                        .collect::<Result<Vec<_>>>()?;
                    (
                        Command::Get {
                            target: GetTarget::Window { app: apps },
                        },
                        fmt,
                    )
                }
            };

            let ctx = ExecutionContext::new_with_verbose(&config, true, false);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if let Some(fmt_str) = fmt {
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        println!("{}", output::format_template(&fmt_str, &value));
                    } else if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        // text output - extract data from result
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        if let Some(app) = value.get("app") {
                            let name = app
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let pid = app.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
                            println!("{} (PID: {})", name, pid);
                        }
                        if let Some(window) = value.get("window") {
                            if let Some(title) = window.get("title").and_then(|v| v.as_str()) {
                                println!("  Title: {}", title);
                            }
                            let x = window.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                            let y = window.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                            let w = window.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                            let h = window.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                            println!("  Position: {}, {}", x, y);
                            println!("  Size: {}x{}", w, h);
                        }
                        if let Some(display) = value.get("display") {
                            let index = display.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                            let name = display
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            println!("  Display: {} ({})", index, name);
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::CheckPermissions { prompt } => {
            let config = config::load_with_override(config_path)?;
            let cmd = Command::CheckPermissions { prompt };
            let ctx = ExecutionContext::new_with_verbose(&config, true, false);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        // text output - extract from result
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        if let Some(res) = value.get("result") {
                            let granted = res
                                .get("granted")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            if granted {
                                println!("✓ Accessibility permissions granted");
                            } else {
                                println!("✗ Accessibility permissions not granted");
                                println!("\nPlease grant permissions in System Settings:");
                                println!("  System Settings > Privacy & Security > Accessibility");
                            }
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Version => {
            let config = config::load_with_override(config_path)?;
            let cmd = Command::Version;
            let ctx = ExecutionContext::new_with_verbose(&config, true, false);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        // text output
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        if let Some(res) = value.get("result") {
                            let version_str = res
                                .get("version_string")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let build_date = res
                                .get("build_date")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            println!("cwm {}", version_str);
                            println!("Built: {}", build_date);

                            if let Some(install_path) =
                                res.get("install_path").and_then(|v| v.as_str())
                            {
                                println!("Installed: {}", install_path);
                            }

                            println!("Repository: https://github.com/{}", env!("GITHUB_REPO"));
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Install {
            path,
            force,
            no_sudo,
            completions,
            no_completions,
            completions_only,
        } => {
            let config = config::load_with_override(config_path)?;
            let cmd = Command::Install {
                path,
                force,
                no_sudo,
                completions,
                no_completions,
                completions_only,
            };
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    // text output is handled by the handler (prints during install)
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Uninstall { path } => {
            let config = config::load_with_override(config_path)?;
            let cmd = Command::Uninstall { path };
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    }
                    // text output is handled by the handler
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Update {
            check,
            force,
            prerelease,
        } => {
            let config = config::load_with_override(config_path)?;
            let cmd = Command::Update {
                check,
                force,
                prerelease,
            };
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        // text output based on result
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        if let Some(res) = value.get("result") {
                            let status = res.get("status").and_then(|v| v.as_str()).unwrap_or("");
                            match status {
                                "up_to_date" => {
                                    println!("You are on the latest version");
                                }
                                "updated" => {
                                    // success message already printed by handler
                                }
                                "cancelled" => {
                                    println!("Update cancelled");
                                }
                                _ => {
                                    // check-only mode
                                    if check {
                                        let available = res
                                            .get("update_available")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                        if available {
                                            let new_ver = res
                                                .get("new_version")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("unknown");
                                            println!("\n🆕 New version available: {}", new_ver);
                                            println!("\nRun 'cwm update' to install");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Spotlight { command } => {
            let config = config::load_with_override(config_path)?;
            let cmd = command.to_command();
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        match result.action {
                            "spotlight_install" => {
                                if let Some(res) = value.get("result") {
                                    let status =
                                        res.get("status").and_then(|v| v.as_str()).unwrap_or("");
                                    match status {
                                        "no_shortcuts" => {
                                            println!("No spotlight shortcuts configured.");
                                            println!("\nAdd shortcuts to your config file:");
                                            println!("  cwm spotlight example");
                                            println!("\nOr edit ~/.cwm/config.json directly.");
                                        }
                                        "installed" => {
                                            let apps_dir = res
                                                .get("apps_directory")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            println!(
                                                "Installing spotlight shortcuts to: {}",
                                                apps_dir
                                            );

                                            if let Some(shortcut) =
                                                res.get("shortcut").and_then(|v| v.as_str())
                                            {
                                                let path = res
                                                    .get("path")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("");
                                                println!("✓ Installed: {}", path);
                                                let _ = shortcut; // used for single shortcut
                                            } else if let Some(count) =
                                                res.get("count").and_then(|v| v.as_u64())
                                            {
                                                if count == 0 {
                                                    println!("No shortcuts were installed.");
                                                } else {
                                                    println!(
                                                        "\n✓ Installed {} shortcut(s):",
                                                        count
                                                    );
                                                    if let Some(paths) =
                                                        res.get("paths").and_then(|v| v.as_array())
                                                    {
                                                        for p in paths {
                                                            if let Some(path_str) = p.as_str() {
                                                                if let Some(name) =
                                                                    std::path::Path::new(path_str)
                                                                        .file_name()
                                                                {
                                                                    println!(
                                                                        "  - {}",
                                                                        name.to_string_lossy()
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            println!("\nShortcuts are now available in Spotlight.");
                                            println!("Search for \"cwm: <name>\" to use them.");
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            "spotlight_list" => {
                                if let Some(res) = value.get("result") {
                                    let shortcuts = res
                                        .get("shortcuts")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| arr.len())
                                        .unwrap_or(0);

                                    if shortcuts == 0 {
                                        println!("No spotlight shortcuts installed.");
                                        println!("\nTo install shortcuts:");
                                        println!(
                                            "  1. Add shortcuts to config: cwm spotlight example"
                                        );
                                        println!("  2. Install them: cwm spotlight install");
                                    } else {
                                        println!("Installed spotlight shortcuts:\n");
                                        if let Some(arr) =
                                            res.get("shortcuts").and_then(|v| v.as_array())
                                        {
                                            for name in arr {
                                                if let Some(n) = name.as_str() {
                                                    println!("  cwm: {}", n);
                                                }
                                            }
                                        }
                                        println!("\nTotal: {} shortcut(s)", shortcuts);
                                        if let Some(dir) =
                                            res.get("apps_directory").and_then(|v| v.as_str())
                                        {
                                            println!("Location: {}", dir);
                                        }
                                    }
                                }
                            }
                            "spotlight_remove" => {
                                if let Some(res) = value.get("result") {
                                    if let Some(shortcut) =
                                        res.get("shortcut").and_then(|v| v.as_str())
                                    {
                                        println!("✓ Removed: cwm: {}", shortcut);
                                    } else if let Some(count) =
                                        res.get("count").and_then(|v| v.as_u64())
                                    {
                                        if count == 0 {
                                            println!("No spotlight shortcuts to remove.");
                                        } else {
                                            println!("✓ Removed {} shortcut(s)", count);
                                        }
                                    }
                                }
                            }
                            "spotlight_example" => {
                                if let Some(res) = value.get("result") {
                                    if let Some(examples) = res.get("examples") {
                                        println!("Add this to your config.json:\n");
                                        println!("\"spotlight\": ");
                                        let json = serde_json::to_string_pretty(examples)
                                            .unwrap_or_else(|_| "[]".to_string());
                                        println!("{}", json);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Events { command } => match command {
            EventsCommands::Listen { event, app, format } => {
                super::events::listen(event, app, format, &output_mode)
            }
            EventsCommands::Wait {
                event,
                app,
                timeout,
            } => {
                let exit_code = super::events::wait(event, app, timeout, &output_mode)?;
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
                Ok(())
            }
        },

        Commands::Undo => {
            let config = config::load_with_override(config_path)?;
            let cmd = Command::Undo;
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        if let Some(res) = value.get("result") {
                            if let Some(app) = res
                                .get("app")
                                .and_then(|a| a.get("name"))
                                .and_then(|n| n.as_str())
                            {
                                let x = res
                                    .get("restored")
                                    .and_then(|r| r.get("x"))
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let y = res
                                    .get("restored")
                                    .and_then(|r| r.get("y"))
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let w = res
                                    .get("restored")
                                    .and_then(|r| r.get("width"))
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let h = res
                                    .get("restored")
                                    .and_then(|r| r.get("height"))
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                println!("Restored {} to {}x{} at ({}, {})", app, w, h, x, y);
                            }
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::Redo => {
            let config = config::load_with_override(config_path)?;
            let cmd = Command::Redo;
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        if let Some(res) = value.get("result") {
                            if let Some(app) = res
                                .get("app")
                                .and_then(|a| a.get("name"))
                                .and_then(|n| n.as_str())
                            {
                                let x = res
                                    .get("restored")
                                    .and_then(|r| r.get("x"))
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let y = res
                                    .get("restored")
                                    .and_then(|r| r.get("y"))
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let w = res
                                    .get("restored")
                                    .and_then(|r| r.get("width"))
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let h = res
                                    .get("restored")
                                    .and_then(|r| r.get("height"))
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                println!("Restored {} to {}x{} at ({}, {})", app, w, h, x, y);
                            }
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }

        Commands::History { command } => {
            let config = config::load_with_override(config_path)?;
            let cmd = command.to_command();
            let ctx = ExecutionContext::cli_with_config_path(&config, false, config_path);

            match actions::execute(cmd, &ctx) {
                Ok(result) => {
                    if output_mode.is_json() {
                        output::print_json(&result);
                    } else {
                        let value = serde_json::to_value(&result).unwrap_or_default();
                        match result.action {
                            "history_list" => {
                                if let Some(res) = value.get("result") {
                                    let undo_count = res
                                        .get("undo")
                                        .and_then(|u| u.as_array())
                                        .map(|a| a.len())
                                        .unwrap_or(0);
                                    let redo_count = res
                                        .get("redo")
                                        .and_then(|r| r.as_array())
                                        .map(|a| a.len())
                                        .unwrap_or(0);
                                    println!("Undo stack: {} entries", undo_count);
                                    println!("Redo stack: {} entries", redo_count);
                                }
                            }
                            "history_clear" => {
                                println!("History cleared");
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    if output_mode.is_json() {
                        output::print_json_error(err.code, &err.message);
                        std::process::exit(err.code);
                    } else {
                        Err(anyhow!("{}", err.message))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_resource_value_enum() {
        // verify all variants can be parsed from strings
        assert!(matches!(
            ListResource::from_str("apps", true),
            Ok(ListResource::Apps)
        ));
        assert!(matches!(
            ListResource::from_str("displays", true),
            Ok(ListResource::Displays)
        ));
        assert!(matches!(
            ListResource::from_str("aliases", true),
            Ok(ListResource::Aliases)
        ));

        // case insensitive
        assert!(matches!(
            ListResource::from_str("APPS", true),
            Ok(ListResource::Apps)
        ));
        assert!(matches!(
            ListResource::from_str("Displays", true),
            Ok(ListResource::Displays)
        ));
    }

    #[test]
    fn test_app_summary_serialization() {
        let app = AppSummary {
            name: "Safari".to_string(),
            pid: 1234,
        };

        let json = serde_json::to_string(&app).unwrap();
        assert!(json.contains("\"name\":\"Safari\""));
        assert!(json.contains("\"pid\":1234"));

        // should not contain detailed fields
        assert!(!json.contains("bundle_id"));
        assert!(!json.contains("titles"));
    }

    #[test]
    fn test_app_detailed_serialization() {
        let app = AppDetailed {
            name: "Safari".to_string(),
            pid: 1234,
            bundle_id: Some("com.apple.Safari".to_string()),
            titles: vec!["GitHub".to_string(), "Google".to_string()],
        };

        let json = serde_json::to_string(&app).unwrap();
        assert!(json.contains("\"name\":\"Safari\""));
        assert!(json.contains("\"pid\":1234"));
        assert!(json.contains("\"bundle_id\":\"com.apple.Safari\""));
        assert!(json.contains("\"titles\":[\"GitHub\",\"Google\"]"));
    }

    #[test]
    fn test_app_detailed_serialization_null_bundle_id() {
        let app = AppDetailed {
            name: "Test".to_string(),
            pid: 1,
            bundle_id: None,
            titles: vec![],
        };

        let json = serde_json::to_string(&app).unwrap();
        assert!(json.contains("\"bundle_id\":null"));
        assert!(json.contains("\"titles\":[]"));
    }

    #[test]
    fn test_display_summary_serialization() {
        let display = DisplaySummary {
            index: 0,
            name: "Built-in Display".to_string(),
            width: 2560,
            height: 1600,
            is_main: true,
        };

        let json = serde_json::to_string(&display).unwrap();
        assert!(json.contains("\"index\":0"));
        assert!(json.contains("\"name\":\"Built-in Display\""));
        assert!(json.contains("\"width\":2560"));
        assert!(json.contains("\"height\":1600"));
        assert!(json.contains("\"is_main\":true"));

        // should not contain detailed fields
        assert!(!json.contains("vendor_id"));
        assert!(!json.contains("unique_id"));
    }

    #[test]
    fn test_display_detailed_serialization() {
        let display = DisplayDetailed {
            index: 0,
            name: "Built-in Display".to_string(),
            width: 2560,
            height: 1600,
            x: 0,
            y: 0,
            is_main: true,
            is_builtin: true,
            display_id: 1,
            vendor_id: Some(0x0610),
            model_id: Some(0xA032),
            serial_number: None,
            unit_number: 0,
            unique_id: "0610_A032_unit0".to_string(),
        };

        let json = serde_json::to_string(&display).unwrap();
        assert!(json.contains("\"index\":0"));
        assert!(json.contains("\"x\":0"));
        assert!(json.contains("\"y\":0"));
        assert!(json.contains("\"is_builtin\":true"));
        assert!(json.contains("\"display_id\":1"));
        assert!(json.contains("\"vendor_id\":1552")); // 0x0610 = 1552
        assert!(json.contains("\"model_id\":41010")); // 0xA032 = 41010
        assert!(json.contains("\"serial_number\":null"));
        assert!(json.contains("\"unique_id\":\"0610_A032_unit0\""));
    }

    #[test]
    fn test_alias_summary_serialization() {
        let alias = AliasSummary {
            name: "builtin".to_string(),
            alias_type: "system".to_string(),
            resolved: true,
            display_index: Some(0),
        };

        let json = serde_json::to_string(&alias).unwrap();
        assert!(json.contains("\"name\":\"builtin\""));
        assert!(json.contains("\"type\":\"system\"")); // renamed via serde
        assert!(json.contains("\"resolved\":true"));
        assert!(json.contains("\"display_index\":0"));
    }

    #[test]
    fn test_alias_summary_unresolved() {
        let alias = AliasSummary {
            name: "office".to_string(),
            alias_type: "user".to_string(),
            resolved: false,
            display_index: None,
        };

        let json = serde_json::to_string(&alias).unwrap();
        assert!(json.contains("\"resolved\":false"));
        assert!(json.contains("\"display_index\":null"));
    }

    #[test]
    fn test_alias_detailed_serialization() {
        let alias = AliasDetailed {
            name: "builtin".to_string(),
            alias_type: "system".to_string(),
            resolved: true,
            display_index: Some(0),
            display_name: Some("Built-in Display".to_string()),
            display_unique_id: Some("0610_A032_unit0".to_string()),
            description: Some("Built-in display".to_string()),
            mapped_ids: None,
        };

        let json = serde_json::to_string(&alias).unwrap();
        assert!(json.contains("\"display_name\":\"Built-in Display\""));
        assert!(json.contains("\"display_unique_id\":\"0610_A032_unit0\""));
        assert!(json.contains("\"description\":\"Built-in display\""));
        // mapped_ids should be skipped when None
        assert!(!json.contains("mapped_ids"));
    }

    #[test]
    fn test_alias_detailed_user_with_mapped_ids() {
        let alias = AliasDetailed {
            name: "office".to_string(),
            alias_type: "user".to_string(),
            resolved: true,
            display_index: Some(1),
            display_name: Some("External Monitor".to_string()),
            display_unique_id: Some("1E6D_5B11_12345".to_string()),
            description: None,
            mapped_ids: Some(vec![
                "1E6D_5B11_12345".to_string(),
                "10AC_D0B3_67890".to_string(),
            ]),
        };

        let json = serde_json::to_string(&alias).unwrap();
        assert!(json.contains("\"type\":\"user\""));
        assert!(json.contains("\"mapped_ids\":[\"1E6D_5B11_12345\",\"10AC_D0B3_67890\"]"));
        // description should be skipped when None
        assert!(!json.contains("description"));
    }

    #[test]
    fn test_list_response_serialization() {
        let response = ListResponse {
            items: vec![
                AppSummary {
                    name: "Safari".to_string(),
                    pid: 1234,
                },
                AppSummary {
                    name: "Chrome".to_string(),
                    pid: 5678,
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"items\":["));
        assert!(json.contains("\"name\":\"Safari\""));
        assert!(json.contains("\"name\":\"Chrome\""));

        // verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["items"].is_array());
        assert_eq!(parsed["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_list_response_empty() {
        let response: ListResponse<AppSummary> = ListResponse { items: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(json, "{\"items\":[]}");
    }

    // ========================================================================
    // resolve_app_name tests
    // ========================================================================

    #[test]
    fn test_resolve_app_name_normal() {
        let result = resolve_app_name("Safari").unwrap();
        assert_eq!(result, "Safari");
    }

    #[test]
    fn test_resolve_app_name_with_spaces() {
        let result = resolve_app_name("Visual Studio Code").unwrap();
        assert_eq!(result, "Visual Studio Code");
    }

    #[test]
    fn test_resolve_app_name_empty() {
        let result = resolve_app_name("");
        // empty string is valid (not stdin)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    // ========================================================================
    // CLI parsing tests (using clap)
    // ========================================================================

    #[test]
    fn test_cli_parse_focus_single_app() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "focus", "--app", "Safari"]).unwrap();

        match cli.command {
            Commands::Focus {
                app,
                launch,
                no_launch,
                verbose,
            } => {
                assert_eq!(app, vec!["Safari"]);
                assert!(!launch);
                assert!(!no_launch);
                assert!(!verbose);
            }
            _ => panic!("Expected Focus command"),
        }
    }

    #[test]
    fn test_cli_parse_focus_multiple_apps() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm", "focus", "--app", "Safari", "--app", "Chrome", "--app", "Firefox",
        ])
        .unwrap();

        match cli.command {
            Commands::Focus { app, .. } => {
                assert_eq!(app, vec!["Safari", "Chrome", "Firefox"]);
            }
            _ => panic!("Expected Focus command"),
        }
    }

    #[test]
    fn test_cli_parse_focus_with_launch() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "focus", "--app", "Safari", "--launch"]).unwrap();

        match cli.command {
            Commands::Focus {
                launch, no_launch, ..
            } => {
                assert!(launch);
                assert!(!no_launch);
            }
            _ => panic!("Expected Focus command"),
        }
    }

    #[test]
    fn test_cli_parse_maximize_without_app() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "maximize"]).unwrap();

        match cli.command {
            Commands::Maximize { app, .. } => {
                assert!(app.is_none());
            }
            _ => panic!("Expected Maximize command"),
        }
    }

    #[test]
    fn test_cli_parse_maximize_with_app() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "maximize", "--app", "Safari"]).unwrap();

        match cli.command {
            Commands::Maximize { app, .. } => {
                assert_eq!(app, Some("Safari".to_string()));
            }
            _ => panic!("Expected Maximize command"),
        }
    }

    #[test]
    fn test_cli_parse_resize_percent() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "resize", "--to", "80"]).unwrap();

        match cli.command {
            Commands::Resize {
                to, app, overflow, ..
            } => {
                assert_eq!(to, "80");
                assert!(app.is_none());
                assert!(!overflow);
            }
            _ => panic!("Expected Resize command"),
        }
    }

    #[test]
    fn test_cli_parse_resize_with_overflow() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "resize", "--to", "80", "--overflow"]).unwrap();

        match cli.command {
            Commands::Resize { overflow, .. } => {
                assert!(overflow);
            }
            _ => panic!("Expected Resize command"),
        }
    }

    #[test]
    fn test_cli_parse_move_to_next() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "move", "--to", "next"]).unwrap();

        match cli.command {
            Commands::Move {
                to, display, app, ..
            } => {
                assert_eq!(to, Some("next".to_string()));
                assert!(display.is_none());
                assert!(app.is_none());
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn test_cli_parse_move_with_app() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["cwm", "move", "--to", "top-left", "--app", "Safari"]).unwrap();

        match cli.command {
            Commands::Move {
                to, display, app, ..
            } => {
                assert_eq!(to, Some("top-left".to_string()));
                assert!(display.is_none());
                assert_eq!(app, Some("Safari".to_string()));
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn test_cli_parse_move_with_display() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "move", "--display", "2"]).unwrap();

        match cli.command {
            Commands::Move {
                to, display, app, ..
            } => {
                assert!(to.is_none());
                assert_eq!(display, Some("2".to_string()));
                assert!(app.is_none());
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn test_cli_parse_move_with_to_and_display() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["cwm", "move", "--to", "50%,50%", "--display", "next"]).unwrap();

        match cli.command {
            Commands::Move {
                to, display, app, ..
            } => {
                assert_eq!(to, Some("50%,50%".to_string()));
                assert_eq!(display, Some("next".to_string()));
                assert!(app.is_none());
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn test_cli_parse_list_apps() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "list", "apps"]).unwrap();

        match cli.command {
            Commands::List {
                resource,
                json,
                detailed,
                ..
            } => {
                assert!(matches!(resource, Some(ListResource::Apps)));
                assert!(!json);
                assert!(!detailed);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_cli_parse_list_with_json() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "list", "displays", "--json"]).unwrap();

        match cli.command {
            Commands::List { resource, json, .. } => {
                assert!(matches!(resource, Some(ListResource::Displays)));
                assert!(json);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_cli_parse_list_with_detailed() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "list", "aliases", "--detailed"]).unwrap();

        match cli.command {
            Commands::List {
                resource, detailed, ..
            } => {
                assert!(matches!(resource, Some(ListResource::Aliases)));
                assert!(detailed);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_cli_parse_global_json_flag() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "--json", "list", "apps"]).unwrap();

        assert!(cli.json);
        assert!(!cli.no_json);
    }

    #[test]
    fn test_cli_parse_global_quiet_flag() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "--quiet", "maximize"]).unwrap();

        assert!(cli.quiet);
    }

    #[test]
    fn test_cli_parse_config_override() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["cwm", "--config", "/tmp/config.json", "list", "apps"]).unwrap();

        assert_eq!(
            cli.config,
            Some(std::path::PathBuf::from("/tmp/config.json"))
        );
    }

    // ========================================================================
    // Additional CLI parsing tests
    // ========================================================================

    #[test]
    fn test_cli_parse_record_shortcut() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "record", "shortcut"]).unwrap();

        match cli.command {
            Commands::Record { command } => match command {
                RecordCommands::Shortcut {
                    action,
                    app,
                    launch,
                    no_launch,
                    yes,
                } => {
                    assert!(action.is_none());
                    assert!(app.is_none());
                    assert!(!launch);
                    assert!(!no_launch);
                    assert!(!yes);
                }
                _ => panic!("Expected Shortcut subcommand"),
            },
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_cli_parse_record_shortcut_with_action() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm", "record", "shortcut", "--action", "focus", "--app", "Safari",
        ])
        .unwrap();

        match cli.command {
            Commands::Record { command } => match command {
                RecordCommands::Shortcut { action, app, .. } => {
                    assert_eq!(action, Some("focus".to_string()));
                    assert_eq!(app, Some("Safari".to_string()));
                }
                _ => panic!("Expected Shortcut subcommand"),
            },
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_cli_parse_record_shortcut_with_yes() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "record", "shortcut", "-y"]).unwrap();

        match cli.command {
            Commands::Record { command } => match command {
                RecordCommands::Shortcut { yes, .. } => {
                    assert!(yes);
                }
                _ => panic!("Expected Shortcut subcommand"),
            },
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_cli_parse_record_layout() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "record", "layout"]).unwrap();

        match cli.command {
            Commands::Record { command } => match command {
                RecordCommands::Layout { app, display } => {
                    assert!(app.is_empty());
                    assert!(display.is_none());
                }
                _ => panic!("Expected Layout subcommand"),
            },
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_cli_parse_record_layout_with_apps() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm", "record", "layout", "--app", "Safari", "--app", "Chrome",
        ])
        .unwrap();

        match cli.command {
            Commands::Record { command } => match command {
                RecordCommands::Layout { app, display } => {
                    assert_eq!(app, vec!["Safari".to_string(), "Chrome".to_string()]);
                    assert!(display.is_none());
                }
                _ => panic!("Expected Layout subcommand"),
            },
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_cli_parse_record_layout_with_display() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "record", "layout", "--display", "1"]).unwrap();

        match cli.command {
            Commands::Record { command } => match command {
                RecordCommands::Layout { app, display } => {
                    assert!(app.is_empty());
                    assert_eq!(display, Some("1".to_string()));
                }
                _ => panic!("Expected Layout subcommand"),
            },
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_cli_parse_record_layout_with_app_and_display() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm", "record", "layout", "--app", "Safari", "-d", "external",
        ])
        .unwrap();

        match cli.command {
            Commands::Record { command } => match command {
                RecordCommands::Layout { app, display } => {
                    assert_eq!(app, vec!["Safari".to_string()]);
                    assert_eq!(display, Some("external".to_string()));
                }
                _ => panic!("Expected Layout subcommand"),
            },
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_cli_parse_check_permissions() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "check-permissions"]).unwrap();

        match cli.command {
            Commands::CheckPermissions { prompt } => {
                assert!(!prompt);
            }
            _ => panic!("Expected CheckPermissions command"),
        }
    }

    #[test]
    fn test_cli_parse_check_permissions_with_prompt() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "check-permissions", "--prompt"]).unwrap();

        match cli.command {
            Commands::CheckPermissions { prompt } => {
                assert!(prompt);
            }
            _ => panic!("Expected CheckPermissions command"),
        }
    }

    #[test]
    fn test_cli_parse_install() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "install"]).unwrap();

        match cli.command {
            Commands::Install {
                path,
                force,
                no_sudo,
                completions,
                no_completions,
                completions_only,
            } => {
                assert!(path.is_none());
                assert!(!force);
                assert!(!no_sudo);
                assert!(completions.is_none());
                assert!(!no_completions);
                assert!(!completions_only);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parse_install_with_options() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm",
            "install",
            "--path",
            "/usr/local/bin",
            "--force",
            "--no-sudo",
        ])
        .unwrap();

        match cli.command {
            Commands::Install {
                path,
                force,
                no_sudo,
                ..
            } => {
                assert_eq!(path, Some(std::path::PathBuf::from("/usr/local/bin")));
                assert!(force);
                assert!(no_sudo);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parse_install_with_completions() {
        use clap::Parser;

        // --completions without value defaults to "auto"
        let cli = Cli::try_parse_from(["cwm", "install", "--completions"]).unwrap();
        match cli.command {
            Commands::Install { completions, .. } => {
                assert_eq!(completions, Some("auto".to_string()));
            }
            _ => panic!("Expected Install command"),
        }

        // --completions=zsh
        let cli = Cli::try_parse_from(["cwm", "install", "--completions=zsh"]).unwrap();
        match cli.command {
            Commands::Install { completions, .. } => {
                assert_eq!(completions, Some("zsh".to_string()));
            }
            _ => panic!("Expected Install command"),
        }

        // --completions-only requires --completions
        let cli =
            Cli::try_parse_from(["cwm", "install", "--completions-only", "--completions=bash"])
                .unwrap();
        match cli.command {
            Commands::Install {
                completions,
                completions_only,
                ..
            } => {
                assert_eq!(completions, Some("bash".to_string()));
                assert!(completions_only);
            }
            _ => panic!("Expected Install command"),
        }

        // --no-completions
        let cli = Cli::try_parse_from(["cwm", "install", "--no-completions"]).unwrap();
        match cli.command {
            Commands::Install { no_completions, .. } => {
                assert!(no_completions);
            }
            _ => panic!("Expected Install command"),
        }
    }

    #[test]
    fn test_cli_parse_uninstall() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "uninstall"]).unwrap();

        match cli.command {
            Commands::Uninstall { path } => {
                assert!(path.is_none());
            }
            _ => panic!("Expected Uninstall command"),
        }
    }

    #[test]
    fn test_cli_parse_uninstall_with_path() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "uninstall", "--path", "/usr/local/bin"]).unwrap();

        match cli.command {
            Commands::Uninstall { path } => {
                assert_eq!(path, Some(std::path::PathBuf::from("/usr/local/bin")));
            }
            _ => panic!("Expected Uninstall command"),
        }
    }

    #[test]
    fn test_cli_parse_update() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "update"]).unwrap();

        match cli.command {
            Commands::Update {
                check,
                force,
                prerelease,
            } => {
                assert!(!check);
                assert!(!force);
                assert!(!prerelease);
            }
            _ => panic!("Expected Update command"),
        }
    }

    #[test]
    fn test_cli_parse_update_with_options() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["cwm", "update", "--check", "--force", "--prerelease"]).unwrap();

        match cli.command {
            Commands::Update {
                check,
                force,
                prerelease,
            } => {
                assert!(check);
                assert!(force);
                assert!(prerelease);
            }
            _ => panic!("Expected Update command"),
        }
    }

    #[test]
    fn test_cli_parse_spotlight_install() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "spotlight", "install"]).unwrap();

        match cli.command {
            Commands::Spotlight { command } => match command {
                SpotlightCommands::Install { name, force } => {
                    assert!(name.is_none());
                    assert!(!force);
                }
                _ => panic!("Expected Install subcommand"),
            },
            _ => panic!("Expected Spotlight command"),
        }
    }

    #[test]
    fn test_cli_parse_spotlight_install_with_options() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm",
            "spotlight",
            "install",
            "--name",
            "Focus Safari",
            "--force",
        ])
        .unwrap();

        match cli.command {
            Commands::Spotlight { command } => match command {
                SpotlightCommands::Install { name, force } => {
                    assert_eq!(name, Some("Focus Safari".to_string()));
                    assert!(force);
                }
                _ => panic!("Expected Install subcommand"),
            },
            _ => panic!("Expected Spotlight command"),
        }
    }

    #[test]
    fn test_cli_parse_spotlight_remove() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "spotlight", "remove", "Focus Safari"]).unwrap();

        match cli.command {
            Commands::Spotlight { command } => match command {
                SpotlightCommands::Remove { name, all } => {
                    assert_eq!(name, Some("Focus Safari".to_string()));
                    assert!(!all);
                }
                _ => panic!("Expected Remove subcommand"),
            },
            _ => panic!("Expected Spotlight command"),
        }
    }

    #[test]
    fn test_cli_parse_spotlight_remove_all() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "spotlight", "remove", "--all"]).unwrap();

        match cli.command {
            Commands::Spotlight { command } => match command {
                SpotlightCommands::Remove { name, all } => {
                    assert!(name.is_none());
                    assert!(all);
                }
                _ => panic!("Expected Remove subcommand"),
            },
            _ => panic!("Expected Spotlight command"),
        }
    }

    #[test]
    fn test_cli_parse_daemon_start() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "daemon", "start"]).unwrap();

        match cli.command {
            Commands::Daemon { command } => match command {
                DaemonCommands::Start { log, foreground } => {
                    assert!(log.is_none());
                    assert!(!foreground);
                }
                _ => panic!("Expected Start subcommand"),
            },
            _ => panic!("Expected Daemon command"),
        }
    }

    #[test]
    fn test_cli_parse_daemon_start_with_options() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm",
            "daemon",
            "start",
            "--log",
            "/tmp/cwm.log",
            "--foreground",
        ])
        .unwrap();

        match cli.command {
            Commands::Daemon { command } => match command {
                DaemonCommands::Start { log, foreground } => {
                    assert_eq!(log, Some("/tmp/cwm.log".to_string()));
                    assert!(foreground);
                }
                _ => panic!("Expected Start subcommand"),
            },
            _ => panic!("Expected Daemon command"),
        }
    }

    #[test]
    fn test_cli_parse_daemon_install() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "daemon", "install"]).unwrap();

        match cli.command {
            Commands::Daemon { command } => match command {
                DaemonCommands::Install { bin, log } => {
                    assert!(bin.is_none());
                    assert!(log.is_none());
                }
                _ => panic!("Expected Install subcommand"),
            },
            _ => panic!("Expected Daemon command"),
        }
    }

    #[test]
    fn test_cli_parse_daemon_install_with_options() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm",
            "daemon",
            "install",
            "--bin",
            "/usr/local/bin/cwm",
            "--log",
            "/tmp/cwm.log",
        ])
        .unwrap();

        match cli.command {
            Commands::Daemon { command } => match command {
                DaemonCommands::Install { bin, log } => {
                    assert_eq!(bin, Some("/usr/local/bin/cwm".to_string()));
                    assert_eq!(log, Some("/tmp/cwm.log".to_string()));
                }
                _ => panic!("Expected Install subcommand"),
            },
            _ => panic!("Expected Daemon command"),
        }
    }

    #[test]
    fn test_cli_parse_get_focused() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "get", "focused"]).unwrap();

        match cli.command {
            Commands::Get { command } => match command {
                GetCommands::Focused { format } => {
                    assert!(format.is_none());
                }
                _ => panic!("Expected Focused subcommand"),
            },
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_cli_parse_get_focused_with_format() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "get", "focused", "--format", "{app.name}"]).unwrap();

        match cli.command {
            Commands::Get { command } => match command {
                GetCommands::Focused { format } => {
                    assert_eq!(format, Some("{app.name}".to_string()));
                }
                _ => panic!("Expected Focused subcommand"),
            },
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_cli_parse_get_window() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "get", "window", "--app", "Safari"]).unwrap();

        match cli.command {
            Commands::Get { command } => match command {
                GetCommands::Window { app, format } => {
                    assert_eq!(app, vec!["Safari".to_string()]);
                    assert!(format.is_none());
                }
                _ => panic!("Expected Window subcommand"),
            },
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_cli_parse_get_window_multiple_apps() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "cwm",
            "get",
            "window",
            "--app",
            "/safari/i",
            "--app",
            "/chrome/i",
        ])
        .unwrap();

        match cli.command {
            Commands::Get { command } => match command {
                GetCommands::Window { app, format } => {
                    assert_eq!(app, vec!["/safari/i".to_string(), "/chrome/i".to_string()]);
                    assert!(format.is_none());
                }
                _ => panic!("Expected Window subcommand"),
            },
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_cli_parse_config_set() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["cwm", "config", "set", "settings.fuzzy_threshold", "3"]).unwrap();

        match cli.command {
            Commands::Config { command } => match command {
                ConfigCommands::Set { key, value } => {
                    assert_eq!(key, "settings.fuzzy_threshold");
                    assert_eq!(value, "3");
                }
                _ => panic!("Expected Set subcommand"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_parse_list_with_names() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["cwm", "list", "apps", "--names"]).unwrap();

        match cli.command {
            Commands::List { names, .. } => {
                assert!(names);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_cli_parse_list_with_format() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["cwm", "list", "apps", "--format", "{name} ({pid})"]).unwrap();

        match cli.command {
            Commands::List { format, .. } => {
                assert_eq!(format, Some("{name} ({pid})".to_string()));
            }
            _ => panic!("Expected List command"),
        }
    }

    // ========================================================================
    // resolve_app_names tests
    // ========================================================================

    #[test]
    fn test_resolve_app_names_single() {
        let apps = vec!["Safari".to_string()];
        let result = resolve_app_names(&apps).unwrap();
        assert_eq!(result, vec!["Safari"]);
    }

    #[test]
    fn test_resolve_app_names_multiple() {
        let apps = vec![
            "Safari".to_string(),
            "Chrome".to_string(),
            "Firefox".to_string(),
        ];
        let result = resolve_app_names(&apps).unwrap();
        assert_eq!(result, vec!["Safari", "Chrome", "Firefox"]);
    }

    #[test]
    fn test_resolve_app_names_empty() {
        let apps: Vec<String> = vec![];
        let result = resolve_app_names(&apps).unwrap();
        assert!(result.is_empty());
    }
}
