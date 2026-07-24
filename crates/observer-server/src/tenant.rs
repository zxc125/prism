//! 租户配置与注册表：API key -> tenant 映射，多租户隔离的基础。
//!
//! `tenants.json` 配置文件（见 docs/架构/P9-多租户运营加固（方案）.md §2.1），
//! 启动时加载到 [`TenantRegistry`]。console 内嵌模式不传 tenants_file，走隐式单租户
//!（[`crate::ServerConfig`] 无 registry，[`crate::handle_request`] 透传 `tenant=None`）。

use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use observer_storage::{RedactConfig, RetentionPolicy};

/// 单个租户的配置（来自 tenants.json 一条记录）。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantConfig {
    pub key: String,
    pub tenant_id: String,
    #[serde(default)]
    pub app_ids: Vec<String>,
    #[serde(default)]
    pub quota_bytes: Option<u64>,
    #[serde(default)]
    pub retention: RetentionPolicy,
    #[serde(default)]
    pub redact: RedactConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

/// 限流配置（服务端运行时策略，非存储层关切）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitConfig {
    /// 每分钟请求数上限（滑动窗口）。
    pub max_rpm: Option<u32>,
}

/// key -> [`TenantConfig`] 注册表。`Arc<RwLock>` 便于热加载（P9 起步仅启动加载）。
#[derive(Clone)]
pub struct TenantRegistry {
    tenants: Arc<RwLock<Vec<TenantConfig>>>,
}

impl TenantRegistry {
    pub fn load(path: &Path) -> Result<Self, String> {
        let tenants = load_tenants(path)?;
        Ok(Self {
            tenants: Arc::new(RwLock::new(tenants)),
        })
    }

    /// 按 bearer key 查租户配置（clone 一份）。
    pub fn lookup(&self, key: &str) -> Option<TenantConfig> {
        self.tenants
            .read()
            .ok()?
            .iter()
            .find(|t| t.key == key)
            .cloned()
    }

    /// 所有租户快照（供保留清扫线程）。
    pub fn tenants(&self) -> Vec<TenantConfig> {
        self.tenants.read().map(|v| v.clone()).unwrap_or_default()
    }

    /// 热加载（替换全部）。
    pub fn reload(&self, path: &Path) -> Result<(), String> {
        let tenants = load_tenants(path)?;
        let mut w = self.tenants.write().map_err(|e| e.to_string())?;
        *w = tenants;
        Ok(())
    }
}

fn load_tenants(path: &Path) -> Result<Vec<TenantConfig>, String> {
    let s = fs::read_to_string(path).map_err(|e| format!("读取 tenants.json 失败: {e}"))?;
    let tenants: Vec<TenantConfig> =
        serde_json::from_str(&s).map_err(|e| format!("解析 tenants.json 失败: {e}"))?;
    for t in &tenants {
        if t.key.is_empty() || t.tenant_id.is_empty() {
            return Err("tenants.json: key/tenantId 不能为空".into());
        }
        // tenantId 会成为目录名，必须防路径穿越
        if !t
            .tenant_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(format!(
                "tenants.json: tenantId {} 含非法字符（仅 [A-Za-z0-9_-]）",
                t.tenant_id
            ));
        }
        // 编译 scrubbers 早暴露错误（启动即失败好过请求时 500）
        t.redact.to_opts()?;
    }
    Ok(tenants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_tenants(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
    }

    #[test]
    fn load_and_lookup() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("tenants.json");
        write_tenants(
            &p,
            r#"[
              { "key": "sk_a", "tenantId": "acme", "appIds": ["app1"] },
              { "key": "sk_b", "tenantId": "beta", "appIds": [] }
            ]"#,
        );
        let reg = TenantRegistry::load(&p).unwrap();
        assert_eq!(reg.lookup("sk_a").unwrap().tenant_id, "acme");
        assert_eq!(reg.lookup("sk_b").unwrap().tenant_id, "beta");
        assert!(reg.lookup("sk_unknown").is_none());
        assert_eq!(reg.tenants().len(), 2);
    }

    /// tenantId 含路径分隔符必须被拒（防目录穿越）。
    #[test]
    fn rejects_bad_tenant_id() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("tenants.json");
        write_tenants(&p, r#"[{ "key": "k", "tenantId": "../etc" }]"#);
        assert!(TenantRegistry::load(&p).is_err());
    }

    /// 非法 scrubber regex 启动即报错。
    #[test]
    fn rejects_bad_scrubber() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("tenants.json");
        write_tenants(
            &p,
            r#"[{ "key": "k", "tenantId": "t", "redact": { "scrubbers": ["["] } }]"#,
        );
        assert!(TenantRegistry::load(&p).is_err());
    }
}
