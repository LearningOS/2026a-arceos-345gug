//! Exercise 6: a minimal RISC-V hypervisor.
//!
//! The hypervisor runs in HS-mode (Host Supervisor) and executes a small guest
//! kernel in VS-mode (Virtual Supervisor).  The guest is placed at the guest
//! physical address 0x8020_0000 and translated to a host physical frame by a
//! stage-2 page table installed in `hgatp` (hypervisor guest address
//! translation and protection).  The guest issues a legacy SBI shutdown call;
//! the resulting `ecall` traps back to HS-mode and is turned into a normal
//! hypervisor exit.

#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]
#![feature(riscv_ext_intrinsics)]

#[cfg(feature = "axstd")]
extern crate axstd as std;
extern crate alloc;
#[macro_use]
extern crate axlog;

mod csrs;
mod loader;
mod regs;
mod sbi;
mod vcpu;

use core::mem::forget;

use axhal::mem::{phys_to_virt, virt_to_phys, PhysAddr, VirtAddr, PAGE_SIZE_4K};
use axhal::paging::{MappingFlags, PageSize, PageTable};
use csrs::defs::hstatus;
use csrs::{RiscvCsrTrait, CSR};
use loader::load_vm_image;
use regs::GprIndex::{A0, A1};
use riscv::register::{scause, sstatus, stval};
use sbi::SbiMessage;
use tock_registers::LocalRegisterCopy;
use vcpu::{VmCpuRegisters, _run_guest};

/// Guest physical address at which the guest kernel is linked.
const VM_ENTRY: usize = 0x8020_0000;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    ax_println!("Hypervisor ...");

    // 1. Load the guest image (a flat binary linked at VM_ENTRY) and copy it
    //    into a page owned by the hypervisor.
    let image = match load_vm_image("/sbin/skernel2") {
        Ok(image) => image,
        Err(e) => panic!("Cannot load guest image! {:?}", e),
    };
    let guest_paddr = load_guest_image(&image);
    ax_println!(
        "Guest: GPA {:#x} -> HPA {:#x}, {} bytes",
        VM_ENTRY,
        guest_paddr,
        image.len()
    );

    // 2. Build the stage-2 page table and program hgatp.
    let ept_root = build_stage2_page_table(guest_paddr);
    ax_println!("Stage-2 root: {:#x}", ept_root);
    prepare_vm_pgtable(ept_root);

    // 3. Prepare the guest CPU state (hstatus/sepc/sstatus) and enter it.
    let mut ctx = VmCpuRegisters::default();
    prepare_guest_context(&mut ctx);

    // Keep host interrupts disabled while the tiny guest runs.  Otherwise a
    // timer interrupt would arrive with stvec temporarily pointing at
    // `_guest_exit` and be mistaken for a VM exit.
    //
    // NOTE: `riscv::interrupt::disable()` from this crate writes `mstatus`,
    // which is illegal in HS-mode; clear `sstatus.SIE` instead.
    unsafe { riscv::register::sstatus::clear_sie() };
    while !run_guest(&mut ctx) {}

    ax_println!("[SimpleHV]: ok!");
    // Ask OpenSBI (M-mode) to power the machine off.
    axhal::misc::terminate();
}

/// Copies the guest image into one freshly allocated page.
///
/// The page is mapped in the kernel's direct map, so its virtual address can
/// be converted to the host physical address used by the stage-2 page table.
fn load_guest_image(image: &[u8]) -> PhysAddr {
    assert!(
        !image.is_empty() && image.len() <= PAGE_SIZE_4K,
        "guest image must fit in one page"
    );
    let page = std::os::arceos::modules::axalloc::global_allocator()
        .alloc_pages(1, PAGE_SIZE_4K)
        .expect("no memory for the guest image");
    let vaddr = VirtAddr::from_usize(page);
    unsafe {
        core::ptr::write_bytes(vaddr.as_mut_ptr(), 0, PAGE_SIZE_4K);
        core::ptr::copy_nonoverlapping(image.as_ptr(), vaddr.as_mut_ptr(), image.len());
    }
    virt_to_phys(vaddr)
}

/// Builds an Sv39 page table mapping the guest physical address `VM_ENTRY` to
/// the host frame containing the guest code.
///
/// `hgatp` with the Sv39x4 mode requires a root page table that is 16 KiB
/// aligned (and has 2048 entries).  `PageTable::try_new` only allocates an
/// ordinary 4 KiB Sv39 root, so the root content is copied into a 16 KiB
/// aligned allocation.  Guest addresses below 2^39 have the same page-table
/// indices in Sv39 and Sv39x4, so the 512 copied entries address the same
/// lower levels.
fn build_stage2_page_table(guest_paddr: PhysAddr) -> PhysAddr {
    let mut pt = PageTable::try_new().expect("failed to allocate stage-2 root");
    let flush = pt
        .map(
            VirtAddr::from_usize(VM_ENTRY),
            guest_paddr,
            PageSize::Size4K,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
        )
        .expect("failed to map the guest page");
    flush.ignore();

    let aligned = std::os::arceos::modules::axalloc::global_allocator()
        .alloc_pages(4, 0x4000)
        .expect("no memory for the Sv39x4 root");
    let root_vaddr = VirtAddr::from_usize(aligned);
    unsafe {
        core::ptr::write_bytes(root_vaddr.as_mut_ptr(), 0, 0x4000);
        core::ptr::copy_nonoverlapping(
            phys_to_virt(pt.root_paddr()).as_ptr(),
            root_vaddr.as_mut_ptr(),
            PAGE_SIZE_4K,
        );
    }
    let root_paddr = virt_to_phys(root_vaddr);

    // Leak the temporary page table: its lower-level tables are still used by
    // the stage-2 walk through the copied root.
    forget(pt);
    root_paddr
}

/// Programs `hgatp` with the Sv39x4 mode and the stage-2 root address.
fn prepare_vm_pgtable(ept_root: PhysAddr) {
    let hgatp = 8usize << 60 | usize::from(ept_root) >> 12;
    unsafe {
        core::arch::asm!(
            "csrw hgatp, {hgatp}",
            hgatp = in(reg) hgatp,
        );
        core::arch::riscv64::hfence_gvma_all();
    }
}

/// Runs the guest once and handles the resulting VM exit.
///
/// Returns `true` when the guest requested shutdown.
fn run_guest(ctx: &mut VmCpuRegisters) -> bool {
    unsafe {
        _run_guest(ctx);
    }
    vmexit_handler(ctx)
}

#[allow(unreachable_code)]
fn vmexit_handler(ctx: &mut VmCpuRegisters) -> bool {
    use scause::{Exception, Trap};

    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
            let sbi_msg = SbiMessage::from_regs(ctx.guest_regs.gprs.a_regs()).ok();
            ax_println!("VmExit Reason: VSuperEcall: {:?}", sbi_msg);
            match sbi_msg {
                Some(SbiMessage::Reset(_)) => {
                    let a0 = ctx.guest_regs.gprs.reg(A0);
                    let a1 = ctx.guest_regs.gprs.reg(A1);
                    ax_println!("a0 = {:#x}, a1 = {:#x}", a0, a1);
                    assert_eq!(a0, 0x6688);
                    assert_eq!(a1, 0x1234);
                    ax_println!("Shutdown vm normally!");
                    true
                }
                Some(other) => panic!("unsupported SBI message: {:?}", other),
                None => panic!("bad SBI message!"),
            }
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            panic!(
                "Bad instruction: {:#x} sepc: {:#x}",
                stval::read(),
                ctx.guest_regs.sepc
            );
        }
        Trap::Exception(Exception::LoadGuestPageFault) => {
            panic!(
                "LoadGuestPageFault: stval={:#x} sepc: {:#x}",
                stval::read(),
                ctx.guest_regs.sepc
            );
        }
        Trap::Exception(Exception::InstructionGuestPageFault) => {
            panic!(
                "InstructionGuestPageFault: stval={:#x} sepc: {:#x}",
                stval::read(),
                ctx.guest_regs.sepc
            );
        }
        Trap::Exception(Exception::StoreGuestPageFault) => {
            panic!(
                "StoreGuestPageFault: stval={:#x} sepc: {:#x}",
                stval::read(),
                ctx.guest_regs.sepc
            );
        }
        other => {
            panic!(
                "Unhandled trap: {:?}, sepc: {:#x}, stval: {:#x}",
                other,
                ctx.guest_regs.sepc,
                stval::read()
            );
        }
    }
}

/// Prepares the guest CPU state so that `sret` enters VS-mode at `VM_ENTRY`.
fn prepare_guest_context(ctx: &mut VmCpuRegisters) {
    // hstatus.SPV=1 makes sret enter guest mode (V=1).
    // hstatus.SPVP=Supervisor makes the guest run in VS-mode.
    let mut hstatus = LocalRegisterCopy::<usize, hstatus::Register>::new(
        riscv::register::hstatus::read().bits(),
    );
    hstatus.modify(hstatus::spv::Guest);
    hstatus.modify(hstatus::spvp::Supervisor);
    CSR.hstatus.write_value(hstatus.get());
    ctx.guest_regs.hstatus = hstatus.get();

    // The guest starts in supervisor mode.  SPIE is cleared so that the guest
    // runs with interrupts disabled (sret copies SPIE into SIE).
    let mut sstatus = sstatus::read();
    sstatus.set_spp(sstatus::SPP::Supervisor);
    sstatus.set_spie(false);
    ctx.guest_regs.sstatus = sstatus.bits();

    // Guest entry point.
    ctx.guest_regs.sepc = VM_ENTRY;
}
