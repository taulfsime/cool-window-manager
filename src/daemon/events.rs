//! event system for daemon
//!
//! provides event types, event bus for managing subscribers, and event emission.
//! external tools can subscribe to events via persistent socket connections.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::mpsc;

// ============================================================================
// Event Types
// ============================================================================

/// all supported event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// application was launched (detected by app watcher)
    #[serde(rename = "app.launched")]
    AppLaunched,
    /// application was focused by cwm
    #[serde(rename = "app.focused")]
    AppFocused,
    /// application was terminated by cwm
    #[serde(rename = "app.terminated")]
    AppTerminated,
    /// window was maximized by cwm
    #[serde(rename = "window.maximized")]
    WindowMaximized,
    /// window was resized by cwm
    #[serde(rename = "window.resized")]
    WindowResized,
    /// window was moved by cwm
    #[serde(rename = "window.moved")]
    WindowMoved,
    /// window was closed by cwm
    #[serde(rename = "window.closed")]
    WindowClosed,
    /// display was connected
    #[serde(rename = "display.connected")]
    DisplayConnected,
    /// display was disconnected
    #[serde(rename = "display.disconnected")]
    DisplayDisconnected,
}

impl EventType {
    /// get the string representation of this event type
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::AppLaunched => "app.launched",
            EventType::AppFocused => "app.focused",
            EventType::AppTerminated => "app.terminated",
            EventType::WindowMaximized => "window.maximized",
            EventType::WindowResized => "window.resized",
            EventType::WindowMoved => "window.moved",
            EventType::WindowClosed => "window.closed",
            EventType::DisplayConnected => "display.connected",
            EventType::DisplayDisconnected => "display.disconnected",
        }
    }

    /// get all event types
    pub fn all() -> &'static [EventType] {
        &[
            EventType::AppLaunched,
            EventType::AppFocused,
            EventType::AppTerminated,
            EventType::WindowMaximized,
            EventType::WindowResized,
            EventType::WindowMoved,
            EventType::WindowClosed,
            EventType::DisplayConnected,
            EventType::DisplayDisconnected,
        ]
    }

    /// get description for this event type
    pub fn description(&self) -> &'static str {
        match self {
            EventType::AppLaunched => "Application was launched (detected by app watcher)",
            EventType::AppFocused => "Application was focused by cwm",
            EventType::AppTerminated => "Application was terminated by cwm",
            EventType::WindowMaximized => "Window was maximized by cwm",
            EventType::WindowResized => "Window was resized by cwm",
            EventType::WindowMoved => "Window was moved by cwm",
            EventType::WindowClosed => "Window was closed by cwm",
            EventType::DisplayConnected => "Display was connected",
            EventType::DisplayDisconnected => "Display was disconnected",
        }
    }

    /// parse event type from string
    pub fn parse(s: &str) -> Option<EventType> {
        match s {
            "app.launched" => Some(EventType::AppLaunched),
            "app.focused" => Some(EventType::AppFocused),
            "app.terminated" => Some(EventType::AppTerminated),
            "window.maximized" => Some(EventType::WindowMaximized),
            "window.resized" => Some(EventType::WindowResized),
            "window.moved" => Some(EventType::WindowMoved),
            "window.closed" => Some(EventType::WindowClosed),
            "display.connected" => Some(EventType::DisplayConnected),
            "display.disconnected" => Some(EventType::DisplayDisconnected),
            _ => None,
        }
    }

    /// check if this event type matches a filter pattern
    /// patterns: "*", "app.*", "window.*", "daemon.*", or exact match
    pub fn matches_filter(&self, filter: &str) -> bool {
        let type_str = self.as_str();

        if filter == "*" {
            return true;
        }

        if let Some(prefix) = filter.strip_suffix(".*") {
            return type_str.starts_with(prefix) && type_str.len() > prefix.len();
        }

        type_str == filter
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Event Data
// ============================================================================

/// event-specific data
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum EventData {
    /// app event data (app.launched, app.focused)
    App {
        app: String,
        pid: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        titles: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        match_type: Option<String>,
    },

    /// app terminated event data
    AppTerminated { app: String, pid: i32, force: bool },

    /// window event data (window.maximized, window.resized, window.moved)
    Window {
        app: String,
        pid: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        titles: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        x: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        y: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        windows_closed: Option<usize>,
    },

    /// display event data (display.connected, display.disconnected)
    Display {
        index: usize,
        name: String,
        unique_id: String,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        is_main: bool,
        is_builtin: bool,
        aliases: Vec<String>,
    },
}

impl EventData {
    /// create app event data
    pub fn app(app: String, pid: i32) -> Self {
        EventData::App {
            app,
            pid,
            titles: None,
            match_type: None,
        }
    }

    /// create app event data with titles
    #[allow(dead_code)]
    pub fn app_with_titles(app: String, pid: i32, titles: Vec<String>) -> Self {
        EventData::App {
            app,
            pid,
            titles: Some(titles),
            match_type: None,
        }
    }

    /// create app focused event data with match type
    pub fn app_focused(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        match_type: String,
    ) -> Self {
        EventData::App {
            app,
            pid,
            titles,
            match_type: Some(match_type),
        }
    }

    /// create app terminated event data
    pub fn app_terminated(app: String, pid: i32, force: bool) -> Self {
        EventData::AppTerminated { app, pid, force }
    }

    /// create window maximized event data
    pub fn window_maximized(app: String, pid: i32, titles: Option<Vec<String>>) -> Self {
        EventData::Window {
            app,
            pid,
            titles,
            width: None,
            height: None,
            x: None,
            y: None,
            display: None,
            windows_closed: None,
        }
    }

    /// create window resized event data
    pub fn window_resized(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        width: i32,
        height: i32,
    ) -> Self {
        EventData::Window {
            app,
            pid,
            titles,
            width: Some(width),
            height: Some(height),
            x: None,
            y: None,
            display: None,
            windows_closed: None,
        }
    }

    /// create window moved event data
    pub fn window_moved(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        x: i32,
        y: i32,
        display: Option<String>,
    ) -> Self {
        EventData::Window {
            app,
            pid,
            titles,
            width: None,
            height: None,
            x: Some(x),
            y: Some(y),
            display,
            windows_closed: None,
        }
    }

    /// create window closed event data
    pub fn window_closed(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        windows_closed: usize,
    ) -> Self {
        EventData::Window {
            app,
            pid,
            titles,
            width: None,
            height: None,
            x: None,
            y: None,
            display: None,
            windows_closed: Some(windows_closed),
        }
    }

    /// create display event data
    #[allow(clippy::too_many_arguments)]
    pub fn display(
        index: usize,
        name: String,
        unique_id: String,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        is_main: bool,
        is_builtin: bool,
        aliases: Vec<String>,
    ) -> Self {
        EventData::Display {
            index,
            name,
            unique_id,
            width,
            height,
            x,
            y,
            is_main,
            is_builtin,
            aliases,
        }
    }

    /// get the app name if this event has one
    pub fn app_name(&self) -> Option<&str> {
        match self {
            EventData::App { app, .. } => Some(app),
            EventData::AppTerminated { app, .. } => Some(app),
            EventData::Window { app, .. } => Some(app),
            EventData::Display { .. } => None,
        }
    }

    /// get the window titles if this event has them
    pub fn titles(&self) -> Option<&[String]> {
        match self {
            EventData::App { titles, .. } => titles.as_deref(),
            EventData::AppTerminated { .. } => None,
            EventData::Window { titles, .. } => titles.as_deref(),
            EventData::Display { .. } => None,
        }
    }
}

// ============================================================================
// Event
// ============================================================================

/// a single event
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    /// event type
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// timestamp (ISO 8601)
    pub ts: DateTime<Utc>,
    /// event-specific data
    pub data: EventData,
}

impl Event {
    /// create a new event
    pub fn new(event_type: EventType, data: EventData) -> Self {
        Self {
            event_type,
            ts: Utc::now(),
            data,
        }
    }

    /// create app.launched event
    pub fn app_launched(app: String, pid: i32) -> Self {
        Self::new(EventType::AppLaunched, EventData::app(app, pid))
    }

    /// create app.launched event with titles
    #[allow(dead_code)]
    pub fn app_launched_with_titles(app: String, pid: i32, titles: Vec<String>) -> Self {
        Self::new(
            EventType::AppLaunched,
            EventData::app_with_titles(app, pid, titles),
        )
    }

    /// create app.focused event
    pub fn app_focused(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        match_type: String,
    ) -> Self {
        Self::new(
            EventType::AppFocused,
            EventData::app_focused(app, pid, titles, match_type),
        )
    }

    /// create app.terminated event
    pub fn app_terminated(app: String, pid: i32, force: bool) -> Self {
        Self::new(
            EventType::AppTerminated,
            EventData::app_terminated(app, pid, force),
        )
    }

    /// create window.maximized event
    pub fn window_maximized(app: String, pid: i32, titles: Option<Vec<String>>) -> Self {
        Self::new(
            EventType::WindowMaximized,
            EventData::window_maximized(app, pid, titles),
        )
    }

    /// create window.resized event
    pub fn window_resized(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        width: i32,
        height: i32,
    ) -> Self {
        Self::new(
            EventType::WindowResized,
            EventData::window_resized(app, pid, titles, width, height),
        )
    }

    /// create window.moved event
    pub fn window_moved(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        x: i32,
        y: i32,
        display: Option<String>,
    ) -> Self {
        Self::new(
            EventType::WindowMoved,
            EventData::window_moved(app, pid, titles, x, y, display),
        )
    }

    /// create window.closed event
    pub fn window_closed(
        app: String,
        pid: i32,
        titles: Option<Vec<String>>,
        windows_closed: usize,
    ) -> Self {
        Self::new(
            EventType::WindowClosed,
            EventData::window_closed(app, pid, titles, windows_closed),
        )
    }

    /// create display.connected event
    #[allow(clippy::too_many_arguments)]
    pub fn display_connected(
        index: usize,
        name: String,
        unique_id: String,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        is_main: bool,
        is_builtin: bool,
        aliases: Vec<String>,
    ) -> Self {
        Self::new(
            EventType::DisplayConnected,
            EventData::display(
                index, name, unique_id, width, height, x, y, is_main, is_builtin, aliases,
            ),
        )
    }

    /// create display.disconnected event
    #[allow(clippy::too_many_arguments)]
    pub fn display_disconnected(
        index: usize,
        name: String,
        unique_id: String,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        is_main: bool,
        is_builtin: bool,
        aliases: Vec<String>,
    ) -> Self {
        Self::new(
            EventType::DisplayDisconnected,
            EventData::display(
                index, name, unique_id, width, height, x, y, is_main, is_builtin, aliases,
            ),
        )
    }

    /// check if this event matches the given filters
    pub fn matches_filters(&self, event_filters: &[String], app_filters: &[String]) -> bool {
        // check event type filter
        if !event_filters.is_empty() {
            let matches_event = event_filters
                .iter()
                .any(|f| self.event_type.matches_filter(f));
            if !matches_event {
                return false;
            }
        }

        // check app filter
        if !app_filters.is_empty() {
            if let Some(app_name) = self.data.app_name() {
                let titles = self.data.titles().unwrap_or(&[]);
                let matches_app = app_filters
                    .iter()
                    .any(|f| matches_app_filter(app_name, titles, f));
                if !matches_app {
                    return false;
                }
            } else {
                // event has no app, but app filter is specified
                return false;
            }
        }

        true
    }

    /// format event as JSON-RPC notification
    pub fn to_jsonrpc_notification(&self) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": self,
        })
        .to_string()
    }
}

/// check if app name or any title matches the filter
/// supports exact match, prefix match, and regex (/pattern/ or /pattern/i)
fn matches_app_filter(app_name: &str, titles: &[String], filter: &str) -> bool {
    // check if filter is a regex pattern
    if let Some(stripped) = filter.strip_prefix('/') {
        if let Some(end_slash) = stripped.rfind('/') {
            let pattern = &stripped[..end_slash];
            let flags = &stripped[end_slash + 1..];
            let case_insensitive = flags.contains('i');

            let regex = if case_insensitive {
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
            } else {
                regex::Regex::new(pattern)
            };

            if let Ok(re) = regex {
                // match against app name
                if re.is_match(app_name) {
                    return true;
                }
                // match against titles
                for title in titles {
                    if re.is_match(title) {
                        return true;
                    }
                }
            }
            return false;
        }
    }

    // exact match (case-insensitive)
    let filter_lower = filter.to_lowercase();
    let app_lower = app_name.to_lowercase();

    if app_lower == filter_lower {
        return true;
    }

    // prefix match
    if app_lower.starts_with(&filter_lower) {
        return true;
    }

    // check titles
    for title in titles {
        let title_lower = title.to_lowercase();
        if title_lower == filter_lower || title_lower.starts_with(&filter_lower) {
            return true;
        }
    }

    false
}

// ============================================================================
// Subscriber
// ============================================================================

/// a subscriber to events
struct Subscriber {
    /// unique subscriber id
    #[allow(dead_code)]
    id: u64,
    /// event type filters (empty = all events)
    event_filters: Vec<String>,
    /// app name filters (empty = all apps)
    app_filters: Vec<String>,
    /// channel to send events to
    sender: mpsc::UnboundedSender<Event>,
}

impl Subscriber {
    /// check if this subscriber wants the given event
    fn wants_event(&self, event: &Event) -> bool {
        event.matches_filters(&self.event_filters, &self.app_filters)
    }
}

// ============================================================================
// EventBus
// ============================================================================

/// manages event subscribers and broadcasts events
pub struct EventBus {
    /// next subscriber id
    next_id: AtomicU64,
    /// active subscribers
    subscribers: RwLock<HashMap<u64, Subscriber>>,
    /// recent events for debugging (limited buffer)
    #[allow(dead_code)]
    recent_events: Mutex<Vec<Event>>,
}

impl EventBus {
    /// create a new event bus
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            subscribers: RwLock::new(HashMap::new()),
            recent_events: Mutex::new(Vec::new()),
        }
    }

    /// subscribe to events with filters
    /// returns (subscription_id, receiver)
    pub fn subscribe(
        &self,
        event_filters: Vec<String>,
        app_filters: Vec<String>,
    ) -> (u64, mpsc::UnboundedReceiver<Event>) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = mpsc::unbounded_channel();

        let subscriber = Subscriber {
            id,
            event_filters,
            app_filters,
            sender,
        };

        if let Ok(mut subs) = self.subscribers.write() {
            subs.insert(id, subscriber);
        }

        (id, receiver)
    }

    /// unsubscribe from events
    pub fn unsubscribe(&self, id: u64) {
        if let Ok(mut subs) = self.subscribers.write() {
            subs.remove(&id);
        }
    }

    /// emit an event to all matching subscribers
    pub fn emit(&self, event: Event) {
        // store in recent events (for debugging)
        if let Ok(mut recent) = self.recent_events.lock() {
            recent.push(event.clone());
            // keep only last 100 events
            if recent.len() > 100 {
                recent.remove(0);
            }
        }

        // broadcast to subscribers
        if let Ok(subs) = self.subscribers.read() {
            for subscriber in subs.values() {
                if subscriber.wants_event(&event) {
                    // ignore send errors (subscriber disconnected)
                    let _ = subscriber.sender.send(event.clone());
                }
            }
        }
    }

    /// get the number of active subscribers
    #[allow(dead_code)]
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.read().map(|s| s.len()).unwrap_or(0)
    }

    /// expand filter patterns to actual event types
    pub fn expand_filters(filters: &[String]) -> Vec<String> {
        if filters.is_empty() {
            // no filters = all events
            return EventType::all()
                .iter()
                .map(|e| e.as_str().to_string())
                .collect();
        }

        let mut result = Vec::new();
        for filter in filters {
            if filter == "*" {
                return EventType::all()
                    .iter()
                    .map(|e| e.as_str().to_string())
                    .collect();
            }

            if filter.ends_with(".*") {
                // expand pattern to matching event types
                for event_type in EventType::all() {
                    if event_type.matches_filter(filter) {
                        result.push(event_type.as_str().to_string());
                    }
                }
            } else if EventType::parse(filter).is_some() {
                result.push(filter.clone());
            }
        }

        result.sort();
        result.dedup();
        result
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Global Event Bus
// ============================================================================

lazy_static::lazy_static! {
    /// global event bus instance
    pub static ref EVENT_BUS: Arc<EventBus> = Arc::new(EventBus::new());
}

/// emit an event to the global event bus
pub fn emit(event: Event) {
    EVENT_BUS.emit(event);
}

/// subscribe to events on the global event bus
pub fn subscribe(
    event_filters: Vec<String>,
    app_filters: Vec<String>,
) -> (u64, mpsc::UnboundedReceiver<Event>) {
    EVENT_BUS.subscribe(event_filters, app_filters)
}

/// unsubscribe from the global event bus
pub fn unsubscribe(id: u64) {
    EVENT_BUS.unsubscribe(id);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::AppLaunched.as_str(), "app.launched");
        assert_eq!(EventType::WindowResized.as_str(), "window.resized");
        assert_eq!(EventType::DisplayConnected.as_str(), "display.connected");
        assert_eq!(
            EventType::DisplayDisconnected.as_str(),
            "display.disconnected"
        );
    }

    #[test]
    fn test_event_type_parse() {
        assert_eq!(
            EventType::parse("app.launched"),
            Some(EventType::AppLaunched)
        );
        assert_eq!(
            EventType::parse("display.connected"),
            Some(EventType::DisplayConnected)
        );
        assert_eq!(
            EventType::parse("display.disconnected"),
            Some(EventType::DisplayDisconnected)
        );
        assert_eq!(EventType::parse("invalid"), None);
    }

    #[test]
    fn test_event_type_matches_filter_exact() {
        assert!(EventType::AppLaunched.matches_filter("app.launched"));
        assert!(!EventType::AppLaunched.matches_filter("app.focused"));
    }

    #[test]
    fn test_event_type_matches_filter_wildcard() {
        assert!(EventType::AppLaunched.matches_filter("*"));
        assert!(EventType::DisplayConnected.matches_filter("*"));
    }

    #[test]
    fn test_event_type_matches_filter_prefix() {
        assert!(EventType::AppLaunched.matches_filter("app.*"));
        assert!(EventType::AppFocused.matches_filter("app.*"));
        assert!(!EventType::WindowResized.matches_filter("app.*"));

        assert!(EventType::WindowResized.matches_filter("window.*"));
        assert!(EventType::WindowMoved.matches_filter("window.*"));
        assert!(!EventType::AppLaunched.matches_filter("window.*"));

        assert!(EventType::DisplayConnected.matches_filter("display.*"));
        assert!(EventType::DisplayDisconnected.matches_filter("display.*"));
        assert!(!EventType::AppLaunched.matches_filter("display.*"));
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::app_launched("Safari".to_string(), 1234);
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"type\":\"app.launched\""));
        assert!(json.contains("\"app\":\"Safari\""));
        assert!(json.contains("\"pid\":1234"));
        assert!(json.contains("\"ts\":"));
    }

    #[test]
    fn test_event_data_app_name() {
        let app_data = EventData::app("Safari".to_string(), 123);
        assert_eq!(app_data.app_name(), Some("Safari"));

        let window_data = EventData::window_maximized("Chrome".to_string(), 456, None);
        assert_eq!(window_data.app_name(), Some("Chrome"));

        let display_data = EventData::display(
            0,
            "Test Display".to_string(),
            "1E6D_5B11_12345".to_string(),
            1920,
            1080,
            0,
            0,
            true,
            false,
            vec!["main".to_string()],
        );
        assert!(display_data.app_name().is_none());
    }

    #[test]
    fn test_matches_app_filter_exact() {
        assert!(matches_app_filter("Safari", &[], "Safari"));
        assert!(matches_app_filter("Safari", &[], "safari")); // case insensitive
        assert!(!matches_app_filter("Safari", &[], "Chrome"));
    }

    #[test]
    fn test_matches_app_filter_prefix() {
        assert!(matches_app_filter("Safari", &[], "Saf"));
        assert!(matches_app_filter("Google Chrome", &[], "Google"));
        assert!(!matches_app_filter("Safari", &[], "Chrome"));
    }

    #[test]
    fn test_matches_app_filter_title() {
        let titles = vec!["GitHub - Pull Request".to_string()];
        assert!(matches_app_filter("Safari", &titles, "GitHub"));
        assert!(matches_app_filter("Safari", &titles, "github")); // case insensitive
    }

    #[test]
    fn test_matches_app_filter_regex() {
        assert!(matches_app_filter("Safari", &[], "/safari/i"));
        assert!(matches_app_filter("Safari", &[], "/^Saf/"));
        assert!(!matches_app_filter("Safari", &[], "/chrome/i"));

        let titles = vec!["GitHub - PR #123".to_string()];
        assert!(matches_app_filter("Safari", &titles, "/PR #\\d+/"));
    }

    #[test]
    fn test_event_matches_filters() {
        let event = Event::app_launched("Safari".to_string(), 1234);

        // no filters = matches all
        assert!(event.matches_filters(&[], &[]));

        // event type filter
        assert!(event.matches_filters(&["app.launched".to_string()], &[]));
        assert!(event.matches_filters(&["app.*".to_string()], &[]));
        assert!(!event.matches_filters(&["window.*".to_string()], &[]));

        // app filter
        assert!(event.matches_filters(&[], &["Safari".to_string()]));
        assert!(event.matches_filters(&[], &["safari".to_string()]));
        assert!(!event.matches_filters(&[], &["Chrome".to_string()]));

        // both filters
        assert!(event.matches_filters(&["app.launched".to_string()], &["Safari".to_string()]));
        assert!(!event.matches_filters(&["app.launched".to_string()], &["Chrome".to_string()]));
    }

    #[test]
    fn test_event_bus_subscribe_unsubscribe() {
        let bus = EventBus::new();

        assert_eq!(bus.subscriber_count(), 0);

        let (id1, _rx1) = bus.subscribe(vec![], vec![]);
        assert_eq!(bus.subscriber_count(), 1);

        let (id2, _rx2) = bus.subscribe(vec!["app.*".to_string()], vec![]);
        assert_eq!(bus.subscriber_count(), 2);

        bus.unsubscribe(id1);
        assert_eq!(bus.subscriber_count(), 1);

        bus.unsubscribe(id2);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_event_bus_emit() {
        let bus = EventBus::new();

        let (_id, mut rx) = bus.subscribe(vec!["app.*".to_string()], vec![]);

        bus.emit(Event::app_launched("Safari".to_string(), 1234));
        // display event should not be received by app.* subscriber
        bus.emit(Event::display_connected(
            0,
            "Test".to_string(),
            "1234".to_string(),
            1920,
            1080,
            0,
            0,
            true,
            false,
            vec![],
        ));

        // check that we received the app event
        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type, EventType::AppLaunched);

        // display event should not be received
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_event_bus_expand_filters() {
        // empty = all events
        let expanded = EventBus::expand_filters(&[]);
        assert_eq!(expanded.len(), EventType::all().len());

        // wildcard = all events
        let expanded = EventBus::expand_filters(&["*".to_string()]);
        assert_eq!(expanded.len(), EventType::all().len());

        // prefix pattern
        let expanded = EventBus::expand_filters(&["app.*".to_string()]);
        assert_eq!(
            expanded,
            vec!["app.focused", "app.launched", "app.terminated"]
        );

        // exact match
        let expanded = EventBus::expand_filters(&["app.launched".to_string()]);
        assert_eq!(expanded, vec!["app.launched"]);

        // multiple patterns
        let expanded =
            EventBus::expand_filters(&["app.launched".to_string(), "window.*".to_string()]);
        assert!(expanded.contains(&"app.launched".to_string()));
        assert!(expanded.contains(&"window.maximized".to_string()));
        assert!(expanded.contains(&"window.resized".to_string()));
        assert!(expanded.contains(&"window.moved".to_string()));
        assert!(expanded.contains(&"window.closed".to_string()));
    }

    #[test]
    fn test_event_to_jsonrpc_notification() {
        let event = Event::app_launched("Safari".to_string(), 1234);
        let json = event.to_jsonrpc_notification();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"event\""));
        assert!(json.contains("\"params\":"));
    }

    #[test]
    fn test_display_event_serialization() {
        let event = Event::display_connected(
            1,
            "LG UltraFine".to_string(),
            "1E6D_5B11_12345".to_string(),
            3840,
            2160,
            2560,
            0,
            false,
            false,
            vec!["external".to_string(), "office".to_string()],
        );
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"type\":\"display.connected\""));
        assert!(json.contains("\"name\":\"LG UltraFine\""));
        assert!(json.contains("\"unique_id\":\"1E6D_5B11_12345\""));
        assert!(json.contains("\"width\":3840"));
        assert!(json.contains("\"height\":2160"));
        assert!(json.contains("\"is_main\":false"));
        assert!(json.contains("\"is_builtin\":false"));
        assert!(json.contains("\"aliases\":[\"external\",\"office\"]"));
    }

    #[test]
    fn test_display_event_matches_filters() {
        let event = Event::display_connected(
            0,
            "Test".to_string(),
            "1234".to_string(),
            1920,
            1080,
            0,
            0,
            true,
            true,
            vec![],
        );

        // no filters = matches all
        assert!(event.matches_filters(&[], &[]));

        // event type filter
        assert!(event.matches_filters(&["display.connected".to_string()], &[]));
        assert!(event.matches_filters(&["display.*".to_string()], &[]));
        assert!(!event.matches_filters(&["app.*".to_string()], &[]));

        // app filter should not match display events (no app name)
        assert!(!event.matches_filters(&[], &["Safari".to_string()]));
    }

    #[test]
    fn test_event_bus_expand_filters_display() {
        let expanded = EventBus::expand_filters(&["display.*".to_string()]);
        assert_eq!(expanded, vec!["display.connected", "display.disconnected"]);
    }
}
