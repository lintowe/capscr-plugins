# Grayscale

Rewrites every capture to grayscale (BT.601 luma) before it's saved, copied, or
uploaded — a compact showcase of the v0.5 image-blob `on_capture` API.

- **Hook:** `on_capture` (receives pixels, returns a replacement image)
- **Capability:** `image = ["read", "modify"]`
- **Requires:** capscr 0.5.0+

Pure byte math, no dependencies. A good template for any per-pixel filter
(invert, sepia, threshold, redaction blur, …).
