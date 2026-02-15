// main integration test file
// run with: cargo test --test integration

#[path = "integration_tests/common.rs"]
#[macro_use]
mod common;

#[path = "integration_tests/test_channels.rs"]
mod test_channels;

#[path = "integration_tests/test_completions.rs"]
mod test_completions;

#[path = "integration_tests/test_cli_config.rs"]
mod test_cli_config;

#[path = "integration_tests/test_cli_daemon.rs"]
mod test_cli_daemon;

#[path = "integration_tests/test_events.rs"]
mod test_events;

#[path = "integration_tests/test_cli_focus.rs"]
mod test_cli_focus;

#[path = "integration_tests/test_cli_get.rs"]
mod test_cli_get;

#[path = "integration_tests/test_cli_maximize.rs"]
mod test_cli_maximize;

#[path = "integration_tests/test_cli_move.rs"]
mod test_cli_move;

#[path = "integration_tests/test_cli_resize.rs"]
mod test_cli_resize;

#[path = "integration_tests/test_cli_version.rs"]
mod test_cli_version;

#[path = "integration_tests/test_install.rs"]
mod test_install;

#[path = "integration_tests/test_list.rs"]
mod test_list;

#[path = "integration_tests/test_man_page.rs"]
mod test_man_page;

#[path = "integration_tests/test_rollback.rs"]
mod test_rollback;

#[path = "integration_tests/test_spotlight.rs"]
mod test_spotlight;

#[path = "integration_tests/test_update.rs"]
mod test_update;
