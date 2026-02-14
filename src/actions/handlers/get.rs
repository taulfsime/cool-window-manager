//! get action handlers

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::{ActionResult, AppData, DisplayData, WindowData};
use crate::window::{manager, matching};

/// execute get focused window action
pub fn execute_get_focused(_ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    let (app, window_data, display_data) =
        manager::get_focused_window_info().map_err(ActionError::from)?;

    Ok(ActionResult::get(
        AppData::from(&app),
        WindowData {
            x: window_data.x,
            y: window_data.y,
            width: window_data.width,
            height: window_data.height,
            title: window_data.title,
        },
        DisplayData {
            index: display_data.index,
            name: display_data.name,
            width: None,
            height: None,
        },
    ))
}

/// execute get window for specific app action
pub fn execute_get_window(
    app: Vec<String>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if app.is_empty() {
        return execute_get_focused(ctx);
    }

    let running_apps = matching::get_running_apps().map_err(ActionError::from)?;

    // try each app in order until one matches
    for app_name in &app {
        let match_result =
            matching::find_app(app_name, &running_apps, ctx.config.settings.fuzzy_threshold);

        if let Some(result) = match_result {
            if ctx.verbose {
                eprintln!("Matched {} -> {}", app_name, result.describe());
            }

            let (_, window_data, display_data) =
                manager::get_window_info_for_app(&result.app).map_err(ActionError::from)?;

            return Ok(ActionResult::get(
                AppData::from(&result.app),
                WindowData {
                    x: window_data.x,
                    y: window_data.y,
                    width: window_data.width,
                    height: window_data.height,
                    title: window_data.title,
                },
                DisplayData {
                    index: display_data.index,
                    name: display_data.name,
                    width: None,
                    height: None,
                },
            ));
        } else if ctx.verbose {
            eprintln!("App '{}' not found, trying next...", app_name);
        }
    }

    Err(ActionError::app_not_found(format!(
        "no matching app found, tried: {}",
        app.join(", ")
    )))
}
