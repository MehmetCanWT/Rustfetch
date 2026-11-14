#!/usr/bin/env bash
# ==============================================================================
# RustFetch — Universal One-Line Installer
# Supported: Arch, Debian, Ubuntu, Fedora, Alpine, Void, NixOS, openSUSE, etc.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/rustfetch/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/rustfetch/main/install.sh | RUSTFETCH_VERSION=v0.1.5 bash
# or locally:
#   ./install.sh
#
# Environment variables:
#   RUSTFETCH_VERSION       Git tag/ref to build (default: default branch)
#   RUSTFETCH_MODE          2d|3d default display mode for a fresh config
#   RUSTFETCH_FORCE_CONFIG  1 to regenerate an existing config.toml
#   NONINTERACTIVE          non-empty/1 to skip the interactive prompt
#   CI                      standard CI flag; also skips the interactive prompt
# ==============================================================================

set -eo pipefail

REPO="MehmetCanWT/rustfetch"
GITHUB_URL="https://github.com/${REPO}"
MIN_RUST_MINOR=88
MIN_RUST_VERSION="1.${MIN_RUST_MINOR}"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() {
    printf '%b==>%b %s\n' "${BLUE}" "${NC}" "$*"
}

warn() {
    printf '%bWarning:%b %s\n' "${YELLOW}" "${NC}" "$*" >&2
}

die() {
    printf '%bError:%b %s\n' "${RED}" "${NC}" "$*" >&2
    exit 1
}

echo -e "${BLUE}==>${NC} Installing ${GREEN}RustFetch${NC}..."

# 0. Ensure Homebrew (Bazzite) and Cargo environments are loaded if available
for dir in "/home/linuxbrew/.linuxbrew/bin" "$HOME/.cargo/bin" "$HOME/.local/bin"; do
    if [ -d "$dir" ] && [[ ":$PATH:" != *":$dir:"* ]]; then
        export PATH="$dir:$PATH"
    fi
done
if ! command -v cargo &>/dev/null && [ -f "$HOME/.cargo/env" ]; then
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env"
fi

# 1. Detect OS and Architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

if [ "$OS" != "linux" ]; then
    die "RustFetch currently supports Linux only."
fi

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        die "Unsupported architecture: ${ARCH} (supported: x86_64, aarch64)"
        ;;
esac

# Detect libc (musl vs gnu, e.g. Alpine Linux)
IS_MUSL=false
if [ -f /etc/alpine-release ] || (ldd --version 2>&1 | grep -iq musl); then
    IS_MUSL=true
fi
LIBC="gnu"
if [ "$IS_MUSL" = "true" ]; then
    LIBC="musl"
fi
log "Detected target: ${TARGET_ARCH}-${LIBC}"

TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

ensure_build_tools() {
    if ! command -v cargo &>/dev/null; then
        echo -e "${YELLOW}==>${NC} Cargo is not installed. Installing minimal Rust toolchain..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
        export PATH="$HOME/.cargo/bin:$PATH"
        # shellcheck source=/dev/null
        . "$HOME/.cargo/env" 2>/dev/null || true
    fi

    # Check for C compiler/linker
    if ! command -v cc &>/dev/null && ! command -v gcc &>/dev/null && ! command -v clang &>/dev/null; then
        echo -e "${YELLOW}==>${NC} C compiler not found. Attempting to install build tools..."
        if [ -f /run/ostree-booted ] || command -v rpm-ostree &>/dev/null; then
            if command -v brew &>/dev/null; then
                brew install gcc || true
            fi
            if ! command -v cc &>/dev/null && ! command -v gcc &>/dev/null && [ -c /dev/tty ]; then
                sudo </dev/tty rpm-ostree install --apply-live --idempotent -y gcc || true
            fi
        elif command -v apt-get &>/dev/null && [ -c /dev/tty ]; then
            sudo </dev/tty apt-get update -qq && sudo </dev/tty apt-get install -y -qq build-essential || true
        elif command -v dnf &>/dev/null && [ -c /dev/tty ]; then
            sudo </dev/tty dnf install -y -q gcc || true
        elif command -v pacman &>/dev/null && [ -c /dev/tty ]; then
            sudo </dev/tty pacman -Sy --noconfirm base-devel || true
        elif command -v apk &>/dev/null && [ -c /dev/tty ]; then
            sudo </dev/tty apk add gcc musl-dev || true
        fi
    fi
}

# Returns success when the active rustc is at least MIN_RUST_VERSION (1.88).
compiler_supports_msrv() {
    local version major minor
    version="$(rustc --version 2>/dev/null | awk '{print $2}')" || return 1
    [ -n "$version" ] || return 1
    major="${version%%.*}"
    minor="${version#*.}"
    minor="${minor%%.*}"
    case "$major" in
        ''|*[!0-9]*) return 1 ;;
    esac
    case "$minor" in
        ''|*[!0-9]*) return 1 ;;
    esac
    if [ "$major" -gt 1 ]; then
        return 0
    fi
    [ "$major" -eq 1 ] && [ "$minor" -ge "$MIN_RUST_MINOR" ]
}

ensure_msrv() {
    if compiler_supports_msrv; then
        return 0
    fi
    local current
    current="$(rustc --version 2>/dev/null || true)"
    warn "Rust ${current:-(not installed)} is older than the required ${MIN_RUST_VERSION}."
    if command -v rustup &>/dev/null; then
        echo -e "${BLUE}==>${NC} Updating stable Rust toolchain via rustup..."
        rustup update stable >/dev/null 2>&1 || true
        rustup default stable >/dev/null 2>&1 || true
        if compiler_supports_msrv; then
            log "Rust toolchain updated: $(rustc --version)"
            return 0
        fi
    fi
    die "Rust >= ${MIN_RUST_VERSION} is required. Install it via https://rustup.rs and re-run this installer."
}

# 2. Compile binary (atomic install helper is defined below, before use)
#    Local checkout detection: the workspace root manifest does not name the
#    binary, so look at cli/Cargo.toml instead of grepping the root file.
if [ -z "${RUSTFETCH_VERSION:-}" ] \
    && [ -f "Cargo.toml" ] \
    && [ -f "cli/Cargo.toml" ] \
    && grep -q 'name = "rustfetch"' cli/Cargo.toml; then
    ensure_build_tools
    ensure_msrv
    BUILD_DIR="$(pwd)"
    log "Compiling local release binary (LTO optimized)..."
    cargo build --release --locked -p rustfetch
else
    ensure_build_tools
    ensure_msrv

    SRC_DIR="$TEMP_DIR/source"
    if ! command -v git &>/dev/null; then
        die "git is required to fetch the RustFetch sources."
    fi

    if [ -n "${RUSTFETCH_VERSION:-}" ]; then
        CLEAN_VERSION="${RUSTFETCH_VERSION#v}"
        log "Fetching RustFetch source for '${RUSTFETCH_VERSION}' from ${GITHUB_URL}..."
        # Try explicit tag forms first; if no git tag exists yet, fall back to main.
        if ! git clone --depth 1 --branch "v${CLEAN_VERSION}" "${GITHUB_URL}.git" "$SRC_DIR" 2>/dev/null \
            && ! git clone --depth 1 --branch "${CLEAN_VERSION}" "${GITHUB_URL}.git" "$SRC_DIR" 2>/dev/null; then
            warn "Tag '${RUSTFETCH_VERSION}' not found; building latest main branch instead."
            rm -rf "$SRC_DIR"
            git clone --depth 1 "${GITHUB_URL}.git" "$SRC_DIR"
        fi
    else
        log "Fetching latest source code from ${GITHUB_URL}..."
        git clone --depth 1 "${GITHUB_URL}.git" "$SRC_DIR"
    fi

    log "Compiling release binary (LTO optimized)..."
    (cd "$SRC_DIR" && cargo build --release --locked -p rustfetch)
    BUILD_DIR="$SRC_DIR"
fi

# 3. Locate binary
BINARY_PATH=""
if [ -f "$BUILD_DIR/target/release/rustfetch" ]; then
    BINARY_PATH="$BUILD_DIR/target/release/rustfetch"
elif [ -f "$BUILD_DIR/rustfetch" ]; then
    BINARY_PATH="$BUILD_DIR/rustfetch"
else
    BINARY_PATH="$(find "$BUILD_DIR" -type f -name rustfetch -perm -111 | head -n 1)"
fi

if [ -z "$BINARY_PATH" ] || [ ! -f "$BINARY_PATH" ]; then
    die "Could not find compiled rustfetch binary."
fi

# Atomically replace a destination file: write next to it, then rename.
install_atomic() {
    local src="$1" dst="$2" mode="${3:-755}" tmp
    tmp="$(mktemp "${dst}.tmp.XXXXXX")" || return 1
    if install -m "$mode" "$src" "$tmp" 2>/dev/null; then
        mv -f "$tmp" "$dst"
        return 0
    fi
    rm -f "$tmp"
    return 1
}

# 4. Determine installation destination
SYSTEM_BIN="/usr/local/bin"
USER_BIN="${XDG_BIN_HOME:-$HOME/.local/bin}"

if [ -n "${RUSTFETCH_INSTALL_DIR:-}" ]; then
    FINAL_BIN_DIR="$RUSTFETCH_INSTALL_DIR"
elif [ "$EUID" -eq 0 ] || [ -w "$SYSTEM_BIN" ]; then
    FINAL_BIN_DIR="$SYSTEM_BIN"
else
    FINAL_BIN_DIR="$USER_BIN"
fi

mkdir -p "$FINAL_BIN_DIR"
if ! install_atomic "$BINARY_PATH" "$FINAL_BIN_DIR/rustfetch" 755; then
    die "Failed to install rustfetch to ${FINAL_BIN_DIR}."
fi

# Install rfetch binary (prefer compiled binary if present, fallback to symlink)
RFETCH_SRC=""
if [ -f "$BUILD_DIR/target/release/rfetch" ]; then
    RFETCH_SRC="$BUILD_DIR/target/release/rfetch"
elif [ -f "$BUILD_DIR/rfetch" ]; then
    RFETCH_SRC="$BUILD_DIR/rfetch"
fi

if [ -n "$RFETCH_SRC" ]; then
    if ! install_atomic "$RFETCH_SRC" "$FINAL_BIN_DIR/rfetch" 755; then
        warn "Could not install the rfetch binary; falling back to a symlink."
        ln -sfn "$FINAL_BIN_DIR/rustfetch" "$FINAL_BIN_DIR/rfetch"
    fi
else
    ln -sfn "$FINAL_BIN_DIR/rustfetch" "$FINAL_BIN_DIR/rfetch"
fi

# Validate the freshly installed binary before touching user configuration.
INSTALLED_BIN="$FINAL_BIN_DIR/rustfetch"
if [ ! -x "$INSTALLED_BIN" ]; then
    die "Installed binary ${INSTALLED_BIN} is not executable."
fi
if ! "$INSTALLED_BIN" --version >/dev/null 2>&1; then
    die "Installed binary ${INSTALLED_BIN} failed to run (--version)."
fi

# 5. Create config directory
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/rustfetch"
mkdir -p "$CONFIG_DIR/logos"

# 6. Install shell completions
log "Installing shell auto-completions..."
RFETCH_BIN="$FINAL_BIN_DIR/rfetch"
BASH_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions"
mkdir -p "$BASH_DIR"
if "$INSTALLED_BIN" --completions bash > "$BASH_DIR/rustfetch.tmp" 2>/dev/null; then
    mv -f "$BASH_DIR/rustfetch.tmp" "$BASH_DIR/rustfetch"
    if [ -x "$RFETCH_BIN" ] && "$RFETCH_BIN" --completions bash > "$BASH_DIR/rfetch.tmp" 2>/dev/null; then
        mv -f "$BASH_DIR/rfetch.tmp" "$BASH_DIR/rfetch"
    else
        rm -f "$BASH_DIR/rfetch.tmp"
    fi
else
    rm -f "$BASH_DIR/rustfetch.tmp"
    warn "Could not generate bash completions."
fi

ZSH_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/zsh/site-functions"
mkdir -p "$ZSH_DIR"
if "$INSTALLED_BIN" --completions zsh > "$ZSH_DIR/_rustfetch.tmp" 2>/dev/null; then
    mv -f "$ZSH_DIR/_rustfetch.tmp" "$ZSH_DIR/_rustfetch"
    if [ -x "$RFETCH_BIN" ] && "$RFETCH_BIN" --completions zsh > "$ZSH_DIR/_rfetch.tmp" 2>/dev/null; then
        mv -f "$ZSH_DIR/_rfetch.tmp" "$ZSH_DIR/_rfetch"
    else
        rm -f "$ZSH_DIR/_rfetch.tmp"
    fi
else
    rm -f "$ZSH_DIR/_rustfetch.tmp"
    warn "Could not generate zsh completions."
fi

FISH_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
mkdir -p "$FISH_DIR"
if "$INSTALLED_BIN" --completions fish > "$FISH_DIR/rustfetch.fish.tmp" 2>/dev/null; then
    mv -f "$FISH_DIR/rustfetch.fish.tmp" "$FISH_DIR/rustfetch.fish"
    if [ -x "$RFETCH_BIN" ] && "$RFETCH_BIN" --completions fish > "$FISH_DIR/rfetch.fish.tmp" 2>/dev/null; then
        mv -f "$FISH_DIR/rfetch.fish.tmp" "$FISH_DIR/rfetch.fish"
    else
        rm -f "$FISH_DIR/rfetch.fish.tmp"
    fi
else
    rm -f "$FISH_DIR/rustfetch.fish.tmp"
    warn "Could not generate fish completions."
fi

# 7. Install man pages
MAN_SRC=""
if [ -f "$BUILD_DIR/docs/rustfetch.1" ]; then
    MAN_SRC="$BUILD_DIR/docs/rustfetch.1"
elif [ -f "docs/rustfetch.1" ]; then
    MAN_SRC="docs/rustfetch.1"
fi

if [ -n "$MAN_SRC" ]; then
    log "Installing man pages..."
    SYS_MAN_DIR="/usr/local/share/man/man1"
    USER_MAN_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/man/man1"
    MAN_DEST_DIR=""

    # /usr/local/share may not exist yet; create it when we can, otherwise fall
    # back to the per-user data directory.
    if [ "$FINAL_BIN_DIR" = "$SYSTEM_BIN" ] \
        && mkdir -p "$SYS_MAN_DIR" 2>/dev/null \
        && [ -w "$SYS_MAN_DIR" ]; then
        MAN_DEST_DIR="$SYS_MAN_DIR"
    else
        if mkdir -p "$USER_MAN_DIR" 2>/dev/null; then
            MAN_DEST_DIR="$USER_MAN_DIR"
        fi
    fi

    if [ -n "$MAN_DEST_DIR" ]; then
        if install_atomic "$MAN_SRC" "$MAN_DEST_DIR/rustfetch.1" 644; then
            ln -sfn rustfetch.1 "$MAN_DEST_DIR/rfetch.1"
        else
            warn "Could not install the man page to ${MAN_DEST_DIR}."
        fi
    else
        warn "Could not find a writable directory for the man page."
    fi
fi

# 8. Configure default display mode (preserve existing user config on updates!)
CONFIG_FILE="$CONFIG_DIR/config.toml"

INTERACTIVE_INSTALL=false
if [ -c /dev/tty ] \
    && [ "${CI:-0}" != "1" ] \
    && [ "${NONINTERACTIVE:-0}" != "1" ] \
    && [ "${DEBIAN_FRONTEND:-}" != "noninteractive" ]; then
    INTERACTIVE_INSTALL=true
fi

write_default_config() {
    local target="$1" tmp
    tmp="$(mktemp "${target}.tmp.XXXXXX")" || return 1
    if "$INSTALLED_BIN" --print-default-config > "$tmp" 2>/dev/null && [ -s "$tmp" ]; then
        mv -f "$tmp" "$target"
        return 0
    fi
    rm -f "$tmp"
    return 1
}

if [ -s "$CONFIG_FILE" ] && [ "${RUSTFETCH_FORCE_CONFIG:-0}" != "1" ]; then
    echo ""
    echo -e "${GREEN}==>${NC} Existing user configuration found at ${BLUE}${CONFIG_FILE}${NC} (preserved without changes)."
    write_default_config "$CONFIG_DIR/config.toml.default" || warn "Could not write config.toml.default."
else
    echo ""
    echo -e "${BLUE}==>${NC} Choose default logo display mode:"
    echo -e "  ${GREEN}1)${NC} Standard 2D   — Classic fastfetch-style static ASCII (~4ms, recommended for .bashrc) [Default]"
    echo -e "  ${GREEN}2)${NC} 3D Animated   — Real-time spinning 3D ASCII relief logo (can also run anytime with 'rfetch --3d')"

    MODE_CHOICE="2d"
    if [ -n "${RUSTFETCH_MODE:-}" ]; then
        case "$RUSTFETCH_MODE" in
            2|"3d"|"animated"|"three_d") MODE_CHOICE="3d" ;;
            *) MODE_CHOICE="2d" ;;
        esac
    elif [ "$INTERACTIVE_INSTALL" = "true" ]; then
        read -rp "Select mode [1-2] (default: 1): " USER_INPUT </dev/tty 2>/dev/null || USER_INPUT="1"
        case "$USER_INPUT" in
            2|"3d"|"animated"|"three_d")
                MODE_CHOICE="3d"
                ;;
            *)
                MODE_CHOICE="2d"
                ;;
        esac
    else
        MODE_CHOICE="2d"
    fi

    if ! write_default_config "$CONFIG_FILE"; then
        die "Failed to generate the default configuration at ${CONFIG_FILE}."
    fi

    if [ "$MODE_CHOICE" = "3d" ]; then
        echo -e "${GREEN}==>${NC} Configured default mode: ${BLUE}3D Animated${NC} (3d = true)"
        if grep -q "\[general\.logo\.three_d\]" "$CONFIG_FILE"; then
            sed -i '/\[general\.logo\.three_d\]/,/^\[/ s/enabled = .*/enabled = true/' "$CONFIG_FILE"
        fi
    else
        echo -e "${GREEN}==>${NC} Configured default mode: ${BLUE}Standard 2D${NC} (3d = false)"
        if grep -q "\[general\.logo\.three_d\]" "$CONFIG_FILE"; then
            sed -i '/\[general\.logo\.three_d\]/,/^\[/ s/enabled = .*/enabled = false/' "$CONFIG_FILE"
        fi
        if grep -q "3d = true" "$CONFIG_FILE"; then
            sed -i 's/3d = true/3d = false/' "$CONFIG_FILE"
        fi
    fi

    # 9. Install shell startup integration: fresh config + interactive install +
    #    a verified working binary only. Never touch shell rc files in CI or
    #    when the user piped the installer non-interactively.
    if [ "$INTERACTIVE_INSTALL" = "true" ] && "$INSTALLED_BIN" --version >/dev/null 2>&1; then
        log "Installing shell startup integration..."
        if ! "$INSTALLED_BIN" --install-hook 2>/dev/null; then
            warn "Could not install the shell startup hook; run 'rustfetch --install-hook' manually."
        fi
    fi
fi

echo ""
echo -e "${GREEN}==> Installation completed successfully!${NC}"
echo -e "Binaries installed to: ${BLUE}${FINAL_BIN_DIR}${NC}"
echo -e "You can now run '${GREEN}rustfetch${NC}' or '${GREEN}rfetch${NC}' in your terminal."

if [ "$FINAL_BIN_DIR" = "$USER_BIN" ]; then
    case ":$PATH:" in
        *":$USER_BIN:"*) ;;
        *)
            echo ""
            echo -e "${YELLOW}Note:${NC} '${USER_BIN}' is not in your \$PATH."
            echo -e "Add it by adding this line to your ~/.bashrc or ~/.zshrc:"
            echo -e "  ${BLUE}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
            ;;
    esac
fi
echo ""
