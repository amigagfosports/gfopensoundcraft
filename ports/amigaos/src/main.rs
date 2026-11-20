// Copyright (c) 2026 AmiCradle Team. MIT OR Apache-2.0.
#![cfg_attr(target_arch = "m68k", no_main)]
#![cfg_attr(target_arch = "m68k", no_std)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_arch = "m68k")]
extern crate alloc;

#[cfg(any(target_arch = "m68k", test))]
mod platform;

#[cfg(target_arch = "m68k")]
use alloc::vec::Vec;
#[cfg(target_arch = "m68k")]
use platform::{BINDINGS, STARTUP_LINES};

#[cfg(target_arch = "m68k")]
#[allow(unsafe_code, reason = "the fixed AmigaDOS entry symbol is the port's isolated ABI boundary")]
#[unsafe(no_mangle)]
pub extern "C" fn gf_amiga_main(_args: *const u8, _length: i32) -> i32 {
    amiga_runtime::run(|runtime| {
        let allocation_proof: Vec<_> = BINDINGS.iter().collect();
        for line in STARTUP_LINES {
            runtime.put_str(line);
        }
        if allocation_proof.len() == BINDINGS.len() { 0 } else { 20 }
    })
}

#[cfg(target_arch = "m68k")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_arch = "m68k"))]
fn main() {}
