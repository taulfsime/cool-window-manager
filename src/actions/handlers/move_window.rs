//! move action handler

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::handlers::common::{resolve_app_target, AppResolution};
use crate::actions::result::{ActionResult, AppData, DisplayData, PositionData};
use crate::display::DisplayTarget;
use crate::window::manager::{self, MoveTarget};

/// execute move action
pub fn execute(
    app: Vec<String>,
    to: Option<MoveTarget>,
    display: Option<DisplayTarget>,
    launch: Option<bool>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    match resolve_app_target(&app, launch, ctx)? {
        AppResolution::Found { app, match_info } => {
            let (x, y, display_index, display_name) = manager::move_window(
                Some(&app),
                to.as_ref(),
                display.as_ref(),
                ctx.verbose,
                &ctx.config.display_aliases,
            )
            .map_err(ActionError::from)?;

            Ok(ActionResult::move_window(
                AppData::from(&app),
                PositionData { x, y },
                DisplayData {
                    index: display_index,
                    name: display_name,
                    width: None,
                    height: None,
                },
                Some(match_info),
            ))
        }
        AppResolution::Launched { app_name } => Ok(ActionResult::launched(app_name)),
        AppResolution::Focused => {
            let (x, y, display_index, display_name) = manager::move_window(
                None,
                to.as_ref(),
                display.as_ref(),
                ctx.verbose,
                &ctx.config.display_aliases,
            )
            .map_err(ActionError::from)?;

            Ok(ActionResult::move_window(
                AppData {
                    name: "focused".to_string(),
                    pid: 0,
                    bundle_id: None,
                },
                PositionData { x, y },
                DisplayData {
                    index: display_index,
                    name: display_name,
                    width: None,
                    height: None,
                },
                None,
            ))
        }
    }
}
