//! action error types

use crate::cli::exit_codes;

/// error returned by action execution
#[derive(Debug, Clone)]
pub struct ActionError {
    /// exit code (maps to JSON-RPC error code via -32000 - code)
    pub code: i32,
    /// error message
    pub message: String,
    /// suggested alternatives (e.g., similar app names)
    pub suggestions: Vec<String>,
}

impl ActionError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            suggestions: Vec::new(),
        }
    }

    pub fn with_suggestions(
        code: i32,
        message: impl Into<String>,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            suggestions,
        }
    }

    pub fn app_not_found(message: impl Into<String>) -> Self {
        Self::new(exit_codes::APP_NOT_FOUND, message)
    }

    pub fn app_not_found_with_suggestions(
        message: impl Into<String>,
        suggestions: Vec<String>,
    ) -> Self {
        Self::with_suggestions(exit_codes::APP_NOT_FOUND, message, suggestions)
    }

    #[allow(dead_code)]
    pub fn window_not_found(message: impl Into<String>) -> Self {
        Self::new(exit_codes::WINDOW_NOT_FOUND, message)
    }

    #[allow(dead_code)]
    pub fn display_not_found(message: impl Into<String>) -> Self {
        Self::new(exit_codes::DISPLAY_NOT_FOUND, message)
    }

    pub fn invalid_args(message: impl Into<String>) -> Self {
        Self::new(exit_codes::INVALID_ARGS, message)
    }

    #[allow(dead_code)]
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(exit_codes::PERMISSION_DENIED, message)
    }

    pub fn not_supported(message: impl Into<String>) -> Self {
        Self::new(exit_codes::ERROR, message)
    }

    pub fn general(message: impl Into<String>) -> Self {
        Self::new(exit_codes::ERROR, message)
    }

    /// check if this error has suggestions
    #[allow(dead_code)]
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ActionError {}

impl From<anyhow::Error> for ActionError {
    fn from(e: anyhow::Error) -> Self {
        ActionError::general(e.to_string())
    }
}
