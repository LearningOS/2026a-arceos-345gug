#!/usr/bin/env bash
# Exercise 1 self-test: build the app, boot it under QEMU, and check that the
# ANSI escape sequences and the greeting really appear on the serial console.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

LOG="$ROOT/target/test-print-with-color.log"
mkdir -p "$ROOT/target"

set +e
timeout 120 make A=exercises/print_with_color LOG=warn QEMU_LOG=n PFLASH=n run \
    2>&1 | tee "$LOG"
status=${PIPESTATUS[0]}
set -e

if [ "$status" -ne 0 ]; then
    echo "FAIL: QEMU/make exited with status $status" >&2
    exit 1
fi

check() {
    if grep -qF -- "$1" "$LOG"; then
        echo "PASS: found $2"
    else
        echo "FAIL: missing $2" >&2
        exit 1
    fi
}

check $'\x1b[1m'  "bold ANSI sequence"
check $'\x1b[32m' "green ANSI sequence"
check $'\x1b[36m' "cyan ANSI sequence"
check $'\x1b[0m'  "ANSI reset sequence"
check 'Hello, Arceos!' "greeting text"
check '[WithColor]: ok!' "success marker"

echo "test-print.sh: ALL CHECKS PASSED"
