//! CLI handlers for events commands

use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

use crate::daemon::ipc;

use super::exit_codes;
use super::output::OutputMode;

/// listen for events and stream to stdout
pub fn listen(
    event_filters: Vec<String>,
    app_filters: Vec<String>,
    format: Option<String>,
    output_mode: &OutputMode,
) -> Result<()> {
    // check if daemon is running
    if !ipc::is_daemon_running() {
        return Err(anyhow!(
            "Daemon is not running. Start with: cwm daemon start"
        ));
    }

    // connect to daemon and subscribe
    let socket_path = ipc::get_socket_path();
    let mut stream = std::os::unix::net::UnixStream::connect(&socket_path)
        .map_err(|e| anyhow!("Failed to connect to daemon: {}", e))?;

    // build subscribe request
    let events_param = if event_filters.is_empty() {
        "*".to_string()
    } else {
        event_filters.join(",")
    };

    let app_param = app_filters.join(",");

    let request = if app_param.is_empty() {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "params": { "events": events_param },
            "id": 1
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "params": { "events": events_param, "app": app_param },
            "id": 1
        })
    };

    // send subscribe request
    use std::io::{BufRead, BufReader};
    stream.write_all(request.to_string().as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // read subscription response
    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    // parse response to check for errors
    let response: serde_json::Value = serde_json::from_str(&response_line)
        .map_err(|e| anyhow!("Invalid response from daemon: {}", e))?;

    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(anyhow!("Subscription failed: {}", message));
    }

    // print subscription info if not quiet
    if !output_mode.is_json() && std::io::stdout().is_terminal() {
        let subscribed = response
            .get("result")
            .and_then(|r| r.get("subscribed"))
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "all".to_string());

        eprintln!("Listening for events: {}", subscribed);
        eprintln!("Press Ctrl+C to stop\n");
    }

    // stream events
    let use_json = output_mode.is_json() || !std::io::stdout().is_terminal();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // connection closed
                break;
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // parse event notification
                if let Ok(notification) = serde_json::from_str::<serde_json::Value>(line) {
                    if notification.get("method") == Some(&serde_json::json!("event")) {
                        if let Some(params) = notification.get("params") {
                            output_event(params, &format, use_json)?;
                        }
                    }
                }
            }
            Err(e) => {
                return Err(anyhow!("Error reading from daemon: {}", e));
            }
        }
    }

    Ok(())
}

/// wait for specific event(s) then exit
pub fn wait(
    event_filters: Vec<String>,
    app_filters: Vec<String>,
    timeout: Option<u64>,
    output_mode: &OutputMode,
) -> Result<i32> {
    // check if daemon is running
    if !ipc::is_daemon_running() {
        eprintln!("Daemon is not running. Start with: cwm daemon start");
        return Ok(exit_codes::DAEMON_NOT_RUNNING);
    }

    // connect to daemon and subscribe
    let socket_path = ipc::get_socket_path();
    let mut stream = std::os::unix::net::UnixStream::connect(&socket_path)
        .map_err(|e| anyhow!("Failed to connect to daemon: {}", e))?;

    // build subscribe request
    let events_param = if event_filters.is_empty() {
        "*".to_string()
    } else {
        event_filters.join(",")
    };

    let app_param = app_filters.join(",");

    let request = if app_param.is_empty() {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "params": { "events": events_param },
            "id": 1
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "subscribe",
            "params": { "events": events_param, "app": app_param },
            "id": 1
        })
    };

    // send subscribe request
    use std::io::{BufRead, BufReader};
    stream.write_all(request.to_string().as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // set read timeout if specified
    let timeout_duration = timeout.map(Duration::from_secs);
    if let Some(duration) = timeout_duration {
        stream.set_read_timeout(Some(duration))?;
    }

    let start_time = Instant::now();

    // read subscription response
    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    // parse response to check for errors
    let response: serde_json::Value = serde_json::from_str(&response_line)
        .map_err(|e| anyhow!("Invalid response from daemon: {}", e))?;

    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(anyhow!("Subscription failed: {}", message));
    }

    // wait for first matching event
    loop {
        // check timeout
        if let Some(duration) = timeout_duration {
            if start_time.elapsed() >= duration {
                return Ok(exit_codes::TIMEOUT);
            }
        }

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // connection closed
                return Err(anyhow!("Connection to daemon closed unexpectedly"));
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // parse event notification
                if let Ok(notification) = serde_json::from_str::<serde_json::Value>(line) {
                    if notification.get("method") == Some(&serde_json::json!("event")) {
                        if let Some(params) = notification.get("params") {
                            // output the matching event
                            let use_json =
                                output_mode.is_json() || !std::io::stdout().is_terminal();
                            output_event(params, &None, use_json)?;
                            return Ok(exit_codes::SUCCESS);
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // timeout on read, check overall timeout
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Ok(exit_codes::TIMEOUT);
            }
            Err(e) => {
                return Err(anyhow!("Error reading from daemon: {}", e));
            }
        }
    }
}

/// output a single event
fn output_event(event: &serde_json::Value, format: &Option<String>, use_json: bool) -> Result<()> {
    let mut stdout = std::io::stdout().lock();

    if use_json {
        // output as JSON
        writeln!(stdout, "{}", serde_json::to_string(event)?)?;
    } else if let Some(fmt) = format {
        // custom format
        let output = format_event(event, fmt);
        writeln!(stdout, "{}", output)?;
    } else {
        // default text format: [HH:MM:SS] type: app (pid: N)
        let ts = event
            .get("ts")
            .and_then(|t| t.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "??:??:??".to_string());

        let event_type = event
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");

        let data = event.get("data");
        let app = data
            .and_then(|d| d.get("app"))
            .and_then(|a| a.as_str())
            .unwrap_or("");
        let pid = data
            .and_then(|d| d.get("pid"))
            .and_then(|p| p.as_i64())
            .map(|p| p.to_string())
            .unwrap_or_default();

        if app.is_empty() {
            writeln!(stdout, "[{}] {}", ts, event_type)?;
        } else {
            writeln!(stdout, "[{}] {}: {} (pid: {})", ts, event_type, app, pid)?;
        }
    }

    stdout.flush()?;
    Ok(())
}

/// format event using template string
fn format_event(event: &serde_json::Value, template: &str) -> String {
    let mut result = template.to_string();

    // replace {type}
    if let Some(t) = event.get("type").and_then(|v| v.as_str()) {
        result = result.replace("{type}", t);
    }

    // replace {ts}
    if let Some(ts) = event.get("ts").and_then(|v| v.as_str()) {
        result = result.replace("{ts}", ts);
    }

    // replace {data.field} patterns
    if let Some(data) = event.get("data") {
        for (key, value) in data.as_object().into_iter().flatten() {
            let placeholder = format!("{{data.{}}}", key);
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                _ => value.to_string(),
            };
            result = result.replace(&placeholder, &value_str);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_event_type() {
        let event = serde_json::json!({
            "type": "app.launched",
            "ts": "2026-02-14T10:30:45Z",
            "data": {"app": "Safari", "pid": 1234}
        });

        assert_eq!(format_event(&event, "{type}"), "app.launched");
    }

    #[test]
    fn test_format_event_data_fields() {
        let event = serde_json::json!({
            "type": "app.launched",
            "ts": "2026-02-14T10:30:45Z",
            "data": {"app": "Safari", "pid": 1234}
        });

        assert_eq!(format_event(&event, "{data.app}"), "Safari");
        assert_eq!(format_event(&event, "{data.pid}"), "1234");
        assert_eq!(
            format_event(&event, "{type}: {data.app} ({data.pid})"),
            "app.launched: Safari (1234)"
        );
    }

    #[test]
    fn test_format_event_missing_field() {
        let event = serde_json::json!({
            "type": "daemon.started",
            "ts": "2026-02-14T10:30:45Z",
            "data": {"pid": 5678}
        });

        // missing field stays as placeholder
        assert_eq!(format_event(&event, "{data.app}"), "{data.app}");
        assert_eq!(format_event(&event, "{data.pid}"), "5678");
    }
}
