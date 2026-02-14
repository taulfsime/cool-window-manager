//! common utilities for action handlers

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::MatchData;
use crate::window::matching::AppInfo;
use crate::window::{manager, matching};

/// result of resolving an app target
pub enum AppResolution {
    /// found a matching app
    Found { app: AppInfo, match_info: MatchData },
    /// no app specified, use focused window
    Focused,
    /// app was launched (not running, but launch=true)
    Launched { app_name: String },
}

/// resolve app target from app names list
///
/// - empty list = use focused window (returns Focused)
/// - tries each app in order until one matches (returns Found)
/// - if none match and launch=true, launches first app (returns Launched)
/// - if none match and launch=false, returns error
pub fn resolve_app_target(
    apps: &[String],
    launch: Option<bool>,
    ctx: &ExecutionContext,
) -> Result<AppResolution, ActionError> {
    // empty list = use focused window
    if apps.is_empty() {
        return Ok(AppResolution::Focused);
    }

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
            return Ok(AppResolution::Found {
                app: result.app,
                match_info: match_data,
            });
        } else if ctx.verbose {
            eprintln!("App '{}' not found, trying next...", app_name);
        }
    }

    // no app found, check if we should launch the first one
    let should_launch = resolve_launch_behavior(launch, ctx.config.settings.launch);

    if should_launch {
        let first_app = &apps[0];
        if ctx.verbose {
            eprintln!("No apps found, launching '{}'...", first_app);
        }
        manager::launch_app(first_app, ctx.verbose).map_err(ActionError::from)?;
        return Ok(AppResolution::Launched {
            app_name: first_app.clone(),
        });
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

/// resolve launch behavior from override and config default
pub fn resolve_launch_behavior(launch_override: Option<bool>, config_default: bool) -> bool {
    match launch_override {
        Some(true) => true,
        Some(false) => false,
        None => config_default,
    }
}
