mod commands;

pub use commands::Cli;

use anyhow::Result;

pub fn run(cli: Cli) -> Result<()> {
    commands::execute(cli)
}
