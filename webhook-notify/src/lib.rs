#![cfg(target_arch = "wasm32")] // wasm-only plugin; empty lib on other targets
//! capscr plugin: POST the uploaded URL to a webhook (Discord/Slack-compatible).
//!
//! Demonstrates `config_get` + `fetch_post` (capscr 0.5+). The webhook endpoint
//! is read at runtime from `config.toml` (`webhook_url = "..."`), so no source
//! edit is needed — a Discord user just drops a config file. The `fetch`
//! capability in plugin.toml still has to cover the URL's host (it does for
//! Discord by default). Body is `{"content": "<url>"}`, accepted by both Discord
//! and Slack incoming webhooks. See docs/plugin-runtime.md for the ABI.

const CONFIG_KEY: &str = "webhook_url";
const CONTENT_TYPE: &str = "application/json";

// the host reuses this one buffer for every payload it writes to us (the hook
// argument, config_get's value, fetch_post's response). capscr serialises hook
// calls, so reusing it is safe — as long as we copy each value out before the
// next host call overwrites it.
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
    // key* -> packed (ptr<<32)|len of the value, 0 if absent
    fn config_get(key_ptr: i32, key_len: i32) -> i64;
    // url*, content_type*, body* -> packed ptr/len of response (0 on failure)
    fn fetch_post(
        url_ptr: i32,
        url_len: i32,
        ct_ptr: i32,
        ct_len: i32,
        body_ptr: i32,
        body_len: i32,
    ) -> i64;
}

/// read a host-written (ptr,len) region (a hook arg, config_get value, or
/// fetch_post response) into an owned String so it survives the next host call
/// that reuses SCRATCH. None on a 0 packed value or invalid utf-8.
unsafe fn owned_from_packed(packed: i64) -> Option<String> {
    if packed == 0 {
        return None;
    }
    let ptr = ((packed as u64) >> 32) as usize;
    let len = (packed as u64 & 0xffff_ffff) as usize;
    owned_from_ptr(ptr as i32, len as i32)
}

unsafe fn owned_from_ptr(ptr: i32, len: i32) -> Option<String> {
    if ptr < 0 || len < 0 {
        return None;
    }
    let slice = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    core::str::from_utf8(slice).ok().map(str::to_owned)
}

/// on_upload_success payload is the result URL (utf-8). look up the webhook URL
/// from config, then POST `{"content": "<url>"}` to it.
#[no_mangle]
pub extern "C" fn capscr_on_upload_success(ptr: i32, len: i32) {
    unsafe {
        // 1. copy the uploaded URL out before any other host call reuses SCRATCH
        let uploaded = match owned_from_ptr(ptr, len) {
            Some(s) => s,
            None => return,
        };
        // 2. fetch the configured webhook URL (writes into SCRATCH); copy it out
        let webhook = match owned_from_packed(config_get(
            CONFIG_KEY.as_ptr() as i32,
            CONFIG_KEY.len() as i32,
        )) {
            Some(s) => s,
            None => return, // not configured — nothing to do
        };
        // 3. build the body (separate heap allocation, not in SCRATCH)
        let body = format!("{{\"content\":\"{}\"}}", json_escape(&uploaded));
        // 4. POST. webhook + body are owned heap strings, stable across the call
        let _ = fetch_post(
            webhook.as_ptr() as i32,
            webhook.len() as i32,
            CONTENT_TYPE.as_ptr() as i32,
            CONTENT_TYPE.len() as i32,
            body.as_ptr() as i32,
            body.len() as i32,
        );
    }
}

/// minimal JSON string escaping for the characters a URL could plausibly carry
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
