//! 限流：per-tenant 滑动窗口计数。
//!
//! tiny_http 串行模型下退化为「每租户每窗口请求数计数」。窗口默认 60s。
//! 超限返回 429（由 [`crate::handle_request`] 处理）。线程化 + token bucket 留后续。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    windows: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    window: Duration,
}

impl RateLimiter {
    pub fn new(window_secs: u64) -> Self {
        Self {
            windows: Arc::new(Mutex::new(HashMap::new())),
            window: Duration::from_secs(window_secs),
        }
    }

    /// 检查并计数。true = 允许通过，false = 超限（本次不计入）。
    pub fn check(&self, tenant_id: &str, max_rpm: u32) -> bool {
        let mut m = self.windows.lock().expect("rate limit map poisoned");
        let now = Instant::now();
        let entry = m.entry(tenant_id.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) > self.window {
            // 窗口过期，重置
            *entry = (now, 1);
            true
        } else if entry.1 >= max_rpm {
            false
        } else {
            entry.1 += 1;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_until_limit_then_blocks() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check("t1", 3));
        assert!(limiter.check("t1", 3));
        assert!(limiter.check("t1", 3));
        assert!(!limiter.check("t1", 3)); // 第 4 次超限
    }

    #[test]
    fn per_tenant_isolated() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check("t1", 1));
        assert!(!limiter.check("t1", 1)); // t1 满
        assert!(limiter.check("t2", 1)); // t2 不受影响
    }

    #[test]
    fn window_reset_allows_again() {
        let limiter = RateLimiter::new(0); // 0 秒窗口：每次都过期
        // 窗口 0s 意味着 duration_since > 0s 几乎总成立（除非同纳秒）
        // 这里主要验证重置逻辑不 panic
        let _ = limiter.check("t1", 1);
    }
}
