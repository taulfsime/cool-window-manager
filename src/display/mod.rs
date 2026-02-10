use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_main: bool,
    pub display_id: u32,
    pub vendor_id: Option<u32>,
    pub model_id: Option<u32>,
    pub serial_number: Option<u32>,
    pub unit_number: u32,
    pub is_builtin: bool,
}

impl DisplayInfo {
    pub fn describe(&self) -> String {
        let main_marker = if self.is_main { " (main)" } else { "" };
        format!(
            "Display {}: {} - {}x{} at ({}, {}){}",
            self.index, self.name, self.width, self.height, self.x, self.y, main_marker
        )
    }

    pub fn describe_detailed(&self) -> String {
        let mut lines = vec![format!("Display {}:", self.index)];
        lines.push(format!("  Name:          {}", self.name));
        lines.push(format!("  Resolution:    {}x{}", self.width, self.height));
        lines.push(format!("  Position:      ({}, {})", self.x, self.y));
        lines.push(format!("  Display ID:    {}", self.display_id));
        lines.push(format!("  Unit Number:   {}", self.unit_number));

        if let Some(vendor) = self.vendor_id {
            lines.push(format!("  Vendor ID:     0x{:04X} ({})", vendor, vendor_name(vendor)));
        }
        if let Some(model) = self.model_id {
            lines.push(format!("  Model ID:      0x{:04X}", model));
        }
        if let Some(serial) = self.serial_number {
            lines.push(format!("  Serial Number: {}", serial));
        }

        lines.push(format!("  Built-in:      {}", if self.is_builtin { "Yes" } else { "No" }));
        lines.push(format!("  Main Display:  {}", if self.is_main { "Yes" } else { "No" }));

        lines.join("\n")
    }

    /// unique identifier that persists across reboots
    /// format: vendor_model_serial (if available) or display_id
    pub fn unique_id(&self) -> String {
        match (self.vendor_id, self.model_id, self.serial_number) {
            (Some(v), Some(m), Some(s)) => format!("{:04X}_{:04X}_{}", v, m, s),
            (Some(v), Some(m), None) => format!("{:04X}_{:04X}_unit{}", v, m, self.unit_number),
            _ => format!("display_{}", self.display_id),
        }
    }
}

fn vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x0610 => "Apple",
        0x1E6D => "LG",
        0x10AC => "Dell",
        0x0469 => "HP",
        0x34AC => "Samsung",
        0x0E6A => "ASUS",
        0x0D1E => "BenQ",
        0x0220 => "ViewSonic",
        0x0026 => "Acer",
        0x4C2D => "Lenovo",
        _ => "Unknown",
    }
}

/// Get list of all displays
#[cfg(target_os = "macos")]
pub fn get_displays() -> Result<Vec<DisplayInfo>> {
    use core_graphics::display::CGDisplay;

    let display_ids = CGDisplay::active_displays()
        .map_err(|e| anyhow!("Failed to get displays: {:?}", e))?;

    let main_display_id = CGDisplay::main().id;

    let mut displays: Vec<DisplayInfo> = display_ids
        .iter()
        .enumerate()
        .map(|(index, &id)| {
            let display = CGDisplay::new(id);
            let bounds = display.bounds();

            let vendor_id = display.vendor_number();
            let model_id = display.model_number();
            let serial_number = display.serial_number();
            let unit_number = display.unit_number();
            let is_builtin = display.is_builtin();

            // try to get a meaningful name
            let name = get_display_name(id, vendor_id, model_id, is_builtin);

            DisplayInfo {
                index,
                name,
                width: bounds.size.width as u32,
                height: bounds.size.height as u32,
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                is_main: id == main_display_id,
                display_id: id,
                vendor_id: if vendor_id != 0 { Some(vendor_id) } else { None },
                model_id: if model_id != 0 { Some(model_id) } else { None },
                serial_number: if serial_number != 0 { Some(serial_number) } else { None },
                unit_number,
                is_builtin,
            }
        })
        .collect();

    // sort by x position (left to right)
    displays.sort_by_key(|d| d.x);

    // re-assign indices after sorting
    for (i, display) in displays.iter_mut().enumerate() {
        display.index = i;
    }

    Ok(displays)
}

#[cfg(target_os = "macos")]
fn get_display_name(display_id: u32, vendor_id: u32, model_id: u32, is_builtin: bool) -> String {
    // try to get the localized name from IOKit via NSScreen
    if let Some(name) = get_nsscreen_name(display_id) {
        return name;
    }

    // fallback to a descriptive name
    if is_builtin {
        return "Built-in Display".to_string();
    }

    let vendor = vendor_name(vendor_id);
    if vendor != "Unknown" {
        format!("{} Display (0x{:04X})", vendor, model_id)
    } else {
        format!("External Display {}", display_id)
    }
}

#[cfg(target_os = "macos")]
fn get_nsscreen_name(display_id: u32) -> Option<String> {
    use objc2_app_kit::NSScreen;
    use objc2::MainThreadMarker;
    use objc2::msg_send;

    // NSScreen requires main thread
    let mtm = MainThreadMarker::new()?;
    let screens = NSScreen::screens(mtm);

    for screen in screens.iter() {
        let device_desc = screen.deviceDescription();
        let screen_number_key = objc2_foundation::NSString::from_str("NSScreenNumber");

        if let Some(screen_number) = device_desc.objectForKey(&screen_number_key) {
            // NSScreenNumber is an NSNumber containing the CGDirectDisplayID
            let screen_id: u32 = unsafe {
                msg_send![&*screen_number, unsignedIntValue]
            };

            if screen_id == display_id {
                let localized_name = screen.localizedName();
                return Some(localized_name.to_string());
            }
        }
    }

    None
}

#[cfg(not(target_os = "macos"))]
pub fn get_displays() -> Result<Vec<DisplayInfo>> {
    Err(anyhow!("Display enumeration is only supported on macOS"))
}

/// Parse display target from string
#[derive(Debug, Clone)]
pub enum DisplayTarget {
    Next,
    Prev,
    Index(usize),
}

impl DisplayTarget {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "next" => Ok(DisplayTarget::Next),
            "prev" | "previous" => Ok(DisplayTarget::Prev),
            _ => {
                let index: usize = s
                    .parse()
                    .map_err(|_| anyhow!("Invalid display target: '{}'. Use 'next', 'prev', or a display index (0-based)", s))?;
                Ok(DisplayTarget::Index(index))
            }
        }
    }
}

/// Get the target display based on current display and target
pub fn resolve_target_display<'a>(
    current_display_index: usize,
    target: &DisplayTarget,
    displays: &'a [DisplayInfo],
) -> Result<&'a DisplayInfo> {
    if displays.is_empty() {
        return Err(anyhow!("No displays found"));
    }

    let target_index = match target {
        DisplayTarget::Next => (current_display_index + 1) % displays.len(),
        DisplayTarget::Prev => {
            if current_display_index == 0 {
                displays.len() - 1
            } else {
                current_display_index - 1
            }
        }
        DisplayTarget::Index(index) => {
            if *index >= displays.len() {
                return Err(anyhow!(
                    "Display index {} out of range. Available displays: 0-{}",
                    index,
                    displays.len() - 1
                ));
            }
            *index
        }
    };

    Ok(&displays[target_index])
}

pub fn print_displays(detailed: bool) -> Result<()> {
    let displays = get_displays()?;

    if displays.is_empty() {
        println!("No displays found");
        return Ok(());
    }

    if detailed {
        println!("Connected Displays:\n");
        for (i, display) in displays.iter().enumerate() {
            println!("{}", display.describe_detailed());
            println!("  Unique ID:     {}", display.unique_id());
            if i < displays.len() - 1 {
                println!();
            }
        }
    } else {
        println!("Available displays:");
        for display in &displays {
            println!("  {}", display.describe());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_displays() -> Vec<DisplayInfo> {
        vec![
            DisplayInfo {
                index: 0,
                name: "Built-in Display".to_string(),
                width: 2560,
                height: 1600,
                x: 0,
                y: 0,
                is_main: true,
                display_id: 1,
                vendor_id: Some(0x0610),
                model_id: Some(0xA032),
                serial_number: None,
                unit_number: 0,
                is_builtin: true,
            },
            DisplayInfo {
                index: 1,
                name: "External Monitor".to_string(),
                width: 3840,
                height: 2160,
                x: 2560,
                y: 0,
                is_main: false,
                display_id: 2,
                vendor_id: Some(0x1E6D),
                model_id: Some(0x5B11),
                serial_number: Some(12345),
                unit_number: 0,
                is_builtin: false,
            },
            DisplayInfo {
                index: 2,
                name: "Third Display".to_string(),
                width: 1920,
                height: 1080,
                x: 6400,
                y: 0,
                is_main: false,
                display_id: 3,
                vendor_id: Some(0x10AC),
                model_id: Some(0xD0B3),
                serial_number: Some(67890),
                unit_number: 0,
                is_builtin: false,
            },
        ]
    }

    #[test]
    fn test_display_info_describe() {
        let display = DisplayInfo {
            index: 0,
            name: "Test Display".to_string(),
            width: 1920,
            height: 1080,
            x: 100,
            y: 200,
            is_main: false,
            display_id: 1,
            vendor_id: None,
            model_id: None,
            serial_number: None,
            unit_number: 0,
            is_builtin: false,
        };
        assert_eq!(
            display.describe(),
            "Display 0: Test Display - 1920x1080 at (100, 200)"
        );
    }

    #[test]
    fn test_display_info_describe_main() {
        let display = DisplayInfo {
            index: 0,
            name: "Main Display".to_string(),
            width: 2560,
            height: 1440,
            x: 0,
            y: 0,
            is_main: true,
            display_id: 1,
            vendor_id: Some(0x0610),
            model_id: Some(0xA032),
            serial_number: None,
            unit_number: 0,
            is_builtin: true,
        };
        assert_eq!(
            display.describe(),
            "Display 0: Main Display - 2560x1440 at (0, 0) (main)"
        );
    }

    #[test]
    fn test_unique_id_with_serial() {
        let display = DisplayInfo {
            index: 0,
            name: "Test".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            is_main: false,
            display_id: 123,
            vendor_id: Some(0x1E6D),
            model_id: Some(0x5B11),
            serial_number: Some(12345),
            unit_number: 0,
            is_builtin: false,
        };
        assert_eq!(display.unique_id(), "1E6D_5B11_12345");
    }

    #[test]
    fn test_unique_id_without_serial() {
        let display = DisplayInfo {
            index: 0,
            name: "Test".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            is_main: false,
            display_id: 123,
            vendor_id: Some(0x0610),
            model_id: Some(0xA032),
            serial_number: None,
            unit_number: 2,
            is_builtin: true,
        };
        assert_eq!(display.unique_id(), "0610_A032_unit2");
    }

    #[test]
    fn test_unique_id_fallback() {
        let display = DisplayInfo {
            index: 0,
            name: "Test".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            is_main: false,
            display_id: 456,
            vendor_id: None,
            model_id: None,
            serial_number: None,
            unit_number: 0,
            is_builtin: false,
        };
        assert_eq!(display.unique_id(), "display_456");
    }

    #[test]
    fn test_display_target_parse_next() {
        let target = DisplayTarget::parse("next").unwrap();
        assert!(matches!(target, DisplayTarget::Next));

        let target = DisplayTarget::parse("NEXT").unwrap();
        assert!(matches!(target, DisplayTarget::Next));

        let target = DisplayTarget::parse("Next").unwrap();
        assert!(matches!(target, DisplayTarget::Next));
    }

    #[test]
    fn test_display_target_parse_prev() {
        let target = DisplayTarget::parse("prev").unwrap();
        assert!(matches!(target, DisplayTarget::Prev));

        let target = DisplayTarget::parse("PREV").unwrap();
        assert!(matches!(target, DisplayTarget::Prev));

        let target = DisplayTarget::parse("previous").unwrap();
        assert!(matches!(target, DisplayTarget::Prev));

        let target = DisplayTarget::parse("PREVIOUS").unwrap();
        assert!(matches!(target, DisplayTarget::Prev));
    }

    #[test]
    fn test_display_target_parse_index() {
        let target = DisplayTarget::parse("0").unwrap();
        assert!(matches!(target, DisplayTarget::Index(0)));

        let target = DisplayTarget::parse("1").unwrap();
        assert!(matches!(target, DisplayTarget::Index(1)));

        let target = DisplayTarget::parse("42").unwrap();
        assert!(matches!(target, DisplayTarget::Index(42)));
    }

    #[test]
    fn test_display_target_parse_invalid() {
        assert!(DisplayTarget::parse("invalid").is_err());
        assert!(DisplayTarget::parse("").is_err());
        assert!(DisplayTarget::parse("abc").is_err());
        assert!(DisplayTarget::parse("-1").is_err());
    }

    #[test]
    fn test_resolve_target_display_next() {
        let displays = test_displays();

        // from display 0, next should be 1
        let result = resolve_target_display(0, &DisplayTarget::Next, &displays).unwrap();
        assert_eq!(result.index, 1);

        // from display 1, next should be 2
        let result = resolve_target_display(1, &DisplayTarget::Next, &displays).unwrap();
        assert_eq!(result.index, 2);

        // from display 2, next should wrap to 0
        let result = resolve_target_display(2, &DisplayTarget::Next, &displays).unwrap();
        assert_eq!(result.index, 0);
    }

    #[test]
    fn test_resolve_target_display_prev() {
        let displays = test_displays();

        // from display 2, prev should be 1
        let result = resolve_target_display(2, &DisplayTarget::Prev, &displays).unwrap();
        assert_eq!(result.index, 1);

        // from display 1, prev should be 0
        let result = resolve_target_display(1, &DisplayTarget::Prev, &displays).unwrap();
        assert_eq!(result.index, 0);

        // from display 0, prev should wrap to 2
        let result = resolve_target_display(0, &DisplayTarget::Prev, &displays).unwrap();
        assert_eq!(result.index, 2);
    }

    #[test]
    fn test_resolve_target_display_index() {
        let displays = test_displays();

        let result = resolve_target_display(0, &DisplayTarget::Index(1), &displays).unwrap();
        assert_eq!(result.index, 1);

        let result = resolve_target_display(1, &DisplayTarget::Index(0), &displays).unwrap();
        assert_eq!(result.index, 0);

        let result = resolve_target_display(0, &DisplayTarget::Index(2), &displays).unwrap();
        assert_eq!(result.index, 2);
    }

    #[test]
    fn test_resolve_target_display_index_out_of_range() {
        let displays = test_displays();

        let result = resolve_target_display(0, &DisplayTarget::Index(3), &displays);
        assert!(result.is_err());

        let result = resolve_target_display(0, &DisplayTarget::Index(100), &displays);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_target_display_empty_displays() {
        let displays: Vec<DisplayInfo> = vec![];

        let result = resolve_target_display(0, &DisplayTarget::Next, &displays);
        assert!(result.is_err());

        let result = resolve_target_display(0, &DisplayTarget::Prev, &displays);
        assert!(result.is_err());

        let result = resolve_target_display(0, &DisplayTarget::Index(0), &displays);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_target_display_single_display() {
        let displays = vec![DisplayInfo {
            index: 0,
            name: "Only Display".to_string(),
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
        }];

        // next wraps to same display
        let result = resolve_target_display(0, &DisplayTarget::Next, &displays).unwrap();
        assert_eq!(result.index, 0);

        // prev wraps to same display
        let result = resolve_target_display(0, &DisplayTarget::Prev, &displays).unwrap();
        assert_eq!(result.index, 0);
    }
}
