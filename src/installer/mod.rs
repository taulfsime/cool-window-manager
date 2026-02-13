pub mod github;
pub mod install;
pub mod paths;
pub mod rollback;
pub mod update;

pub use install::{install_binary, uninstall_binary};
pub use paths::detect_install_paths;
pub use update::{check_for_updates, check_for_updates_silent, perform_update};
