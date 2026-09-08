# 私有化部署

Prism 是本地优先的——数据留在你手里。`observer-server` 是一个单二进制 HTTP server，把「接收 + 存储 + 回放分析」整套搬到你自己的机器或私有云。

**本页你将完成**：拿到 `observer-server` 二进制 → 以单租户或多租户形态跑起来 →（可选）浏览器化托管 + TLS。前置：已安装 Rust 工具链（`cargo --version` 可用，没有先装 [rustup](https://rustup.rs)）。

## 形态选择

| 形态 | 命令 | 适用 |
| --- | --- | --- |
| **桌面 console** | Tauri App | 个人 / 单机，零运维 |
| **单二进制 server** | `observer-server` | 团队 / 私有云，SDK 直接上报 |
| **浏览器化托管** | `observer-server --web-dir` | 零安装，浏览器开地址即用 |

## 获取二进制

```sh
# 主路径：源码构建（observer-server 不发布到 crates.io）
git clone https://github.com/zxc125/prism.git
cd prism
cargo install --path crates/observer-server
observer-server --help   # 装到 ~/.cargo/bin，确认可用
```

也可以从 [GitHub Releases](https://github.com/zxc125/prism/releases) 下载预编译产物（若你的平台已提供），下载后放进 `PATH` 即可。

## 单租户

最简部署：一个 API key，扁平目录。

```sh
observer-server \
  --bind 0.0.0.0:8080 \
  --data-dir ./recordings \
  --token sk_your_api_key
```

- `--bind`：监听地址端口。仅本机用可绑 `127.0.0.1:8080`，对外服务绑 `0.0.0.0:8080`。
- `--data-dir`：会话存储根目录，不存在会自动创建；备份 = 拷这个目录。
- `--token`：API key。客户端请求须带 `Authorization: Bearer sk_your_api_key`；**留空 = 不鉴权，仅建议本机回环使用**。
- 等价环境变量：`OBSERVER_BIND` / `OBSERVER_DATA_DIR` / `OBSERVER_TOKEN`。

**✅ 验证**：`curl http://127.0.0.1:8080/whoami -H "Authorization: Bearer sk_your_api_key"` 返回租户上下文 JSON。

## 多租户

一份 `tenants.json` 管多个团队 / 多个应用，各自配额、保留、脱敏、限流。

```sh
observer-server \
  --bind 0.0.0.0:8080 \
  --data-dir ./recordings \
  --tenants-file ./tenants.json
```

`tenants.json` 字段：

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
  {
    "key": "sk_beta_dev",
    "tenantId": "beta",
    "appIds": [],
    "retention": { "maxSessions": 200 }
  }
]
```

::: danger scrubbers 是 JSON 字符串里的正则——反斜杠要双写
上例 `"Bearer\\s+[\\w.-]+"` 看起来多了一层 `\`，这是**必须的**：JSON 字符串中 `\s` 是非法转义，解析直接报错。正则里的每个 `\` 在 JSON 里都要写成 `\\`。

```text
✅ 正确： "scrubbers": ["Bearer\\s+[\\w.-]+"]
❌ 错误： "scrubbers": ["Bearer\s+[\w.-]+"]   ← JSON 解析失败，server 起不来
```
:::

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `key` | ✅ | 该租户的 API key（`Authorization: Bearer`） |
| `tenantId` | ✅ | 租户标识，存储隔离为 `recordings/<tenantId>/<sessionId>/` |
| `appIds` | ✅ | 允许的 appId 集合；空数组 = 不限制。上报 / 上传的 `session.appId` 须在此集合内 |
| `quotaBytes` | ➖ | 该租户磁盘配额（字节） |
| `retention` | ➖ | `{ maxAgeDays?, maxSessions? }`，超限自动清理 |
| `redact` | ➖ | 服务端入库前脱敏（同 SDK `redact` 选项 + `scrubbers` 正则） |
| `rateLimit` | ➖ | `{ maxRpm }`，每分钟请求上限 |

## 常驻运行（systemd）

生产环境建议用 systemd 托管（开机自启 + 崩溃重启）：

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
journalctl -u observer-server -f   # 看日志（server 日志输出到 stderr）
```

## 浏览器化托管（零客户端安装）

> 本节需要先克隆本仓库（静态产物来自仓库前端构建）。

把 console 前端构建产物交给 server 托管，团队成员浏览器打开地址即得完整分析台，无需安装桌面应用：

```sh
# 1. 在仓库根构建 console 前端（产物在 dist/）
pnpm install
pnpm build

# 2. server 托管静态文件 + 提供 API
observer-server \
  --bind 0.0.0.0:8080 \
  --data-dir ./recordings \
  --tenants-file ./tenants.json \
  --web-dir ./dist
```

未命中 API 的请求 fallback 到静态文件（SPA 模式 + 路径穿越防护 + MIME 分派）。浏览器开 `http://<host>:8080` 即用；`GET /whoami` 暴露当前 tenant 上下文 + 配额余量。

## 反向代理终止 TLS

server 本身只跑 HTTP，TLS 建议由反代终止。

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
        # 大 bundle 上传
        client_max_body_size 200m;
    }
}
```

```text [Caddyfile]
prism.internal {
    reverse_proxy 127.0.0.1:8080
    # Caddy 自动签发并续期 TLS 证书
}
```

:::

## 数据目录与保留

- `--data-dir` 即会话存储根；备份 = 拷目录，迁移 = 移目录。
- 多租户按 `recordings/<tenantId>/<sessionId>/` 隔离。
- `retention` 触发自动清理（按 `maxAgeDays` / `maxSessions`）。

## 数据卫生

- **服务端脱敏**：`tenants.json` 的 `redact` 在入库前剥离 network body / headers，并用 `scrubbers` 正则 scrub token / 邮箱等。
- **限流**：`rateLimit.maxRpm` 防滥用。
- **响应压缩**：API 响应支持 gzip（响应体大于 1KB 且客户端带 `Accept-Encoding: gzip` 时），落盘高压缩比 JSON。
- 离线 bundle 上传时，凭 key 鉴权，落入对应 tenant；`session.appId` 须与 key 授权集合匹配，否则拒收。

## SDK 侧对接

把 `endpoint` 指向你的部署地址即可，本地与云端同一份代码：

```ts
import { init } from "@prism-obs/observer-sdk";

init({
  appId: "shop-web",
  endpoint: "https://prism.internal",   // 你的 observer-server
  token: "sk_acme_prod",                 // 租户 key
});
```

离线 bundle 上传：`POST /sessions/import`（带 Bearer key）。

## 故障排查

| 现象 | 多半是 | 解法 |
| --- | --- | --- |
| server 起不来，报 tenants.json 解析错误 | `scrubbers` 正则的 `\` 没双写 | 见上文 JSON 转义警示 |
| SDK 上报 401 | token / key 不匹配 | 单租户对照 `--token`，多租户对照 tenants.json 的 `key` |
| SDK 上报 403 / 拒收 | `appId` 不在该租户 `appIds` 集合 | 把 appId 加进集合，或置空数组不限制 |
| 浏览器打开白屏 | `--web-dir` 未指向构建产物 | 确认指向 `dist/` 目录且 `pnpm build` 已跑过 |

## 升级与运维

- 升级 = 拉取最新代码重新 `cargo install --path crates/observer-server` 替换二进制重启；数据目录格式向前兼容。
- 日志输出到 stderr；健康检查可探 `GET /whoami`（开启鉴权时需带 Bearer key）。
- 单租户起步、多租户按需开启——同一份二进制，配置开关而非代码分支。
