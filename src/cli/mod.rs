mod commands;
mod convert;
pub mod exit_codes;
pub mod output;

pub use commands::Cli;

use anyhow::Result;

pub fn run(cli: Cli) -> Result<()> {
    commands::execute(cli)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_reexport() {
        // verify Cli is properly re-exported
        let cli = Cli::try_parse_from(["cwm", "version"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_help() {
        // verify help works
        let result = Cli::try_parse_from(["cwm", "--help"]);
        // --help causes an error (but it's the expected "help" error)
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_version_flag() {
        // verify --version works
        let result = Cli::try_parse_from(["cwm", "--version"]);
        // --version causes an error (but it's the expected "version" error)
        assert!(result.is_err());
    }
}
