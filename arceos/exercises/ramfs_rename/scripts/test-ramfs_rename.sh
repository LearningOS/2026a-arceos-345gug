#!/usr/bin/env bash
# Exercise 2 self-test: create a file in the RAM filesystem, rename it in the
# same directory, read it back, and verify the data survived the rename.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

LOG="$ROOT/target/test-ramfs-rename.log"
mkdir -p "$ROOT/target"

set +e
timeout 180 make A=exercises/ramfs_rename FEATURES=driver-ramdisk \
    LOG=warn QEMU_LOG=n PFLASH=n run \
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

check "Create '/tmp/f1' and write [hello] ..." "file creation"
check "Rename '/tmp/f1' to '/tmp/f2' ..." "rename request"
check "Read '/tmp/f2' content: [hello] ok!" "content after rename"
check "[Ramfs-Rename]: ok!" "success marker"

if grep -q "Unsupported\|NotFound\|Error:" "$LOG"; then
    echo "FAIL: unexpected error in the log" >&2
    exit 1
fi

echo "test-ramfs_rename.sh: ALL CHECKS PASSED"
