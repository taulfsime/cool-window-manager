//! display configuration change watcher
//!
//! monitors for display connect/disconnect using CoreGraphics callbacks

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;

use crate::config::DisplayAliases;
use crate::daemon::events::{self, Event};
use crate::display::{get_displays, DisplayInfo};

// cached display state
struct DisplayState {
    displays: HashMap<String, DisplayInfo>, // keyed by unique_id
    aliases: DisplayAliases,
}

static DISPLAY_STATE: Mutex<Option<DisplayState>> = Mutex::new(None);

/// compute which aliases match a display
fn compute_aliases_for_display(
    display: &DisplayInfo,
    user_aliases: &DisplayAliases,
) -> Vec<String> {
    let mut aliases = Vec::new();

    // system aliases
    if display.is_builtin {
        aliases.push("builtin".to_string());
    } else {
        aliases.push("external".to_string());
    }
    if display.is_main {
        aliases.push("main".to_string());
    } else {
        aliases.push("secondary".to_string());
    }

    // user-defined aliases
    let display_id = display.unique_id();
    for (alias_name, id_list) in user_aliases {
        if id_list
            .iter()
            .any(|id| id.eq_ignore_ascii_case(&display_id))
        {
            aliases.push(alias_name.clone());
        }
    }

    aliases.sort();
    aliases
}

/// check for display changes and emit events
fn check_display_changes() {
    let Ok(new_displays) = get_displays() else {
        return;
    };

    let mut guard = match DISPLAY_STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };

    let Some(ref mut state) = *guard else {
        return;
    };

    let new_map: HashMap<String, DisplayInfo> = new_displays
        .iter()
        .map(|d| (d.unique_id(), d.clone()))
        .collect();

    // find disconnected displays (in old, not in new)
    for (id, display) in &state.displays {
        if !new_map.contains_key(id) {
            let aliases = compute_aliases_for_display(display, &state.aliases);
            events::emit(Event::display_disconnected(
                display.index,
                display.name.clone(),
                display.unique_id(),
                display.width,
                display.height,
                display.x,
                display.y,
                display.is_main,
                display.is_builtin,
                aliases,
            ));
        }
    }

    // find connected displays (in new, not in old)
    for (id, display) in &new_map {
        if !state.displays.contains_key(id) {
            let aliases = compute_aliases_for_display(display, &state.aliases);
            events::emit(Event::display_connected(
                display.index,
                display.name.clone(),
                display.unique_id(),
                display.width,
                display.height,
                display.x,
                display.y,
                display.is_main,
                display.is_builtin,
                aliases,
            ));
        }
    }

    // update cached state
    state.displays = new_map;
}

/// CoreGraphics reconfiguration callback
extern "C" fn display_callback(_display: u32, flags: u32, _user_info: *mut c_void) {
    // CGDisplayBeginConfigurationFlag = 1 << 0
    // skip the "begin configuration" callback - wait for changes to complete
    const BEGIN_CONFIG: u32 = 1 << 0;
    if flags & BEGIN_CONFIG != 0 {
        return;
    }

    check_display_changes();
}

/// start watching for display changes
pub fn start_watching(aliases: DisplayAliases) -> anyhow::Result<()> {
    let displays = get_displays()?;
    let display_map: HashMap<String, DisplayInfo> = displays
        .iter()
        .map(|d| (d.unique_id(), d.clone()))
        .collect();

    {
        let mut guard = DISPLAY_STATE
            .lock()
            .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        *guard = Some(DisplayState {
            displays: display_map,
            aliases,
        });
    }

    unsafe {
        CGDisplayRegisterReconfigurationCallback(display_callback, std::ptr::null_mut());
    }

    Ok(())
}

/// stop watching for display changes
pub fn stop_watching() {
    unsafe {
        CGDisplayRemoveReconfigurationCallback(display_callback, std::ptr::null_mut());
    }

    if let Ok(mut guard) = DISPLAY_STATE.lock() {
        *guard = None;
    }
}

/// update the cached aliases (call when config changes)
#[allow(dead_code)]
pub fn update_aliases(aliases: DisplayAliases) {
    if let Ok(mut guard) = DISPLAY_STATE.lock() {
        if let Some(ref mut state) = *guard {
            state.aliases = aliases;
        }
    }
}

// FFI declarations for CGDisplay reconfiguration
extern "C" {
    fn CGDisplayRegisterReconfigurationCallback(
        callback: extern "C" fn(u32, u32, *mut c_void),
        user_info: *mut c_void,
    ) -> i32;

    fn CGDisplayRemoveReconfigurationCallback(
        callback: extern "C" fn(u32, u32, *mut c_void),
        user_info: *mut c_void,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_display(index: usize, is_main: bool, is_builtin: bool) -> DisplayInfo {
        DisplayInfo {
            index,
            name: format!("Display {}", index),
            width: 1920,
            height: 1080,
            x: (index as i32) * 1920,
            y: 0,
            is_main,
            display_id: index as u32,
            vendor_id: Some(0x1E6D),
            model_id: Some(0x5B11),
            serial_number: Some(index as u32),
            unit_number: 0,
            is_builtin,
        }
    }

    #[test]
    fn test_compute_aliases_builtin_main() {
        let display = test_display(0, true, true);
        let user_aliases = HashMap::new();

        let aliases = compute_aliases_for_display(&display, &user_aliases);

        assert!(aliases.contains(&"builtin".to_string()));
        assert!(aliases.contains(&"main".to_string()));
        assert!(!aliases.contains(&"external".to_string()));
        assert!(!aliases.contains(&"secondary".to_string()));
    }

    #[test]
    fn test_compute_aliases_external_secondary() {
        let display = test_display(1, false, false);
        let user_aliases = HashMap::new();

        let aliases = compute_aliases_for_display(&display, &user_aliases);

        assert!(aliases.contains(&"external".to_string()));
        assert!(aliases.contains(&"secondary".to_string()));
        assert!(!aliases.contains(&"builtin".to_string()));
        assert!(!aliases.contains(&"main".to_string()));
    }

    #[test]
    fn test_compute_aliases_with_user_aliases() {
        let display = test_display(1, false, false);
        let mut user_aliases = HashMap::new();
        user_aliases.insert("office".to_string(), vec!["1E6D_5B11_1".to_string()]);
        user_aliases.insert("home".to_string(), vec!["OTHER_ID".to_string()]);

        let aliases = compute_aliases_for_display(&display, &user_aliases);

        assert!(aliases.contains(&"office".to_string()));
        assert!(!aliases.contains(&"home".to_string()));
    }

    #[test]
    fn test_compute_aliases_case_insensitive_id_match() {
        let display = test_display(1, false, false);
        let mut user_aliases = HashMap::new();
        // user alias with lowercase id should still match
        user_aliases.insert("office".to_string(), vec!["1e6d_5b11_1".to_string()]);

        let aliases = compute_aliases_for_display(&display, &user_aliases);

        assert!(aliases.contains(&"office".to_string()));
    }

    #[test]
    fn test_compute_aliases_sorted() {
        let display = test_display(1, false, false);
        let mut user_aliases = HashMap::new();
        user_aliases.insert("zebra".to_string(), vec!["1E6D_5B11_1".to_string()]);
        user_aliases.insert("alpha".to_string(), vec!["1E6D_5B11_1".to_string()]);

        let aliases = compute_aliases_for_display(&display, &user_aliases);

        // should be sorted alphabetically
        let alpha_pos = aliases.iter().position(|a| a == "alpha").unwrap();
        let zebra_pos = aliases.iter().position(|a| a == "zebra").unwrap();
        assert!(alpha_pos < zebra_pos);
    }
}
