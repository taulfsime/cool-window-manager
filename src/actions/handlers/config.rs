//! config action handlers

use std::path::Path;

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::ActionResult;
use crate::config::{self, Config};

/// execute config show action
pub fn execute_show(ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    // serialize the current config to JSON
    let config_json = serde_json::to_value(ctx.config.clone()).map_err(|e| {
        ActionError::new(
            crate::cli::exit_codes::ERROR,
            format!("failed to serialize config: {}", e),
        )
    })?;

    Ok(ActionResult::simple("config_show", config_json))
}

/// execute config path action
pub fn execute_path(config_override: Option<&Path>) -> Result<ActionResult, ActionError> {
    let path = config::get_config_path_with_override(config_override).map_err(|e| {
        ActionError::new(
            crate::cli::exit_codes::ERROR,
            format!("failed to get config path: {}", e),
        )
    })?;

    Ok(ActionResult::simple(
        "config_path",
        serde_json::json!({
            "path": path.to_string_lossy(),
        }),
    ))
}

/// execute config verify action
pub fn execute_verify(config_override: Option<&Path>) -> Result<ActionResult, ActionError> {
    let path = config::get_config_path_with_override(config_override).map_err(|e| {
        ActionError::new(
            crate::cli::exit_codes::ERROR,
            format!("failed to get config path: {}", e),
        )
    })?;

    let errors = config::verify(&path).map_err(|e| {
        ActionError::new(
            crate::cli::exit_codes::ERROR,
            format!("failed to verify config: {}", e),
        )
    })?;

    let valid = errors.is_empty();

    Ok(ActionResult::simple(
        "config_verify",
        serde_json::json!({
            "valid": valid,
            "errors": errors,
            "path": path.to_string_lossy(),
        }),
    ))
}

/// execute config default action (show default config with examples)
pub fn execute_default(_ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    let default_config = config::default_with_examples();
    let config_json = serde_json::to_value(&default_config).map_err(|e| {
        ActionError::new(
            crate::cli::exit_codes::ERROR,
            format!("failed to serialize default config: {}", e),
        )
    })?;

    Ok(ActionResult::simple("config_default", config_json))
}

/// config set - requires CLI (modifies files)
pub fn execute_set(
    key: &str,
    value: &str,
    config_override: Option<&Path>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "config set is only available via CLI",
        ));
    }

    let mut cfg = config::load_with_override(config_override).map_err(ActionError::from)?;
    config::set_value(&mut cfg, key, value).map_err(ActionError::from)?;
    config::save_with_override(&cfg, config_override).map_err(ActionError::from)?;

    Ok(ActionResult::simple(
        "config_set",
        serde_json::json!({
            "key": key,
            "value": value,
        }),
    ))
}

/// config reset - requires CLI (modifies files)
pub fn execute_reset(
    config_override: Option<&Path>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "config reset is only available via CLI",
        ));
    }

    let cfg = Config::default();
    config::save_with_override(&cfg, config_override).map_err(ActionError::from)?;

    Ok(ActionResult::simple(
        "config_reset",
        serde_json::json!({"status": "reset"}),
    ))
}
