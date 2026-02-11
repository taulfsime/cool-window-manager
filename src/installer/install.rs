use anyhow::{anyhow, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    };
    version_info.save()?;

    println!("✓ Installed cwm to {}", target_path.display());

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

    // optionally remove version info
    let version_path = dirs::home_dir()
        .map(|h| h.join(".cwm").join("version.json"))
        .unwrap_or_default();

    if version_path.exists() {
        println!(
            "\nRemove version info at {}? [y/N]: ",
            version_path.display()
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
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
