#!/bin/sh
# cwm installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/taulfsime/cool-window-manager/main/install.sh | sh

set -e

GITHUB_REPO="taulfsime/cool-window-manager"
INSTALL_DIR=""
CHANNEL=""

# colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # no color

info() {
    printf "${GREEN}$1${NC}\n"
}

warn() {
    printf "${YELLOW}$1${NC}\n"
}

error() {
    printf "${RED}$1${NC}\n"
}

# detect architecture with Rosetta check
detect_arch() {
    ARCH=$(uname -m)
    if [ "$ARCH" = "arm64" ]; then
        # check for Rosetta
        if [ "$(sysctl -n sysctl.proc_translated 2>/dev/null)" = "1" ]; then
            warn "Detected Rosetta emulation"
            printf "Install native Apple Silicon (arm64) or Intel (x86_64) version? [arm64/x86_64]: "
            read -r ARCH_CHOICE
            ARCH="${ARCH_CHOICE:-arm64}"
        fi
    fi
    
    case "$ARCH" in
        arm64|aarch64) echo "aarch64-apple-darwin" ;;
        x86_64) echo "x86_64-apple-darwin" ;;
        *) error "Unsupported architecture: $ARCH"; exit 1 ;;
    esac
}

# select release channel
select_channel() {
    if [ -n "$CHANNEL" ]; then
        return
    fi
    
    echo ""
    echo "Select release channel:"
    echo "  1. Stable (recommended) - Well-tested releases"
    echo "  2. Beta - Preview features, generally stable"
    echo "  3. Dev - Latest features, may be unstable"
    printf "Choice [1]: "
    read -r CHOICE
    
    case "${CHOICE:-1}" in
        1) CHANNEL="stable" ;;
        2) CHANNEL="beta" ;;
        3) CHANNEL="dev" ;;
        *) error "Invalid choice"; exit 1 ;;
    esac
    
    info "✓ Selected: $CHANNEL channel"
}

# detect available installation paths
detect_install_paths() {
    echo ""
    echo "Available installation directories:"
    echo ""
    
    NUM=1
    PATHS=""
    
    # check ~/.local/bin
    DIR="$HOME/.local/bin"
    if [ -d "$DIR" ] || [ ! -e "$DIR" ]; then
        WRITABLE=$([ -w "$DIR" ] || [ -w "$HOME/.local" ] || [ -w "$HOME" ] && echo "✓ writable" || echo "✗ not writable")
        IN_PATH=$(echo "$PATH" | grep -q "$DIR" && echo "✓ in PATH" || echo "✗ not in PATH")
        echo "  $NUM. ~/.local/bin  $WRITABLE  $IN_PATH"
        PATHS="$PATHS $DIR"
        NUM=$((NUM + 1))
    fi
    
    # check ~/.cargo/bin
    DIR="$HOME/.cargo/bin"
    if [ -d "$DIR" ]; then
        WRITABLE=$([ -w "$DIR" ] && echo "✓ writable" || echo "✗ not writable")
        IN_PATH=$(echo "$PATH" | grep -q "$DIR" && echo "✓ in PATH" || echo "✗ not in PATH")
        echo "  $NUM. ~/.cargo/bin  $WRITABLE  $IN_PATH"
        PATHS="$PATHS $DIR"
        NUM=$((NUM + 1))
    fi
    
    # check /usr/local/bin
    DIR="/usr/local/bin"
    if [ -d "$DIR" ]; then
        WRITABLE=$([ -w "$DIR" ] && echo "✓ writable" || echo "✗ needs sudo")
        IN_PATH=$(echo "$PATH" | grep -q "$DIR" && echo "✓ in PATH" || echo "✗ not in PATH")
        echo "  $NUM. /usr/local/bin  $WRITABLE  $IN_PATH"
        PATHS="$PATHS $DIR"
        NUM=$((NUM + 1))
    fi
    
    # check /opt/homebrew/bin
    DIR="/opt/homebrew/bin"
    if [ -d "$DIR" ]; then
        WRITABLE=$([ -w "$DIR" ] && echo "✓ writable" || echo "✗ needs sudo")
        IN_PATH=$(echo "$PATH" | grep -q "$DIR" && echo "✓ in PATH" || echo "✗ not in PATH")
        echo "  $NUM. /opt/homebrew/bin  $WRITABLE  $IN_PATH"
        PATHS="$PATHS $DIR"
        NUM=$((NUM + 1))
    fi
    
    echo "  $NUM. Custom path..."
    echo ""
    printf "Choice [1]: "
    read -r CHOICE
    
    IDX=1
    for DIR in $PATHS; do
        if [ "${CHOICE:-1}" = "$IDX" ]; then
            INSTALL_DIR="$DIR"
            break
        fi
        IDX=$((IDX + 1))
    done
    
    if [ -z "$INSTALL_DIR" ]; then
        printf "Enter custom path: "
        read -r INSTALL_DIR
        # expand tilde
        INSTALL_DIR=$(eval echo "$INSTALL_DIR")
    fi
    
    # create directory if needed
    if [ ! -d "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
    fi
}

# find release for channel
find_release_url() {
    ARCH="$1"
    
    API_URL="https://api.github.com/repos/$GITHUB_REPO/releases"
    RELEASES=$(curl -s "$API_URL")
    
    # find matching release
    ASSET_URL=$(echo "$RELEASES" | grep -o "\"browser_download_url\": *\"[^\"]*${CHANNEL}[^\"]*${ARCH}[^\"]*\.tar\.gz\"" | head -1 | cut -d'"' -f4)
    
    if [ -z "$ASSET_URL" ]; then
        # try fallback channels if stable not found
        if [ "$CHANNEL" = "stable" ]; then
            warn "No stable release found, checking for beta..."
            ASSET_URL=$(echo "$RELEASES" | grep -o "\"browser_download_url\": *\"[^\"]*beta[^\"]*${ARCH}[^\"]*\.tar\.gz\"" | head -1 | cut -d'"' -f4)
            
            if [ -z "$ASSET_URL" ]; then
                warn "No beta release found, checking for dev..."
                ASSET_URL=$(echo "$RELEASES" | grep -o "\"browser_download_url\": *\"[^\"]*dev[^\"]*${ARCH}[^\"]*\.tar\.gz\"" | head -1 | cut -d'"' -f4)
            fi
        fi
    fi
    
    echo "$ASSET_URL"
}

# install shell completions
install_completions() {
    SHELL_NAME=$(basename "$SHELL")
    
    # check if shell is supported
    case "$SHELL_NAME" in
        zsh|bash|fish) ;;
        *)
            # unsupported shell, skip silently
            return 0
            ;;
    esac
    
    echo ""
    printf "Install shell completions for $SHELL_NAME? [Y/n]: "
    read -r INSTALL_COMP
    
    case "${INSTALL_COMP:-y}" in
        [Nn]|[Nn][Oo])
            return 0
            ;;
    esac
    
    # use cwm to install completions
    if "$INSTALL_DIR/cwm" install --completions-only --completions="$SHELL_NAME" 2>/dev/null; then
        : # success message printed by cwm
    else
        warn "Failed to install completions automatically"
        echo "You can install them later with: cwm install --completions-only --completions=$SHELL_NAME"
    fi
}

# download and install
install_cwm() {
    ARCH=$(detect_arch)
    
    echo ""
    info "Fetching latest $CHANNEL release for $ARCH..."
    
    ASSET_URL=$(find_release_url "$ARCH")
    
    if [ -z "$ASSET_URL" ]; then
        error "No release found for $CHANNEL channel on $ARCH"
        error "Please check https://github.com/$GITHUB_REPO/releases"
        exit 1
    fi
    
    # extract version info from filename
    FILENAME=$(basename "$ASSET_URL")
    VERSION=$(echo "$FILENAME" | sed 's/cwm-\(.*\)-'"$ARCH"'\.tar\.gz/\1/')
    
    info "Installing cwm ($VERSION)..."
    
    # create temp directory
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    # download binary
    echo "Downloading..."
    curl -L "$ASSET_URL" -o "$TEMP_DIR/cwm.tar.gz" --progress-bar
    
    # download checksum
    curl -sL "$ASSET_URL.sha256" -o "$TEMP_DIR/checksum.txt"
    
    # verify checksum
    echo "Verifying checksum..."
    EXPECTED_HASH=$(cut -d' ' -f1 "$TEMP_DIR/checksum.txt")
    
    if command -v shasum >/dev/null 2>&1; then
        ACTUAL_HASH=$(shasum -a 256 "$TEMP_DIR/cwm.tar.gz" | cut -d' ' -f1)
    elif command -v sha256sum >/dev/null 2>&1; then
        ACTUAL_HASH=$(sha256sum "$TEMP_DIR/cwm.tar.gz" | cut -d' ' -f1)
    else
        warn "No checksum tool found, skipping verification"
        ACTUAL_HASH="$EXPECTED_HASH"
    fi
    
    if [ "$ACTUAL_HASH" != "$EXPECTED_HASH" ]; then
        error "Checksum verification failed!"
        error "Expected: $EXPECTED_HASH"
        error "Actual:   $ACTUAL_HASH"
        exit 1
    fi
    info "✓ Checksum verified"
    
    # extract
    echo "Extracting..."
    tar -xzf "$TEMP_DIR/cwm.tar.gz" -C "$TEMP_DIR"
    
    # install
    if [ -w "$INSTALL_DIR" ]; then
        mv "$TEMP_DIR/cwm" "$INSTALL_DIR/cwm"
    else
        info "Installing to $INSTALL_DIR (requires sudo)..."
        sudo mv "$TEMP_DIR/cwm" "$INSTALL_DIR/cwm"
    fi
    
    chmod +x "$INSTALL_DIR/cwm"
    
    # install man page
    MAN_DIR="/usr/local/share/man/man1"
    if [ -f "$TEMP_DIR/cwm.1" ]; then
        echo "Installing man page..."
        if [ ! -d "$MAN_DIR" ]; then
            if sudo mkdir -p "$MAN_DIR" 2>/dev/null; then
                :
            else
                warn "Could not create $MAN_DIR, skipping man page"
            fi
        fi
        if [ -d "$MAN_DIR" ]; then
            if [ -w "$MAN_DIR" ]; then
                cp "$TEMP_DIR/cwm.1" "$MAN_DIR/cwm.1"
                info "✓ Man page installed (run 'man cwm' to view)"
            elif sudo cp "$TEMP_DIR/cwm.1" "$MAN_DIR/cwm.1" 2>/dev/null; then
                info "✓ Man page installed (run 'man cwm' to view)"
            else
                warn "Could not install man page to $MAN_DIR"
            fi
        fi
    fi
    
    # save version info
    mkdir -p "$HOME/.cwm"
    cat > "$HOME/.cwm/version.json" <<EOF
{
  "current": "$VERSION",
  "previous": null,
  "last_seen_available": null,
  "install_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "install_path": "$INSTALL_DIR/cwm"
}
EOF
    
    # create default config if not exists
    if [ ! -f "$HOME/.cwm/config.json" ]; then
        CHANNELS_CONFIG='{"dev": false, "beta": false, "stable": true}'
        if [ "$CHANNEL" = "beta" ]; then
            CHANNELS_CONFIG='{"dev": false, "beta": true, "stable": true}'
        elif [ "$CHANNEL" = "dev" ]; then
            CHANNELS_CONFIG='{"dev": true, "beta": true, "stable": true}'
        fi
        
        cat > "$HOME/.cwm/config.json" <<EOF
{
  "shortcuts": [],
  "app_rules": [],
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
      "channels": $CHANNELS_CONFIG,
      "telemetry": {
        "enabled": false,
        "include_system_info": false
      }
    }
  }
}
EOF
    fi
    
    echo ""
    info "✓ cwm installed successfully!"
    echo ""
    
    # install shell completions
    install_completions
    
    # check PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        warn "⚠️  $INSTALL_DIR is not in your PATH"
        echo ""
        
        # detect shell
        SHELL_NAME=$(basename "$SHELL")
        case "$SHELL_NAME" in
            zsh) RC_FILE="~/.zshrc" ;;
            bash) RC_FILE="~/.bashrc" ;;
            fish) RC_FILE="~/.config/fish/config.fish" ;;
            *) RC_FILE="~/.profile" ;;
        esac
        
        echo "Add this line to your $RC_FILE:"
        if [ "$SHELL_NAME" = "fish" ]; then
            echo "  set -gx PATH $INSTALL_DIR \$PATH"
        else
            echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        fi
        echo ""
        echo "Then reload your shell or run: source $RC_FILE"
        echo ""
        
        printf "Would you like to add it automatically? [y/N]: "
        read -r ADD_PATH
        if [ "$ADD_PATH" = "y" ] || [ "$ADD_PATH" = "Y" ]; then
            RC_FILE_EXPANDED=$(eval echo "$RC_FILE")
            if [ "$SHELL_NAME" = "fish" ]; then
                echo "set -gx PATH $INSTALL_DIR \$PATH" >> "$RC_FILE_EXPANDED"
            else
                echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$RC_FILE_EXPANDED"
            fi
            info "✓ Added to $RC_FILE"
            echo "Please run: source $RC_FILE"
        fi
    else
        echo "Run 'cwm --help' to get started!"
        echo "Run 'cwm check-permissions --prompt' to enable accessibility"
    fi
}

# parse arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --channel)
            CHANNEL="$2"
            shift 2
            ;;
        --dev)
            CHANNEL="dev"
            shift
            ;;
        --beta)
            CHANNEL="beta"
            shift
            ;;
        --stable)
            CHANNEL="stable"
            shift
            ;;
        --path)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "cwm installer"
            echo ""
            echo "Usage: install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --channel <CHANNEL>  Release channel (stable, beta, dev)"
            echo "  --stable             Install stable release (default)"
            echo "  --beta               Install beta release"
            echo "  --dev                Install dev release"
            echo "  --path <PATH>        Installation directory"
            echo "  --help, -h           Show this help"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# main
echo "Welcome to cwm installer!"

select_channel

if [ -z "$INSTALL_DIR" ]; then
    detect_install_paths
fi

install_cwm
