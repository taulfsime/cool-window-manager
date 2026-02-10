pub mod hotkeys;
pub mod ipc;

use anyhow::Result;

pub fn start() -> Result<()> {
    // TODO: implement daemon start
    Ok(())
}

pub fn stop() -> Result<()> {
    // TODO: implement daemon stop
    Ok(())
}

pub fn status() -> Result<bool> {
    // TODO: implement daemon status check
    Ok(false)
}
