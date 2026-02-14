// integration tests for shell completion installation

use crate::common::*;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

// counter for unique test directory names
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// create a unique test directory name
fn unique_test_name(prefix: &str) -> String {
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();
    format!("{}_{:?}_{}", prefix, thread_id, count)
}

/// create a test config file with updates disabled
fn create_test_config(test_dir: &std::path::Path) -> std::path::PathBuf {
    let config_path = test_dir.join("config.json");
    let config = serde_json::json!({
        "shortcuts": [],
        "app_rules": [],
        "spotlight": [],
        "display_aliases": {},
        "settings": {
            "fuzzy_threshold": 2,
            "launch": false,
            "animate": false,
            "delay_ms": 500,
            "update": {
                "enabled": false
            }
        }
    });
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
        .expect("Failed to write test config");
    config_path
}

/// helper to run cwm install command with completions flags
fn run_install_completions(
    args: &[&str],
    config_path: &std::path::Path,
    env_vars: &[(&str, &str)],
) -> std::process::Output {
    let binary = cwm_binary_path();
    let mut cmd_args = vec![
        "--config",
        config_path.to_str().unwrap(),
        "--no-json",
        "install",
    ];
    cmd_args.extend(args);

    let mut cmd = Command::new(&binary);
    cmd.args(&cmd_args);
    cmd.env("CWM_GITHUB_API_URL", mock_server_url());

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    cmd.output().expect("Failed to run cwm install")
}

// ============================================================================
// completions-only mode tests
// ============================================================================

#[test]
fn test_completions_only_zsh() {
    let test_dir = create_test_dir(&unique_test_name("completions_zsh"));
    let config_path = create_test_config(&test_dir);

    // create a custom HOME to avoid modifying user's actual completions
    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=zsh"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "completions-only --completions=zsh should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // verify completion file was created
    let completion_path = fake_home.join(".zsh/completions/_cwm");
    assert!(
        completion_path.exists(),
        "zsh completion file should exist at {}",
        completion_path.display()
    );

    // verify content is valid zsh completion
    let content = fs::read_to_string(&completion_path).expect("Failed to read completion");
    assert!(
        content.contains("#compdef cwm"),
        "zsh completion should have #compdef header"
    );
    assert!(
        content.contains("focus"),
        "completion should include focus command"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_completions_only_bash() {
    let test_dir = create_test_dir(&unique_test_name("completions_bash"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=bash"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "completions-only --completions=bash should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let completion_path = fake_home.join(".bash_completion.d/cwm");
    assert!(
        completion_path.exists(),
        "bash completion file should exist at {}",
        completion_path.display()
    );

    let content = fs::read_to_string(&completion_path).expect("Failed to read completion");
    assert!(
        content.contains("_cwm") || content.contains("complete"),
        "bash completion should define completion function"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_completions_only_fish() {
    let test_dir = create_test_dir(&unique_test_name("completions_fish"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=fish"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "completions-only --completions=fish should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let completion_path = fake_home.join(".config/fish/completions/cwm.fish");
    assert!(
        completion_path.exists(),
        "fish completion file should exist at {}",
        completion_path.display()
    );

    let content = fs::read_to_string(&completion_path).expect("Failed to read completion");
    assert!(
        content.contains("complete -c cwm"),
        "fish completion should use complete -c cwm"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_completions_only_all() {
    let test_dir = create_test_dir(&unique_test_name("completions_all"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=all"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "completions-only --completions=all should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // verify all three completion files were created
    let zsh_path = fake_home.join(".zsh/completions/_cwm");
    let bash_path = fake_home.join(".bash_completion.d/cwm");
    let fish_path = fake_home.join(".config/fish/completions/cwm.fish");

    assert!(
        zsh_path.exists(),
        "zsh completion should exist: {}",
        zsh_path.display()
    );
    assert!(
        bash_path.exists(),
        "bash completion should exist: {}",
        bash_path.display()
    );
    assert!(
        fish_path.exists(),
        "fish completion should exist: {}",
        fish_path.display()
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// auto-detection tests
// ============================================================================

#[test]
fn test_completions_auto_detect_zsh() {
    let test_dir = create_test_dir(&unique_test_name("completions_auto_zsh"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    // set SHELL to zsh for auto-detection
    let output = run_install_completions(
        &["--completions-only", "--completions"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap()), ("SHELL", "/bin/zsh")],
    );

    assert!(
        output.status.success(),
        "completions auto-detect should succeed with SHELL=/bin/zsh: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let zsh_path = fake_home.join(".zsh/completions/_cwm");
    assert!(
        zsh_path.exists(),
        "zsh completion should be auto-installed when SHELL=/bin/zsh"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_completions_auto_detect_bash() {
    let test_dir = create_test_dir(&unique_test_name("completions_auto_bash"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions"],
        &config_path,
        &[
            ("HOME", fake_home.to_str().unwrap()),
            ("SHELL", "/usr/local/bin/bash"),
        ],
    );

    assert!(
        output.status.success(),
        "completions auto-detect should succeed with SHELL=/usr/local/bin/bash: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bash_path = fake_home.join(".bash_completion.d/cwm");
    assert!(
        bash_path.exists(),
        "bash completion should be auto-installed when SHELL=/usr/local/bin/bash"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// no-completions flag tests
// ============================================================================

#[test]
fn test_no_completions_skips_installation() {
    let test_dir = create_test_dir(&unique_test_name("no_completions"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    // create a fake install path so the install command has somewhere to "install"
    let install_path = test_dir.join("bin");
    fs::create_dir_all(&install_path).expect("Failed to create install path");

    let binary = cwm_binary_path();
    let output = Command::new(&binary)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--no-json",
            "install",
            "--no-completions",
            "--path",
            install_path.to_str().unwrap(),
        ])
        .env("HOME", fake_home.to_str().unwrap())
        .env("CWM_GITHUB_API_URL", mock_server_url())
        .output()
        .expect("Failed to run cwm install");

    // the install may fail for other reasons (permissions, etc) but that's ok
    // we just want to verify no completion files were created
    let zsh_path = fake_home.join(".zsh/completions/_cwm");
    let bash_path = fake_home.join(".bash_completion.d/cwm");
    let fish_path = fake_home.join(".config/fish/completions/cwm.fish");

    assert!(
        !zsh_path.exists(),
        "zsh completion should not exist with --no-completions"
    );
    assert!(
        !bash_path.exists(),
        "bash completion should not exist with --no-completions"
    );
    assert!(
        !fish_path.exists(),
        "fish completion should not exist with --no-completions"
    );

    // verify stdout mentions skipping completions or doesn't mention installing them
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // should not contain "Installing completions" or similar
    assert!(
        !combined.contains("Installing completions")
            || combined.contains("Skipping")
            || combined.contains("--no-completions"),
        "output should indicate completions were skipped"
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// invalid shell argument tests
// ============================================================================

#[test]
fn test_completions_invalid_shell() {
    let test_dir = create_test_dir(&unique_test_name("completions_invalid"));
    let config_path = create_test_config(&test_dir);

    let output = run_install_completions(
        &["--completions-only", "--completions=invalid"],
        &config_path,
        &[],
    );

    assert!(
        !output.status.success(),
        "completions with invalid shell should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown shell") || stderr.contains("invalid") || stderr.contains("Valid"),
        "error should mention unknown/invalid shell: {}",
        stderr
    );

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// completions-only without --completions flag
// ============================================================================

#[test]
fn test_completions_only_requires_completions_flag_or_interactive() {
    let test_dir = create_test_dir(&unique_test_name("completions_only_no_flag"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    // run with --completions-only but no --completions flag
    // this should either prompt (which will fail in non-interactive) or auto-detect
    let output = run_install_completions(
        &["--completions-only"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap()), ("SHELL", "/bin/zsh")],
    );

    // in non-interactive mode (piped), this should either:
    // 1. auto-detect and install (success)
    // 2. fail because it can't prompt
    // either is acceptable behavior
    if output.status.success() {
        // if it succeeded, verify completion was installed
        let zsh_path = fake_home.join(".zsh/completions/_cwm");
        assert!(
            zsh_path.exists(),
            "if completions-only succeeds, completion file should exist"
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// completion content validation tests
// ============================================================================

#[test]
fn test_completion_contains_all_commands() {
    let test_dir = create_test_dir(&unique_test_name("completions_commands"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=zsh"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(output.status.success());

    let completion_path = fake_home.join(".zsh/completions/_cwm");
    let content = fs::read_to_string(&completion_path).expect("Failed to read completion");

    // verify all main commands are present
    let commands = [
        "focus",
        "maximize",
        "resize",
        "move-display",
        "list",
        "get",
        "config",
        "daemon",
        "install",
        "uninstall",
        "update",
        "version",
        "spotlight",
    ];

    for cmd in commands {
        assert!(
            content.contains(cmd),
            "completion should include '{}' command",
            cmd
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_completion_contains_flags() {
    let test_dir = create_test_dir(&unique_test_name("completions_flags"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=zsh"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(output.status.success());

    let completion_path = fake_home.join(".zsh/completions/_cwm");
    let content = fs::read_to_string(&completion_path).expect("Failed to read completion");

    // verify common flags are present
    let flags = ["--app", "--json", "--verbose", "--config", "--help"];

    for flag in flags {
        assert!(
            content.contains(flag),
            "completion should include '{}' flag",
            flag
        );
    }

    cleanup_test_dir(&test_dir);
}

// ============================================================================
// output message tests
// ============================================================================

#[test]
fn test_completions_success_message() {
    let test_dir = create_test_dir(&unique_test_name("completions_message"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=zsh"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // should mention installation success
    assert!(
        stdout.contains("Installed") || stdout.contains("completion") || stdout.contains("zsh"),
        "output should confirm completion installation: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_completions_all_success_message() {
    let test_dir = create_test_dir(&unique_test_name("completions_all_message"));
    let config_path = create_test_config(&test_dir);

    let fake_home = test_dir.join("home");
    fs::create_dir_all(&fake_home).expect("Failed to create fake home");

    let output = run_install_completions(
        &["--completions-only", "--completions=all"],
        &config_path,
        &[("HOME", fake_home.to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // should mention all three shells
    assert!(
        stdout.contains("zsh") || stdout.contains("Zsh"),
        "output should mention zsh: {}",
        stdout
    );
    assert!(
        stdout.contains("bash") || stdout.contains("Bash"),
        "output should mention bash: {}",
        stdout
    );
    assert!(
        stdout.contains("fish") || stdout.contains("Fish"),
        "output should mention fish: {}",
        stdout
    );

    cleanup_test_dir(&test_dir);
}
