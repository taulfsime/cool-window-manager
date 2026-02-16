# Conditions System

cwm supports a powerful conditions system that allows shortcuts and app rules to execute only when specific criteria are met. This enables context-aware window management based on time, display configuration, and application state.

## Quick Start

Add a `when` field to any shortcut or app rule:

```jsonc
{
  "shortcuts": [
    {
      "keys": "ctrl+alt+m",
      "action": "maximize",
      "when": { "display.count": { ">=": 2 } }  // only when docked
    }
  ]
}
```

## Table of Contents

- [Condition Fields](#condition-fields)
- [Comparison Operators](#comparison-operators)
- [Logical Operators](#logical-operators)
- [Global Definitions](#global-definitions)
- [Examples](#examples)

## Condition Fields

### Time Conditions

| Field | Type | Description |
|-------|------|-------------|
| `time` | string | Time range(s) in 12h or 24h format |
| `time.day` | string | Day(s) of the week |

**Time format examples:**
- `"9:00-17:00"` - 9 AM to 5 PM (24h format)
- `"9:00AM-5:00PM"` - 9 AM to 5 PM (12h format)
- `"22:00-06:00"` - overnight range (10 PM to 6 AM next day)
- `"9:00-12:00,14:00-18:00"` - multiple ranges (comma-separated)

**Day format examples:**
- `"mon"` - single day
- `"mon-fri"` - day range (Monday through Friday)
- `"mon,wed,fri"` - specific days (comma-separated)
- `"sat,sun"` - weekend

### Display Conditions

| Field | Type | Description |
|-------|------|-------------|
| `display.count` | number | Number of connected displays |
| `display.connected` | string | Check if a display alias is connected |

**System aliases** (always available):
- `builtin` - built-in display (MacBook screen)
- `external` - any external monitor
- `main` - primary display

**User-defined aliases** can be configured in `display_aliases`:

```jsonc
{
  "display_aliases": {
    "office": ["10AC_D0B3_67890"],
    "home": ["1E6D_5B11_12345"]
  }
}
```

Find display IDs with: `cwm list displays --detailed`

### App Conditions

| Field | Type | Description |
|-------|------|-------------|
| `app` | string | Target app name/title match (fuzzy, prefix, regex) |
| `app.running` | string | Check if an app is running |
| `app.focused` | string/bool | Check which app has focus |
| `app.fullscreen` | bool | Target window fullscreen state |
| `app.minimized` | bool | Target window minimized state |
| `app.display` | string | Which display the target window is on |

**App matching supports:**
- Exact match (case-insensitive): `"Safari"`
- Prefix match: `"Goo"` matches "Google Chrome"
- Regex match: `"/^Google/i"` (JavaScript-style regex)

## Comparison Operators

All operators have three equivalent forms:

| Symbol | Short | Long | Description |
|--------|-------|------|-------------|
| `==` | `eq` | `equals` | Equal to |
| `!=` | `ne` | `not_equals` | Not equal to |
| `>` | `gt` | `greater_than` | Greater than |
| `>=` | `gte` | `greater_than_or_equal` | Greater than or equal |
| `<` | `lt` | `less_than` | Less than |
| `<=` | `lte` | `less_than_or_equal` | Less than or equal |
| `in` | - | - | Set membership |

**Usage:**

```jsonc
// all these are equivalent
{ "display.count": { ">=": 2 } }
{ "display.count": { "gte": 2 } }
{ "display.count": { "greater_than_or_equal": 2 } }

// simple equality (implicit ==)
{ "app.running": "Safari" }

// set membership
{ "display.connected": { "in": ["external", "office"] } }
```

## Logical Operators

### `all` (AND)

All conditions must be true:

```jsonc
{
  "all": [
    { "display.count": { ">=": 2 } },
    { "time.day": "mon-fri" }
  ]
}
```

### `any` (OR)

Any condition must be true:

```jsonc
{
  "any": [
    { "app.running": "Safari" },
    { "app.running": "Chrome" }
  ]
}
```

### `not` (NOT)

Negate a condition:

```jsonc
{
  "not": { "app.fullscreen": true }
}
```

### Implicit AND

Multiple fields in one object are implicitly ANDed:

```jsonc
// these are equivalent
{ "time": "9:00-17:00", "time.day": "mon-fri" }
{ "all": [{ "time": "9:00-17:00" }, { "time.day": "mon-fri" }] }
```

## Global Definitions

Define reusable conditions at the config root and reference them with `$ref`:

```jsonc
{
  "conditions": {
    "work_hours": { "time": "9:00-17:00", "time.day": "mon-fri" },
    "docked": { "display.count": { ">=": 2 } },
    "at_office": {
      "all": [
        { "$ref": "docked" },
        { "display.connected": "office" }
      ]
    }
  },
  "shortcuts": [
    {
      "keys": "ctrl+alt+m",
      "action": "maximize",
      "when": { "$ref": "docked" }
    },
    {
      "keys": "ctrl+alt+s",
      "action": "move:office",
      "app": "Slack",
      "when": { "$ref": "at_office" }
    }
  ]
}
```

## Examples

### Work Hours Only

```jsonc
{
  "shortcuts": [
    {
      "keys": "ctrl+alt+s",
      "action": "focus",
      "app": "Slack",
      "when": {
        "time": "9:00-17:00",
        "time.day": "mon-fri"
      }
    }
  ]
}
```

### Multi-Monitor Setup

```jsonc
{
  "shortcuts": [
    {
      "keys": "ctrl+alt+e",
      "action": "move:external",
      "when": { "display.connected": "external" }
    }
  ],
  "app_rules": [
    {
      "app": "Slack",
      "action": "move:external",
      "when": { "display.count": { ">=": 2 } }
    }
  ]
}
```

### Context-Aware Rules

```jsonc
{
  "conditions": {
    "at_home": { "display.connected": "home" },
    "at_office": { "display.connected": "office" }
  },
  "app_rules": [
    {
      "app": "Slack",
      "action": "move:1",
      "when": { "$ref": "at_home" }
    },
    {
      "app": "Slack",
      "action": "move:2",
      "when": { "$ref": "at_office" }
    }
  ]
}
```

### Prevent Action on Fullscreen

```jsonc
{
  "shortcuts": [
    {
      "keys": "ctrl+alt+m",
      "action": "maximize",
      "when": {
        "not": { "app.fullscreen": true }
      }
    }
  ]
}
```

### Browser Focus (Any Browser)

```jsonc
{
  "shortcuts": [
    {
      "keys": "ctrl+alt+b",
      "action": "focus",
      "app": "Safari",
      "when": {
        "any": [
          { "app.running": "Safari" },
          { "app.running": "Chrome" },
          { "app.running": "Firefox" }
        ]
      }
    }
  ]
}
```

### Night Mode

```jsonc
{
  "shortcuts": [
    {
      "keys": "ctrl+alt+n",
      "action": "resize:80",
      "when": {
        "any": [
          { "time": "22:00-23:59" },
          { "time": "00:00-06:00" }
        ]
      }
    }
  ]
}
```

## Rule Evaluation

When multiple rules match the same trigger (e.g., same hotkey or app launch):

1. Rules are evaluated in order (first defined = first checked)
2. **First matching rule wins** - subsequent rules are skipped
3. This allows fallback patterns:

```jsonc
{
  "app_rules": [
    {
      "app": "Slack",
      "action": "move:office",
      "when": { "display.connected": "office" }
    },
    {
      "app": "Slack",
      "action": "move:external",
      "when": { "display.connected": "external" }
    },
    {
      "app": "Slack",
      "action": "maximize"
      // no condition = fallback when others don't match
    }
  ]
}
```

## Debugging

If a condition isn't working as expected:

1. Check the daemon logs: `cwm daemon start --log /tmp/cwm.log`
2. Look for "Condition not met" messages
3. Verify display aliases: `cwm list displays --detailed`
4. Test time conditions by temporarily changing the range
5. Use `cwm config verify` to check for syntax errors
