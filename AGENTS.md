# AGENTS.md

> context and guidance for AI coding agents working on this repository

## Documentation Maintenance

When making changes to this repository, always keep documentation in sync:

### AGENTS.md

Update this file when:
- adding, removing, or renaming files or modules
- changing the repository structure
- adding new dependencies
- introducing new patterns or conventions
- modifying configuration schema
- changing build, test, or run procedures

### README.md

Update the README when:
- adding or removing CLI commands
- changing command flags or arguments
- modifying configuration options or schema
- changing installation or build steps
- updating usage examples

### General Rules

- documentation updates should be part of the same change, not a separate commit
- keep examples accurate and working
- remove references to deleted features
- update tables and lists to reflect current state

---

## Project Overview

**cwm** (Cool Window Manager) is a macOS window manager written in Rust. It provides CLI commands and global hotkeys for managing application windows.

### Core Functionality

- **focus**: bring an application window to the foreground
- **maximize**: resize a window to fill the screen
- **resize**: resize a window to a percentage of the screen (centered)
- **move-display**: move a window to another monitor

### Architecture

The project has two modes of operation:

1. **CLI mode**: direct command execution (`cwm focus --app Safari`)
2. **Daemon mode**: background process listening for global hotkeys and app launches

### Platform

macOS only. The codebase requires macOS and will not compile on other platforms.

---

## Repository Structure

```
cool-window-mng/
├── Cargo.toml              # project manifest and dependencies
├── Cargo.lock              # locked dependency versions
├── build.rs                # build script for git version info
├── README.md               # user-facing documentation
├── AGENTS.md               # this file
├── install.sh              # shell installer script
├── .gitignore              # ignores target/ and session- files
├── .github/
│   └── workflows/
│       └── ci.yml          # GitHub Actions CI/CD workflow
├── scripts/
│   ├── release-beta.sh     # helper script to create beta releases
│   ├── release-stable.sh   # helper script to create stable releases
│   └── list-releases.sh    # helper script to list all releases
├── tests/
│   ├── integration.rs      # integration test entry point
│   └── integration_tests/
│       ├── common.rs       # shared test utilities
│       ├── test_install.rs # install test scenarios
│       ├── test_update.rs  # update test scenarios
│       ├── test_rollback.rs # rollback test scenarios
│       └── test_channels.rs # channel switching tests
└── src/
    ├── main.rs             # entry point, update check, delegates to cli::run()
    ├── version.rs          # version management (Version, VersionInfo)
    ├── cli/
    │   ├── mod.rs          # module exports
    │   └── commands.rs     # CLI command definitions and execution
    ├── config/
    │   ├── mod.rs          # config loading, saving, value manipulation, JSONC parsing
    │   ├── schema.rs       # Config, Shortcut, AppRule, Settings, UpdateSettings
    │   └── json_schema.rs  # JSON schema definition for editor autocompletion
    ├── daemon/
    │   ├── mod.rs          # daemon lifecycle, action execution, hotkey parsing
    │   ├── hotkeys.rs      # global hotkey recording and listening (CGEventTap)
    │   ├── ipc.rs          # PID file management at /tmp/cwm.pid
    │   ├── launchd.rs      # macOS launchd plist for auto-start
    │   └── app_watcher.rs  # NSWorkspace notifications for app launches
    ├── display/
    │   └── mod.rs          # display enumeration, multi-monitor targeting
    ├── installer/
    │   ├── mod.rs          # module exports
    │   ├── paths.rs        # installation path detection and validation
    │   ├── install.rs      # binary installation logic
    │   ├── github.rs       # GitHub API client for releases
    │   ├── update.rs       # update checking and downloading
    │   └── rollback.rs     # safe update with automatic rollback
    ├── spotlight/
    │   ├── mod.rs          # module exports, example config printing
    │   └── generator.rs    # app bundle generation for Spotlight integration
    └── window/
        ├── mod.rs          # module exports
        ├── accessibility.rs # Accessibility API permission handling
        ├── manager.rs      # window manipulation (focus, maximize, move, resize)
        └── matching.rs     # fuzzy application name matching (Levenshtein)
```

---

## Module Architecture

### cli/

Handles command-line argument parsing and command execution.

| File | Responsibility |
|------|----------------|
| `mod.rs` | re-exports `run()` and `Cli` |
| `commands.rs` | defines `Cli` struct with clap derive, `Commands` enum, and `run()` function that dispatches to appropriate handlers |

Commands defined: `focus`, `maximize`, `move-display`, `resize`, `list-apps`, `list-displays`, `check-permissions`, `record-shortcut`, `config`, `daemon`, `version`, `install`, `uninstall`, `update`, `spotlight`

### config/

Manages JSONC configuration file at `~/.cwm/config.json` or `~/.cwm/config.jsonc`.

| File | Responsibility |
|------|----------------|
| `mod.rs` | `load()`, `save()`, `get_config_path()`, `ensure_cwm_dir()`, `set_value()`, `verify()`, JSONC parsing, `*_with_override()` variants for custom config paths |
| `schema.rs` | data structures with serde derive |
| `json_schema.rs` | JSON schema definition and `write_schema_file()` |

**Config file format:**
- Supports both `.json` and `.jsonc` extensions
- JSONC allows single-line (`//`) and multi-line (`/* */`) comments
- If both `config.json` and `config.jsonc` exist, an error is raised
- Schema file auto-generated at `~/.cwm/config.schema.json`
- Config includes `$schema` field for editor autocompletion

Key types:
- `Config`: root configuration object with `$schema` field
- `Shortcut`: hotkey binding with keys, action, app, optional overrides
- `AppRule`: automatic action when an app launches
- `SpotlightShortcut`: macOS Spotlight integration shortcut definition
- `Settings`: global defaults (fuzzy_threshold, launch, animate, delay_ms, retry, update)
- `Retry`: retry configuration (count, delay_ms, backoff)
- `UpdateSettings`: update configuration (enabled, check_frequency, auto_update, channels, telemetry)
- `UpdateChannels`: channel selection (dev, beta, stable)
- `TelemetrySettings`: error reporting settings

### installer/

Handles installation, updates, and version management.

| File | Responsibility |
|------|----------------|
| `mod.rs` | module exports and coordination |
| `paths.rs` | installation path detection and validation |
| `install.rs` | binary installation logic |
| `update.rs` | update checking and downloading |
| `github.rs` | GitHub API client for releases |
| `rollback.rs` | safe update with automatic rollback |

### version.rs

Standalone module for version information.

- `Version` struct with commit hash, timestamp, channel
- `VersionInfo` for persistent version tracking at `~/.cwm/version.json`
  - Includes `schema_version` field to track when JSON schema was last generated
- Build-time version embedding via `build.rs`

### spotlight/

macOS Spotlight integration via generated app bundles.

| File | Responsibility |
|------|----------------|
| `mod.rs` | module exports, `print_example_config()`, constants |
| `generator.rs` | `install_shortcut()`, `install_all()`, `remove_shortcut()`, `remove_all()`, `get_installed_shortcuts()`, app bundle generation |

The spotlight module creates minimal `.app` bundles in `~/Applications/cwm/` that:
- Appear in Spotlight search with "cwm: " prefix
- Execute cwm commands via shell scripts
- Show notifications on errors via osascript
- Run without showing in the Dock (LSUIElement)

Key functions:
- `install_shortcut()`: creates a single app bundle from SpotlightShortcut config
- `install_all()`: installs all configured shortcuts, triggers Spotlight reindex
- `remove_shortcut()`: removes a single shortcut by name
- `remove_all()`: removes all cwm spotlight shortcuts
- `get_installed_shortcuts()`: lists installed shortcut names

### daemon/

Background process that listens for hotkeys and app launches.

| File | Responsibility |
|------|----------------|
| `mod.rs` | `start_daemon()`, `stop_daemon()`, `daemon_status()`, action execution, modifier/key parsing |
| `hotkeys.rs` | `record_shortcut()`, `listen_for_hotkeys()` using CGEventTap |
| `ipc.rs` | `write_pid_file()`, `read_pid_file()`, `remove_pid_file()` |
| `launchd.rs` | `install_launchd()`, `uninstall_launchd()` for auto-start |
| `app_watcher.rs` | `AppWatcher` struct, NSWorkspace notification observer |

The daemon uses:
- PID file at `/tmp/cwm.pid` for single-instance enforcement
- Signal handlers (SIGTERM, SIGINT) for graceful shutdown
- Tokio async runtime for concurrent hotkey and app watching

### display/

Multi-monitor support and display targeting.

| File | Responsibility |
|------|----------------|
| `mod.rs` | `list_displays()`, `DisplayInfo` struct, `DisplayTarget` enum, `resolve_target_display()` |

Display targets: `next`, `prev`, or numeric index (1-based). Wraps around at boundaries.

### window/

Core window manipulation using macOS Accessibility API.

| File | Responsibility |
|------|----------------|
| `mod.rs` | re-exports key types and functions |
| `accessibility.rs` | `check_accessibility_permissions()`, `prompt_accessibility_permissions()` |
| `manager.rs` | `focus_app()`, `maximize_app()`, `move_to_display()`, `resize_app()`, `launch_app()` |
| `matching.rs` | `find_app()`, `get_running_apps()`, `get_window_titles()`, `AppInfo`, `MatchType` enum, Levenshtein distance calculation |

---

## Key Concepts

### Fuzzy Matching

Applications are matched by name and window title with the following priority:

1. **Name exact match** (case-insensitive): `"safari"` matches `"Safari"`
2. **Name prefix match**: `"saf"` matches `"Safari"`
3. **Name fuzzy match**: Levenshtein distance within threshold (default: 2)
4. **Title exact match**: `"New Tab"` matches Chrome window with that title
5. **Title prefix match**: `"GitHub - taulfsime"` matches window starting with that
6. **Title fuzzy match**: Levenshtein distance within threshold

Window titles are retrieved via the Accessibility API (`AXTitle` attribute) for all windows of each running application. The `AppInfo` struct contains a `titles: Vec<String>` field with all window titles.

The threshold is configurable via `settings.fuzzy_threshold` in config.

### Configuration Hierarchy

Launch behavior follows a priority chain:

1. CLI flags (`--launch` / `--no-launch`) - highest priority
2. Shortcut-specific `launch` field
3. Global `settings.launch` - lowest priority

This pattern is implemented in `should_launch()`.

### Daemon Architecture

The daemon runs as a background process with:

- **Event tap**: CGEventTap intercepts keyboard events system-wide
- **App watcher**: NSWorkspace notifications detect app launches
- **Action execution**: parses hotkey, finds matching shortcut, executes action
- **Graceful shutdown**: responds to SIGTERM/SIGINT, cleans up PID file

### Action System

All window actions share a common pattern:

1. Resolve target app (from `--app` flag or frontmost window)
2. Find matching running application using fuzzy matching
3. Optionally launch app if not running
4. Get window reference via Accessibility API
5. Perform the action (focus, maximize, resize, move)

---

## macOS Platform Notes

### Accessibility API

The window manipulation code uses the macOS Accessibility API (`AXUIElement`).

**Safety considerations:**
- Requires user to grant accessibility permissions in System Preferences
- Uses raw CoreFoundation pointers (`AXUIElementRef`)
- Memory management via `CFRelease` is manual in some places
- Permission check should happen before any window operations

Key functions in `window/accessibility.rs`:
- `AXIsProcessTrusted()` - check if permissions granted
- `AXIsProcessTrustedWithOptions()` - prompt user for permissions

### CoreGraphics Event Taps

Global hotkey listening uses CGEventTap in `daemon/hotkeys.rs`.

**Safety considerations:**
- Event taps require accessibility permissions
- The callback runs on a separate run loop
- Uses `unsafe` blocks for FFI calls to CoreGraphics
- Raw pointers passed to/from C functions
- Must properly manage CGEventTap lifecycle (enable/disable/invalidate)

### Objective-C Bindings

The codebase uses `objc2` crate family for Objective-C interop:

- `objc2`: runtime bindings, message sending
- `objc2-app-kit`: NSWorkspace, NSRunningApplication, NSScreen
- `objc2-foundation`: NSArray, NSString, NSDictionary
- `block2`: Objective-C block support for callbacks

**Safety considerations:**
- Objective-C objects have their own reference counting (retain/release)
- `objc2` handles most memory management automatically
- Notification observers must be properly removed on cleanup
- Some APIs return nullable pointers that must be checked

---

## Dependencies

### Core

| Crate | Purpose |
|-------|---------|
| `clap` (v4) | CLI argument parsing with derive macros |
| `serde` + `serde_json` | JSON serialization for config |
| `json5` (v0.4) | JSONC parsing (comments, trailing commas) |
| `dirs` (v5) | cross-platform home directory |
| `strsim` (v0.11) | Levenshtein distance for fuzzy matching |
| `anyhow` | error handling with context |
| `thiserror` | custom error type derivation |
| `tokio` (full) | async runtime for daemon |
| `chrono` | timestamp formatting with serde support |
| `libc` | signal handling |
| `reqwest` (v0.11) | HTTP client for GitHub API |
| `sha2` (v0.10) | SHA256 checksum verification |
| `hex` (v0.4) | hex encoding for checksums |
| `tempfile` (v3) | temporary files during updates |
| `indicatif` (v0.17) | progress bars for downloads |
| `rand` (v0.8) | random delays for rate limiting |
| `shellexpand` (v3) | path expansion (~/.local/bin) |

### macOS-Specific

| Crate | Purpose |
|-------|---------|
| `core-foundation` (v0.10) | CF types (CFString, CFArray, CFDictionary) |
| `core-graphics` (v0.24) | display info, window geometry |
| `objc2` (v0.6) | Objective-C runtime |
| `block2` (v0.6) | Objective-C blocks |
| `objc2-app-kit` (v0.3) | NSWorkspace, NSRunningApplication, NSScreen |
| `objc2-foundation` (v0.3) | NSArray, NSString, NSDictionary |

### System Frameworks (FFI)

- ApplicationServices (Accessibility API)
- CoreGraphics (event taps, display enumeration)
- CoreFoundation (memory management, types)
- AppKit (workspace notifications)

---

## Build, Test, Run

### Build

```bash
# debug build
cargo build

# release build (recommended for actual use)
cargo build --release

# build with specific channel
RELEASE_CHANNEL=stable cargo build --release

# binary location
./target/release/cwm
```

The build.rs script automatically embeds:
- Git commit hash
- Commit timestamp
- Build date
- Release channel
- GitHub repository URL

### Test

```bash
# run all tests (unit + integration)
cargo test
```

#### Unit Tests

Tests are located in `#[cfg(test)]` modules within:

| File | Test Coverage |
|------|---------------|
| `src/config/mod.rs` | config value parsing, key setting, JSONC parsing, multi-extension support, config path override |
| `src/config/schema.rs` | `should_launch` priority logic, update settings serialization, `$schema` field |
| `src/config/json_schema.rs` | schema validity, required definitions, file writing |
| `src/display/mod.rs` | display target parsing, resolution with wraparound |
| `src/window/matching.rs` | name matching (exact, prefix, fuzzy), title matching (exact, prefix, fuzzy) |
| `src/daemon/hotkeys.rs` | hotkey string parsing |
| `src/version.rs` | version parsing, comparison, serialization |
| `src/installer/paths.rs` | path detection, writability checks, PATH detection |
| `src/installer/github.rs` | release parsing, architecture detection |
| `src/spotlight/generator.rs` | shell escaping, Info.plist generation, shell script generation |

#### Integration Tests

Integration tests for install, update, and rollback features:

| File | Test Coverage |
|------|---------------|
| `tests/integration_tests/test_install.rs` | fresh install, directory creation, permissions, force overwrite |
| `tests/integration_tests/test_update.rs` | version checking, download, checksum verification, binary replacement |
| `tests/integration_tests/test_rollback.rs` | backup creation, test failure rollback, checksum mismatch, corrupt download |
| `tests/integration_tests/test_channels.rs` | dev/beta/stable channels, channel priority, upgrade paths |

Note: Some integration tests require a mock GitHub API server and are skipped when not available.

### Run

```bash
# check permissions first
cwm check-permissions --prompt

# direct commands
cwm focus --app Safari
cwm maximize
cwm resize 80
cwm move-display next

# daemon mode
cwm daemon start
cwm daemon status
cwm daemon stop

# auto-start on login
cwm daemon install
cwm daemon uninstall
```

### Permissions

The tool requires **Accessibility permissions** to function:

1. Run `cwm check-permissions --prompt`
2. Grant permission in System Preferences > Privacy & Security > Accessibility
3. Restart terminal if needed

---

## Configuration

### File Location

- Configuration: `~/.cwm/config.json` or `~/.cwm/config.jsonc`
- Schema: `~/.cwm/config.schema.json` (auto-generated)
- Version info: `~/.cwm/version.json`
- Override config path (priority order):
  1. `--config <path>` CLI flag (highest priority)
  2. `CWM_CONFIG` environment variable
  3. Default `~/.cwm/config.json`

**Note:** If both `config.json` and `config.jsonc` exist, an error is raised. The config supports JSONC format with single-line (`//`) and multi-line (`/* */`) comments. When using `--config`, the specified file must exist (no auto-creation).

### Schema

```json
{
  "$schema": "./config.schema.json",
  "display_aliases": {
    "office": ["10AC_D0B3_67890"],
    "home": ["1E6D_5B11_12345", "10AC_D0B3_67890"]
  },
  "shortcuts": [
    {
      "keys": "ctrl+alt+s",
      "action": "focus",
      "app": "Slack",
      "launch": true
    },
    {
      "keys": "ctrl+alt+e",
      "action": "move_display:external"
    },
    {
      "keys": "ctrl+alt+o",
      "action": "move_display:office"
    }
  ],
  "app_rules": [
    {
      "app": "Terminal",
      "action": "maximize",
      "delay_ms": 1000
    }
  ],
  "spotlight": [
    {
      "name": "Focus Safari",
      "action": "focus",
      "app": "Safari",
      "launch": true
    },
    {
      "name": "Maximize Window",
      "action": "maximize"
    },
    {
      "name": "Move to Next Display",
      "action": "move_display:next"
    },
    {
      "name": "Resize 80%",
      "action": "resize:80"
    }
  ],
  "settings": {
    "fuzzy_threshold": 2,
    "launch": false,
    "animate": false,
    "delay_ms": 500,
    "retry": {
      "count": 10,
      "delay_ms": 100,
      "backoff": 1.5
    },
    "update": {
      "enabled": true,
      "check_frequency": "daily",
      "auto_update": "prompt",
      "channels": {
        "dev": false,
        "beta": false,
        "stable": true
      },
      "telemetry": {
        "enabled": false,
        "include_system_info": false
      }
    }
  }
}
```

### Display Aliases

Map display names to unique hardware IDs for multi-location setups (home/office).

| Field | Type | Description |
|-------|------|-------------|
| `display_aliases` | object | Maps alias names to arrays of unique display IDs |

**System Aliases** (automatically available, no config needed):
- `builtin` - Built-in display (e.g., MacBook screen)
- `external` - Any external monitor
- `main` - Primary display
- `secondary` - Secondary display

**User-Defined Aliases** - Create custom names for specific monitors:

```json
{
  "display_aliases": {
    "office_main": ["10AC_D0B3_67890"],
    "home_external": ["1E6D_5B11_12345", "10AC_D0B3_67890"]
  }
}
```

Find unique display IDs:
```bash
cwm list-displays --detailed
# Shows: Display 1: LG Display (1920x1080) [unique_id: 1E6D_5B11_12345]
```

Use in actions:
```json
{
  "action": "move_display:external",
  "action": "move_display:office_main"
}
```

### Shortcut Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `keys` | string | yes | hotkey combination (e.g., `"ctrl+alt+s"`) |
| `action` | string | yes | one of: `focus`, `maximize`, `resize`, `move_display:<target>` |
| `app` | string | no | application name (fuzzy matched) |
| `launch` | bool | no | override global launch behavior |

### App Rule Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `app` | string | yes | application name (exact match) |
| `action` | string | yes | action to perform on launch |
| `delay_ms` | number | no | delay before applying rule |

### Spotlight Shortcut Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | name displayed in Spotlight (prefixed with "cwm: ") |
| `action` | string | yes | same format as shortcuts: `focus`, `maximize`, `move_display:next`, `resize:80` |
| `app` | string | for focus | target application name (fuzzy matched) |
| `launch` | bool | no | launch app if not running |

### Modifier Keys

Supported modifiers in hotkey strings:
- `ctrl` or `control`
- `alt` or `option`
- `shift`
- `cmd` or `command`

Example: `"ctrl+alt+shift+m"`

---

## Error Handling

The codebase uses `anyhow::Result` throughout for error propagation. Errors are enriched with context using `.context()` or `.with_context()`.

Common error scenarios:
- Accessibility permissions not granted
- Application not found (no fuzzy match within threshold)
- Window not found for application
- Config file parse errors
- Daemon already running / not running

---

## Logging

The daemon logs to stdout with timestamps using `chrono`. Log format:

```
[2024-01-15 10:30:45] Starting daemon...
[2024-01-15 10:30:45] Listening for hotkeys
[2024-01-15 10:30:50] Executing action: focus Slack
```

No log levels or external logging framework; simple `println!` with timestamp prefix.

---

## CI/CD

The project uses GitHub Actions for continuous integration and deployment.

### Release Strategy

- **Dev releases**: Automatic on every push to main
- **Beta releases**: Manual via git tags (`beta-{commit}`)
- **Stable releases**: Manual via git tags (`stable-{commit}`)

### Workflows

Located in `.github/workflows/ci.yml`:

1. **Test Job**: Runs on all PRs and pushes
   - cargo test
   - cargo fmt check
   - cargo clippy

2. **Build Job**: Creates release binaries
   - Builds for both x86_64 and aarch64 macOS
   - Strips debug symbols for smaller size
   - Creates tar.gz archives with checksums

3. **Release Job**: Publishes to GitHub Releases
   - Auto-generates release notes from commits
   - Includes link to full diff
   - Cleans up old dev releases (keeps last 3)

### Creating Releases

```bash
# Dev release (automatic on push to main)
git push origin main

# Beta release
./scripts/release-beta.sh

# Stable release
./scripts/release-stable.sh

# List all releases
./scripts/list-releases.sh
```

### Tag Format

- Beta: `beta-{8-char-commit-hash}` (e.g., `beta-a3f2b1c4`)
- Stable: `stable-{8-char-commit-hash}` (e.g., `stable-a3f2b1c4`)
- Deprecated: `deprecated-{8-char-commit-hash}` (marks bad releases)
