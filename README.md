# cwm - Cool Window Manager

A macOS window manager with CLI and global hotkeys. Manage windows by app name with fuzzy matching.

**Website:** [cwm.taulfsime.com](https://cwm.taulfsime.com) - Interactive demo and documentation

## Installation

### Homebrew (Recommended)

```bash
brew tap taulfsime/cwm https://github.com/taulfsime/cool-window-manager
brew install cwm
```

### Quick Install Script

```bash
# Install latest stable release
curl -fsSL https://raw.githubusercontent.com/taulfsime/cool-window-manager/main/install.sh | sh

# Install specific channel
curl -fsSL https://raw.githubusercontent.com/taulfsime/cool-window-manager/main/install.sh | sh -s -- --beta
curl -fsSL https://raw.githubusercontent.com/taulfsime/cool-window-manager/main/install.sh | sh -s -- --dev
```

### Build from Source

```bash
# Prerequisites: Rust (install via rustup.rs)
git clone https://github.com/taulfsime/cool-window-manager.git
cd cool-window-manager

# Build release binary
cargo build --release

# Install to PATH
./target/release/cwm install
```

## Documentation

cwm includes a man page that is installed automatically with the binary.

```bash
man cwm
```

The man page is installed to `/usr/local/share/man/man1/cwm.1` and is updated automatically when you run `cwm update`.

## Shell Completions

cwm supports tab completion for Bash, Zsh, and Fish shells.

### Automatic Installation

Completions are installed automatically when you run `cwm install`. You'll be prompted to choose your shell.

```bash
cwm install                      # prompts for completions
cwm install --completions        # auto-detect shell and install
cwm install --completions=zsh    # install for specific shell
cwm install --completions=all    # install for all shells
cwm install --no-completions     # skip completions
```

### Manual Installation

If you need to install completions separately (e.g., after building from source):

```bash
# install completions only (after binary is already installed)
cwm install --completions-only --completions=zsh
cwm install --completions-only --completions=bash
cwm install --completions-only --completions=fish
cwm install --completions-only --completions=all
```

### Completion Locations

| Shell | Location |
|-------|----------|
| Zsh | `~/.zsh/completions/_cwm` |
| Bash | `~/.bash_completion.d/cwm` |
| Fish | `~/.config/fish/completions/cwm.fish` |

### Enabling Completions

After installation, you may need to enable completions in your shell:

**Zsh** - Add to `~/.zshrc`:
```bash
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

**Bash** - Add to `~/.bashrc`:
```bash
[ -f ~/.bash_completion.d/cwm ] && source ~/.bash_completion.d/cwm
```

**Fish** - Completions are auto-loaded. Just open a new terminal.

### Updating Completions

Completions are automatically refreshed when you run `cwm update`. They are also removed when you run `cwm uninstall`.

## Permissions

cwm requires Accessibility permissions to control windows.

```bash
# Check permission status
cwm check-permissions

# Prompt to grant permissions
cwm check-permissions --prompt
```

Then go to **System Settings > Privacy & Security > Accessibility** and enable your terminal app.

## Updating

cwm can automatically check for and install updates.

### Manual Update

```bash
# Check for updates
cwm update --check

# Install available updates
cwm update

# Force update even if on latest
cwm update --force
```

### Automatic Updates

Configure automatic update behavior in `~/.cwm/config.json`:

```bash
# Enable automatic updates
cwm config set settings.update.auto_update always

# Set to prompt before updating (default)
cwm config set settings.update.auto_update prompt

# Disable automatic updates
cwm config set settings.update.auto_update never
```

### Release Channels

cwm has three release channels:

- **stable**: Well-tested releases (default)
- **beta**: Preview features, generally stable
- **dev**: Latest features, may be unstable

Configure which channels to follow:

```bash
# Enable beta channel (also receives stable updates)
cwm config set settings.update.channels.beta true

# Enable dev channel (receives all updates)
cwm config set settings.update.channels.dev true
```

### Update Frequency

```bash
# Set update check frequency
cwm config set settings.update.check_frequency daily   # Default
cwm config set settings.update.check_frequency weekly
cwm config set settings.update.check_frequency manual
```

## Commands

### Quick Reference

| Command | Description |
|---------|-------------|
| `cwm focus --app <name>` | Focus an application window |
| `cwm maximize [--app <name>]` | Maximize a window to fill the screen |
| `cwm move [--to <position>] [--display <target>] [--app <name>]` | Move a window to a position and/or display |
| `cwm resize --to <size> [--app <name>]` | Resize a window to a target size |
| `cwm list <resource>` | List resources (apps, displays, aliases, events) |
| `cwm check-permissions` | Check accessibility permissions |
| `cwm config <subcommand>` | Manage configuration |
| `cwm events <subcommand>` | Subscribe to window events |
| `cwm daemon <subcommand>` | Manage background daemon |
| `cwm spotlight <subcommand>` | Manage Spotlight integration |
| `cwm record <shortcut\|layout>` | Record keyboard shortcuts or window layouts |
| `cwm install` | Install cwm to system PATH |
| `cwm uninstall` | Remove cwm from system |
| `cwm update` | Update to latest version |
| `cwm version` | Display version information |

### version

Display version information.

```bash
cwm version
# Output: cwm a3f2b1c4 (stable, 2024-02-11)
```

### install

Install cwm to system PATH.

```bash
cwm install                      # Interactive installation
cwm install --path ~/.local/bin  # Specific directory
cwm install --force              # Overwrite existing
cwm install --completions        # Auto-detect shell and install completions
cwm install --completions=zsh    # Install completions for specific shell
cwm install --no-completions     # Skip completion installation
cwm install --completions-only --completions=zsh  # Only install completions
```

### uninstall

Remove cwm from system.

```bash
cwm uninstall
cwm uninstall --path ~/.local/bin  # From specific directory
```

### update

Update cwm to latest version.

```bash
cwm update                   # Update to latest
cwm update --check           # Check only, don't install
cwm update --force           # Force reinstall
cwm update --prerelease      # Include beta/dev releases
```

### focus

Bring an application window to the foreground.

```bash
cwm focus --app Safari
cwm focus --app slck             # fuzzy matching by name
cwm focus --app "New Tab"        # match by window title
cwm focus --app "GitHub - cool"  # match by title prefix
cwm focus --app '/^Google/'      # regex: apps starting with "Google"
cwm focus --app '/chrome|safari/i'  # regex: any browser (case-insensitive)
cwm focus --app Chrome --verbose
cwm focus --app Safari --app Chrome  # try Safari first, fallback to Chrome
cwm focus -a Safari -a Chrome        # short form
```

Options:
- `--app, -a <NAME>` - Target app name or window title (required, repeatable, supports fuzzy and regex matching)
- `--launch` - Launch first app if none found
- `--no-launch` - Never launch app
- `--verbose, -v` - Show matching details

When multiple `--app` flags are provided, cwm tries each in order and focuses the first one found.

### maximize

Maximize a window to fill the screen (excluding menu bar and dock).

```bash
cwm maximize                     # maximize focused window
cwm maximize --app "Finder"      # maximize specific app
cwm maximize --app "Safari" -v   # verbose output
```

Options:
- `--app, -a <NAME>` - Target app name (optional, uses focused window if omitted)
- `--launch` - Launch app if not running
- `--no-launch` - Never launch app
- `--verbose, -v` - Show details

### move

Move a window to a specific position and/or another display.

```bash
# position only (on current display)
cwm move --to top-left           # move to top-left corner
cwm move --to top-right          # move to top-right corner
cwm move --to bottom-left        # move to bottom-left corner
cwm move --to bottom-right       # move to bottom-right corner
cwm move --to left               # move to left edge (vertically centered)
cwm move --to right              # move to right edge (vertically centered)
cwm move --to 50%,50%            # center window (window center at 50%,50%)
cwm move --to 100px,200px        # absolute position in pixels
cwm move --to 100pt,200pt        # absolute position in points
cwm move --to +100,-50           # relative movement from current position

# display only (keeps relative position)
cwm move --display next          # move to next display
cwm move --display prev          # move to previous display
cwm move --display 1             # move to display index 1 (1-based)
cwm move --display external      # move to display alias "external"

# combined position and display
cwm move --to top-left --display next
cwm move --to 50%,50% --display 2 --app "Terminal"
```

Options:
- `--to, -t <POSITION>` - Target position: anchor, percentage, pixels, points, or relative
- `--display, -d <TARGET>` - Display target: `next`, `prev`, index (1-based), or alias name
- `--app, -a <NAME>` - Target app name (optional, uses focused window if omitted)
- `--launch` - Launch app if not running
- `--no-launch` - Never launch app
- `--verbose, -v` - Show details

**Note:** At least one of `--to` or `--display` is required.

#### Display Aliases

Use display aliases to reference monitors by name instead of index. This is useful when you have different monitor setups at home and office.

**System Aliases** (automatically available):
- `builtin` - Built-in display (e.g., MacBook screen)
- `external` - Any external monitor
- `main` - Primary display
- `secondary` - Secondary display

**User-Defined Aliases** - Map custom names to specific monitors in config:

```json
{
  "display_aliases": {
    "office_main": ["10AC_D0B3_67890"],
    "home_external": ["1E6D_5B11_12345", "10AC_D0B3_67890"]
  },
  "shortcuts": [
    {
      "keys": "ctrl+alt+e",
      "action": "move:external"
    },
    {
      "keys": "ctrl+alt+o",
      "action": "move:office_main"
    }
  ]
}
```

Find your display's unique ID:

```bash
cwm list displays --json --detailed
# Returns JSON with unique_id field for each display

cwm list aliases
# Shows all available aliases and their current resolution
```

### resize

Resize a window to a target size (centered on screen).

```bash
# Percentage (all equivalent)
cwm resize --to 80               # 80% of screen
cwm resize --to 80%              # explicit percentage
cwm resize --to 0.8              # decimal (0.8 = 80%)
cwm resize -t 80                 # short form

# Full screen
cwm resize --to full             # 100% (same as maximize)

# Pixels
cwm resize --to 1920px           # 1920 pixels wide (height auto)
cwm resize --to 1920x1080px      # exact pixel dimensions

# Points (macOS native units)
cwm resize --to 800pt            # 800 points wide (height auto)
cwm resize --to 800x600pt        # exact point dimensions

# With app targeting
cwm resize --to 80 --app Safari
cwm resize -t 1920px -a Chrome

# Allow overflow beyond screen bounds
cwm resize --to 2500px --overflow
```

Options:
- `--to, -t <SIZE>` - Target size (required). Formats:
  - Percentage: `80`, `80%`, `0.8` (decimal)
  - Keyword: `full` (100%)
  - Pixels: `1920px` (width only), `1920x1080px` (exact)
  - Points: `800pt` (width only), `800x600pt` (exact)
- `--app, -a <NAME>` - Target app name (optional, uses focused window if omitted)
- `--overflow` - Allow window to extend beyond screen bounds (default: clamp to screen)
- `--launch` - Launch app if not running
- `--no-launch` - Never launch app
- `--verbose, -v` - Show details

When using width-only pixel/point values, height is calculated to maintain the display's aspect ratio.

### list

List resources (apps, displays, or aliases).

```bash
cwm list apps                    # list running applications
cwm list displays                # list available displays
cwm list aliases                 # list display aliases
cwm list events                  # list available event types

# JSON output
cwm list apps --json             # basic JSON (name, pid)
cwm list apps --json --detailed  # full JSON (includes bundle_id, titles)

cwm list displays --json         # basic JSON (index, name, resolution, is_main)
cwm list displays --json --detailed  # full JSON (includes vendor, model, unique_id)

cwm list aliases --json          # basic JSON (name, type, resolved, display_index)
cwm list aliases --json --detailed   # full JSON (includes display_name, mapped_ids)

# Scriptable output (no jq needed)
cwm list apps --names            # one name per line
cwm list apps --format '{name} ({pid})'  # custom format
cwm list displays --format '{index}: {name} ({width}x{height})'
```

Resources:
- `apps` - Running applications with their window titles
- `displays` - Available displays with resolution and position
- `aliases` - Display aliases (system: builtin, external, main, secondary; and user-defined)
- `events` - Available event types for subscription

Options:
- `--json` - Output in JSON format
- `--names` - Output one name per line (ideal for piping to fzf/xargs)
- `--format <TEMPLATE>` - Custom output format using `{field}` placeholders
- `--detailed, -d` - Include additional fields in output

### get

Get information about windows.

```bash
cwm get focused                  # info about focused window
cwm get window --app Safari      # info about specific app's window
cwm get window --app '/safari/i' --app '/chrome/i'  # try Safari, fallback to Chrome

# Custom format output
cwm get focused --format '{app.name}'           # just the app name
cwm get focused --format '{window.width}x{window.height}'  # dimensions
cwm get window --app Safari --format '{display.name}'      # which display
```

Subcommands:
- `focused` - Get info about the currently focused window
- `window --app <NAME>` - Get info about a specific app's window (repeatable, tries each in order)

Options:
- `--app, -a <NAME>` - Target app name (required for `window`, repeatable, supports fuzzy and regex matching)
- `--format <TEMPLATE>` - Custom output format using `{field}` placeholders

Available fields for format:
- `app.name`, `app.pid`, `app.bundle_id`
- `window.title`, `window.x`, `window.y`, `window.width`, `window.height`
- `display.index`, `display.name`

### events

Subscribe to real-time window manager events. Requires the daemon to be running.

```bash
# stream all events
cwm events listen

# filter by event type (glob patterns)
cwm events listen --event "app.*"
cwm events listen --event "window.*"

# filter by app name (supports regex)
cwm events listen --app Safari
cwm events listen --app '/chrome|safari/i'

# wait for specific event
cwm events wait --event app.focused --app Safari
cwm events wait --event window.maximized --timeout 30

# scripting: run command when app is focused
cwm events wait --event app.focused --app Slack && say "Slack focused"
```

Subcommands:
- `listen` - Stream events to stdout (JSON, one per line)
- `wait` - Block until a specific event occurs

#### events listen

Stream events continuously until interrupted (Ctrl+C).

Options:
- `--event <PATTERN>` - Filter by event type using glob patterns (e.g., `"app.*"`, `"window.moved"`)
- `--app <NAME>` - Filter by app name (repeatable, supports regex matching)

Output format (JSON, one event per line):
```json
{"event":"app.focused","ts":1707900000000,"app":"Safari","pid":1234}
{"event":"window.resized","ts":1707900001000,"app":"Safari","width":1200,"height":800}
```

#### events wait

Block until a matching event occurs, then exit.

Options:
- `--event <TYPE>` - Event type to wait for (required, e.g., `app.focused`, `window.maximized`)
- `--app <NAME>` - Filter by app name (repeatable, supports regex matching)
- `--timeout <SECONDS>` - Timeout in seconds (default: no timeout)
- `--quiet` - No output on success, exit code only

Exit codes:
- `0` - Event received
- `1` - Error
- `8` - Timeout
- `9` - Daemon not running

#### Event Types

| Event | Description | Data Fields |
|-------|-------------|-------------|
| `app.launched` | App launched (via app_rules) | `app`, `pid` |
| `app.focused` | App window focused | `app`, `pid`, `match_type` |
| `window.maximized` | Window maximized | `app`, `pid` |
| `window.resized` | Window resized | `app`, `pid`, `width`, `height` |
| `window.moved` | Window moved | `app`, `pid`, `x`, `y`, `display_index`, `display_name` |
| `display.connected` | Display connected | `index`, `name`, `unique_id`, `width`, `height`, `x`, `y`, `is_main`, `is_builtin`, `aliases` |
| `display.disconnected` | Display disconnected | `index`, `name`, `unique_id`, `width`, `height`, `x`, `y`, `is_main`, `is_builtin`, `aliases` |

All events include:
- `event` - Event type string
- `ts` - Unix timestamp in milliseconds

### config

Manage configuration.

```bash
cwm config show                  # show current config
cwm config path                  # show config file path
cwm config set <key> <value>     # set a value
cwm config reset                 # reset to defaults
cwm config default               # show default config with examples
cwm config verify                # verify config for errors
```

Config keys:
- `settings.launch` - Launch apps if not running (true/false)
- `settings.animate` - Animate window movements (true/false)
- `settings.fuzzy_threshold` - Levenshtein distance threshold (default: 2)
- `settings.delay_ms` - Default delay before app rule actions (default: 500)
- `settings.retry.count` - Number of retry attempts (default: 10)
- `settings.retry.delay_ms` - Initial retry delay in milliseconds (default: 100)
- `settings.retry.backoff` - Backoff multiplier for each retry (default: 1.5)
- `settings.update.enabled` - Enable update checking (true/false)
- `settings.update.check_frequency` - Update check frequency (daily/weekly/manual)
- `settings.update.auto_update` - Auto-update mode (always/prompt/never)
- `settings.update.channels.dev` - Enable dev channel (true/false)
- `settings.update.channels.beta` - Enable beta channel (true/false)
- `settings.update.channels.stable` - Enable stable channel (true/false)

### record

Record keyboard shortcuts or window layouts for use in config.

#### record shortcut

Record a keyboard shortcut:

```bash
cwm record shortcut              # just print the keys
cwm record shortcut --action focus --app "Slack"  # save to config
```

#### record layout

Record current window positions and sizes to generate app_rules config:

```bash
cwm record layout                              # record all visible windows
cwm record layout --app Safari --app Chrome    # record specific apps only
cwm record layout --display 1                  # record windows on display 1
cwm record layout --app Safari --display 0     # record Safari if on display 0
```

The command outputs multiple config variants:
- **Full**: position + size rules
- **Position only**: just move rules
- **Display only**: move to display aliases (portable across setups)
- **Size only**: just resize rules

### daemon

Manage the background daemon for global hotkeys.

```bash
cwm daemon start
cwm daemon stop
cwm daemon status
cwm daemon install              # install to run on login
cwm daemon uninstall            # remove from login items
```

Options for `install`:
- `--bin <PATH>` - Path to cwm binary (defaults to current executable)
- `--log <PATH>` - Log file path for the daemon

#### IPC Socket

When running, the daemon exposes a Unix socket at `~/.cwm/cwm.sock` for inter-process communication. This allows external tools to control cwm without spawning new processes.

The IPC uses JSON-RPC 2.0 style format, consistent with the CLI `--json` output. The `"jsonrpc": "2.0"` field is optional:

```bash
# ping the daemon (minimal JSON - jsonrpc field optional)
echo '{"method":"ping","id":1}' | nc -U ~/.cwm/cwm.sock
# {"jsonrpc":"2.0","result":"pong","id":"1"}

# focus an app
echo '{"method":"focus","params":{"app":"Safari"},"id":1}' | nc -U ~/.cwm/cwm.sock
# {"jsonrpc":"2.0","result":{"message":"Focused Safari","app":"Safari"},"id":"1"}

# notification (no response, fire and forget - omit id)
echo '{"method":"focus","params":{"app":"Safari"}}' | nc -U ~/.cwm/cwm.sock

# get daemon status
echo '{"method":"status","id":1}' | nc -U ~/.cwm/cwm.sock | jq .

# list running apps
echo '{"method":"list_apps","id":1}' | nc -U ~/.cwm/cwm.sock | jq '.result.apps[].name'

# plain text protocol (for simple scripting)
echo "focus:Safari" | nc -U ~/.cwm/cwm.sock
# OK
```

Available methods: `ping`, `status`, `focus`, `maximize`, `resize`, `move`, `list_apps`, `list_displays`, `list_events`, `subscribe`, `action`.

#### Event Subscription via IPC

Subscribe to events over a persistent socket connection:

```bash
# subscribe to all events (keeps connection open)
echo '{"method":"subscribe","id":1}' | nc -U ~/.cwm/cwm.sock

# subscribe with filters
echo '{"method":"subscribe","params":{"events":["app.*"],"apps":["Safari"]},"id":1}' | nc -U ~/.cwm/cwm.sock
```

The daemon streams events as JSON-RPC notifications (no `id` field) until the connection closes.

For detailed IPC documentation and examples in Python, Node.js, Ruby, Go, Rust, and Hammerspoon, see [SCRIPTS.md](SCRIPTS.md#ipc-socket).

### spotlight

Manage macOS Spotlight integration. Creates app bundles that appear in Spotlight search.

```bash
cwm spotlight install           # install all configured shortcuts
cwm spotlight install --name "Focus Safari"  # install specific shortcut
cwm spotlight install --force   # overwrite existing shortcuts
cwm spotlight list              # list installed shortcuts
cwm spotlight remove "Focus Safari"  # remove specific shortcut
cwm spotlight remove --all      # remove all cwm shortcuts
cwm spotlight example           # show example configuration
```

Shortcuts appear in Spotlight with a "cwm: " prefix. For example, search for "cwm: Focus Safari" to run the shortcut.

## Scripting

cwm is designed to be easily scriptable and composable with Unix tools. JSON output uses [JSON-RPC 2.0](https://www.jsonrpc.org/specification) format for easy integration.

### Output Modes

| Flag | Description |
|------|-------------|
| (none) | Human-readable when TTY, JSON-RPC when piped |
| `--json` / `-j` | Force JSON-RPC output |
| `--no-json` | Force text output even when piped |
| `--names` | One name per line (for fzf/xargs) |
| `--format` | Custom format with `{field}` placeholders |
| `--quiet` / `-q` | No output on success |

### JSON-RPC Format

```bash
# success response
cwm focus --app Safari --json
# {"jsonrpc":"2.0","result":{"action":"focus","app":{"name":"Safari",...}},"id":null}

# error response
cwm focus --app NonExistent --json
# {"jsonrpc":"2.0","error":{"code":-32002,"message":"...","data":{"suggestions":[...]}},"id":null}

# parse with jq
cwm get focused --json | jq -r '.result.app.name'
```

### Exit Codes

| Code | JSON-RPC | Meaning |
|------|----------|---------|
| 0 | -32000 | Success |
| 1 | -32001 | General error |
| 2 | -32002 | App not found |
| 3 | -32003 | Permission denied |
| 4 | -32004 | Invalid arguments |
| 5 | -32005 | Config error |
| 6 | -32006 | Window not found |
| 7 | -32007 | Display not found |

### Examples

```bash
# pipe app name from stdin
echo "Safari" | cwm focus --app -

# fzf integration (no jq needed)
cwm list apps --names | fzf | xargs cwm focus --app

# check if app is running
cwm focus --app Slack -q && echo "Slack is running"

# custom format for scripting
cwm list apps --format '{name}\t{pid}' | while IFS=$'\t' read name pid; do
  echo "App: $name (PID: $pid)"
done
```

For more scripts and recipes, see [SCRIPTS.md](SCRIPTS.md).

## Global Options

These options can be used with any command:

- `--config <PATH>` - Use a custom config file instead of the default location
- `--json` / `-j` - Force JSON output
- `--no-json` - Force text output even when piped
- `--quiet` / `-q` - Suppress output on success

```bash
cwm --config /path/to/test-config.json focus --app Safari
cwm --config ~/custom.json config show
```

Config path priority:
1. `--config` flag (highest)
2. `CWM_CONFIG` environment variable
3. Default `~/.cwm/config.json`

When using `--config`, the specified file must exist (no auto-creation).

## Configuration

Config file location: `~/.cwm/config.json` or `~/.cwm/config.jsonc`

The config file supports JSONC format (JSON with Comments):
- Single-line comments: `// comment`
- Multi-line comments: `/* comment */`

If both `config.json` and `config.jsonc` exist, an error is raised.

A JSON schema is auto-generated at `~/.cwm/config.schema.json` for editor autocompletion and validation. The config includes a `$schema` field that references this schema.

Example config:

```json
{
  "$schema": "./config.schema.json",
  // global hotkey shortcuts
  "shortcuts": [
    {
      "keys": "ctrl+alt+s",
      "action": "focus",
      "app": "Slack",
      "launch": true
    },
    {
      "keys": "ctrl+alt+c",
      "action": "focus",
      "app": "Google Chrome"
    },
    {
      "keys": "ctrl+alt+g",
      "action": "focus",
      "app": "GitHub"
    },
    {
      "keys": "ctrl+alt+return",
      "action": "maximize"
    },
    {
      "keys": "ctrl+alt+right",
      "action": "move:next"
    },
    {
      "keys": "ctrl+alt+7",
      "action": "resize:75"
    },
    {
      "keys": "ctrl+alt+f",
      "action": "resize:full"
    }
  ],
  "app_rules": [
    {
      "app": "Slack",
      "action": "move:1"
    },
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
      "name": "Focus Chrome",
      "action": "focus",
      "app": "Google Chrome",
      "icon": "/path/to/custom-icon.icns"
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

### Shortcut format

- `keys` - Key combination (e.g., `ctrl+alt+s`, `cmd+shift+return`)
- `action` - One of:
  - `focus` - Focus an application window
  - `maximize` - Maximize window to fill screen
  - `move:<target>` - Move window to position and/or display. Target formats:
    - `move:next` - Move to next display
    - `move:prev` - Move to previous display
    - `move:<index>` - Move to display by index (1-based)
    - `move:<alias>` - Move to display by alias name
    - `move:top-left` - Move to anchor position (top-left, top-right, bottom-left, bottom-right, left, right)
    - `move:50%,50%` - Move window center to percentage position
    - `move:100px,200px` - Move to absolute pixel position
    - `move:+100,-50` - Relative movement from current position
    - `move:top-left;display=next` - Combined position and display (semicolon separates arguments)
  - `resize:<size>` - Resize window. Size formats:
    - `resize:80` - 80% of screen
    - `resize:0.8` - 80% (decimal)
    - `resize:full` - 100%
    - `resize:1920px` - 1920 pixels wide
    - `resize:1920x1080px` - exact pixel dimensions
    - `resize:800pt` - 800 points wide
    - `resize:800x600pt` - exact point dimensions
- `app` - Target app name or window title (optional for maximize/move/resize, fuzzy matched)
- `launch` - Override global setting (optional)

The `app` field matches against both application names and window titles. For example, `"GitHub"` will match a Safari or Chrome window with "GitHub" in its title.

### App rules

App rules automatically apply actions when applications are launched. The daemon watches for new app launches and executes the configured action.

- `app` - Application name to match (case-insensitive, supports prefix matching)
- `action` - Same action format as shortcuts: `maximize`, `move:N`, `resize:N`, etc.
- `delay_ms` - Delay in milliseconds before executing the action (optional, overrides global setting)

The global default delay is set via `settings.delay_ms` (default: 500ms). This delay allows the window to appear before the action is applied.

If the window is not ready after the initial delay, the action will be retried with exponential backoff:
- `settings.retry.count` - Number of retry attempts (default: 10)
- `settings.retry.delay_ms` - Initial retry delay in milliseconds (default: 100)
- `settings.retry.backoff` - Backoff multiplier for each retry (default: 1.5, meaning each retry waits 1.5x longer)

This is useful for automatically moving apps to specific monitors or resizing them when launched.

### Spotlight shortcuts

Spotlight shortcuts create macOS app bundles that appear in Spotlight search. When triggered, they execute cwm commands.

- `name` - Name displayed in Spotlight (prefixed with "cwm: ")
- `action` - Same format as shortcuts: `focus`, `maximize`, `move:next`, `resize:80`
- `app` - Target app name (required for `focus`, optional for others)
- `launch` - Launch app if not running (optional)
- `icon` - Custom icon (optional). Can be:
  - Path to `.icns` file: `/path/to/icon.icns`
  - Path to `.png` file: `~/icons/custom.png` (converted to icns)
  - App name to extract icon from: `Safari`
  - If not specified, uses target app's icon (when `app` is set) or default cwm icon

After adding shortcuts to config, install them with:

```bash
cwm spotlight install
```

Shortcuts are installed to `~/Applications/cwm/` and indexed by Spotlight. Search for "cwm: <name>" to run them.

### App matching

Apps are matched by name and window title with priority:

**Name matching:**
1. Exact match (case-insensitive)
2. Prefix match
3. Regex match (if query is `/pattern/` or `/pattern/i`)
4. Fuzzy match (Levenshtein distance within threshold)

**Title matching (if no name match):**
5. Title exact match
6. Title prefix match
7. Title regex match
8. Title fuzzy match

**Standard examples:**
- `"Slack"` → Slack (name exact)
- `"slck"` → Slack (name fuzzy, distance=1)
- `"Goo"` → Google Chrome (name prefix)
- `"New Tab"` → Google Chrome (title exact)
- `"GitHub - taulfsime"` → Safari (title prefix)

**Regex examples:**
- `"/^Google/"` → Google Chrome (regex name match)
- `"/chrome|safari/i"` → Chrome or Safari (case-insensitive alternation)
- `"/Pull Request #\d+/"` → window with PR number in title

Regex uses JavaScript-style `/pattern/` syntax. Add `/i` suffix for case-insensitive matching. Invalid regex patterns return no match.

## Website

The project has an interactive landing page at [cwm.taulfsime.com](https://cwm.taulfsime.com) featuring:

- **Interactive terminal** - Try cwm commands in your browser
- **Live preview** - Watch animated window manipulations as you type
- **Auto-demo** - See cwm in action immediately on page load
- **Multi-display simulation** - Add up to 4 virtual displays

The site is built with vanilla HTML/CSS/JS (no build step) and deployed automatically via GitHub Pages.

## License

MIT
