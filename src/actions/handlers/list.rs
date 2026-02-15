//! list action handlers

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::ActionResult;
use crate::daemon::events::EventType;
use crate::display;
use crate::window::matching;

/// execute list apps action
pub fn execute_list_apps(
    detailed: bool,
    _ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    let apps = matching::get_running_apps().map_err(ActionError::from)?;

    let items: Vec<serde_json::Value> = if detailed {
        apps.iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "pid": a.pid,
                    "bundle_id": a.bundle_id,
                    "titles": a.titles,
                })
            })
            .collect()
    } else {
        apps.iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "pid": a.pid,
                })
            })
            .collect()
    };

    Ok(ActionResult::list("list-apps", items))
}

/// execute list displays action
pub fn execute_list_displays(
    detailed: bool,
    _ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    let displays = display::get_displays().map_err(ActionError::from)?;

    let items: Vec<serde_json::Value> = if detailed {
        displays
            .iter()
            .map(|d| {
                serde_json::json!({
                    "index": d.index,
                    "name": d.name,
                    "width": d.width,
                    "height": d.height,
                    "x": d.x,
                    "y": d.y,
                    "is_main": d.is_main,
                    "is_builtin": d.is_builtin,
                    "display_id": d.display_id,
                    "vendor_id": d.vendor_id,
                    "model_id": d.model_id,
                    "serial_number": d.serial_number,
                    "unit_number": d.unit_number,
                    "unique_id": d.unique_id(),
                })
            })
            .collect()
    } else {
        displays
            .iter()
            .map(|d| {
                serde_json::json!({
                    "index": d.index,
                    "name": d.name,
                    "width": d.width,
                    "height": d.height,
                    "is_main": d.is_main,
                })
            })
            .collect()
    };

    Ok(ActionResult::list("list-displays", items))
}

/// execute list aliases action
pub fn execute_list_aliases(
    detailed: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    let displays = display::get_displays().map_err(ActionError::from)?;

    // system aliases
    let system_aliases = vec![
        ("builtin", "Built-in display (e.g., MacBook screen)"),
        ("external", "Any external monitor"),
        ("main", "Primary display"),
        ("secondary", "Secondary display"),
    ];

    let mut items: Vec<serde_json::Value> = Vec::new();

    // add system aliases
    for (name, description) in system_aliases {
        let resolved = display::resolve_alias(name, &ctx.config.display_aliases, &displays);
        let (is_resolved, display_index, display_name, display_unique_id) = match &resolved {
            Ok(d) => (
                true,
                Some(d.index),
                Some(d.name.clone()),
                Some(d.unique_id()),
            ),
            Err(_) => (false, None, None, None),
        };

        if detailed {
            items.push(serde_json::json!({
                "name": name,
                "type": "system",
                "description": description,
                "resolved": is_resolved,
                "display_index": display_index,
                "display_name": display_name,
                "display_unique_id": display_unique_id,
            }));
        } else {
            items.push(serde_json::json!({
                "name": name,
                "type": "system",
                "resolved": is_resolved,
                "display_index": display_index,
            }));
        }
    }

    // add user-defined aliases
    for (name, ids) in &ctx.config.display_aliases {
        let resolved = display::resolve_alias(name, &ctx.config.display_aliases, &displays);
        let (is_resolved, display_index, display_name, display_unique_id) = match &resolved {
            Ok(d) => (
                true,
                Some(d.index),
                Some(d.name.clone()),
                Some(d.unique_id()),
            ),
            Err(_) => (false, None, None, None),
        };

        if detailed {
            items.push(serde_json::json!({
                "name": name,
                "type": "user",
                "configured_ids": ids,
                "resolved": is_resolved,
                "display_index": display_index,
                "display_name": display_name,
                "display_unique_id": display_unique_id,
            }));
        } else {
            items.push(serde_json::json!({
                "name": name,
                "type": "user",
                "resolved": is_resolved,
                "display_index": display_index,
            }));
        }
    }

    Ok(ActionResult::list("list-aliases", items))
}

/// execute list events action
pub fn execute_list_events(
    detailed: bool,
    _ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    let items: Vec<serde_json::Value> = EventType::all()
        .iter()
        .map(|e| {
            if detailed {
                serde_json::json!({
                    "name": e.as_str(),
                    "description": e.description(),
                })
            } else {
                serde_json::json!({
                    "name": e.as_str(),
                })
            }
        })
        .collect();

    Ok(ActionResult::list("list-events", items))
}
