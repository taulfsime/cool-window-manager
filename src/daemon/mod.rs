pub mod hotkeys;
pub mod ipc;

use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::{self, should_launch, Config};
use crate::window::{manager, matching};

use hotkeys::Hotkey;
use ipc::{get_pid_file_path, is_daemon_running, remove_pid_file, write_pid_file};

static DAEMON_SHOULD_STOP: AtomicBool = AtomicBool::new(false);

/// Start the daemon in the foreground (blocking)
pub fn start_foreground() -> Result<()> {
    if is_daemon_running() {
        return Err(anyhow!("Daemon is already running"));
    }

    // write PID file
    write_pid_file()?;

    // set up signal handler for graceful shutdown
    setup_signal_handlers()?;

    println!("cwm daemon starting...");

    // load config
    let config = config::load()?;

    if config.shortcuts.is_empty() {
        println!("No shortcuts configured. Add shortcuts with 'cwm record-shortcut'");
    } else {
        println!("Loaded {} shortcut(s)", config.shortcuts.len());
    }

    // parse shortcuts into hotkeys
    let shortcuts = parse_shortcuts(&config)?;

    if shortcuts.is_empty() {
        println!("No valid shortcuts to listen for");
        remove_pid_file()?;
        return Ok(());
    }

    for (hotkey, action) in &shortcuts {
        println!("  {} -> {}", hotkey.to_string(), action);
    }

    println!("\nListening for hotkeys... (Ctrl+C to stop)");

    // clone config for the callback
    let config_for_callback = config.clone();

    // start the hotkey listener
    hotkeys::start_hotkey_listener(shortcuts, move |action, _hotkey| {
        if let Err(e) = execute_action(action, &config_for_callback) {
            eprintln!("Error executing action '{}': {}", action, e);
        }
    })?;

    // cleanup
    remove_pid_file()?;
    println!("\nDaemon stopped.");

    Ok(())
}

/// Start the daemon in the background (daemonized)
pub fn start() -> Result<()> {
    if is_daemon_running() {
        return Err(anyhow!("Daemon is already running"));
    }

    // fork the process
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // get the current executable path
        let exe = std::env::current_exe()?;

        // spawn a detached child process that runs the daemon
        let child = Command::new(&exe)
            .arg("daemon")
            .arg("run-foreground")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        println!("Daemon started with PID {}", child.id());
        println!("Use 'cwm daemon status' to check status");
        println!("Use 'cwm daemon stop' to stop");
    }

    #[cfg(not(target_os = "macos"))]
    {
        return Err(anyhow!("Daemon is only supported on macOS"));
    }

    Ok(())
}

/// Stop the running daemon
pub fn stop() -> Result<()> {
    if !is_daemon_running() {
        return Err(anyhow!("Daemon is not running"));
    }

    let pid_path = get_pid_file_path();
    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: i32 = pid_str.trim().parse()?;

    // send SIGTERM to the daemon
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()?;

        if status.success() {
            println!("Sent stop signal to daemon (PID {})", pid);

            // wait a bit for it to stop
            std::thread::sleep(std::time::Duration::from_millis(500));

            if !is_daemon_running() {
                println!("Daemon stopped");
                // clean up PID file if it still exists
                let _ = remove_pid_file();
            } else {
                println!("Daemon may still be stopping...");
            }
        } else {
            return Err(anyhow!("Failed to send stop signal to daemon"));
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        return Err(anyhow!("Daemon is only supported on macOS"));
    }

    Ok(())
}

/// Check daemon status
pub fn status() -> Result<bool> {
    let running = is_daemon_running();

    if running {
        let pid_path = get_pid_file_path();
        if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
            println!("Daemon is running (PID {})", pid_str.trim());
        } else {
            println!("Daemon is running");
        }
    } else {
        println!("Daemon is not running");
    }

    Ok(running)
}

/// Parse shortcuts from config into Hotkey structs
fn parse_shortcuts(config: &Config) -> Result<Vec<(Hotkey, String)>> {
    let mut result = Vec::new();

    for shortcut in &config.shortcuts {
        match Hotkey::parse(&shortcut.keys) {
            Ok(hotkey) => {
                // build action string including app if present
                let action = if let Some(ref app) = shortcut.app {
                    format!("{}:{}", shortcut.action, app)
                } else {
                    shortcut.action.clone()
                };
                result.push((hotkey, action));
            }
            Err(e) => {
                eprintln!("Warning: Invalid shortcut '{}': {}", shortcut.keys, e);
            }
        }
    }

    Ok(result)
}

/// Execute an action triggered by a hotkey
fn execute_action(action: &str, config: &Config) -> Result<()> {
    // parse action string
    let (action_type, action_arg) = if let Some(idx) = action.find(':') {
        (&action[..idx], Some(&action[idx + 1..]))
    } else {
        (action, None)
    };

    match action_type {
        "focus" => {
            let app_name = action_arg.ok_or_else(|| anyhow!("focus action requires app name"))?;

            // find the shortcut config for this action to get launch_if_not_running
            let shortcut_launch = find_shortcut_launch(config, action);

            let running_apps = matching::get_running_apps()?;
            let match_result = matching::find_app(app_name, &running_apps, config.matching.fuzzy_threshold);

            match match_result {
                Some(result) => {
                    manager::focus_app(&result.app, false)?;
                }
                None => {
                    let do_launch = should_launch(false, false, shortcut_launch, config.behavior.launch_if_not_running);
                    if do_launch {
                        manager::launch_app(app_name, false)?;
                    }
                }
            }
        }
        "maximize" => {
            let target_app = if let Some(app_name) = action_arg {
                let running_apps = matching::get_running_apps()?;
                matching::find_app(app_name, &running_apps, config.matching.fuzzy_threshold)
                    .map(|r| r.app)
            } else {
                None
            };

            manager::maximize_window(target_app.as_ref(), false)?;
        }
        "move_display" => {
            let target_str = action_arg.ok_or_else(|| anyhow!("move_display requires target"))?;

            // check if there's an app specified after the target
            let (display_target_str, app_name) = if let Some(idx) = target_str.find(':') {
                (&target_str[..idx], Some(&target_str[idx + 1..]))
            } else {
                (target_str, None)
            };

            let display_target = crate::display::DisplayTarget::parse(display_target_str)?;

            let target_app = if let Some(name) = app_name {
                let running_apps = matching::get_running_apps()?;
                matching::find_app(name, &running_apps, config.matching.fuzzy_threshold)
                    .map(|r| r.app)
            } else {
                None
            };

            manager::move_to_display(target_app.as_ref(), &display_target, false)?;
        }
        _ => {
            return Err(anyhow!("Unknown action: {}", action_type));
        }
    }

    Ok(())
}

/// Find the launch_if_not_running setting for a specific action
fn find_shortcut_launch(config: &Config, action: &str) -> Option<bool> {
    for shortcut in &config.shortcuts {
        let shortcut_action = if let Some(ref app) = shortcut.app {
            format!("{}:{}", shortcut.action, app)
        } else {
            shortcut.action.clone()
        };

        if shortcut_action == action {
            return shortcut.launch_if_not_running;
        }
    }
    None
}

/// Set up signal handlers for graceful shutdown
fn setup_signal_handlers() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        unsafe {
            libc::signal(libc::SIGTERM, handle_signal as *const () as usize);
            libc::signal(libc::SIGINT, handle_signal as *const () as usize);
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
extern "C" fn handle_signal(_sig: libc::c_int) {
    DAEMON_SHOULD_STOP.store(true, Ordering::SeqCst);
    hotkeys::stop_hotkey_listener();
}
