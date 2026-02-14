//! unified action layer for CLI, IPC, and HTTP API
//!
//! this module provides a shared execution layer that all interfaces use,
//! ensuring consistent behavior and response formats across CLI commands,
//! IPC socket requests, and future HTTP API endpoints

mod command;
mod context;
mod error;
mod handlers;
mod parse;
mod result;

pub use command::{
    Command, ConfigCommand, DaemonCommand, GetTarget, ListResource, SpotlightCommand,
};
pub use context::ExecutionContext;
pub use error::ActionError;
pub use parse::JsonRpcRequest;
pub use result::ActionResult;

/// execute a command with the given context
///
/// this is the main entry point for all command execution, used by CLI, IPC, and HTTP
pub fn execute(cmd: Command, ctx: &ExecutionContext) -> Result<ActionResult, ActionError> {
    // check if command is interactive and we're not in CLI mode
    if cmd.is_interactive() && !ctx.is_cli {
        return Err(ActionError::not_supported(format!(
            "{} requires interactive input (CLI only)",
            cmd.method_name()
        )));
    }

    match cmd {
        // window commands
        Command::Focus { app, launch } => handlers::focus::execute(app, launch, ctx),
        Command::Maximize { app, launch } => handlers::maximize::execute(app, launch, ctx),
        Command::Resize {
            app,
            to,
            overflow,
            launch,
        } => handlers::resize::execute(app, to, overflow, launch, ctx),
        Command::Move {
            app,
            to,
            display,
            launch,
        } => handlers::move_window::execute(app, to, display, launch, ctx),

        // query commands
        Command::List { resource, detailed } => match resource {
            ListResource::Apps => handlers::list::execute_list_apps(detailed, ctx),
            ListResource::Displays => handlers::list::execute_list_displays(detailed, ctx),
            ListResource::Aliases => handlers::list::execute_list_aliases(detailed, ctx),
        },
        Command::Get { target } => match target {
            GetTarget::Focused => handlers::get::execute_get_focused(ctx),
            GetTarget::Window { app } => handlers::get::execute_get_window(app, ctx),
        },

        // system commands
        Command::Ping => handlers::system::execute_ping(ctx),
        Command::Status => handlers::system::execute_status(ctx),
        Command::Version => handlers::system::execute_version(ctx),
        Command::CheckPermissions { prompt } => {
            handlers::system::execute_check_permissions(prompt, ctx)
        }
        Command::RecordShortcut { .. } => {
            // record_shortcut is always interactive, handled by CLI directly
            Err(ActionError::not_supported(
                "record_shortcut requires interactive input (CLI only)",
            ))
        }

        // daemon commands
        Command::Daemon(daemon_cmd) => match daemon_cmd {
            DaemonCommand::Status => handlers::daemon::execute_status(ctx),
            DaemonCommand::Start { log, foreground } => {
                handlers::daemon::execute_start(log, foreground, ctx)
            }
            DaemonCommand::Stop => handlers::daemon::execute_stop(ctx),
            DaemonCommand::Install { bin, log } => handlers::daemon::execute_install(bin, log, ctx),
            DaemonCommand::Uninstall => handlers::daemon::execute_uninstall(ctx),
        },

        // config commands
        Command::Config(config_cmd) => {
            let config_override = ctx.config_path_override();
            match config_cmd {
                ConfigCommand::Show => handlers::config::execute_show(ctx),
                ConfigCommand::Path => handlers::config::execute_path(config_override),
                ConfigCommand::Verify => handlers::config::execute_verify(config_override),
                ConfigCommand::Default => handlers::config::execute_default(ctx),
                ConfigCommand::Set { ref key, ref value } => {
                    handlers::config::execute_set(key, value, config_override, ctx)
                }
                ConfigCommand::Reset => handlers::config::execute_reset(config_override, ctx),
            }
        }

        // spotlight commands
        Command::Spotlight(spotlight_cmd) => match spotlight_cmd {
            SpotlightCommand::List => handlers::spotlight::execute_list(ctx),
            SpotlightCommand::Example => handlers::spotlight::execute_example(ctx),
            SpotlightCommand::Install { ref name, force } => {
                handlers::spotlight::execute_install(name.as_deref(), force, ctx)
            }
            SpotlightCommand::Remove { ref name, all } => {
                handlers::spotlight::execute_remove(name.as_deref(), all, ctx)
            }
        },

        // install commands
        Command::Install {
            path,
            force,
            no_sudo,
            completions,
            no_completions,
            completions_only,
        } => handlers::install::execute_install(
            path,
            force,
            no_sudo,
            completions,
            no_completions,
            completions_only,
            ctx,
        ),
        Command::Uninstall { path } => handlers::install::execute_uninstall(path, ctx),
        Command::Update {
            check: true,
            prerelease,
            ..
        } => handlers::install::execute_update_check(prerelease, ctx),
        Command::Update {
            force, prerelease, ..
        } => handlers::install::execute_update(force, prerelease, ctx),
    }
}
