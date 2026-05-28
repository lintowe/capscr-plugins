# Capture Logger

A minimal reference plugin: it writes a line to capscr's log whenever a capture
is saved or an upload succeeds. Useful as a starting template and for confirming
the plugin runtime is dispatching events.

- **Hooks:** `on_capture_saved`, `on_upload_success`
- **Capability:** none (the `log` host import is always available)
- **Requires:** capscr 0.4.0+
