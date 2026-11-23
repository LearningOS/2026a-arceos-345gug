//! Minimal guest kernel for the `simple_hv` exercise.
//!
//! It is loaded at guest physical address 0x8020_0000 and starts directly in
//! VS-mode.  It performs an SBI legacy shutdown call (`a7 = 8`) carrying two
//! marker values in `a0`/`a1`; the hypervisor checks them before powering the
//! machine off.

#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    asm!(
        "li a0, 0x6688",   // marker checked by the hypervisor
        "li a1, 0x1234",   // second marker
        "li a7, 8",        // SBI legacy extension: shutdown
        "ecall",           // trap to HS-mode: VirtualSupervisorEnvCall
        options(noreturn)
    )
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
