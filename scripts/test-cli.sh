#!/usr/bin/env bash
# End-to-end checks for public CLI behavior. Invoked by scripts/test.sh.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR" || exit 1

if [[ $# -ne 1 ]]; then
    printf 'usage: %s <temporary-directory>\n' "$0" >&2
    exit 64
fi

TMP_DIR="$1"
BIN="target/debug/rustfetch"

# Keep the developer's real Cargo/Rustup homes: only HOME and the XDG
# directories must be isolated, otherwise cargo would re-download the registry.
REAL_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
REAL_RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"

# Full isolation: HOME and every XDG directory point into the temporary dir, so
# the tests can never touch the user's real config, cache, completions, shell
# rc files or logo directories.
export HOME="$TMP_DIR/home"
mkdir -p "$HOME"
export CARGO_HOME="$REAL_CARGO_HOME"
export RUSTUP_HOME="$REAL_RUSTUP_HOME"
export XDG_CONFIG_HOME="$TMP_DIR/config"
export XDG_CACHE_HOME="$TMP_DIR/cache"
export XDG_DATA_HOME="$TMP_DIR/data"
export XDG_STATE_HOME="$TMP_DIR/state"
export NO_COLOR=1

note() {
    printf '\n==> %s\n' "$*"
}

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

# rg is preferred, but it is optional: fall back to fixed-string grep.
SEARCH_BIN="grep"
if command -v rg >/dev/null 2>&1; then
    SEARCH_BIN="rg"
fi

assert_contains() {
    local pattern="$1" file="$2"
    [ -f "$file" ] || fail "missing file: $file"
    if ! "$SEARCH_BIN" -qF -- "$pattern" "$file"; then
        fail "expected to find '$pattern' in $file"
    fi
}

assert_not_contains() {
    local pattern="$1" file="$2"
    [ -f "$file" ] || fail "missing file: $file"
    if "$SEARCH_BIN" -qF -- "$pattern" "$file"; then
        fail "did not expect '$pattern' in $file"
    fi
}

assert_nonempty() {
    [ -s "$1" ] || fail "expected a non-empty file: $1"
}

expect_exit() {
    local expected="$1" actual=0
    shift
    set +e
    "$@" >/dev/null 2>&1
    actual=$?
    set -e
    if [ "$actual" -ne "$expected" ]; then
        fail "expected exit code $expected, got $actual: $*"
    fi
}

DEFAULT_CONFIG="$TMP_DIR/default-config.toml"
JSON_OUTPUT="$TMP_DIR/output.json"
COMPLETION_OUTPUT="$TMP_DIR/rustfetch.bash"
FRAME_OUTPUT="$TMP_DIR/image-frame.txt"

note "cargo build -p rustfetch --locked"
cargo build -p rustfetch --locked

note "$BIN --version"
"$BIN" --version > "$TMP_DIR/version.txt"
assert_contains "rustfetch" "$TMP_DIR/version.txt"

note "$BIN --print-default-config"
"$BIN" --print-default-config > "$DEFAULT_CONFIG"
assert_nonempty "$DEFAULT_CONFIG"

# --dump-config must print the ACTIVE configuration, not the default template.
note "$BIN --config <custom> --dump-config"
CUSTOM_CONFIG="$TMP_DIR/custom-config.toml"
printf '[general]\nseparator = "@@"\n' > "$CUSTOM_CONFIG"
"$BIN" --config "$CUSTOM_CONFIG" --dump-config > "$TMP_DIR/dump-config.txt"
assert_nonempty "$TMP_DIR/dump-config.txt"
assert_contains 'separator = "@@"' "$TMP_DIR/dump-config.txt"

# Invalid -c must fail with the documented "runtime error" exit code (1).
note "$BIN --config <invalid> (expect exit 1)"
printf 'not valid toml [[[\n' > "$TMP_DIR/bad-config.toml"
expect_exit 1 "$BIN" --config "$TMP_DIR/bad-config.toml" --no-logo

# Empty --shading is a usage error (clap exits with 2).
note "$BIN --shading '' (expect exit 2)"
expect_exit 2 "$BIN" --3d --shading "" --frames 1 --no-update-check

note "$BIN --3d --shading <mode> (dot/wireframe/solid)"
for shading in dot wireframe solid; do
    "$BIN" --3d --shading "$shading" --frames 1 --no-update-check > /dev/null 2>&1 \
        || fail "--shading $shading should be accepted"
done

note "$BIN --json --no-logo"
"$BIN" --json --no-logo --no-update-check > "$JSON_OUTPUT"
assert_nonempty "$JSON_OUTPUT"
if command -v python3 >/dev/null 2>&1; then
    python3 -c 'import json, sys; value = json.load(open(sys.argv[1], encoding="utf-8")); assert isinstance(value, list), "expected a JSON array"' "$JSON_OUTPUT" \
        || fail "--json did not produce a valid JSON array"
else
    assert_contains '"name"' "$JSON_OUTPUT"
fi

note "$BIN --waybar"
WAYBAR_OUTPUT="$TMP_DIR/output_waybar.json"
"$BIN" --waybar --no-update-check > "$WAYBAR_OUTPUT"
assert_contains '"class":"rustfetch"' "$WAYBAR_OUTPUT"
assert_contains '"tooltip"' "$WAYBAR_OUTPUT"
assert_contains '"text"' "$WAYBAR_OUTPUT"

note "$BIN --format waybar / json"
"$BIN" --format waybar --no-update-check > "$WAYBAR_OUTPUT"
assert_contains '"class":"rustfetch"' "$WAYBAR_OUTPUT"
"$BIN" --format WAYBAR --no-update-check > "$WAYBAR_OUTPUT"
assert_contains '"class":"rustfetch"' "$WAYBAR_OUTPUT"
"$BIN" --format json --no-update-check > "$TMP_DIR/output_format_json.json"
assert_contains '"name"' "$TMP_DIR/output_format_json.json"

note "$BIN --no-logo --border --border-image-position frame"
"$BIN" --no-logo --border --border-title "System" \
    --border-image images/arch.png --border-image-width 4 \
    --border-image-position frame --no-update-check > "$FRAME_OUTPUT"
assert_contains 'System' "$FRAME_OUTPUT"
assert_contains 'OS' "$FRAME_OUTPUT"

note "$BIN --preset <preset> --json --no-logo"
for preset in default minimal clean dots card brackets neofetch retro modern compact; do
    "$BIN" --preset "$preset" --json --no-logo --no-update-check > /dev/null \
        || fail "preset $preset failed"
done

note "$BIN --completions bash"
"$BIN" --completions bash > "$COMPLETION_OUTPUT"
assert_contains 'rustfetch' "$COMPLETION_OUTPUT"

note "$BIN --init bash"
"$BIN" --init bash > /dev/null

note "$BIN --preset does-not-exist (expect non-zero)"
if "$BIN" --preset does-not-exist --no-logo --no-update-check > /dev/null 2>&1; then
    fail "unknown presets must return a non-zero status"
fi

# Piped / redirected output must be free of ANSI escapes and CRLF.
note "$BIN --no-logo > file (ANSI/CRLF free)"
"$BIN" --no-logo --no-update-check > "$TMP_DIR/pipe-output.txt"
assert_nonempty "$TMP_DIR/pipe-output.txt"
if LC_ALL=C grep -q $'\x1b' "$TMP_DIR/pipe-output.txt"; then
    fail "piped output must not contain ANSI escape sequences"
fi
if LC_ALL=C grep -q $'\r' "$TMP_DIR/pipe-output.txt"; then
    fail "piped output must not contain CRLF line endings"
fi

note "$BIN update notification and --self-update / --update checks"
UPDATE_OUTPUT="$TMP_DIR/update-output.txt"
COLUMNS=120 RUSTFETCH_LATEST_VERSION="9.9.9" "$BIN" --no-logo > "$UPDATE_OUTPUT"
assert_contains 'Update available' "$UPDATE_OUTPUT"
assert_contains 'rustfetch --self-update' "$UPDATE_OUTPUT"

NO_UPDATE_OUTPUT="$TMP_DIR/no-update-output.txt"
RUSTFETCH_LATEST_VERSION="9.9.9" "$BIN" --no-logo --no-update-check > "$NO_UPDATE_OUTPUT"
assert_not_contains 'Update available' "$NO_UPDATE_OUTPUT"

"$BIN" --help > "$TMP_DIR/help.txt"
assert_contains '--self-update' "$TMP_DIR/help.txt"
assert_contains '--update' "$TMP_DIR/help.txt"

mkdir -p "$XDG_CONFIG_HOME/rustfetch"
printf '# keep user config\n[general]\npadding = 4\n' > "$XDG_CONFIG_HOME/rustfetch/config.toml"
RUSTFETCH_LATEST_VERSION="0.1.0" "$BIN" --self-update > "$TMP_DIR/self-update.txt"
assert_contains 'already up to date' "$TMP_DIR/self-update.txt"
RUSTFETCH_LATEST_VERSION="0.1.0" "$BIN" --update > "$TMP_DIR/update-alias.txt"
assert_contains 'already up to date' "$TMP_DIR/update-alias.txt"
assert_contains '# keep user config' "$XDG_CONFIG_HOME/rustfetch/config.toml"

note "$BIN discovery, doctor, and cat mascot checks"
"$BIN" --list-modules > "$TMP_DIR/list-modules.txt"
assert_contains 'os_age' "$TMP_DIR/list-modules.txt"
"$BIN" --list-themes > "$TMP_DIR/list-themes.txt"
assert_contains 'catppuccin' "$TMP_DIR/list-themes.txt"
"$BIN" --list-presets > "$TMP_DIR/list-presets.txt"
assert_contains 'modern' "$TMP_DIR/list-presets.txt"
"$BIN" --list-logos > "$TMP_DIR/list-logos.txt"
assert_contains 'bengal' "$TMP_DIR/list-logos.txt"
"$BIN" --clean-cache > "$TMP_DIR/clean-cache.txt"
assert_contains 'Cleared' "$TMP_DIR/clean-cache.txt"
RUSTFETCH_LATEST_VERSION="0.1.2" "$BIN" --doctor --no-update-check > "$TMP_DIR/doctor.txt"
assert_contains 'Diagnostics' "$TMP_DIR/doctor.txt"
"$BIN" -l bengal --no-update-check > "$TMP_DIR/bengal.txt"
assert_contains 'Bengal' "$TMP_DIR/bengal.txt"
"$BIN" -l turkish_van --no-update-check > "$TMP_DIR/van.txt"
assert_contains 'Turkish Van' "$TMP_DIR/van.txt"

# Shell hook: user content must survive, the managed block must be idempotent.
note "$BIN --install-hook (isolated HOME)"
BASHRC="$HOME/.bashrc"
printf '# user bashrc line\nexport PATH="$PATH:/opt/example"\n' > "$BASHRC"
SHELL=/bin/bash "$BIN" --install-hook > "$TMP_DIR/hook-install.log" 2>&1 \
    || fail "--install-hook failed"
assert_contains '# user bashrc line' "$BASHRC"
assert_contains '# >>> rustfetch hook >>>' "$BASHRC"
assert_contains '# <<< rustfetch hook <<<' "$BASHRC"

SHELL=/bin/bash "$BIN" --install-hook > /dev/null 2>&1 \
    || fail "second --install-hook failed"
HOOK_COUNT="$(grep -cF '# >>> rustfetch hook >>>' "$BASHRC" || true)"
[ "$HOOK_COUNT" -eq 1 ] || fail "hook block is not idempotent (count=$HOOK_COUNT)"

SHELL=/bin/bash "$BIN" --uninstall-hook > /dev/null 2>&1 \
    || fail "--uninstall-hook failed"
assert_contains '# user bashrc line' "$BASHRC"
assert_not_contains '# >>> rustfetch hook >>>' "$BASHRC"

note "CLI checks passed"
