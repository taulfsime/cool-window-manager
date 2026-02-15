use chrono::{Datelike, TimeZone};
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    // get git info first (needed for man page generation)
    let (commit, short_commit, timestamp, dirty) = get_git_info();

    // release channel from environment or default to dev
    let channel = std::env::var("RELEASE_CHANNEL").unwrap_or_else(|_| "dev".to_string());

    // generate CalVer semantic version
    let semver = generate_calver(&timestamp, &channel, &short_commit);

    // generate man page to OUT_DIR
    generate_man_page(out_path, &semver);

    // compile spotlight stub to OUT_DIR
    compile_spotlight_stub(out_path);

    // set environment variables for compilation
    println!("cargo:rustc-env=GIT_COMMIT={}", commit);
    println!("cargo:rustc-env=GIT_COMMIT_SHORT={}", short_commit);
    println!("cargo:rustc-env=GIT_TIMESTAMP={}", timestamp);
    println!("cargo:rustc-env=GIT_DIRTY={}", dirty);

    // only rerun if .git/HEAD changes
    if Path::new(".git/HEAD").exists() {
        println!("cargo:rerun-if-changed=.git/HEAD");
    }

    // build date
    let build_date = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // github repo from environment or default
    let repo = std::env::var("CWM_GITHUB_REPO")
        .unwrap_or_else(|_| "taulfsime/cool-window-manager".to_string());
    println!("cargo:rustc-env=GITHUB_REPO={}", repo);

    println!("cargo:rustc-env=RELEASE_CHANNEL={}", channel);
    println!("cargo:rustc-env=SEMVER={}", semver);

    // rerun if source files change
    println!("cargo:rerun-if-changed=scripts/spotlight_stub.c");
    println!("cargo:rerun-if-changed=src/cli/commands.rs");
}

fn get_git_info() -> (String, String, String, bool) {
    // try to get git commit hash
    let commit = match Command::new("git").args(["rev-parse", "HEAD"]).output() {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .unwrap_or_default()
            .trim()
            .to_string(),
        _ => "unknown".to_string(),
    };

    // get short commit hash
    let short_commit = if commit.len() >= 8 {
        commit[..8].to_string()
    } else {
        commit.clone()
    };

    // get commit timestamp
    let timestamp = match Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .output()
    {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .unwrap_or_default()
            .trim()
            .to_string(),
        _ => "0".to_string(),
    };

    // check if working directory is dirty
    let dirty = match Command::new("git").args(["status", "--porcelain"]).output() {
        Ok(output) if output.status.success() => !output.stdout.is_empty(),
        _ => false,
    };

    (commit, short_commit, timestamp, dirty)
}

/// generates CalVer version string from commit timestamp
/// format: YYYY.M.D+channel.commit (dev/beta) or YYYY.M.D+commit (stable)
fn generate_calver(timestamp: &str, channel: &str, short_commit: &str) -> String {
    let ts: i64 = timestamp.parse().unwrap_or(0);

    let dt = if ts > 0 {
        chrono::Utc.timestamp_opt(ts, 0).single()
    } else {
        None
    }
    .unwrap_or_else(chrono::Utc::now);

    let calver = format!("{}.{}.{}", dt.year(), dt.month(), dt.day());

    match channel {
        "stable" => format!("{}+{}", calver, short_commit),
        _ => format!("{}+{}.{}", calver, channel, short_commit),
    }
}

/// generates man page using clap_mangen
fn generate_man_page(out_dir: &Path, version: &str) {
    use clap::{Arg, Command as ClapCommand};

    let man_dir = out_dir.join("man");
    std::fs::create_dir_all(&man_dir).ok();

    // leak the version string to get a 'static lifetime (acceptable in build script)
    let version_static: &'static str = Box::leak(version.to_string().into_boxed_str());

    // build a minimal CLI definition for man page generation
    // this mirrors src/cli/commands.rs but without the env!() macro dependency
    let cmd = ClapCommand::new("cwm")
        .about("A macOS window manager with CLI and global hotkeys")
        .version(version_static)
        .arg(
            Arg::new("config")
                .long("config")
                .global(true)
                .help("Path to config file (overrides CWM_CONFIG env var and default location)"),
        )
        .arg(
            Arg::new("json")
                .short('j')
                .long("json")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("Output in JSON format (auto-enabled when stdout is piped)"),
        )
        .arg(
            Arg::new("no-json")
                .long("no-json")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("json")
                .help("Force text output even when stdout is piped"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("Suppress all output on success (errors still go to stderr)"),
        )
        .subcommand(
            ClapCommand::new("focus")
                .about("Focus an application window")
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .required(true)
                        .action(clap::ArgAction::Append)
                        .help("Target app name(s) (fuzzy matched), tries each in order until one is found"),
                )
                .arg(
                    Arg::new("launch")
                        .long("launch")
                        .action(clap::ArgAction::SetTrue)
                        .conflicts_with("no-launch")
                        .help("Force launch app if not running (launches first app in list)"),
                )
                .arg(
                    Arg::new("no-launch")
                        .long("no-launch")
                        .action(clap::ArgAction::SetTrue)
                        .help("Never launch app even if configured to"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(clap::ArgAction::SetTrue)
                        .help("Show verbose output including match details"),
                ),
        )
        .subcommand(
            ClapCommand::new("maximize")
                .about("Maximize a window")
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .action(clap::ArgAction::Append)
                        .help("Target app name (defaults to frontmost window)"),
                )
                .arg(
                    Arg::new("launch")
                        .long("launch")
                        .action(clap::ArgAction::SetTrue)
                        .help("Force launch app if not running"),
                )
                .arg(
                    Arg::new("no-launch")
                        .long("no-launch")
                        .action(clap::ArgAction::SetTrue)
                        .help("Never launch app even if configured to"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(clap::ArgAction::SetTrue)
                        .help("Show verbose output"),
                ),
        )
        .subcommand(
            ClapCommand::new("resize")
                .about("Resize a window")
                .arg(
                    Arg::new("to")
                        .long("to")
                        .required(true)
                        .help("Target size: percentage (80), pixels (1920x1080), or points (800x600pt)"),
                )
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .action(clap::ArgAction::Append)
                        .help("Target app name (defaults to frontmost window)"),
                )
                .arg(
                    Arg::new("launch")
                        .long("launch")
                        .action(clap::ArgAction::SetTrue)
                        .help("Force launch app if not running"),
                )
                .arg(
                    Arg::new("no-launch")
                        .long("no-launch")
                        .action(clap::ArgAction::SetTrue)
                        .help("Never launch app even if configured to"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(clap::ArgAction::SetTrue)
                        .help("Show verbose output"),
                ),
        )
        .subcommand(
            ClapCommand::new("move")
                .about("Move a window to a position and/or display")
                .arg(
                    Arg::new("to")
                        .long("to")
                        .help("Target position: anchor (top-left, center, etc.), coordinates (100,200), or percentage (50%,50%)"),
                )
                .arg(
                    Arg::new("display")
                        .short('d')
                        .long("display")
                        .help("Target display: next, prev, number (1-based), or alias"),
                )
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .action(clap::ArgAction::Append)
                        .help("Target app name (defaults to frontmost window)"),
                )
                .arg(
                    Arg::new("launch")
                        .long("launch")
                        .action(clap::ArgAction::SetTrue)
                        .help("Force launch app if not running"),
                )
                .arg(
                    Arg::new("no-launch")
                        .long("no-launch")
                        .action(clap::ArgAction::SetTrue)
                        .help("Never launch app even if configured to"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(clap::ArgAction::SetTrue)
                        .help("Show verbose output"),
                ),
        )
        .subcommand(
            ClapCommand::new("list")
                .about("List resources")
                .arg(
                    Arg::new("resource")
                        .required(true)
                        .value_parser(["apps", "displays", "aliases", "events"])
                        .help("Resource to list: apps, displays, aliases, events"),
                )
                .arg(
                    Arg::new("names")
                        .long("names")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output only names, one per line"),
                )
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .help("Custom format string (e.g., '{name} - {pid}')"),
                )
                .arg(
                    Arg::new("detailed")
                        .long("detailed")
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed information"),
                ),
        )
        .subcommand(
            ClapCommand::new("get")
                .about("Get window information")
                .arg(
                    Arg::new("target")
                        .required(true)
                        .value_parser(["focused", "window"])
                        .help("What to get: focused (frontmost window) or window (specific app)"),
                )
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .action(clap::ArgAction::Append)
                        .help("Target app name (required for 'window' target)"),
                )
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .help("Custom format string"),
                ),
        )
        .subcommand(
            ClapCommand::new("check-permissions")
                .about("Check accessibility permissions")
                .arg(
                    Arg::new("prompt")
                        .long("prompt")
                        .action(clap::ArgAction::SetTrue)
                        .help("Prompt user to grant permissions if not granted"),
                ),
        )
        .subcommand(
            ClapCommand::new("record")
                .about("Record keyboard shortcuts or window layouts")
                .arg(
                    Arg::new("what")
                        .required(true)
                        .value_parser(["shortcut", "layout"])
                        .help("What to record: shortcut or layout"),
                )
                .arg(
                    Arg::new("timeout")
                        .long("timeout")
                        .help("Timeout in seconds (default: 10)"),
                ),
        )
        .subcommand(
            ClapCommand::new("config")
                .about("Manage configuration")
                .arg(
                    Arg::new("action")
                        .required(true)
                        .value_parser(["show", "path", "set", "reset", "default", "verify"])
                        .help("Config action: show, path, set, reset, default, verify"),
                )
                .arg(
                    Arg::new("key")
                        .help("Config key for set/reset (e.g., settings.fuzzy_threshold)"),
                )
                .arg(
                    Arg::new("value")
                        .help("Value for set action"),
                ),
        )
        .subcommand(
            ClapCommand::new("daemon")
                .about("Manage background daemon")
                .arg(
                    Arg::new("action")
                        .required(true)
                        .value_parser(["start", "stop", "status", "install", "uninstall"])
                        .help("Daemon action: start, stop, status, install, uninstall"),
                ),
        )
        .subcommand(
            ClapCommand::new("events")
                .about("Subscribe to window events")
                .arg(
                    Arg::new("action")
                        .required(true)
                        .value_parser(["listen", "wait"])
                        .help("Events action: listen (stream events) or wait (wait for specific event)"),
                )
                .arg(
                    Arg::new("filter")
                        .long("filter")
                        .help("Event filter pattern (glob syntax, e.g., 'window.*')"),
                )
                .arg(
                    Arg::new("timeout")
                        .long("timeout")
                        .help("Timeout in seconds for wait action"),
                ),
        )
        .subcommand(
            ClapCommand::new("spotlight")
                .about("Manage Spotlight integration")
                .arg(
                    Arg::new("action")
                        .required(true)
                        .value_parser(["install", "list", "remove", "example"])
                        .help("Spotlight action: install, list, remove, example"),
                )
                .arg(
                    Arg::new("name")
                        .help("Shortcut name for remove action"),
                ),
        )
        .subcommand(
            ClapCommand::new("install")
                .about("Install cwm to system PATH")
                .arg(
                    Arg::new("path")
                        .long("path")
                        .help("Installation directory (default: auto-detect)"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .action(clap::ArgAction::SetTrue)
                        .help("Overwrite existing installation"),
                )
                .arg(
                    Arg::new("completions")
                        .long("completions")
                        .num_args(0..=1)
                        .default_missing_value("all")
                        .help("Install shell completions (bash, zsh, fish, or all)"),
                )
                .arg(
                    Arg::new("no-completions")
                        .long("no-completions")
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip shell completion installation"),
                )
                .arg(
                    Arg::new("completions-only")
                        .long("completions-only")
                        .action(clap::ArgAction::SetTrue)
                        .help("Only install shell completions"),
                ),
        )
        .subcommand(ClapCommand::new("uninstall").about("Remove cwm from system"))
        .subcommand(
            ClapCommand::new("update")
                .about("Update to latest version")
                .arg(
                    Arg::new("check")
                        .long("check")
                        .action(clap::ArgAction::SetTrue)
                        .help("Only check for updates, don't install"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .action(clap::ArgAction::SetTrue)
                        .help("Force update even if already on latest"),
                )
                .arg(
                    Arg::new("prerelease")
                        .long("prerelease")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include prerelease versions"),
                ),
        )
        .subcommand(ClapCommand::new("version").about("Display version information"));

    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();

    if let Err(e) = man.render(&mut buffer) {
        eprintln!("Warning: Failed to generate man page: {}", e);
        // write empty placeholder
        std::fs::write(man_dir.join("cwm.1"), b"").ok();
        return;
    }

    if let Err(e) = std::fs::write(man_dir.join("cwm.1"), &buffer) {
        eprintln!("Warning: Failed to write man page: {}", e);
    }
}

/// compiles the spotlight stub executable to OUT_DIR
fn compile_spotlight_stub(out_dir: &Path) {
    let stub_source = Path::new("scripts/spotlight_stub.c");
    let assets_dir = out_dir.join("assets");
    let stub_output = assets_dir.join("spotlight_stub");

    // ensure assets directory exists
    std::fs::create_dir_all(&assets_dir).ok();

    // check if source exists
    if !stub_source.exists() {
        // create empty placeholder for initial compilation
        std::fs::write(&stub_output, b"").ok();
        return;
    }

    // get target architecture
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let arch_flag = match target_arch.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        _ => "arm64", // default to arm64 for Apple Silicon
    };

    // compile directly with clang
    let compile_status = Command::new("clang")
        .args([
            "-arch",
            arch_flag,
            "-O2",
            "-o",
            stub_output.to_str().unwrap(),
            stub_source.to_str().unwrap(),
        ])
        .status();

    match compile_status {
        Ok(status) if status.success() => {
            // compilation succeeded
        }
        _ => {
            // create empty placeholder if compilation fails
            std::fs::write(&stub_output, b"").ok();
            eprintln!("Warning: Failed to compile spotlight stub, using placeholder");
        }
    }
}
