use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use crate::config::{self, Config};
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
    ListDisplays,

    /// List running application windows
    ListWindows,

    /// Check accessibility permissions
    CheckPermissions {
        /// Prompt to grant permissions if not already granted
        #[arg(long)]
        prompt: bool,
    },
}

#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
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

            let match_result = matching::find_app(&app, &running_apps, config.matching.fuzzy_threshold);

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
                        config.behavior.launch_if_not_running,
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
                    matching::find_app(&app_name, &running_apps, config.matching.fuzzy_threshold);

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
                            config.behavior.launch_if_not_running,
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

            manager::maximize_window(target_app.as_ref(), verbose)?;
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
                    matching::find_app(&app_name, &running_apps, config.matching.fuzzy_threshold);

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
                            config.behavior.launch_if_not_running,
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

        Commands::RecordShortcut {
            action: _,
            app: _,
            launch: _,
            no_launch: _,
            yes: _,
        } => {
            println!("Record shortcut command not yet implemented");
            println!("This will capture keyboard input and output a shortcut string.");
            Ok(())
        }

        Commands::Daemon { command } => match command {
            DaemonCommands::Start => {
                println!("Starting daemon...");
                println!("Daemon start not yet implemented");
                Ok(())
            }
            DaemonCommands::Stop => {
                println!("Stopping daemon...");
                println!("Daemon stop not yet implemented");
                Ok(())
            }
            DaemonCommands::Status => {
                println!("Daemon status not yet implemented");
                Ok(())
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
        },

        Commands::ListDisplays => {
            display::print_displays()?;
            Ok(())
        }

        Commands::ListWindows => {
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
