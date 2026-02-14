//! daemon action handlers

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::ActionResult;
use crate::daemon::ipc;

/// execute daemon status action
pub fn execute_status(_ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    let running = ipc::is_daemon_running();
    let socket_path = ipc::get_socket_path();
    let pid_path = ipc::get_pid_file_path();

    // try to read pid if running
    let pid = if running {
        std::fs::read_to_string(&pid_path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
    } else {
        None
    };

    Ok(ActionResult::simple(
        "daemon_status",
        serde_json::json!({
            "running": running,
            "pid": pid,
            "socket_path": socket_path.to_string_lossy(),
            "pid_file": pid_path.to_string_lossy(),
        }),
    ))
}

/// daemon start - requires CLI (spawns background process)
pub fn execute_start(
    log: Option<String>,
    foreground: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "daemon start is only available via CLI",
        ));
    }

    if foreground {
        // this blocks, so we won't return a result
        crate::daemon::start_foreground(log).map_err(ActionError::from)?;
        // unreachable in normal operation
        Ok(ActionResult::simple(
            "daemon_start",
            serde_json::json!({"status": "stopped"}),
        ))
    } else {
        crate::daemon::start(log).map_err(ActionError::from)?;
        Ok(ActionResult::simple(
            "daemon_start",
            serde_json::json!({"status": "started", "background": true}),
        ))
    }
}

/// daemon stop - requires CLI (sends signal to process)
pub fn execute_stop(ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "daemon stop is only available via CLI",
        ));
    }

    crate::daemon::stop().map_err(ActionError::from)?;
    Ok(ActionResult::simple(
        "daemon_stop",
        serde_json::json!({"status": "stopped"}),
    ))
}

/// daemon install - requires CLI (creates launchd plist)
pub fn execute_install(
    bin: Option<String>,
    log: Option<String>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "daemon install is only available via CLI",
        ));
    }

    crate::daemon::install(bin, log).map_err(ActionError::from)?;
    Ok(ActionResult::simple(
        "daemon_install",
        serde_json::json!({"status": "installed"}),
    ))
}

/// daemon uninstall - requires CLI (removes launchd plist)
pub fn execute_uninstall(ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "daemon uninstall is only available via CLI",
        ));
    }

    crate::daemon::uninstall().map_err(ActionError::from)?;
    Ok(ActionResult::simple(
        "daemon_uninstall",
        serde_json::json!({"status": "uninstalled"}),
    ))
}
