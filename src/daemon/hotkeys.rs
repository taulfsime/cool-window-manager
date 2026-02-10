use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
    pub shift: bool,
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            ctrl: false,
            alt: false,
            cmd: false,
            shift: false,
        }
    }
}



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub keys: Vec<String>,
}

impl Hotkey {
    /// Parse a hotkey string like "ctrl+alt+s" or "cmd+shift+s+f"
    pub fn parse(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Empty hotkey string"));
        }

        let parts: Vec<String> = trimmed.split('+').map(|p| p.trim().to_lowercase()).collect();

        let mut modifiers = Modifiers::default();
        let mut keys: Vec<String> = Vec::new();

        for part in &parts {
            match part.as_str() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" | "option" | "opt" => modifiers.alt = true,
                "cmd" | "command" | "meta" | "super" => modifiers.cmd = true,
                "shift" => modifiers.shift = true,
                "" => {} // skip empty parts
                _ => {
                    keys.push(part.clone());
                }
            }
        }

        if keys.is_empty() {
            return Err(anyhow!("No key specified in hotkey: '{}'", s));
        }

        Ok(Hotkey { modifiers, keys })
    }

    /// Convert hotkey back to string representation
    pub fn to_string(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();

        if self.modifiers.ctrl {
            parts.push("ctrl");
        }
        if self.modifiers.alt {
            parts.push("alt");
        }
        if self.modifiers.cmd {
            parts.push("cmd");
        }
        if self.modifiers.shift {
            parts.push("shift");
        }

        for key in &self.keys {
            parts.push(key);
        }

        parts.join("+")
    }
}

/// Convert a macOS virtual keycode to a key name
fn keycode_to_string(keycode: i64) -> Option<String> {
    let key = match keycode {
        0 => "a",
        1 => "s",
        2 => "d",
        3 => "f",
        4 => "h",
        5 => "g",
        6 => "z",
        7 => "x",
        8 => "c",
        9 => "v",
        11 => "b",
        12 => "q",
        13 => "w",
        14 => "e",
        15 => "r",
        16 => "y",
        17 => "t",
        18 => "1",
        19 => "2",
        20 => "3",
        21 => "4",
        22 => "6",
        23 => "5",
        24 => "=",
        25 => "9",
        26 => "7",
        27 => "-",
        28 => "8",
        29 => "0",
        30 => "]",
        31 => "o",
        32 => "u",
        33 => "[",
        34 => "i",
        35 => "p",
        36 => "return",
        37 => "l",
        38 => "j",
        39 => "'",
        40 => "k",
        41 => ";",
        42 => "\\",
        43 => ",",
        44 => "/",
        45 => "n",
        46 => "m",
        47 => ".",
        48 => "tab",
        49 => "space",
        50 => "`",
        51 => "backspace",
        53 => "escape",
        55 => "cmd",
        56 => "shift",
        57 => "capslock",
        58 => "alt",
        59 => "ctrl",
        60 => "shift",
        61 => "alt",
        62 => "ctrl",
        63 => "fn",
        96 => "f5",
        97 => "f6",
        98 => "f7",
        99 => "f3",
        100 => "f8",
        101 => "f9",
        103 => "f11",
        105 => "f13",
        107 => "f14",
        109 => "f10",
        111 => "f12",
        113 => "f15",
        114 => "help",
        115 => "home",
        116 => "pageup",
        117 => "delete",
        118 => "f4",
        119 => "end",
        120 => "f2",
        121 => "pagedown",
        122 => "f1",
        123 => "left",
        124 => "right",
        125 => "down",
        126 => "up",
        _ => return None,
    };

    Some(key.to_string())
}

/// Check if keycode is a modifier key
fn is_modifier_key(keycode: i64) -> bool {
    matches!(keycode, 55 | 56 | 57 | 58 | 59 | 60 | 61 | 62 | 63)
}

#[cfg(target_os = "macos")]
#[allow(static_mut_refs)]
mod macos {
    use super::*;
    use std::collections::BTreeSet;
    use std::io::{self, Write};

    // modifier flags
    const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;
    const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000;
    const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
    const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;

    /// Extract modifiers from flags
    fn extract_modifiers(flags: u64) -> Modifiers {
        Modifiers {
            ctrl: (flags & K_CG_EVENT_FLAG_MASK_CONTROL) != 0,
            alt: (flags & K_CG_EVENT_FLAG_MASK_ALTERNATE) != 0,
            cmd: (flags & K_CG_EVENT_FLAG_MASK_COMMAND) != 0,
            shift: (flags & K_CG_EVENT_FLAG_MASK_SHIFT) != 0,
        }
    }

    /// Build display string for current state
    fn build_display_string(modifiers: &Modifiers, keys: &BTreeSet<String>) -> String {
        let mut parts: Vec<&str> = Vec::new();

        if modifiers.ctrl {
            parts.push("ctrl");
        }
        if modifiers.alt {
            parts.push("alt");
        }
        if modifiers.cmd {
            parts.push("cmd");
        }
        if modifiers.shift {
            parts.push("shift");
        }

        // sort keys for consistent display
        let mut key_vec: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        key_vec.sort();
        parts.extend(key_vec);

        if parts.is_empty() {
            "(press keys...)".to_string()
        } else {
            parts.join("+")
        }
    }

    type CGEventRef = *mut std::ffi::c_void;
    type CGEventTapProxy = *mut std::ffi::c_void;
    type CFMachPortRef = *mut std::ffi::c_void;
    type CFRunLoopSourceRef = *mut std::ffi::c_void;
    type CFRunLoopRef = *mut std::ffi::c_void;

    type CGEventTapCallBack = extern "C" fn(
        proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        user_info: *mut std::ffi::c_void,
    ) -> CGEventRef;

    const K_CG_EVENT_KEY_DOWN: u32 = 10;
    const K_CG_EVENT_KEY_UP: u32 = 11;
    const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
    const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;
    const K_CG_HID_EVENT_TAP: u32 = 0;
    const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: CGEventTapCallBack,
            user_info: *mut std::ffi::c_void,
        ) -> CFMachPortRef;

        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
        fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
        fn CGEventGetFlags(event: CGEventRef) -> u64;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFMachPortCreateRunLoopSource(
            allocator: *const std::ffi::c_void,
            port: CFMachPortRef,
            order: i64,
        ) -> CFRunLoopSourceRef;

        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const std::ffi::c_void);
        fn CFRunLoopRun();
        fn CFRunLoopStop(rl: CFRunLoopRef);
        fn CFRelease(cf: *const std::ffi::c_void);

        static kCFRunLoopCommonModes: *const std::ffi::c_void;
    }

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

    // global state for callback
    static mut CURRENT_MODIFIERS: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        cmd: false,
        shift: false,
    };
    static mut CURRENT_KEYS: Option<BTreeSet<String>> = None;
    static mut LAST_HOTKEY: Option<Hotkey> = None;
    static mut SHOULD_STOP: bool = false;
    static mut CANCELLED: bool = false;
    static mut CURRENT_RUN_LOOP: CFRunLoopRef = std::ptr::null_mut();
    static mut LAST_DISPLAY: String = String::new();
    static mut EVENT_TAP: CFMachPortRef = std::ptr::null_mut();

    fn clear_line_and_print(s: &str) {
        print!("\r\x1b[K{}", s);
        let _ = io::stdout().flush();
    }

    fn stop_recording(cancelled: bool) {
        unsafe {
            SHOULD_STOP = true;
            CANCELLED = cancelled;
            if !CURRENT_RUN_LOOP.is_null() {
                CFRunLoopStop(CURRENT_RUN_LOOP);
            }
        }
    }

    extern "C" fn event_callback(
        _proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        _user_info: *mut std::ffi::c_void,
    ) -> CGEventRef {
        unsafe {
            // handle tap disabled by timeout (re-enable it)
            if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT {
                if !EVENT_TAP.is_null() {
                    CGEventTapEnable(EVENT_TAP, true);
                }
                return event;
            }

            let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
            let flags = CGEventGetFlags(event);

            // update modifiers from flags
            CURRENT_MODIFIERS = extract_modifiers(flags);

            // initialize keys set if needed
            if CURRENT_KEYS.is_none() {
                CURRENT_KEYS = Some(BTreeSet::new());
            }

            match event_type {
                K_CG_EVENT_KEY_DOWN => {
                    // ESC to confirm and exit
                    if keycode == 53 {
                        // save current state as final hotkey if we have keys
                        if let Some(ref keys) = CURRENT_KEYS {
                            if !keys.is_empty() {
                                LAST_HOTKEY = Some(Hotkey {
                                    modifiers: CURRENT_MODIFIERS,
                                    keys: keys.iter().cloned().collect(),
                                });
                            }
                        }
                        stop_recording(LAST_HOTKEY.is_none());
                        return std::ptr::null_mut();
                    }

                    // add key if it's not a modifier
                    if !is_modifier_key(keycode) {
                        if let Some(key) = keycode_to_string(keycode) {
                            if let Some(ref mut keys) = CURRENT_KEYS {
                                keys.insert(key);

                                // save as potential hotkey
                                LAST_HOTKEY = Some(Hotkey {
                                    modifiers: CURRENT_MODIFIERS,
                                    keys: keys.iter().cloned().collect(),
                                });
                            }
                        }
                    }
                }
                K_CG_EVENT_KEY_UP => {
                    // remove key on key up
                    if !is_modifier_key(keycode) {
                        if let Some(key) = keycode_to_string(keycode) {
                            if let Some(ref mut keys) = CURRENT_KEYS {
                                keys.remove(&key);
                            }
                        }
                    }
                }
                K_CG_EVENT_FLAGS_CHANGED => {
                    // modifiers already updated above
                }
                _ => {}
            }

            // update display
            let keys = CURRENT_KEYS.as_ref().map(|k| k.clone()).unwrap_or_default();
            let display = build_display_string(&CURRENT_MODIFIERS, &keys);

            // only update if changed
            if display != LAST_DISPLAY {
                LAST_DISPLAY = display.clone();

                let status = if LAST_HOTKEY.is_some() {
                    format!("Current: {}  [ESC to confirm]", display)
                } else {
                    format!("Current: {}  [ESC to cancel]", display)
                };
                clear_line_and_print(&status);
            }

            // consume the event
            std::ptr::null_mut()
        }
    }

    // store the initial frontmost app PID when recording starts
    static mut INITIAL_FRONTMOST_PID: i32 = 0;

    /// Get the current frontmost application PID
    fn get_frontmost_pid() -> i32 {
        use objc2_app_kit::NSWorkspace;
        
        let workspace = NSWorkspace::sharedWorkspace();
        let frontmost = workspace.frontmostApplication();
        
        match frontmost {
            Some(app) => app.processIdentifier(),
            None => -1,
        }
    }

    /// Check if the same app that was focused when we started is still focused
    fn is_same_app_focused() -> bool {
        unsafe {
            let current_frontmost = get_frontmost_pid();
            if current_frontmost <= 0 || INITIAL_FRONTMOST_PID <= 0 {
                return true; // assume focused if we can't tell
            }
            current_frontmost == INITIAL_FRONTMOST_PID
        }
    }

    pub fn record_hotkey_impl() -> Result<Hotkey> {
        unsafe {
            // reset global state
            CURRENT_MODIFIERS = Modifiers::default();
            CURRENT_KEYS = Some(BTreeSet::new());
            LAST_HOTKEY = None;
            SHOULD_STOP = false;
            CANCELLED = false;
            LAST_DISPLAY = String::new();
            
            // capture the frontmost app PID at start (this is the terminal running us)
            INITIAL_FRONTMOST_PID = get_frontmost_pid();

            println!("Recording... Press your key combination, then ESC to confirm.");
            println!("(ESC without keys will cancel. Losing focus will cancel.)\n");
            print!("Current: (press keys...)");
            let _ = io::stdout().flush();

            // event mask for key down, key up, and flags changed
            let event_mask: u64 = (1 << K_CG_EVENT_KEY_DOWN)
                | (1 << K_CG_EVENT_KEY_UP)
                | (1 << K_CG_EVENT_FLAGS_CHANGED);

            // create event tap
            let tap = CGEventTapCreate(
                K_CG_HID_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                event_mask,
                event_callback,
                std::ptr::null_mut(),
            );

            if tap.is_null() {
                println!();
                return Err(anyhow!(
                    "Failed to create event tap. Make sure accessibility permissions are granted."
                ));
            }

            EVENT_TAP = tap;

            // create run loop source
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                CFRelease(tap);
                EVENT_TAP = std::ptr::null_mut();
                println!();
                return Err(anyhow!("Failed to create run loop source"));
            }

            // add to run loop
            let run_loop = CFRunLoopGetCurrent();
            CURRENT_RUN_LOOP = run_loop;
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);

            // enable tap
            CGEventTapEnable(tap, true);

            // spawn a thread to check for focus loss
            let check_focus = std::thread::spawn(|| {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    
                    if SHOULD_STOP {
                        break;
                    }
                    
                    if !is_same_app_focused() {
                        clear_line_and_print("Focus lost - cancelled.\n");
                        stop_recording(true);
                        break;
                    }
                }
            });

            // run loop
            CFRunLoopRun();

            // cleanup
            SHOULD_STOP = true; // signal focus thread to stop
            let _ = check_focus.join();
            
            CURRENT_RUN_LOOP = std::ptr::null_mut();
            EVENT_TAP = std::ptr::null_mut();
            CFRelease(source);
            CFRelease(tap);

            println!(); // newline after the status line

            if CANCELLED {
                return Err(anyhow!("Recording cancelled"));
            }

            // get result
            LAST_HOTKEY
                .take()
                .ok_or_else(|| anyhow!("Recording cancelled - no key combination captured"))
        }
    }

    // listener state
    static mut LISTENER_SHORTCUTS: Option<Vec<(Hotkey, String)>> = None;
    static mut LISTENER_CALLBACK: Option<Box<dyn Fn(&str, &Hotkey) + Send>> = None;
    static mut LISTENER_RUNNING: bool = false;
    static mut LISTENER_RUN_LOOP: CFRunLoopRef = std::ptr::null_mut();
    static mut LISTENER_EVENT_TAP: CFMachPortRef = std::ptr::null_mut();
    static mut LISTENER_PRESSED_KEYS: Option<BTreeSet<String>> = None;
    static mut LISTENER_MODIFIERS: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        cmd: false,
        shift: false,
    };

    /// Check if the current key combination matches a registered hotkey
    fn check_hotkey_match(modifiers: &Modifiers, keys: &BTreeSet<String>) -> Option<(String, Hotkey)> {
        unsafe {
            if let Some(ref shortcuts) = LISTENER_SHORTCUTS {
                for (hotkey, action) in shortcuts {
                    // check modifiers match
                    if hotkey.modifiers.ctrl != modifiers.ctrl
                        || hotkey.modifiers.alt != modifiers.alt
                        || hotkey.modifiers.cmd != modifiers.cmd
                        || hotkey.modifiers.shift != modifiers.shift
                    {
                        continue;
                    }

                    // check all hotkey keys are pressed
                    let hotkey_keys: BTreeSet<String> = hotkey.keys.iter().cloned().collect();
                    if hotkey_keys == *keys {
                        return Some((action.clone(), hotkey.clone()));
                    }
                }
            }
        }
        None
    }

    extern "C" fn listener_callback(
        _proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        _user_info: *mut std::ffi::c_void,
    ) -> CGEventRef {
        unsafe {
            // handle tap disabled by timeout
            if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT {
                if !LISTENER_EVENT_TAP.is_null() {
                    CGEventTapEnable(LISTENER_EVENT_TAP, true);
                }
                return event;
            }

            let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
            let flags = CGEventGetFlags(event);

            // update modifiers
            LISTENER_MODIFIERS = extract_modifiers(flags);

            // initialize keys set if needed
            if LISTENER_PRESSED_KEYS.is_none() {
                LISTENER_PRESSED_KEYS = Some(BTreeSet::new());
            }

            match event_type {
                K_CG_EVENT_KEY_DOWN => {
                    if !is_modifier_key(keycode) {
                        if let Some(key) = keycode_to_string(keycode) {
                            if let Some(ref mut keys) = LISTENER_PRESSED_KEYS {
                                keys.insert(key);

                                // check for hotkey match
                                if let Some((action, hotkey)) = check_hotkey_match(&LISTENER_MODIFIERS, keys) {
                                    if let Some(ref callback) = LISTENER_CALLBACK {
                                        callback(&action, &hotkey);
                                    }
                                    // consume the event to prevent it from propagating
                                    return std::ptr::null_mut();
                                }
                            }
                        }
                    }
                }
                K_CG_EVENT_KEY_UP => {
                    if !is_modifier_key(keycode) {
                        if let Some(key) = keycode_to_string(keycode) {
                            if let Some(ref mut keys) = LISTENER_PRESSED_KEYS {
                                keys.remove(&key);
                            }
                        }
                    }
                }
                K_CG_EVENT_FLAGS_CHANGED => {
                    // modifiers already updated above
                }
                _ => {}
            }

            // pass the event through
            event
        }
    }

    pub fn start_listener_impl<F>(shortcuts: Vec<(Hotkey, String)>, callback: F) -> Result<()>
    where
        F: Fn(&str, &Hotkey) + Send + 'static,
    {
        unsafe {
            if LISTENER_RUNNING {
                return Err(anyhow!("Listener already running"));
            }

            LISTENER_SHORTCUTS = Some(shortcuts);
            LISTENER_CALLBACK = Some(Box::new(callback));
            LISTENER_PRESSED_KEYS = Some(BTreeSet::new());
            LISTENER_MODIFIERS = Modifiers::default();
            LISTENER_RUNNING = true;

            // event mask for key events
            let event_mask: u64 = (1 << K_CG_EVENT_KEY_DOWN)
                | (1 << K_CG_EVENT_KEY_UP)
                | (1 << K_CG_EVENT_FLAGS_CHANGED);

            // create event tap
            let tap = CGEventTapCreate(
                K_CG_HID_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                event_mask,
                listener_callback,
                std::ptr::null_mut(),
            );

            if tap.is_null() {
                LISTENER_RUNNING = false;
                LISTENER_SHORTCUTS = None;
                LISTENER_CALLBACK = None;
                return Err(anyhow!(
                    "Failed to create event tap. Make sure accessibility permissions are granted."
                ));
            }

            LISTENER_EVENT_TAP = tap;

            // create run loop source
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                CFRelease(tap);
                LISTENER_EVENT_TAP = std::ptr::null_mut();
                LISTENER_RUNNING = false;
                LISTENER_SHORTCUTS = None;
                LISTENER_CALLBACK = None;
                return Err(anyhow!("Failed to create run loop source"));
            }

            // add to run loop
            let run_loop = CFRunLoopGetCurrent();
            LISTENER_RUN_LOOP = run_loop;
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);

            // enable tap
            CGEventTapEnable(tap, true);

            // run the loop - this blocks until stopped
            CFRunLoopRun();

            // cleanup
            LISTENER_RUNNING = false;
            LISTENER_RUN_LOOP = std::ptr::null_mut();
            LISTENER_EVENT_TAP = std::ptr::null_mut();
            CFRelease(source);
            CFRelease(tap);
            LISTENER_SHORTCUTS = None;
            LISTENER_CALLBACK = None;
            LISTENER_PRESSED_KEYS = None;
        }

        Ok(())
    }

    pub fn stop_listener_impl() {
        unsafe {
            if LISTENER_RUNNING && !LISTENER_RUN_LOOP.is_null() {
                CFRunLoopStop(LISTENER_RUN_LOOP);
            }
        }
    }
}

/// Record a single keypress and return the hotkey string
#[cfg(target_os = "macos")]
pub fn record_hotkey() -> Result<String> {
    let hotkey = macos::record_hotkey_impl()?;
    Ok(hotkey.to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn record_hotkey() -> Result<String> {
    Err(anyhow!("Hotkey recording is only supported on macOS"))
}



/// Start listening for global hotkeys and call the callback when one is pressed
#[cfg(target_os = "macos")]
pub fn start_hotkey_listener<F>(shortcuts: Vec<(Hotkey, String)>, callback: F) -> Result<()>
where
    F: Fn(&str, &Hotkey) + Send + 'static,
{
    macos::start_listener_impl(shortcuts, callback)
}

#[cfg(not(target_os = "macos"))]
pub fn start_hotkey_listener<F>(_shortcuts: Vec<(Hotkey, String)>, _callback: F) -> Result<()>
where
    F: Fn(&str, &Hotkey) + Send + 'static,
{
    Err(anyhow!("Hotkey listening is only supported on macOS"))
}

/// Stop the hotkey listener
#[cfg(target_os = "macos")]
pub fn stop_hotkey_listener() {
    macos::stop_listener_impl();
}

#[cfg(not(target_os = "macos"))]
pub fn stop_hotkey_listener() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_hotkey() {
        let hk = Hotkey::parse("ctrl+alt+s").unwrap();
        assert!(hk.modifiers.ctrl);
        assert!(hk.modifiers.alt);
        assert!(!hk.modifiers.cmd);
        assert!(!hk.modifiers.shift);
        assert_eq!(hk.keys, vec!["s"]);
    }

    #[test]
    fn test_parse_hotkey_with_cmd() {
        let hk = Hotkey::parse("cmd+shift+return").unwrap();
        assert!(!hk.modifiers.ctrl);
        assert!(!hk.modifiers.alt);
        assert!(hk.modifiers.cmd);
        assert!(hk.modifiers.shift);
        assert_eq!(hk.keys, vec!["return"]);
    }

    #[test]
    fn test_parse_hotkey_multiple_keys() {
        let hk = Hotkey::parse("ctrl+s+f").unwrap();
        assert!(hk.modifiers.ctrl);
        assert_eq!(hk.keys, vec!["s", "f"]);
    }

    #[test]
    fn test_parse_hotkey_aliases() {
        let hk = Hotkey::parse("control+option+command+a").unwrap();
        assert!(hk.modifiers.ctrl);
        assert!(hk.modifiers.alt);
        assert!(hk.modifiers.cmd);
        assert_eq!(hk.keys, vec!["a"]);
    }

    #[test]
    fn test_hotkey_to_string() {
        let hk = Hotkey {
            modifiers: Modifiers {
                ctrl: true,
                alt: true,
                cmd: false,
                shift: false,
            },
            keys: vec!["s".to_string()],
        };
        assert_eq!(hk.to_string(), "ctrl+alt+s");
    }

    #[test]
    fn test_hotkey_to_string_multiple_keys() {
        let hk = Hotkey {
            modifiers: Modifiers {
                ctrl: true,
                alt: false,
                cmd: false,
                shift: false,
            },
            keys: vec!["s".to_string(), "f".to_string()],
        };
        assert_eq!(hk.to_string(), "ctrl+s+f");
    }

    #[test]
    fn test_parse_invalid_hotkey() {
        assert!(Hotkey::parse("").is_err());
        assert!(Hotkey::parse("ctrl+alt").is_err()); // no keys
    }
}
