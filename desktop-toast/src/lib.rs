#![cfg(target_arch = "wasm32")] // wasm-only plugin; empty lib on other targets
//! capscr plugin: pop a desktop notification with the URL after an upload.
//!
//! Targets the capscr WASM plugin runtime (v0.4+). See docs/plugin-runtime.md.

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
    fn notify(title_ptr: i32, title_len: i32, body_ptr: i32, body_len: i32) -> i32;
}

/// on_upload_success payload is the result URL (utf-8). show it as the toast
/// body with a fixed title. the title lives in our data segment (linear memory),
/// so the host can read it at (ptr,len) just like the body.
#[no_mangle]
pub extern "C" fn capscr_on_upload_success(ptr: i32, len: i32) {
    let title = "Uploaded";
    unsafe {
        let _ = notify(title.as_ptr() as i32, title.len() as i32, ptr, len);
    }
}
