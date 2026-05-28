#![cfg(target_arch = "wasm32")] // wasm-only plugin; empty lib on other targets
//! capscr plugin: add a solid border around each capture.
//!
//! Streamlined WASM port of the original native borders plugin (solid style),
//! driven by the v0.5 image-blob `on_capture` API. The sandbox has no
//! filesystem, so the styling is a built-in default (the native version loaded
//! it from TOML). Pure byte math, no dependencies — the original's extra styles
//! (drop shadow, rounded corners, double/dashed/3-D) port the same way once a
//! wasm toolchain is wired up to compile against the `image` crate.

const BORDER: u32 = 8; // thickness in px on every side
const COLOR: [u8; 4] = [40, 40, 40, 255]; // opaque dark gray
const MAX_DIM: u32 = 16384; // host rejects larger replacements; bail to match

// host writes the input [w][h][mode][rgba] blob here; OUTPUT holds the bordered
// replacement. capscr serialises hook calls per plugin, so reusing buffers is
// safe and leak-free.
static mut SCRATCH: Vec<u8> = Vec::new();
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
/// replacement [w:u32][h:u32][rgba] blob.
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

    let nw = w + BORDER * 2;
    let nh = h + BORDER * 2;
    if nw > MAX_DIM || nh > MAX_DIM {
        return 0; // the host would reject an oversized replacement anyway
    }
    let row_bytes = w as usize * 4;
    let nrow_bytes = nw as usize * 4;
    let out_len = 8 + nrow_bytes * nh as usize;

    // SAFETY: serialised calls; the host copies the bytes out before the next
    // call clears this buffer
    unsafe {
        let out = &mut *core::ptr::addr_of_mut!(OUTPUT);
        out.clear();
        out.reserve(out_len);
        out.extend_from_slice(&nw.to_le_bytes());
        out.extend_from_slice(&nh.to_le_bytes());
        // fill the whole canvas with the border colour
        for _ in 0..(nw as usize * nh as usize) {
            out.extend_from_slice(&COLOR);
        }
        // blit the original into the centre, row by row
        let body = &mut out[8..];
        for row in 0..h as usize {
            let src = &rgba[row * row_bytes..row * row_bytes + row_bytes];
            let dst = (row + BORDER as usize) * nrow_bytes + BORDER as usize * 4;
            body[dst..dst + row_bytes].copy_from_slice(src);
        }
        let p = out.as_ptr() as i64;
        (p << 32) | (out_len as i64 & 0xffff_ffff)
    }
}
