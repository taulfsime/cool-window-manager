# cwm Scripts & Recipes

Useful scripts, one-liners, and integration patterns for window management automation.

## Table of Contents

- [Output Formats](#output-formats)
- [JSON-RPC Format](#json-rpc-format)
- [Exit Codes](#exit-codes)
- [One-Liners](#one-liners)
- [fzf Integration](#fzf-integration)
- [Shell Functions](#shell-functions)
- [Layout Scripts](#layout-scripts)
- [Automation](#automation)
- [Tool Integration](#tool-integration)

---

## Output Formats

cwm supports multiple output formats for different use cases:

| Flag | Description | Example |
|------|-------------|---------|
| (none) | Human-readable when TTY, JSON-RPC when piped | `cwm list apps` |
| `--json` / `-j` | Force JSON-RPC output | `cwm list apps --json` |
| `--no-json` | Force text output even when piped | `cwm list apps --no-json \| cat` |
| `--names` | One name per line (ideal for fzf/xargs) | `cwm list apps --names` |
| `--format` | Custom format with `{field}` placeholders | `cwm list apps --format '{name} ({pid})'` |
| `--quiet` / `-q` | No output on success | `cwm focus --app Safari -q` |

### Auto-Detection

When stdout is piped (not a TTY), cwm automatically outputs JSON-RPC:

```bash
# these are equivalent when piped
cwm list apps | jq .result
cwm list apps --json | jq .result
```

To force text output when piping:

```bash
cwm list apps --no-json | grep Safari
```

### Format String Examples

```bash
# simple name list
cwm list apps --format '{name}'

# tab-separated for further processing
cwm list apps --format '{name}\t{pid}\t{bundle_id}'

# custom display format
cwm list displays --format 'Display {index}: {name} ({width}x{height})'

# with detailed fields
cwm list apps --detailed --format '{name} - {titles}'
```

---

## JSON-RPC Format

cwm uses [JSON-RPC 2.0](https://www.jsonrpc.org/specification) format for machine-readable output, making it easy to integrate with other tools and build automation.

### Success Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "action": "focus",
    "app": {
      "name": "Safari",
      "pid": 1234,
      "bundle_id": "com.apple.Safari"
    },
    "match": {
      "type": "exact",
      "query": "safari"
    }
  },
  "id": null
}
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32002,
    "message": "no matching app found, tried: NonExistent",
    "data": {
      "suggestions": ["Safari", "Chrome", "Terminal"]
    }
  },
  "id": null
}
```

### Error Codes

JSON-RPC uses negative error codes. cwm maps its exit codes to the -32000 range (reserved for application errors):

| Exit Code | JSON-RPC Code | Meaning |
|-----------|---------------|---------|
| 0 | -32000 | Success (not used in errors) |
| 1 | -32001 | General error |
| 2 | -32002 | App not found |
| 3 | -32003 | Permission denied |
| 4 | -32004 | Invalid arguments |
| 5 | -32005 | Config error |
| 6 | -32006 | Window not found |
| 7 | -32007 | Display not found |

### Parsing with jq

```bash
# get the result
cwm focus --app Safari --json | jq '.result'

# check for errors
cwm focus --app NonExistent --json | jq 'if .error then .error.message else .result.app.name end'

# extract specific fields
cwm get focused --json | jq -r '.result.app.name'
cwm list apps --json | jq -r '.result.items[].name'
```

---

## Exit Codes

cwm uses specific exit codes to help scripts handle different failure scenarios:

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | SUCCESS | Command completed successfully |
| 1 | ERROR | General error |
| 2 | APP_NOT_FOUND | Application not running |
| 3 | PERMISSION_DENIED | Accessibility permissions not granted |
| 4 | INVALID_ARGS | Invalid command-line arguments |
| 5 | CONFIG_ERROR | Configuration file error |
| 6 | WINDOW_NOT_FOUND | App running but has no window |
| 7 | DISPLAY_NOT_FOUND | Target display doesn't exist |

### Checking Exit Codes

```bash
cwm focus --app Safari
case $? in
  0) echo "Focused successfully" ;;
  2) echo "Safari is not running" ;;
  3) echo "Grant accessibility permissions" ;;
  *) echo "Unknown error: $?" ;;
esac
```

### JSON-RPC Error Output

When using `--json`, errors follow JSON-RPC 2.0 format:

```bash
cwm focus --app NonExistent --json
# {"jsonrpc":"2.0","error":{"code":-32002,"message":"no matching app found, tried: NonExistent","data":{"suggestions":["Safari","Chrome"...]}},"id":null}
```

---

## One-Liners

### Basic Operations

```bash
# focus app with fallback chain
cwm focus --app Safari || cwm focus --app Chrome || cwm focus --app Firefox

# check if app is running (silent)
cwm focus --app Slack -q && echo "Slack is running"

# maximize all windows matching a pattern
cwm list apps --names | grep -i code | xargs -I{} cwm maximize --app "{}"

# get focused app name
cwm get focused --format '{app.name}'

# get focused window dimensions
cwm get focused --format '{window.width}x{window.height}'
```

### Piping App Names

```bash
# pipe app name from another command
echo "Safari" | cwm focus --app -

# focus the first app from a filtered list
cwm list apps --names | grep -i term | head -1 | cwm focus --app -
```

---

## fzf Integration

cwm's `--names` flag makes it easy to integrate with fzf without requiring jq.

### Basic Pickers

```bash
# focus any running app
cwm list apps --names | fzf | xargs cwm focus --app

# focus with preview showing window info
cwm list apps --names | fzf --preview 'cwm get window --app {} 2>/dev/null || echo "No window"' | xargs cwm focus --app

# move window to selected display
cwm list displays --names | fzf --prompt="Move to: " | xargs cwm move-display

# resize with preset sizes
echo -e "25\n50\n75\n100\nfull" | fzf --prompt="Resize to %: " | xargs cwm resize --to
```

### Advanced fzf Usage

```bash
# multi-select apps to maximize
cwm list apps --names | fzf --multi --prompt="Maximize: " | xargs -I{} cwm maximize --app "{}"

# app picker with custom format preview
cwm list apps --format '{name}|{pid}|{bundle_id}' | \
  fzf --delimiter='|' --with-nth=1 --preview='echo "PID: {2}\nBundle: {3}"' | \
  cut -d'|' -f1 | xargs cwm focus --app
```

---

## Shell Functions

Add these to your `~/.zshrc` or `~/.bashrc`:

### Basic Functions

```bash
# focus app with fzf
f() {
  local app
  app=$(cwm list apps --names | fzf --prompt="Focus: " --height=40%) \
    && cwm focus --app "$app"
}

# move window to display with fzf
m() {
  local display
  display=$(cwm list displays --format '{index}: {name}' | fzf --prompt="Display: " | cut -d: -f1) \
    && cwm move-display "$display"
}

# quick resize
r() {
  local size="${1:-$(echo -e '50\n75\n100' | fzf --prompt='Size %: ')}"
  cwm resize --to "$size"
}
```

### Advanced Functions

```bash
# full workflow: pick app, then action
wm() {
  local app action
  app=$(cwm list apps --names | fzf --prompt="App: ") || return
  action=$(echo -e "focus\nmaximize\nresize\nmove" | fzf --prompt="Action: ") || return
  case "$action" in
    focus) cwm focus --app "$app" ;;
    maximize) cwm maximize --app "$app" ;;
    resize) 
      local size=$(echo -e '50\n75\n100' | fzf --prompt='Size %: ')
      cwm resize --to "$size" --app "$app" 
      ;;
    move) 
      local display=$(cwm list displays --names | fzf --prompt="Display: ")
      cwm move-display "$display" --app "$app" 
      ;;
  esac
}

# get info about focused window
winfo() {
  cwm get focused
}

# get info about specific app
winfo-app() {
  local app="${1:-$(cwm list apps --names | fzf --prompt='App: ')}"
  cwm get window --app "$app"
}
```

---

## Layout Scripts

### Development Setup

```bash
#!/bin/bash
# ~/bin/layout-dev.sh - IDE left, terminal right, browser on second display
set -e

# main display: code editor maximized
cwm focus --app "VS Code" --launch
cwm maximize --app "VS Code"

# check if second display exists
display_count=$(cwm list displays --json | jq 'length')

if [ "$display_count" -gt 1 ]; then
  cwm focus --app Terminal --launch
  cwm move-display 2 --app Terminal
  cwm maximize --app Terminal
fi
```

### Meeting Mode

```bash
#!/bin/bash
# ~/bin/layout-meeting.sh - maximize video call app

# try common video call apps in order
for app in Zoom "Microsoft Teams" "Google Meet" FaceTime; do
  if cwm focus --app "$app" -q 2>/dev/null; then
    cwm maximize --app "$app"
    echo "Maximized $app"
    exit 0
  fi
done

echo "No video call app found"
exit 1
```

### Tiling Layout

```bash
#!/bin/bash
# ~/bin/layout-tile.sh - tile two apps side by side

app1="${1:-}"
app2="${2:-}"

if [ -z "$app1" ] || [ -z "$app2" ]; then
  echo "Usage: layout-tile.sh <app1> <app2>"
  exit 1
fi

# get display dimensions
width=$(cwm get focused --format '{window.width}')

# resize first app to left half
cwm focus --app "$app1"
cwm resize --to 50 --app "$app1"

# resize second app to right half
cwm focus --app "$app2"
cwm resize --to 50 --app "$app2"
```

### Save/Restore Layout

```bash
#!/bin/bash
# save-layout.sh - save current window positions

layout_file="${1:-$HOME/.cwm-layout.sh}"

echo "#!/bin/bash" > "$layout_file"
echo "# saved layout - $(date)" >> "$layout_file"

cwm list apps --json --detailed | jq -r '
  .items[] | 
  "cwm focus --app \"\(.name)\" && cwm resize --to \(.width // 800)x\(.height // 600)px --app \"\(.name)\""
' >> "$layout_file"

chmod +x "$layout_file"
echo "Layout saved to $layout_file"
```

---

## Automation

### Watch for App Launch

```bash
#!/bin/bash
# auto-maximize.sh - auto-maximize specific apps when they launch

apps_to_maximize="Slack|Discord|Zoom"

while true; do
  cwm list apps --names | grep -E "$apps_to_maximize" | while read app; do
    # check if we've already maximized this app (use a temp file as state)
    state_file="/tmp/cwm-maximized-$(echo $app | tr ' ' '_')"
    if [ ! -f "$state_file" ]; then
      sleep 1  # wait for window to appear
      if cwm maximize --app "$app" -q 2>/dev/null; then
        touch "$state_file"
        echo "Maximized: $app"
      fi
    fi
  done
  sleep 5
done
```

### Display Connect Handler

```bash
#!/bin/bash
# display-handler.sh - run when display configuration changes

displays=$(cwm list displays --json | jq 'length')

case $displays in
  1) 
    echo "Single display mode"
    ~/bin/layout-laptop.sh 
    ;;
  2) 
    echo "Dual display mode"
    ~/bin/layout-desk.sh 
    ;;
  3) 
    echo "Triple display mode"
    ~/bin/layout-office.sh 
    ;;
esac
```

---

## Tool Integration

### Raycast Script

```bash
#!/bin/bash

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title Focus App
# @raycast.mode silent
# @raycast.argument1 { "type": "text", "placeholder": "App name" }

/usr/local/bin/cwm focus --app "$1" --launch
```

### Alfred Workflow

```bash
# Script Filter returning Alfred JSON format
/usr/local/bin/cwm list apps --json | jq '{
  items: [.items[] | {
    title: .name,
    arg: .name,
    subtitle: "PID: \(.pid)",
    icon: { type: "fileicon", path: "/Applications/\(.name).app" }
  }]
}'
```

### Keyboard Maestro

```applescript
do shell script "/usr/local/bin/cwm focus --app Safari"
```

### Hammerspoon

```lua
-- ~/.hammerspoon/init.lua

local function cwm(args)
  return hs.execute("/usr/local/bin/cwm " .. args, true)
end

local function focusApp(name)
  cwm("focus --app '" .. name .. "'")
end

-- hotkeys for common apps
hs.hotkey.bind({"cmd", "alt"}, "s", function() focusApp("Safari") end)
hs.hotkey.bind({"cmd", "alt"}, "t", function() focusApp("Terminal") end)
hs.hotkey.bind({"cmd", "alt"}, "c", function() focusApp("VS Code") end)

-- maximize current window
hs.hotkey.bind({"cmd", "alt"}, "m", function() cwm("maximize") end)

-- move to next display
hs.hotkey.bind({"cmd", "alt"}, "n", function() cwm("move-display next") end)
```

### Karabiner-Elements

```json
{
  "description": "Hyper+F to focus picker",
  "manipulators": [{
    "type": "basic",
    "from": {
      "key_code": "f",
      "modifiers": {
        "mandatory": ["left_shift", "left_command", "left_option", "left_control"]
      }
    },
    "to": [{
      "shell_command": "/usr/local/bin/cwm list apps --names | /opt/homebrew/bin/fzf-tmux | xargs /usr/local/bin/cwm focus --app"
    }]
  }]
}
```

---

## Tips

### Combining with GNU Parallel

```bash
# maximize multiple apps in parallel
cwm list apps --names | grep -E 'Safari|Chrome|Firefox' | parallel cwm maximize --app {}
```

### Debugging Scripts

```bash
# verbose mode shows match details
cwm focus --app Safari -v

# check what cwm sees
cwm list apps --detailed --json | jq .
cwm list displays --detailed --json | jq .

# test JSON output
cwm focus --app Safari --json | jq .
```

### JSON Processing Without jq

If jq is not available, use `--names` or `--format`:

```bash
# instead of: cwm list apps --json | jq -r '.items[].name'
cwm list apps --names

# instead of: cwm list apps --json | jq -r '.items[] | "\(.name) \(.pid)"'
cwm list apps --format '{name} {pid}'
```

### Error Handling in Scripts

```bash
#!/bin/bash
set -e  # exit on error

# with error handling
if ! cwm focus --app Safari -q 2>/dev/null; then
  echo "Safari not running, launching..."
  cwm focus --app Safari --launch
fi

# capture JSON errors
result=$(cwm focus --app NonExistent --json 2>&1)
if echo "$result" | jq -e '.success == false' >/dev/null 2>&1; then
  echo "Error: $(echo "$result" | jq -r '.error.message')"
  echo "Suggestions: $(echo "$result" | jq -r '.error.suggestions | join(", ")')"
fi
```
