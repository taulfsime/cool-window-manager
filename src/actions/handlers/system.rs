//! system action handlers (ping, status, version, check_permissions)

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::ActionResult;
use crate::daemon::ipc;
use crate::version::Version;
use crate::window::accessibility;

/// execute ping action (health check)
pub fn execute_ping(_ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    Ok(ActionResult::simple("ping", serde_json::json!("pong")))
}

/// execute status action (daemon status)
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
        "status",
        serde_json::json!({
            "running": running,
            "pid": pid,
            "socket_path": socket_path.to_string_lossy(),
            "pid_file": pid_path.to_string_lossy(),
        }),
    ))
}

/// execute version action
pub fn execute_version(_ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    let version = Version::current();

    Ok(ActionResult::simple(
        "version",
        serde_json::json!({
            "semver": version.semver,
            "commit": version.short_commit,
            "channel": version.channel,
            "timestamp": version.timestamp.to_rfc3339(),
            "build_date": version.build_date.to_rfc3339(),
            "dirty": version.dirty,
            "version_string": version.version_string(),
            "full_version": version.full_version_string(),
        }),
    ))
}

/// execute check_permissions action
pub fn execute_check_permissions(
    prompt: bool,
    _ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    let trusted = if prompt {
        accessibility::check_and_prompt()
    } else {
        accessibility::is_trusted()
    };

    Ok(ActionResult::simple(
        "check_permissions",
        serde_json::json!({
            "granted": trusted,
        }),
    ))
}
