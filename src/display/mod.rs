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
}

impl DisplayInfo {
    pub fn describe(&self) -> String {
        let main_marker = if self.is_main { " (main)" } else { "" };
        format!(
            "Display {}: {} - {}x{} at ({}, {}){}",
            self.index, self.name, self.width, self.height, self.x, self.y, main_marker
        )
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

            DisplayInfo {
                index,
                name: format!("Display {}", id),
                width: bounds.size.width as u32,
                height: bounds.size.height as u32,
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                is_main: id == main_display_id,
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

pub fn print_displays() -> Result<()> {
    let displays = get_displays()?;

    if displays.is_empty() {
        println!("No displays found");
        return Ok(());
    }

    println!("Available displays:");
    for display in &displays {
        println!("  {}", display.describe());
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
            },
            DisplayInfo {
                index: 1,
                name: "External Monitor".to_string(),
                width: 3840,
                height: 2160,
                x: 2560,
                y: 0,
                is_main: false,
            },
            DisplayInfo {
                index: 2,
                name: "Third Display".to_string(),
                width: 1920,
                height: 1080,
                x: 6400,
                y: 0,
                is_main: false,
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
        };
        assert_eq!(
            display.describe(),
            "Display 0: Main Display - 2560x1440 at (0, 0) (main)"
        );
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
        }];

        // next wraps to same display
        let result = resolve_target_display(0, &DisplayTarget::Next, &displays).unwrap();
        assert_eq!(result.index, 0);

        // prev wraps to same display
        let result = resolve_target_display(0, &DisplayTarget::Prev, &displays).unwrap();
        assert_eq!(result.index, 0);
    }
}
