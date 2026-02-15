//! kill action handler

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::{ActionResult, AppData, MatchData};
use crate::window::manager;
use crate::window::matching::{self, AppInfo};

/// default timeout for waiting for app termination (5 seconds)
const DEFAULT_WAIT_TIMEOUT_MS: u64 = 5000;

/// execute kill action
pub fn execute(
    app: Vec<String>,
    force: bool,
    wait: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if app.is_empty() {
        return Err(ActionError::invalid_args(
            "kill requires at least one app name",
        ));
    }

    // find matching app (no launch behavior for kill)
    let (found_app, match_info) = find_app_for_kill(&app, ctx)?;

    // terminate the app
    let accepted =
        manager::terminate_app(&found_app, force, ctx.verbose).map_err(ActionError::from)?;

    if !accepted {
        return Err(ActionError::general(format!(
            "{} declined to terminate (may have unsaved changes)",
            found_app.name
        )));
    }

    // optionally wait for termination
    let terminated = if wait {
        manager::wait_for_termination(found_app.pid, DEFAULT_WAIT_TIMEOUT_MS, ctx.verbose)
    } else {
        // we don't know if it actually terminated yet
        true
    };

    if wait && !terminated {
        return Err(ActionError::general(format!(
            "timeout waiting for {} to terminate",
            found_app.name
        )));
    }

    Ok(ActionResult::kill(
        AppData::from(&found_app),
        match_info,
        force,
        terminated,
    ))
}

/// find app for kill action (no launch behavior)
fn find_app_for_kill(
    apps: &[String],
    ctx: &ExecutionContext,
) -> Result<(AppInfo, MatchData), ActionError> {
    let running_apps = matching::get_running_apps().map_err(ActionError::from)?;

    // try each app in order
    for app_name in apps {
        let match_result =
            matching::find_app(app_name, &running_apps, ctx.config.settings.fuzzy_threshold);

        if let Some(result) = match_result {
            if ctx.verbose {
                eprintln!("Matched {} -> {}", app_name, result.describe());
            }
            let match_data = MatchData::from_match_result(&result, app_name);
            return Ok((result.app, match_data));
        } else if ctx.verbose {
            eprintln!("App '{}' not found, trying next...", app_name);
        }
    }

    // error: no app found
    let suggestions: Vec<String> = running_apps
        .iter()
        .take(10)
        .map(|a| a.name.clone())
        .collect();

    Err(ActionError::app_not_found_with_suggestions(
        format!("no matching app found, tried: {}", apps.join(", ")),
        suggestions,
    ))
}
