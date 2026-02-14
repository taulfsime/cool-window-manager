//! move-display action handler

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::handlers::common::{resolve_app_target, AppResolution};
use crate::actions::result::{ActionResult, AppData, DisplayData};
use crate::display::DisplayTarget;
use crate::window::manager;

/// execute move-display action
pub fn execute(
    app: Vec<String>,
    target: DisplayTarget,
    launch: Option<bool>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    match resolve_app_target(&app, launch, ctx)? {
        AppResolution::Found { app, match_info } => {
            let (display_index, display_name) = manager::move_to_display_with_aliases(
                Some(&app),
                &target,
                ctx.verbose,
                &ctx.config.display_aliases,
            )
            .map_err(ActionError::from)?;

            Ok(ActionResult::move_display(
                AppData::from(&app),
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
            let (display_index, display_name) = manager::move_to_display_with_aliases(
                None,
                &target,
                ctx.verbose,
                &ctx.config.display_aliases,
            )
            .map_err(ActionError::from)?;

            Ok(ActionResult::move_display(
                AppData {
                    name: "focused".to_string(),
                    pid: 0,
                    bundle_id: None,
                },
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
