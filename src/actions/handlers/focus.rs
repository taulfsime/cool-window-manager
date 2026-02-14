//! focus action handler

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::handlers::common::{resolve_app_target, AppResolution};
use crate::actions::result::{ActionResult, AppData};
use crate::window::manager;

/// execute focus action
pub fn execute(
    app: Vec<String>,
    launch: Option<bool>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if app.is_empty() {
        return Err(ActionError::invalid_args(
            "focus requires at least one app name",
        ));
    }

    match resolve_app_target(&app, launch, ctx)? {
        AppResolution::Found { app, match_info } => {
            manager::focus_app(&app, ctx.verbose).map_err(ActionError::from)?;
            Ok(ActionResult::focus(AppData::from(&app), match_info))
        }
        AppResolution::Launched { app_name } => Ok(ActionResult::launched(app_name)),
        AppResolution::Focused => {
            // focus requires an app name, this shouldn't happen
            Err(ActionError::invalid_args(
                "focus requires at least one app name",
            ))
        }
    }
}
