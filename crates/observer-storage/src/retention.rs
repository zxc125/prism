//! 会话保留策略：超龄/孤儿/超量清理（带保护名单变体）。
//!
//! 供 observer-server 后台清扫线程（多租户 per-tenant）、console 本地模式（单租户）
//! 与 tauri-plugin-observer Local 清扫线程（P23）共用。纯逻辑，吃 `&Path`，零 tauri 依赖。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::bundle::{list_sessions, validate_session_id};
use crate::storage::now_ms;

/// 保留策略：任一字段为 None 表示不限该维度。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RetentionPolicy {
    /// 会话 endedAt 超过 max_age_days 被清理（未结束的会话不在本路径——孤儿见
    /// [`RetentionPolicy::orphan_max_age_days`]）。
    pub max_age_days: Option<u32>,
    /// 按 startedAt 倒序保留 max_sessions 条，余者清理。
    pub max_sessions: Option<u32>,
    /// 孤儿清理（P23）：无 endedAt 且 startedAt 超过 orphan_max_age_days 的会话被清理，
    /// 覆盖崩溃/强杀/未登出直接退出产生的无收口会话（活跃会话由 protect 名单豁免）。
    /// None = 不清孤儿（0.1.x 语义）。
    pub orphan_max_age_days: Option<u32>,
}

impl RetentionPolicy {
    pub fn is_empty(&self) -> bool {
        self.max_age_days.is_none()
            && self.max_sessions.is_none()
            && self.orphan_max_age_days.is_none()
    }
}

/// `Some(0)` 视同 `None`：策略生效首启 30s 内删光存量不可能是合法意图，防宿主笔误。
fn effective(v: Option<u32>) -> Option<u32> {
    v.filter(|&x| x > 0)
}

/// 删除纪律：meta.id 先过 [`validate_session_id`]（id 取自 session.json，防 `../`
/// 路径穿越与 id≠目录名误删），`remove_dir_all` 实际成功才算删除。
fn remove_session_dir(root: &Path, id: &str) -> bool {
    if !validate_session_id(id) {
        return false;
    }
    fs::remove_dir_all(root.join(id)).is_ok()
}

/// 按策略清理 root 下的会话，`protect` 名单中的会话 id 三条路径均跳过（如插件侧
/// 活跃会话，P23）。返回实际删除数（删除失败不计，如 Windows 文件锁，下轮重试）。
///
/// - 超龄：`endedAt < now - max_age_days` 删除（无 endedAt 的会话不在本路径）。
/// - 孤儿：无 endedAt 且 `startedAt < now - orphan_max_age_days` 删除（起点不明保守保留）。
/// - 超量：非 protect 会话按 startedAt 倒序保留前 max_sessions 条（protect 不占名额），
///   余者删除。
///
/// 纪律：`Some(0)` 一律视同 `None`；meta.id 非法跳过不删不计数；session.json 缺失/
/// 损坏的目录不在扫描结果内，三条路径均不可见（已知边界）。策略为空（三维全
/// None/0）直接返回 0，不做扫描。
pub fn enforce_retention_protect(
    root: &Path,
    policy: &RetentionPolicy,
    protect: &[&str],
) -> Result<usize, String> {
    let max_age = effective(policy.max_age_days);
    let orphan_age = effective(policy.orphan_max_age_days);
    let max_sessions = effective(policy.max_sessions);
    if max_age.is_none() && orphan_age.is_none() && max_sessions.is_none() {
        return Ok(0);
    }
    let protected: HashSet<&str> = protect.iter().copied().collect();
    let mut sessions = list_sessions(root);
    let now = now_ms();
    let mut removed = 0usize;

    // 超龄清理（只认 endedAt）
    if let Some(days) = max_age {
        let cutoff = now - (days as i64) * 86_400_000;
        sessions.retain(|s| {
            let Some(id) = s["id"].as_str() else {
                return true;
            };
            if protected.contains(id) {
                return true;
            }
            let overage = matches!(s["endedAt"].as_i64(), Some(t) if t < cutoff);
            if overage && remove_session_dir(root, id) {
                removed += 1;
                return false;
            }
            true
        });
    }

    // 孤儿清理（无 endedAt 且 startedAt 超龄）
    if let Some(days) = orphan_age {
        let cutoff = now - (days as i64) * 86_400_000;
        sessions.retain(|s| {
            let Some(id) = s["id"].as_str() else {
                return true;
            };
            if protected.contains(id) {
                return true;
            }
            if s["endedAt"].as_i64().is_some() {
                return true; // 有收口归超龄/超量路径
            }
            let Some(started) = s["startedAt"].as_i64() else {
                return true;
            };
            if started < cutoff && remove_session_dir(root, id) {
                removed += 1;
                return false;
            }
            true
        });
    }

    // 超量清理：非 protect 集合按 startedAt 倒序，保留前 max_sessions
    if let Some(max) = max_sessions {
        let mut eligible: Vec<&serde_json::Value> = sessions
            .iter()
            .filter(|s| s["id"].as_str().map_or(true, |id| !protected.contains(id)))
            .collect();
        eligible.sort_by(|a, b| b["startedAt"].as_i64().cmp(&a["startedAt"].as_i64()));
        for s in eligible.iter().skip(max as usize) {
            if let Some(id) = s["id"].as_str() {
                if remove_session_dir(root, id) {
                    removed += 1;
                }
            }
        }
    }

    Ok(removed)
}

/// 兼容保留（0.1.x 签名）：等价 protect 空名单。既有调用方（server 清扫线程 /
/// console 单租户）策略不含 [`RetentionPolicy::orphan_max_age_days`]（serde default
/// None）→ 行为不变，仅删除纪律（防呆/校验/实际计数）随变体生效。
pub fn enforce_retention(root: &Path, policy: &RetentionPolicy) -> Result<usize, String> {
    enforce_retention_protect(root, policy, &[])
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
        make_session(root.path(), "900000000002", now - 86_400_000 * 40, Some(now - 86_400_000 * 40));
        make_session(root.path(), "900000000003", now - 1000, Some(now - 1000));
        make_session(root.path(), "900000000004", now - 2000, None); // 活跃，不清理

        let removed = enforce_retention(
            root.path(),
            &RetentionPolicy {
                max_age_days: Some(30),
                max_sessions: None,
                orphan_max_age_days: None,
            },
        )
        .unwrap();
        assert_eq!(removed, 1);
        assert!(!root.path().join("900000000002").exists());
        assert!(root.path().join("900000000003").exists());
        assert!(root.path().join("900000000004").exists());
    }

    /// 超量按 startedAt 倒序淘汰。
    #[test]
    fn enforces_max_sessions() {
        let root = tempdir().unwrap();
        make_session(root.path(), "1001", 1000, Some(1100));
        make_session(root.path(), "1002", 2000, Some(2100));
        make_session(root.path(), "1003", 3000, Some(3100));

        let removed = enforce_retention(
            root.path(),
            &RetentionPolicy {
                max_age_days: None,
                max_sessions: Some(2),
                orphan_max_age_days: None,
            },
        )
        .unwrap();
        assert_eq!(removed, 1);
        // 保留最新两条 s3/s2，淘汰 s1
        assert!(!root.path().join("1001").exists());
        assert!(root.path().join("1002").exists());
        assert!(root.path().join("1003").exists());
    }

    /// 空策略不扫描、不删。
    #[test]
    fn empty_policy_noop() {
        let root = tempdir().unwrap();
        make_session(root.path(), "1001", 1000, Some(1100));
        let removed = enforce_retention(root.path(), &RetentionPolicy::default()).unwrap();
        assert_eq!(removed, 0);
        assert!(root.path().join("1001").exists());
    }

    /// protect 名单中的已收口超龄会话保留（超龄路径被 protect 收窄）。
    #[test]
    fn protect_skips_in_max_age() {
        let root = tempdir().unwrap();
        let now = now_ms();
        make_session(root.path(), "900000000001", now - 86_400_000 * 40, Some(now - 86_400_000 * 40));
        make_session(root.path(), "900000000002", now - 86_400_000 * 40, Some(now - 86_400_000 * 40));

        let policy = RetentionPolicy {
            max_age_days: Some(30),
            max_sessions: None,
            orphan_max_age_days: None,
        };
        let removed = enforce_retention_protect(root.path(), &policy, &["900000000001"]).unwrap();
        assert_eq!(removed, 1);
        assert!(root.path().join("900000000001").exists());
        assert!(!root.path().join("900000000002").exists());
    }

    /// 超量路径 protect 不占名额：受保护的最老会话保留，顺位多删一条。
    #[test]
    fn protect_skips_in_max_sessions() {
        let root = tempdir().unwrap();
        make_session(root.path(), "1001", 1000, Some(1100));
        make_session(root.path(), "1002", 2000, Some(2100));
        make_session(root.path(), "1003", 3000, Some(3100));
        make_session(root.path(), "1004", 4000, Some(4100));

        let policy = RetentionPolicy {
            max_age_days: None,
            max_sessions: Some(2),
            orphan_max_age_days: None,
        };
        let removed = enforce_retention_protect(root.path(), &policy, &["1001"]).unwrap();
        // eligible = s2/s3/s4（s1 受保护不占名额），保留 s4/s3，删 s2
        assert_eq!(removed, 1);
        assert!(root.path().join("1001").exists());
        assert!(!root.path().join("1002").exists());
        assert!(root.path().join("1003").exists());
        assert!(root.path().join("1004").exists());
    }

    /// 孤儿：无 endedAt 且 startedAt 超龄可清（orphan_max_age_days 显式开启）。
    #[test]
    fn orphan_swept_by_started_at() {
        let root = tempdir().unwrap();
        let now = now_ms();
        make_session(root.path(), "800000000001", now - 86_400_000 * 40, None);
        make_session(root.path(), "800000000002", now - 86_400_000 * 40, Some(now - 86_400_000 * 40));

        let policy = RetentionPolicy {
            max_age_days: None,
            max_sessions: None,
            orphan_max_age_days: Some(30),
        };
        let removed = enforce_retention_protect(root.path(), &policy, &[]).unwrap();
        assert_eq!(removed, 1);
        assert!(!root.path().join("800000000001").exists());
        assert!(root.path().join("800000000002").exists()); // 有 endedAt 不归孤儿路径
    }

    /// 孤儿路径同样受 protect 豁免（宿主合法长活会话，P22 D5）。
    #[test]
    fn orphan_skips_protected() {
        let root = tempdir().unwrap();
        let now = now_ms();
        make_session(root.path(), "800000000003", now - 86_400_000 * 40, None);

        let policy = RetentionPolicy {
            max_age_days: None,
            max_sessions: None,
            orphan_max_age_days: Some(30),
        };
        let removed = enforce_retention_protect(root.path(), &policy, &["800000000003"]).unwrap();
        assert_eq!(removed, 0);
        assert!(root.path().join("800000000003").exists());
    }

    /// orphan 字段 None = 0.1.x 语义：无 endedAt 一律保留（legacy 回归锚）。
    #[test]
    fn orphan_none_keeps_legacy_semantics() {
        let root = tempdir().unwrap();
        let now = now_ms();
        make_session(root.path(), "800000000001", now - 86_400_000 * 40, None);

        let policy = RetentionPolicy {
            max_age_days: Some(30),
            max_sessions: None,
            orphan_max_age_days: None,
        };
        let removed = enforce_retention_protect(root.path(), &policy, &[]).unwrap();
        assert_eq!(removed, 0);
        assert!(root.path().join("800000000001").exists());
    }

    /// `Some(0)` 三维一律视同 None：不删任何东西（防宿主笔误）。
    #[test]
    fn zero_values_treated_as_noop() {
        let root = tempdir().unwrap();
        let now = now_ms();
        make_session(root.path(), "900000000002", now - 86_400_000 * 40, Some(now - 86_400_000 * 40));
        make_session(root.path(), "800000000001", now - 86_400_000 * 40, None);

        let policy = RetentionPolicy {
            max_age_days: Some(0),
            max_sessions: Some(0),
            orphan_max_age_days: Some(0),
        };
        let removed = enforce_retention_protect(root.path(), &policy, &[]).unwrap();
        assert_eq!(removed, 0);
        assert!(root.path().join("900000000002").exists());
        assert!(root.path().join("800000000001").exists());
    }

    /// meta.id 非法（路径穿越 / 与目录名不符）跳过不删不计数。
    #[test]
    fn invalid_session_id_skipped() {
        let root = tempdir().unwrap();
        let now = now_ms();
        let dir = root.path().join("dir-a");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({
                "id": "../evil",
                "startedAt": now - 86_400_000 * 40,
                "endedAt": now - 86_400_000 * 40
            })
            .to_string(),
        )
        .unwrap();

        let policy = RetentionPolicy {
            max_age_days: Some(30),
            max_sessions: None,
            orphan_max_age_days: None,
        };
        let removed = enforce_retention_protect(root.path(), &policy, &[]).unwrap();
        assert_eq!(removed, 0);
        assert!(root.path().join("dir-a").exists());
    }

    /// 原函数 = protect 空名单（委托等价回归锚）。
    #[test]
    fn delegate_empty_protect_equivalent() {
        let root = tempdir().unwrap();
        let now = now_ms();
        make_session(root.path(), "900000000002", now - 86_400_000 * 40, Some(now - 86_400_000 * 40));

        let policy = RetentionPolicy {
            max_age_days: Some(30),
            max_sessions: None,
            orphan_max_age_days: None,
        };
        let removed = enforce_retention(root.path(), &policy).unwrap();
        assert_eq!(removed, 1);
        assert!(!root.path().join("900000000002").exists());
    }
}
