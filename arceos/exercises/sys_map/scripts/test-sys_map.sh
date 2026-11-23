#!/usr/bin/env bash
# Exercise 5 self-test:
#   1. build the freestanding `mapfile` user payload;
#   2. build a FAT32 disk image containing /sbin/mapfile;
#   3. boot the monolithic kernel, which loads the payload, and let it
#      create a file, mmap it and read the contents back.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

LOG="$ROOT/target/test-sys-map.log"
IMG="$ROOT/exercises/sys_map/disk.img"
mkdir -p "$ROOT/target"

echo "== building user payload =="
make -C payload/mapfile

echo "== building FAT32 disk image =="
python3 exercises/tools/mkdisk.py "$IMG" \
    /sbin/mapfile="$ROOT/payload/mapfile/mapfile.elf"

set +e
timeout 300 make A=exercises/sys_map BLK=y DISK_IMG="$IMG" \
    LOG=warn QEMU_LOG=n PFLASH=n run 2>&1 | tee "$LOG"
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

check "MapFile ..." "user program start"
check "handle_syscall [222]" "mmap syscall dispatched"
check "sys_mmap: file map at" "file-backed mapping"
check "Read back content:" "mapped content printed"
check "hello, arceos!" "file contents read through mmap"
check "MapFile ok!" "user program success marker"
check "monolithic kernel exit [Some(0)] normally!" "user process exit code"
check "[SysMap]: ok!" "kernel success marker"

if grep -q "panicked\|Unimplemented syscall" "$LOG"; then
    echo "FAIL: panic or unimplemented syscall in the log" >&2
    exit 1
fi

echo "test-sys_map.sh: ALL CHECKS PASSED"
