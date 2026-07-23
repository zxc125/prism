//! 插件运行时配置。

use serde::{Deserialize, Serialize};

/// 部署模式：Rust 落盘（console 自录）或仅协调（外部应用走 HTTP 上报）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Rust 侧直接落盘到 `appDataDir/recordings/`。console 自身 self-obs 用。
    Local,
    /// Rust 侧只管窗口协调 + 状态 + 事件驱动，不落盘；前端 `HttpSink` 上报到 console。
    Remote,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Local
    }
}

fn default_main_label() -> String {
    "main".to_string()
}

/// 插件配置。通过 [`crate::init_with`] 注入。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserverConfig {
    /// 部署模式，默认 Local。
    #[serde(default)]
    pub mode: Mode,
    /// 主窗口 label：其关闭 = 退出进程，不拦截为隐藏。默认 `"main"`。
    #[serde(default = "default_main_label")]
    pub main_label: String,
    /// 跳过 focus 记录的 label 前缀（如 console 的 `"player-"` 回放窗口）。默认空。
    /// 同时也用于 close 拦截跳过此类窗口。
    #[serde(default)]
    pub skip_focus_prefix: String,
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            main_label: default_main_label(),
            skip_focus_prefix: String::new(),
        }
    }
}
