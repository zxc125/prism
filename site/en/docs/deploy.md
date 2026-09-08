# Self-Hosting

Prism is local-first — your data stays with you. `observer-server` is a single-binary HTTP server that brings intake + storage + replay analysis onto your own machine or private cloud.

**In this page**: get the `observer-server` binary → run it single- or multi-tenant → (optional) browser hosting + TLS. Prerequisite: a Rust toolchain (`cargo --version` works; otherwise install [rustup](https://rustup.rs) first).

## Form factors

| Form | Command | For |
| --- | --- | --- |
| **Desktop console** | Tauri app | personal / single machine, zero ops |
| **Single binary** | `observer-server` | team / private cloud, SDKs report directly |
| **Browser-hosted** | `observer-server --web-dir` | zero install, open a URL in the browser |

## Get the binary

```sh
# Main path: build from source (observer-server is not published to crates.io)
git clone https://github.com/zxc125/prism.git
cd prism
cargo install --path crates/observer-server
observer-server --help   # installed into ~/.cargo/bin
```

You can also grab a prebuilt binary from [GitHub Releases](https://github.com/zxc125/prism/releases) (if your platform is provided) and put it on `PATH`.

## Single tenant

Simplest: one API key, flat directory.

```sh
observer-server \
  --bind 0.0.0.0:8080 \
  --data-dir ./recordings \
  --token sk_your_api_key
```

- `--bind`: listen address. Loopback-only use `127.0.0.1:8080`; expose as `0.0.0.0:8080`.
- `--data-dir`: session storage root, created if missing; backup = copy this directory.
- `--token`: API key. Clients send `Authorization: Bearer sk_your_api_key`; **empty = no auth, loopback use only**.
- Env vars: `OBSERVER_BIND` / `OBSERVER_DATA_DIR` / `OBSERVER_TOKEN`.

**✅ Check**: `curl http://127.0.0.1:8080/whoami -H "Authorization: Bearer sk_your_api_key"` returns the tenant context JSON.

## Multi-tenant

One `tenants.json` governs multiple teams / apps, each with its own quota, retention, redaction, and rate limit.

```sh
observer-server \
  --bind 0.0.0.0:8080 \
  --data-dir ./recordings \
  --tenants-file ./tenants.json
```

```json
[
  {
    "key": "sk_acme_prod",
    "tenantId": "acme",
    "appIds": ["shop-web", "shop-admin"],
    "quotaBytes": 5368709120,
    "retention": { "maxAgeDays": 30, "maxSessions": 5000 },
    "redact": {
      "stripNetworkBody": true,
      "stripNetworkHeaders": true,
      "scrubbers": ["Bearer\\s+[\\w.-]+", "[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+"]
    },
    "rateLimit": { "maxRpm": 600 }
  },
  { "key": "sk_beta_dev", "tenantId": "beta", "appIds": [], "retention": { "maxSessions": 200 } }
]
```

::: danger scrubbers are regexes inside JSON strings — double the backslashes
The `"Bearer\\s+[\\w.-]+"` above deliberately has an extra layer of `\`: inside a JSON string `\s` is an invalid escape and **parsing fails on the spot**. Every regex `\` must be written as `\\` in JSON.

```text
✅ correct: "scrubbers": ["Bearer\\s+[\\w.-]+"]
❌ wrong:   "scrubbers": ["Bearer\s+[\w.-]+"]   ← JSON parse error, server won't start
```
:::

| Field | Required | Notes |
| --- | --- | --- |
| `key` | ✅ | tenant API key (`Authorization: Bearer`) |
| `tenantId` | ✅ | isolates storage as `recordings/<tenantId>/<sessionId>/` |
| `appIds` | ✅ | allowed appIds; empty = unrestricted. Reported `session.appId` must match |
| `quotaBytes` | ➖ | disk quota in bytes |
| `retention` | ➖ | `{ maxAgeDays?, maxSessions? }` auto-cleanup |
| `redact` | ➖ | server-side redaction on ingest (same options as SDK `redact` + `scrubbers`) |
| `rateLimit` | ➖ | `{ maxRpm }` requests-per-minute cap |

## Run as a service (systemd)

For production, host it under systemd (auto-start + crash restart):

```ini
# /etc/systemd/system/observer-server.service
[Unit]
Description=Prism observer-server
After=network.target

[Service]
ExecStart=/usr/local/bin/observer-server --bind 0.0.0.0:8080 --data-dir /var/lib/prism/recordings --tenants-file /etc/prism/tenants.json
Environment=OBSERVER_TOKEN=sk_your_api_key
Restart=on-failure
User=prism

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl enable --now observer-server
journalctl -u observer-server -f   # logs (the server logs to stderr)
```

## Browser hosting (zero client install)

> This section requires cloning the repo first (the static assets come from the repo's frontend build).

Serve the console frontend build from the server; teammates open a URL for the full analysis console, no desktop app needed:

```sh
# 1. build the console frontend at the repo root (output in dist/)
pnpm install
pnpm build

# 2. serve static files + API from the server
observer-server \
  --bind 0.0.0.0:8080 \
  --data-dir ./recordings \
  --tenants-file ./tenants.json \
  --web-dir ./dist
```

Non-API requests fall back to static files (SPA mode + path-traversal guard + MIME dispatch). Open `http://<host>:8080`; `GET /whoami` returns the current tenant context + remaining quota.

## TLS via reverse proxy

The server runs HTTP only; terminate TLS at a reverse proxy.

::: code-group

```nginx [nginx]
server {
    listen 443 ssl http2;
    server_name prism.internal;

    ssl_certificate     /etc/ssl/prism.pem;
    ssl_certificate_key /etc/ssl/prism.key;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        # large bundle uploads
        client_max_body_size 200m;
    }
}
```

```text [Caddyfile]
prism.internal {
    reverse_proxy 127.0.0.1:8080
    # Caddy auto-issues and renews TLS
}
```

:::

## Data directory & retention

- `--data-dir` is the storage root; backup = copy the directory, migrate = move it.
- Multi-tenant isolation: `recordings/<tenantId>/<sessionId>/`.
- `retention` auto-cleans by `maxAgeDays` / `maxSessions`.

## Data hygiene

- **Server-side redaction**: `tenants.json` `redact` strips network body/headers and scrubs tokens/emails on ingest.
- **Rate limiting**: `rateLimit.maxRpm`.
- **Response compression**: API responses are gzipped (body > 1KB and the client sends `Accept-Encoding: gzip`); on-disk JSON compresses well.
- Offline bundle uploads authenticate by key and land in the matching tenant; `session.appId` must be in the key's allowed set or it's rejected.

## Point the SDK at your deployment

Same code for local and cloud — just change the `endpoint`:

```ts
import { init } from "@prism-obs/observer-sdk";

init({
  appId: "shop-web",
  endpoint: "https://prism.internal",   // your observer-server
  token: "sk_acme_prod",                 // tenant key
});
```

Offline bundle upload: `POST /sessions/import` (with the Bearer key).

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| Server won't start, tenants.json parse error | unescaped `\` in `scrubbers` regexes | see the JSON escaping warning above |
| SDK reports 401 | token / key mismatch | single-tenant: check `--token`; multi-tenant: check the tenant's `key` |
| SDK reports 403 / rejected | `appId` not in the tenant's `appIds` | add the appId to the set, or use an empty array for unrestricted |
| Blank page in the browser | `--web-dir` doesn't point at the build | confirm it points at `dist/` and `pnpm build` has run |

## Upgrades & ops

- Upgrade = pull the latest code, re-run `cargo install --path crates/observer-server`, swap the binary and restart; the data directory format is forward-compatible.
- Logs go to stderr; health-check via `GET /whoami` (send the Bearer key when auth is on).
- Start single-tenant, enable multi-tenant when needed — same binary, a config switch, not a code fork.
