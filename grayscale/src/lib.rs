#![cfg(target_arch = "wasm32")] // wasm-only plugin; empty lib on other targets
//! capscr plugin: convert each capture to grayscale (BT.601 luma).
//!
//! A minimal showcase of the v0.5 image-blob `on_capture` API — it rewrites the
//! captured pixels and returns a replacement image. Pure byte math, no deps.
//! See docs/plugin-runtime.md for the on_capture wire format.

// host writes the input blob ([w][h][mode][rgba]) here via capscr_alloc
static mut SCRATCH: Vec<u8> = Vec::new();
// our replacement blob ([w][h][rgba]); the host reads it after the hook returns,
// before the next (serialised) call reuses it — so reusing one buffer is safe
static mut OUTPUT: Vec<u8> = Vec::new();

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

/// on_capture: 0 = continue unchanged, >0 = packed (ptr<<32)|len of a
/// replacement [w:u32][h:u32][rgba] blob. (We never cancel.)
#[no_mangle]
pub extern "C" fn capscr_on_capture(ptr: i32, len: i32) -> i64 {
    if ptr < 0 || len < 12 {
        return 0;
    }
    let input = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let w = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let h = u32::from_le_bytes([input[4], input[5], input[6], input[7]]);
    // input[8..12] is the capture mode — unused here
    let rgba = &input[12..];
    let expected = (w as usize)
        .saturating_mul(h as usize)
        .saturating_mul(4);
    if rgba.len() != expected || expected == 0 {
        return 0;
    }

    let out_len = 8 + rgba.len();
    // SAFETY: serialised calls; we return ptr/len and the host copies the bytes
    // out before the next call clears this buffer
    unsafe {
        let out = &mut *core::ptr::addr_of_mut!(OUTPUT);
        out.clear();
        out.reserve(out_len);
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        for px in rgba.chunks_exact(4) {
            let y = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
            let g = y as u8;
            out.push(g);
            out.push(g);
            out.push(g);
            out.push(px[3]); // preserve alpha
        }
        let p = out.as_ptr() as i64;
        (p << 32) | (out_len as i64 & 0xffff_ffff)
    }
}
