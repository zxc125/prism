//! 导出/导入前脱敏：剥离/scrub 会话中的 PII（network body/headers、console args、url 等）。
//!
//! Rust 移植自 [`redact.ts`](../../packages/observer-sdk/src/redact.ts)，供服务端
//! `POST /sessions/import` 入库前集中脱敏（P9）。客户端 SDK 仍在导出前自脱敏，
//! 服务端 redact 是第二道闸（per-tenant scrubbers），**不可逆**。
//!
//! 仅处理 `type:6` 诊断信号事件；DOM 快照等不动（与 TS 侧一致）。

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 脱敏配置（来自 tenants.json，scrubbers 为 regex 源码字符串）。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactConfig {
    #[serde(default = "default_true")]
    pub strip_network_body: bool,
    #[serde(default = "default_true")]
    pub strip_network_headers: bool,
    #[serde(default)]
    pub drop_network: bool,
    #[serde(default)]
    pub drop_console: bool,
    #[serde(default)]
    pub scrubbers: Vec<String>,
}

impl Default for RedactConfig {
    fn default() -> Self {
        Self {
            strip_network_body: true,
            strip_network_headers: true,
            drop_network: false,
            drop_console: false,
            scrubbers: vec![],
        }
    }
}

fn default_true() -> bool {
    true
}

/// 编译后的脱敏选项（scrubbers 已编译为 [`Regex`]，运行时零解析成本）。
pub struct RedactOpts {
    pub strip_network_body: bool,
    pub strip_network_headers: bool,
    pub drop_network: bool,
    pub drop_console: bool,
    pub scrubbers: Vec<Regex>,
}

impl Default for RedactOpts {
    fn default() -> Self {
        Self {
            strip_network_body: true,
            strip_network_headers: true,
            drop_network: false,
            drop_console: false,
            scrubbers: vec![],
        }
    }
}

impl RedactConfig {
    /// 编译 scrubbers regex，构建运行时 [`RedactOpts`]。
    pub fn to_opts(&self) -> Result<RedactOpts, String> {
        let scrubbers = self
            .scrubbers
            .iter()
            .map(|s| {
                Regex::new(s).map_err(|e| format!("非法 scrubber regex {s:?}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(RedactOpts {
            strip_network_body: self.strip_network_body,
            strip_network_headers: self.strip_network_headers,
            drop_network: self.drop_network,
            drop_console: self.drop_console,
            scrubbers,
        })
    }
}

/// 对 bundle 就地脱敏（segments 内 `type:6` 信号事件 + session.url）。
/// 改变传入 bundle，不返回新对象。
pub fn redact_bundle(bundle: &mut Value, opts: &RedactOpts) {
    if let Some(segs) = bundle.get_mut("segments").and_then(|v| v.as_object_mut()) {
        for events_val in segs.values_mut() {
            let Some(arr) = events_val.as_array_mut() else {
                continue;
            };
            let mut out: Vec<Value> = Vec::with_capacity(arr.len());
            for e in arr.drain(..) {
                if let Some(kept) = redact_event(e, opts) {
                    out.push(kept);
                }
            }
            *events_val = Value::Array(out);
        }
    }
    // session.url 可能带 query token，过一遍 scrubbers
    if !opts.scrubbers.is_empty() {
        if let Some(url) = bundle
            .get_mut("session")
            .and_then(|s| s.get_mut("url"))
            .and_then(|u| u.as_str().map(|s| s.to_string()))
        {
            let scrubbed = scrub_str(&url, &opts.scrubbers);
            if let Some(url_val) = bundle.get_mut("session").and_then(|s| s.get_mut("url")) {
                *url_val = Value::String(scrubbed);
            }
        }
    }
}

/// 单个事件脱敏；返回 None 表示丢弃该事件。
fn redact_event(e: Value, opts: &RedactOpts) -> Option<Value> {
    // 非 type:6（DOM 快照等）不动
    if e.get("type").and_then(|t| t.as_i64()) != Some(6) {
        return Some(e);
    }
    let mut e = e;
    let data = e.get_mut("data")?;
    let plugin = data
        .get("plugin")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    if plugin == "network" && opts.drop_network {
        return None;
    }
    if plugin == "console" && opts.drop_console {
        return None;
    }
    if let Some(payload) = data.get_mut("payload") {
        if plugin == "network" {
            if let Some(obj) = payload.as_object_mut() {
                if opts.strip_network_body {
                    obj.remove("reqBody");
                    obj.remove("resBody");
                }
                if opts.strip_network_headers {
                    obj.remove("reqHeaders");
                    obj.remove("resHeaders");
                }
            }
        }
        if !opts.scrubbers.is_empty() {
            scrub_value(payload, &opts.scrubbers);
        }
    }
    Some(e)
}

fn scrub_str(s: &str, scrubbers: &[Regex]) -> String {
    let mut out = s.to_string();
    for re in scrubbers {
        out = re.replace_all(&out, "[REDACTED]").to_string();
    }
    out
}

/// 递归对字符串值套用 scrubbers；非字符串原样返回。
fn scrub_value(v: &mut Value, scrubbers: &[Regex]) {
    match v {
        Value::String(s) => {
            *s = scrub_str(s, scrubbers);
        }
        Value::Array(arr) => {
            for x in arr {
                scrub_value(x, scrubbers);
            }
        }
        Value::Object(obj) => {
            for (_, val) in obj.iter_mut() {
                scrub_value(val, scrubbers);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bundle_with_events(events: Vec<Value>) -> Value {
        json!({
            "format": "prism-session",
            "version": 1,
            "session": { "id": "s1", "url": "https://app.example.com/?token=secret123" },
            "windows": [],
            "segments": { "web#1": events },
            "annotations": [],
        })
    }

    /// network body/headers 被剥离，scrubbers 命中 url query token。
    #[test]
    fn redact_strips_network_and_scrubs() {
        let mut bundle = bundle_with_events(vec![
            json!({ "type": 2, "timestamp": 1 }),
            json!({
                "type": 6, "timestamp": 2,
                "data": { "plugin": "network", "payload": {
                    "url": "https://api.example.com/me",
                    "reqBody": "{\"password\":\"hunter2\"}",
                    "resBody": "{\"email\":\"a@b.com\"}",
                    "reqHeaders": { "Authorization": "Bearer xyz" },
                    "method": "GET"
                }}
            }),
            json!({
                "type": 6, "timestamp": 3,
                "data": { "plugin": "console", "payload": { "level": "error", "args": ["boom"] } }
            }),
        ]);
        let opts = RedactConfig {
            scrubbers: vec![r"token=\w+".to_string(), r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+".to_string()],
            ..Default::default()
        }
        .to_opts()
        .unwrap();

        redact_bundle(&mut bundle, &opts);

        let segs = bundle["segments"]["web#1"].as_array().unwrap();
        assert_eq!(segs.len(), 3); // 都保留（dropNetwork/dropConsole 默认 false）
        // DOM 事件不动
        assert_eq!(segs[0]["type"], 2);

        let net_payload = &segs[1]["data"]["payload"];
        assert!(net_payload.get("reqBody").is_none(), "reqBody 应被剥离");
        assert!(net_payload.get("resBody").is_none(), "resBody 应被剥离");
        assert!(net_payload.get("reqHeaders").is_none(), "reqHeaders 应被剥离");
        assert_eq!(net_payload["method"], "GET"); // 非敏感字段保留
        assert_eq!(net_payload["url"], "https://api.example.com/me"); // 未命中 scrubber

        // session.url 的 query token 被 scrub
        assert_eq!(
            bundle["session"]["url"],
            "https://app.example.com/?[REDACTED]"
        );
    }

    /// dropNetwork 丢弃整个 network 事件，console 保留。
    #[test]
    fn redact_drops_network_events() {
        let mut bundle = bundle_with_events(vec![
            json!({ "type": 6, "timestamp": 1, "data": { "plugin": "network", "payload": {} } }),
            json!({ "type": 6, "timestamp": 2, "data": { "plugin": "console", "payload": {} } }),
        ]);
        let opts = RedactOpts {
            drop_network: true,
            ..Default::default()
        };
        redact_bundle(&mut bundle, &opts);
        let segs = bundle["segments"]["web#1"].as_array().unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0]["data"]["plugin"], "console");
    }

    /// scrubbers 递归命中 console args 中的邮箱。
    #[test]
    fn redact_scrubs_nested_strings() {
        let mut bundle = bundle_with_events(vec![json!({
            "type": 6, "timestamp": 1,
            "data": { "plugin": "console", "payload": {
                "level": "log",
                "args": ["user email: a@b.com", { "nested": "cc@d.com" }]
            }}
        })]);
        let opts = RedactConfig {
            scrubbers: vec![r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+".to_string()],
            strip_network_body: false,
            strip_network_headers: false,
            ..Default::default()
        }
        .to_opts()
        .unwrap();
        redact_bundle(&mut bundle, &opts);
        let args = bundle["segments"]["web#1"][0]["data"]["payload"]["args"].as_array().unwrap();
        assert_eq!(args[0], "user email: [REDACTED]");
        assert_eq!(args[1]["nested"], "[REDACTED]");
    }

    /// 非法 scrubber regex 应报错。
    #[test]
    fn bad_scrubber_regex_errors() {
        let cfg = RedactConfig {
            scrubbers: vec!["[".to_string()],
            ..Default::default()
        };
        assert!(cfg.to_opts().is_err());
    }
}
