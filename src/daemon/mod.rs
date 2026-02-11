pub mod app_watcher;
pub mod hotkeys;
pub mod ipc;
mod launchd;

use anyhow::{anyhow, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::config::{self, should_launch, Config};
use crate::window::{manager, matching};

use hotkeys::Hotkey;
use ipc::{get_pid_file_path, is_daemon_running, remove_pid_file, write_pid_file};
pub use launchd::{install, uninstall};

static DAEMON_SHOULD_STOP: AtomicBool = AtomicBool::new(false);
static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

fn log(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{}] {}", timestamp, msg);

    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "{}", line);
            let _ = file.flush();
            return;
        }
    }

    println!("{}", line);
}

fn log_err(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{}] ERROR: {}", timestamp, msg);

    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "{}", line);
            let _ = file.flush();
            return;
        }
    }

    eprintln!("{}", line);
}

fn setup_logging(log_path: Option<String>) -> Result<()> {
    if let Some(path) = log_path {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| anyhow!("Failed to open log file '{}': {}", path, e))?;

        if let Ok(mut guard) = LOG_FILE.lock() {
            *guard = Some(file);
        }
    }
    Ok(())
}

/// Start the daemon in the foreground (blocking)
pub fn start_foreground(log_path: Option<String>) -> Result<()> {
    if is_daemon_running() {
        return Err(anyhow!("Daemon is already running"));
    }

    setup_logging(log_path)?;

    // write PID file
    write_pid_file()?;

    // set up signal handler for graceful shutdown
    setup_signal_handlers()?;

    log("cwm daemon starting...");

    // load config
    let config = config::load()?;

    if config.shortcuts.is_empty() {
        log("No shortcuts configured. Add shortcuts with 'cwm record-shortcut'");
    } else {
        log(&format!("Loaded {} shortcut(s)", config.shortcuts.len()));
    }

    if config.app_rules.is_empty() {
        log("No app rules configured");
    } else {
        log(&format!("Loaded {} app rule(s)", config.app_rules.len()));
        for rule in &config.app_rules {
            log(&format!("  {} -> {}", rule.app, rule.action));
        }
    }

    // parse shortcuts into hotkeys
    let shortcuts = parse_shortcuts(&config)?;

    let has_shortcuts = !shortcuts.is_empty();
    let has_app_rules = !config.app_rules.is_empty();

    if !has_shortcuts && !has_app_rules {
        log("No shortcuts or app rules to listen for");
        remove_pid_file()?;
        return Ok(());
    }

    if has_shortcuts {
        for (hotkey, action) in &shortcuts {
            log(&format!("  {} -> {}", hotkey, action));
        }
    }

    // start app watcher if we have rules
    if has_app_rules {
        let config_for_watcher = config.clone();
        let global_delay = config.settings.delay_ms;
        app_watcher::start_watching(config.app_rules.clone(), move |rule, _pid| {
            let delay = rule.delay_ms.unwrap_or(global_delay);
            log(&format!(
                "App '{}' launched, executing: {} (delay: {}ms)",
                rule.app_name, rule.action, delay
            ));
            // delay to let the window appear
            std::thread::sleep(std::time::Duration::from_millis(delay));
            if let Err(e) =
                execute_action_for_app(&rule.action, &rule.app_name, &config_for_watcher)
            {
                log_err(&format!(
                    "Failed to execute '{}' for '{}': {}",
                    rule.action, rule.app_name, e
                ));
            }
        })?;
        log("Watching for app launches...");
    }

    if has_shortcuts {
        log("Listening for hotkeys... (Ctrl+C to stop)");
    } else {
        log("Watching for app launches... (Ctrl+C to stop)");
    }

    // clone config for the callback
    let config_for_callback = config.clone();

    // start the hotkey listener (this runs the main run loop)
    // even with no shortcuts, we need the run loop for app watcher notifications
    hotkeys::start_hotkey_listener(shortcuts, move |action, hotkey| {
        log(&format!("Hotkey triggered: {} -> {}", hotkey, action));
        if let Err(e) = execute_action(action, &config_for_callback) {
            log_err(&format!("Failed to execute '{}': {}", action, e));
        }
    })?;

    // cleanup
    app_watcher::stop_watching();
    remove_pid_file()?;
    log("Daemon stopped.");

    Ok(())
}

/// Start the daemon in the background (daemonized)
pub fn start(log_path: Option<String>) -> Result<()> {
    if is_daemon_running() {
        return Err(anyhow!("Daemon is already running"));
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let exe = std::env::current_exe()?;

        let mut cmd = Command::new(&exe);
        cmd.arg("daemon").arg("run-foreground");

        if let Some(ref path) = log_path {
            cmd.arg("--log").arg(path);
        }

        let child = cmd
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        println!("Daemon started with PID {}", child.id());
        if let Some(path) = log_path {
            println!("Logging to: {}", path);
        }
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

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()?;

        if status.success() {
            println!("Sent stop signal to daemon (PID {})", pid);

            std::thread::sleep(std::time::Duration::from_millis(500));

            if !is_daemon_running() {
                println!("Daemon stopped");
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

fn parse_shortcuts(config: &Config) -> Result<Vec<(Hotkey, String)>> {
    let mut result = Vec::new();

    for shortcut in &config.shortcuts {
        match Hotkey::parse(&shortcut.keys) {
            Ok(hotkey) => {
                let action = if let Some(ref app) = shortcut.app {
                    format!("{}:{}", shortcut.action, app)
                } else {
                    shortcut.action.clone()
                };
                result.push((hotkey, action));
            }
            Err(e) => {
                log_err(&format!("Invalid shortcut '{}': {}", shortcut.keys, e));
            }
        }
    }

    Ok(result)
}

fn execute_action(action: &str, config: &Config) -> Result<()> {
    let (action_type, action_arg) = if let Some(idx) = action.find(':') {
        (&action[..idx], Some(&action[idx + 1..]))
    } else {
        (action, None)
    };

    match action_type {
        "focus" => {
            let app_name = action_arg.ok_or_else(|| anyhow!("focus action requires app name"))?;

            let shortcut_launch = find_shortcut_launch(config, action);

            let running_apps = matching::get_running_apps()?;
            let match_result =
                matching::find_app(app_name, &running_apps, config.settings.fuzzy_threshold);

            match match_result {
                Some(result) => {
                    manager::focus_app(&result.app, false)?;
                }
                None => {
                    let do_launch =
                        should_launch(false, false, shortcut_launch, config.settings.launch);
                    if do_launch {
                        manager::launch_app(app_name, false)?;
                    }
                }
            }
        }
        "maximize" => {
            let target_app = if let Some(app_name) = action_arg {
                let running_apps = matching::get_running_apps()?;
                matching::find_app(app_name, &running_apps, config.settings.fuzzy_threshold)
                    .map(|r| r.app)
            } else {
                None
            };

            manager::maximize_app(target_app.as_ref(), false)?;
        }
        "move_display" => {
            let target_str = action_arg.ok_or_else(|| anyhow!("move_display requires target"))?;

            let (display_target_str, app_name) = if let Some(idx) = target_str.find(':') {
                (&target_str[..idx], Some(&target_str[idx + 1..]))
            } else {
                (target_str, None)
            };

            let display_target = crate::display::DisplayTarget::parse(display_target_str)?;

            let target_app = if let Some(name) = app_name {
                let running_apps = matching::get_running_apps()?;
                matching::find_app(name, &running_apps, config.settings.fuzzy_threshold)
                    .map(|r| r.app)
            } else {
                None
            };

            manager::move_to_display(target_app.as_ref(), &display_target, false)?;
        }
        "resize" => {
            let size_str = action_arg.ok_or_else(|| anyhow!("resize requires size"))?;

            // parse size:app or just size
            let (percent_str, app_name) = if let Some(idx) = size_str.find(':') {
                (&size_str[..idx], Some(&size_str[idx + 1..]))
            } else {
                (size_str, None)
            };

            let percent: u32 = if percent_str.eq_ignore_ascii_case("full") {
                100
            } else {
                percent_str.parse().map_err(|_| {
                    anyhow!(
                        "Invalid size '{}'. Use a number 1-100 or 'full'",
                        percent_str
                    )
                })?
            };

            let target_app = if let Some(name) = app_name {
                let running_apps = matching::get_running_apps()?;
                matching::find_app(name, &running_apps, config.settings.fuzzy_threshold)
                    .map(|r| r.app)
            } else {
                None
            };

            manager::resize_app(target_app.as_ref(), percent, false)?;
        }
        _ => {
            return Err(anyhow!("Unknown action: {}", action_type));
        }
    }

    Ok(())
}

/// Execute an action for an app by name (used by app watcher)
fn execute_action_for_app(action: &str, app_name: &str, config: &Config) -> Result<()> {
    // find the app by name in running apps
    let running_apps = matching::get_running_apps()?;
    let match_result = matching::find_app(app_name, &running_apps, config.settings.fuzzy_threshold);

    let target_app = match match_result {
        Some(result) => result.app,
        None => {
            return Err(anyhow!("Could not find running application '{}'", app_name));
        }
    };

    execute_action_for_app_info(action, &target_app, config)
}

/// Execute an action for a specific app (used by app watcher)
fn execute_action_for_app_info(
    action: &str,
    target_app: &matching::AppInfo,
    config: &Config,
) -> Result<()> {
    let (action_type, action_arg) = if let Some(idx) = action.find(':') {
        (&action[..idx], Some(&action[idx + 1..]))
    } else {
        (action, None)
    };

    // first, try to activate the app to ensure its windows are accessible
    // this helps with apps that don't expose windows until focused
    if let Err(e) = manager::focus_app(target_app, false) {
        log(&format!("Note: Could not focus app before action: {}", e));
    }

    // retry logic with exponential backoff from config
    let max_retries = config.settings.retry.count;
    let initial_delay = config.settings.retry.delay_ms;
    let backoff = config.settings.retry.backoff;

    let mut last_error = None;
    let mut current_delay = initial_delay as f64;

    for attempt in 0..max_retries {
        let result = match action_type {
            "focus" => manager::focus_app(target_app, false),
            "maximize" => manager::maximize_app(Some(target_app), false),
            "move_display" => {
                let target_str = match action_arg {
                    Some(s) => s,
                    None => return Err(anyhow!("move_display requires target")),
                };
                let display_target = crate::display::DisplayTarget::parse(target_str)?;
                manager::move_to_display(Some(target_app), &display_target, false)
            }
            "resize" => {
                let size_str = match action_arg {
                    Some(s) => s,
                    None => return Err(anyhow!("resize requires size")),
                };
                let percent: u32 = if size_str.eq_ignore_ascii_case("full") {
                    100
                } else {
                    size_str.parse().map_err(|_| {
                        anyhow!("Invalid size '{}'. Use a number 1-100 or 'full'", size_str)
                    })?
                };
                manager::resize_app(Some(target_app), percent, false)
            }
            _ => {
                return Err(anyhow!("Unknown action: {}", action_type));
            }
        };

        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                let err_str = e.to_string();
                // retry on "no windows" or "attribute unsupported" errors
                if err_str.contains("no windows")
                    || err_str.contains("-25204")
                    || err_str.contains("Application has no windows")
                {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        let delay_ms = current_delay as u64;
                        log(&format!(
                            "Window not ready, retrying in {}ms (attempt {}/{})",
                            delay_ms,
                            attempt + 1,
                            max_retries
                        ));
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        current_delay *= backoff;
                        continue;
                    }
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Failed after {} retries", max_retries)))
}

fn find_shortcut_launch(config: &Config, action: &str) -> Option<bool> {
    for shortcut in &config.shortcuts {
        let shortcut_action = if let Some(ref app) = shortcut.app {
            format!("{}:{}", shortcut.action, app)
        } else {
            shortcut.action.clone()
        };

        if shortcut_action == action {
            return shortcut.launch;
        }
    }
    None
}

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
    app_watcher::stop_watching();
    hotkeys::stop_hotkey_listener();
}
