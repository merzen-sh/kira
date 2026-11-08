#!/usr/bin/env bash

set -Eeuo pipefail

BINARY="target/release/kira"

echo "Building release..."
make release

if [[ ! -x "$BINARY" ]]; then
    echo "ERROR: Release binary not found or not executable: $BINARY"
    exit 1
fi

echo "Running smoke test..."

"$BINARY" --version >/dev/null 2>&1 && {
    echo "PASS: Binary starts successfully"
    exit 0
}

echo "FAIL: Binary execution failed"
exit 1
