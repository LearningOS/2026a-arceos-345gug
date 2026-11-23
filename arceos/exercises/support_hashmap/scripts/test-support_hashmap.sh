#!/usr/bin/env bash
# Exercise 4 self-test: use `HashMap` from axstd::collections inside a no_std
# kernel app, insert 50,000 string keys and verify every value.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

LOG="$ROOT/target/test-support-hashmap.log"
mkdir -p "$ROOT/target"

set +e
timeout 240 make A=exercises/support_hashmap LOG=warn QEMU_LOG=n PFLASH=n run \
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

check "Running memory tests..." "test start"
check "test_hashmap() OK!" "HashMap verification"
check "Memory tests run OK!" "success marker"

echo "test-support_hashmap.sh: ALL CHECKS PASSED"
