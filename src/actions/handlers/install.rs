//! install/uninstall/update action handlers

use std::path::PathBuf;

use crate::actions::context::ExecutionContext;
use crate::actions::error::ActionError;
use crate::actions::result::ActionResult;
use crate::version::Version;

/// install - requires CLI (modifies system files, may require sudo, interactive path selection)
pub fn execute_install(
    path: Option<PathBuf>,
    force: bool,
    no_sudo: bool,
    completions: Option<String>,
    no_completions: bool,
    completions_only: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    use crate::installer::{detect_install_paths, install_binary, paths::check_writable};

    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "install is only available via CLI",
        ));
    }

    // handle completions-only mode
    if completions_only {
        let shell_arg = completions.as_deref().unwrap_or("auto");
        return install_completions_only(shell_arg);
    }

    let target_dir = if let Some(p) = path {
        p
    } else {
        // interactive path selection
        let paths = detect_install_paths();

        if paths.is_empty() {
            return Err(ActionError::general(
                "no suitable installation directories found",
            ));
        }

        println!("Where would you like to install cwm?\n");
        for (i, path_info) in paths.iter().enumerate() {
            println!("  {}. {}", i + 1, path_info.status_line());
        }
        println!("  {}. Custom path...", paths.len() + 1);

        print!("\nChoice [1]: ");
        use std::io::{self, Write};
        io::stdout()
            .flush()
            .map_err(|e| ActionError::general(e.to_string()))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| ActionError::general(e.to_string()))?;
        let choice = input.trim();

        let idx = if choice.is_empty() {
            0
        } else {
            choice
                .parse::<usize>()
                .map(|n| n.saturating_sub(1))
                .unwrap_or(0)
        };

        if idx < paths.len() {
            paths[idx].path.clone()
        } else {
            // custom path
            print!("Enter custom path: ");
            io::stdout()
                .flush()
                .map_err(|e| ActionError::general(e.to_string()))?;
            let mut custom = String::new();
            io::stdin()
                .read_line(&mut custom)
                .map_err(|e| ActionError::general(e.to_string()))?;
            PathBuf::from(shellexpand::tilde(custom.trim()).to_string())
        }
    };

    // check if we need sudo
    let needs_sudo = !no_sudo && !check_writable(&target_dir);

    install_binary(&target_dir, force, needs_sudo).map_err(ActionError::from)?;

    // handle shell completions
    let mut completion_paths: Vec<String> = Vec::new();
    if !no_completions {
        completion_paths = install_completions_interactive(completions.as_deref())?;
    }

    let mut result = serde_json::json!({
        "status": "installed",
        "path": target_dir.join("cwm").to_string_lossy(),
    });

    if !completion_paths.is_empty() {
        result["completions"] = serde_json::json!(completion_paths);
    }

    Ok(ActionResult::simple("install", result))
}

/// install completions only (skip binary installation)
fn install_completions_only(shell_arg: &str) -> Result<ActionResult, ActionError> {
    use crate::installer::completions as comp;

    println!("Installing shell completions...");

    match comp::install_completions_for_arg(shell_arg) {
        Ok(installed) => {
            let mut paths = Vec::new();
            for (shell, path) in &installed {
                println!("  ✓ {}: {}", shell.name(), path.display());
                paths.push(path.to_string_lossy().to_string());
            }

            // print enable instructions for first shell
            if let Some((shell, path)) = installed.first() {
                comp::print_enable_instructions(*shell, path);
            }

            Ok(ActionResult::simple(
                "install",
                serde_json::json!({
                    "status": "completions_installed",
                    "completions": paths,
                }),
            ))
        }
        Err(e) => Err(ActionError::general(format!(
            "failed to install completions: {}",
            e
        ))),
    }
}

/// install completions interactively based on argument
fn install_completions_interactive(
    completions_arg: Option<&str>,
) -> Result<Vec<String>, ActionError> {
    use crate::installer::completions as comp;

    let shells_to_install = match completions_arg {
        None => {
            // prompt user
            match comp::prompt_shell_selection() {
                Ok(Some(shells)) => shells,
                Ok(None) => return Ok(Vec::new()), // user declined
                Err(e) => {
                    eprintln!("⚠️  Failed to prompt for completions: {}", e);
                    return Ok(Vec::new());
                }
            }
        }
        Some("auto") => {
            // detect shell
            match comp::detect_shell() {
                Some(shell) => vec![shell],
                None => {
                    println!("Could not detect shell. Skipping completions.");
                    println!("Run 'cwm install --completions=<shell>' to install manually.");
                    return Ok(Vec::new());
                }
            }
        }
        Some("all") => comp::CompletionShell::all().to_vec(),
        Some(shell_name) => match shell_name.parse() {
            Ok(shell) => vec![shell],
            Err(_) => {
                eprintln!("Unknown shell: {}. Skipping completions.", shell_name);
                return Ok(Vec::new());
            }
        },
    };

    if shells_to_install.is_empty() {
        return Ok(Vec::new());
    }

    println!();
    println!("Installing shell completions...");

    let mut installed_paths = Vec::new();
    for shell in &shells_to_install {
        match comp::install_for_shell(*shell) {
            Ok(path) => {
                println!("  ✓ {}: {}", shell.name(), path.display());
                installed_paths.push(path.to_string_lossy().to_string());
            }
            Err(e) => {
                eprintln!("  ✗ {}: {}", shell.name(), e);
            }
        }
    }

    // print enable instructions for detected/current shell
    if let Some(current) = comp::detect_shell() {
        if shells_to_install.contains(&current) {
            comp::print_enable_instructions(current, &current.user_path());
        }
    } else if let Some(first) = shells_to_install.first() {
        comp::print_enable_instructions(*first, &first.user_path());
    }

    Ok(installed_paths)
}

/// uninstall - requires CLI (modifies system files, may require sudo)
pub fn execute_uninstall(
    path: Option<PathBuf>,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    use crate::installer::uninstall_binary;

    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "uninstall is only available via CLI",
        ));
    }

    uninstall_binary(path.as_deref()).map_err(ActionError::from)?;

    Ok(ActionResult::simple(
        "uninstall",
        serde_json::json!({"status": "uninstalled"}),
    ))
}

/// update check - can be done via IPC (read-only, but network access needed)
pub fn execute_update_check(
    prerelease: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    use crate::installer::check_for_updates_with_settings;

    let current = Version::current();

    // for IPC, just return current version without network check
    if !ctx.is_cli {
        return Ok(ActionResult::simple(
            "update_check",
            serde_json::json!({
                "semver": current.semver,
                "current_version": current.full_version_string(),
                "channel": current.channel,
                "commit": current.short_commit,
            }),
        ));
    }

    // apply prerelease flag to settings
    let mut settings = ctx.config.settings.update.clone();
    if prerelease {
        settings.channels.beta = true;
        settings.channels.dev = true;
    }

    // CLI check - verbose but non-interactive (no prompts for --check)
    match check_for_updates_with_settings(&settings, true, false) {
        Ok(Some(release)) => Ok(ActionResult::simple(
            "update_check",
            serde_json::json!({
                "current_version": current.full_version_string(),
                "update_available": true,
                "new_version": release.version,
                "channel": release.channel,
                "download_url": release.download_url,
                "size": release.size,
                "release_notes": release.release_notes,
            }),
        )),
        Ok(None) => Ok(ActionResult::simple(
            "update_check",
            serde_json::json!({
                "current_version": current.full_version_string(),
                "update_available": false,
            }),
        )),
        Err(e) => Err(ActionError::general(format!(
            "failed to check for updates: {}",
            e
        ))),
    }
}

/// update - requires CLI (downloads and replaces binary)
pub fn execute_update(
    force: bool,
    prerelease: bool,
    ctx: &ExecutionContext,
) -> Result<ActionResult, ActionError> {
    use crate::config;
    use crate::installer::{check_for_updates, perform_update};

    if !ctx.is_cli {
        return Err(ActionError::not_supported(
            "update is only available via CLI",
        ));
    }

    let mut settings = ctx.config.settings.update.clone();

    // enable prerelease channels if requested
    if prerelease {
        settings.channels.beta = true;
        settings.channels.dev = true;
    }

    let current = Version::current();
    println!("Current version: {}", current.version_string());
    println!("Checking for updates...");

    match check_for_updates(&settings, true)? {
        Some(release) => {
            println!("\n🆕 New version available: {}", release.version);

            if let Some(ref notes) = release.release_notes {
                println!("\nRelease notes:");
                println!("{}", notes);
            }

            println!("\nUpdate size: {:.2} MB", release.size as f64 / 1_048_576.0);

            if !force {
                print!("Install update? [Y/n]: ");
                use std::io::{self, Write};
                io::stdout()
                    .flush()
                    .map_err(|e| ActionError::general(e.to_string()))?;

                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| ActionError::general(e.to_string()))?;

                if input.trim().to_lowercase() == "n" {
                    return Ok(ActionResult::simple(
                        "update",
                        serde_json::json!({"status": "cancelled"}),
                    ));
                }
            }

            let new_version = release.version.clone();
            perform_update(release, force).map_err(ActionError::from)?;

            // update last check time in config
            let mut cfg = ctx.config.clone();
            cfg.settings.update.last_check = Some(chrono::Utc::now());
            if let Err(e) = config::save(&cfg) {
                eprintln!("warning: failed to save config: {}", e);
            }

            Ok(ActionResult::simple(
                "update",
                serde_json::json!({
                    "status": "updated",
                    "previous_version": current.full_version_string(),
                    "new_version": new_version,
                }),
            ))
        }
        None => {
            // update last check time
            let mut cfg = ctx.config.clone();
            cfg.settings.update.last_check = Some(chrono::Utc::now());
            if let Err(e) = config::save(&cfg) {
                eprintln!("warning: failed to save config: {}", e);
            }

            Ok(ActionResult::simple(
                "update",
                serde_json::json!({
                    "status": "up_to_date",
                    "current_version": current.full_version_string(),
                }),
            ))
        }
    }
}
