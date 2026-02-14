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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_plist_basic() {
        let plist = generate_plist("/usr/local/bin/cwm", None);

        // should contain XML declaration
        assert!(plist.contains("<?xml version=\"1.0\""));
        // should contain plist DOCTYPE
        assert!(plist.contains("<!DOCTYPE plist"));
        // should contain the label
        assert!(plist.contains(LAUNCHD_LABEL));
        // should contain the binary path
        assert!(plist.contains("/usr/local/bin/cwm"));
        // should contain daemon and run-foreground arguments
        assert!(plist.contains("<string>daemon</string>"));
        assert!(plist.contains("<string>run-foreground</string>"));
        // should have RunAtLoad true
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
        // should have KeepAlive false
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("<false/>"));
    }

    #[test]
    fn test_generate_plist_with_log_path() {
        let plist = generate_plist("/usr/local/bin/cwm", Some("/var/log/cwm.log"));

        // should contain the log arguments
        assert!(plist.contains("<string>--log</string>"));
        assert!(plist.contains("<string>/var/log/cwm.log</string>"));
    }

    #[test]
    fn test_generate_plist_without_log_path() {
        let plist = generate_plist("/usr/local/bin/cwm", None);

        // should NOT contain log arguments
        assert!(!plist.contains("--log"));
    }

    #[test]
    fn test_generate_plist_valid_xml_structure() {
        let plist = generate_plist("/usr/local/bin/cwm", Some("/tmp/cwm.log"));

        // check basic XML structure
        assert!(plist.starts_with("<?xml"));
        assert!(plist.contains("<plist version=\"1.0\">"));
        assert!(plist.contains("</plist>"));
        assert!(plist.contains("<dict>"));
        assert!(plist.contains("</dict>"));
        assert!(plist.contains("<array>"));
        assert!(plist.contains("</array>"));
    }

    #[test]
    fn test_generate_plist_contains_label() {
        let plist = generate_plist("/path/to/cwm", None);

        // should contain the Label key and value
        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains(&format!("<string>{}</string>", LAUNCHD_LABEL)));
    }

    #[test]
    fn test_generate_plist_program_arguments_order() {
        let plist = generate_plist("/usr/local/bin/cwm", Some("/tmp/log.txt"));

        // verify the order of arguments in ProgramArguments
        let binary_pos = plist.find("/usr/local/bin/cwm").unwrap();
        let daemon_pos = plist.find("<string>daemon</string>").unwrap();
        let foreground_pos = plist.find("<string>run-foreground</string>").unwrap();
        let log_flag_pos = plist.find("<string>--log</string>").unwrap();
        let log_path_pos = plist.find("<string>/tmp/log.txt</string>").unwrap();

        // binary should come first, then daemon, then run-foreground, then --log, then log path
        assert!(binary_pos < daemon_pos);
        assert!(daemon_pos < foreground_pos);
        assert!(foreground_pos < log_flag_pos);
        assert!(log_flag_pos < log_path_pos);
    }

    #[test]
    fn test_launchd_label_constant() {
        assert_eq!(LAUNCHD_LABEL, "com.cwm.daemon");
    }

    #[test]
    fn test_generate_plist_special_characters_in_path() {
        let plist = generate_plist("/Users/test user/bin/cwm", None);

        // should contain the path with space
        assert!(plist.contains("/Users/test user/bin/cwm"));
    }

    #[test]
    fn test_generate_plist_log_path_with_spaces() {
        let plist = generate_plist(
            "/usr/local/bin/cwm",
            Some("/Users/test/Library/Logs/cwm log.txt"),
        );

        assert!(plist.contains("/Users/test/Library/Logs/cwm log.txt"));
    }

    #[test]
    fn test_generate_plist_absolute_paths() {
        let plist = generate_plist("/absolute/path/to/cwm", Some("/var/log/cwm.log"));

        assert!(plist.contains("/absolute/path/to/cwm"));
        assert!(plist.contains("/var/log/cwm.log"));
    }

    #[test]
    fn test_generate_plist_no_keep_alive() {
        let plist = generate_plist("/usr/local/bin/cwm", None);

        // KeepAlive should be false - daemon manages its own lifecycle
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("<false/>"));
        // should NOT have KeepAlive true
        let keep_alive_pos = plist.find("<key>KeepAlive</key>").unwrap();
        let after_keep_alive = &plist[keep_alive_pos..];
        assert!(after_keep_alive.contains("<false/>"));
    }

    #[test]
    fn test_generate_plist_run_at_load() {
        let plist = generate_plist("/usr/local/bin/cwm", None);

        // RunAtLoad should be true
        assert!(plist.contains("<key>RunAtLoad</key>"));
        let run_at_load_pos = plist.find("<key>RunAtLoad</key>").unwrap();
        let after_run_at_load = &plist[run_at_load_pos..];
        assert!(after_run_at_load.contains("<true/>"));
    }

    #[test]
    fn test_get_plist_path() {
        let path = get_plist_path().unwrap();
        let path_str = path.to_string_lossy();

        // should be in LaunchAgents
        assert!(path_str.contains("LaunchAgents"));
        // should have the correct filename
        assert!(path_str.ends_with("com.cwm.daemon.plist"));
    }

    #[test]
    fn test_generate_plist_daemon_argument() {
        let plist = generate_plist("/usr/local/bin/cwm", None);

        // should have daemon as first argument
        assert!(plist.contains("<string>daemon</string>"));
    }

    #[test]
    fn test_generate_plist_run_foreground_argument() {
        let plist = generate_plist("/usr/local/bin/cwm", None);

        // should have run-foreground as second argument
        assert!(plist.contains("<string>run-foreground</string>"));
    }
}
