//! maximize action handler

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::handlers::common::{resolve_app_target, AppResolution};
use crate::actions::result::{ActionResult, AppData};
use crate::window::manager;

/// execute maximize action
pub fn execute(
    app: Vec<String>,
    launch: Option<bool>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    match resolve_app_target(&app, launch, ctx)? {
        AppResolution::Found { app, match_info } => {
            manager::maximize_app(Some(&app), ctx.verbose).map_err(ActionError::from)?;
            Ok(ActionResult::maximize(
                AppData::from(&app),
                Some(match_info),
            ))
        }
        AppResolution::Launched { app_name } => Ok(ActionResult::launched(app_name)),
        AppResolution::Focused => {
            manager::maximize_app(None, ctx.verbose).map_err(ActionError::from)?;
            Ok(ActionResult::maximize(
                AppData {
                    name: "focused".to_string(),
                    pid: 0,
                    bundle_id: None,
                },
                None,
            ))
        }
    }
}
