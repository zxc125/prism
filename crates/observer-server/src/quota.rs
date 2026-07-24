//! 每租户磁盘配额追踪：惰性扫描初始化 + 写入累加。
//!
//! 配额在 ingest 前预检（[`QuotaTracker::check`]），写入成功后记账（[`QuotaTracker::add`]）。
//! 用量按未压缩字节估算（偏保守，安全）。启动时不全量扫描，首次访问该租户时才扫。
//! 周期重扫校正漂移留后续（P9 起步够用）。

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct QuotaTracker {
    usage: Arc<Mutex<HashMap<String, Arc<AtomicU64>>>>,
}

impl QuotaTracker {
    pub fn new() -> Self {
        Self {
            usage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 取（或惰性初始化）租户用量句柄。首次访问扫描 tenant_root。
    fn handle(&self, tenant_id: &str, tenant_root: &Path) -> Arc<AtomicU64> {
        // fast path：已初始化
        {
            let m = self.usage.lock().expect("quota map poisoned");
            if let Some(arc) = m.get(tenant_id) {
                return arc.clone();
            }
        }
        // slow path：扫描目录
        let bytes = dir_size(tenant_root);
        let arc = Arc::new(AtomicU64::new(bytes));
        let mut m = self.usage.lock().expect("quota map poisoned");
        // 双检：另一线程可能已插入
        m.entry(tenant_id.to_string()).or_insert(arc).clone()
    }

    /// 检查写入 `incoming` 字节后是否超配额。true = 允许。
    pub fn check(&self, tenant_id: &str, tenant_root: &Path, quota: u64, incoming: u64) -> bool {
        let arc = self.handle(tenant_id, tenant_root);
        let current = arc.load(Ordering::Relaxed);
        current.saturating_add(incoming) <= quota
    }

    /// 记账写入字节数。
    pub fn add(&self, tenant_id: &str, tenant_root: &Path, bytes: u64) {
        let arc = self.handle(tenant_id, tenant_root);
        arc.fetch_add(bytes, Ordering::Relaxed);
    }
}

/// 递归求目录总字节。不存在返回 0。
fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(meta) = e.metadata() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn check_and_add_tracks_usage() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let tracker = QuotaTracker::new();

        // 初始 0
        assert!(tracker.check("t1", root, 1000, 100));
        tracker.add("t1", root, 100);
        // 再加 900 = 1000，刚好不超
        assert!(tracker.check("t1", root, 1000, 900));
        // 再加 1 超限
        assert!(!tracker.check("t1", root, 1000, 901));
    }

    /// 惰性初始化：扫描已有文件。
    #[test]
    fn init_scans_existing_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.json"), "hello").unwrap(); // 5 bytes
        let tracker = QuotaTracker::new();
        // 首次 check 触发扫描，5 bytes 已存在 -> 再加 995 ok，996 超限
        assert!(tracker.check("t1", root, 1000, 995));
        assert!(!tracker.check("t1", root, 1000, 996));
    }

    /// 不同租户隔离。
    #[test]
    fn per_tenant_isolation() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let tracker = QuotaTracker::new();
        tracker.add("t1", root, 500);
        // t2 不受 t1 影响
        assert!(tracker.check("t2", root, 100, 100));
    }

    /// 多次 add 累加；handle 只扫描一次（后续 fast path）。
    #[test]
    fn add_accumulates() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let tracker = QuotaTracker::new();
        tracker.add("t1", root, 100);
        tracker.add("t1", root, 200);
        // 现在 300，配额 350：再加 50 ok，51 超限
        assert!(tracker.check("t1", root, 350, 50));
        assert!(!tracker.check("t1", root, 350, 51));
    }
}
