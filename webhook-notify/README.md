# Webhook Notify

POSTs the uploaded link to a webhook whenever an upload succeeds — a reference
for the v0.5 `config_get` + `fetch_post` host imports. The body is
`{"content": "<url>"}`, which both **Discord** and **Slack** incoming webhooks
accept as-is.

- **Hooks:** `on_upload_success`
- **Capability:** `fetch = ["https://discord.com/api/webhooks/*"]`
- **Requires:** capscr 0.5.0+

## setup

The webhook URL is read at runtime from the plugin's `config.toml` — no source
edits. After installing, create:

```
%APPDATA%\com.capscr.capscr\data\plugins\webhook-notify\config.toml
```

```toml
webhook_url = "https://discord.com/api/webhooks/123456/your-token"
```

For Discord that's all — the `fetch` capability already allows any
`discord.com/api/webhooks/*` URL. For **Slack** (or any other host), also edit
the `fetch` pattern in `plugin.toml` to cover your `hooks.slack.com/...` URL and
rebuild, since the host only permits fetch/fetch_post to declared hosts.

## how it works

1. `on_upload_success` fires with the uploaded URL.
2. `config_get("webhook_url")` returns the configured endpoint.
3. `fetch_post` sends `{"content":"<url>"}` to it as `application/json`.

## security

The host enforces https-only, blocks non-web ports, runs the same SSRF guard as
uploads (private/loopback/metadata IPs rejected, DNS double-resolved), disables
redirects, and caps the request + response at 1 MiB each. The webhook URL must
match the `fetch` capability pattern or the POST is denied.
