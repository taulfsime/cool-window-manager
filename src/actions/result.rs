//! action result types - unified response format for all interfaces

use serde::Serialize;

use crate::window::matching::{AppInfo, MatchResult, MatchType};

/// result of an action execution
#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    /// action that was performed
    pub action: &'static str,
    /// result data (varies by action type)
    #[serde(flatten)]
    pub data: ActionData,
}

/// action-specific result data
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ActionData {
    /// focus action result
    Focus {
        app: AppData,
        #[serde(rename = "match")]
        match_info: MatchData,
    },

    /// maximize action result
    Maximize {
        app: AppData,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "match")]
        match_info: Option<MatchData>,
    },

    /// resize action result
    Resize {
        app: AppData,
        size: SizeData,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "match")]
        match_info: Option<MatchData>,
    },

    /// move action result
    Move {
        app: AppData,
        position: PositionData,
        display: DisplayData,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "match")]
        match_info: Option<MatchData>,
    },

    /// kill action result
    Kill {
        app: AppData,
        #[serde(rename = "match")]
        match_info: MatchData,
        force: bool,
        terminated: bool,
    },

    /// close action result
    Close {
        app: AppData,
        #[serde(rename = "match")]
        match_info: MatchData,
        windows_closed: usize,
    },

    /// list action result
    List { items: Vec<serde_json::Value> },

    /// get window info result
    Get {
        app: AppData,
        window: WindowData,
        display: DisplayData,
    },

    /// launched app (when app was not running and was launched)
    Launched { app: String, message: String },

    /// undo action result
    Undo { app: AppData, restored: WindowData },

    /// redo action result
    Redo { app: AppData, restored: WindowData },

    /// simple result with arbitrary JSON data (for system commands, etc.)
    Simple { result: serde_json::Value },
}

/// basic app information
#[derive(Debug, Clone, Serialize)]
pub struct AppData {
    pub name: String,
    pub pid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
}

impl From<&AppInfo> for AppData {
    fn from(app: &AppInfo) -> Self {
        Self {
            name: app.name.clone(),
            pid: app.pid,
            bundle_id: app.bundle_id.clone(),
        }
    }
}

/// match information showing how an app was found
#[derive(Debug, Clone, Serialize)]
pub struct MatchData {
    #[serde(rename = "type")]
    pub match_type: String,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<usize>,
}

impl MatchData {
    pub fn from_match_result(result: &MatchResult, query: &str) -> Self {
        let (match_type, distance) = match &result.match_type {
            MatchType::Exact => ("exact", None),
            MatchType::Prefix => ("prefix", None),
            MatchType::Regex { .. } => ("regex", None),
            MatchType::Fuzzy { distance } => ("fuzzy", Some(*distance)),
            MatchType::TitleExact { .. } => ("title_exact", None),
            MatchType::TitlePrefix { .. } => ("title_prefix", None),
            MatchType::TitleRegex { .. } => ("title_regex", None),
            MatchType::TitleFuzzy { distance, .. } => ("title_fuzzy", Some(*distance)),
        };

        Self {
            match_type: match_type.to_string(),
            query: query.to_string(),
            distance,
        }
    }
}

/// size information for resize results
#[derive(Debug, Clone, Serialize)]
pub struct SizeData {
    pub width: u32,
    pub height: u32,
}

/// position information for move results
#[derive(Debug, Clone, Serialize)]
pub struct PositionData {
    pub x: i32,
    pub y: i32,
}

/// display information
#[derive(Debug, Clone, Serialize)]
pub struct DisplayData {
    pub index: usize,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// window geometry information
#[derive(Debug, Clone, Serialize)]
pub struct WindowData {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

// helper constructors for ActionResult
impl ActionResult {
    pub fn focus(app: AppData, match_info: MatchData) -> Self {
        Self {
            action: "focus",
            data: ActionData::Focus { app, match_info },
        }
    }

    pub fn maximize(app: AppData, match_info: Option<MatchData>) -> Self {
        Self {
            action: "maximize",
            data: ActionData::Maximize { app, match_info },
        }
    }

    pub fn resize(app: AppData, size: SizeData, match_info: Option<MatchData>) -> Self {
        Self {
            action: "resize",
            data: ActionData::Resize {
                app,
                size,
                match_info,
            },
        }
    }

    pub fn move_window(
        app: AppData,
        position: PositionData,
        display: DisplayData,
        match_info: Option<MatchData>,
    ) -> Self {
        Self {
            action: "move",
            data: ActionData::Move {
                app,
                position,
                display,
                match_info,
            },
        }
    }

    pub fn list(action: &'static str, items: Vec<serde_json::Value>) -> Self {
        Self {
            action,
            data: ActionData::List { items },
        }
    }

    pub fn get(app: AppData, window: WindowData, display: DisplayData) -> Self {
        Self {
            action: "get",
            data: ActionData::Get {
                app,
                window,
                display,
            },
        }
    }

    pub fn launched(app: String) -> Self {
        Self {
            action: "focus",
            data: ActionData::Launched {
                message: format!("{} launched, run command again once ready", app),
                app,
            },
        }
    }

    pub fn kill(app: AppData, match_info: MatchData, force: bool, terminated: bool) -> Self {
        Self {
            action: "kill",
            data: ActionData::Kill {
                app,
                match_info,
                force,
                terminated,
            },
        }
    }

    pub fn close(app: AppData, match_info: MatchData, windows_closed: usize) -> Self {
        Self {
            action: "close",
            data: ActionData::Close {
                app,
                match_info,
                windows_closed,
            },
        }
    }

    /// create a simple result with arbitrary JSON data
    pub fn simple(action: &'static str, result: serde_json::Value) -> Self {
        Self {
            action,
            data: ActionData::Simple { result },
        }
    }

    pub fn undo(app: AppData, restored: WindowData) -> Self {
        Self {
            action: "undo",
            data: ActionData::Undo { app, restored },
        }
    }

    pub fn redo(app: AppData, restored: WindowData) -> Self {
        Self {
            action: "redo",
            data: ActionData::Redo { app, restored },
        }
    }
}
