#!/usr/bin/env bash
# Exercise 6 self-test:
#   1. build the tiny guest kernel (`skernel2`);
#   2. build a FAT32 disk image containing /sbin/skernel2;
#   3. boot ArceOS with a CPU that implements the RISC-V hypervisor
#      extension, run the guest in VS-mode, catch its SBI shutdown ecall and
#      verify that QEMU powers off normally.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

LOG="$ROOT/target/test-simple-hv.log"
IMG="$ROOT/exercises/simple_hv/disk.img"
mkdir -p "$ROOT/target"

echo "== building guest kernel =="
cargo build -p skernel2 --target riscv64gc-unknown-none-elf --release
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary \
    "$ROOT/target/riscv64gc-unknown-none-elf/release/skernel2" \
    "$ROOT/payload/skernel2/skernel2"

echo "== building FAT32 disk image =="
python3 exercises/tools/mkdisk.py "$IMG" \
    /sbin/skernel2="$ROOT/payload/skernel2/skernel2"

set +e
timeout 300 make A=exercises/simple_hv BLK=y DISK_IMG="$IMG" \
    QEMU_CPU=max LOG=warn QEMU_LOG=n PFLASH=n run 2>&1 | tee "$LOG"
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

check "Hypervisor ..." "hypervisor start"
check "Guest image size: 20 byte(s)" "guest image loaded"
check "Guest: GPA 0x80200000 -> HPA" "stage-2 mapping"
check "VmExit Reason: VSuperEcall" "VM exit captured"
check "a0 = 0x6688, a1 = 0x1234" "guest marker registers"
check "Shutdown vm normally!" "normal guest shutdown"
check "[SimpleHV]: ok!" "hypervisor success marker"

if grep -q "panicked" "$LOG"; then
    echo "FAIL: panic in the hypervisor log" >&2
    exit 1
fi

echo "test-simple_hv.sh: ALL CHECKS PASSED"
