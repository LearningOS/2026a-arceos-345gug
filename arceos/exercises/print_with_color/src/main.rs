//! Exercise 1: colored console output.
//!
//! ANSI (American National Standards Institute) escape sequences are used to
//! switch the foreground color of the serial console.  A sequence has the form
//! `ESC [ <SGR parameters> m`, where `ESC` is the byte `0x1b` and SGR means
//! Select Graphic Rendition.  `ESC[0m` resets all attributes.

#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

/// Color related ANSI escape sequences.
mod ansi {
    /// Reset every graphic attribute (color, bold, underline, ...).
    pub const RESET: &str = "\x1b[0m";
    /// Bold text.
    pub const BOLD: &str = "\x1b[1m";
    /// Foreground colors.
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // The greeting itself: green/bold tag plus a cyan message.
    println!(
        "{}{}[WithColor]{}: {}{}Hello, Arceos!{}",
        ansi::BOLD,
        ansi::GREEN,
        ansi::RESET,
        ansi::BOLD,
        ansi::CYAN,
        ansi::RESET,
    );

    // A small palette makes the effect obvious when the serial output is
    // displayed on an ANSI-compatible terminal.
    println!(
        "{}R{} {}G{} {}B{} {}M{} {}C{} {}Y{}",
        ansi::RED,
        ansi::RESET,
        ansi::GREEN,
        ansi::RESET,
        ansi::BLUE,
        ansi::RESET,
        ansi::MAGENTA,
        ansi::RESET,
        ansi::CYAN,
        ansi::RESET,
        ansi::YELLOW,
        ansi::RESET,
    );

    println!("{}[WithColor]: ok!{}", ansi::GREEN, ansi::RESET);
}
