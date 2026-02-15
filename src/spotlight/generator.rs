use anyhow::{anyhow, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::config::SpotlightShortcut;

use super::icons::resolve_icon;
use super::signing::sign_app_bundle;
use super::{default_apps_directory, BUNDLE_ID_PREFIX, SHORTCUT_PREFIX};

/// embedded stub executable (compiled at build time)
const STUB_EXECUTABLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/spotlight_stub"));

/// returns the directory where spotlight apps are installed
pub fn get_apps_directory() -> PathBuf {
    default_apps_directory()
}

/// generates an app bundle for a spotlight shortcut
pub fn generate_app_bundle(shortcut: &SpotlightShortcut, apps_dir: &Path) -> Result<PathBuf> {
    let app_name = format!("{}{}.app", SHORTCUT_PREFIX, shortcut.name);
    let app_path = apps_dir.join(&app_name);

    // create app bundle structure
    let contents_dir = app_path.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let resources_dir = contents_dir.join("Resources");

    fs::create_dir_all(&macos_dir).with_context(|| {
        format!(
            "failed to create app bundle directory: {}",
            macos_dir.display()
        )
    })?;

    fs::create_dir_all(&resources_dir).with_context(|| {
        format!(
            "failed to create resources directory: {}",
            resources_dir.display()
        )
    })?;

    // write Info.plist with enhanced keys
    let info_plist = generate_info_plist(shortcut);
    let plist_path = contents_dir.join("Info.plist");
    fs::write(&plist_path, info_plist)
        .with_context(|| format!("failed to write Info.plist: {}", plist_path.display()))?;

    // write PkgInfo file
    let pkginfo_path = contents_dir.join("PkgInfo");
    fs::write(&pkginfo_path, "APPL????")
        .with_context(|| format!("failed to write PkgInfo: {}", pkginfo_path.display()))?;

    // write executable (use compiled stub if available, otherwise shell script)
    let exec_path = macos_dir.join("run");
    if !STUB_EXECUTABLE.is_empty() {
        // use compiled stub
        fs::write(&exec_path, STUB_EXECUTABLE)
            .with_context(|| format!("failed to write executable: {}", exec_path.display()))?;

        // write command file for the stub
        let ipc_command = build_ipc_command(shortcut);
        let cmd_path = macos_dir.join("cwm_command.txt");
        fs::write(&cmd_path, &ipc_command)
            .with_context(|| format!("failed to write command file: {}", cmd_path.display()))?;
    } else {
        // fallback to shell script
        let script = generate_shell_script(shortcut);
        fs::write(&exec_path, script)
            .with_context(|| format!("failed to write script: {}", exec_path.display()))?;
    }

    // make executable
    let mut perms = fs::metadata(&exec_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&exec_path, perms)?;

    // resolve and write icon
    let icon_path = resources_dir.join("AppIcon.icns");
    if let Err(e) = resolve_icon(
        shortcut.icon.as_deref(),
        shortcut.app.as_deref(),
        &icon_path,
    ) {
        eprintln!("Warning: failed to set icon for '{}': {}", shortcut.name, e);
        // continue without icon - not fatal
    }

    Ok(app_path)
}

/// generates the Info.plist content for an app bundle with enhanced keys
fn generate_info_plist(shortcut: &SpotlightShortcut) -> String {
    let bundle_name = shortcut.display_name();
    let bundle_id = format!("{}.{}", BUNDLE_ID_PREFIX, shortcut.identifier());

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>run</string>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundleDisplayName</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>{}</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>LSUIElement</key>
    <true/>
    <key>LSBackgroundOnly</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSAppleEventsUsageDescription</key>
    <string>cwm uses AppleScript to display notifications</string>
</dict>
</plist>
"#,
        bundle_name, bundle_name, bundle_id
    )
}

/// generates the shell script content for an app bundle (fallback when stub not available)
fn generate_shell_script(shortcut: &SpotlightShortcut) -> String {
    // build the action string for the daemon IPC
    // format: action[:arg] (same as daemon's execute_action expects)
    let ipc_command = build_ipc_command(shortcut);

    let error_title = format!("{} Error", shortcut.display_name());

    format!(
        r#"#!/bin/bash
# {}
# generated by cwm spotlight

SOCKET="$HOME/.cwm/cwm.sock"
COMMAND="{}"

# try to send command via socket to daemon (preferred - no permission issues)
if [ -S "$SOCKET" ]; then
    RESPONSE=$(echo "$COMMAND" | nc -U "$SOCKET" -w 2 2>/dev/null)
    if [ $? -eq 0 ]; then
        if echo "$RESPONSE" | grep -q "\"error\""; then
            ERROR_MSG=$(echo "$RESPONSE" | grep -o '"message":"[^"]*"' | cut -d'"' -f4)
            osascript -e "display notification \"$ERROR_MSG\" with title \"{}\""
            exit 1
        fi
        # got a response, assume success
        exit 0
    fi
fi

# daemon not running - show helpful message
osascript -e 'display dialog "cwm daemon is not running.\n\nStart it with:\n  cwm daemon start\n\nOr enable auto-start:\n  cwm daemon install" buttons {{"OK"}} default button "OK" with title "{}"'
exit 1
"#,
        shortcut.display_name(),
        ipc_command,
        error_title,
        error_title
    )
}

/// builds the IPC command string for the daemon
fn build_ipc_command(shortcut: &SpotlightShortcut) -> String {
    // the daemon expects actions in format: action[:arg1[:arg2]]
    // e.g., "focus:Safari", "maximize", "move:next", "resize:80"

    let (action_type, action_arg) = if let Some(idx) = shortcut.action.find(':') {
        (&shortcut.action[..idx], Some(&shortcut.action[idx + 1..]))
    } else {
        (shortcut.action.as_str(), None)
    };

    match action_type {
        "focus" => {
            if let Some(ref app) = shortcut.app {
                format!("focus:{}", app)
            } else {
                "focus".to_string()
            }
        }
        "maximize" => {
            if let Some(ref app) = shortcut.app {
                format!("maximize:{}", app)
            } else {
                "maximize".to_string()
            }
        }
        "move" => {
            let target = action_arg.unwrap_or("next");
            if let Some(ref app) = shortcut.app {
                format!("move:{}:{}", target, app)
            } else {
                format!("move:{}", target)
            }
        }
        "resize" => {
            let size = action_arg.unwrap_or("80");
            if let Some(ref app) = shortcut.app {
                format!("resize:{}:{}", size, app)
            } else {
                format!("resize:{}", size)
            }
        }
        _ => shortcut.action.clone(),
    }
}

/// installs a single spotlight shortcut
pub fn install_shortcut(shortcut: &SpotlightShortcut, force: bool) -> Result<PathBuf> {
    let apps_dir = get_apps_directory();

    // ensure apps directory exists
    fs::create_dir_all(&apps_dir)
        .with_context(|| format!("failed to create apps directory: {}", apps_dir.display()))?;

    let app_name = format!("{}{}.app", SHORTCUT_PREFIX, shortcut.name);
    let app_path = apps_dir.join(&app_name);

    // check if already exists
    if app_path.exists() {
        if force {
            fs::remove_dir_all(&app_path).with_context(|| {
                format!("failed to remove existing app: {}", app_path.display())
            })?;
        } else {
            return Err(anyhow!(
                "App '{}' already exists. Use --force to overwrite.",
                app_name
            ));
        }
    }

    // generate the app bundle
    let app_path = generate_app_bundle(shortcut, &apps_dir)?;

    // sign the app bundle (best effort)
    match sign_app_bundle(&app_path) {
        Ok(result) => {
            if !result.is_success() && !result.is_skipped() {
                eprintln!(
                    "Warning: code signing for '{}': {}",
                    shortcut.name,
                    result.description()
                );
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: code signing failed for '{}': {}",
                shortcut.name, e
            );
        }
    }

    Ok(app_path)
}

/// installs all spotlight shortcuts from config
pub fn install_all(shortcuts: &[SpotlightShortcut], force: bool) -> Result<Vec<PathBuf>> {
    let mut installed = Vec::new();

    for shortcut in shortcuts {
        match install_shortcut(shortcut, force) {
            Ok(path) => {
                installed.push(path);
            }
            Err(e) => {
                // continue with other shortcuts but report error
                eprintln!("Warning: failed to install '{}': {}", shortcut.name, e);
            }
        }
    }

    // trigger Spotlight reindex
    reindex_spotlight()?;

    Ok(installed)
}

/// removes a single spotlight shortcut by name
pub fn remove_shortcut(name: &str) -> Result<()> {
    let apps_dir = get_apps_directory();
    let app_name = format!("{}{}.app", SHORTCUT_PREFIX, name);
    let app_path = apps_dir.join(&app_name);

    if !app_path.exists() {
        return Err(anyhow!("Shortcut '{}' not found", name));
    }

    fs::remove_dir_all(&app_path)
        .with_context(|| format!("failed to remove app: {}", app_path.display()))?;

    // trigger Spotlight reindex
    reindex_spotlight()?;

    Ok(())
}

/// removes all cwm spotlight shortcuts
pub fn remove_all() -> Result<usize> {
    let apps_dir = get_apps_directory();

    if !apps_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;

    for entry in fs::read_dir(&apps_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(SHORTCUT_PREFIX) && name.ends_with(".app") {
                    fs::remove_dir_all(&path)?;
                    count += 1;
                }
            }
        }
    }

    // remove the cwm apps directory if empty
    if fs::read_dir(&apps_dir)?.next().is_none() {
        fs::remove_dir(&apps_dir).ok();
    }

    // trigger Spotlight reindex
    reindex_spotlight()?;

    Ok(count)
}

/// returns list of installed cwm spotlight shortcuts
pub fn get_installed_shortcuts() -> Result<Vec<String>> {
    let apps_dir = get_apps_directory();

    if !apps_dir.exists() {
        return Ok(Vec::new());
    }

    let mut shortcuts = Vec::new();

    for entry in fs::read_dir(&apps_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(SHORTCUT_PREFIX) && name.ends_with(".app") {
                    // extract the shortcut name without prefix and .app suffix
                    let shortcut_name = name
                        .strip_prefix(SHORTCUT_PREFIX)
                        .and_then(|n| n.strip_suffix(".app"))
                        .unwrap_or(name);
                    shortcuts.push(shortcut_name.to_string());
                }
            }
        }
    }

    shortcuts.sort();
    Ok(shortcuts)
}

/// triggers Spotlight to reindex the apps directory
fn reindex_spotlight() -> Result<()> {
    let apps_dir = get_apps_directory();

    if !apps_dir.exists() {
        return Ok(());
    }

    // use mdimport to trigger reindex
    let status = std::process::Command::new("mdimport")
        .arg(&apps_dir)
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => {
            // mdimport failed but this is not critical
            eprintln!("Warning: Spotlight reindex may have failed. You may need to wait for automatic indexing.");
            Ok(())
        }
        Err(e) => {
            eprintln!("Warning: could not trigger Spotlight reindex: {}", e);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_info_plist() {
        let shortcut = SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
            icon: None,
        };

        let plist = generate_info_plist(&shortcut);

        assert!(plist.contains("cwm: Focus Safari"));
        assert!(plist.contains("com.cwm.spotlight.focus-safari"));
        assert!(plist.contains("<key>LSUIElement</key>"));
        assert!(plist.contains("<true/>"));
        // check for new keys
        assert!(plist.contains("<key>CFBundleInfoDictionaryVersion</key>"));
        assert!(plist.contains("<string>6.0</string>"));
        assert!(plist.contains("<key>CFBundleIconFile</key>"));
        assert!(plist.contains("<string>AppIcon</string>"));
        assert!(plist.contains("<key>NSHighResolutionCapable</key>"));
    }

    #[test]
    fn test_generate_shell_script() {
        let shortcut = SpotlightShortcut {
            name: "Focus Safari".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: Some(true),
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        assert!(script.contains("#!/bin/bash"));
        assert!(script.contains("cwm: Focus Safari"));
        // socket-based IPC command format
        assert!(script.contains("COMMAND=\"focus:Safari\""));
        assert!(script.contains("nc -U"));
        assert!(script.contains(".cwm/cwm.sock"));
        assert!(script.contains("osascript"));
    }

    #[test]
    fn test_generate_shell_script_with_spaces_in_app() {
        let shortcut = SpotlightShortcut {
            name: "Focus VS Code".to_string(),
            action: "focus".to_string(),
            app: Some("Visual Studio Code".to_string()),
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        // app name with spaces in IPC command
        assert!(script.contains("COMMAND=\"focus:Visual Studio Code\""));
    }

    #[test]
    fn test_generate_shell_script_move_display() {
        let shortcut = SpotlightShortcut {
            name: "Move Next".to_string(),
            action: "move_display:next".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        // socket-based IPC command format
        assert!(script.contains("COMMAND=\"move_display:next\""));
    }

    #[test]
    fn test_generate_shell_script_resize() {
        let shortcut = SpotlightShortcut {
            name: "Resize 80".to_string(),
            action: "resize:80".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        // socket-based IPC command format
        assert!(script.contains("COMMAND=\"resize:80\""));
    }

    #[test]
    fn test_build_ipc_command_focus() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "focus:Safari");
    }

    #[test]
    fn test_build_ipc_command_focus_no_app() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "focus".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "focus");
    }

    #[test]
    fn test_build_ipc_command_maximize() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "maximize".to_string(),
            app: Some("Terminal".to_string()),
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "maximize:Terminal");
    }

    #[test]
    fn test_build_ipc_command_move() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "move:next".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "move:next");
    }

    #[test]
    fn test_build_ipc_command_move_with_app() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "move:prev".to_string(),
            app: Some("Finder".to_string()),
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "move:prev:Finder");
    }

    #[test]
    fn test_build_ipc_command_resize() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "resize:75".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "resize:75");
    }

    #[test]
    fn test_build_ipc_command_resize_with_app() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "resize:50".to_string(),
            app: Some("Notes".to_string()),
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "resize:50:Notes");
    }

    // ========================================================================
    // generate_info_plist edge cases
    // ========================================================================

    #[test]
    fn test_generate_info_plist_special_characters_in_name() {
        let shortcut = SpotlightShortcut {
            name: "Focus \"Safari\" & Chrome".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
            icon: None,
        };

        let plist = generate_info_plist(&shortcut);

        // should contain the name (XML escaping handled by format!)
        assert!(plist.contains("cwm: Focus \"Safari\" & Chrome"));
        assert!(plist.contains("CFBundleExecutable"));
        assert!(plist.contains("<string>run</string>"));
    }

    #[test]
    fn test_generate_info_plist_maximize_action() {
        let shortcut = SpotlightShortcut {
            name: "Maximize Window".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let plist = generate_info_plist(&shortcut);

        assert!(plist.contains("cwm: Maximize Window"));
        assert!(plist.contains("com.cwm.spotlight.maximize-window"));
        assert!(plist.contains("<key>LSBackgroundOnly</key>"));
    }

    #[test]
    fn test_generate_info_plist_move_display_action() {
        let shortcut = SpotlightShortcut {
            name: "Move to Next".to_string(),
            action: "move_display:next".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let plist = generate_info_plist(&shortcut);

        assert!(plist.contains("cwm: Move to Next"));
        assert!(plist.contains("com.cwm.spotlight.move-to-next"));
    }

    #[test]
    fn test_generate_info_plist_resize_action() {
        let shortcut = SpotlightShortcut {
            name: "Resize 75%".to_string(),
            action: "resize:75".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let plist = generate_info_plist(&shortcut);

        assert!(plist.contains("cwm: Resize 75%"));
        // identifier converts % to empty
        assert!(plist.contains("com.cwm.spotlight.resize-75"));
    }

    #[test]
    fn test_generate_info_plist_unicode_name() {
        let shortcut = SpotlightShortcut {
            name: "Focus 日本語App".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
            icon: None,
        };

        let plist = generate_info_plist(&shortcut);

        assert!(plist.contains("cwm: Focus 日本語App"));
    }

    // ========================================================================
    // generate_shell_script edge cases
    // ========================================================================

    #[test]
    fn test_generate_shell_script_maximize_no_app() {
        let shortcut = SpotlightShortcut {
            name: "Maximize".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        assert!(script.contains("COMMAND=\"maximize\""));
        assert!(script.contains("#!/bin/bash"));
    }

    #[test]
    fn test_generate_shell_script_maximize_with_app() {
        let shortcut = SpotlightShortcut {
            name: "Maximize Terminal".to_string(),
            action: "maximize".to_string(),
            app: Some("Terminal".to_string()),
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        assert!(script.contains("COMMAND=\"maximize:Terminal\""));
    }

    #[test]
    fn test_generate_shell_script_move_prev() {
        let shortcut = SpotlightShortcut {
            name: "Move Prev".to_string(),
            action: "move:prev".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        assert!(script.contains("COMMAND=\"move:prev\""));
    }

    #[test]
    fn test_generate_shell_script_move_with_app() {
        let shortcut = SpotlightShortcut {
            name: "Move Safari Next".to_string(),
            action: "move:next".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        assert!(script.contains("COMMAND=\"move:next:Safari\""));
    }

    #[test]
    fn test_generate_shell_script_resize_default() {
        // resize without explicit size should default to 80
        let shortcut = SpotlightShortcut {
            name: "Resize Default".to_string(),
            action: "resize".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        // default size is 80
        assert!(script.contains("COMMAND=\"resize:80\""));
    }

    #[test]
    fn test_generate_shell_script_move_default() {
        // move without explicit target should default to next
        let shortcut = SpotlightShortcut {
            name: "Move Default".to_string(),
            action: "move".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        // default target is next
        assert!(script.contains("COMMAND=\"move:next\""));
    }

    #[test]
    fn test_generate_shell_script_unknown_action() {
        // unknown actions should pass through as-is
        let shortcut = SpotlightShortcut {
            name: "Custom Action".to_string(),
            action: "custom:arg1:arg2".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        assert!(script.contains("COMMAND=\"custom:arg1:arg2\""));
    }

    #[test]
    fn test_generate_shell_script_error_notification() {
        let shortcut = SpotlightShortcut {
            name: "Test Action".to_string(),
            action: "focus".to_string(),
            app: Some("Safari".to_string()),
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        // should contain error notification with proper title
        assert!(script.contains("cwm: Test Action Error"));
        assert!(script.contains("display notification"));
    }

    #[test]
    fn test_generate_shell_script_daemon_not_running_dialog() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
            icon: None,
        };

        let script = generate_shell_script(&shortcut);

        // should contain daemon not running dialog
        assert!(script.contains("cwm daemon is not running"));
        assert!(script.contains("cwm daemon start"));
        assert!(script.contains("cwm daemon install"));
    }

    // ========================================================================
    // build_ipc_command edge cases
    // ========================================================================

    #[test]
    fn test_build_ipc_command_maximize_no_app() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "maximize".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "maximize");
    }

    #[test]
    fn test_build_ipc_command_move_default_target() {
        // move without target should default to "next"
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "move".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "move:next");
    }

    #[test]
    fn test_build_ipc_command_resize_default_size() {
        // resize without size should default to "80"
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "resize".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "resize:80");
    }

    #[test]
    fn test_build_ipc_command_unknown_action_passthrough() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "unknown_action:with:args".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        // unknown actions pass through unchanged
        assert_eq!(build_ipc_command(&shortcut), "unknown_action:with:args");
    }

    #[test]
    fn test_build_ipc_command_focus_with_spaces() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "focus".to_string(),
            app: Some("Visual Studio Code".to_string()),
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "focus:Visual Studio Code");
    }

    #[test]
    fn test_build_ipc_command_move_numeric() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "move:2".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "move:2");
    }

    #[test]
    fn test_build_ipc_command_resize_full() {
        let shortcut = SpotlightShortcut {
            name: "Test".to_string(),
            action: "resize:full".to_string(),
            app: None,
            launch: None,
            icon: None,
        };
        assert_eq!(build_ipc_command(&shortcut), "resize:full");
    }

    // ========================================================================
    // get_apps_directory test
    // ========================================================================

    #[test]
    fn test_get_apps_directory() {
        let dir = get_apps_directory();
        // should be ~/Applications/cwm
        assert!(dir.to_string_lossy().contains("Applications"));
        assert!(dir.to_string_lossy().contains("cwm"));
    }
}
