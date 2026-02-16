# AGENTS.md

> context and guidance for AI coding agents working on this repository

## Important: Running cwm Commands

**Always use a local test config file when running cwm commands.** Never use the user's main config at `~/.cwm/config.json` as it will interfere with their personal setup.

```bash
# create a test config in the project directory (already in .gitignore)
cat > .cwm-test-config.json << 'EOF'
{
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
}
EOF

# always use --config flag when running cwm
./target/debug/cwm --config .cwm-test-config.json list apps
./target/debug/cwm --config .cwm-test-config.json focus --app Safari
```

This prevents:
- modifying the user's shortcuts and app rules
- triggering update checks that print messages
- interfering with the user's daemon configuration

---

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
- **move**: move a window to a position (anchors, coordinates, percentages) and/or another display

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
├── build.rs                # build script for git version info and spotlight stub compilation
├── README.md               # user-facing documentation
├── AGENTS.md               # this file
├── install.sh              # shell installer script
├── .gitignore              # ignores target/ and session- files
├── .github/
│   └── workflows/
│       └── ci.yml          # GitHub Actions CI/CD workflow
├── scripts/
│   ├── ci.sh               # run CI checks locally (test, fmt, clippy)
│   ├── release-beta.sh     # helper script to create beta releases
│   ├── release-stable.sh   # helper script to create stable releases
│   ├── list-releases.sh    # helper script to list all releases
│   ├── coverage.sh         # generate test coverage report
│   ├── manual-test.sh      # interactive manual testing guide
│   └── spotlight_stub.c    # C source for Spotlight app stub executable
├── homebrew/
│   └── cwm.rb              # Homebrew formula (auto-updated by CI on stable releases)
├── docs/                   # landing page (GitHub Pages at cwm.taulfsime.com)
│   ├── index.html          # main page with terminal + preview
│   ├── CNAME               # custom domain configuration
│   ├── css/
│   │   └── style.css       # red theme terminal styling
│   ├── js/
│   │   ├── terminal.js     # terminal emulator core
│   │   ├── commands.js     # command handlers
│   │   ├── preview.js      # mock desktop animations
│   │   └── demo.js         # auto-demo sequence
│   └── assets/
│       └── favicon.svg     # site favicon
├── tests/
│   ├── integration.rs      # integration test entry point
│   └── integration_tests/
│       ├── common.rs       # shared test utilities
│       ├── test_install.rs # install test scenarios
│       ├── test_update.rs  # update test scenarios
│       ├── test_rollback.rs # rollback test scenarios
│       ├── test_channels.rs # channel switching tests
│       ├── test_conditions.rs # conditions system tests
│       └── test_man_page.rs # man page content tests
├── SCRIPTS.md              # scripting recipes and examples
├── CONDITIONS.md           # conditions system documentation
├── examples/               # example configuration files
│   ├── 01-basic-shortcuts.jsonc
│   ├── 02-work-hours-conditions.jsonc
│   ├── 03-multi-monitor-setup.jsonc
│   ├── 04-home-office-locations.jsonc
│   ├── 05-app-state-conditions.jsonc
│   ├── 06-developer-workflow.jsonc
│   ├── 07-night-mode.jsonc
│   ├── 08-fallback-rules.jsonc
│   ├── 09-complex-conditions.jsonc
│   └── 10-full-featured.jsonc
├── man/                    # generated man pages (gitignored)
│   └── cwm.1               # main man page (generated by generate-man)
└── src/
    ├── main.rs             # entry point, update check, delegates to cli::run()
    ├── lib.rs              # library crate exposing modules for auxiliary binaries
    ├── version.rs          # version management (Version, VersionInfo)
    ├── bin/
    │   └── generate-man.rs # man page generator using clap_mangen
    ├── actions/
    │   ├── mod.rs          # unified action layer, execute() dispatcher
    │   ├── command.rs      # Command enum - single source of truth for all commands
    │   ├── context.rs      # ExecutionContext with config and is_cli flag
    │   ├── error.rs        # ActionError with exit codes and suggestions
    │   ├── params.rs       # ActionParams (legacy, deprecated)
    │   ├── parse.rs        # JsonRpcRequest parsing for IPC
    │   ├── result.rs       # ActionResult and response data types
    │   └── handlers/
    │       ├── mod.rs      # handler module exports
    │       ├── common.rs   # shared utilities (resolve_app_target, AppResolution)
    │       ├── focus.rs    # focus action handler
    │       ├── maximize.rs # maximize action handler
    │       ├── resize.rs   # resize action handler
    │       ├── move_window.rs # move action handler
    │       ├── list.rs     # list apps/displays/aliases handlers
    │       ├── get.rs      # get focused/window handlers
    │       ├── system.rs   # ping, status, version, check_permissions handlers
    │       ├── daemon.rs   # daemon status handler (start/stop are CLI-only)
    │       ├── config.rs   # config show/path/verify handlers
    │       ├── spotlight.rs # spotlight list/example handlers
    │       └── install.rs  # update_check handler (install/update are CLI-only)
    ├── cli/
    │   ├── mod.rs          # module exports
    │   ├── commands.rs     # CLI command definitions and execution
    │   ├── convert.rs      # Commands::to_command() conversion to unified Command
    │   ├── exit_codes.rs   # documented exit codes for scripting
    │   ├── output.rs       # output formatting (JSON, text, format strings)
    │   └── events.rs       # CLI handlers for events listen/wait commands
    ├── conditions/
    │   ├── mod.rs          # module exports, public API
    │   ├── types.rs        # Condition, CompareOp, Value, FieldCondition types
    │   ├── parser.rs       # JSON to AST parsing, $ref resolution
    │   ├── eval.rs         # EvalContext, evaluate() function
    │   └── time.rs         # time range and day parsing
    ├── config/
    │   ├── mod.rs          # config loading, saving, value manipulation, JSONC parsing
    │   ├── schema.rs       # Config, Shortcut, AppRule, Settings, UpdateSettings
    │   └── json_schema.rs  # JSON schema definition for editor autocompletion
    ├── daemon/
    │   ├── mod.rs          # daemon lifecycle, action execution, hotkey parsing, IPC handling
    │   ├── hotkeys.rs      # global hotkey recording and listening (CGEventTap)
    │   ├── ipc.rs          # PID file management, Unix socket IPC (JSON-RPC 2.0 only)
    │   ├── launchd.rs      # macOS launchd plist for auto-start
    │   ├── app_watcher.rs  # NSWorkspace notifications for app launches
    │   ├── display_watcher.rs # CoreGraphics display connect/disconnect detection
    │   └── events.rs       # Event enum, EventBus for pub/sub, subscriber management
    ├── display/
    │   └── mod.rs          # display enumeration, multi-monitor targeting
    ├── installer/
    │   ├── mod.rs          # module exports
    │   ├── paths.rs        # installation path detection and validation
    │   ├── install.rs      # binary installation logic
    │   ├── completions.rs  # shell completion generation (bash, zsh, fish)
    │   ├── github.rs       # GitHub API client for releases
    │   ├── update.rs       # update checking and downloading
    │   └── rollback.rs     # safe update with automatic rollback
    ├── spotlight/
    │   ├── mod.rs          # module exports, example config printing
    │   ├── generator.rs    # app bundle generation for Spotlight integration
    │   ├── icons.rs        # icon resolution and extraction from app bundles
    │   └── signing.rs      # ad-hoc code signing for app bundles
    └── window/
        ├── mod.rs          # module exports
        ├── accessibility.rs # Accessibility API permission handling
        ├── manager.rs      # window manipulation (focus, maximize, move, resize)
        └── matching.rs     # fuzzy application name matching (Levenshtein)
```

---

## Module Architecture

### actions/

Unified action layer providing consistent behavior across CLI, IPC, and future HTTP API.

| File | Responsibility |
|------|----------------|
| `mod.rs` | `execute(Command, &ExecutionContext)` dispatcher, module exports |
| `command.rs` | `Command` enum - single source of truth for all commands |
| `context.rs` | `ExecutionContext` with config reference and `is_cli` flag |
| `error.rs` | `ActionError` with exit codes, suggestions, and helper constructors |
| `params.rs` | `ActionParams` enum (legacy, deprecated - use `Command` instead) |
| `parse.rs` | `JsonRpcRequest` struct, `to_command()` for JSON-RPC → Command conversion |
| `result.rs` | `ActionResult`, `ActionData`, `AppData`, `MatchData`, etc. |

**Handler modules** (`handlers/`):

| File | Responsibility |
|------|----------------|
| `common.rs` | `AppResolution` enum, `resolve_app_target()`, `resolve_launch_behavior()` |
| `focus.rs` | focus action - brings app window to foreground |
| `maximize.rs` | maximize action - fills screen |
| `resize.rs` | resize action - percentage, pixels, or points |
| `move_window.rs` | move action - positions window (anchors, coordinates, percentages) and/or moves to another display |
| `list.rs` | list apps/displays/aliases/events |
| `get.rs` | get focused window or specific app's window info |
| `system.rs` | ping, status, version, check_permissions |
| `daemon.rs` | daemon status (start/stop/install are CLI-only) |
| `config.rs` | config show/path/verify/default (set/reset are CLI-only) |
| `spotlight.rs` | spotlight list/example (install/remove are CLI-only) |
| `install.rs` | update_check (install/uninstall/update are CLI-only) |
| `record.rs` | record layout handler (record shortcut is CLI-only) |

**Key types:**

- `Command`: Unified enum for all commands (Focus, Maximize, Resize, Move, List, Get, Ping, Status, Version, CheckPermissions, Record, Daemon, Events, Config, Spotlight, Install, Uninstall, Update)
- `ExecutionContext`: Contains config reference, verbose flag, and `is_cli` flag
- `ActionResult`: Serializable result with action name and typed data
- `ActionError`: Error with exit code, message, and optional suggestions

**Design principles:**

- Commands are defined once in `Command` enum
- All transports (CLI, IPC) parse their input into `Command`
- `app` parameter is always `Vec<String>` (tries each in order)
- Interactive commands return error via IPC (CLI-only)
- `is_cli` flag distinguishes interactive vs non-interactive contexts

### cli/

Handles command-line argument parsing and command execution.

| File | Responsibility |
|------|----------------|
| `mod.rs` | re-exports `run()` and `Cli` |
| `commands.rs` | defines `Cli` struct with clap derive, `Commands` enum, and `run()` function that dispatches to appropriate handlers |
| `convert.rs` | `Commands::to_command()` conversion from CLI args to unified `Command` enum |
| `exit_codes.rs` | exit code constants for scripting (SUCCESS, ERROR, APP_NOT_FOUND, etc.) |
| `output.rs` | output formatting: `OutputMode` enum, JSON-RPC 2.0 response types, `format_template()` for custom format strings |
| `events.rs` | CLI handlers for `events listen` and `events wait` commands |

Commands defined: `focus`, `maximize`, `move`, `resize`, `list`, `get`, `check-permissions`, `record`, `config`, `daemon`, `events`, `version`, `install`, `uninstall`, `update`, `spotlight`

**CLI Command Reference:**

| Command | Usage | Description |
|---------|-------|-------------|
| `focus` | `cwm focus --app <name> [--app <name>...]` | Focus an application window (tries each app in order) |
| `maximize` | `cwm maximize [--app <name>]` | Maximize window to fill screen |
| `move` | `cwm move [--to <position>] [--display <target>] [--app <name>]` | Move window to position and/or display |
| `resize` | `cwm resize --to <size> [--app <name>]` | Resize window (percent, pixels, or points) |
| `list` | `cwm list <apps\|displays\|aliases\|events> [--json] [--names] [--format] [--detailed]` | List resources |
| `get` | `cwm get <focused\|window> [--app <name>] [--format]` | Get window information |
| `check-permissions` | `cwm check-permissions [--prompt]` | Check accessibility permissions |
| `record` | `cwm record <shortcut\|layout> [OPTIONS]` | Record keyboard shortcuts or window layouts |
| `config` | `cwm config <show\|path\|set\|reset\|default\|verify>` | Manage configuration |
| `events` | `cwm events <listen\|wait>` | Subscribe to window events |
| `daemon` | `cwm daemon <start\|stop\|status\|install\|uninstall>` | Manage background daemon |
| `spotlight` | `cwm spotlight <install\|list\|remove\|example>` | Manage Spotlight integration |
| `install` | `cwm install [--path <dir>] [--force] [--completions[=SHELL]] [--no-completions] [--completions-only]` | Install cwm to system PATH |
| `uninstall` | `cwm uninstall [--path <dir>]` | Remove cwm from system |
| `update` | `cwm update [--check] [--force] [--prerelease]` | Update to latest version |
| `version` | `cwm version` | Display version information |

Common flags for window commands: `--launch`, `--no-launch`, `--verbose/-v`

Global flags: `--json/-j`, `--no-json`, `--quiet/-q`, `--config <path>`

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

Handles installation, updates, version management, man page installation, and shell completions.

| File | Responsibility |
|------|----------------|
| `mod.rs` | module exports, `MAN_PAGE` embedded bytes, `MAN_DIR` constant |
| `paths.rs` | installation path detection and validation |
| `install.rs` | binary and man page installation logic (`install_binary()`, `install_man_page()`, `uninstall_binary()`, `uninstall_man_page()`) |
| `completions.rs` | shell completion generation and installation for bash, zsh, fish |
| `update.rs` | update checking and downloading (also refreshes man page and completions) |
| `github.rs` | GitHub API client for releases |
| `rollback.rs` | safe update with automatic rollback (restores man page on rollback) |

Man page installation:
- Installed to `/usr/local/share/man/man1/cwm.1`
- Embedded in binary via `include_bytes!`
- Automatically installed/updated with binary
- Prompts user to continue if installation fails

Shell completion installation:
- Supports bash, zsh, and fish shells
- User locations: `~/.zsh/completions/_cwm`, `~/.bash_completion.d/cwm`, `~/.config/fish/completions/cwm.fish`
- Generated at runtime using `clap_complete`
- Automatically refreshed on `cwm update`
- Removed on `cwm uninstall`

### version.rs

Standalone module for version information.

- `Version` struct with commit hash, timestamp, channel, and CalVer semver
- `VersionInfo` for persistent version tracking at `~/.cwm/version.json`
  - Includes `schema_version` field to track when JSON schema was last generated
- Build-time version embedding via `build.rs`

**CalVer Versioning:**

cwm uses Calendar Versioning (CalVer) with the format `YYYY.M.D+{metadata}`:

| Channel | Format | Example |
|---------|--------|---------|
| Stable | `YYYY.M.D+{commit}` | `2026.2.14+a3f2b1c4` |
| Beta | `YYYY.M.D+beta.{commit}` | `2026.2.14+beta.a3f2b1c4` |
| Dev | `YYYY.M.D+dev.{commit}` | `2026.2.14+dev.a3f2b1c4` |

- Date is derived from the commit timestamp (not build time)
- Commit hash ensures uniqueness for multiple releases per day
- Build metadata (after `+`) is ignored for version precedence per semver spec
- Version comparison still uses timestamp internally for reliability

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
| `mod.rs` | `start_daemon()`, `stop_daemon()`, `daemon_status()`, action execution, modifier/key parsing, IPC request handling |
| `hotkeys.rs` | `record_shortcut()`, `listen_for_hotkeys()` using CGEventTap |
| `ipc.rs` | PID file management, Unix socket IPC (`IpcRequest`, `IpcResponse`, `send_request()`, `send_command()`) |
| `launchd.rs` | `install_launchd()`, `uninstall_launchd()` for auto-start |
| `app_watcher.rs` | `AppWatcher` struct, NSWorkspace notification observer |
| `display_watcher.rs` | `start_watching()`, `stop_watching()`, CoreGraphics display reconfiguration callbacks |
| `events.rs` | `Event` enum, `EventBus` for pub/sub, `Subscriber` management, event filtering |

The daemon uses:
- PID file at `/tmp/cwm.pid` for single-instance enforcement
- Unix socket at `~/.cwm/cwm.sock` for IPC
- Signal handlers (SIGTERM, SIGINT) for graceful shutdown
- Tokio async runtime for concurrent hotkey and app watching

**IPC Protocol:**

The daemon exposes a Unix socket for inter-process communication, allowing external tools to control cwm without spawning new processes.

- Socket location: `~/.cwm/cwm.sock`
- Protocol: JSON-RPC 2.0 (the `"jsonrpc": "2.0"` field is optional for convenience)
- All requests must be JSON format
- Available methods: all commands from `Command` enum (focus, maximize, resize, move, list, get, ping, status, version, config, subscribe, etc.)

**Request format:**
```json
{"method": "focus", "params": {"app": ["Safari", "Chrome"]}, "id": 1}
```

**Response format (JSON-RPC 2.0):**
```json
{"jsonrpc": "2.0", "result": {...}, "id": "1"}
```

**Notifications:** Requests without `id` are treated as notifications (no response sent).

**Key differences from CLI:**
- `app` parameter is always an array: `{"app": ["Safari"]}` not `{"app": "Safari"}`
- Single string is accepted and converted to single-element array for convenience
- Interactive commands (record_shortcut, install prompts) return errors via IPC

Key types in `ipc.rs`:
- `IpcRequest`: Parsed request with method, params, and optional id
- `format_success_response()`: Format JSON-RPC success response
- `format_error_response()`: Format JSON-RPC error response
- `send_jsonrpc()`: Send JSON-RPC 2.0 request to daemon

### display/

Multi-monitor support and display targeting.

| File | Responsibility |
|------|----------------|
| `mod.rs` | `list_displays()`, `DisplayInfo` struct, `DisplayTarget` enum, `resolve_target_display()` |

Display targets: `next`, `prev`, or numeric index (1-based). Wraps around at boundaries.

### conditions/

Condition evaluation system for conditional shortcut and app rule execution.

| File | Responsibility |
|------|----------------|
| `mod.rs` | module exports, public API |
| `types.rs` | `Condition`, `CompareOp`, `Value`, `FieldCondition` enums/structs |
| `parser.rs` | JSON to AST parsing, operator forms, `$ref` resolution, `ConditionDefinitions` |
| `eval.rs` | `EvalContext`, `evaluate()` function, field evaluators |
| `time.rs` | time range parsing (including overnight), day specs |

**Key types:**

- `Condition`: Enum with variants `All`, `Any`, `Not`, `Field`, `Ref`
- `FieldCondition`: Field name, comparison operator, and value
- `CompareOp`: `Eq`, `Ne`, `Gt`, `Gte`, `Lt`, `Lte`, `In`
- `Value`: `String`, `Number`, `Float`, `Bool`, `List`
- `EvalContext`: Runtime context with displays, running apps, focused app, target window state
- `WindowState`: Window state info (title, display, fullscreen, minimized)

**Supported condition fields:**

| Field | Type | Description |
|-------|------|-------------|
| `time` | string | Time range(s): `"9:00-17:00"`, `"22:00-06:00"` (overnight) |
| `time.day` | string | Day(s): `"mon"`, `"mon-fri"`, `"mon,wed,fri"` |
| `display.count` | number | Number of connected displays |
| `display.connected` | string | Display alias is connected |
| `app` | string | Target app name/title match |
| `app.running` | string | Check if app is running |
| `app.focused` | string/bool | Which app has focus |
| `app.fullscreen` | bool | Target window fullscreen state |
| `app.minimized` | bool | Target window minimized state |
| `app.display` | string | Which display target window is on |

**Integration points:**

- `daemon/mod.rs`: `check_shortcut_condition()`, `check_app_rule_condition()`
- `config/schema.rs`: `conditions` field on `Config`, `when` field on `Shortcut` and `AppRule`

### window/

Core window manipulation using macOS Accessibility API.

| File | Responsibility |
|------|----------------|
| `mod.rs` | re-exports key types and functions |
| `accessibility.rs` | `check_accessibility_permissions()`, `prompt_accessibility_permissions()` |
| `manager.rs` | `focus_app()`, `maximize_app()`, `move_to_display()`, `resize_app()`, `launch_app()` |
| `matching.rs` | `find_app()`, `get_running_apps()`, `get_window_titles()`, `AppInfo`, `MatchType` enum, Levenshtein distance calculation |

### docs/

Landing page hosted on GitHub Pages at [cwm.taulfsime.com](https://cwm.taulfsime.com).

| File | Responsibility |
|------|----------------|
| `index.html` | main HTML structure with terminal and preview panes |
| `CNAME` | custom domain configuration for GitHub Pages |
| `package.json` | npm scripts for local development |
| `css/style.css` | red theme, terminal styling, responsive layout, animations |
| `js/terminal.js` | terminal emulator (input handling, history, output rendering, sounds) |
| `js/commands.js` | command registry and handlers (help, install, cwm subcommands) |
| `js/preview.js` | mock desktop with animated windows, multi-display support |
| `js/demo.js` | auto-demo sequence that runs on page load |
| `assets/favicon.svg` | red terminal icon |

**Features:**
- Interactive terminal emulator with command history and tab completion
- Mock macOS desktop preview showing window manipulations in real-time
- Auto-demo on page load showcasing focus, maximize, move, resize
- Dynamic display management (2-4 displays with adaptive grid layout)
- Red color theme with syntax highlighting
- Optional keyboard sound effects
- Fully responsive (stacked layout on mobile)
- No build step required (vanilla HTML/CSS/JS)

**Local development:**
```bash
cd docs
npm run dev  # starts server at http://localhost:3000
```

**Deployment:**
- Automatic via GitHub Actions on push to main
- Deploys `docs/` folder to GitHub Pages
- Custom domain configured via `CNAME` file

---

## Key Concepts

### App Matching

Applications are matched by name and window title with the following priority:

1. **Name exact match** (case-insensitive): `"safari"` matches `"Safari"`
2. **Name prefix match**: `"saf"` matches `"Safari"`
3. **Name regex match**: `/^Google/` matches `"Google Chrome"`
4. **Name fuzzy match**: Levenshtein distance within threshold (default: 2)
5. **Title exact match**: `"New Tab"` matches Chrome window with that title
6. **Title prefix match**: `"GitHub - taulfsime"` matches window starting with that
7. **Title regex match**: `/GitHub.*PR/` matches window title containing "GitHub" and "PR"
8. **Title fuzzy match**: Levenshtein distance within threshold

Window titles are retrieved via the Accessibility API (`AXTitle` attribute) for all windows of each running application. The `AppInfo` struct contains a `titles: Vec<String>` field with all window titles.

The fuzzy threshold is configurable via `settings.fuzzy_threshold` in config.

#### Regex Matching

Regex patterns use JavaScript-style `/pattern/` syntax:

- `/Safari.*/` - case-sensitive regex matching
- `/chrome/i` - case-insensitive regex (with `i` flag)
- `/^Google/` - anchored pattern (must start with "Google")
- `/Safari|Chrome|Firefox/i` - alternation (match any browser)

Invalid regex patterns return no match (fail silently).

Examples:
```bash
cwm focus --app '/^Google/'           # apps starting with "Google"
cwm focus --app '/chrome|safari/i'    # any browser (case-insensitive)
cwm focus --app '/Pull Request #\d+/' # PR windows by title
```

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

### Man Page Generation

The man page (`man/cwm.1`) is generated by `cargo run --bin generate-man` and embedded into the binary via `include_bytes!`.

- `build.rs` creates an empty placeholder if the file doesn't exist (allows initial compilation)
- CI generates the real man page before running tests
- Local dev: run `cargo run --bin generate-man` to generate the man page, or tests will skip gracefully
- The `man/` directory is in `.gitignore` (not committed)

```bash
# generate man page locally
cargo run --bin generate-man

# view generated man page
man man/cwm.1
```

### Test

```bash
# run all CI checks locally (test, fmt, clippy)
./scripts/ci.sh

# run all tests (unit + integration)
cargo test

# run only unit tests
cargo test --lib

# run only integration tests
cargo test --test integration

# generate coverage report
./scripts/coverage.sh
```

**Important:** Always run `./scripts/ci.sh` before pushing to ensure CI will pass. This runs the same checks as GitHub Actions:
- `cargo test --all-features`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`

#### Unit Tests

Tests are located in `#[cfg(test)]` modules within:

| File | Test Coverage |
|------|---------------|
| `src/actions/command.rs` | Command::is_interactive(), ListResource parsing, method_name() |
| `src/actions/parse.rs` | JSON-RPC parsing, to_command() for all command types, app array handling |
| `src/cli/commands.rs` | ListResource enum parsing, JSON serialization structs, CLI argument parsing, helper functions (resolve_app_name, match_result_to_data, app_info_to_data) |
| `src/cli/convert.rs` | Commands::to_command() conversion for all CLI commands |
| `src/config/mod.rs` | config value parsing, key setting, JSONC parsing, multi-extension support, config path override |
| `src/config/schema.rs` | `should_launch` priority logic, update settings serialization, `$schema` field |
| `src/config/json_schema.rs` | schema validity, required definitions, file writing |
| `src/daemon/ipc.rs` | JSON-RPC request parsing, response formatting, notification handling |
| `src/display/mod.rs` | display target parsing, resolution with wraparound |
| `src/window/matching.rs` | name matching (exact, prefix, fuzzy), title matching (exact, prefix, fuzzy) |
| `src/window/manager.rs` | ResizeTarget parsing, find_display_for_point, WindowData/DisplayDataInfo serialization |
| `src/daemon/mod.rs` | parse_shortcuts, find_shortcut_launch, handle_ipc_request |
| `src/daemon/events.rs` | Event serialization, EventBus pub/sub, subscriber filtering, display events |
| `src/daemon/display_watcher.rs` | compute_aliases_for_display, system/user alias matching |
| `src/daemon/hotkeys.rs` | hotkey string parsing, keycode conversion, modifier extraction |
| `src/daemon/launchd.rs` | plist generation |
| `src/daemon/ipc.rs` | IPC request parsing, response formatting |
| `src/version.rs` | version parsing, comparison, serialization |
| `src/installer/paths.rs` | path detection, writability checks, PATH detection |
| `src/installer/github.rs` | release parsing, architecture detection |
| `src/installer/update.rs` | checksum verification |
| `src/spotlight/generator.rs` | shell escaping, Info.plist generation, shell script generation |

#### Integration Tests

Integration tests for CLI commands, install, update, and rollback:

| File | Test Coverage |
|------|---------------|
| `tests/integration_tests/test_cli_config.rs` | config show/path/set/reset/default/verify commands |
| `tests/integration_tests/test_cli_focus.rs` | focus command with various flags |
| `tests/integration_tests/test_cli_resize.rs` | resize command with various size formats |
| `tests/integration_tests/test_cli_maximize.rs` | maximize command with various flags |
| `tests/integration_tests/test_cli_move.rs` | move command with various positions and display targets |
| `tests/integration_tests/test_cli_get.rs` | get focused/window commands |
| `tests/integration_tests/test_cli_daemon.rs` | daemon status/stop commands |
| `tests/integration_tests/test_cli_version.rs` | version command |
| `tests/integration_tests/test_spotlight.rs` | spotlight install/list/remove/example commands |
| `tests/integration_tests/test_completions.rs` | shell completion installation for bash, zsh, fish |
| `tests/integration_tests/test_list.rs` | list apps/displays/aliases with various output formats |
| `tests/integration_tests/test_install.rs` | fresh install, directory creation, permissions, force overwrite |
| `tests/integration_tests/test_update.rs` | version checking, download, checksum verification, binary replacement |
| `tests/integration_tests/test_rollback.rs` | backup creation, test failure rollback, checksum mismatch, corrupt download |
| `tests/integration_tests/test_channels.rs` | dev/beta/stable channels, channel priority, upgrade paths |
| `tests/integration_tests/test_man_page.rs` | man page existence, sections (NAME, SYNOPSIS, DESCRIPTION, SUBCOMMANDS), command documentation |
| `tests/integration_tests/test_events.rs` | events listen/wait commands, list events, event filtering |

Note: Some integration tests require a mock GitHub API server and are skipped when not available.

Note: Man page tests skip gracefully if the man page hasn't been generated (run `cargo run --bin generate-man` to generate it locally).

#### Manual Tests

Interactive testing guide for visual verification:

```bash
./scripts/manual-test.sh
```

This script guides you through testing features that require visual verification:
- Focus command (app switching)
- Maximize command (window fills screen)
- Resize command (various sizes)
- Move-display command (multi-monitor)
- Get window info
- Spotlight integration

#### Test Coverage

Current coverage: ~56% overall

Files with macOS API dependencies (0% coverage, acceptable):
- `src/window/accessibility.rs` - Pure Accessibility API calls
- `src/daemon/app_watcher.rs` - NSWorkspace notifications
- `src/installer/install.rs` - File system operations with sudo
- `src/installer/rollback.rs` - File system operations
- `src/main.rs` - Entry point
- `src/spotlight/mod.rs` - Entry point

### Run

```bash
# check permissions first
cwm check-permissions --prompt

# direct commands
cwm focus --app Safari
cwm focus --app Safari --app Chrome  # try Safari, fallback to Chrome
cwm maximize
cwm resize --to 80
cwm move --display next

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
      "action": "move:external"
    },
    {
      "keys": "ctrl+alt+o",
      "action": "move:office"
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
      "action": "move:next"
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
  "action": "move:external",
  "action": "move:office_main"
}
```

### Shortcut Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `keys` | string | yes | hotkey combination (e.g., `"ctrl+alt+s"`) |
| `action` | string | yes | one of: `focus`, `maximize`, `resize`, `move:<target>` |
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
| `action` | string | yes | same format as shortcuts: `focus`, `maximize`, `move:next`, `resize:80` |
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
