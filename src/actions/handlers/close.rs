//! close action handler

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::{ActionResult, AppData, MatchData};
use crate::window::manager;
use crate::window::matching::{self, AppInfo};

/// execute close action
pub fn execute(app: Vec<String>, ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    if app.is_empty() {
        return Err(ActionError::invalid_args(
            "close requires at least one app name",
        ));
    }

    // find matching app (no launch behavior for close)
    let (found_app, match_info) = find_app_for_close(&app, ctx)?;

    // close all windows of the app
    let windows_closed =
        manager::close_app_windows(&found_app, ctx.verbose).map_err(ActionError::from)?;

    Ok(ActionResult::close(
        AppData::from(&found_app),
        match_info,
        windows_closed,
    ))
}

/// find app for close action (no launch behavior)
fn find_app_for_close(
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
