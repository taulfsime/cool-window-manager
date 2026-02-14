pub mod app_watcher;
pub mod hotkeys;
pub mod ipc;
mod launchd;

use anyhow::{anyhow, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::{self, should_launch, Config};
use crate::window::{manager, matching};

use hotkeys::Hotkey;
use ipc::{
    format_error, format_error_response, format_success_response, get_pid_file_path,
    get_socket_path, is_daemon_running, remove_pid_file, remove_socket_file, write_pid_file,
    IpcRequest,
};
pub use launchd::{install, uninstall};

use crate::cli::exit_codes;

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
    } else if has_app_rules {
        log("Watching for app launches... (Ctrl+C to stop)");
    } else {
        log("Listening for IPC commands... (Ctrl+C to stop)");
    }

    // clone config for the callback
    let config_for_callback = config.clone();
    let config_for_socket = Arc::new(config.clone());

    // start socket listener in a separate thread
    let socket_config = Arc::clone(&config_for_socket);
    let socket_handle = std::thread::spawn(move || {
        if let Err(e) = start_socket_listener(socket_config) {
            log_err(&format!("Socket listener error: {}", e));
        }
    });
    log(&format!(
        "IPC socket listening on {}",
        get_socket_path().display()
    ));

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
    stop_socket_listener();
    let _ = socket_handle.join();
    remove_socket_file()?;
    remove_pid_file()?;
    log("Daemon stopped.");

    Ok(())
}

/// Start the daemon in the background (daemonized)
pub fn start(log_path: Option<String>) -> Result<()> {
    use std::process::Command;

    if is_daemon_running() {
        return Err(anyhow!("Daemon is already running"));
    }

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

    Ok(())
}

/// Stop the running daemon
pub fn stop() -> Result<()> {
    use std::process::Command;

    if !is_daemon_running() {
        return Err(anyhow!("Daemon is not running"));
    }

    let pid_path = get_pid_file_path();
    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: i32 = pid_str.trim().parse()?;

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

            manager::move_to_display_with_aliases(
                target_app.as_ref(),
                &display_target,
                false,
                &config.display_aliases,
            )?;
        }
        "resize" => {
            use crate::window::ResizeTarget;

            let size_str = action_arg.ok_or_else(|| anyhow!("resize requires size"))?;

            // parse size:app or just size
            let (target_str, app_name) = if let Some(idx) = size_str.find(':') {
                (&size_str[..idx], Some(&size_str[idx + 1..]))
            } else {
                (size_str, None)
            };

            let resize_target = ResizeTarget::parse(target_str)?;

            let target_app = if let Some(name) = app_name {
                let running_apps = matching::get_running_apps()?;
                matching::find_app(name, &running_apps, config.settings.fuzzy_threshold)
                    .map(|r| r.app)
            } else {
                None
            };

            manager::resize_app(target_app.as_ref(), &resize_target, false, false)?;
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
                use crate::window::ResizeTarget;

                let size_str = match action_arg {
                    Some(s) => s,
                    None => return Err(anyhow!("resize requires size")),
                };
                let resize_target = ResizeTarget::parse(size_str)?;
                manager::resize_app(Some(target_app), &resize_target, false, false).map(|_| ())
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
    unsafe {
        libc::signal(libc::SIGTERM, handle_signal as *const () as usize);
        libc::signal(libc::SIGINT, handle_signal as *const () as usize);
    }
    Ok(())
}

extern "C" fn handle_signal(_sig: libc::c_int) {
    DAEMON_SHOULD_STOP.store(true, Ordering::SeqCst);
    app_watcher::stop_watching();
    hotkeys::stop_hotkey_listener();
    stop_socket_listener();
}

// socket listener state
static SOCKET_SHOULD_STOP: AtomicBool = AtomicBool::new(false);

/// Start the Unix socket listener for IPC
fn start_socket_listener(config: Arc<Config>) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;

    let socket_path = get_socket_path();

    // ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // remove stale socket file
    let _ = std::fs::remove_file(&socket_path);

    let listener =
        UnixListener::bind(&socket_path).map_err(|e| anyhow!("Failed to bind socket: {}", e))?;

    // set non-blocking so we can check for stop signal
    listener.set_nonblocking(true)?;

    SOCKET_SHOULD_STOP.store(false, Ordering::SeqCst);

    while !SOCKET_SHOULD_STOP.load(Ordering::SeqCst) && !DAEMON_SHOULD_STOP.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                // set blocking for this connection
                stream.set_nonblocking(false)?;

                // read command
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    let line = line.trim();
                    if !line.is_empty() {
                        // parse request and handle it
                        if let Some(response) = handle_ipc_message(line, &config) {
                            let _ = stream.write_all(response.as_bytes());
                            let _ = stream.write_all(b"\n");
                        }
                        // if None, it was a notification - no response needed
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // no connection available, sleep briefly and check again
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                log_err(&format!("Socket accept error: {}", e));
            }
        }
    }

    // cleanup
    let _ = std::fs::remove_file(&socket_path);

    Ok(())
}

/// Handle an IPC message and return the response string (or None for notifications)
fn handle_ipc_message(line: &str, config: &Config) -> Option<String> {
    // parse the request (auto-detects format)
    let request = match IpcRequest::parse(line) {
        Ok(req) => req,
        Err(e) => {
            // can't parse - create a minimal text request for error response
            let fake_req = IpcRequest::parse("error").unwrap();
            return format_error(&fake_req, &format!("Invalid request: {}", e));
        }
    };

    log(&format!(
        "IPC request: {} {:?} (format: {:?})",
        request.method, request.params, request.format
    ));

    // handle the request and get result
    let result = handle_ipc_request(&request, config);

    // format response based on input format
    match result {
        Ok(value) => format_success_response(&request, value),
        Err((code, msg)) => format_error_response(&request, code, &msg),
    }
}

/// Handle a parsed IPC request
/// Returns Ok(json_value) on success, Err((code, message)) on error
fn handle_ipc_request(
    request: &IpcRequest,
    config: &Config,
) -> Result<serde_json::Value, (i32, String)> {
    match request.method.as_str() {
        "ping" => Ok(serde_json::json!("pong")),

        "status" => Ok(serde_json::json!({
            "running": true,
            "pid": std::process::id(),
            "shortcuts": config.shortcuts.len(),
            "app_rules": config.app_rules.len(),
            "socket": get_socket_path().display().to_string(),
        })),

        "focus" => {
            let app = request.params.get("app").ok_or_else(|| {
                (
                    exit_codes::INVALID_ARGS,
                    "focus requires 'app' parameter".to_string(),
                )
            })?;

            let action = format!("focus:{}", app);
            execute_action(&action, config)
                .map(|()| serde_json::json!({"message": format!("Focused {}", app), "app": app}))
                .map_err(|e| (exit_codes::APP_NOT_FOUND, e.to_string()))
        }

        "maximize" => {
            let action = match request.params.get("app") {
                Some(app) => format!("maximize:{}", app),
                None => "maximize".to_string(),
            };
            execute_action(&action, config)
                .map(|()| serde_json::json!({"message": "Window maximized"}))
                .map_err(|e| (exit_codes::WINDOW_NOT_FOUND, e.to_string()))
        }

        "resize" => {
            let to = request.params.get("to").ok_or_else(|| {
                (
                    exit_codes::INVALID_ARGS,
                    "resize requires 'to' parameter".to_string(),
                )
            })?;

            let action = match request.params.get("app") {
                Some(app) => format!("resize:{}:{}", to, app),
                None => format!("resize:{}", to),
            };
            execute_action(&action, config)
                .map(|()| serde_json::json!({"message": format!("Window resized to {}", to), "to": to}))
                .map_err(|e| (exit_codes::WINDOW_NOT_FOUND, e.to_string()))
        }

        "move_display" => {
            let target = request.params.get("target").ok_or_else(|| {
                (
                    exit_codes::INVALID_ARGS,
                    "move_display requires 'target' parameter".to_string(),
                )
            })?;

            let action = match request.params.get("app") {
                Some(app) => format!("move_display:{}:{}", target, app),
                None => format!("move_display:{}", target),
            };
            execute_action(&action, config)
                .map(|()| serde_json::json!({"message": format!("Window moved to display {}", target), "target": target}))
                .map_err(|e| (exit_codes::DISPLAY_NOT_FOUND, e.to_string()))
        }

        "list_apps" => matching::get_running_apps()
            .map(|apps| {
                let app_list: Vec<serde_json::Value> = apps
                    .iter()
                    .map(|app| {
                        serde_json::json!({
                            "name": app.name,
                            "pid": app.pid,
                            "bundle_id": app.bundle_id,
                            "titles": app.titles,
                        })
                    })
                    .collect();
                serde_json::json!({ "apps": app_list })
            })
            .map_err(|e| (exit_codes::ERROR, e.to_string())),

        "list_displays" => crate::display::get_displays()
            .map(|displays| {
                let display_list: Vec<serde_json::Value> = displays
                    .iter()
                    .map(|d| {
                        serde_json::json!({
                            "index": d.index,
                            "name": d.name,
                            "width": d.width,
                            "height": d.height,
                            "is_main": d.is_main,
                            "is_builtin": d.is_builtin,
                        })
                    })
                    .collect();
                serde_json::json!({ "displays": display_list })
            })
            .map_err(|e| (exit_codes::ERROR, format!("{}", e))),

        "action" => {
            // execute a raw action string (legacy format)
            let action = request.params.get("action").ok_or_else(|| {
                (
                    exit_codes::INVALID_ARGS,
                    "action requires 'action' parameter".to_string(),
                )
            })?;

            execute_action(action, config)
                .map(|()| serde_json::json!({"message": "Action executed"}))
                .map_err(|e| (exit_codes::ERROR, e.to_string()))
        }

        _ => Err((
            exit_codes::INVALID_ARGS,
            format!("Unknown method: {}", request.method),
        )),
    }
}

fn stop_socket_listener() {
    SOCKET_SHOULD_STOP.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Settings, Shortcut};

    fn create_test_config(shortcuts: Vec<Shortcut>) -> Config {
        Config {
            shortcuts,
            app_rules: vec![],
            spotlight: vec![],
            display_aliases: std::collections::HashMap::new(),
            settings: Settings::default(),
            schema: None,
        }
    }

    // ========================================================================
    // parse_shortcuts tests
    // ========================================================================

    #[test]
    fn test_parse_shortcuts_empty() {
        let config = create_test_config(vec![]);
        let result = parse_shortcuts(&config).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_shortcuts_single_valid() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+s".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
        }]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 1);

        let (hotkey, action) = &result[0];
        assert!(hotkey.modifiers.ctrl);
        assert!(hotkey.modifiers.alt);
        assert_eq!(hotkey.keys, vec!["s"]);
        assert_eq!(action, "focus:Safari");
    }

    #[test]
    fn test_parse_shortcuts_without_app() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+m".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
        }]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 1);

        let (_, action) = &result[0];
        assert_eq!(action, "maximize");
    }

    #[test]
    fn test_parse_shortcuts_multiple() {
        let config = create_test_config(vec![
            Shortcut {
                keys: "ctrl+alt+s".to_string(),
                action: "focus".to_string(),
                app: Some("Safari".to_string()),
                launch: Some(true),
            },
            Shortcut {
                keys: "ctrl+alt+m".to_string(),
                action: "maximize".to_string(),
                app: None,
                launch: None,
            },
            Shortcut {
                keys: "ctrl+alt+n".to_string(),
                action: "move_display".to_string(),
                app: Some("next".to_string()),
                launch: None,
            },
        ]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_parse_shortcuts_invalid_key_skipped() {
        let config = create_test_config(vec![
            Shortcut {
                keys: "ctrl+alt+".to_string(), // invalid - no key
                action: "focus".to_string(),
                app: Some("Safari".to_string()),
                launch: None,
            },
            Shortcut {
                keys: "ctrl+alt+m".to_string(), // valid
                action: "maximize".to_string(),
                app: None,
                launch: None,
            },
        ]);

        let result = parse_shortcuts(&config).unwrap();
        // invalid shortcut should be skipped, valid one should be included
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "maximize");
    }

    // ========================================================================
    // find_shortcut_launch tests
    // ========================================================================

    #[test]
    fn test_find_shortcut_launch_found_with_launch_true() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+s".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
        }]);

        let result = find_shortcut_launch(&config, "focus:Safari");
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_find_shortcut_launch_found_with_launch_false() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+s".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(false),
        }]);

        let result = find_shortcut_launch(&config, "focus:Safari");
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_find_shortcut_launch_found_without_launch_field() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+s".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
        }]);

        let result = find_shortcut_launch(&config, "focus:Safari");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_shortcut_launch_not_found() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+s".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
        }]);

        let result = find_shortcut_launch(&config, "focus:Chrome");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_shortcut_launch_action_without_app() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+m".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
        }]);

        let result = find_shortcut_launch(&config, "maximize");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_shortcut_launch_multiple_shortcuts() {
        let config = create_test_config(vec![
            Shortcut {
                keys: "ctrl+alt+s".to_string(),
                action: "focus".to_string(),
                app: Some("Safari".to_string()),
                launch: Some(true),
            },
            Shortcut {
                keys: "ctrl+alt+c".to_string(),
                action: "focus".to_string(),
                app: Some("Chrome".to_string()),
                launch: Some(false),
            },
        ]);

        assert_eq!(find_shortcut_launch(&config, "focus:Safari"), Some(true));
        assert_eq!(find_shortcut_launch(&config, "focus:Chrome"), Some(false));
        assert_eq!(find_shortcut_launch(&config, "focus:Firefox"), None);
    }

    // ========================================================================
    // parse_shortcuts edge cases
    // ========================================================================

    #[test]
    fn test_parse_shortcuts_with_move_display_action() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+n".to_string(),
            action: "move_display".to_string(),
            app: Some("next".to_string()),
            launch: None,
        }]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "move_display:next");
    }

    #[test]
    fn test_parse_shortcuts_with_resize_action() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+r".to_string(),
            action: "resize".to_string(),
            app: Some("80".to_string()),
            launch: None,
        }]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "resize:80");
    }

    #[test]
    fn test_parse_shortcuts_all_modifiers() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+shift+cmd+s".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
        }]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 1);

        let (hotkey, _) = &result[0];
        assert!(hotkey.modifiers.ctrl);
        assert!(hotkey.modifiers.alt);
        assert!(hotkey.modifiers.shift);
        assert!(hotkey.modifiers.cmd);
    }

    #[test]
    fn test_parse_shortcuts_function_key() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+f1".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
        }]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.keys, vec!["f1"]);
    }

    #[test]
    fn test_parse_shortcuts_special_keys() {
        let config = create_test_config(vec![
            Shortcut {
                keys: "ctrl+space".to_string(),
                action: "maximize".to_string(),
                app: None,
                launch: None,
            },
            Shortcut {
                keys: "ctrl+tab".to_string(),
                action: "focus".to_string(),
                app: Some("Safari".to_string()),
                launch: None,
            },
        ]);

        let result = parse_shortcuts(&config).unwrap();
        assert_eq!(result.len(), 2);
    }

    // ========================================================================
    // find_shortcut_launch edge cases
    // ========================================================================

    #[test]
    fn test_find_shortcut_launch_case_sensitive_action() {
        let config = create_test_config(vec![Shortcut {
            keys: "ctrl+alt+s".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
        }]);

        // action matching is case-sensitive
        assert_eq!(find_shortcut_launch(&config, "focus:Safari"), Some(true));
        assert_eq!(find_shortcut_launch(&config, "Focus:Safari"), None);
        assert_eq!(find_shortcut_launch(&config, "focus:safari"), None);
    }

    #[test]
    fn test_find_shortcut_launch_empty_config() {
        let config = create_test_config(vec![]);

        assert_eq!(find_shortcut_launch(&config, "focus:Safari"), None);
        assert_eq!(find_shortcut_launch(&config, "maximize"), None);
    }

    #[test]
    fn test_find_shortcut_launch_first_match_wins() {
        let config = create_test_config(vec![
            Shortcut {
                keys: "ctrl+alt+s".to_string(),
                action: "focus".to_string(),
                app: Some("Safari".to_string()),
                launch: Some(true),
            },
            Shortcut {
                keys: "ctrl+shift+s".to_string(),
                action: "focus".to_string(),
                app: Some("Safari".to_string()),
                launch: Some(false),
            },
        ]);

        // first matching shortcut wins
        assert_eq!(find_shortcut_launch(&config, "focus:Safari"), Some(true));
    }

    // ========================================================================
    // IpcRequest handling tests (via handle_ipc_request)
    // ========================================================================

    #[test]
    fn test_handle_ipc_request_ping() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "ping"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!("pong"));
    }

    #[test]
    fn test_handle_ipc_request_status() {
        let config = create_test_config(vec![
            Shortcut {
                keys: "ctrl+alt+s".to_string(),
                action: "focus".to_string(),
                app: Some("Safari".to_string()),
                launch: None,
            },
            Shortcut {
                keys: "ctrl+alt+m".to_string(),
                action: "maximize".to_string(),
                app: None,
                launch: None,
            },
        ]);
        let request = IpcRequest::parse(r#"{"method": "status"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["running"], true);
        assert_eq!(value["shortcuts"], 2);
        assert_eq!(value["app_rules"], 0);
    }

    #[test]
    fn test_handle_ipc_request_unknown_method() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "unknown_method"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_err());

        let (code, msg) = result.unwrap_err();
        assert_eq!(code, exit_codes::INVALID_ARGS);
        assert!(msg.contains("Unknown method"));
    }

    #[test]
    fn test_handle_ipc_request_focus_missing_app() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "focus"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_err());

        let (code, msg) = result.unwrap_err();
        assert_eq!(code, exit_codes::INVALID_ARGS);
        assert!(msg.contains("app"));
    }

    #[test]
    fn test_handle_ipc_request_resize_missing_to() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "resize"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_err());

        let (code, msg) = result.unwrap_err();
        assert_eq!(code, exit_codes::INVALID_ARGS);
        assert!(msg.contains("to"));
    }

    #[test]
    fn test_handle_ipc_request_move_display_missing_target() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "move_display"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_err());

        let (code, msg) = result.unwrap_err();
        assert_eq!(code, exit_codes::INVALID_ARGS);
        assert!(msg.contains("target"));
    }

    #[test]
    fn test_handle_ipc_request_action_missing_action() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "action"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_err());

        let (code, msg) = result.unwrap_err();
        assert_eq!(code, exit_codes::INVALID_ARGS);
        assert!(msg.contains("action"));
    }
}
