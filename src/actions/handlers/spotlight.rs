//! spotlight action handlers

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::ActionResult;
use crate::spotlight;

/// execute spotlight list action
pub fn execute_list(_ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    let installed = spotlight::get_installed_shortcuts().map_err(|e| {
        ActionError::new(
            crate::cli::exit_codes::ERROR,
            format!("failed to list shortcuts: {}", e),
        )
    })?;

    let apps_dir = spotlight::get_apps_directory();

    Ok(ActionResult::simple(
        "spotlight_list",
        serde_json::json!({
            "shortcuts": installed,
            "apps_directory": apps_dir.to_string_lossy(),
            "count": installed.len(),
        }),
    ))
}

/// execute spotlight example action
pub fn execute_example(_ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    let examples = spotlight::get_example_shortcuts();

    Ok(ActionResult::simple(
        "spotlight_example",
        serde_json::json!({
            "examples": examples,
        }),
    ))
}

/// spotlight install - requires CLI (creates files, triggers Spotlight reindex)
pub fn execute_install(
    name: Option<&str>,
    force: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "spotlight install is only available via CLI",
        ));
    }

    if ctx.config.spotlight.is_empty() {
        return Ok(ActionResult::simple(
            "spotlight_install",
            serde_json::json!({
                "status": "no_shortcuts",
                "message": "no spotlight shortcuts configured",
            }),
        ));
    }

    let apps_dir = spotlight::get_apps_directory();

    if let Some(shortcut_name) = name {
        // install specific shortcut
        let shortcut = ctx
            .config
            .spotlight
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(shortcut_name))
            .ok_or_else(|| {
                ActionError::invalid_args(format!(
                    "shortcut '{}' not found in config",
                    shortcut_name
                ))
            })?;

        let path = spotlight::install_shortcut(shortcut, force).map_err(ActionError::from)?;

        Ok(ActionResult::simple(
            "spotlight_install",
            serde_json::json!({
                "status": "installed",
                "shortcut": shortcut_name,
                "path": path.to_string_lossy(),
                "apps_directory": apps_dir.to_string_lossy(),
            }),
        ))
    } else {
        // install all shortcuts
        let installed =
            spotlight::install_all(&ctx.config.spotlight, force).map_err(ActionError::from)?;

        let paths: Vec<String> = installed
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        Ok(ActionResult::simple(
            "spotlight_install",
            serde_json::json!({
                "status": "installed",
                "count": installed.len(),
                "paths": paths,
                "apps_directory": apps_dir.to_string_lossy(),
            }),
        ))
    }
}

/// spotlight remove - requires CLI (deletes files)
pub fn execute_remove(
    name: Option<&str>,
    all: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "spotlight remove is only available via CLI",
        ));
    }

    if all {
        let count = spotlight::remove_all().map_err(ActionError::from)?;
        Ok(ActionResult::simple(
            "spotlight_remove",
            serde_json::json!({
                "status": "removed",
                "count": count,
            }),
        ))
    } else if let Some(shortcut_name) = name {
        spotlight::remove_shortcut(shortcut_name).map_err(ActionError::from)?;
        Ok(ActionResult::simple(
            "spotlight_remove",
            serde_json::json!({
                "status": "removed",
                "shortcut": shortcut_name,
            }),
        ))
    } else {
        Err(ActionError::invalid_args(
            "specify a shortcut name or use --all to remove all shortcuts",
        ))
    }
}
