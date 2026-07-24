//! 会话保留策略：超龄/超量清理。
//!
//! 供 observer-server 后台清扫线程（多租户 per-tenant）与 console 本地模式（单租户）
//! 共用。纯逻辑，吃 `&Path`，零 tauri 依赖。

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::bundle::list_sessions;
use crate::storage::now_ms;

/// 保留策略：任一字段为 None 表示不限该维度。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    /// 会话 endedAt 超过 max_age_days 被清理（未结束的会话不清理）。
    pub max_age_days: Option<u32>,
    /// 按 startedAt 倒序保留 max_sessions 条，余者清理。
    pub max_sessions: Option<u32>,
}

impl RetentionPolicy {
    pub fn is_empty(&self) -> bool {
        self.max_age_days.is_none() && self.max_sessions.is_none()
    }
}

/// 按策略清理 root 下的会话：先超龄，再超量。返回移除数量。
///
/// - 超龄：`endedAt < now - max_age_days` 删除（无 endedAt 的活跃会话跳过）。
/// - 超量：剩余按 startedAt 倒序，保留前 max_sessions 条，余者删除。
///
/// 策略为空时直接返回 0，不做扫描。
pub fn enforce_retention(root: &Path, policy: &RetentionPolicy) -> Result<usize, String> {
    if policy.is_empty() {
        return Ok(0);
    }
    let mut sessions = list_sessions(root);
    let now = now_ms();
    let mut removed = 0usize;

    // 超龄清理
    if let Some(days) = policy.max_age_days {
        let cutoff = now - (days as i64) * 86_400_000;
        sessions.retain(|s| {
            let keep = s["endedAt"].as_i64().map_or(true, |t| t >= cutoff);
            if !keep {
                if let Some(id) = s["id"].as_str() {
                    let _ = fs::remove_dir_all(root.join(id));
                    removed += 1;
                }
            }
            keep
        });
    }

    // 超量清理：按 startedAt 倒序，保留前 max_sessions
    if let Some(max) = policy.max_sessions {
        sessions.sort_by(|a, b| b["startedAt"].as_i64().cmp(&a["startedAt"].as_i64()));
        for s in sessions.iter().skip(max as usize) {
            if let Some(id) = s["id"].as_str() {
                let _ = fs::remove_dir_all(root.join(id));
                removed += 1;
            }
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn make_session(root: &Path, id: &str, started: i64, ended: Option<i64>) {
        let dir = root.join(id);
        fs::create_dir_all(dir.join("segments")).unwrap();
        let mut s = json!({ "id": id, "startedAt": started });
        if let Some(e) = ended {
            s["endedAt"] = json!(e);
        }
        fs::write(dir.join("session.json"), s.to_string()).unwrap();
    }

    /// 超龄会话被清，活跃会话（无 endedAt）与未超龄保留。
    #[test]
    fn enforces_max_age() {
        let root = tempdir().unwrap();
        let now = now_ms();
        make_session(root.path(), "old", now - 86_400_000 * 40, Some(now - 86_400_000 * 40));
        make_session(root.path(), "fresh", now - 1000, Some(now - 1000));
        make_session(root.path(), "active", now - 2000, None); // 活跃，不清理

        let removed = enforce_retention(
            root.path(),
            &RetentionPolicy {
                max_age_days: Some(30),
                max_sessions: None,
            },
        )
        .unwrap();
        assert_eq!(removed, 1);
        assert!(!root.path().join("old").exists());
        assert!(root.path().join("fresh").exists());
        assert!(root.path().join("active").exists());
    }

    /// 超量按 startedAt 倒序淘汰。
    #[test]
    fn enforces_max_sessions() {
        let root = tempdir().unwrap();
        make_session(root.path(), "s1", 1000, Some(1100));
        make_session(root.path(), "s2", 2000, Some(2100));
        make_session(root.path(), "s3", 3000, Some(3100));

        let removed = enforce_retention(
            root.path(),
            &RetentionPolicy {
                max_age_days: None,
                max_sessions: Some(2),
            },
        )
        .unwrap();
        assert_eq!(removed, 1);
        // 保留最新两条 s3/s2，淘汰 s1
        assert!(!root.path().join("s1").exists());
        assert!(root.path().join("s2").exists());
        assert!(root.path().join("s3").exists());
    }

    /// 空策略不扫描、不删。
    #[test]
    fn empty_policy_noop() {
        let root = tempdir().unwrap();
        make_session(root.path(), "s1", 1000, Some(1100));
        let removed = enforce_retention(root.path(), &RetentionPolicy::default()).unwrap();
        assert_eq!(removed, 0);
        assert!(root.path().join("s1").exists());
    }
}
