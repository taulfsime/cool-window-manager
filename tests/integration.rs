// main integration test file
// run with: cargo test --test integration

#[path = "integration_tests/common.rs"]
#[macro_use]
mod common;

#[path = "integration_tests/test_channels.rs"]
mod test_channels;

#[path = "integration_tests/test_install.rs"]
mod test_install;

#[path = "integration_tests/test_list.rs"]
mod test_list;

#[path = "integration_tests/test_rollback.rs"]
mod test_rollback;

#[path = "integration_tests/test_update.rs"]
mod test_update;
