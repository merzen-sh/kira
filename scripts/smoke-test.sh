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

OUTPUT="$("$BINARY" --version)"
if [[ "$OUTPUT" != kira\ * ]]; then
    echo "FAIL: unexpected --version output: $OUTPUT"
    exit 1
fi
echo "PASS: $OUTPUT"
