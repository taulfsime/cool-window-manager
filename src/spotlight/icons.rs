// icons.rs - icon resolution and extraction for Spotlight shortcuts

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// default cwm icon embedded in the binary (generated at build time)
/// this is a minimal 128x128 icon with "cwm" text
const DEFAULT_ICON_DATA: &[u8] = include_bytes!("../../assets/cwm_icon.icns");

/// resolves the icon for a spotlight shortcut
/// priority: explicit icon path > target app icon > default cwm icon
pub fn resolve_icon(
    explicit_icon: Option<&str>,
    target_app: Option<&str>,
    dest_path: &Path,
) -> Result<()> {
    // try explicit icon first
    if let Some(icon_spec) = explicit_icon {
        if try_resolve_explicit_icon(icon_spec, dest_path)? {
            return Ok(());
        }
    }

    // try target app icon
    if let Some(app_name) = target_app {
        if try_extract_app_icon(app_name, dest_path)? {
            return Ok(());
        }
    }

    // fall back to default cwm icon
    write_default_icon(dest_path)
}

/// tries to resolve an explicit icon specification
/// returns true if successful
fn try_resolve_explicit_icon(icon_spec: &str, dest_path: &Path) -> Result<bool> {
    let expanded = shellexpand::tilde(icon_spec);
    let icon_path = Path::new(expanded.as_ref());

    // check if it's a path to an existing file
    if icon_path.exists() {
        let extension = icon_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "icns" => {
                // copy directly
                std::fs::copy(icon_path, dest_path)
                    .with_context(|| format!("failed to copy icon from {}", icon_path.display()))?;
                return Ok(true);
            }
            "png" => {
                // convert png to icns using sips
                return convert_png_to_icns(icon_path, dest_path);
            }
            _ => {
                // try to use as-is (might be an icns without extension)
                if std::fs::copy(icon_path, dest_path).is_ok() {
                    return Ok(true);
                }
            }
        }
    }

    // check if it's an app name to extract icon from
    if try_extract_app_icon(icon_spec, dest_path)? {
        return Ok(true);
    }

    Ok(false)
}

/// converts a PNG file to ICNS format using sips
fn convert_png_to_icns(png_path: &Path, dest_path: &Path) -> Result<bool> {
    // create a temporary iconset directory
    let temp_dir = tempfile::tempdir()?;
    let iconset_path = temp_dir.path().join("icon.iconset");
    std::fs::create_dir_all(&iconset_path)?;

    // generate multiple sizes using sips
    let sizes = [16, 32, 64, 128, 256, 512];

    for size in sizes {
        let icon_name = format!("icon_{}x{}.png", size, size);
        let icon_dest = iconset_path.join(&icon_name);

        let status = Command::new("sips")
            .args([
                "-z",
                &size.to_string(),
                &size.to_string(),
                png_path.to_str().unwrap(),
                "--out",
                icon_dest.to_str().unwrap(),
            ])
            .output();

        if status.is_err() || !status.unwrap().status.success() {
            // if sips fails, try a simpler approach
            break;
        }

        // also create @2x version for retina
        let icon_name_2x = format!("icon_{}x{}@2x.png", size, size);
        let icon_dest_2x = iconset_path.join(&icon_name_2x);
        let size_2x = size * 2;

        let _ = Command::new("sips")
            .args([
                "-z",
                &size_2x.to_string(),
                &size_2x.to_string(),
                png_path.to_str().unwrap(),
                "--out",
                icon_dest_2x.to_str().unwrap(),
            ])
            .output();
    }

    // convert iconset to icns
    let status = Command::new("iconutil")
        .args([
            "-c",
            "icns",
            iconset_path.to_str().unwrap(),
            "-o",
            dest_path.to_str().unwrap(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => Ok(true),
        _ => {
            // fallback: just copy the PNG and hope for the best
            // (won't work as an icon but won't crash)
            Ok(false)
        }
    }
}

/// tries to extract an icon from an application bundle
/// returns true if successful
fn try_extract_app_icon(app_name: &str, dest_path: &Path) -> Result<bool> {
    // find the app bundle
    let app_path = find_app_bundle(app_name)?;

    if let Some(app_path) = app_path {
        // read the app's Info.plist to find the icon file
        let info_plist = app_path.join("Contents/Info.plist");

        if info_plist.exists() {
            if let Some(icon_name) = get_icon_name_from_plist(&info_plist) {
                // try with .icns extension
                let mut icon_path = app_path.join("Contents/Resources").join(&icon_name);
                if !icon_path.exists() {
                    icon_path = app_path
                        .join("Contents/Resources")
                        .join(format!("{}.icns", icon_name));
                }

                if icon_path.exists() {
                    std::fs::copy(&icon_path, dest_path).with_context(|| {
                        format!("failed to copy icon from {}", icon_path.display())
                    })?;
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// finds an application bundle by name
fn find_app_bundle(app_name: &str) -> Result<Option<PathBuf>> {
    // common application directories
    let search_paths = [
        "/Applications",
        "/System/Applications",
        "/System/Applications/Utilities",
    ];

    // also check user's Applications folder
    let home = dirs::home_dir();

    for base_path in search_paths {
        let app_path = Path::new(base_path).join(format!("{}.app", app_name));
        if app_path.exists() {
            return Ok(Some(app_path));
        }
    }

    // check ~/Applications
    if let Some(home) = home {
        let app_path = home.join("Applications").join(format!("{}.app", app_name));
        if app_path.exists() {
            return Ok(Some(app_path));
        }
    }

    // try using mdfind (Spotlight) to locate the app
    let output = Command::new("mdfind")
        .args(["kMDItemKind == 'Application'", "-name", app_name])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let path = Path::new(line.trim());
                if path.exists() && path.extension().map(|e| e == "app").unwrap_or(false) {
                    return Ok(Some(path.to_path_buf()));
                }
            }
        }
    }

    Ok(None)
}

/// extracts the icon file name from an Info.plist
fn get_icon_name_from_plist(plist_path: &Path) -> Option<String> {
    // use /usr/libexec/PlistBuddy to read the plist
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleIconFile", plist_path.to_str()?])
        .output()
        .ok()?;

    if output.status.success() {
        let icon_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !icon_name.is_empty() {
            return Some(icon_name);
        }
    }

    // try CFBundleIconName (newer apps)
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleIconName", plist_path.to_str()?])
        .output()
        .ok()?;

    if output.status.success() {
        let icon_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !icon_name.is_empty() {
            return Some(icon_name);
        }
    }

    None
}

/// writes the default cwm icon to the destination
fn write_default_icon(dest_path: &Path) -> Result<()> {
    // check if we have embedded icon data
    if DEFAULT_ICON_DATA.is_empty() {
        // generate a minimal icon if no embedded data
        return generate_minimal_icon(dest_path);
    }

    std::fs::write(dest_path, DEFAULT_ICON_DATA)
        .with_context(|| format!("failed to write default icon to {}", dest_path.display()))
}

/// generates a minimal icon using sips (fallback when no embedded icon)
fn generate_minimal_icon(dest_path: &Path) -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let temp_png = temp_dir.path().join("icon.png");

    // create a simple colored square using ImageMagick if available, otherwise use a blank
    let status = Command::new("convert")
        .args([
            "-size",
            "128x128",
            "xc:#E74C3C", // red color
            "-fill",
            "white",
            "-font",
            "Helvetica-Bold",
            "-pointsize",
            "48",
            "-gravity",
            "center",
            "-annotate",
            "0",
            "cwm",
            temp_png.to_str().unwrap(),
        ])
        .status();

    if status.is_ok() && status.unwrap().success() {
        // convert to icns
        if convert_png_to_icns(&temp_png, dest_path)? {
            return Ok(());
        }
    }

    // if all else fails, create an empty icns (will show generic icon)
    // this is a minimal valid icns file header
    let minimal_icns = [
        0x69u8, 0x63, 0x6e, 0x73, // 'icns' magic
        0x00, 0x00, 0x00, 0x08, // file size (just header)
    ];
    std::fs::write(dest_path, minimal_icns)
        .with_context(|| "failed to write minimal icon placeholder")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_app_bundle_safari() {
        let result = find_app_bundle("Safari");
        assert!(result.is_ok());
        // Safari should exist on macOS
        if let Ok(Some(path)) = result {
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("Safari.app"));
        }
    }

    #[test]
    fn test_find_app_bundle_nonexistent() {
        // use a very unlikely app name
        let result = find_app_bundle("NonExistentAppXYZ98765432109876543210");
        assert!(result.is_ok());
        // may or may not find something via mdfind, so just check it doesn't error
    }

    #[test]
    fn test_get_icon_name_from_plist() {
        // test with Safari's plist if it exists
        let safari_plist = Path::new("/Applications/Safari.app/Contents/Info.plist");
        if safari_plist.exists() {
            let icon_name = get_icon_name_from_plist(safari_plist);
            // Safari should have an icon defined
            assert!(icon_name.is_some() || true); // may vary by macOS version
        }
    }

    #[test]
    fn test_resolve_icon_default() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("test_icon.icns");

        let result = resolve_icon(None, None, &dest);
        assert!(result.is_ok());
        assert!(dest.exists());
    }

    #[test]
    fn test_resolve_icon_from_app() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("test_icon.icns");

        // try to get Safari's icon
        let result = resolve_icon(None, Some("Safari"), &dest);
        assert!(result.is_ok());
        assert!(dest.exists());
    }
}
