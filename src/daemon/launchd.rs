use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const LAUNCHD_LABEL: &str = "com.cwm.daemon";

fn get_plist_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{}.plist", LAUNCHD_LABEL)))
}

fn generate_plist(bin_path: &str, log_path: Option<&str>) -> String {
    let mut args = format!(
        r#"    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>daemon</string>
        <string>run-foreground</string>"#,
        bin_path
    );

    if let Some(log) = log_path {
        args.push_str(&format!(
            r#"
        <string>--log</string>
        <string>{}</string>"#,
            log
        ));
    }

    args.push_str("\n    </array>");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
{}
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        LAUNCHD_LABEL, args
    )
}

fn is_loaded() -> bool {
    let output = Command::new("launchctl")
        .args(["list", LAUNCHD_LABEL])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

fn launchctl_load(plist_path: &PathBuf) -> Result<()> {
    let status = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(plist_path)
        .status()
        .context("Failed to run launchctl load")?;

    if !status.success() {
        return Err(anyhow!("launchctl load failed"));
    }
    Ok(())
}

fn launchctl_unload(plist_path: &PathBuf) -> Result<()> {
    let status = Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(plist_path)
        .status()
        .context("Failed to run launchctl unload")?;

    if !status.success() {
        return Err(anyhow!("launchctl unload failed"));
    }
    Ok(())
}

/// Install the daemon to run on login via launchd
pub fn install(bin_path: Option<String>, log_path: Option<String>) -> Result<()> {
    let plist_path = get_plist_path()?;

    // resolve binary path
    let bin = if let Some(path) = bin_path {
        PathBuf::from(path)
    } else {
        std::env::current_exe().context("Failed to get current executable path")?
    };

    if !bin.exists() {
        return Err(anyhow!("Binary not found: {}", bin.display()));
    }

    let bin_str = bin
        .to_str()
        .ok_or_else(|| anyhow!("Binary path contains invalid UTF-8"))?;

    // unload existing if loaded
    if is_loaded() {
        println!("Unloading existing daemon...");
        let _ = launchctl_unload(&plist_path);
    }

    // ensure LaunchAgents directory exists
    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent).context("Failed to create LaunchAgents directory")?;
    }

    // generate and write plist
    let plist_content = generate_plist(bin_str, log_path.as_deref());
    fs::write(&plist_path, &plist_content)
        .with_context(|| format!("Failed to write plist to {}", plist_path.display()))?;

    println!("Created: {}", plist_path.display());

    // load the agent
    launchctl_load(&plist_path)?;

    println!("Daemon installed and loaded");
    println!("  Binary: {}", bin.display());
    if let Some(log) = log_path {
        println!("  Log: {}", log);
    }
    println!("\nThe daemon will start automatically on login");
    println!("Use 'cwm daemon status' to check if it's running");

    Ok(())
}

/// Uninstall the daemon from login items
pub fn uninstall() -> Result<()> {
    let plist_path = get_plist_path()?;

    if !plist_path.exists() {
        println!("Daemon is not installed");
        return Ok(());
    }

    // unload if loaded
    if is_loaded() {
        println!("Unloading daemon...");
        launchctl_unload(&plist_path)?;
    }

    // remove plist file
    fs::remove_file(&plist_path)
        .with_context(|| format!("Failed to remove {}", plist_path.display()))?;

    println!("Daemon uninstalled");
    println!("Removed: {}", plist_path.display());

    Ok(())
}
