//! undo/redo/history action handlers
//!
//! these handlers communicate with the daemon via IPC since history
//! is managed in-memory by the daemon with debounced persistence

use std::collections::HashMap;

use crate::actions::error::ActionError;
use crate::actions::result::ActionResult;
use crate::cli::exit_codes;
use crate::daemon::ipc::{is_daemon_running, send_jsonrpc};

/// execute undo action via daemon IPC
#[allow(unused_variables)]
pub fn execute_undo(
    ctx: &crate::actions::context::ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !is_daemon_running() {
        return Err(ActionError::new(
            exit_codes::ERROR,
            "Daemon not running. Start with 'cwm daemon start'",
        ));
    }

    let params: HashMap<String, String> = HashMap::new();
    let response_str = send_jsonrpc("undo", params, Some("1"))
        .map_err(|e| ActionError::general(format!("IPC error: {}", e)))?;

    // parse the response JSON
    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| ActionError::general(format!("Invalid JSON response: {}", e)))?;

    // check for error in response
    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(ActionError::general(message));
    }

    if let Some(result) = response.get("result") {
        Ok(ActionResult::simple("undo", result.clone()))
    } else {
        Err(ActionError::general("Invalid response from daemon"))
    }
}

/// execute redo action via daemon IPC
#[allow(unused_variables)]
pub fn execute_redo(
    ctx: &crate::actions::context::ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !is_daemon_running() {
        return Err(ActionError::new(
            exit_codes::ERROR,
            "Daemon not running. Start with 'cwm daemon start'",
        ));
    }

    let params: HashMap<String, String> = HashMap::new();
    let response_str = send_jsonrpc("redo", params, Some("1"))
        .map_err(|e| ActionError::general(format!("IPC error: {}", e)))?;

    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| ActionError::general(format!("Invalid JSON response: {}", e)))?;

    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(ActionError::general(message));
    }

    if let Some(result) = response.get("result") {
        Ok(ActionResult::simple("redo", result.clone()))
    } else {
        Err(ActionError::general("Invalid response from daemon"))
    }
}

/// execute history list action via daemon IPC
#[allow(unused_variables)]
pub fn execute_list(
    ctx: &crate::actions::context::ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !is_daemon_running() {
        return Err(ActionError::new(
            exit_codes::ERROR,
            "Daemon not running. Start with 'cwm daemon start'",
        ));
    }

    let params: HashMap<String, String> = HashMap::new();
    let response_str = send_jsonrpc("history_list", params, Some("1"))
        .map_err(|e| ActionError::general(format!("IPC error: {}", e)))?;

    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| ActionError::general(format!("Invalid JSON response: {}", e)))?;

    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(ActionError::general(message));
    }

    if let Some(result) = response.get("result") {
        Ok(ActionResult::simple("history_list", result.clone()))
    } else {
        Err(ActionError::general("Invalid response from daemon"))
    }
}

/// execute history clear action via daemon IPC
#[allow(unused_variables)]
pub fn execute_clear(
    ctx: &crate::actions::context::ExecutionContext,
) -> Result<ActionResult, ActionError> {
    if !is_daemon_running() {
        return Err(ActionError::new(
            exit_codes::ERROR,
            "Daemon not running. Start with 'cwm daemon start'",
        ));
    }

    let params: HashMap<String, String> = HashMap::new();
    let response_str = send_jsonrpc("history_clear", params, Some("1"))
        .map_err(|e| ActionError::general(format!("IPC error: {}", e)))?;

    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| ActionError::general(format!("Invalid JSON response: {}", e)))?;

    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(ActionError::general(message));
    }

    if let Some(result) = response.get("result") {
        Ok(ActionResult::simple("history_clear", result.clone()))
    } else {
        Err(ActionError::general("Invalid response from daemon"))
    }
}
