#!/bin/bash
# manual-test.sh - Interactive manual testing for cwm
#
# This script guides you through manual testing of cwm features
# that require visual verification or user interaction.
#
# Usage:
#   ./scripts/manual-test.sh

set -e

# colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# find cwm binary
if [ -f "./target/release/cwm" ]; then
    CWM="./target/release/cwm"
elif [ -f "./target/debug/cwm" ]; then
    CWM="./target/debug/cwm"
else
    echo -e "${RED}Error: cwm binary not found. Run 'cargo build' first.${NC}"
    exit 1
fi

# create test config
TEST_CONFIG=$(mktemp)
cat > "$TEST_CONFIG" << 'EOF'
{
  "shortcuts": [],
  "app_rules": [],
  "spotlight": [
    {
      "name": "Test Focus Finder",
      "action": "focus",
      "app": "Finder",
      "launch": true
    },
    {
      "name": "Test Maximize",
      "action": "maximize"
    }
  ],
  "display_aliases": {
    "test_alias": ["nonexistent_id"]
  },
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

cleanup() {
    rm -f "$TEST_CONFIG"
}
trap cleanup EXIT

run_cwm() {
    "$CWM" --config "$TEST_CONFIG" "$@"
}

prompt() {
    echo ""
    echo -e "${CYAN}$1${NC}"
    echo -n "Press Enter to continue (or 'q' to quit, 's' to skip)... "
    read -r response
    if [ "$response" = "q" ]; then
        echo "Exiting."
        exit 0
    fi
    if [ "$response" = "s" ]; then
        return 1
    fi
    return 0
}

section() {
    echo ""
    echo -e "${YELLOW}============================================${NC}"
    echo -e "${YELLOW}  $1${NC}"
    echo -e "${YELLOW}============================================${NC}"
}

verify() {
    echo ""
    echo -e "${BLUE}Did it work correctly? (y/n)${NC} "
    read -r response
    if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
        echo -e "${GREEN}✓ Verified${NC}"
        return 0
    else
        echo -e "${RED}✗ Failed${NC}"
        return 1
    fi
}

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  cwm Manual Testing Guide${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "This script will guide you through manual testing of cwm."
echo "Each test requires visual verification."
echo ""
echo "Binary: $CWM"
echo ""

# ----------------------------------------------------------------------------
# Prerequisites
# ----------------------------------------------------------------------------

section "Prerequisites"

echo "Before testing, please ensure:"
echo "  1. You have at least 2-3 apps open (e.g., Finder, Safari, Terminal)"
echo "  2. cwm has Accessibility permissions"
echo ""
echo "Checking permissions..."
run_cwm check-permissions
echo ""

if ! prompt "Ready to start testing?"; then
    echo "Skipping prerequisites check"
fi

# ----------------------------------------------------------------------------
# Focus Tests
# ----------------------------------------------------------------------------

section "Focus Command Tests"

echo "Test 1: Focus Finder"
echo "Expected: Finder should come to the foreground"
if prompt "Running: cwm focus --app Finder"; then
    run_cwm focus --app Finder --no-json -v
    verify
fi

echo ""
echo "Test 2: Focus with fuzzy matching"
echo "Expected: Safari should come to the foreground (fuzzy match 'safar')"
if prompt "Running: cwm focus --app safar"; then
    run_cwm focus --app safar --no-json -v || true
    verify
fi

echo ""
echo "Test 3: Focus multiple apps (fallback)"
echo "Expected: First available app should be focused"
if prompt "Running: cwm focus --app NonExistent --app Finder"; then
    run_cwm focus --app NonExistent --app Finder --no-json -v
    verify
fi

# ----------------------------------------------------------------------------
# Maximize Tests
# ----------------------------------------------------------------------------

section "Maximize Command Tests"

echo "Test 1: Maximize focused window"
echo "Expected: Current window should fill the screen (excluding menu bar/dock)"
if prompt "Running: cwm maximize"; then
    run_cwm maximize --no-json -v
    verify
fi

echo ""
echo "Test 2: Maximize specific app"
echo "Expected: Finder window should maximize"
if prompt "Running: cwm maximize --app Finder"; then
    run_cwm maximize --app Finder --no-json -v
    verify
fi

# ----------------------------------------------------------------------------
# Resize Tests
# ----------------------------------------------------------------------------

section "Resize Command Tests"

echo "Test 1: Resize to 50%"
echo "Expected: Window should resize to 50% of screen, centered"
if prompt "Running: cwm resize --to 50"; then
    run_cwm resize --to 50 --no-json -v
    verify
fi

echo ""
echo "Test 2: Resize to 80%"
echo "Expected: Window should resize to 80% of screen, centered"
if prompt "Running: cwm resize --to 80"; then
    run_cwm resize --to 80 --no-json -v
    verify
fi

echo ""
echo "Test 3: Resize using decimal"
echo "Expected: Window should resize to 75% of screen"
if prompt "Running: cwm resize --to 0.75"; then
    run_cwm resize --to 0.75 --no-json -v
    verify
fi

echo ""
echo "Test 4: Resize to full"
echo "Expected: Window should fill the screen (same as maximize)"
if prompt "Running: cwm resize --to full"; then
    run_cwm resize --to full --no-json -v
    verify
fi

echo ""
echo "Test 5: Resize to specific pixels"
echo "Expected: Window should resize to 800x600 pixels, centered"
if prompt "Running: cwm resize --to 800x600px"; then
    run_cwm resize --to 800x600px --no-json -v
    verify
fi

# ----------------------------------------------------------------------------
# Move Display Tests
# ----------------------------------------------------------------------------

section "Move Display Command Tests"

DISPLAY_COUNT=$(run_cwm list displays --names 2>/dev/null | wc -l | tr -d ' ')
echo "Detected $DISPLAY_COUNT display(s)"

if [ "$DISPLAY_COUNT" -lt 2 ]; then
    echo -e "${YELLOW}Only 1 display detected. Skipping move-display tests.${NC}"
    echo "Connect an external monitor to test this feature."
else
    echo "Test 1: Move to next display"
    echo "Expected: Window should move to the next display"
    if prompt "Running: cwm move-display next"; then
        run_cwm move-display next --no-json -v
        verify
    fi
    
    echo ""
    echo "Test 2: Move to previous display"
    echo "Expected: Window should move back to the previous display"
    if prompt "Running: cwm move-display prev"; then
        run_cwm move-display prev --no-json -v
        verify
    fi
    
    echo ""
    echo "Test 3: Move to display by index"
    echo "Expected: Window should move to display 0"
    if prompt "Running: cwm move-display 0"; then
        run_cwm move-display 0 --no-json -v
        verify
    fi
fi

# ----------------------------------------------------------------------------
# Get Window Info Tests
# ----------------------------------------------------------------------------

section "Get Window Info Tests"

echo "Test 1: Get focused window info"
echo "Expected: Should show app name, position, size, and display"
if prompt "Running: cwm get focused"; then
    run_cwm get focused --no-json
    verify
fi

echo ""
echo "Test 2: Get focused window info (JSON)"
echo "Expected: Should show JSON with app, window, and display objects"
if prompt "Running: cwm get focused --json"; then
    run_cwm get focused --json
    verify
fi

echo ""
echo "Test 3: Get specific app window info"
echo "Expected: Should show Finder window info"
if prompt "Running: cwm get window --app Finder"; then
    run_cwm get window --app Finder --no-json
    verify
fi

# ----------------------------------------------------------------------------
# Spotlight Tests
# ----------------------------------------------------------------------------

section "Spotlight Integration Tests"

echo "Test 1: Install spotlight shortcuts"
echo "Expected: Should install shortcuts to ~/Applications/cwm/"
if prompt "Running: cwm spotlight install"; then
    run_cwm spotlight install
    verify
fi

echo ""
echo "Test 2: List installed shortcuts"
echo "Expected: Should show 'Test Focus Finder' and 'Test Maximize'"
if prompt "Running: cwm spotlight list"; then
    run_cwm spotlight list
    verify
fi

echo ""
echo "Test 3: Test Spotlight search"
echo "Expected: Open Spotlight (Cmd+Space) and search for 'cwm: Test'"
if prompt "Open Spotlight and search for 'cwm: Test'"; then
    verify
fi

echo ""
echo "Test 4: Remove spotlight shortcuts"
echo "Expected: Should remove all cwm spotlight shortcuts"
if prompt "Running: cwm spotlight remove --all"; then
    run_cwm spotlight remove --all
    verify
fi

# ----------------------------------------------------------------------------
# Summary
# ----------------------------------------------------------------------------

section "Testing Complete"

echo "Manual testing is complete!"
echo ""
echo "If any tests failed, please check:"
echo "  1. Accessibility permissions are granted"
echo "  2. The target apps are running"
echo "  3. No other window manager is interfering"
echo ""
echo "To report issues, include:"
echo "  - macOS version"
echo "  - cwm version: $(run_cwm version | head -1)"
echo "  - Steps to reproduce"
