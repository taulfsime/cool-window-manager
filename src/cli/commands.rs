use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
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

            let match_result =
                matching::find_app(&app, &running_apps, config.settings.fuzzy_threshold);

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
                    let should_launch =
                        config::should_launch(launch, no_launch, None, config.settings.launch);

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
                size.parse()
                    .map_err(|_| anyhow!("Invalid size '{}'. Use a number 1-100 or 'full'", size))?
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
            let json =
                serde_json::to_string_pretty(&shortcut).context("Failed to serialize shortcut")?;
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
                let config = config::load()?;
                let json =
                    serde_json::to_string_pretty(&config).context("Failed to serialize config")?;
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
                let json =
                    serde_json::to_string_pretty(&config).context("Failed to serialize config")?;
                println!("{}", json);
                Ok(())
            }
            ConfigCommands::Verify => {
                let path = config::get_config_path();
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
                for title in &app.titles {
                    println!("    - {}", title);
                }
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
            use crate::config;
            use crate::installer::{check_for_updates, perform_update};
            use crate::version::Version;

            let mut config = config::load()?;

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
                        config::save(&config)?;
                    }
                }
                None => {
                    println!("You are on the latest version");

                    // update last check time
                    config.settings.update.last_check = Some(chrono::Utc::now());
                    config::save(&config)?;
                }
            }

            Ok(())
        }

        Commands::Spotlight { command } => match command {
            SpotlightCommands::Install { name, force } => {
                let config = config::load()?;

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
