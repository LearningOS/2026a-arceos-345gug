//! Test payload for the `sys_map` exercise.
//!
//! This is a freestanding `no_std` RISC-V Linux-style user program.  It uses
//! raw `ecall` instructions (no libc) so it can be built with the Rust target
//! that ArceOS already uses:
//!
//! 1. create `/tmp/test_file` and write "hello, arceos!";
//! 2. open it read-only and `mmap` 32 bytes with `PROT_READ | MAP_PRIVATE`;
//! 3. print the mapped memory (the kernel must have filled it with the file
//!    contents) and exit.

#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;
use core::slice;

const AT_FDCWD: usize = -100isize as usize;
const O_RDONLY: usize = 0;
const O_WRONLY: usize = 0o1;
const O_CREAT: usize = 0o100;
const O_TRUNC: usize = 0o1000;

const SYS_OPENAT: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_MMAP: usize = 222;

const PROT_READ: usize = 0x1;
const MAP_PRIVATE: usize = 0x2;

const FILE_PATH: &[u8] = b"/tmp/test_file\0";
const CONTENT: &[u8] = b"hello, arceos!";

#[inline(always)]
unsafe fn syscall1(num: usize, a0: usize) -> isize {
    let ret: isize;
    asm!(
        "ecall",
        inlateout("a0") a0 => ret,
        in("a7") num,
        options(nostack)
    );
    ret
}

#[inline(always)]
unsafe fn syscall3(num: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let ret: isize;
    asm!(
        "ecall",
        inlateout("a0") a0 => ret,
        in("a1") a1,
        in("a2") a2,
        in("a7") num,
        options(nostack)
    );
    ret
}

#[inline(always)]
unsafe fn syscall4(num: usize, a0: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let ret: isize;
    asm!(
        "ecall",
        inlateout("a0") a0 => ret,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        in("a7") num,
        options(nostack)
    );
    ret
}

#[inline(always)]
unsafe fn syscall6(
    num: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> isize {
    let ret: isize;
    asm!(
        "ecall",
        inlateout("a0") a0 => ret,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        in("a4") a4,
        in("a5") a5,
        in("a7") num,
        options(nostack)
    );
    ret
}

/// Writes the whole buffer to `fd`, retrying on short writes.
unsafe fn write_all(fd: usize, buf: &[u8]) -> bool {
    let mut written = 0;
    while written < buf.len() {
        let n = syscall3(
            SYS_WRITE,
            fd,
            buf.as_ptr().add(written) as usize,
            buf.len() - written,
        );
        if n <= 0 {
            return false;
        }
        written += n as usize;
    }
    true
}

unsafe fn print(s: &[u8]) {
    let _ = write_all(1, s);
}

unsafe fn exit(code: usize) -> ! {
    let _ = syscall1(SYS_EXIT, code);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        print(b"MapFile ...\n");

        // Create (or truncate) the file that will be mapped.
        let fd = syscall4(
            SYS_OPENAT,
            AT_FDCWD,
            FILE_PATH.as_ptr() as usize,
            O_WRONLY | O_CREAT | O_TRUNC,
            0o600,
        );
        if fd < 0 {
            print(b"Create file error!\n");
            exit(1);
        }
        if !write_all(fd as usize, CONTENT) || !write_all(fd as usize, b"\0") {
            print(b"Write file error!\n");
            exit(1);
        }
        let _ = syscall1(SYS_CLOSE, fd as usize);

        // Re-open it read-only and map the first 32 bytes.
        let fd = syscall4(
            SYS_OPENAT,
            AT_FDCWD,
            FILE_PATH.as_ptr() as usize,
            O_RDONLY,
            0,
        );
        if fd < 0 {
            print(b"Open file error!\n");
            exit(1);
        }

        let addr = syscall6(SYS_MMAP, 0, 32, PROT_READ, MAP_PRIVATE, fd as usize, 0);
        if addr <= 0 {
            print(b"Map file error!\n");
            exit(1);
        }

        // Print the NUL-terminated string found in the mapped memory.
        //
        // The whole line is emitted with a single `write` syscall so that the
        // kernel's per-syscall log lines cannot be interleaved between the
        // prefix and the mapped contents. This also matches the line produced
        // by the C reference payload.
        let ptr = addr as *const u8;
        let mut len = 0;
        while len < 32 && *ptr.add(len) != 0 {
            len += 1;
        }
        const PREFIX: &[u8] = b"Read back content: ";
        let mut line = [0u8; 64];
        line[..PREFIX.len()].copy_from_slice(PREFIX);
        let mapped = slice::from_raw_parts(ptr, len);
        line[PREFIX.len()..PREFIX.len() + len].copy_from_slice(mapped);
        line[PREFIX.len() + len] = b'\n';
        let _ = write_all(1, &line[..PREFIX.len() + len + 1]);

        let _ = syscall1(SYS_CLOSE, fd as usize);
        print(b"MapFile ok!\n");
        exit(0);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
