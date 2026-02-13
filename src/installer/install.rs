use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::installer::{MAN_DIR, MAN_PAGE};
use crate::version::{Version, VersionInfo};

pub fn install_binary(target_dir: &Path, force: bool, use_sudo: bool) -> Result<()> {
    // get current executable
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    // ensure target directory exists
    if !target_dir.exists() {
        if use_sudo {
            Command::new("sudo")
                .args(["mkdir", "-p"])
                .arg(target_dir)
                .status()
                .context("Failed to create directory with sudo")?;
        } else {
            fs::create_dir_all(target_dir)
                .with_context(|| format!("Failed to create directory: {}", target_dir.display()))?;
        }
    }

    let target_path = target_dir.join("cwm");

    // check if already exists
    if target_path.exists() && !force {
        return Err(anyhow!(
            "cwm is already installed at {}. Use --force to overwrite.",
            target_path.display()
        ));
    }

    // copy binary
    if use_sudo {
        // use sudo cp
        let status = Command::new("sudo")
            .args(["cp", "-f"])
            .arg(&current_exe)
            .arg(&target_path)
            .status()
            .context("Failed to copy binary with sudo")?;

        if !status.success() {
            return Err(anyhow!("Failed to install with sudo"));
        }

        // set permissions with sudo
        Command::new("sudo")
            .args(["chmod", "755"])
            .arg(&target_path)
            .status()
            .context("Failed to set permissions with sudo")?;
    } else {
        // regular copy
        fs::copy(&current_exe, &target_path)
            .with_context(|| format!("Failed to copy binary to {}", target_path.display()))?;

        // set executable permissions
        let mut perms = fs::metadata(&target_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms).context("Failed to set executable permissions")?;
    }

    // save version info
    let version_info = VersionInfo {
        current: Version::current().full_version_string(),
        previous: None,
        last_seen_available: None,
        install_date: chrono::Utc::now(),
        install_path: target_path.clone(),
        schema_version: None,
    };
    version_info.save()?;

    println!("✓ Installed cwm to {}", target_path.display());

    // install man page
    install_man_page(use_sudo)?;

    // check if in PATH
    if !crate::installer::paths::is_in_path(target_dir) {
        println!("\n⚠️  {} is not in your PATH", target_dir.display());
        println!(
            "\n{}",
            crate::installer::paths::get_path_instructions(target_dir)
        );
    }

    Ok(())
}

pub fn uninstall_binary(target_dir: Option<&Path>) -> Result<()> {
    let paths = if let Some(dir) = target_dir {
        vec![dir.join("cwm")]
    } else {
        // check common locations
        let mut paths = vec![];
        let locations = [
            "~/.local/bin/cwm",
            "~/.cargo/bin/cwm",
            "/usr/local/bin/cwm",
            "/opt/homebrew/bin/cwm",
        ];

        for loc in &locations {
            let expanded = shellexpand::tilde(loc);
            let path = PathBuf::from(expanded.as_ref());
            if path.exists() {
                paths.push(path);
            }
        }
        paths
    };

    if paths.is_empty() {
        return Err(anyhow!("cwm not found in any standard location"));
    }

    for path in &paths {
        println!("Found cwm at: {}", path.display());
    }

    if paths.len() > 1 {
        println!(
            "\nMultiple installations found. Please specify --path to choose which to uninstall."
        );
        return Ok(());
    }

    let path = &paths[0];

    // check if we need sudo
    let needs_sudo = !check_removable(path);

    if needs_sudo {
        println!("Removing {} (requires sudo)...", path.display());
        let status = Command::new("sudo")
            .args(["rm", "-f"])
            .arg(path)
            .status()
            .context("Failed to remove with sudo")?;

        if !status.success() {
            return Err(anyhow!("Failed to uninstall with sudo"));
        }
    } else {
        fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))?;
    }

    println!("✓ Uninstalled cwm from {}", path.display());

    // remove man page (warn on failure, don't abort)
    uninstall_man_page()?;

    // optionally remove version info
    let version_path = dirs::home_dir()
        .map(|h| h.join(".cwm").join("version.json"))
        .unwrap_or_default();

    if version_path.exists() {
        print!(
            "\nRemove version info at {}? [y/N]: ",
            version_path.display()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() == "y" {
            fs::remove_file(&version_path)?;
            println!("✓ Removed version info");

            // remove .cwm directory if empty
            if let Some(parent) = version_path.parent() {
                if fs::read_dir(parent)?.next().is_none() {
                    fs::remove_dir(parent)?;
                    println!("✓ Removed empty .cwm directory");
                }
            }
        }
    }

    Ok(())
}

fn check_removable(path: &Path) -> bool {
    // try to write to parent directory
    if let Some(parent) = path.parent() {
        let test_file = parent.join(".cwm_remove_test");
        match fs::write(&test_file, "test") {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
                true
            }
            Err(_) => false,
        }
    } else {
        false
    }
}

/// install man page to system man directory
/// returns Ok(true) if installed, Ok(false) if skipped by user
pub fn install_man_page(use_sudo: bool) -> Result<bool> {
    let man_dir = Path::new(MAN_DIR);
    let man_path = man_dir.join("cwm.1");

    println!("Installing man page to {}...", man_path.display());

    // ensure directory exists
    if !man_dir.exists() {
        let result = if use_sudo {
            Command::new("sudo")
                .args(["mkdir", "-p"])
                .arg(man_dir)
                .status()
        } else {
            fs::create_dir_all(man_dir).map(|_| std::process::ExitStatus::default())
        };

        if let Err(e) = result {
            return handle_man_page_error(&format!("Failed to create directory: {}", e));
        }
    }

    // write man page
    let write_result = if use_sudo {
        write_man_page_with_sudo(&man_path)
    } else {
        fs::write(&man_path, MAN_PAGE).map_err(|e| anyhow!(e))
    };

    match write_result {
        Ok(_) => {
            println!("✓ Installed man page to {}", man_path.display());
            Ok(true)
        }
        Err(e) => handle_man_page_error(&e.to_string()),
    }
}

fn write_man_page_with_sudo(path: &Path) -> Result<()> {
    // write to temp file first, then sudo mv
    let temp_path = std::env::temp_dir().join("cwm.1.tmp");
    fs::write(&temp_path, MAN_PAGE)?;

    let status = Command::new("sudo")
        .args(["mv", "-f"])
        .arg(&temp_path)
        .arg(path)
        .status()
        .context("Failed to move man page with sudo")?;

    if !status.success() {
        return Err(anyhow!("Failed to install man page with sudo"));
    }

    // set permissions
    Command::new("sudo")
        .args(["chmod", "644"])
        .arg(path)
        .status()
        .context("Failed to set man page permissions")?;

    Ok(())
}

fn handle_man_page_error(error: &str) -> Result<bool> {
    eprintln!();
    eprintln!("⚠️  Failed to install man page: {}", error);
    eprintln!("   Location: {}/cwm.1", MAN_DIR);
    eprintln!();

    print!("Continue without man page? [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    if input.is_empty() || input == "y" || input == "yes" {
        println!("Skipping man page installation");
        Ok(false)
    } else {
        Err(anyhow!("Installation cancelled by user"))
    }
}

/// remove man page from system man directory
/// returns Ok(true) if removed, Ok(false) if not found or skipped
pub fn uninstall_man_page() -> Result<bool> {
    let man_path = Path::new(MAN_DIR).join("cwm.1");

    if !man_path.exists() {
        return Ok(false);
    }

    println!("Removing man page from {}...", man_path.display());

    let needs_sudo = !check_removable(&man_path);

    let result = if needs_sudo {
        Command::new("sudo")
            .args(["rm", "-f"])
            .arg(&man_path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        fs::remove_file(&man_path).is_ok()
    };

    if result {
        println!("✓ Removed man page");
        Ok(true)
    } else {
        eprintln!("⚠️  Failed to remove man page at {}", man_path.display());
        eprintln!("   You may need to remove it manually");
        Ok(false)
    }
}
