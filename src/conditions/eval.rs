//! condition evaluator
//!
//! evaluates parsed conditions against the current system state

use super::time::{is_day_spec_match, is_time_in_ranges, parse_time_ranges};
use super::types::{CompareOp, Condition, FieldCondition, Value};
use crate::display::DisplayInfo;
use crate::window::matching::AppInfo;

/// window state information for condition evaluation
#[derive(Debug, Clone, Default)]
pub struct WindowState {
    /// window title
    pub title: Option<String>,
    /// display index where window is located
    pub display_index: Option<usize>,
    /// display alias/name where window is located
    pub display_name: Option<String>,
    /// is window in fullscreen mode
    pub is_fullscreen: bool,
    /// is window minimized
    pub is_minimized: bool,
}

/// context for evaluating conditions
#[derive(Debug)]
pub struct EvalContext<'a> {
    /// connected displays
    pub displays: &'a [DisplayInfo],
    /// display aliases from config (name -> list of unique IDs)
    pub display_aliases: &'a std::collections::HashMap<String, Vec<String>>,
    /// running applications
    pub running_apps: &'a [AppInfo],
    /// currently focused app name
    pub focused_app: Option<&'a str>,
    /// target app name (the app being acted on)
    pub target_app: Option<&'a str>,
    /// target window state
    pub target_window: Option<&'a WindowState>,
}

impl<'a> EvalContext<'a> {
    /// create a new evaluation context
    pub fn new(
        displays: &'a [DisplayInfo],
        display_aliases: &'a std::collections::HashMap<String, Vec<String>>,
        running_apps: &'a [AppInfo],
    ) -> Self {
        Self {
            displays,
            display_aliases,
            running_apps,
            focused_app: None,
            target_app: None,
            target_window: None,
        }
    }

    /// set the focused app
    pub fn with_focused_app(mut self, app: Option<&'a str>) -> Self {
        self.focused_app = app;
        self
    }

    /// set the target app
    pub fn with_target_app(mut self, app: Option<&'a str>) -> Self {
        self.target_app = app;
        self
    }

    /// set the target window state
    pub fn with_target_window(mut self, window: Option<&'a WindowState>) -> Self {
        self.target_window = window;
        self
    }
}

/// evaluate a condition against the given context
pub fn evaluate(condition: &Condition, ctx: &EvalContext) -> bool {
    match condition {
        Condition::All(conditions) => {
            // empty All = true (vacuous truth)
            conditions.iter().all(|c| evaluate(c, ctx))
        }
        Condition::Any(conditions) => {
            // empty Any = false
            conditions.iter().any(|c| evaluate(c, ctx))
        }
        Condition::Not(inner) => !evaluate(inner, ctx),
        Condition::Field(fc) => evaluate_field(fc, ctx),
        Condition::Ref(name) => {
            // refs should be resolved during parsing
            // if we get here, it's an error - treat as false
            eprintln!("warning: unresolved condition reference: {}", name);
            false
        }
    }
}

fn evaluate_field(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    match fc.field.as_str() {
        // time conditions
        "time" => evaluate_time(fc, ctx),
        "time.day" => evaluate_time_day(fc),

        // display conditions
        "display.count" => evaluate_display_count(fc, ctx),
        "display.connected" => evaluate_display_connected(fc, ctx),

        // app conditions
        "app" => evaluate_app(fc, ctx),
        "app.running" => evaluate_app_running(fc, ctx),
        "app.focused" => evaluate_app_focused(fc, ctx),
        "app.fullscreen" => evaluate_app_fullscreen(fc, ctx),
        "app.minimized" => evaluate_app_minimized(fc, ctx),
        "app.display" => evaluate_app_display(fc, ctx),

        // unknown field
        _ => {
            eprintln!("warning: unknown condition field: {}", fc.field);
            false
        }
    }
}

// ============================================================================
// Time Conditions
// ============================================================================

fn evaluate_time(fc: &FieldCondition, _ctx: &EvalContext) -> bool {
    match &fc.value {
        Value::String(s) => {
            // parse time range(s) and check if current time is within
            parse_time_ranges(s)
                .map(|ranges| is_time_in_ranges(&ranges))
                .unwrap_or(false)
        }
        Value::List(list) if fc.op == CompareOp::In => {
            // multiple time ranges with 'in' operator
            for v in list {
                if let Value::String(s) = v {
                    if let Some(ranges) = parse_time_ranges(s) {
                        if is_time_in_ranges(&ranges) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

fn evaluate_time_day(fc: &FieldCondition) -> bool {
    match &fc.value {
        Value::String(s) => is_day_spec_match(s),
        Value::List(list) if fc.op == CompareOp::In => {
            // multiple day specs with 'in' operator
            for v in list {
                if let Value::String(s) = v {
                    if is_day_spec_match(s) {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

// ============================================================================
// Display Conditions
// ============================================================================

fn evaluate_display_count(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    let count = ctx.displays.len() as i64;
    compare_number(fc.op, count, &fc.value)
}

fn evaluate_display_connected(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    match &fc.value {
        Value::String(alias) => is_display_connected(alias, ctx),
        Value::List(list) if fc.op == CompareOp::In => {
            // any of the displays connected
            list.iter().any(|v| {
                if let Value::String(alias) = v {
                    is_display_connected(alias, ctx)
                } else {
                    false
                }
            })
        }
        _ => false,
    }
}

fn is_display_connected(alias: &str, ctx: &EvalContext) -> bool {
    let alias_lower = alias.to_lowercase();

    // check system aliases first
    match alias_lower.as_str() {
        "builtin" => return ctx.displays.iter().any(|d| d.is_builtin),
        "external" => return ctx.displays.iter().any(|d| !d.is_builtin),
        "main" => return ctx.displays.iter().any(|d| d.is_main),
        _ => {}
    }

    // check user-defined aliases
    if let Some(ids) = ctx.display_aliases.get(&alias_lower) {
        for display in ctx.displays {
            let unique_id = display.unique_id();
            if ids.iter().any(|id| id.eq_ignore_ascii_case(&unique_id)) {
                return true;
            }
        }
    }

    // check by display name
    ctx.displays
        .iter()
        .any(|d| d.name.to_lowercase() == alias_lower)
}

// ============================================================================
// App Conditions
// ============================================================================

fn evaluate_app(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    // matches target app name or window title
    let target_app = match ctx.target_app {
        Some(app) => app,
        None => return false,
    };

    match &fc.value {
        Value::String(pattern) => app_matches(target_app, ctx.target_window, pattern),
        Value::List(list) if fc.op == CompareOp::In => list.iter().any(|v| {
            if let Value::String(pattern) = v {
                app_matches(target_app, ctx.target_window, pattern)
            } else {
                false
            }
        }),
        _ => false,
    }
}

fn app_matches(app_name: &str, window: Option<&WindowState>, pattern: &str) -> bool {
    // check if pattern is a regex
    if pattern.starts_with('/') && pattern.len() > 2 {
        if let Some(end) = pattern[1..].rfind('/') {
            let regex_str = &pattern[1..=end];
            let flags = &pattern[end + 2..];
            let case_insensitive = flags.contains('i');

            if let Ok(re) = if case_insensitive {
                regex::RegexBuilder::new(regex_str)
                    .case_insensitive(true)
                    .build()
            } else {
                regex::Regex::new(regex_str)
            } {
                // match against app name
                if re.is_match(app_name) {
                    return true;
                }
                // match against window title
                if let Some(w) = window {
                    if let Some(title) = &w.title {
                        if re.is_match(title) {
                            return true;
                        }
                    }
                }
            }
            return false;
        }
    }

    // exact or prefix match (case-insensitive)
    let pattern_lower = pattern.to_lowercase();
    let app_lower = app_name.to_lowercase();

    if app_lower == pattern_lower || app_lower.starts_with(&pattern_lower) {
        return true;
    }

    // check window title
    if let Some(w) = window {
        if let Some(title) = &w.title {
            let title_lower = title.to_lowercase();
            if title_lower == pattern_lower
                || title_lower.starts_with(&pattern_lower)
                || title_lower.contains(&pattern_lower)
            {
                return true;
            }
        }
    }

    false
}

fn evaluate_app_running(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    match &fc.value {
        Value::String(app_name) => is_app_running(app_name, ctx),
        Value::List(list) if fc.op == CompareOp::In => list.iter().any(|v| {
            if let Value::String(app_name) = v {
                is_app_running(app_name, ctx)
            } else {
                false
            }
        }),
        _ => false,
    }
}

fn is_app_running(app_name: &str, ctx: &EvalContext) -> bool {
    let name_lower = app_name.to_lowercase();
    ctx.running_apps.iter().any(|app| {
        app.name.to_lowercase() == name_lower || app.name.to_lowercase().starts_with(&name_lower)
    })
}

fn evaluate_app_focused(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    match &fc.value {
        Value::Bool(b) => {
            // { "app.focused": true } - something is focused
            // { "app.focused": false } - nothing is focused
            let has_focus = ctx.focused_app.is_some();
            if fc.op == CompareOp::Eq {
                has_focus == *b
            } else {
                has_focus != *b
            }
        }
        Value::String(app_name) => {
            // { "app.focused": "Safari" } - Safari is focused
            let name_lower = app_name.to_lowercase();
            ctx.focused_app
                .map(|f| {
                    f.to_lowercase() == name_lower || f.to_lowercase().starts_with(&name_lower)
                })
                .unwrap_or(false)
        }
        Value::List(list) if fc.op == CompareOp::In => {
            // { "app.focused": { "in": ["Safari", "Chrome"] } }
            let focused = match ctx.focused_app {
                Some(f) => f.to_lowercase(),
                None => return false,
            };
            list.iter().any(|v| {
                if let Value::String(app_name) = v {
                    let name_lower = app_name.to_lowercase();
                    focused == name_lower || focused.starts_with(&name_lower)
                } else {
                    false
                }
            })
        }
        _ => false,
    }
}

fn evaluate_app_fullscreen(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    let is_fullscreen = ctx.target_window.map(|w| w.is_fullscreen).unwrap_or(false);

    match &fc.value {
        Value::Bool(expected) => {
            if fc.op == CompareOp::Eq {
                is_fullscreen == *expected
            } else {
                is_fullscreen != *expected
            }
        }
        _ => false,
    }
}

fn evaluate_app_minimized(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    let is_minimized = ctx.target_window.map(|w| w.is_minimized).unwrap_or(false);

    match &fc.value {
        Value::Bool(expected) => {
            if fc.op == CompareOp::Eq {
                is_minimized == *expected
            } else {
                is_minimized != *expected
            }
        }
        _ => false,
    }
}

fn evaluate_app_display(fc: &FieldCondition, ctx: &EvalContext) -> bool {
    let window = match ctx.target_window {
        Some(w) => w,
        None => return false,
    };

    match &fc.value {
        Value::String(alias) => window_on_display(window, alias, ctx),
        Value::List(list) if fc.op == CompareOp::In => list.iter().any(|v| {
            if let Value::String(alias) = v {
                window_on_display(window, alias, ctx)
            } else {
                false
            }
        }),
        _ => false,
    }
}

fn window_on_display(window: &WindowState, alias: &str, ctx: &EvalContext) -> bool {
    let alias_lower = alias.to_lowercase();

    // get the display where the window is
    let display = match window.display_index {
        Some(idx) => ctx.displays.get(idx),
        None => None,
    };

    // check system aliases
    if let Some(d) = display {
        match alias_lower.as_str() {
            "builtin" => return d.is_builtin,
            "external" => return !d.is_builtin,
            "main" => return d.is_main,
            _ => {}
        }

        // check user-defined aliases
        if let Some(ids) = ctx.display_aliases.get(&alias_lower) {
            let unique_id = d.unique_id();
            if ids.iter().any(|id| id.eq_ignore_ascii_case(&unique_id)) {
                return true;
            }
        }

        // check by display name
        if d.name.to_lowercase() == alias_lower {
            return true;
        }
    }

    // also check window's display_name if set
    if let Some(name) = &window.display_name {
        if name.to_lowercase() == alias_lower {
            return true;
        }
    }

    false
}

// ============================================================================
// Comparison Helpers
// ============================================================================

fn compare_number(op: CompareOp, actual: i64, expected: &Value) -> bool {
    let expected_num = match expected {
        Value::Number(n) => *n,
        Value::Float(f) => *f as i64,
        _ => return false,
    };

    match op {
        CompareOp::Eq => actual == expected_num,
        CompareOp::Ne => actual != expected_num,
        CompareOp::Gt => actual > expected_num,
        CompareOp::Gte => actual >= expected_num,
        CompareOp::Lt => actual < expected_num,
        CompareOp::Lte => actual <= expected_num,
        CompareOp::In => {
            if let Value::List(list) = expected {
                list.iter().any(|v| {
                    if let Value::Number(n) = v {
                        actual == *n
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_context<'a>(displays: &'a [DisplayInfo], apps: &'a [AppInfo]) -> EvalContext<'a> {
        static EMPTY_ALIASES: std::sync::LazyLock<HashMap<String, Vec<String>>> =
            std::sync::LazyLock::new(HashMap::new);
        EvalContext::new(displays, &EMPTY_ALIASES, apps)
    }

    #[test]
    fn test_evaluate_all_empty() {
        let cond = Condition::All(vec![]);
        let ctx = make_context(&[], &[]);
        assert!(evaluate(&cond, &ctx)); // empty AND = true
    }

    #[test]
    fn test_evaluate_any_empty() {
        let cond = Condition::Any(vec![]);
        let ctx = make_context(&[], &[]);
        assert!(!evaluate(&cond, &ctx)); // empty OR = false
    }

    #[test]
    fn test_evaluate_not() {
        let cond = Condition::Not(Box::new(Condition::All(vec![])));
        let ctx = make_context(&[], &[]);
        assert!(!evaluate(&cond, &ctx)); // NOT true = false
    }

    #[test]
    fn test_evaluate_display_count() {
        let displays = vec![
            DisplayInfo {
                index: 0,
                name: "Built-in".to_string(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                is_main: true,
                display_id: 1,
                vendor_id: None,
                model_id: None,
                serial_number: None,
                unit_number: 0,
                is_builtin: true,
            },
            DisplayInfo {
                index: 1,
                name: "External".to_string(),
                width: 2560,
                height: 1440,
                x: 1920,
                y: 0,
                is_main: false,
                display_id: 2,
                vendor_id: None,
                model_id: None,
                serial_number: None,
                unit_number: 0,
                is_builtin: false,
            },
        ];

        let ctx = make_context(&displays, &[]);

        // display.count == 2
        let cond = Condition::Field(FieldCondition::eq("display.count", Value::Number(2)));
        assert!(evaluate(&cond, &ctx));

        // display.count >= 2
        let cond = Condition::Field(FieldCondition::new(
            "display.count",
            CompareOp::Gte,
            Value::Number(2),
        ));
        assert!(evaluate(&cond, &ctx));

        // display.count > 2
        let cond = Condition::Field(FieldCondition::new(
            "display.count",
            CompareOp::Gt,
            Value::Number(2),
        ));
        assert!(!evaluate(&cond, &ctx));
    }

    #[test]
    fn test_evaluate_display_connected() {
        let displays = vec![
            DisplayInfo {
                index: 0,
                name: "Built-in".to_string(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                is_main: true,
                display_id: 1,
                vendor_id: None,
                model_id: None,
                serial_number: None,
                unit_number: 0,
                is_builtin: true,
            },
            DisplayInfo {
                index: 1,
                name: "Dell U2720Q".to_string(),
                width: 2560,
                height: 1440,
                x: 1920,
                y: 0,
                is_main: false,
                display_id: 2,
                vendor_id: None,
                model_id: None,
                serial_number: None,
                unit_number: 0,
                is_builtin: false,
            },
        ];

        let ctx = make_context(&displays, &[]);

        // builtin connected
        let cond = Condition::Field(FieldCondition::eq(
            "display.connected",
            Value::String("builtin".to_string()),
        ));
        assert!(evaluate(&cond, &ctx));

        // external connected
        let cond = Condition::Field(FieldCondition::eq(
            "display.connected",
            Value::String("external".to_string()),
        ));
        assert!(evaluate(&cond, &ctx));

        // main connected
        let cond = Condition::Field(FieldCondition::eq(
            "display.connected",
            Value::String("main".to_string()),
        ));
        assert!(evaluate(&cond, &ctx));
    }

    #[test]
    fn test_evaluate_app_running() {
        let apps = vec![
            AppInfo {
                name: "Safari".to_string(),
                pid: 123,
                bundle_id: Some("com.apple.Safari".to_string()),
                titles: vec![],
            },
            AppInfo {
                name: "Terminal".to_string(),
                pid: 456,
                bundle_id: Some("com.apple.Terminal".to_string()),
                titles: vec![],
            },
        ];

        let ctx = make_context(&[], &apps);

        // Safari running
        let cond = Condition::Field(FieldCondition::eq(
            "app.running",
            Value::String("Safari".to_string()),
        ));
        assert!(evaluate(&cond, &ctx));

        // Chrome not running
        let cond = Condition::Field(FieldCondition::eq(
            "app.running",
            Value::String("Chrome".to_string()),
        ));
        assert!(!evaluate(&cond, &ctx));

        // any of Safari, Chrome running
        let cond = Condition::Field(FieldCondition::is_in(
            "app.running",
            vec![
                Value::String("Safari".to_string()),
                Value::String("Chrome".to_string()),
            ],
        ));
        assert!(evaluate(&cond, &ctx));
    }

    #[test]
    fn test_evaluate_app_focused() {
        let ctx = make_context(&[], &[]).with_focused_app(Some("Safari"));

        // Safari focused
        let cond = Condition::Field(FieldCondition::eq(
            "app.focused",
            Value::String("Safari".to_string()),
        ));
        assert!(evaluate(&cond, &ctx));

        // Chrome not focused
        let cond = Condition::Field(FieldCondition::eq(
            "app.focused",
            Value::String("Chrome".to_string()),
        ));
        assert!(!evaluate(&cond, &ctx));

        // something focused (bool)
        let cond = Condition::Field(FieldCondition::eq("app.focused", Value::Bool(true)));
        assert!(evaluate(&cond, &ctx));
    }

    #[test]
    fn test_evaluate_app_fullscreen() {
        let window = WindowState {
            is_fullscreen: true,
            ..Default::default()
        };
        let ctx = make_context(&[], &[]).with_target_window(Some(&window));

        let cond = Condition::Field(FieldCondition::eq("app.fullscreen", Value::Bool(true)));
        assert!(evaluate(&cond, &ctx));

        let cond = Condition::Field(FieldCondition::eq("app.fullscreen", Value::Bool(false)));
        assert!(!evaluate(&cond, &ctx));
    }

    #[test]
    fn test_evaluate_complex_condition() {
        let displays = vec![DisplayInfo {
            index: 0,
            name: "External".to_string(),
            width: 2560,
            height: 1440,
            x: 0,
            y: 0,
            is_main: true,
            display_id: 1,
            vendor_id: None,
            model_id: None,
            serial_number: None,
            unit_number: 0,
            is_builtin: false,
        }];

        let apps = vec![AppInfo {
            name: "Slack".to_string(),
            pid: 123,
            bundle_id: None,
            titles: vec![],
        }];

        let window = WindowState {
            is_fullscreen: false,
            ..Default::default()
        };

        let ctx = make_context(&displays, &apps)
            .with_target_app(Some("Terminal"))
            .with_target_window(Some(&window));

        // (display.count >= 1) AND (app.running: Slack) AND (NOT app.fullscreen)
        let cond = Condition::All(vec![
            Condition::Field(FieldCondition::new(
                "display.count",
                CompareOp::Gte,
                Value::Number(1),
            )),
            Condition::Field(FieldCondition::eq(
                "app.running",
                Value::String("Slack".to_string()),
            )),
            Condition::Not(Box::new(Condition::Field(FieldCondition::eq(
                "app.fullscreen",
                Value::Bool(true),
            )))),
        ]);

        assert!(evaluate(&cond, &ctx));
    }
}
