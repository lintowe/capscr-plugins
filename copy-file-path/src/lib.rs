#![cfg(target_arch = "wasm32")] // wasm-only plugin; empty lib on other targets
//! capscr plugin: copy the saved capture's file path to the clipboard.
//!
//! Targets the capscr WASM plugin runtime (v0.4+). Built as a cdylib for
//! wasm32-unknown-unknown; the host loads `plugin.wasm` and calls the exported
//! hooks. See docs/plugin-runtime.md in the capscr repo for the ABI.

// reusable scratch buffer the host writes hook payloads into. capscr serialises
// hook calls per plugin, so one shared buffer is safe and avoids the per-call
// leak of the alloc-and-forget pattern.
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
    // returns 0 ok, <0 denied/error (we don't act on it here)
    fn clipboard_write_text(ptr: i32, len: i32) -> i32;
}

/// on_capture_saved payload is the absolute path (utf-8). forward it straight to
/// the clipboard — the host reads our linear memory at (ptr,len) for both calls.
#[no_mangle]
pub extern "C" fn capscr_on_capture_saved(ptr: i32, len: i32) {
    unsafe {
        let _ = clipboard_write_text(ptr, len);
    }
}
