#![cfg(target_arch = "wasm32")] // wasm-only plugin; empty lib on other targets
//! capscr plugin: log capture-saved and upload-success events.
//!
//! The smallest useful reference plugin — it forwards each event payload to the
//! host `log` import (info level). Needs no capability. Targets the capscr WASM
//! plugin runtime (v0.4+); see docs/plugin-runtime.md for the ABI.

static mut SCRATCH: Vec<u8> = Vec::new();

#[no_mangle]
pub extern "C" fn capscr_alloc(size: i32) -> i32 {
    let size = size.max(0) as usize;
    // SAFETY: single-threaded wasm; calls are serialised by the host store lock
    unsafe {
        let buf = &mut *core::ptr::addr_of_mut!(SCRATCH);
        buf.clear();
        buf.reserve(size);
        buf.as_mut_ptr() as i32
    }
}

#[link(wasm_import_module = "capscr")]
extern "C" {
    // level: 0 error, 1 warn, 2 info, 3 debug
    fn log(level: i32, ptr: i32, len: i32);
}

const INFO: i32 = 2;

#[no_mangle]
pub extern "C" fn capscr_on_capture_saved(ptr: i32, len: i32) {
    unsafe { log(INFO, ptr, len) }
}

#[no_mangle]
pub extern "C" fn capscr_on_upload_success(ptr: i32, len: i32) {
    unsafe { log(INFO, ptr, len) }
}
