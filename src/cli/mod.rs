mod commands;
pub mod exit_codes;
pub mod output;

pub use commands::Cli;

use anyhow::Result;

pub fn run(cli: Cli) -> Result<()> {
    commands::execute(cli)
}
