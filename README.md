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
cwm focus --app "slck"           # fuzzy matching by name
cwm focus --app "New Tab"        # match by window title
cwm focus --app "GitHub - cool"  # match by title prefix
cwm focus --app "Chrome" --verbose
```

Options:
- `--app, -a <NAME>` - Target app name or window title (required, fuzzy matched)
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

### resize

Resize a window to a percentage of the screen (centered).

```bash
cwm resize 75                    # resize focused window to 75%
cwm resize full                  # resize to 100% (same as maximize)
cwm resize 80 --app "Safari"     # resize specific app to 80%
cwm resize 50 -v                 # verbose output
```

Options:
- `<SIZE>` - Percentage (1-100) or `full` for 100%
- `--app, -a <NAME>` - Target app name (optional, uses focused window if omitted)
- `--launch` - Launch app if not running
- `--no-launch` - Never launch app
- `--verbose, -v` - Show details

### list-apps

List all running applications with their window titles.

```bash
cwm list-apps
```

Output includes app name, PID, bundle ID, and all window titles for each app.

### list-displays

List available displays with their identifiers.

```bash
cwm list-displays                # simple view
cwm list-displays --detailed     # show all identifiers
```

Options:
- `--detailed, -d` - Show detailed information including vendor ID, model ID, serial number, and unique ID

The detailed view shows:
- **Display ID** - macOS display identifier (may change on reconnect)
- **Vendor ID** - Manufacturer ID with known vendor names (Apple, LG, Dell, etc.)
- **Model ID** - Model identifier
- **Serial Number** - Display serial (if available)
- **Unique ID** - Stable identifier for the display (vendor_model_serial or vendor_model_unitN)

### config

Manage configuration.

```bash
cwm config show                  # show current config
cwm config path                  # show config file path
cwm config set <key> <value>     # set a value
cwm config reset                 # reset to defaults
cwm config default               # show default config with examples
```

Config keys:
- `settings.launch` - Launch apps if not running (true/false)
- `settings.animate` - Animate window movements (true/false)
- `settings.fuzzy_threshold` - Levenshtein distance threshold (default: 2)
- `settings.delay_ms` - Default delay before app rule actions (default: 500)
- `settings.retry.count` - Number of retry attempts (default: 10)
- `settings.retry.delay_ms` - Initial retry delay in milliseconds (default: 100)
- `settings.retry.backoff` - Backoff multiplier for each retry (default: 1.5)

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
      "action": "move_display:next"
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
      "action": "move_display:1"
    },
    {
      "app": "Terminal",
      "action": "maximize",
      "delay_ms": 1000
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
    }
  }
}
```

### Shortcut format

- `keys` - Key combination (e.g., `ctrl+alt+s`, `cmd+shift+return`)
- `action` - One of: `focus`, `maximize`, `move_display:next`, `move_display:prev`, `move_display:N`, `resize:N`, `resize:full`
- `app` - Target app name or window title (optional for maximize/move_display/resize, fuzzy matched)
- `launch` - Override global setting (optional)

The `app` field matches against both application names and window titles. For example, `"GitHub"` will match a Safari or Chrome window with "GitHub" in its title.

### App rules

App rules automatically apply actions when applications are launched. The daemon watches for new app launches and executes the configured action.

- `app` - Application name to match (case-insensitive, supports prefix matching)
- `action` - Same action format as shortcuts: `maximize`, `move_display:N`, `resize:N`, etc.
- `delay_ms` - Delay in milliseconds before executing the action (optional, overrides global setting)

The global default delay is set via `settings.delay_ms` (default: 500ms). This delay allows the window to appear before the action is applied.

If the window is not ready after the initial delay, the action will be retried with exponential backoff:
- `settings.retry.count` - Number of retry attempts (default: 10)
- `settings.retry.delay_ms` - Initial retry delay in milliseconds (default: 100)
- `settings.retry.backoff` - Backoff multiplier for each retry (default: 1.5, meaning each retry waits 1.5x longer)

This is useful for automatically moving apps to specific monitors or resizing them when launched.

### Fuzzy matching

Apps are matched by name and window title with priority:

**Name matching:**
1. Exact match (case-insensitive)
2. Prefix match
3. Fuzzy match (Levenshtein distance within threshold)

**Title matching (if no name match):**
4. Title exact match
5. Title prefix match
6. Title fuzzy match

Examples:
- `"Slack"` → Slack (name exact)
- `"slck"` → Slack (name fuzzy, distance=1)
- `"Goo"` → Google Chrome (name prefix)
- `"New Tab"` → Google Chrome (title exact)
- `"GitHub - taulfsime"` → Safari (title prefix)

## License

MIT
