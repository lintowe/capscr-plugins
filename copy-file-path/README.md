# Copy File Path

After capscr saves a capture, this plugin copies the file's absolute path to the
clipboard — handy for pasting the path into a terminal, file dialog, or chat.

- **Hook:** `on_capture_saved`
- **Capability:** `clipboard = ["write"]`
- **Requires:** capscr 0.4.0+

The path is forwarded straight to the host `clipboard_write_text` import; no
configuration.
