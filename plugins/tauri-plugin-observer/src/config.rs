//! 插件运行时配置。

use serde::{Deserialize, Serialize};

use observer_storage::RetentionPolicy;

/// 部署模式：Rust 落盘（console 自录 / 外部应用 opt-in 本地落盘）或仅协调（外部应用走 HTTP 上报）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Rust 侧直接落盘到 `<dir_base>/<dir_name>`（默认 `appDataDir/recordings/`，
    /// P20 可配）。console self-obs 与外部应用 opt-in 本地落盘（P16）均用此模式。
    Local,
    /// Rust 侧只管窗口协调 + 状态 + 事件驱动，不落盘；前端 `HttpSink` 上报到 console。
    Remote,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Local
    }
}

/// Local 模式落盘基目录（P20）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DirBase {
    /// `appDataDir`（默认，与 0.3.x 现状一致）。
    #[default]
    AppData,
    /// 资源目录：Windows NSIS 安装布局下为主程序 exe 所在目录。
    ResourceDir,
}

fn default_main_label() -> String {
    "main".to_string()
}

fn default_source() -> String {
    "self".to_string()
}

fn default_dir_name() -> String {
    "recordings".to_string()
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
    /// 写入 session.json 的 `source` 字段。console 自录保持默认 `"self"`；外部应用
    /// 本地落盘建议 `"tauri"`（console 导入后来源色按此生效）。
    #[serde(default = "default_source")]
    pub source: String,
    /// 外部应用标识，写入 session.json 的 `appId` 键；None 时省键（P16 D9，与
    /// console 现网 session.json 形态一致）。默认 None。
    #[serde(default)]
    pub app_id: Option<String>,
    /// Local 模式落盘基目录，默认 AppData（P20）。
    #[serde(default)]
    pub dir_base: DirBase,
    /// 基目录下的子目录名，默认 `"recordings"`（P20）。
    #[serde(default = "default_dir_name")]
    pub dir_name: String,
    /// Local 模式落盘保留策略（P23）；缺省 = 三维均 None = 不清扫（现状）。
    /// 声明策略即授权插件后台自动清理（opt-in）；`orphan_max_age_days` 覆盖
    /// 崩溃/强杀/未登出直接退出产生的无 endedAt 会话，活跃会话始终豁免。
    #[serde(default)]
    pub retention: RetentionPolicy,
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            main_label: default_main_label(),
            skip_focus_prefix: String::new(),
            source: default_source(),
            app_id: None,
            dir_base: DirBase::AppData,
            dir_name: default_dir_name(),
            retention: RetentionPolicy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 存量序列化往返兼容：缺 retention 键反序列化得空策略（零行为变化）。
    #[test]
    fn missing_retention_defaults_to_empty() {
        let cfg: ObserverConfig = serde_json::from_str("{}").unwrap();
        assert!(cfg.retention.is_empty());
        assert_eq!(cfg.retention, RetentionPolicy::default());
    }

    /// camelCase 全字段往返：宿主声明策略后 serde 形态稳定。
    #[test]
    fn retention_roundtrip_camel_case() {
        let cfg = ObserverConfig {
            retention: RetentionPolicy {
                max_age_days: Some(30),
                max_sessions: None,
                orphan_max_age_days: Some(30),
            },
            ..Default::default()
        };
        let s = serde_json::to_string(&cfg).unwrap();
        assert!(s.contains("\"retention\":{\"maxAgeDays\":30"));
        assert!(s.contains("\"orphanMaxAgeDays\":30"));
        let back: ObserverConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.retention.max_age_days, Some(30));
        assert_eq!(back.retention.orphan_max_age_days, Some(30));
        assert_eq!(back.retention.max_sessions, None);
    }
}
