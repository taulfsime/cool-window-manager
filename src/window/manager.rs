use anyhow::{anyhow, Result};
use std::str::FromStr;

use super::accessibility;
use super::matching::AppInfo;

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_graphics::display::CGDisplay;

/// Target size for resize operations
#[derive(Debug, Clone, PartialEq)]
pub enum ResizeTarget {
    /// Percentage of screen (1-100)
    Percent(u32),
    /// Width in pixels, height optional (maintains aspect ratio if None)
    Pixels { width: u32, height: Option<u32> },
    /// Width in points, height optional (maintains aspect ratio if None)
    Points { width: u32, height: Option<u32> },
}

impl ResizeTarget {
    /// Parse a resize target string
    ///
    /// Supported formats:
    /// - `80` - 80% of screen
    /// - `80%` - 80% of screen (explicit)
    /// - `0.8` - 80% of screen (decimal)
    /// - `full` - 100% of screen
    /// - `1920px` - 1920 pixels wide (height auto)
    /// - `1920x1080px` - exact pixel dimensions
    /// - `800pt` - 800 points wide (height auto)
    /// - `800x600pt` - exact point dimensions
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim().to_lowercase();

        // handle "full" keyword
        if s == "full" {
            return Ok(ResizeTarget::Percent(100));
        }

        // handle pixel dimensions: 1920px or 1920x1080px
        if s.ends_with("px") {
            let dims = s.trim_end_matches("px");
            return Self::parse_dimensions(dims, |w, h| ResizeTarget::Pixels {
                width: w,
                height: h,
            });
        }

        // handle point dimensions: 800pt or 800x600pt
        if s.ends_with("pt") {
            let dims = s.trim_end_matches("pt");
            return Self::parse_dimensions(dims, |w, h| ResizeTarget::Points {
                width: w,
                height: h,
            });
        }

        // handle percentage: 80% or 80
        let percent_str = s.trim_end_matches('%');

        // check if it's a decimal (0.0 to 1.0)
        if percent_str.contains('.') {
            let decimal: f64 = percent_str
                .parse()
                .map_err(|_| anyhow!("Invalid decimal value: '{}'", percent_str))?;

            if decimal <= 0.0 || decimal > 1.0 {
                return Err(anyhow!(
                    "Decimal value must be between 0.0 and 1.0 (exclusive of 0), got: {}",
                    decimal
                ));
            }

            let percent = (decimal * 100.0).round() as u32;
            return Ok(ResizeTarget::Percent(percent.clamp(1, 100)));
        }

        // integer percentage
        let percent: u32 = percent_str
            .parse()
            .map_err(|_| anyhow!("Invalid size value: '{}'. Use a number (1-100), decimal (0.1-1.0), 'full', or dimensions like '1920px'", s))?;

        if percent == 0 || percent > 100 {
            return Err(anyhow!(
                "Percentage must be between 1 and 100, got: {}",
                percent
            ));
        }

        Ok(ResizeTarget::Percent(percent))
    }

    /// Parse dimension string like "1920" or "1920x1080"
    fn parse_dimensions<F>(dims: &str, constructor: F) -> Result<Self>
    where
        F: FnOnce(u32, Option<u32>) -> ResizeTarget,
    {
        if dims.contains('x') {
            // exact dimensions: WIDTHxHEIGHT
            let parts: Vec<&str> = dims.split('x').collect();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid dimensions format: '{}'. Use WIDTHxHEIGHT (e.g., 1920x1080)",
                    dims
                ));
            }

            let width: u32 = parts[0]
                .parse()
                .map_err(|_| anyhow!("Invalid width: '{}'", parts[0]))?;
            let height: u32 = parts[1]
                .parse()
                .map_err(|_| anyhow!("Invalid height: '{}'", parts[1]))?;

            if width == 0 || height == 0 {
                return Err(anyhow!("Width and height must be greater than 0"));
            }

            Ok(constructor(width, Some(height)))
        } else {
            // width only
            let width: u32 = dims
                .parse()
                .map_err(|_| anyhow!("Invalid width: '{}'", dims))?;

            if width == 0 {
                return Err(anyhow!("Width must be greater than 0"));
            }

            Ok(constructor(width, None))
        }
    }
}

impl FromStr for ResizeTarget {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        ResizeTarget::parse(s)
    }
}

type AXUIElementRef = *mut std::ffi::c_void;

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

const K_AX_ERROR_SUCCESS: i32 = 0;

/// Focus an application by bringing it to the foreground
#[allow(deprecated)]
pub fn focus_app(app: &AppInfo, verbose: bool) -> Result<()> {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

    if !accessibility::is_trusted() {
        return Err(anyhow!(
            "Accessibility permissions required. Run 'cwm check-permissions' for help."
        ));
    }

    if verbose {
        println!("Focusing: {} (PID: {})", app.name, app.pid);
    }

    let running_app = NSRunningApplication::runningApplicationWithProcessIdentifier(app.pid);

    let Some(running_app) = running_app else {
        return Err(anyhow!(
            "Could not find running application with PID {}",
            app.pid
        ));
    };

    // ActivateIgnoringOtherApps is deprecated in macOS 14 but still works for older versions
    let success =
        running_app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);

    if !success {
        return Err(anyhow!("Failed to activate application: {}", app.name));
    }

    if verbose {
        println!("Done.");
    }

    Ok(())
}

/// Get the frontmost window of an application
unsafe fn get_frontmost_window(pid: i32) -> Result<AXUIElementRef> {
    use core_foundation::base::CFTypeRef;

    let app_element = AXUIElementCreateApplication(pid);
    if app_element.is_null() {
        return Err(anyhow!("Failed to create AXUIElement for PID {}", pid));
    }

    // first try AXFocusedWindow - works better for focused/active apps
    let focused_window_attr = CFString::new("AXFocusedWindow");
    let mut focused_window: CFTypeRef = std::ptr::null_mut();

    let result = AXUIElementCopyAttributeValue(
        app_element,
        focused_window_attr.as_concrete_TypeRef(),
        &mut focused_window,
    );

    if result == K_AX_ERROR_SUCCESS && !focused_window.is_null() {
        core_foundation::base::CFRelease(app_element as CFTypeRef);
        return Ok(focused_window as AXUIElementRef);
    }

    // fall back to AXWindows array
    let windows_attr = CFString::new("AXWindows");
    let mut windows_value: CFTypeRef = std::ptr::null_mut();

    let result = AXUIElementCopyAttributeValue(
        app_element,
        windows_attr.as_concrete_TypeRef(),
        &mut windows_value,
    );

    if result != K_AX_ERROR_SUCCESS || windows_value.is_null() {
        core_foundation::base::CFRelease(app_element as CFTypeRef);
        return Err(anyhow!(
            "Failed to get windows for application (error: {})",
            result
        ));
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

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: core_foundation::array::CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(
        array: core_foundation::array::CFArrayRef,
        index: isize,
    ) -> *const std::ffi::c_void;
}

/// Get the focused window (from the frontmost application)
unsafe fn get_focused_window() -> Result<(AXUIElementRef, i32)> {
    use objc2_app_kit::NSWorkspace;

    // get the frontmost application
    let workspace = NSWorkspace::sharedWorkspace();
    let frontmost_app = workspace.frontmostApplication();

    let Some(frontmost_app) = frontmost_app else {
        return Err(anyhow!("No frontmost application"));
    };

    let pid = frontmost_app.processIdentifier();

    if pid <= 0 {
        return Err(anyhow!("Invalid PID for frontmost application"));
    }

    let window = get_frontmost_window(pid)?;
    Ok((window, pid))
}

/// Set window position
unsafe fn set_window_position(window: AXUIElementRef, x: f64, y: f64) -> Result<()> {
    use core_foundation::base::CFTypeRef;

    let position_attr = CFString::new("AXPosition");

    // create an AXValue for the position (CGPoint)
    let point = core_graphics::geometry::CGPoint::new(x, y);
    let position_value = AXValueCreate(
        K_AX_VALUE_TYPE_CG_POINT,
        &point as *const _ as *const std::ffi::c_void,
    );

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
        return Err(anyhow!(
            "Failed to set window position: {} ({})",
            err_msg,
            result
        ));
    }

    Ok(())
}

/// Set window size
unsafe fn set_window_size(window: AXUIElementRef, width: f64, height: f64) -> Result<()> {
    use core_foundation::base::CFTypeRef;

    let size_attr = CFString::new("AXSize");

    // create an AXValue for the size (CGSize)
    let size = core_graphics::geometry::CGSize::new(width, height);
    let size_value = AXValueCreate(
        K_AX_VALUE_TYPE_CG_SIZE,
        &size as *const _ as *const std::ffi::c_void,
    );

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

type AXValueRef = *mut std::ffi::c_void;

const K_AX_VALUE_TYPE_CG_POINT: u32 = 1;
const K_AX_VALUE_TYPE_CG_SIZE: u32 = 2;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXValueCreate(value_type: u32, value: *const std::ffi::c_void) -> AXValueRef;
}

/// Get the display bounds, accounting for menu bar
fn get_usable_display_bounds() -> (f64, f64, f64, f64) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;

    // try to get main thread marker - if we're not on main thread, fall back to CGDisplay
    let Some(mtm) = MainThreadMarker::new() else {
        let display = CGDisplay::main();
        let bounds = display.bounds();
        return (
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        );
    };

    let screen = NSScreen::mainScreen(mtm);
    let Some(screen) = screen else {
        // fallback to CGDisplay
        let display = CGDisplay::main();
        let bounds = display.bounds();
        return (
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        );
    };

    // visibleFrame excludes menu bar and dock
    let visible_frame = screen.visibleFrame();
    let frame = screen.frame();

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

/// Maximize an app's window to fill the screen
pub fn maximize_app(app: Option<&AppInfo>, verbose: bool) -> Result<()> {
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
        println!(
            "Usable display bounds: {}x{} at ({}, {})",
            width, height, x, y
        );
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
        println!("App maximized");
    }

    Ok(())
}

/// Launch an application by name
pub fn launch_app(app_name: &str, verbose: bool) -> Result<()> {
    use std::process::Command;

    if verbose {
        println!("Launching: {}", app_name);
    }

    let status = Command::new("open").arg("-a").arg(app_name).status()?;

    if !status.success() {
        return Err(anyhow!("Failed to launch application: {}", app_name));
    }

    if verbose {
        println!("Launched successfully.");
    }

    Ok(())
}

/// Get current window position
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
unsafe fn get_window_size(window: AXUIElementRef) -> Result<(f64, f64)> {
    use core_foundation::base::CFTypeRef;

    let size_attr = CFString::new("AXSize");
    let mut size_value: CFTypeRef = std::ptr::null_mut();

    let result =
        AXUIElementCopyAttributeValue(window, size_attr.as_concrete_TypeRef(), &mut size_value);

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

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXValueGetValue(
        value: AXValueRef,
        value_type: u32,
        value_ptr: *mut std::ffi::c_void,
    ) -> bool;
}

/// Find which display a point is on
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
fn get_usable_bounds_for_display(
    display: &crate::display::DisplayInfo,
) -> Result<(f64, f64, f64, f64)> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;

    // try to get main thread marker - if we're not on main thread, fall back to display bounds
    let Some(mtm) = MainThreadMarker::new() else {
        return Ok((
            display.x as f64,
            display.y as f64,
            display.width as f64,
            display.height as f64,
        ));
    };

    let screens = NSScreen::screens(mtm);
    let main_screen = NSScreen::mainScreen(mtm);

    let Some(main_screen) = main_screen else {
        // fallback: use display bounds directly
        return Ok((
            display.x as f64,
            display.y as f64,
            display.width as f64,
            display.height as f64,
        ));
    };

    let main_frame = main_screen.frame();

    for screen in screens.iter() {
        let frame = screen.frame();

        // convert NSScreen coordinates (bottom-left origin) to our coordinates (top-left origin)
        let screen_x = frame.origin.x as i32;
        let screen_y = (main_frame.size.height - frame.origin.y - frame.size.height) as i32;

        // check if this screen matches our display
        if screen_x == display.x && screen_y == display.y {
            let visible_frame = screen.visibleFrame();

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

/// Move a window to another display
pub fn move_to_display(
    app: Option<&AppInfo>,
    target: &crate::display::DisplayTarget,
    verbose: bool,
) -> Result<()> {
    move_to_display_with_aliases(app, target, verbose, &Default::default())?;
    Ok(())
}

/// Move window to display, returns (display_index, display_name) on success
pub fn move_to_display_with_aliases(
    app: Option<&AppInfo>,
    target: &crate::display::DisplayTarget,
    verbose: bool,
    display_aliases: &std::collections::HashMap<String, Vec<String>>,
) -> Result<(usize, String)> {
    use crate::display::{get_displays, resolve_target_display_with_aliases};
    use core_foundation::base::CFTypeRef;

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
    let target_display = resolve_target_display_with_aliases(
        current_display_index,
        target,
        &displays,
        display_aliases,
    )?;

    if verbose {
        println!("Target display: {}", target_display.describe());
    }

    // get usable bounds for target display
    let (tx, ty, tw, th) = get_usable_bounds_for_display(target_display)?;

    if verbose {
        println!(
            "Target display usable bounds: {}x{} at ({}, {})",
            tw, th, tx, ty
        );
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
        println!(
            "Moving window to: {}x{} at ({}, {})",
            new_w, new_h, new_x, new_y
        );
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
    }
    // silent on success in non-verbose mode (Unix convention)

    Ok((target_display.index, target_display.name.clone()))
}

/// Resize an app's window to a target size (centered)
///
/// The `overflow` parameter controls whether the window can extend beyond screen bounds.
/// If false (default), dimensions are clamped to the usable display area.
///
/// Returns the final (width, height) of the window.
pub fn resize_app(
    app: Option<&AppInfo>,
    target: &ResizeTarget,
    overflow: bool,
    verbose: bool,
) -> Result<(u32, u32)> {
    use core_foundation::base::CFTypeRef;

    if !accessibility::is_trusted() {
        return Err(anyhow!(
            "Accessibility permissions required. Run 'cwm check-permissions' for help."
        ));
    }

    // 100% is just maximize (but we need to return size)
    if matches!(target, ResizeTarget::Percent(100)) {
        maximize_app(app, verbose)?;
        // get the maximized size
        let (_, _, dw, dh) = get_usable_display_bounds();
        return Ok((dw as u32, dh as u32));
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
        println!("Resizing window for PID {} to {:?}", pid, target);
    }

    // get usable display bounds (excluding menu bar and dock)
    let (dx, dy, dw, dh) = get_usable_display_bounds();

    if verbose {
        println!("Usable display bounds: {}x{} at ({}, {})", dw, dh, dx, dy);
    }

    // calculate new dimensions based on target
    let (mut new_w, mut new_h) = match target {
        ResizeTarget::Percent(percent) => {
            let scale = *percent as f64 / 100.0;
            (dw * scale, dh * scale)
        }
        ResizeTarget::Pixels { width, height } => {
            let w = *width as f64;
            let h = match height {
                Some(h) => *h as f64,
                None => {
                    // maintain display aspect ratio
                    w * (dh / dw)
                }
            };
            (w, h)
        }
        ResizeTarget::Points { width, height } => {
            // on macOS, points are the same as pixels for our purposes
            // (the system handles scaling for Retina displays)
            let w = *width as f64;
            let h = match height {
                Some(h) => *h as f64,
                None => {
                    // maintain display aspect ratio
                    w * (dh / dw)
                }
            };
            (w, h)
        }
    };

    // clamp to screen bounds unless overflow is enabled
    if !overflow {
        if new_w > dw {
            if verbose {
                println!("Clamping width from {} to {} (screen limit)", new_w, dw);
            }
            new_w = dw;
        }
        if new_h > dh {
            if verbose {
                println!("Clamping height from {} to {} (screen limit)", new_h, dh);
            }
            new_h = dh;
        }
    }

    // center the window
    let new_x = dx + (dw - new_w) / 2.0;
    let new_y = dy + (dh - new_h) / 2.0;

    if verbose {
        println!("New size: {}x{} at ({}, {})", new_w, new_h, new_x, new_y);
    }

    unsafe {
        set_window_position(window, new_x, new_y)?;
        set_window_size(window, new_w, new_h)?;
        core_foundation::base::CFRelease(window as CFTypeRef);
    }

    // format output message based on target type
    let _size_desc = match target {
        ResizeTarget::Percent(p) => format!("{}%", p),
        ResizeTarget::Pixels { width, height } => match height {
            Some(h) => format!("{}x{}px", width, h),
            None => format!("{}px wide", width),
        },
        ResizeTarget::Points { width, height } => match height {
            Some(h) => format!("{}x{}pt", width, h),
            None => format!("{}pt wide", width),
        },
    };

    if verbose {
        println!("Done.");
    }
    // silent on success in non-verbose mode (Unix convention)

    Ok((new_w as u32, new_h as u32))
}

/// Window data for JSON output
#[derive(serde::Serialize)]
pub struct WindowData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Display data for JSON output
#[derive(serde::Serialize)]
pub struct DisplayDataInfo {
    pub index: usize,
    pub name: String,
}

/// Get window title from AXUIElement
unsafe fn get_window_title(window: AXUIElementRef) -> Option<String> {
    use core_foundation::base::CFTypeRef;
    use core_foundation::string::CFString;

    let title_attr = CFString::new("AXTitle");
    let mut title_value: CFTypeRef = std::ptr::null();

    let result =
        AXUIElementCopyAttributeValue(window, title_attr.as_concrete_TypeRef(), &mut title_value);

    if result != K_AX_ERROR_SUCCESS || title_value.is_null() {
        return None;
    }

    // convert CFString to Rust String
    let cf_string = core_foundation::string::CFString::wrap_under_get_rule(
        title_value as core_foundation::string::CFStringRef,
    );
    let title = cf_string.to_string();

    core_foundation::base::CFRelease(title_value);

    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

/// Get information about the currently focused window
pub fn get_focused_window_info() -> Result<(AppInfo, WindowData, DisplayDataInfo)> {
    use objc2_app_kit::NSWorkspace;

    if !accessibility::is_trusted() {
        return Err(anyhow!(
            "Accessibility permissions required. Run 'cwm check-permissions' for help."
        ));
    }

    let (window, pid) = unsafe { get_focused_window()? };

    // get app info
    let workspace = NSWorkspace::sharedWorkspace();
    let frontmost_app = workspace.frontmostApplication();

    let app_name = frontmost_app
        .as_ref()
        .and_then(|app| app.localizedName())
        .map(|name| name.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let bundle_id = frontmost_app
        .as_ref()
        .and_then(|app| app.bundleIdentifier())
        .map(|id| id.to_string());

    let app = AppInfo {
        name: app_name,
        pid,
        bundle_id,
        titles: vec![],
    };

    // get window position and size
    let (x, y) = unsafe { get_window_position(window)? };
    let (w, h) = unsafe { get_window_size(window)? };
    let title = unsafe { get_window_title(window) };

    unsafe {
        core_foundation::base::CFRelease(window as core_foundation::base::CFTypeRef);
    }

    let window_data = WindowData {
        title,
        x: x as i32,
        y: y as i32,
        width: w as u32,
        height: h as u32,
    };

    // find which display the window is on
    let displays = crate::display::get_displays()?;
    let display_index = find_display_for_point(x + w / 2.0, y + h / 2.0, &displays);
    let display = displays
        .iter()
        .find(|d| d.index == display_index)
        .ok_or_else(|| anyhow!("Display not found"))?;

    let display_data = DisplayDataInfo {
        index: display.index,
        name: display.name.clone(),
    };

    Ok((app, window_data, display_data))
}

/// Get information about a specific app's window
pub fn get_window_info_for_app(app: &AppInfo) -> Result<(AppInfo, WindowData, DisplayDataInfo)> {
    if !accessibility::is_trusted() {
        return Err(anyhow!(
            "Accessibility permissions required. Run 'cwm check-permissions' for help."
        ));
    }

    let window = unsafe { get_frontmost_window(app.pid)? };

    // get window position and size
    let (x, y) = unsafe { get_window_position(window)? };
    let (w, h) = unsafe { get_window_size(window)? };
    let title = unsafe { get_window_title(window) };

    unsafe {
        core_foundation::base::CFRelease(window as core_foundation::base::CFTypeRef);
    }

    let window_data = WindowData {
        title,
        x: x as i32,
        y: y as i32,
        width: w as u32,
        height: h as u32,
    };

    // find which display the window is on
    let displays = crate::display::get_displays()?;
    let display_index = find_display_for_point(x + w / 2.0, y + h / 2.0, &displays);
    let display = displays
        .iter()
        .find(|d| d.index == display_index)
        .ok_or_else(|| anyhow!("Display not found"))?;

    let display_data = DisplayDataInfo {
        index: display.index,
        name: display.name.clone(),
    };

    Ok((app.clone(), window_data, display_data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_target_parse_integer_percent() {
        assert_eq!(
            ResizeTarget::parse("80").unwrap(),
            ResizeTarget::Percent(80)
        );
        assert_eq!(ResizeTarget::parse("1").unwrap(), ResizeTarget::Percent(1));
        assert_eq!(
            ResizeTarget::parse("100").unwrap(),
            ResizeTarget::Percent(100)
        );
        assert_eq!(
            ResizeTarget::parse("50").unwrap(),
            ResizeTarget::Percent(50)
        );
    }

    #[test]
    fn test_resize_target_parse_explicit_percent() {
        assert_eq!(
            ResizeTarget::parse("80%").unwrap(),
            ResizeTarget::Percent(80)
        );
        assert_eq!(
            ResizeTarget::parse("100%").unwrap(),
            ResizeTarget::Percent(100)
        );
        assert_eq!(ResizeTarget::parse("1%").unwrap(), ResizeTarget::Percent(1));
    }

    #[test]
    fn test_resize_target_parse_decimal() {
        assert_eq!(
            ResizeTarget::parse("0.8").unwrap(),
            ResizeTarget::Percent(80)
        );
        assert_eq!(
            ResizeTarget::parse("0.5").unwrap(),
            ResizeTarget::Percent(50)
        );
        assert_eq!(
            ResizeTarget::parse("1.0").unwrap(),
            ResizeTarget::Percent(100)
        );
        assert_eq!(
            ResizeTarget::parse("0.75").unwrap(),
            ResizeTarget::Percent(75)
        );
        assert_eq!(
            ResizeTarget::parse("0.01").unwrap(),
            ResizeTarget::Percent(1)
        );
    }

    #[test]
    fn test_resize_target_parse_full() {
        assert_eq!(
            ResizeTarget::parse("full").unwrap(),
            ResizeTarget::Percent(100)
        );
        assert_eq!(
            ResizeTarget::parse("FULL").unwrap(),
            ResizeTarget::Percent(100)
        );
        assert_eq!(
            ResizeTarget::parse("Full").unwrap(),
            ResizeTarget::Percent(100)
        );
    }

    #[test]
    fn test_resize_target_parse_pixels_width_only() {
        assert_eq!(
            ResizeTarget::parse("1920px").unwrap(),
            ResizeTarget::Pixels {
                width: 1920,
                height: None
            }
        );
        assert_eq!(
            ResizeTarget::parse("800px").unwrap(),
            ResizeTarget::Pixels {
                width: 800,
                height: None
            }
        );
    }

    #[test]
    fn test_resize_target_parse_pixels_exact() {
        assert_eq!(
            ResizeTarget::parse("1920x1080px").unwrap(),
            ResizeTarget::Pixels {
                width: 1920,
                height: Some(1080)
            }
        );
        assert_eq!(
            ResizeTarget::parse("800x600px").unwrap(),
            ResizeTarget::Pixels {
                width: 800,
                height: Some(600)
            }
        );
    }

    #[test]
    fn test_resize_target_parse_points_width_only() {
        assert_eq!(
            ResizeTarget::parse("800pt").unwrap(),
            ResizeTarget::Points {
                width: 800,
                height: None
            }
        );
        assert_eq!(
            ResizeTarget::parse("1200pt").unwrap(),
            ResizeTarget::Points {
                width: 1200,
                height: None
            }
        );
    }

    #[test]
    fn test_resize_target_parse_points_exact() {
        assert_eq!(
            ResizeTarget::parse("800x600pt").unwrap(),
            ResizeTarget::Points {
                width: 800,
                height: Some(600)
            }
        );
        assert_eq!(
            ResizeTarget::parse("1440x900pt").unwrap(),
            ResizeTarget::Points {
                width: 1440,
                height: Some(900)
            }
        );
    }

    #[test]
    fn test_resize_target_parse_case_insensitive() {
        assert_eq!(
            ResizeTarget::parse("1920PX").unwrap(),
            ResizeTarget::Pixels {
                width: 1920,
                height: None
            }
        );
        assert_eq!(
            ResizeTarget::parse("800PT").unwrap(),
            ResizeTarget::Points {
                width: 800,
                height: None
            }
        );
        assert_eq!(
            ResizeTarget::parse("1920X1080PX").unwrap(),
            ResizeTarget::Pixels {
                width: 1920,
                height: Some(1080)
            }
        );
    }

    #[test]
    fn test_resize_target_parse_whitespace() {
        assert_eq!(
            ResizeTarget::parse("  80  ").unwrap(),
            ResizeTarget::Percent(80)
        );
        assert_eq!(
            ResizeTarget::parse(" 1920px ").unwrap(),
            ResizeTarget::Pixels {
                width: 1920,
                height: None
            }
        );
    }

    #[test]
    fn test_resize_target_parse_invalid_percent() {
        assert!(ResizeTarget::parse("0").is_err());
        assert!(ResizeTarget::parse("101").is_err());
        assert!(ResizeTarget::parse("200").is_err());
        assert!(ResizeTarget::parse("-50").is_err());
    }

    #[test]
    fn test_resize_target_parse_invalid_decimal() {
        assert!(ResizeTarget::parse("0.0").is_err());
        assert!(ResizeTarget::parse("1.5").is_err());
        assert!(ResizeTarget::parse("-0.5").is_err());
    }

    #[test]
    fn test_resize_target_parse_invalid_dimensions() {
        assert!(ResizeTarget::parse("0px").is_err());
        assert!(ResizeTarget::parse("0x0px").is_err());
        assert!(ResizeTarget::parse("1920x0px").is_err());
        assert!(ResizeTarget::parse("0x1080px").is_err());
        assert!(ResizeTarget::parse("abcpx").is_err());
        assert!(ResizeTarget::parse("1920xabcpx").is_err());
    }

    #[test]
    fn test_resize_target_parse_invalid_format() {
        assert!(ResizeTarget::parse("").is_err());
        assert!(ResizeTarget::parse("abc").is_err());
        assert!(ResizeTarget::parse("px").is_err());
        assert!(ResizeTarget::parse("xpx").is_err());
    }

    #[test]
    fn test_resize_target_from_str() {
        let target: ResizeTarget = "80".parse().unwrap();
        assert_eq!(target, ResizeTarget::Percent(80));

        let target: ResizeTarget = "1920px".parse().unwrap();
        assert_eq!(
            target,
            ResizeTarget::Pixels {
                width: 1920,
                height: None
            }
        );
    }

    // ========================================================================
    // find_display_for_point tests
    // ========================================================================

    fn create_test_displays() -> Vec<crate::display::DisplayInfo> {
        vec![
            crate::display::DisplayInfo {
                index: 0,
                name: "Main Display".to_string(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                is_main: true,
                is_builtin: true,
                display_id: 1,
                vendor_id: None,
                model_id: None,
                serial_number: None,
                unit_number: 0,
            },
            crate::display::DisplayInfo {
                index: 1,
                name: "External Display".to_string(),
                width: 2560,
                height: 1440,
                x: 1920,
                y: 0,
                is_main: false,
                is_builtin: false,
                display_id: 2,
                vendor_id: Some(1234),
                model_id: Some(5678),
                serial_number: Some(9999),
                unit_number: 1,
            },
        ]
    }

    #[test]
    fn test_find_display_for_point_on_main_display() {
        let displays = create_test_displays();

        // center of main display
        assert_eq!(find_display_for_point(960.0, 540.0, &displays), 0);

        // top-left corner of main display
        assert_eq!(find_display_for_point(0.0, 0.0, &displays), 0);

        // bottom-right corner of main display (just inside)
        assert_eq!(find_display_for_point(1919.0, 1079.0, &displays), 0);
    }

    #[test]
    fn test_find_display_for_point_on_external_display() {
        let displays = create_test_displays();

        // center of external display
        assert_eq!(find_display_for_point(3200.0, 720.0, &displays), 1);

        // top-left corner of external display
        assert_eq!(find_display_for_point(1920.0, 0.0, &displays), 1);

        // bottom-right corner of external display (just inside)
        assert_eq!(find_display_for_point(4479.0, 1439.0, &displays), 1);
    }

    #[test]
    fn test_find_display_for_point_outside_all_displays() {
        let displays = create_test_displays();

        // point outside all displays defaults to 0
        assert_eq!(find_display_for_point(-100.0, -100.0, &displays), 0);
        assert_eq!(find_display_for_point(10000.0, 10000.0, &displays), 0);
    }

    #[test]
    fn test_find_display_for_point_empty_displays() {
        let displays: Vec<crate::display::DisplayInfo> = vec![];

        // with no displays, defaults to 0
        assert_eq!(find_display_for_point(100.0, 100.0, &displays), 0);
    }

    #[test]
    fn test_find_display_for_point_on_boundary() {
        let displays = create_test_displays();

        // exactly on the boundary between displays (x=1920 is external)
        assert_eq!(find_display_for_point(1920.0, 500.0, &displays), 1);

        // just before boundary (x=1919 is main)
        assert_eq!(find_display_for_point(1919.0, 500.0, &displays), 0);
    }

    // ========================================================================
    // WindowData serialization tests
    // ========================================================================

    #[test]
    fn test_window_data_serialization_with_title() {
        let data = WindowData {
            title: Some("Test Window".to_string()),
            x: 100,
            y: 200,
            width: 800,
            height: 600,
        };

        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["title"], "Test Window");
        assert_eq!(json["x"], 100);
        assert_eq!(json["y"], 200);
        assert_eq!(json["width"], 800);
        assert_eq!(json["height"], 600);
    }

    #[test]
    fn test_window_data_serialization_without_title() {
        let data = WindowData {
            title: None,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        let json = serde_json::to_value(&data).unwrap();
        // title should be skipped when None
        assert!(json.get("title").is_none());
        assert_eq!(json["x"], 0);
        assert_eq!(json["y"], 0);
        assert_eq!(json["width"], 1920);
        assert_eq!(json["height"], 1080);
    }

    #[test]
    fn test_window_data_serialization_negative_coords() {
        let data = WindowData {
            title: None,
            x: -100,
            y: -50,
            width: 800,
            height: 600,
        };

        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["x"], -100);
        assert_eq!(json["y"], -50);
    }

    // ========================================================================
    // DisplayDataInfo serialization tests
    // ========================================================================

    #[test]
    fn test_display_data_info_serialization() {
        let data = DisplayDataInfo {
            index: 0,
            name: "Built-in Display".to_string(),
        };

        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["index"], 0);
        assert_eq!(json["name"], "Built-in Display");
    }

    #[test]
    fn test_display_data_info_serialization_external() {
        let data = DisplayDataInfo {
            index: 1,
            name: "LG UltraWide".to_string(),
        };

        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["index"], 1);
        assert_eq!(json["name"], "LG UltraWide");
    }

    // ========================================================================
    // ResizeTarget debug/clone tests
    // ========================================================================

    #[test]
    fn test_resize_target_debug() {
        let target = ResizeTarget::Percent(80);
        let debug_str = format!("{:?}", target);
        assert!(debug_str.contains("Percent"));
        assert!(debug_str.contains("80"));
    }

    #[test]
    fn test_resize_target_clone() {
        let target = ResizeTarget::Pixels {
            width: 1920,
            height: Some(1080),
        };
        let cloned = target.clone();
        assert_eq!(target, cloned);
    }

    #[test]
    fn test_resize_target_partial_eq() {
        let a = ResizeTarget::Percent(50);
        let b = ResizeTarget::Percent(50);
        let c = ResizeTarget::Percent(60);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ========================================================================
    // Additional edge cases for parse_dimensions
    // ========================================================================

    #[test]
    fn test_resize_target_parse_dimensions_multiple_x() {
        // multiple x separators should fail
        assert!(ResizeTarget::parse("100x200x300px").is_err());
    }

    #[test]
    fn test_resize_target_parse_very_large_values() {
        // very large pixel values should work
        let target = ResizeTarget::parse("10000x10000px").unwrap();
        assert_eq!(
            target,
            ResizeTarget::Pixels {
                width: 10000,
                height: Some(10000)
            }
        );
    }

    #[test]
    fn test_resize_target_parse_single_digit() {
        assert_eq!(ResizeTarget::parse("1").unwrap(), ResizeTarget::Percent(1));
        assert_eq!(
            ResizeTarget::parse("1px").unwrap(),
            ResizeTarget::Pixels {
                width: 1,
                height: None
            }
        );
        assert_eq!(
            ResizeTarget::parse("1pt").unwrap(),
            ResizeTarget::Points {
                width: 1,
                height: None
            }
        );
    }

    #[test]
    fn test_resize_target_parse_decimal_edge_cases() {
        // very small decimal rounds to 0, but clamped to 1
        assert_eq!(
            ResizeTarget::parse("0.001").unwrap(),
            ResizeTarget::Percent(1)
        );

        // decimal that rounds up
        assert_eq!(
            ResizeTarget::parse("0.995").unwrap(),
            ResizeTarget::Percent(100)
        );
    }
}
