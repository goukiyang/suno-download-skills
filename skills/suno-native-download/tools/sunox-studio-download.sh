#!/bin/sh
set -eu
SUNO_TOOL_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
  echo '此预编译工具仅支持macOS Apple Silicon；其他平台源码未验证。' >&2
  exit 1
fi
exec "$SUNO_TOOL_DIR/bin/sunox-studio-download" "$@"
