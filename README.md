# cwm - Cool Window Manager

A macOS window manager with CLI and global hotkeys. Manage windows by app name with fuzzy matching.

**Website:** [cwm.taulfsime.com](https://cwm.taulfsime.com) - Interactive demo and documentation

## Installation

### Quick Install (Recommended)

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
| `cwm move-display <target> [--app <name>]` | Move a window to another display |
| `cwm resize --to <size> [--app <name>]` | Resize a window to a target size |
| `cwm list <resource>` | List resources (apps, displays, aliases) |
| `cwm check-permissions` | Check accessibility permissions |
| `cwm config <subcommand>` | Manage configuration |
| `cwm daemon <subcommand>` | Manage background daemon |
| `cwm spotlight <subcommand>` | Manage Spotlight integration |
| `cwm record-shortcut` | Record a keyboard shortcut |
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
cwm focus --app Chrome --verbose
cwm focus --app Safari --app Chrome  # try Safari first, fallback to Chrome
cwm focus -a Safari -a Chrome        # short form
```

Options:
- `--app, -a <NAME>` - Target app name or window title (required, repeatable, fuzzy matched)
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

### move-display

Move a window to another display.

```bash
cwm move-display next            # move to next display
cwm move-display prev            # move to previous display
cwm move-display 0               # move to display index 0
cwm move-display external        # move to display alias "external"
cwm move-display next --app "Terminal"
```

Options:
- `<TARGET>` - Display target: `next`, `prev`, index (0-based), or alias name
- `--app, -a <NAME>` - Target app name (optional)
- `--launch` - Launch app if not running
- `--no-launch` - Never launch app
- `--verbose, -v` - Show details

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
      "action": "move_display:external"
    },
    {
      "keys": "ctrl+alt+o",
      "action": "move_display:office_main"
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

# JSON output
cwm list apps --json             # basic JSON (name, pid)
cwm list apps --json --detailed  # full JSON (includes bundle_id, titles)

cwm list displays --json         # basic JSON (index, name, resolution, is_main)
cwm list displays --json --detailed  # full JSON (includes vendor, model, unique_id)

cwm list aliases --json          # basic JSON (name, type, resolved, display_index)
cwm list aliases --json --detailed   # full JSON (includes display_name, mapped_ids)
```

Resources:
- `apps` - Running applications with their window titles
- `displays` - Available displays with resolution and position
- `aliases` - Display aliases (system: builtin, external, main, secondary; and user-defined)

Options:
- `--json` - Output in JSON format
- `--detailed, -d` - Include additional fields in JSON output (no effect on text output)

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

## Global Options

These options can be used with any command:

- `--config <PATH>` - Use a custom config file instead of the default location

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

### Shortcut format

- `keys` - Key combination (e.g., `ctrl+alt+s`, `cmd+shift+return`)
- `action` - One of:
  - `focus` - Focus an application window
  - `maximize` - Maximize window to fill screen
  - `move_display:next` - Move to next display
  - `move_display:prev` - Move to previous display
  - `move_display:<index>` - Move to display by index (0-based)
  - `move_display:<alias>` - Move to display by alias name
  - `resize:<size>` - Resize window. Size formats:
    - `resize:80` - 80% of screen
    - `resize:0.8` - 80% (decimal)
    - `resize:full` - 100%
    - `resize:1920px` - 1920 pixels wide
    - `resize:1920x1080px` - exact pixel dimensions
    - `resize:800pt` - 800 points wide
    - `resize:800x600pt` - exact point dimensions
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

### Spotlight shortcuts

Spotlight shortcuts create macOS app bundles that appear in Spotlight search. When triggered, they execute cwm commands.

- `name` - Name displayed in Spotlight (prefixed with "cwm: ")
- `action` - Same format as shortcuts: `focus`, `maximize`, `move_display:next`, `resize:80`
- `app` - Target app name (required for `focus`, optional for others)
- `launch` - Launch app if not running (optional)

After adding shortcuts to config, install them with:

```bash
cwm spotlight install
```

Shortcuts are installed to `~/Applications/cwm/` and indexed by Spotlight. Search for "cwm: <name>" to run them.

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

## Website

The project has an interactive landing page at [cwm.taulfsime.com](https://cwm.taulfsime.com) featuring:

- **Interactive terminal** - Try cwm commands in your browser
- **Live preview** - Watch animated window manipulations as you type
- **Auto-demo** - See cwm in action immediately on page load
- **Multi-display simulation** - Add up to 4 virtual displays

The site is built with vanilla HTML/CSS/JS (no build step) and deployed automatically via GitHub Pages.

## License

MIT
