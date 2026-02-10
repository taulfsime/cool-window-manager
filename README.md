# cwm - Cool Window Manager

A macOS window manager with CLI and global hotkeys. Manage windows by app name with fuzzy matching.

## Building

### Prerequisites

- Rust (install via [rustup](https://rustup.rs/))
- macOS (uses Accessibility API)

### Build

```bash
# Debug build
cargo build

# Release build (recommended)
cargo build --release

# The binary will be at ./target/release/cwm
```

### Install

```bash
# Copy to PATH
cp ./target/release/cwm /usr/local/bin/

# Or run directly
./target/release/cwm --help
```

## Permissions

cwm requires Accessibility permissions to control windows.

```bash
# Check permission status
cwm check-permissions

# Prompt to grant permissions
cwm check-permissions --prompt
```

Then go to **System Settings > Privacy & Security > Accessibility** and enable your terminal app.

## Commands

### focus

Bring an application window to the foreground.

```bash
cwm focus --app "Slack"
cwm focus --app "slck"           # fuzzy matching
cwm focus --app "Chrome" --verbose
```

Options:
- `--app, -a <NAME>` - Target app name (required, fuzzy matched)
- `--launch` - Launch app if not running
- `--no-launch` - Never launch app
- `--verbose, -v` - Show matching details

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

### move-display

Move a window to another display.

```bash
cwm move-display next            # move to next display
cwm move-display prev            # move to previous display
cwm move-display 0               # move to display index 0
cwm move-display next --app "Terminal"
```

Options:
- `<TARGET>` - Display target: `next`, `prev`, or index (0-based)
- `--app, -a <NAME>` - Target app name (optional)
- `--launch` - Launch app if not running
- `--no-launch` - Never launch app
- `--verbose, -v` - Show details

### list-windows

List all running applications.

```bash
cwm list-windows
```

### list-displays

List available displays.

```bash
cwm list-displays
```

### config

Manage configuration.

```bash
cwm config show                  # show current config
cwm config path                  # show config file path
cwm config set <key> <value>     # set a value
cwm config reset                 # reset to defaults
```

Config keys:
- `behavior.launch_if_not_running` - Launch apps if not running (true/false)
- `behavior.animate` - Animate window movements (true/false)
- `matching.fuzzy_threshold` - Levenshtein distance threshold (default: 2)

### record-shortcut

Record a keyboard shortcut for use in config.

```bash
cwm record-shortcut              # just print the keys
cwm record-shortcut --action focus --app "Slack"  # save to config
```

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

## Configuration

Config file location: `~/.cwm.json` (or set `CWM_CONFIG` env var)

Example config:

```json
{
  "shortcuts": [
    {
      "keys": "ctrl+alt+s",
      "action": "focus",
      "app": "Slack",
      "launch_if_not_running": true
    },
    {
      "keys": "ctrl+alt+c",
      "action": "focus",
      "app": "Google Chrome"
    },
    {
      "keys": "ctrl+alt+return",
      "action": "maximize"
    },
    {
      "keys": "ctrl+alt+right",
      "action": "move_display:next"
    }
  ],
  "matching": {
    "fuzzy_threshold": 2
  },
  "behavior": {
    "launch_if_not_running": false,
    "animate": false
  }
}
```

### Shortcut format

- `keys` - Key combination (e.g., `ctrl+alt+s`, `cmd+shift+return`)
- `action` - One of: `focus`, `maximize`, `move_display:next`, `move_display:prev`, `move_display:N`
- `app` - Target app name (optional for maximize/move_display)
- `launch_if_not_running` - Override global setting (optional)

### Fuzzy matching

App names are matched with priority:
1. Exact match (case-insensitive)
2. Prefix match
3. Fuzzy match (Levenshtein distance within threshold)

Examples:
- `"Slack"` → Slack (exact)
- `"slck"` → Slack (fuzzy, distance=1)
- `"Goo"` → Google Chrome (prefix)
- `"chr"` → Google Chrome (fuzzy)

## License

MIT
