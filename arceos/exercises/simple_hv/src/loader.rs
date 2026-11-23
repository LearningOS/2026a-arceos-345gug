//! Loading a raw guest kernel image from the filesystem.

use alloc::vec::Vec;
use std::fs::File;
use std::io::{self, Read};

/// Reads the whole guest image into memory.
///
/// The guest is linked at a guest physical address (GPA) of 0x8020_0000 and
/// executed in VS-mode, so the returned bytes are a flat binary image rather
/// than an ELF file.
pub fn load_vm_image(fname: &str) -> io::Result<Vec<u8>> {
    ax_println!("Load guest image: {}", fname);
    let mut file = File::open(fname)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    ax_println!("Guest image size: {} byte(s)", buf.len());
    Ok(buf)
}
