use anyhow::{anyhow, Result};

use super::accessibility;
use super::matching::AppInfo;

#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::string::CFString;
#[cfg(target_os = "macos")]
use core_graphics::display::CGDisplay;

#[cfg(target_os = "macos")]
type AXUIElementRef = *mut std::ffi::c_void;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: core_foundation::string::CFStringRef,
        value: *mut core_foundation::base::CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: core_foundation::string::CFStringRef,
        value: core_foundation::base::CFTypeRef,
    ) -> i32;
}

#[cfg(target_os = "macos")]
const K_AX_ERROR_SUCCESS: i32 = 0;

/// Focus an application by bringing it to the foreground
#[cfg(target_os = "macos")]
pub fn focus_app(app: &AppInfo, verbose: bool) -> Result<()> {
    use cocoa::appkit::NSApplicationActivateIgnoringOtherApps;
    use cocoa::base::nil;
    use objc::runtime::Object;

    if !accessibility::is_trusted() {
        return Err(anyhow!(
            "Accessibility permissions required. Run 'cwm check-permissions' for help."
        ));
    }

    if verbose {
        println!("Focusing: {} (PID: {})", app.name, app.pid);
    }

    unsafe {
        let running_app: *mut Object = msg_send![
            class!(NSRunningApplication),
            runningApplicationWithProcessIdentifier: app.pid
        ];

        if running_app == nil {
            return Err(anyhow!("Could not find running application with PID {}", app.pid));
        }

        let success: bool = msg_send![
            running_app,
            activateWithOptions: NSApplicationActivateIgnoringOtherApps
        ];

        if !success {
            return Err(anyhow!("Failed to activate application: {}", app.name));
        }
    }

    if verbose {
        println!("Done.");
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn focus_app(_app: &AppInfo, _verbose: bool) -> Result<()> {
    Err(anyhow!("Focus is only supported on macOS"))
}

/// Get the frontmost window of an application
#[cfg(target_os = "macos")]
unsafe fn get_frontmost_window(pid: i32) -> Result<AXUIElementRef> {
    use core_foundation::base::CFTypeRef;

    let app_element = AXUIElementCreateApplication(pid);
    if app_element.is_null() {
        return Err(anyhow!("Failed to create AXUIElement for PID {}", pid));
    }

    // get the windows attribute
    let windows_attr = CFString::new("AXWindows");
    let mut windows_value: CFTypeRef = std::ptr::null_mut();

    let result = AXUIElementCopyAttributeValue(
        app_element,
        windows_attr.as_concrete_TypeRef(),
        &mut windows_value,
    );

    if result != K_AX_ERROR_SUCCESS || windows_value.is_null() {
        core_foundation::base::CFRelease(app_element as CFTypeRef);
        return Err(anyhow!("Failed to get windows for application (error: {})", result));
    }

    // windows_value is a CFArray, get count and first element
    let count = CFArrayGetCount(windows_value as _);

    if count == 0 {
        core_foundation::base::CFRelease(windows_value);
        core_foundation::base::CFRelease(app_element as CFTypeRef);
        return Err(anyhow!("Application has no windows"));
    }

    // get the first (frontmost) window
    let window = CFArrayGetValueAtIndex(windows_value as _, 0) as AXUIElementRef;
    
    if window.is_null() {
        core_foundation::base::CFRelease(windows_value);
        core_foundation::base::CFRelease(app_element as CFTypeRef);
        return Err(anyhow!("Failed to get window at index 0"));
    }

    // retain the window since we're returning it
    core_foundation::base::CFRetain(window as CFTypeRef);
    
    core_foundation::base::CFRelease(windows_value);
    core_foundation::base::CFRelease(app_element as CFTypeRef);

    Ok(window)
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: core_foundation::array::CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(array: core_foundation::array::CFArrayRef, index: isize) -> *const std::ffi::c_void;
}

/// Get the focused window (from the frontmost application)
#[cfg(target_os = "macos")]
unsafe fn get_focused_window() -> Result<(AXUIElementRef, i32)> {
    use cocoa::base::nil;
    use objc::runtime::Object;

    // get the frontmost application
    let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
    let frontmost_app: *mut Object = msg_send![workspace, frontmostApplication];

    if frontmost_app == nil {
        return Err(anyhow!("No frontmost application"));
    }

    let pid: i32 = msg_send![frontmost_app, processIdentifier];

    if pid <= 0 {
        return Err(anyhow!("Invalid PID for frontmost application"));
    }

    let window = get_frontmost_window(pid)?;
    Ok((window, pid))
}

/// Set window position
#[cfg(target_os = "macos")]
unsafe fn set_window_position(window: AXUIElementRef, x: f64, y: f64) -> Result<()> {
    use core_foundation::base::CFTypeRef;

    let position_attr = CFString::new("AXPosition");

    // create an AXValue for the position (CGPoint)
    let point = core_graphics::geometry::CGPoint::new(x, y);
    let position_value = AXValueCreate(K_AX_VALUE_TYPE_CG_POINT, &point as *const _ as *const std::ffi::c_void);

    if position_value.is_null() {
        return Err(anyhow!("Failed to create AXValue for position"));
    }

    let result = AXUIElementSetAttributeValue(
        window,
        position_attr.as_concrete_TypeRef(),
        position_value as CFTypeRef,
    );

    core_foundation::base::CFRelease(position_value as CFTypeRef);

    if result != K_AX_ERROR_SUCCESS {
        let err_msg = match result {
            -25200 => "cannot complete (window may be fullscreen or app restricts access)",
            -25201 => "invalid element",
            -25202 => "invalid observer",
            -25203 => "failure",
            -25204 => "attribute unsupported",
            -25205 => "action unsupported",
            -25206 => "notification unsupported",
            -25207 => "not implemented",
            -25208 => "notification already registered",
            -25209 => "notification not registered",
            -25210 => "API disabled",
            -25211 => "no value",
            -25212 => "parameter error",
            _ => "unknown error",
        };
        return Err(anyhow!("Failed to set window position: {} ({})", err_msg, result));
    }

    Ok(())
}

/// Set window size
#[cfg(target_os = "macos")]
unsafe fn set_window_size(window: AXUIElementRef, width: f64, height: f64) -> Result<()> {
    use core_foundation::base::CFTypeRef;

    let size_attr = CFString::new("AXSize");

    // create an AXValue for the size (CGSize)
    let size = core_graphics::geometry::CGSize::new(width, height);
    let size_value = AXValueCreate(K_AX_VALUE_TYPE_CG_SIZE, &size as *const _ as *const std::ffi::c_void);

    if size_value.is_null() {
        return Err(anyhow!("Failed to create AXValue for size"));
    }

    let result = AXUIElementSetAttributeValue(
        window,
        size_attr.as_concrete_TypeRef(),
        size_value as CFTypeRef,
    );

    core_foundation::base::CFRelease(size_value as CFTypeRef);

    if result != K_AX_ERROR_SUCCESS {
        return Err(anyhow!("Failed to set window size (error: {})", result));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
type AXValueRef = *mut std::ffi::c_void;

#[cfg(target_os = "macos")]
const K_AX_VALUE_TYPE_CG_POINT: u32 = 1;
#[cfg(target_os = "macos")]
const K_AX_VALUE_TYPE_CG_SIZE: u32 = 2;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXValueCreate(value_type: u32, value: *const std::ffi::c_void) -> AXValueRef;
}

/// Get the display bounds, accounting for menu bar
#[cfg(target_os = "macos")]
fn get_usable_display_bounds() -> (f64, f64, f64, f64) {
    use cocoa::base::nil;
    use cocoa::foundation::NSRect;
    use objc::runtime::Object;

    unsafe {
        let screen: *mut Object = msg_send![class!(NSScreen), mainScreen];
        if screen == nil {
            // fallback to CGDisplay
            let display = CGDisplay::main();
            let bounds = display.bounds();
            return (
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                bounds.size.height,
            );
        }

        // visibleFrame excludes menu bar and dock
        let visible_frame: NSRect = msg_send![screen, visibleFrame];
        let frame: NSRect = msg_send![screen, frame];

        // NSScreen uses bottom-left origin, but AX uses top-left
        // convert y coordinate
        let y = frame.size.height - visible_frame.origin.y - visible_frame.size.height;

        (
            visible_frame.origin.x,
            y,
            visible_frame.size.width,
            visible_frame.size.height,
        )
    }
}

/// Maximize a window to fill the screen
#[cfg(target_os = "macos")]
pub fn maximize_window(app: Option<&AppInfo>, verbose: bool) -> Result<()> {
    use core_foundation::base::CFTypeRef;

    if !accessibility::is_trusted() {
        return Err(anyhow!(
            "Accessibility permissions required. Run 'cwm check-permissions' for help."
        ));
    }

    let (window, pid) = unsafe {
        if let Some(app_info) = app {
            let w = get_frontmost_window(app_info.pid)?;
            (w, app_info.pid)
        } else {
            get_focused_window()?
        }
    };

    if verbose {
        println!("Maximizing window for PID: {}", pid);
    }

    // get usable display bounds (excluding menu bar and dock)
    let (x, y, width, height) = get_usable_display_bounds();

    if verbose {
        println!("Usable display bounds: {}x{} at ({}, {})", width, height, x, y);
    }

    unsafe {
        // set position first, then size
        set_window_position(window, x, y)?;
        set_window_size(window, width, height)?;

        // release the window
        core_foundation::base::CFRelease(window as CFTypeRef);
    }

    if verbose {
        println!("Done.");
    } else {
        println!("Window maximized");
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn maximize_window(_app: Option<&AppInfo>, _verbose: bool) -> Result<()> {
    Err(anyhow!("Maximize is only supported on macOS"))
}

/// Launch an application by name
#[cfg(target_os = "macos")]
pub fn launch_app(app_name: &str, verbose: bool) -> Result<()> {
    use std::process::Command;

    if verbose {
        println!("Launching: {}", app_name);
    }

    let status = Command::new("open")
        .arg("-a")
        .arg(app_name)
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to launch application: {}", app_name));
    }

    if verbose {
        println!("Launched successfully.");
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn launch_app(_app_name: &str, _verbose: bool) -> Result<()> {
    Err(anyhow!("Launch is only supported on macOS"))
}

/// Get current window position
#[cfg(target_os = "macos")]
unsafe fn get_window_position(window: AXUIElementRef) -> Result<(f64, f64)> {
    use core_foundation::base::CFTypeRef;

    let position_attr = CFString::new("AXPosition");
    let mut position_value: CFTypeRef = std::ptr::null_mut();

    let result = AXUIElementCopyAttributeValue(
        window,
        position_attr.as_concrete_TypeRef(),
        &mut position_value,
    );

    if result != K_AX_ERROR_SUCCESS || position_value.is_null() {
        return Err(anyhow!("Failed to get window position (error: {})", result));
    }

    let mut point = core_graphics::geometry::CGPoint::new(0.0, 0.0);
    let success = AXValueGetValue(
        position_value as AXValueRef,
        K_AX_VALUE_TYPE_CG_POINT,
        &mut point as *mut _ as *mut std::ffi::c_void,
    );

    core_foundation::base::CFRelease(position_value);

    if !success {
        return Err(anyhow!("Failed to extract position value"));
    }

    Ok((point.x, point.y))
}

/// Get current window size
#[cfg(target_os = "macos")]
unsafe fn get_window_size(window: AXUIElementRef) -> Result<(f64, f64)> {
    use core_foundation::base::CFTypeRef;

    let size_attr = CFString::new("AXSize");
    let mut size_value: CFTypeRef = std::ptr::null_mut();

    let result = AXUIElementCopyAttributeValue(
        window,
        size_attr.as_concrete_TypeRef(),
        &mut size_value,
    );

    if result != K_AX_ERROR_SUCCESS || size_value.is_null() {
        return Err(anyhow!("Failed to get window size (error: {})", result));
    }

    let mut size = core_graphics::geometry::CGSize::new(0.0, 0.0);
    let success = AXValueGetValue(
        size_value as AXValueRef,
        K_AX_VALUE_TYPE_CG_SIZE,
        &mut size as *mut _ as *mut std::ffi::c_void,
    );

    core_foundation::base::CFRelease(size_value);

    if !success {
        return Err(anyhow!("Failed to extract size value"));
    }

    Ok((size.width, size.height))
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXValueGetValue(value: AXValueRef, value_type: u32, value_ptr: *mut std::ffi::c_void) -> bool;
}

/// Find which display a point is on
#[cfg(target_os = "macos")]
fn find_display_for_point(x: f64, y: f64, displays: &[crate::display::DisplayInfo]) -> usize {
    for display in displays {
        let dx = display.x as f64;
        let dy = display.y as f64;
        let dw = display.width as f64;
        let dh = display.height as f64;

        if x >= dx && x < dx + dw && y >= dy && y < dy + dh {
            return display.index;
        }
    }
    // default to first display
    0
}

/// Get usable bounds for a specific display (excluding menu bar and dock)
#[cfg(target_os = "macos")]
fn get_usable_bounds_for_display(display: &crate::display::DisplayInfo) -> Result<(f64, f64, f64, f64)> {
    use cocoa::base::nil;
    use cocoa::foundation::{NSArray, NSRect};
    use objc::runtime::Object;

    unsafe {
        let screens: *mut Object = msg_send![class!(NSScreen), screens];
        let count: usize = NSArray::count(screens) as usize;

        // find the screen that matches our display by comparing frame coordinates
        let main_screen: *mut Object = msg_send![class!(NSScreen), mainScreen];
        let main_frame: NSRect = msg_send![main_screen, frame];

        for i in 0..count {
            let screen: *mut Object = NSArray::objectAtIndex(screens, i as u64);
            if screen == nil {
                continue;
            }

            let frame: NSRect = msg_send![screen, frame];

            // convert NSScreen coordinates (bottom-left origin) to our coordinates (top-left origin)
            let screen_x = frame.origin.x as i32;
            let screen_y = (main_frame.size.height - frame.origin.y - frame.size.height) as i32;

            // check if this screen matches our display
            if screen_x == display.x && screen_y == display.y {
                let visible_frame: NSRect = msg_send![screen, visibleFrame];

                // convert y coordinate
                let y = main_frame.size.height - visible_frame.origin.y - visible_frame.size.height;

                return Ok((
                    visible_frame.origin.x,
                    y,
                    visible_frame.size.width,
                    visible_frame.size.height,
                ));
            }
        }

        // fallback: use display bounds directly (no menu bar/dock adjustment)
        Ok((
            display.x as f64,
            display.y as f64,
            display.width as f64,
            display.height as f64,
        ))
    }
}

/// Move a window to another display
#[cfg(target_os = "macos")]
pub fn move_to_display(
    app: Option<&AppInfo>,
    target: &crate::display::DisplayTarget,
    verbose: bool,
) -> Result<()> {
    use core_foundation::base::CFTypeRef;
    use crate::display::{get_displays, resolve_target_display};

    if !accessibility::is_trusted() {
        return Err(anyhow!(
            "Accessibility permissions required. Run 'cwm check-permissions' for help."
        ));
    }

    let displays = get_displays()?;
    if displays.len() < 2 {
        return Err(anyhow!("Only one display found. Nothing to move to."));
    }

    let (window, pid) = unsafe {
        if let Some(app_info) = app {
            let w = get_frontmost_window(app_info.pid)?;
            (w, app_info.pid)
        } else {
            get_focused_window()?
        }
    };

    // get current window position to determine which display it's on
    let (wx, wy) = unsafe { get_window_position(window)? };
    let (ww, wh) = unsafe { get_window_size(window)? };

    if verbose {
        println!("Window for PID {}: {}x{} at ({}, {})", pid, ww, wh, wx, wy);
    }

    // find current display
    let current_display_index = find_display_for_point(wx + ww / 2.0, wy + wh / 2.0, &displays);

    if verbose {
        println!("Current display: {}", current_display_index);
    }

    // resolve target display
    let target_display = resolve_target_display(current_display_index, target, &displays)?;

    if verbose {
        println!("Target display: {}", target_display.describe());
    }

    // get usable bounds for target display
    let (tx, ty, tw, th) = get_usable_bounds_for_display(target_display)?;

    if verbose {
        println!("Target display usable bounds: {}x{} at ({}, {})", tw, th, tx, ty);
    }

    // calculate new position - try to maintain relative position within display
    // or just center if window is larger than display
    let new_x;
    let new_y;
    let new_w;
    let new_h;

    if ww > tw || wh > th {
        // window is larger than target display, maximize it
        new_x = tx;
        new_y = ty;
        new_w = tw;
        new_h = th;
    } else {
        // center the window on the target display
        new_x = tx + (tw - ww) / 2.0;
        new_y = ty + (th - wh) / 2.0;
        new_w = ww;
        new_h = wh;
    }

    if verbose {
        println!("Moving window to: {}x{} at ({}, {})", new_w, new_h, new_x, new_y);
    }

    unsafe {
        set_window_position(window, new_x, new_y)?;
        if (new_w - ww).abs() > 1.0 || (new_h - wh).abs() > 1.0 {
            set_window_size(window, new_w, new_h)?;
        }
        core_foundation::base::CFRelease(window as CFTypeRef);
    }

    if verbose {
        println!("Done.");
    } else {
        println!("Window moved to display {}", target_display.index);
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn move_to_display(
    _app: Option<&AppInfo>,
    _target: &crate::display::DisplayTarget,
    _verbose: bool,
) -> Result<()> {
    Err(anyhow!("Move to display is only supported on macOS"))
}
