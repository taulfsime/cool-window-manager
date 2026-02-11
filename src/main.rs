mod cli;
mod config;
mod daemon;
mod display;
mod installer;
mod version;
mod window;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    // check for updates in background (non-blocking)
    check_for_updates_if_needed();
    
    let cli = Cli::parse();
    cli::run(cli)
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
    use config::{UpdateFrequency, AutoUpdateMode};
    
    let config = config::load()?;
    let update_settings = &config.settings.update;
    
    // check if updates are enabled
    if !update_settings.enabled {
        return Ok(());
    }
    
    // check if it's time to check
    let should_check = match update_settings.check_frequency {
        UpdateFrequency::Manual => false,
        UpdateFrequency::Daily => {
            update_settings.last_check
                .map(|last| Utc::now() - last > Duration::days(1))
                .unwrap_or(true)
        }
        UpdateFrequency::Weekly => {
            update_settings.last_check
                .map(|last| Utc::now() - last > Duration::weeks(1))
                .unwrap_or(true)
        }
    };
    
    if !should_check {
        return Ok(());
    }
    
    // perform check
    if let Some(release) = installer::check_for_updates(update_settings, false)? {
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
