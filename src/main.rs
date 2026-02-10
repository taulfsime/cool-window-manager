#[macro_use]
extern crate objc;

mod cli;
mod config;
mod daemon;
mod display;
mod window;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::run(cli)
}
