//! resize action handler

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::handlers::common::{resolve_app_target, AppResolution};
use crate::actions::result::{ActionResult, AppData, SizeData};
use crate::window::manager;
use crate::window::manager::ResizeTarget;

/// execute resize action
pub fn execute(
    app: Vec<String>,
    target: ResizeTarget,
    overflow: bool,
    launch: Option<bool>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    match resolve_app_target(&app, launch, ctx)? {
        AppResolution::Found { app, match_info } => {
            let (width, height) = manager::resize_app(Some(&app), &target, overflow, ctx.verbose)
                .map_err(ActionError::from)?;
            Ok(ActionResult::resize(
                AppData::from(&app),
                SizeData { width, height },
                Some(match_info),
            ))
        }
        AppResolution::Launched { app_name } => Ok(ActionResult::launched(app_name)),
        AppResolution::Focused => {
            let (width, height) = manager::resize_app(None, &target, overflow, ctx.verbose)
                .map_err(ActionError::from)?;
            Ok(ActionResult::resize(
                AppData {
                    name: "focused".to_string(),
                    pid: 0,
                    bundle_id: None,
                },
                SizeData { width, height },
                None,
            ))
        }
    }
}
