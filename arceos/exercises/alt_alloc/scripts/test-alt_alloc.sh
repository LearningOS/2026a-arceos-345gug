#!/usr/bin/env bash
# Exercise 3 self-test: replace the global allocator with the early (bump)
# allocator and exercise it through a large Vec allocation and sorting.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

LOG="$ROOT/target/test-alt-alloc.log"
mkdir -p "$ROOT/target"

set +e
timeout 240 make A=exercises/alt_alloc LOG=info QEMU_LOG=n PFLASH=n run \
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

check "use early (bump) allocator." "alternate allocator selection"
check "Running bump tests..." "test start"
check "Bump tests run OK!" "success marker"

echo "test-alt_alloc.sh: ALL CHECKS PASSED"
