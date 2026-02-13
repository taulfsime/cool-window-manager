# cwm Scripts & Recipes

Useful scripts, one-liners, and integration patterns for window management automation.

## Table of Contents

- [Output Formats](#output-formats)
- [JSON-RPC Format](#json-rpc-format)
- [Exit Codes](#exit-codes)
- [IPC Socket](#ipc-socket)
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

## IPC Socket

When the daemon is running, cwm exposes a Unix socket at `~/.cwm/cwm.sock` for inter-process communication. This allows external tools to control cwm without spawning new processes.

### Socket Location

```bash
# default location
~/.cwm/cwm.sock

# check if daemon is running and socket exists
ls -la ~/.cwm/cwm.sock
```

### Protocol

The IPC supports two input formats:

1. **JSON** (recommended) - Standard JSON-RPC 2.0 style, consistent with CLI `--json` output
2. **Plain text** - Minimal format for simple scripting

The `"jsonrpc": "2.0"` field is optional in JSON requests - you can omit it for brevity.

The response format matches the input format:
- JSON input → JSON-RPC 2.0 response
- Plain text input → Plain text response (`OK` or `ERROR: message`)

### JSON Format (Recommended)

Uses JSON-RPC 2.0 style, consistent with cwm's CLI `--json` output. The `"jsonrpc": "2.0"` field is optional.

**Request (full):**
```json
{"jsonrpc": "2.0", "method": "focus", "params": {"app": "Safari"}, "id": 1}
```

**Request (minimal - jsonrpc field omitted):**
```json
{"method": "focus", "params": {"app": "Safari"}, "id": 1}
```

**Success Response:**
```json
{"jsonrpc": "2.0", "result": {"message": "Focused Safari", "app": "Safari"}, "id": "1"}
```

**Error Response:**
```json
{"jsonrpc": "2.0", "error": {"code": -32002, "message": "App not found"}, "id": "1"}
```

**Notification (no response):**
```json
{"method": "focus", "params": {"app": "Safari"}}
```

When `id` is omitted, the request is treated as a notification and no response is sent.

### Available Methods

| Method | Parameters | Description |
|--------|------------|-------------|
| `ping` | none | Health check, returns "pong" |
| `status` | none | Daemon status (pid, shortcuts count, etc.) |
| `focus` | `app` | Focus an application window |
| `maximize` | `app` (optional) | Maximize window |
| `resize` | `to` (required), `app` (optional) | Resize window |
| `move_display` | `target`, `app` (optional) | Move window to display |
| `list_apps` | none | List running applications |
| `list_displays` | none | List available displays |
| `action` | `action` | Execute raw action string |

### Shell Examples (JSON)

```bash
# ping with id (expects response)
echo '{"jsonrpc":"2.0","method":"ping","id":1}' | nc -U ~/.cwm/cwm.sock
# {"jsonrpc":"2.0","result":"pong","id":"1"}

# focus an app
echo '{"jsonrpc":"2.0","method":"focus","params":{"app":"Safari"},"id":1}' | nc -U ~/.cwm/cwm.sock
# {"jsonrpc":"2.0","result":{"message":"Focused Safari","app":"Safari"},"id":"1"}

# notification (no response expected)
echo '{"jsonrpc":"2.0","method":"focus","params":{"app":"Safari"}}' | nc -U ~/.cwm/cwm.sock
# (no output - fire and forget)

# get daemon status
echo '{"jsonrpc":"2.0","method":"status","id":1}' | nc -U ~/.cwm/cwm.sock | jq .

# maximize current window
echo '{"jsonrpc":"2.0","method":"maximize","id":1}' | nc -U ~/.cwm/cwm.sock

# resize to 80%
echo '{"jsonrpc":"2.0","method":"resize","params":{"to":"80"},"id":1}' | nc -U ~/.cwm/cwm.sock

# move to next display
echo '{"jsonrpc":"2.0","method":"move_display","params":{"target":"next"},"id":1}' | nc -U ~/.cwm/cwm.sock

# list running apps
echo '{"jsonrpc":"2.0","method":"list_apps","id":1}' | nc -U ~/.cwm/cwm.sock | jq '.result.apps[].name'

# list displays
echo '{"jsonrpc":"2.0","method":"list_displays","id":1}' | nc -U ~/.cwm/cwm.sock | jq '.result.displays'

# with string id
echo '{"jsonrpc":"2.0","method":"ping","id":"my-request-123"}' | nc -U ~/.cwm/cwm.sock
# {"jsonrpc":"2.0","result":"pong","id":"my-request-123"}
```

### Python Example

```python
#!/usr/bin/env python3
import json
import os
import socket

SOCKET_PATH = os.path.expanduser("~/.cwm/cwm.sock")

def send_jsonrpc(method: str, params: dict = None, id: str = "1") -> dict:
    """Send a JSON-RPC 2.0 request to the cwm daemon."""
    request = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params or {},
        "id": id
    }
    
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(SOCKET_PATH)
        sock.sendall(json.dumps(request).encode() + b"\n")
        sock.shutdown(socket.SHUT_WR)
        
        response = b""
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
        
        return json.loads(response.decode())

def send_notification(method: str, params: dict = None):
    """Send a JSON-RPC 2.0 notification (no response expected)."""
    request = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params or {}
    }
    
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(SOCKET_PATH)
        sock.sendall(json.dumps(request).encode() + b"\n")
        sock.shutdown(socket.SHUT_WR)

# examples
print(send_jsonrpc("ping"))
# {'jsonrpc': '2.0', 'result': 'pong', 'id': '1'}

print(send_jsonrpc("status"))
# {'jsonrpc': '2.0', 'result': {'running': True, 'pid': 12345, ...}, 'id': '1'}

print(send_jsonrpc("focus", {"app": "Safari"}))
# {'jsonrpc': '2.0', 'result': {'message': 'Focused Safari', 'app': 'Safari'}, 'id': '1'}

# fire and forget (notification)
send_notification("focus", {"app": "Safari"})

# list apps
response = send_jsonrpc("list_apps")
if "result" in response:
    for app in response["result"]["apps"]:
        print(f"{app['name']} (PID: {app['pid']})")

# error handling
response = send_jsonrpc("focus", {"app": "NonExistent"})
if "error" in response:
    print(f"Error {response['error']['code']}: {response['error']['message']}")
```

### Node.js Example

```javascript
const net = require('net');
const path = require('path');
const os = require('os');

const SOCKET_PATH = path.join(os.homedir(), '.cwm', 'cwm.sock');

function sendJsonRpc(method, params = {}, id = '1') {
  return new Promise((resolve, reject) => {
    const client = net.createConnection(SOCKET_PATH, () => {
      const request = JSON.stringify({
        jsonrpc: '2.0',
        method,
        params,
        id
      }) + '\n';
      client.write(request);
      client.end();
    });

    let data = '';
    client.on('data', chunk => { data += chunk; });
    client.on('end', () => {
      try {
        resolve(JSON.parse(data));
      } catch (e) {
        reject(new Error(`Invalid response: ${data}`));
      }
    });
    client.on('error', reject);
  });
}

function sendNotification(method, params = {}) {
  return new Promise((resolve, reject) => {
    const client = net.createConnection(SOCKET_PATH, () => {
      const request = JSON.stringify({
        jsonrpc: '2.0',
        method,
        params
        // no id = notification
      }) + '\n';
      client.write(request);
      client.end();
      resolve();
    });
    client.on('error', reject);
  });
}

// examples
async function main() {
  console.log(await sendJsonRpc('ping'));
  // { jsonrpc: '2.0', result: 'pong', id: '1' }

  console.log(await sendJsonRpc('status'));
  // { jsonrpc: '2.0', result: { running: true, pid: 12345, ... }, id: '1' }

  console.log(await sendJsonRpc('focus', { app: 'Safari' }));
  // { jsonrpc: '2.0', result: { message: 'Focused Safari', app: 'Safari' }, id: '1' }

  // fire and forget
  await sendNotification('focus', { app: 'Safari' });

  // error handling
  const response = await sendJsonRpc('focus', { app: 'NonExistent' });
  if (response.error) {
    console.error(`Error ${response.error.code}: ${response.error.message}`);
  }
}

main().catch(console.error);
```

### Ruby Example

```ruby
#!/usr/bin/env ruby
require 'socket'
require 'json'

SOCKET_PATH = File.expand_path('~/.cwm/cwm.sock')

def send_jsonrpc(method, params = {}, id = '1')
  request = {
    jsonrpc: '2.0',
    method: method,
    params: params,
    id: id
  }
  
  UNIXSocket.open(SOCKET_PATH) do |sock|
    sock.puts(request.to_json)
    sock.close_write
    JSON.parse(sock.read)
  end
end

def send_notification(method, params = {})
  request = {
    jsonrpc: '2.0',
    method: method,
    params: params
  }
  
  UNIXSocket.open(SOCKET_PATH) do |sock|
    sock.puts(request.to_json)
    sock.close_write
  end
end

# examples
puts send_jsonrpc('ping')
# {"jsonrpc"=>"2.0", "result"=>"pong", "id"=>"1"}

puts send_jsonrpc('focus', { app: 'Safari' })
# {"jsonrpc"=>"2.0", "result"=>{"message"=>"Focused Safari", "app"=>"Safari"}, "id"=>"1"}

# fire and forget
send_notification('focus', { app: 'Safari' })

# list apps
response = send_jsonrpc('list_apps')
response['result']['apps'].each do |app|
  puts "#{app['name']} (PID: #{app['pid']})"
end
```

### Go Example

```go
package main

import (
    "bufio"
    "encoding/json"
    "fmt"
    "net"
    "os"
    "path/filepath"
)

type JsonRpcRequest struct {
    Jsonrpc string            `json:"jsonrpc"`
    Method  string            `json:"method"`
    Params  map[string]string `json:"params,omitempty"`
    Id      interface{}       `json:"id,omitempty"`
}

type JsonRpcResponse struct {
    Jsonrpc string          `json:"jsonrpc"`
    Result  json.RawMessage `json:"result,omitempty"`
    Error   *RpcError       `json:"error,omitempty"`
    Id      interface{}     `json:"id"`
}

type RpcError struct {
    Code    int    `json:"code"`
    Message string `json:"message"`
}

func sendJsonRpc(method string, params map[string]string, id string) (*JsonRpcResponse, error) {
    socketPath := filepath.Join(os.Getenv("HOME"), ".cwm", "cwm.sock")
    
    conn, err := net.Dial("unix", socketPath)
    if err != nil {
        return nil, err
    }
    defer conn.Close()

    request := JsonRpcRequest{
        Jsonrpc: "2.0",
        Method:  method,
        Params:  params,
        Id:      id,
    }
    
    encoder := json.NewEncoder(conn)
    if err := encoder.Encode(request); err != nil {
        return nil, err
    }

    reader := bufio.NewReader(conn)
    line, err := reader.ReadBytes('\n')
    if err != nil {
        return nil, err
    }

    var response JsonRpcResponse
    if err := json.Unmarshal(line, &response); err != nil {
        return nil, err
    }

    return &response, nil
}

func main() {
    // ping
    resp, _ := sendJsonRpc("ping", nil, "1")
    fmt.Printf("Ping: %s\n", resp.Result)

    // focus app
    resp, _ = sendJsonRpc("focus", map[string]string{"app": "Safari"}, "1")
    if resp.Error != nil {
        fmt.Printf("Error %d: %s\n", resp.Error.Code, resp.Error.Message)
    } else {
        fmt.Printf("Focus: %s\n", resp.Result)
    }

    // list apps
    resp, _ = sendJsonRpc("list_apps", nil, "1")
    fmt.Printf("Apps: %s\n", resp.Result)
}
```

### Rust Example

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    params: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
    id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RpcError {
    code: i32,
    message: String,
}

fn send_jsonrpc(
    method: &str,
    params: HashMap<String, String>,
    id: Option<&str>,
) -> Result<JsonRpcResponse, Box<dyn std::error::Error>> {
    let socket_path = dirs::home_dir()
        .unwrap()
        .join(".cwm")
        .join("cwm.sock");

    let mut stream = UnixStream::connect(socket_path)?;
    
    let request = JsonRpcRequest {
        jsonrpc: "2.0",
        method: method.to_string(),
        params,
        id: id.map(String::from),
    };
    
    let json = serde_json::to_string(&request)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    Ok(serde_json::from_str(&response_line)?)
}

fn main() {
    // ping
    let resp = send_jsonrpc("ping", HashMap::new(), Some("1")).unwrap();
    println!("Ping: {:?}", resp.result);

    // focus app
    let mut params = HashMap::new();
    params.insert("app".to_string(), "Safari".to_string());
    let resp = send_jsonrpc("focus", params, Some("1")).unwrap();
    
    if let Some(error) = resp.error {
        println!("Error {}: {}", error.code, error.message);
    } else {
        println!("Focus: {:?}", resp.result);
    }
}
```

### Hammerspoon Integration

```lua
-- ~/.hammerspoon/init.lua

local socket = require("hs.socket")
local json = require("hs.json")

local cwmSocket = os.getenv("HOME") .. "/.cwm/cwm.sock"
local requestId = 0

local function cwmSend(method, params, callback)
  requestId = requestId + 1
  local request = json.encode({
    jsonrpc = "2.0",
    method = method,
    params = params or {},
    id = tostring(requestId)
  }) .. "\n"
  
  local client = socket.new()
  client:connect(cwmSocket, function()
    client:write(request)
    client:read("\n", function(data)
      if callback then
        local response = json.decode(data)
        callback(response)
      end
      client:disconnect()
    end)
  end)
end

-- fire and forget (notification)
local function cwmNotify(method, params)
  local request = json.encode({
    jsonrpc = "2.0",
    method = method,
    params = params or {}
  }) .. "\n"
  
  local client = socket.new()
  client:connect(cwmSocket, function()
    client:write(request)
    client:disconnect()
  end)
end

-- focus app via IPC (faster than spawning cwm process)
hs.hotkey.bind({"cmd", "alt"}, "s", function()
  cwmSend("focus", { app = "Safari" }, function(resp)
    if resp.error then
      hs.alert.show("Error: " .. resp.error.message)
    end
  end)
end)

-- maximize via IPC (fire and forget)
hs.hotkey.bind({"cmd", "alt"}, "m", function()
  cwmNotify("maximize")
end)

-- get running apps
hs.hotkey.bind({"cmd", "alt"}, "l", function()
  cwmSend("list_apps", nil, function(resp)
    if resp.result then
      local names = {}
      for _, app in ipairs(resp.result.apps) do
        table.insert(names, app.name)
      end
      hs.alert.show(table.concat(names, "\n"))
    end
  end)
end)
```

### Error Handling

JSON-RPC errors include a code and message:

```bash
response=$(echo '{"jsonrpc":"2.0","method":"focus","params":{"app":"NonExistent"},"id":1}' | nc -U ~/.cwm/cwm.sock)

# check for error
if echo "$response" | jq -e '.error' > /dev/null 2>&1; then
  code=$(echo "$response" | jq -r '.error.code')
  message=$(echo "$response" | jq -r '.error.message')
  echo "Error $code: $message"
else
  echo "Success: $(echo "$response" | jq -r '.result')"
fi
```

### Error Codes

| Exit Code | JSON-RPC Code | Meaning |
|-----------|---------------|---------|
| 1 | -32001 | General error |
| 2 | -32002 | App not found |
| 3 | -32003 | Permission denied |
| 4 | -32004 | Invalid arguments |
| 6 | -32006 | Window not found |
| 7 | -32007 | Display not found |

### Plain Text Protocol

For simple scripting, plain text commands are also supported:

```bash
# plain text input -> plain text output
echo "ping" | nc -U ~/.cwm/cwm.sock
# OK

echo "focus:Safari" | nc -U ~/.cwm/cwm.sock
# OK

echo "focus:NonExistent" | nc -U ~/.cwm/cwm.sock
# ERROR: App not found

echo "maximize" | nc -U ~/.cwm/cwm.sock
# OK

echo "resize:80" | nc -U ~/.cwm/cwm.sock
# OK

echo "move_display:next" | nc -U ~/.cwm/cwm.sock
# OK
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
