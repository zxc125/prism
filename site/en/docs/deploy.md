# Self-Hosting

Prism is local-first — your data stays with you. `observer-server` is a single-binary HTTP server that brings intake + storage + replay analysis onto your own machine or private cloud.

## Form factors

| Form | Command | For |
| --- | --- | --- |
| **Desktop console** | Tauri app | personal / single machine, zero ops |
| **Single binary** | `observer-server` | team / private cloud, SDKs report directly |
| **Browser-hosted** | `observer-server --web-dir` | zero install, open a URL in the browser |

## Get the binary

```sh
cargo install observer-server
# or download a prebuilt binary from GitHub Releases
```

## Single tenant

Simplest: one API key, flat directory.

```sh
observer-server \
  --bind 0.0.0.0:8080 \
  --data-dir ./recordings \
  --token sk_your_api_key
```

- Auth: clients send `Authorization: Bearer sk_your_api_key`. Empty `--token` = no auth (loopback only).
- Env vars: `OBSERVER_BIND` / `OBSERVER_DATA_DIR` / `OBSERVER_TOKEN`.

## Multi-tenant

One `tenants.json` governs multiple teams / apps, each with its own quota, retention, redaction, and rate limit.

```sh
observer-server --bind 0.0.0.0:8080 --data-dir ./recordings --tenants-file ./tenants.json
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

| Field | Required | Notes |
| --- | --- | --- |
| `key` | ✅ | tenant API key (`Authorization: Bearer`) |
| `tenantId` | ✅ | isolates storage as `recordings/<tenantId>/<sessionId>/` |
| `appIds` | ✅ | allowed appIds; empty = unrestricted. Reported `session.appId` must match |
| `quotaBytes` | ➖ | disk quota in bytes |
| `retention` | ➖ | `{ maxAgeDays?, maxSessions? }` auto-cleanup |
| `redact` | ➖ | server-side redaction on ingest (same options as SDK `redact` + `scrubbers`) |
| `rateLimit` | ➖ | `{ maxRpm }` requests-per-minute cap |

## Browser-hosted (zero install)

Serve the console frontend build from the server; open the URL for a full analysis console:

```sh
pnpm build                       # build the console frontend into dist/
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
        client_max_body_size 200m;   # large bundle uploads
    }
}
```

```text [Caddyfile]
prism.internal {
    reverse_proxy 127.0.0.1:8080   # Caddy auto-issues and renews TLS
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
- **Compression**: gzip transport; on-disk JSON compresses well.
- Offline bundle uploads authenticate by key and land in the matching tenant; `session.appId` must be in the key's allowed set or it's rejected.

## Point the SDK at your deployment

Same code for local and cloud — just change the `endpoint`:

```ts
import { init } from "@prism-obs/observer-sdk";

init({
  appId: "shop-web",
  endpoint: "https://prism.internal",
  token: "sk_acme_prod",
});
```

Offline bundle upload: `POST /sessions/import` (with the Bearer key).

## Upgrades & ops

- Upgrade = swap the binary and restart; the data directory format is forward-compatible.
- Logs go to stderr; health-check via `GET /whoami`.
- Start single-tenant, enable multi-tenant when needed — same binary, a config switch, not a code fork.
