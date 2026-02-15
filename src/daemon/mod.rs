pub mod app_watcher;
pub mod display_watcher;
pub mod events;
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

use events::Event;

use hotkeys::Hotkey;
use ipc::{
    format_error_response, format_success_response, get_pid_file_path, get_socket_path,
    is_daemon_running, remove_pid_file, remove_socket_file, write_pid_file, IpcRequest,
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
        log("No shortcuts configured. Add shortcuts with 'cwm record shortcut'");
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

    // start display watcher for connect/disconnect events
    display_watcher::start_watching(config.display_aliases.clone())?;
    log("Watching for display changes...");

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
    display_watcher::stop_watching();
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
                    // emit app.focused event
                    events::emit(Event::app_focused(
                        result.app.name.clone(),
                        result.app.pid,
                        Some(result.app.titles.clone()),
                        result.describe(),
                    ));
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

            // emit window.maximized event
            if let Some(ref app) = target_app {
                events::emit(Event::window_maximized(
                    app.name.clone(),
                    app.pid,
                    Some(app.titles.clone()),
                ));
            }
        }
        "move" => {
            let arg_str = action_arg.ok_or_else(|| anyhow!("move requires target"))?;

            // parse format: position or display=target or position;display=target or position:app
            // examples: top-left, next, display=next, top-left;display=2, 50%,50%:Safari
            let (position_and_display, app_name) = if let Some(idx) = arg_str.rfind(':') {
                // check if this colon is part of a display= clause or an app separator
                let before_colon = &arg_str[..idx];
                if before_colon.contains("display=")
                    || before_colon.contains('%')
                    || before_colon.contains("px")
                    || before_colon.contains("pt")
                {
                    // colon is app separator
                    (&arg_str[..idx], Some(&arg_str[idx + 1..]))
                } else {
                    // might be display target like "next" or position like "top-left"
                    (arg_str, None)
                }
            } else {
                (arg_str, None)
            };

            // parse position and display from the string
            let (move_target, display_target) = parse_move_action_arg(position_and_display)?;

            let target_app = if let Some(name) = app_name {
                let running_apps = matching::get_running_apps()?;
                matching::find_app(name, &running_apps, config.settings.fuzzy_threshold)
                    .map(|r| r.app)
            } else {
                None
            };

            let (new_x, new_y, _display_index, display_name) = manager::move_window(
                target_app.as_ref(),
                move_target.as_ref(),
                display_target.as_ref(),
                false,
                &config.display_aliases,
            )?;

            // emit window.moved event
            if let Some(ref app) = target_app {
                events::emit(Event::window_moved(
                    app.name.clone(),
                    app.pid,
                    Some(app.titles.clone()),
                    new_x,
                    new_y,
                    Some(display_name),
                ));
            }
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

            let (width, height) =
                manager::resize_app(target_app.as_ref(), &resize_target, false, false)?;

            // emit window.resized event
            if let Some(ref app) = target_app {
                events::emit(Event::window_resized(
                    app.name.clone(),
                    app.pid,
                    Some(app.titles.clone()),
                    width as i32,
                    height as i32,
                ));
            }
        }
        "kill" => {
            let app_name = action_arg.ok_or_else(|| anyhow!("kill action requires app name"))?;

            // check for :force suffix
            let (target_name, force) = if let Some(idx) = app_name.rfind(":force") {
                (&app_name[..idx], true)
            } else {
                (app_name, false)
            };

            let running_apps = matching::get_running_apps()?;
            let match_result =
                matching::find_app(target_name, &running_apps, config.settings.fuzzy_threshold);

            match match_result {
                Some(result) => {
                    let accepted = manager::terminate_app(&result.app, force, false)?;
                    if !accepted {
                        return Err(anyhow!(
                            "{} declined to terminate (may have unsaved changes)",
                            result.app.name
                        ));
                    }
                    // emit app.terminated event
                    events::emit(Event::app_terminated(
                        result.app.name.clone(),
                        result.app.pid,
                        force,
                    ));
                }
                None => {
                    return Err(anyhow!("Application '{}' not found", target_name));
                }
            }
        }
        "close" => {
            let app_name = action_arg.ok_or_else(|| anyhow!("close action requires app name"))?;

            let running_apps = matching::get_running_apps()?;
            let match_result =
                matching::find_app(app_name, &running_apps, config.settings.fuzzy_threshold);

            match match_result {
                Some(result) => {
                    let windows_closed = manager::close_app_windows(&result.app, false)?;
                    // emit window.closed event
                    events::emit(Event::window_closed(
                        result.app.name.clone(),
                        result.app.pid,
                        Some(result.app.titles.clone()),
                        windows_closed,
                    ));
                }
                None => {
                    return Err(anyhow!("Application '{}' not found", app_name));
                }
            }
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
        let result: Result<Option<Event>> = match action_type {
            "focus" => manager::focus_app(target_app, false).map(|()| {
                Some(Event::app_focused(
                    target_app.name.clone(),
                    target_app.pid,
                    Some(target_app.titles.clone()),
                    "exact".to_string(),
                ))
            }),
            "maximize" => manager::maximize_app(Some(target_app), false).map(|()| {
                Some(Event::window_maximized(
                    target_app.name.clone(),
                    target_app.pid,
                    Some(target_app.titles.clone()),
                ))
            }),
            "move" => {
                let arg_str = match action_arg {
                    Some(s) => s,
                    None => return Err(anyhow!("move requires target")),
                };
                let (move_target, display_target) = parse_move_action_arg(arg_str)?;
                manager::move_window(
                    Some(target_app),
                    move_target.as_ref(),
                    display_target.as_ref(),
                    false,
                    &config.display_aliases,
                )
                .map(|(new_x, new_y, _display_index, display_name)| {
                    Some(Event::window_moved(
                        target_app.name.clone(),
                        target_app.pid,
                        Some(target_app.titles.clone()),
                        new_x,
                        new_y,
                        Some(display_name),
                    ))
                })
            }
            "resize" => {
                use crate::window::ResizeTarget;

                let size_str = match action_arg {
                    Some(s) => s,
                    None => return Err(anyhow!("resize requires size")),
                };
                let resize_target = ResizeTarget::parse(size_str)?;
                manager::resize_app(Some(target_app), &resize_target, false, false).map(
                    |(width, height)| {
                        Some(Event::window_resized(
                            target_app.name.clone(),
                            target_app.pid,
                            Some(target_app.titles.clone()),
                            width as i32,
                            height as i32,
                        ))
                    },
                )
            }
            _ => {
                return Err(anyhow!("Unknown action: {}", action_type));
            }
        };

        match result {
            Ok(event) => {
                if let Some(e) = event {
                    events::emit(e);
                }
                return Ok(());
            }
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

/// Parse move action argument string into position and display targets
/// Formats:
/// - "top-left" -> position only
/// - "next" or "prev" or "0" -> display only (simple targets)
/// - "display=next" -> display only (explicit)
/// - "top-left;display=2" -> both position and display (semicolon separates arguments)
/// - "50%,50%" -> position only (percentage, comma separates coordinates)
fn parse_move_action_arg(
    s: &str,
) -> Result<(
    Option<crate::window::manager::MoveTarget>,
    Option<crate::display::DisplayTarget>,
)> {
    use crate::display::DisplayTarget;
    use crate::window::manager::MoveTarget;

    let s = s.trim();

    // check for explicit display= syntax (display only)
    if let Some(display_part) = s.strip_prefix("display=") {
        let display = DisplayTarget::parse(display_part)?;
        return Ok((None, Some(display)));
    }

    // check for combined format using semicolon: position;display=target
    if s.contains(';') {
        let parts: Vec<&str> = s.splitn(2, ';').collect();
        if parts.len() == 2 {
            let position = MoveTarget::parse(parts[0].trim())?;
            let display_part = parts[1].trim();
            let display_value = display_part
                .strip_prefix("display=")
                .unwrap_or(display_part);
            let display = DisplayTarget::parse(display_value)?;
            return Ok((Some(position), Some(display)));
        }
    }

    // try to parse as display target first (for simple targets: next, prev, 0, 1, etc.)
    if let Ok(display) = DisplayTarget::parse(s) {
        // check if it's a simple display target (next, prev, or numeric)
        match s.to_lowercase().as_str() {
            "next" | "prev" => return Ok((None, Some(display))),
            _ if s.chars().all(|c| c.is_ascii_digit()) => return Ok((None, Some(display))),
            _ => {}
        }
    }

    // try to parse as position
    if let Ok(position) = MoveTarget::parse(s) {
        return Ok((Some(position), None));
    }

    // try display target as fallback (for aliases)
    if let Ok(display) = DisplayTarget::parse(s) {
        return Ok((None, Some(display)));
    }

    Err(anyhow!(
        "invalid move target '{}': expected position (top-left, 50%,50%, etc.) or display (next, prev, 0, etc.)",
        s
    ))
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
                        // check if this is a subscribe request
                        if let Ok(request) = IpcRequest::parse(line) {
                            if request.method == "subscribe" {
                                // handle subscription in a separate thread
                                let config_clone = Arc::clone(&config);
                                let request_clone = request.clone();
                                std::thread::spawn(move || {
                                    handle_subscription(stream, request_clone, &config_clone);
                                });
                                continue;
                            }
                        }

                        // normal request-response handling
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

/// Handle a subscription request - keeps connection open and streams events
fn handle_subscription(
    mut stream: std::os::unix::net::UnixStream,
    request: IpcRequest,
    _config: &Config,
) {
    use std::io::Write;
    use std::time::Duration;

    // parse event and app filters from params
    let event_filters: Vec<String> = request
        .params
        .get("events")
        .map(|s| {
            // handle both single string and comma-separated list
            s.split(',').map(|e| e.trim().to_string()).collect()
        })
        .unwrap_or_default();

    let app_filters: Vec<String> = request
        .params
        .get("app")
        .map(|s| s.split(',').map(|a| a.trim().to_string()).collect())
        .unwrap_or_default();

    // expand filters to actual event types for response
    let subscribed = events::EventBus::expand_filters(&event_filters);

    // subscribe to events
    let (sub_id, mut receiver) = events::subscribe(event_filters, app_filters);

    log(&format!(
        "Subscription {} started: {:?}",
        sub_id, subscribed
    ));

    // send success response
    if let Some(response) =
        format_success_response(&request, serde_json::json!({ "subscribed": subscribed }))
    {
        if stream.write_all(response.as_bytes()).is_err() {
            events::unsubscribe(sub_id);
            return;
        }
        if stream.write_all(b"\n").is_err() {
            events::unsubscribe(sub_id);
            return;
        }
        let _ = stream.flush();
    }

    // set a read timeout so we can check for daemon stop
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));

    // stream events to client
    loop {
        // check if daemon is stopping
        if DAEMON_SHOULD_STOP.load(Ordering::SeqCst) || SOCKET_SHOULD_STOP.load(Ordering::SeqCst) {
            break;
        }

        // try to receive an event (non-blocking with timeout)
        match receiver.try_recv() {
            Ok(event) => {
                let notification = event.to_jsonrpc_notification();
                if stream.write_all(notification.as_bytes()).is_err() {
                    break;
                }
                if stream.write_all(b"\n").is_err() {
                    break;
                }
                if stream.flush().is_err() {
                    break;
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                // no event available, sleep briefly
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                // channel closed
                break;
            }
        }
    }

    // cleanup
    events::unsubscribe(sub_id);
    log(&format!("Subscription {} ended", sub_id));
}

/// Handle an IPC message and return the response string (or None for notifications)
fn handle_ipc_message(line: &str, config: &Config) -> Option<String> {
    // parse the request (JSON-RPC only)
    let request = match IpcRequest::parse(line) {
        Ok(req) => req,
        Err(e) => {
            // can't parse - return JSON-RPC error
            // create a minimal error response without a valid request
            let error = crate::cli::output::JsonRpcError::new(
                exit_codes::INVALID_ARGS,
                format!("Invalid request: {}", e),
            );
            return serde_json::to_string(&error).ok();
        }
    };

    log(&format!(
        "IPC request: {} {:?}",
        request.method, request.params
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
    use crate::actions::{execute, ExecutionContext, JsonRpcRequest};

    // handle special "action" method for raw action strings (hotkey-style)
    if request.method == "action" {
        let action = request.params.get("action").ok_or_else(|| {
            (
                exit_codes::INVALID_ARGS,
                "action requires 'action' parameter".to_string(),
            )
        })?;

        return execute_action(action, config)
            .map(|()| serde_json::json!({"message": "Action executed"}))
            .map_err(|e| (exit_codes::ERROR, e.to_string()));
    }

    // convert IpcRequest to JSON string and parse with JsonRpcRequest
    let json_str = serde_json::json!({
        "method": request.method,
        "params": request.params,
        "id": request.id,
    })
    .to_string();

    let json_request =
        JsonRpcRequest::parse(&json_str).map_err(|e| (exit_codes::INVALID_ARGS, e.message))?;

    // convert to Command and execute
    let cmd = json_request.to_command().map_err(|e| (e.code, e.message))?;

    let ctx = ExecutionContext::new(config, false);

    match execute(cmd, &ctx) {
        Ok(result) => {
            // serialize the ActionResult to JSON
            serde_json::to_value(&result).map_err(|e| (exit_codes::ERROR, e.to_string()))
        }
        Err(err) => Err((err.code, err.message)),
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
        let value = result.unwrap();
        // ping returns ActionResult with action="ping" and flattened data
        assert_eq!(value["action"], "ping");
        assert_eq!(value["result"], "pong");
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
        // status returns ActionResult with flattened data
        assert_eq!(value["action"], "status");
        // check result field contains the status data
        let result_data = &value["result"];
        assert!(result_data.get("running").is_some(), "result: {:?}", value);
        assert!(result_data.get("socket_path").is_some());
        assert!(result_data.get("pid_file").is_some());
    }

    #[test]
    fn test_handle_ipc_request_unknown_method() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "unknown_method"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_err());

        let (code, msg) = result.unwrap_err();
        assert_eq!(code, exit_codes::INVALID_ARGS);
        assert!(msg.contains("unknown") || msg.contains("Unknown"));
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
    fn test_handle_ipc_request_move_missing_target() {
        let config = create_test_config(vec![]);
        let request = IpcRequest::parse(r#"{"method": "move"}"#).unwrap();

        let result = handle_ipc_request(&request, &config);
        assert!(result.is_err());

        let (code, msg) = result.unwrap_err();
        assert_eq!(code, exit_codes::INVALID_ARGS);
        assert!(msg.contains("to") || msg.contains("display"));
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

    // ========================================================================
    // parse_move_action_arg tests
    // ========================================================================

    #[test]
    fn test_parse_move_action_arg_display_next() {
        let (pos, disp) = super::parse_move_action_arg("next").unwrap();
        assert!(pos.is_none());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_display_prev() {
        let (pos, disp) = super::parse_move_action_arg("prev").unwrap();
        assert!(pos.is_none());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_display_numeric() {
        let (pos, disp) = super::parse_move_action_arg("2").unwrap();
        assert!(pos.is_none());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_display_alias() {
        // aliases like "external", "builtin", "office" should work
        let (pos, disp) = super::parse_move_action_arg("external").unwrap();
        assert!(pos.is_none());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_display_explicit() {
        let (pos, disp) = super::parse_move_action_arg("display=next").unwrap();
        assert!(pos.is_none());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_position_anchor() {
        let (pos, disp) = super::parse_move_action_arg("top-left").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_position_percent() {
        let (pos, disp) = super::parse_move_action_arg("50%,50%").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_combined_semicolon() {
        let (pos, disp) = super::parse_move_action_arg("top-left;display=next").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_combined_semicolon_no_prefix() {
        // semicolon without display= prefix should also work
        let (pos, disp) = super::parse_move_action_arg("top-left;next").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_percent_with_display() {
        let (pos, disp) = super::parse_move_action_arg("50%,50%;display=2").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_some());
    }

    // ========================================================================
    // parse_move_action_arg - bare numbers
    // ========================================================================

    #[test]
    fn test_parse_move_action_arg_bare_number_single_is_display() {
        // single bare number is interpreted as display index, not percentage
        // use explicit percentage (50%) or pair (50,50) for position
        let (pos, disp) = super::parse_move_action_arg("2").unwrap();
        assert!(pos.is_none());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_bare_number_pair() {
        // pair of bare numbers is interpreted as percentage position
        let (pos, disp) = super::parse_move_action_arg("25,75").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_bare_number_pair_with_display() {
        let (pos, disp) = super::parse_move_action_arg("50,50;display=next").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_some());
    }

    #[test]
    fn test_parse_move_action_arg_explicit_percent_single() {
        // use explicit % for single-value percentage
        let (pos, disp) = super::parse_move_action_arg("50%").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    // ========================================================================
    // parse_move_action_arg - relative movement
    // ========================================================================

    #[test]
    fn test_parse_move_action_arg_relative_both_axes() {
        let (pos, disp) = super::parse_move_action_arg("+100,-50").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_relative_x_only() {
        let (pos, disp) = super::parse_move_action_arg("+100").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_relative_y_only() {
        let (pos, disp) = super::parse_move_action_arg(",+100").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_relative_with_display() {
        let (pos, disp) = super::parse_move_action_arg("+100,-50;display=2").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_some());
    }

    // ========================================================================
    // parse_move_action_arg - pixels and points
    // ========================================================================

    #[test]
    fn test_parse_move_action_arg_pixels() {
        let (pos, disp) = super::parse_move_action_arg("100,200px").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_points() {
        let (pos, disp) = super::parse_move_action_arg("100,200pt").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_pixels_with_display() {
        let (pos, disp) = super::parse_move_action_arg("100,200px;display=next").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_some());
    }

    // ========================================================================
    // parse_move_action_arg - center anchor
    // ========================================================================

    #[test]
    fn test_parse_move_action_arg_center() {
        let (pos, disp) = super::parse_move_action_arg("center").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_none());
    }

    #[test]
    fn test_parse_move_action_arg_center_with_display() {
        let (pos, disp) = super::parse_move_action_arg("center;display=2").unwrap();
        assert!(pos.is_some());
        assert!(disp.is_some());
    }

    // ========================================================================
    // parse_move_action_arg - error cases
    // ========================================================================

    #[test]
    fn test_parse_move_action_arg_invalid_position() {
        let result = super::parse_move_action_arg("invalid-position");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_move_action_arg_invalid_percent_range() {
        let result = super::parse_move_action_arg("150%");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_move_action_arg_invalid_bare_number_range() {
        // bare number pair with value > 100 should fail
        let result = super::parse_move_action_arg("150,50");
        assert!(result.is_err());
    }
}
