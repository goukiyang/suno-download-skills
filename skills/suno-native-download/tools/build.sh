#!/bin/sh
set -eu
SUNO_TOOL_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SUNO_BUILD_MODE=--offline
if [ "$#" -gt 1 ]; then echo '用法：build.sh [--online]' >&2; exit 2; fi
if [ "${1:-}" = --online ]; then SUNO_BUILD_MODE=; elif [ "$#" -gt 0 ]; then echo '用法：build.sh [--online]' >&2; exit 2; fi
command -v cargo >/dev/null || { echo '需要Rust 1.88+；不会自动安装。' >&2; exit 1; }
SUNO_BUILD_DIR=$(mktemp -d "${TMPDIR:-/tmp}/suno-studio-build.XXXXXX")
trap 'rm -rf "$SUNO_BUILD_DIR"' EXIT HUP INT TERM
export CARGO_TARGET_DIR="$SUNO_BUILD_DIR"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$SUNO_TOOL_DIR/source=suno-native-download-source"
cargo build --locked $SUNO_BUILD_MODE --manifest-path "$SUNO_TOOL_DIR/source/Cargo.toml" --bin sunox
mkdir -p "$SUNO_TOOL_DIR/bin"
# 不覆盖已交付二进制；用户确认新构建后自行选择。
SUNO_NEW_BINARY="$SUNO_TOOL_DIR/bin/sunox-studio-download.local-build"
if [ -e "$SUNO_NEW_BINARY" ] || [ -L "$SUNO_NEW_BINARY" ]; then echo 'local-build已存在，未覆盖。' >&2; exit 1; fi
(set -C; cat "$SUNO_BUILD_DIR/debug/sunox" > "$SUNO_NEW_BINARY")
chmod +x "$SUNO_NEW_BINARY"
"$SUNO_NEW_BINARY" --version
