use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use crate::config::{self, Config, Shortcut};
use crate::daemon::hotkeys;
use crate::display;
use crate::window::{accessibility, manager, matching};

#[derive(Parser)]
#[command(name = "cwm")]
#[command(about = "A macOS window manager with CLI and global hotkeys")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Focus an application window
    Focus {
        /// Target app name (fuzzy matched)
        #[arg(short, long)]
        app: String,

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
        /// Target display: "next", "prev", or display index (0-based)
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

    /// Resize a window to a percentage of the screen (centered)
    Resize {
        /// Percentage of screen (1-100), or "full" for 100%
        size: String,

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

    /// List available displays
    ListDisplays {
        /// Show detailed information including identifiers
        #[arg(short, long)]
        detailed: bool,
    },

    /// List running applications
    ListApps,

    /// Check accessibility permissions
    CheckPermissions {
        /// Prompt to grant permissions if not already granted
        #[arg(long)]
        prompt: bool,
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
}

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Focus {
            app,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load()?;
            let running_apps = matching::get_running_apps()?;

            let match_result = matching::find_app(&app, &running_apps, config.settings.fuzzy_threshold);

            match match_result {
                Some(result) => {
                    if verbose {
                        println!("Matched {} -> {}", app, result.describe());
                    }
                    manager::focus_app(&result.app, verbose)?;
                    if !verbose {
                        println!("Focused: {}", result.app.name);
                    }
                }
                None => {
                    // app not found, check if we should launch
                    let should_launch = config::should_launch(
                        launch,
                        no_launch,
                        None,
                        config.settings.launch,
                    );

                    if should_launch {
                        if verbose {
                            println!("App '{}' not running, launching...", app);
                        }
                        manager::launch_app(&app, verbose)?;
                    } else {
                        return Err(anyhow!(
                            "App '{}' not found. Running apps: {}",
                            app,
                            running_apps
                                .iter()
                                .map(|a| a.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
            }
            Ok(())
        }

        Commands::Maximize {
            app,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load()?;

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
                        let should_launch = config::should_launch(
                            launch,
                            no_launch,
                            None,
                            config.settings.launch,
                        );

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
            let config = config::load()?;
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
                        let should_launch = config::should_launch(
                            launch,
                            no_launch,
                            None,
                            config.settings.launch,
                        );

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

            manager::move_to_display(target_app.as_ref(), &display_target, verbose)?;
            Ok(())
        }

        Commands::Resize {
            size,
            app,
            launch,
            no_launch,
            verbose,
        } => {
            let config = config::load()?;

            // parse size: "full" or a number 1-100
            let percent: u32 = if size.eq_ignore_ascii_case("full") {
                100
            } else {
                size.parse().map_err(|_| {
                    anyhow!("Invalid size '{}'. Use a number 1-100 or 'full'", size)
                })?
            };

            if percent == 0 || percent > 100 {
                return Err(anyhow!("Size must be between 1 and 100"));
            }

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
                        let should_launch = config::should_launch(
                            launch,
                            no_launch,
                            None,
                            config.settings.launch,
                        );

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

            manager::resize_app(target_app.as_ref(), percent, verbose)?;
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
            let json = serde_json::to_string_pretty(&shortcut)
                .context("Failed to serialize shortcut")?;
            println!("\nShortcut to add:\n{}", json);

            // load config and check for duplicates
            let mut config = config::load()?;
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
            config::save(&config)?;
            println!("\nSaved to {}", config::get_config_path().display());

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
            DaemonCommands::Stop => {
                crate::daemon::stop()
            }
            DaemonCommands::Status => {
                crate::daemon::status()?;
                Ok(())
            }
            DaemonCommands::Install { bin, log } => {
                crate::daemon::install(bin, log)
            }
            DaemonCommands::Uninstall => {
                crate::daemon::uninstall()
            }
            DaemonCommands::RunForeground { log } => {
                crate::daemon::start_foreground(log)
            }
        },

        Commands::Config { command } => match command {
            ConfigCommands::Show => {
                let config = config::load()?;
                let json = serde_json::to_string_pretty(&config)
                    .context("Failed to serialize config")?;
                println!("{}", json);
                Ok(())
            }
            ConfigCommands::Path => {
                let path = config::get_config_path();
                println!("{}", path.display());
                Ok(())
            }
            ConfigCommands::Set { key, value } => {
                let mut config = config::load()?;
                config::set_value(&mut config, &key, &value)?;
                config::save(&config)?;
                println!("Set {} = {}", key, value);
                Ok(())
            }
            ConfigCommands::Reset => {
                let config = Config::default();
                config::save(&config)?;
                println!("Configuration reset to defaults");
                Ok(())
            }
            ConfigCommands::Default => {
                let config = config::default_with_examples();
                let json = serde_json::to_string_pretty(&config)
                    .context("Failed to serialize config")?;
                println!("{}", json);
                Ok(())
            }
        },

        Commands::ListDisplays { detailed } => {
            display::print_displays(detailed)?;
            Ok(())
        }

        Commands::ListApps => {
            let apps = matching::get_running_apps()?;
            println!("Running applications:");
            for app in &apps {
                let bundle = app
                    .bundle_id
                    .as_ref()
                    .map(|b| format!(" ({})", b))
                    .unwrap_or_default();
                println!("  {} [PID: {}]{}", app.name, app.pid, bundle);
            }
            println!("\nTotal: {} applications", apps.len());
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
    }
}
