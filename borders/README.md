# Borders

Adds a solid border around each capture.

## what it does

Runs on the `on_capture` event (capscr 0.5.0+). Takes the captured RGBA pixels,
draws a solid border around them, and returns the bordered image as the
replacement so downstream actions (save, clipboard, upload) see the bordered
version.

- **Hook:** `on_capture` (image-blob API)
- **Capability:** `image = ["read", "modify"]`
- **Requires:** capscr 0.5.0+

## status

**v0.2.0: live WASM port (solid style).** This is the first functional build,
targeting capscr's WASM plugin runtime. The styling is a built-in default
(border thickness 8 px, dark gray) because the sandbox has no filesystem access
for a config file.

The original native reference implementation (in this repo's git history) also
did drop shadows, rounded corners, and double/dashed/3-D styles. Those port to
the same `on_capture` byte-in/byte-out shape — and once a config host-API lands,
the TOML config below can drive them again.

## planned config

```toml
style = "solid"          # solid | double | dashed | dotted | groove | ridge | inset | outset
size = 8
color = [40, 40, 40, 255]   # RGBA, 0-255
corner_radius = 0
padding = 0
```

## license

MIT — see `LICENSE` at the repo root.
