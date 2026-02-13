use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;

use crate::config::{self, Config, Shortcut};
use crate::daemon::hotkeys;
use crate::display;
use crate::window::{accessibility, manager, matching};

#[derive(Parser)]
#[command(name = "cwm")]
#[command(about = "A macOS window manager with CLI and global hotkeys")]
#[command(version)]
pub struct Cli {
    /// Path to config file (overrides CWM_CONFIG env var and default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

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

    /// Move a window to another display
    MoveDisplay {
        /// Target display: "next", "prev", display index (0-based), or alias name
        target: String,

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

    /// Record a keyboard shortcut
    RecordShortcut {
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
        /// Resource type to list
        #[arg(value_enum)]
        resource: ListResource,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Include additional fields in JSON output
        #[arg(short, long)]
        detailed: bool,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ListResource {
    /// Running applications
    Apps,
    /// Available displays
    Displays,
    /// Display aliases (system and user-defined)
    Aliases,
}

// JSON output structs for list command

#[derive(Serialize)]
struct ListResponse<T: Serialize> {
    items: Vec<T>,
}

#[derive(Serialize)]
struct AppSummary {
    name: String,
    pid: i32,
}

#[derive(Serialize)]
struct AppDetailed {
    name: String,
    pid: i32,
    bundle_id: Option<String>,
    titles: Vec<String>,
}

#[derive(Serialize)]
struct DisplaySummary {
    index: usize,
    name: String,
    width: u32,
    height: u32,
    is_main: bool,
}

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

#[derive(Serialize)]
struct AliasSummary {
    name: String,
    #[serde(rename = "type")]
    alias_type: String,
    resolved: bool,
    display_index: Option<usize>,
}

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

pub fn execute(cli: Cli) -> Result<()> {
    let config_path = cli.config.as_deref();

    match cli.command {
        Commands::Focus {
            app: apps,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load_with_override(config_path)?;
            let running_apps = matching::get_running_apps()?;

            // try each app in order until one is found
            for app in &apps {
                let match_result =
                    matching::find_app(app, &running_apps, config.settings.fuzzy_threshold);

                if let Some(result) = match_result {
                    if verbose {
                        println!("Matched {} -> {}", app, result.describe());
                    }
                    manager::focus_app(&result.app, verbose)?;
                    if !verbose {
                        println!("Focused: {}", result.app.name);
                    }
                    return Ok(());
                } else if verbose {
                    println!("App '{}' not found, trying next...", app);
                }
            }

            // no app found, check if we should launch the first one
            let should_launch =
                config::should_launch(launch, no_launch, None, config.settings.launch);

            if should_launch {
                let first_app = &apps[0];
                if verbose {
                    println!("No apps found, launching '{}'...", first_app);
                }
                manager::launch_app(first_app, verbose)?;
            } else {
                return Err(anyhow!(
                    "No matching app found. Tried: {}. Running apps: {}",
                    apps.join(", "),
                    running_apps
                        .iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            Ok(())
        }

        Commands::Maximize {
            app,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load_with_override(config_path)?;

            let target_app = if let Some(app_name) = app {
                let running_apps = matching::get_running_apps()?;
                let match_result =
                    matching::find_app(&app_name, &running_apps, config.settings.fuzzy_threshold);

                match match_result {
                    Some(result) => {
                        if verbose {
                            println!("Matched {} -> {}", app_name, result.describe());
                        }
                        Some(result.app)
                    }
                    None => {
                        let should_launch =
                            config::should_launch(launch, no_launch, None, config.settings.launch);

                        if should_launch {
                            if verbose {
                                println!("App '{}' not running, launching...", app_name);
                            }
                            manager::launch_app(&app_name, verbose)?;
                            // after launching, we can't maximize immediately
                            // the app needs time to start
                            println!("App launched. Run maximize again once it's ready.");
                            return Ok(());
                        } else {
                            return Err(anyhow!("App '{}' not found", app_name));
                        }
                    }
                }
            } else {
                None
            };

            manager::maximize_app(target_app.as_ref(), verbose)?;
            Ok(())
        }

        Commands::MoveDisplay {
            target,
            app,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load_with_override(config_path)?;
            let display_target = display::DisplayTarget::parse(&target)?;

            let target_app = if let Some(app_name) = app {
                let running_apps = matching::get_running_apps()?;
                let match_result =
                    matching::find_app(&app_name, &running_apps, config.settings.fuzzy_threshold);

                match match_result {
                    Some(result) => {
                        if verbose {
                            println!("Matched {} -> {}", app_name, result.describe());
                        }
                        Some(result.app)
                    }
                    None => {
                        let should_launch =
                            config::should_launch(launch, no_launch, None, config.settings.launch);

                        if should_launch {
                            if verbose {
                                println!("App '{}' not running, launching...", app_name);
                            }
                            manager::launch_app(&app_name, verbose)?;
                            println!("App launched. Run move-display again once it's ready.");
                            return Ok(());
                        } else {
                            return Err(anyhow!("App '{}' not found", app_name));
                        }
                    }
                }
            } else {
                None
            };

            manager::move_to_display_with_aliases(
                target_app.as_ref(),
                &display_target,
                verbose,
                &config.display_aliases,
            )?;
            Ok(())
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

            // parse the resize target
            let resize_target = ResizeTarget::parse(&to)?;

            let target_app = if let Some(app_name) = app {
                let running_apps = matching::get_running_apps()?;
                let match_result =
                    matching::find_app(&app_name, &running_apps, config.settings.fuzzy_threshold);

                match match_result {
                    Some(result) => {
                        if verbose {
                            println!("Matched {} -> {}", app_name, result.describe());
                        }
                        Some(result.app)
                    }
                    None => {
                        let should_launch =
                            config::should_launch(launch, no_launch, None, config.settings.launch);

                        if should_launch {
                            if verbose {
                                println!("App '{}' not running, launching...", app_name);
                            }
                            manager::launch_app(&app_name, verbose)?;
                            println!("App launched. Run resize again once it's ready.");
                            return Ok(());
                        } else {
                            return Err(anyhow!("App '{}' not found", app_name));
                        }
                    }
                }
            } else {
                None
            };

            manager::resize_app(target_app.as_ref(), &resize_target, overflow, verbose)?;
            Ok(())
        }

        Commands::RecordShortcut {
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
                println!("  cwm record-shortcut --action focus --app \"AppName\"");
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
            };

            if launch {
                shortcut.launch = Some(true);
            } else if no_launch {
                shortcut.launch = Some(false);
            }

            // show what will be added
            let json =
                serde_json::to_string_pretty(&shortcut).context("Failed to serialize shortcut")?;
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

        Commands::Daemon { command } => match command {
            DaemonCommands::Start { log, foreground } => {
                if foreground {
                    crate::daemon::start_foreground(log)
                } else {
                    crate::daemon::start(log)
                }
            }
            DaemonCommands::Stop => crate::daemon::stop(),
            DaemonCommands::Status => {
                crate::daemon::status()?;
                Ok(())
            }
            DaemonCommands::Install { bin, log } => crate::daemon::install(bin, log),
            DaemonCommands::Uninstall => crate::daemon::uninstall(),
            DaemonCommands::RunForeground { log } => crate::daemon::start_foreground(log),
        },

        Commands::Config { command } => match command {
            ConfigCommands::Show => {
                let config = config::load_with_override(config_path)?;
                let json =
                    serde_json::to_string_pretty(&config).context("Failed to serialize config")?;
                println!("{}", json);
                Ok(())
            }
            ConfigCommands::Path => {
                let path = config::get_config_path_with_override(config_path)?;
                println!("{}", path.display());
                Ok(())
            }
            ConfigCommands::Set { key, value } => {
                let mut config = config::load_with_override(config_path)?;
                config::set_value(&mut config, &key, &value)?;
                config::save_with_override(&config, config_path)?;
                println!("Set {} = {}", key, value);
                Ok(())
            }
            ConfigCommands::Reset => {
                let config = Config::default();
                config::save_with_override(&config, config_path)?;
                println!("Configuration reset to defaults");
                Ok(())
            }
            ConfigCommands::Default => {
                let config = config::default_with_examples();
                let json =
                    serde_json::to_string_pretty(&config).context("Failed to serialize config")?;
                println!("{}", json);
                Ok(())
            }
            ConfigCommands::Verify => {
                let path = config::get_config_path_with_override(config_path)?;
                let errors = config::verify(&path)?;

                if errors.is_empty() {
                    println!("✓ Configuration is valid: {}", path.display());
                    Ok(())
                } else {
                    println!(
                        "✗ Configuration has {} error(s): {}",
                        errors.len(),
                        path.display()
                    );
                    println!();
                    for error in &errors {
                        println!("  - {}", error);
                    }
                    Err(anyhow!("configuration validation failed"))
                }
            }
        },

        Commands::List {
            resource,
            json,
            detailed,
        } => {
            match resource {
                ListResource::Apps => {
                    let apps = matching::get_running_apps()?;

                    if json {
                        if detailed {
                            let items: Vec<AppDetailed> = apps
                                .iter()
                                .map(|a| AppDetailed {
                                    name: a.name.clone(),
                                    pid: a.pid,
                                    bundle_id: a.bundle_id.clone(),
                                    titles: a.titles.clone(),
                                })
                                .collect();
                            let response = ListResponse { items };
                            println!(
                                "{}",
                                serde_json::to_string(&response)
                                    .context("Failed to serialize apps")?
                            );
                        } else {
                            let items: Vec<AppSummary> = apps
                                .iter()
                                .map(|a| AppSummary {
                                    name: a.name.clone(),
                                    pid: a.pid,
                                })
                                .collect();
                            let response = ListResponse { items };
                            println!(
                                "{}",
                                serde_json::to_string(&response)
                                    .context("Failed to serialize apps")?
                            );
                        }
                    } else {
                        println!("Running applications:");
                        for app in &apps {
                            let bundle = app
                                .bundle_id
                                .as_ref()
                                .map(|b| format!(" ({})", b))
                                .unwrap_or_default();
                            println!("  {} [PID: {}]{}", app.name, app.pid, bundle);
                            for title in &app.titles {
                                println!("    - {}", title);
                            }
                        }
                        println!("\nTotal: {} applications", apps.len());
                    }
                }

                ListResource::Displays => {
                    let displays = display::get_displays()?;

                    if json {
                        if detailed {
                            let items: Vec<DisplayDetailed> = displays
                                .iter()
                                .map(|d| DisplayDetailed {
                                    index: d.index,
                                    name: d.name.clone(),
                                    width: d.width,
                                    height: d.height,
                                    x: d.x,
                                    y: d.y,
                                    is_main: d.is_main,
                                    is_builtin: d.is_builtin,
                                    display_id: d.display_id,
                                    vendor_id: d.vendor_id,
                                    model_id: d.model_id,
                                    serial_number: d.serial_number,
                                    unit_number: d.unit_number,
                                    unique_id: d.unique_id(),
                                })
                                .collect();
                            let response = ListResponse { items };
                            println!(
                                "{}",
                                serde_json::to_string(&response)
                                    .context("Failed to serialize displays")?
                            );
                        } else {
                            let items: Vec<DisplaySummary> = displays
                                .iter()
                                .map(|d| DisplaySummary {
                                    index: d.index,
                                    name: d.name.clone(),
                                    width: d.width,
                                    height: d.height,
                                    is_main: d.is_main,
                                })
                                .collect();
                            let response = ListResponse { items };
                            println!(
                                "{}",
                                serde_json::to_string(&response)
                                    .context("Failed to serialize displays")?
                            );
                        }
                    } else if displays.is_empty() {
                        println!("No displays found");
                    } else {
                        println!("Available displays:");
                        for d in &displays {
                            println!("  {}", d.describe());
                        }
                    }
                }

                ListResource::Aliases => {
                    let config = config::load_with_override(config_path)?;
                    let displays = display::get_displays()?;

                    let system_aliases = [
                        ("builtin", "Built-in display"),
                        ("external", "External display"),
                        ("main", "Primary display"),
                        ("secondary", "Secondary display"),
                    ];

                    if json {
                        if detailed {
                            let mut items: Vec<AliasDetailed> = Vec::new();

                            // system aliases
                            for (alias_name, description) in &system_aliases {
                                let resolved = display::resolve_alias(
                                    alias_name,
                                    &config.display_aliases,
                                    &displays,
                                );
                                items.push(AliasDetailed {
                                    name: alias_name.to_string(),
                                    alias_type: "system".to_string(),
                                    resolved: resolved.is_ok(),
                                    display_index: resolved.as_ref().ok().map(|d| d.index),
                                    display_name: resolved.as_ref().ok().map(|d| d.name.clone()),
                                    display_unique_id: resolved
                                        .as_ref()
                                        .ok()
                                        .map(|d| d.unique_id()),
                                    description: Some(description.to_string()),
                                    mapped_ids: None,
                                });
                            }

                            // user-defined aliases
                            for (alias_name, mappings) in &config.display_aliases {
                                let resolved = display::resolve_alias(
                                    alias_name,
                                    &config.display_aliases,
                                    &displays,
                                );
                                items.push(AliasDetailed {
                                    name: alias_name.clone(),
                                    alias_type: "user".to_string(),
                                    resolved: resolved.is_ok(),
                                    display_index: resolved.as_ref().ok().map(|d| d.index),
                                    display_name: resolved.as_ref().ok().map(|d| d.name.clone()),
                                    display_unique_id: resolved
                                        .as_ref()
                                        .ok()
                                        .map(|d| d.unique_id()),
                                    description: None,
                                    mapped_ids: Some(mappings.clone()),
                                });
                            }

                            let response = ListResponse { items };
                            println!(
                                "{}",
                                serde_json::to_string(&response)
                                    .context("Failed to serialize aliases")?
                            );
                        } else {
                            let mut items: Vec<AliasSummary> = Vec::new();

                            // system aliases
                            for (alias_name, _) in &system_aliases {
                                let resolved = display::resolve_alias(
                                    alias_name,
                                    &config.display_aliases,
                                    &displays,
                                );
                                items.push(AliasSummary {
                                    name: alias_name.to_string(),
                                    alias_type: "system".to_string(),
                                    resolved: resolved.is_ok(),
                                    display_index: resolved.as_ref().ok().map(|d| d.index),
                                });
                            }

                            // user-defined aliases
                            for alias_name in config.display_aliases.keys() {
                                let resolved = display::resolve_alias(
                                    alias_name,
                                    &config.display_aliases,
                                    &displays,
                                );
                                items.push(AliasSummary {
                                    name: alias_name.clone(),
                                    alias_type: "user".to_string(),
                                    resolved: resolved.is_ok(),
                                    display_index: resolved.as_ref().ok().map(|d| d.index),
                                });
                            }

                            let response = ListResponse { items };
                            println!(
                                "{}",
                                serde_json::to_string(&response)
                                    .context("Failed to serialize aliases")?
                            );
                        }
                    } else {
                        println!("System Aliases:");
                        for (alias_name, description) in &system_aliases {
                            if let Ok(d) = display::resolve_alias(
                                alias_name,
                                &config.display_aliases,
                                &displays,
                            ) {
                                println!(
                                    "  {:<20} → Display {}: {} [{}]",
                                    alias_name,
                                    d.index,
                                    d.name,
                                    d.unique_id()
                                );
                            } else {
                                println!(
                                    "  {:<20} → Not found in current setup ({})",
                                    alias_name, description
                                );
                            }
                        }

                        if !config.display_aliases.is_empty() {
                            println!("\nUser-Defined Aliases:");
                            for (alias_name, mappings) in &config.display_aliases {
                                if let Ok(d) = display::resolve_alias(
                                    alias_name,
                                    &config.display_aliases,
                                    &displays,
                                ) {
                                    println!(
                                        "  {:<20} → Display {}: {} [{}] ✓",
                                        alias_name,
                                        d.index,
                                        d.name,
                                        d.unique_id()
                                    );
                                } else {
                                    println!(
                                        "  {:<20} → Not found (mapped: {})",
                                        alias_name,
                                        mappings.join(", ")
                                    );
                                }
                            }
                        } else {
                            println!("\nNo user-defined aliases configured.");
                        }
                    }
                }
            }

            Ok(())
        }

        Commands::CheckPermissions { prompt } => {
            if prompt {
                let trusted = accessibility::check_and_prompt();
                if trusted {
                    println!("✓ Accessibility permissions granted");
                } else {
                    println!("✗ Accessibility permissions not granted");
                    println!("\nPlease grant permissions in System Settings:");
                    println!("  System Settings > Privacy & Security > Accessibility");
                }
            } else {
                accessibility::print_permission_status()?;
            }
            Ok(())
        }

        Commands::Version => {
            use crate::version::{Version, VersionInfo};

            let version = Version::current();
            println!("cwm {}", version.version_string());
            println!(
                "Built: {}",
                version.build_date.format("%Y-%m-%d %H:%M:%S UTC")
            );

            // try to load version info for install path
            if let Ok(info) = VersionInfo::load() {
                println!("Installed: {}", info.install_path.display());
            }

            println!("Repository: https://github.com/{}", env!("GITHUB_REPO"));
            Ok(())
        }

        Commands::Install {
            path,
            force,
            no_sudo,
        } => {
            use crate::installer::{detect_install_paths, install_binary};

            let target_dir = if let Some(p) = path {
                p
            } else {
                // interactive path selection
                let paths = detect_install_paths();

                if paths.is_empty() {
                    return Err(anyhow!("No suitable installation directories found"));
                }

                println!("Where would you like to install cwm?\n");
                for (i, path) in paths.iter().enumerate() {
                    println!("  {}. {}", i + 1, path.status_line());
                }
                println!("  {}. Custom path...", paths.len() + 1);

                print!("\nChoice [1]: ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let choice = input.trim();

                let idx = if choice.is_empty() {
                    0
                } else {
                    choice
                        .parse::<usize>()
                        .map(|n| n.saturating_sub(1))
                        .unwrap_or(0)
                };

                if idx < paths.len() {
                    paths[idx].path.clone()
                } else {
                    // custom path
                    print!("Enter custom path: ");
                    io::stdout().flush()?;
                    let mut custom = String::new();
                    io::stdin().read_line(&mut custom)?;
                    PathBuf::from(shellexpand::tilde(custom.trim()).to_string())
                }
            };

            // check if we need sudo
            let needs_sudo = !no_sudo && !crate::installer::paths::check_writable(&target_dir);

            install_binary(&target_dir, force, needs_sudo)?;
            Ok(())
        }

        Commands::Uninstall { path } => {
            use crate::installer::uninstall_binary;

            uninstall_binary(path.as_deref())?;
            Ok(())
        }

        Commands::Update {
            check,
            force,
            prerelease,
        } => {
            use crate::installer::{check_for_updates, perform_update};
            use crate::version::Version;

            let mut config = config::load_with_override(config_path)?;

            // enable prerelease channels if requested
            if prerelease {
                config.settings.update.channels.beta = true;
                config.settings.update.channels.dev = true;
            }

            let current = Version::current();
            println!("Current version: {}", current.version_string());

            println!("Checking for updates...");
            match check_for_updates(&config.settings.update, true)? {
                Some(release) => {
                    println!("\n🆕 New version available: {}", release.version);

                    if let Some(ref notes) = release.release_notes {
                        println!("\nRelease notes:");
                        println!("{}", notes);
                    }

                    if check {
                        println!("\nRun 'cwm update' to install");
                    } else {
                        println!("\nUpdate size: {:.2} MB", release.size as f64 / 1_048_576.0);

                        if !force {
                            print!("Install update? [Y/n]: ");
                            use std::io::{self, Write};
                            io::stdout().flush()?;

                            let mut input = String::new();
                            io::stdin().read_line(&mut input)?;

                            if input.trim().to_lowercase() == "n" {
                                println!("Update cancelled");
                                return Ok(());
                            }
                        }

                        perform_update(release, force)?;

                        // update last check time
                        config.settings.update.last_check = Some(chrono::Utc::now());
                        config::save_with_override(&config, config_path)?;
                    }
                }
                None => {
                    println!("You are on the latest version");

                    // update last check time
                    config.settings.update.last_check = Some(chrono::Utc::now());
                    config::save_with_override(&config, config_path)?;
                }
            }

            Ok(())
        }

        Commands::Spotlight { command } => match command {
            SpotlightCommands::Install { name, force } => {
                let config = config::load_with_override(config_path)?;

                if config.spotlight.is_empty() {
                    println!("No spotlight shortcuts configured.");
                    println!("\nAdd shortcuts to your config file:");
                    println!("  cwm spotlight example");
                    println!("\nOr edit ~/.cwm/config.json directly.");
                    return Ok(());
                }

                let apps_dir = crate::spotlight::get_apps_directory();
                println!("Installing spotlight shortcuts to: {}", apps_dir.display());

                if let Some(shortcut_name) = name {
                    // install specific shortcut
                    let shortcut = config
                        .spotlight
                        .iter()
                        .find(|s| s.name.eq_ignore_ascii_case(&shortcut_name))
                        .ok_or_else(|| {
                            anyhow!(
                                "Shortcut '{}' not found in config. Available: {}",
                                shortcut_name,
                                config
                                    .spotlight
                                    .iter()
                                    .map(|s| s.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })?;

                    let path = crate::spotlight::install_shortcut(shortcut, force)?;
                    println!("✓ Installed: {}", path.display());
                } else {
                    // install all shortcuts
                    let installed = crate::spotlight::install_all(&config.spotlight, force)?;

                    if installed.is_empty() {
                        println!("No shortcuts were installed.");
                    } else {
                        println!("\n✓ Installed {} shortcut(s):", installed.len());
                        for path in &installed {
                            if let Some(name) = path.file_name() {
                                println!("  - {}", name.to_string_lossy());
                            }
                        }
                    }
                }

                println!("\nShortcuts are now available in Spotlight.");
                println!("Search for \"cwm: <name>\" to use them.");

                Ok(())
            }

            SpotlightCommands::List => {
                let installed = crate::spotlight::get_installed_shortcuts()?;

                if installed.is_empty() {
                    println!("No spotlight shortcuts installed.");
                    println!("\nTo install shortcuts:");
                    println!("  1. Add shortcuts to config: cwm spotlight example");
                    println!("  2. Install them: cwm spotlight install");
                } else {
                    println!("Installed spotlight shortcuts:\n");
                    for name in &installed {
                        println!("  cwm: {}", name);
                    }
                    println!("\nTotal: {} shortcut(s)", installed.len());
                    println!(
                        "Location: {}",
                        crate::spotlight::get_apps_directory().display()
                    );
                }

                Ok(())
            }

            SpotlightCommands::Remove { name, all } => {
                if all {
                    let count = crate::spotlight::remove_all()?;
                    if count == 0 {
                        println!("No spotlight shortcuts to remove.");
                    } else {
                        println!("✓ Removed {} shortcut(s)", count);
                    }
                } else if let Some(shortcut_name) = name {
                    crate::spotlight::remove_shortcut(&shortcut_name)?;
                    println!("✓ Removed: cwm: {}", shortcut_name);
                } else {
                    return Err(anyhow!(
                        "Specify a shortcut name or use --all to remove all shortcuts"
                    ));
                }

                Ok(())
            }

            SpotlightCommands::Example => {
                crate::spotlight::print_example_config();
                Ok(())
            }
        },
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
}
