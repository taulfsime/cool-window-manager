mod actions;
mod cli;
mod conditions;
mod config;
mod daemon;
mod display;
mod installer;
mod spotlight;
mod version;
mod window;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    // handle broken pipe gracefully (e.g., when piping to `head` or `jq` that exits early)
    reset_sigpipe();

    let cli = Cli::parse();

    // skip background update check when JSON output is expected
    // (either via --json flag or when stdout is piped)
    use std::io::IsTerminal;
    let is_json_output = cli.json || !std::io::stdout().is_terminal();
    if !is_json_output {
        check_for_updates_if_needed();
    }

    cli::run(cli)
}

/// reset SIGPIPE to default behavior (terminate process) instead of panicking
/// this is the standard Unix behavior for CLI tools
fn reset_sigpipe() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

fn check_for_updates_if_needed() {
    // run update check in a separate thread to not block startup
    std::thread::spawn(|| {
        if let Err(e) = do_update_check() {
            // silently ignore errors during background update check
            eprintln!("Update check failed: {}", e);
        }
    });
}

fn do_update_check() -> Result<()> {
    use chrono::{Duration, Utc};
    use config::{AutoUpdateMode, UpdateFrequency};

    let config = config::load()?;
    let update_settings = &config.settings.update;

    // check if updates are enabled
    if !update_settings.enabled {
        return Ok(());
    }

    // check if it's time to check
    let should_check = match update_settings.check_frequency {
        UpdateFrequency::Manual => false,
        UpdateFrequency::Daily => update_settings
            .last_check
            .map(|last| Utc::now() - last > Duration::days(1))
            .unwrap_or(true),
        UpdateFrequency::Weekly => update_settings
            .last_check
            .map(|last| Utc::now() - last > Duration::weeks(1))
            .unwrap_or(true),
    };

    if !should_check {
        return Ok(());
    }

    // perform check (silent - no interactive prompts)
    if let Some(release) = installer::check_for_updates_silent(update_settings)? {
        let current = version::Version::current();

        // always show notification
        eprintln!();
        eprintln!("🆕 New version available: {}", release.version);
        eprintln!("   Current: {}", current.version_string());

        match update_settings.auto_update {
            AutoUpdateMode::Always => {
                eprintln!("   Auto-updating...");
                if let Err(e) = installer::perform_update(release, false) {
                    eprintln!("   Update failed: {}", e);
                }
            }
            AutoUpdateMode::Prompt | AutoUpdateMode::Never => {
                eprintln!("   Run 'cwm update' to install");
            }
        }
        eprintln!();

        // update last check time
        let mut config = config;
        config.settings.update.last_check = Some(Utc::now());
        config::save(&config)?;
    }

    Ok(())
}
