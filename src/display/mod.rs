use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub id: u32,
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
                id,
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
