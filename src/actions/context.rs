//! execution context for actions

use std::path::{Path, PathBuf};

use crate::config::Config;

/// context passed to action handlers
pub struct ExecutionContext<'a> {
    /// configuration
    pub config: &'a Config,
    /// whether to print verbose debug output
    pub verbose: bool,
    /// whether running in CLI context (allows interactive commands)
    pub is_cli: bool,
    /// config file path override (for --config flag)
    pub config_path: Option<PathBuf>,
}

impl<'a> ExecutionContext<'a> {
    /// create context for IPC/HTTP (non-interactive)
    pub fn new(config: &'a Config, verbose: bool) -> Self {
        Self {
            config,
            verbose,
            is_cli: false,
            config_path: None,
        }
    }

    /// create context with explicit is_cli and verbose flags
    pub fn new_with_verbose(config: &'a Config, is_cli: bool, verbose: bool) -> Self {
        Self {
            config,
            verbose,
            is_cli,
            config_path: None,
        }
    }

    /// create CLI context with config path override
    pub fn cli_with_config_path(
        config: &'a Config,
        verbose: bool,
        config_path: Option<&Path>,
    ) -> Self {
        Self {
            config,
            verbose,
            is_cli: true,
            config_path: config_path.map(|p| p.to_path_buf()),
        }
    }

    /// get config path override as Option<&Path>
    pub fn config_path_override(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }
}
