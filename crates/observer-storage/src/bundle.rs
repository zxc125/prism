//! bundle 契约 + 会话读/列举/导入导出。
//!
//! bundle 是平台「会话跨进程/跨机迁移的唯一契约」（见 docs/架构/bundle-规范.md）：
//! 本地文件分享 / 本地 server 实时流 / 云端上传三路共用。本模块提供纯逻辑，
//! 供 console Tauri command 与 [`observer_server`](../../observer-server) HTTP 路由共用。

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::annotations::read_annotations;
use crate::storage::{
    now_ms, read_segment_events, read_segment_first_line, read_segment_last_line, read_segment_text,
    segment_id_from_filename, segment_path,
};

/// bundle 契约标识与版本（与 TS 侧 buildBundle/parseBundle 对齐，见 docs/架构/bundle-规范.md）。
pub const BUNDLE_FORMAT: &str = "prism-session";
pub const BUNDLE_VERSION: i64 = 1;

/// segmentId 合法性：`<label>#<n>`，label 仅 [A-Za-z0-9_-]。
/// segment key 会成为文件名（segments/<key>.jsonl），此校验是路径穿越防护的核心。
pub fn validate_segment_id(id: &str) -> bool {
    let Some((label, n)) = id.split_once('#') else {
        return false;
    };
    if label.is_empty() || n.is_empty() {
        return false;
    }
    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return false;
    }
    n.bytes().all(|b| b.is_ascii_digit())
}

/// session ID 合法性：纯数字（与 [`unique_id`] 生成器一致）。
/// read API 的 `/sessions/:id` 直接 `root.join(id)`，未校验可被 `../` 跨租户逃逸。
/// 多租户（P9）下这是必校验项；单租户也应校验。
pub fn validate_session_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

/// 分配不与本地已有会话冲突的新 id（毫秒时间戳，冲突则自增）。
pub fn unique_id(root: &Path) -> String {
    let mut id = now_ms();
    loop {
        let s = id.to_string();
        if !root.join(&s).exists() {
            return s;
        }
        id += 1;
    }
}

/// 读会话目录组装成回放数据（session + windows + segments + annotations）。
/// 供 console `read_session` 命令与 server `GET /sessions/:id` 共用。
/// segments 读路径 gzip 感知（`.jsonl` / `.jsonl.gz` 均可），见 [`read_segment_events`]。
pub fn read_session(dir: &Path) -> Result<Value, String> {
    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    // filter_map：单行损坏只跳过该行（与 read_session_meta / build_export_bundle 一致）
    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .map(|s| s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default();

    let segments = read_segments_dir(dir);
    let annotations = read_annotations(dir);
    Ok(json!({ "session": session, "windows": windows, "segments": segments, "annotations": annotations }))
}

/// 列出所有会话 meta（按 startedAt 倒序）。
pub fn list_sessions(root: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for e in entries.flatten() {
            if let Ok(meta) = fs::read_to_string(e.path().join("session.json")) {
                if let Ok(v) = serde_json::from_str::<Value>(&meta) {
                    out.push(v);
                }
            }
        }
    }
    out.sort_by(|a, b| b["startedAt"].as_i64().cmp(&a["startedAt"].as_i64()));
    out
}

/// 读 segments 目录所有段（gzip 感知），返回 segmentId -> events 映射。
fn read_segments_dir(dir: &Path) -> Value {
    let mut segments = serde_json::Map::new();
    let seg_dir = dir.join("segments");
    if let Ok(entries) = fs::read_dir(&seg_dir) {
        for e in entries.flatten() {
            let path = e.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let seg_id = segment_id_from_filename(&name).to_string();
            let events = read_segment_events(&path);
            segments.insert(seg_id, Value::Array(events));
        }
    }
    Value::Object(segments)
}

/// 直拼路径的廉价残行校验：括号配平（感知字符串与转义）。
///
/// 单靠「首尾是大括号」会漏掉一种截断（进程恰好在某个 `}` 之后被杀）：那类行能通过
/// 首尾检查，却让整份直拼结果不可解析。逐字节扫一遍比 `serde_json` 全量 parse 便宜
/// 一个数量级，且不需要解析器就能排除残行。
fn json_balanced(line: &str) -> bool {
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escaped = false;
    for b in line.bytes() {
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0 && !in_str
}

/// 段索引：`[{ segmentId, bytes, width, height }]`，O(段数) 读取。
///
/// 只读每段首行（Meta 事件含录制视口宽高）+ 文件字节数，**不碰段内事件**——
/// 这是「元信息先到、段按需拉」的取数基础（P19 D1）。首行缺失/非 Meta 时宽高为 null，
/// 回放侧退化为不缩放。
fn segment_index(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir.join("segments")) else {
        return out;
    };
    for e in entries.flatten() {
        let path = e.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let seg_id = segment_id_from_filename(&name).to_string();
        let bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
        let mut width = Value::Null;
        let mut height = Value::Null;
        let mut first_ts = Value::Null;
        if let Some(line) = read_segment_first_line(&path) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                if let Some(w) = v["data"]["width"].as_u64() {
                    width = json!(w);
                }
                if let Some(h) = v["data"]["height"].as_u64() {
                    height = json!(h);
                }
                if let Some(t) = v["timestamp"].as_i64() {
                    first_ts = json!(t);
                }
            }
        }
        // 尾事件时间戳：会话缺 endedAt / 段缺 hidden 时的时长兜底（外部应用导出的
        // bundle 常见——实测某 172MB 会话 27 段零 hidden、无 endedAt）
        let mut last_ts = Value::Null;
        if let Some(line) = read_segment_last_line(&path) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                if let Some(t) = v["timestamp"].as_i64() {
                    last_ts = json!(t);
                }
            }
        }
        out.push(json!({
            "segmentId": seg_id,
            "bytes": bytes,
            "width": width,
            "height": height,
            "firstTs": first_ts,
            "lastTs": last_ts,
        }));
    }
    out.sort_by(|a, b| a["segmentId"].as_str().cmp(&b["segmentId"].as_str()));
    out
}

/// 元信息 + 段索引（**不含段内事件**）：回放首屏所需的最小数据集合。
/// 供 console `read_session_meta` 与 server `GET /sessions/:id/meta` 共用。
pub fn read_session_meta(dir: &Path) -> Result<Value, String> {
    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // filter_map：单行损坏只跳过该行。旧写法（map + collect::<Option<Vec<_>>>）会让
    // 一行坏数据把整个 windows 数组变成空——而首屏的 label/hiddenAt 全靠它。
    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .map(|s| s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default();
    let annotations = read_annotations(dir);
    Ok(json!({
        "session": session,
        "windows": windows,
        "annotations": annotations,
        "segments": segment_index(dir),
    }))
}

/// 单段事件直拼为 JSON 数组文本：jsonl 每行本身即合法 JSON，串接即可，
/// **不做 `serde_json::Value` 往返**（P19 D2）——省掉实测 2.9GB 的中间树与二次序列化。
/// 调用方经 raw IPC 返回字节，JS 侧一次 `JSON.parse`。
///
/// 只做「首尾必须是大括号」的廉价校验：跳过被截断的残行（进程中途退出的典型损坏），
/// 不做逐行 parse（那正是本函数要避免的成本）。
pub fn read_segment_json(dir: &Path, segment_id: &str) -> Result<String, String> {
    if !validate_segment_id(segment_id) {
        return Err(format!("invalid segment id: {segment_id}"));
    }
    let path = segment_path(dir, segment_id)
        .ok_or_else(|| format!("segment not found: {segment_id}"))?;
    let raw = read_segment_text(&path).ok_or_else(|| format!("读取失败：{}", path.display()))?;
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('[');
    let mut first = true;
    for line in raw.lines() {
        let l = line.trim();
        if !l.starts_with('{') || !l.ends_with('}') || !json_balanced(l) {
            continue;
        }
        if !first {
            out.push(',');
        }
        out.push_str(l);
        first = false;
    }
    out.push(']');
    Ok(out)
}

/// 全库诊断信号（`type:6`）直拼为 JSON 数组文本。
///
/// 只做廉价子串筛选后直拼，**不 parse 任何事件**：DOM 事件占总量约 99%，逐个 parse
/// 正是旧 `read_session` 的主要成本。误纳入的行由 JS 侧按解析后的 `type` 过滤（多传的
/// 量极小）；判据是「包含」而非精确匹配，故不会漏判。
/// 用途：诊断面板首屏即可拿到完整信号流，不必等各段按需加载（P19 回归修复）。
pub fn read_session_signals_json(dir: &Path) -> Result<String, String> {
    let seg_dir = dir.join("segments");
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&seg_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() {
                files.push(p);
            }
        }
    }
    files.sort();
    let mut out = String::new();
    out.push('[');
    let mut first = true;
    for path in files {
        let Some(text) = read_segment_text(&path) else {
            continue;
        };
        for line in text.lines() {
            let l = line.trim();
            if !l.starts_with('{')
                || !l.ends_with('}')
                || !l.contains("\"type\":6")
                || !json_balanced(l)
            {
                continue;
            }
            if !first {
                out.push(',');
            }
            out.push_str(l);
            first = false;
        }
    }
    out.push(']');
    Ok(out)
}

/// 流式导出 bundle 到 writer（P25，[`build_export_bundle`] 的流式等价物）。
///
/// 骨架手写 JSON，segments 按 segmentId 字典序逐段流式：明文段
/// `BufRead::read_line` 逐行；gz 段（`.jsonl.gz`）套 [`flate2::read::MultiGzDecoder`]
/// 流式解压（复用 [`read_segment_first_line`] 验证过的多 member 模式）。每行 trim →
/// 空行跳过 → `serde_json::from_str` 校验 → **通过则直写原文行文本**（不重序列化，
/// 与 P19 D2 直拼精神一致）。session/windows/annotations 小件维持整读 + 序列化内嵌。
///
/// 失败分级（D6）：
/// - **行级**（残行 / 坏 JSON / 单行非 UTF-8）：剔行继续——与 [`read_segment_events`]
///   的 `filter_map` 剔行语义一致；
/// - **流级**（gz 截断等导致 read 持续失败）：连续 8 次 Err 即停止续读该段，
///   **保留已成功解出的行**，不中断导出——段键必存在，导出永不因坏段炸掉。
///
/// 与 [`build_export_bundle`] 的差异（预期改进，非回归）：整段非 UTF-8 / gz 截断时
/// build 路径整段为空，流式保留损坏点之前已解出的行。正常数据下两者解压后
/// Value 级 canonical 相等（键序不同：build 经 BTreeMap 重排，流式保留原文行键序）。
///
/// 内存 O(缓冲 + 最长单事件行)——不随会话总时长增长。返回写出的未压缩字节数。
pub fn write_export_bundle_streaming(dir: &Path, w: &mut impl std::io::Write) -> Result<u64, String> {
    use std::io::BufRead;

    /// write_all + 字节计数（write! 到 impl Write 上做计数需要包装器，直接累计更直白）。
    /// 参数 `impl std::io::Write` 的 bound 使 write_all 无需再 import trait。
    fn put(w: &mut impl std::io::Write, s: &str, n: &mut u64) -> Result<(), String> {
        w.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
        *n += s.len() as u64;
        Ok(())
    }

    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // filter_map：单行损坏只跳过该行（与 build_export_bundle 一致）
    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .map(|s| s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default();
    let annotations = read_annotations(dir);

    let mut n: u64 = 0;
    let head = format!(
        "{{\"format\":{},\"version\":{},\"exportedAt\":{},\"session\":{},\"windows\":{},\"segments\":{{",
        serde_json::to_string(BUNDLE_FORMAT).map_err(|e| e.to_string())?,
        BUNDLE_VERSION,
        now_ms(),
        session,
        serde_json::to_string(&windows).map_err(|e| e.to_string())?,
    );
    put(w, &head, &mut n)?;

    // 段枚举与 read_segments_dir 完全一致（不筛 is_file，读不出内容=空段），
    // 输出前按 segmentId 字典序排序（D4：对齐 build_export_bundle 的 BTreeMap 键序）
    let mut segs: Vec<(String, std::path::PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir.join("segments")) {
        for e in entries.flatten() {
            let path = e.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            segs.push((segment_id_from_filename(&name).to_string(), path));
        }
    }
    segs.sort_by(|a, b| a.0.cmp(&b.0));

    let mut first_seg = true;
    for (seg_id, path) in &segs {
        if !first_seg {
            put(w, ",", &mut n)?;
        }
        first_seg = false;
        // 键经 serde 转义（文件名理论上可含 `"` 等），与 Map insert 的序列化行为一致
        put(w, &serde_json::to_string(seg_id).map_err(|e| e.to_string())?, &mut n)?;
        put(w, ":[", &mut n)?;
        // open 失败（segments/ 混入子目录、权限异常、AV 锁定）= 空段：键保留、内容空
        // （D6：导出不因坏段失败；与 build 路径 read_segment_text .ok() 的容忍语义对齐）
        let Ok(f) = fs::File::open(path) else {
            put(w, "]", &mut n)?;
            continue;
        };
        let is_gz = path.extension().and_then(|e| e.to_str()) == Some("gz");
        let mut reader: Box<dyn BufRead> = if is_gz {
            Box::new(std::io::BufReader::new(flate2::read::MultiGzDecoder::new(f)))
        } else {
            Box::new(std::io::BufReader::new(f))
        };
        let mut first_line = true;
        let mut err_streak = 0u32;
        let mut buf = String::new();
        loop {
            buf.clear();
            match reader.read_line(&mut buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    err_streak = 0;
                    let t = buf.trim();
                    if t.is_empty() || serde_json::from_str::<Value>(t).is_err() {
                        continue; // 行级失败：剔行继续
                    }
                    if !first_line {
                        put(w, ",", &mut n)?;
                    }
                    first_line = false;
                    put(w, t, &mut n)?;
                }
                Err(_) => {
                    // 行读失败（非 UTF-8 行 / 解压错误）：先按行级剔除继续；
                    // 连续失败达上限（gz 截断等解压器卡死）停段，保留已解出行（D6）
                    err_streak += 1;
                    if err_streak >= 8 {
                        break;
                    }
                }
            }
        }
        put(w, "]", &mut n)?;
    }

    put(w, "},\"annotations\":", &mut n)?;
    put(w, &serde_json::to_string(&annotations).map_err(|e| e.to_string())?, &mut n)?;
    put(w, "}", &mut n)?;
    Ok(n)
}

/// 读会话目录组装成 export bundle（不依赖 AppHandle，便于测试）。
pub fn build_export_bundle(dir: &Path) -> Result<Value, String> {
    let session = serde_json::from_str::<Value>(
        &fs::read_to_string(dir.join("session.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let windows: Vec<Value> = fs::read_to_string(dir.join("windows.jsonl"))
        .ok()
        .map(|s| s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default();
    let segments = read_segments_dir(dir);
    let annotations = read_annotations(dir);
    Ok(json!({
        "format": BUNDLE_FORMAT,
        "version": BUNDLE_VERSION,
        "exportedAt": now_ms(),
        "session": session,
        "windows": windows,
        "segments": segments,
        "annotations": annotations,
    }))
}

/// 合并 meta 到 session.json（空串/null 删除字段，否则覆盖），返回写回后的 session。
pub fn merge_session_meta(path: &Path, meta: &Value) -> Result<Value, String> {
    let mut v = serde_json::from_str::<Value>(
        &fs::read_to_string(path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let (Some(obj), Some(m)) = (v.as_object_mut(), meta.as_object()) {
        for (k, val) in m {
            // 空字符串 / null = 删除该字段（清理），否则覆盖
            if (val.is_string() && val.as_str() == Some("")) || val.is_null() {
                obj.remove(k);
            } else {
                obj.insert(k.clone(), val.clone());
            }
        }
    }
    fs::write(path, v.to_string()).map_err(|e| e.to_string())?;
    Ok(v)
}

/// 把 bundle 重建为目标目录的文件结构（session.id 替换为 new_id，标记 importedAt）。
pub fn write_import_bundle(dir: &Path, bundle: &Value, new_id: &str) -> Result<(), String> {
    fs::create_dir_all(dir.join("segments")).map_err(|e| e.to_string())?;
    // session.json：保留原始 meta，替换 id 并标记导入时间
    let mut session = bundle["session"].clone();
    if let Some(obj) = session.as_object_mut() {
        obj.insert("id".into(), json!(new_id));
        obj.insert("importedAt".into(), json!(now_ms()));
    }
    fs::write(dir.join("session.json"), session.to_string()).map_err(|e| e.to_string())?;
    // windows.jsonl
    if let Some(wins) = bundle["windows"].as_array() {
        let mut body = String::new();
        for w in wins {
            body.push_str(&w.to_string());
            body.push('\n');
        }
        fs::write(dir.join("windows.jsonl"), body).map_err(|e| e.to_string())?;
    }
    // segments/<name>.jsonl -- name 来自 bundle，必须校验防路径穿越（B1）
    if let Some(segs) = bundle["segments"].as_object() {
        for (name, events) in segs {
            if !validate_segment_id(name) {
                return Err(format!("非法 segmentId（拒绝以防路径穿越）：{name}"));
            }
            if let Some(arr) = events.as_array() {
                let mut body = String::new();
                for e in arr {
                    body.push_str(&e.to_string());
                    body.push('\n');
                }
                fs::write(dir.join("segments").join(format!("{name}.jsonl")), body)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    // annotations.jsonl
    if let Some(annos) = bundle["annotations"].as_array() {
        crate::annotations::write_annotations(dir, annos)?;
    }
    Ok(())
}

/// 校验 bundle 格式与版本，返回解析后的 Value。
pub fn parse_bundle(content: &str) -> Result<Value, String> {
    let v = serde_json::from_str::<Value>(content).map_err(|e| e.to_string())?;
    if v["format"].as_str() != Some(BUNDLE_FORMAT) {
        return Err(format!("不是有效的会话文件（缺少 format: {BUNDLE_FORMAT}）"));
    }
    let version = v["version"].as_i64().unwrap_or(1);
    if version > BUNDLE_VERSION {
        return Err(format!(
            "不支持的 bundle 版本：{version}（当前支持 ≤{BUNDLE_VERSION}）"
        ));
    }
    Ok(v)
}

/// 导入已校验的 bundle 到 root 下：分配新 id、原子写（.tmp -> rename）。返回新 id。
pub fn import_bundle(root: &Path, bundle: &Value) -> Result<String, String> {
    let new_id = unique_id(root);
    let dir = root.join(&new_id);
    let tmp = root.join(format!("{new_id}.tmp"));
    // 原子写：先写临时目录，成功后 rename；任一步失败清理 tmp，不污染 recordings/
    if tmp.exists() {
        let _ = fs::remove_dir_all(&tmp);
    }
    match write_import_bundle(&tmp, bundle, &new_id) {
        Ok(()) => fs::rename(&tmp, &dir).map_err(|e| {
            let _ = fs::remove_dir_all(&tmp);
            e.to_string()
        })?,
        Err(e) => {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
    }
    Ok(new_id)
}

/// 导入 bundle（从 JSON 字符串）：校验 + 原子写。返回新 id。
/// 供 console `import_session(content)` 与 server `POST /sessions/import` 共用。
pub fn import_bundle_content(root: &Path, content: &str) -> Result<String, String> {
    let v = parse_bundle(content)?;
    import_bundle(root, &v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_jsonl(path: &Path, rows: &[Value]) {
        let mut body = String::new();
        for r in rows {
            body.push_str(&r.to_string());
            body.push('\n');
        }
        fs::write(path, body).unwrap();
    }

    /// 导出 -> 导入闭环：建一个完整会话目录，导出为 bundle，再导入到新目录，
    /// 验证 session id 替换、原字段保留、windows/segments/annotations 均还原。
    #[test]
    fn export_import_roundtrip() {
        let root = tempdir().unwrap();
        let src = root.path().join("session-001");
        fs::create_dir_all(src.join("segments")).unwrap();
        fs::write(
            src.join("session.json"),
            json!({"id":"session-001","source":"self","startedAt":1000,"endedAt":2000})
                .to_string(),
        )
        .unwrap();
        write_jsonl(
            &src.join("windows.jsonl"),
            &[
                json!({"type":"shown","label":"main","segmentId":"main#0","t":1000}),
                json!({"type":"hidden","label":"main","segmentId":"main#0","t":2000}),
            ],
        );
        write_jsonl(
            &src.join("segments/main#0.jsonl"),
            &[
                json!({"type":2,"timestamp":1000}),
                json!({"type":6,"timestamp":1100,"data":{"plugin":"console"}}),
            ],
        );
        crate::annotations::write_annotations(
            &src,
            &[json!({"id":"a1","t":500,"text":"note here","author":"local","createdAt":1200})],
        )
        .unwrap();

        // 导出
        let bundle = build_export_bundle(&src).unwrap();
        assert_eq!(bundle["format"], "prism-session");
        assert_eq!(bundle["session"]["id"], "session-001");
        assert_eq!(bundle["session"]["source"], "self");
        assert_eq!(bundle["windows"].as_array().unwrap().len(), 2);
        assert_eq!(bundle["segments"]["main#0"].as_array().unwrap().len(), 2);
        assert_eq!(bundle["annotations"].as_array().unwrap().len(), 1);
        assert_eq!(bundle["annotations"][0]["text"], "note here");

        // 导入到新目录
        let dst = root.path().join("session-002");
        write_import_bundle(&dst, &bundle, "session-002").unwrap();

        // session id 替换、原字段保留、importedAt 标记
        let session: Value =
            serde_json::from_str(&fs::read_to_string(dst.join("session.json")).unwrap()).unwrap();
        assert_eq!(session["id"], "session-002");
        assert_eq!(session["source"], "self");
        assert_eq!(session["startedAt"], 1000);
        assert_eq!(session["endedAt"], 2000);
        assert!(session["importedAt"].as_i64().unwrap() > 0);

        // windows / segments / annotations 还原
        assert_eq!(
            fs::read_to_string(dst.join("windows.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        assert_eq!(
            fs::read_to_string(dst.join("segments/main#0.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        let annos = read_annotations(&dst);
        assert_eq!(annos.len(), 1);
        assert_eq!(annos[0]["text"], "note here");
        assert_eq!(annos[0]["author"], "local");
    }

    /// 元信息合并：覆盖已有字段、新增字段、空串删除字段。
    #[test]
    fn merge_session_meta_overrides_adds_and_clears() {
        let root = tempdir().unwrap();
        let path = root.path().join("session.json");
        fs::write(
            &path,
            json!({"id":"s1","startedAt":100,"name":"old"}).to_string(),
        )
        .unwrap();

        // 覆盖 name + 新增 note/tags
        let v = merge_session_meta(
            &path,
            &json!({"name":"new name","note":"a note","tags":["bug","login"]}),
        )
        .unwrap();
        assert_eq!(v["name"], "new name");
        assert_eq!(v["note"], "a note");
        assert_eq!(v["tags"].as_array().unwrap().len(), 2);
        assert_eq!(v["startedAt"], 100); // 原字段不受影响

        // 空串 = 删除字段
        let v2 = merge_session_meta(&path, &json!({"name":""})).unwrap();
        assert!(v2.get("name").is_none());
        assert!(v2.get("note").is_some()); // 其他字段保留
    }

    /// segmentId 校验：合法的通过，含路径分隔符/`..` 的拒绝（路径穿越防护）。
    #[test]
    fn segment_id_validation() {
        assert!(validate_segment_id("main#0"));
        assert!(validate_segment_id("web#1"));
        assert!(validate_segment_id("settings#12"));
        assert!(validate_segment_id("a-b_c#9"));
        // 非法：路径穿越企图
        assert!(!validate_segment_id("../etc#0"));
        assert!(!validate_segment_id("a/..#1"));
        assert!(!validate_segment_id("a\\b#1"));
        // 非法：其他形态
        assert!(!validate_segment_id("a#b")); // n 非数字
        assert!(!validate_segment_id("#1")); // label 空
        assert!(!validate_segment_id("a#")); // n 空
        assert!(!validate_segment_id("a")); // 无 #
    }

    /// session ID 校验：纯数字通过，含 `..`/`/`/字母 被拒（read API 路径穿越防护）。
    #[test]
    fn session_id_validation() {
        assert!(validate_session_id("1750000000000"));
        assert!(validate_session_id("0"));
        assert!(validate_session_id("123456789"));
        // 非法：路径穿越企图
        assert!(!validate_session_id("../1"));
        assert!(!validate_session_id("1/2"));
        assert!(!validate_session_id(".."));
        // 非法：非纯数字
        assert!(!validate_session_id("abc"));
        assert!(!validate_session_id("1a"));
        assert!(!validate_session_id(""));
        assert!(!validate_session_id("1-2"));
    }

    /// 含恶意 segment key 的 bundle 必须被拒绝，恶意文件不得落盘（路径穿越防护）。
    #[test]
    fn import_rejects_path_traversal() {
        let root = tempdir().unwrap();
        let dir = root.path().join("dst");
        let bundle = json!({
            "format": BUNDLE_FORMAT,
            "version": BUNDLE_VERSION,
            "session": {"id":"x","startedAt":1},
            "windows": [],
            "segments": { "../evil#0": [{"type":2,"timestamp":1}] },
            "annotations": [],
        });
        let err = write_import_bundle(&dir, &bundle, "dst").unwrap_err();
        assert!(err.contains("路径穿越"), "got: {err}");
        // 未写穿：traversal 目标文件不应存在（无校验时会落到 dst/evil#0.jsonl）
        assert!(
            !dir.join("evil#0.jsonl").exists(),
            "恶意 segment 不得落盘到段目录之外"
        );
    }

    /// parse_bundle：合法 bundle 通过，缺 format / 版本过高被拒。
    #[test]
    fn parse_bundle_validates() {
        let ok = json!({
            "format": BUNDLE_FORMAT, "version": 1,
            "session": {"id":"x","startedAt":1}, "windows": [], "segments": {}, "annotations": [],
        })
        .to_string();
        assert!(parse_bundle(&ok).is_ok());

        let no_format = json!({"version": 1, "session": {}}).to_string();
        assert!(parse_bundle(&no_format).is_err());

        let too_new = json!({"format": BUNDLE_FORMAT, "version": 99, "session": {}}).to_string();
        let err = parse_bundle(&too_new).unwrap_err();
        assert!(err.contains("版本"));
    }

    /// import_bundle_content：完整闭环，分配新 id 并原子落盘。
    #[test]
    fn import_bundle_content_roundtrip() {
        let root = tempdir().unwrap();
        let bundle = json!({
            "format": BUNDLE_FORMAT, "version": 1,
            "session": {"id":"orig","source":"web","startedAt":100},
            "windows": [json!({"type":"shown","label":"web","segmentId":"web#1","t":100})],
            "segments": {"web#1": [json!({"type":2,"timestamp":100})]},
            "annotations": [],
        })
        .to_string();
        let new_id = import_bundle_content(root.path(), &bundle).unwrap();
        let dir = root.path().join(&new_id);
        assert!(dir.join("session.json").exists());
        let session: Value =
            serde_json::from_str(&fs::read_to_string(dir.join("session.json")).unwrap()).unwrap();
        assert_eq!(session["id"], new_id);
        assert_eq!(session["source"], "web");
    }

    /// read_session 支持 gzip segment 文件（混合 plain + gz 目录）。
    #[test]
    fn read_session_with_gzip_segments() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            &dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        // 一个 plain segment + 一个 gzip segment
        crate::storage::append_events_file(
            &dir,
            "plain#0",
            &[json!({"type":2,"timestamp":1})],
        )
        .unwrap();
        crate::storage::append_events_file_with(
            &dir,
            "gz#0",
            &[json!({"type":2,"timestamp":2}), json!({"type":6,"timestamp":3})],
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();

        let data = read_session(&dir).unwrap();
        let segs = data["segments"].as_object().unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs["plain#0"].as_array().unwrap().len(), 1);
        assert_eq!(segs["gz#0"].as_array().unwrap().len(), 2);
    }

    /// P19 D1：段索引只读首行——宽高取自 Meta（type 4），bytes 为文件大小，不含事件正文。
    #[test]
    fn session_meta_index_reads_first_line() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            &dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        crate::storage::append_events_file(
            &dir,
            "main#0",
            &[
                json!({"data":{"width":1300,"height":800},"timestamp":1,"type":4}),
                json!({"type":2,"timestamp":2}),
            ],
        )
        .unwrap();

        let meta = read_session_meta(&dir).unwrap();
        assert_eq!(meta["session"]["id"], "s1");
        let idx = meta["segments"].as_array().unwrap();
        assert_eq!(idx.len(), 1);
        assert_eq!(idx[0]["segmentId"], "main#0");
        assert_eq!(idx[0]["width"], 1300);
        assert_eq!(idx[0]["height"], 800);
        assert_eq!(idx[0]["firstTs"], 1, "首行 Meta 的 timestamp");
        assert_eq!(idx[0]["lastTs"], 2, "尾行事件的 timestamp（时长兜底）");
        assert!(idx[0]["bytes"].as_u64().unwrap() > 0);
        assert!(idx[0].get("events").is_none(), "段索引不含事件正文");
    }

    /// 尾行读取：文件以换行结尾（写侧惯例），应取最后一个**非空**行。
    #[test]
    fn segment_last_line_handles_trailing_newline() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        crate::storage::append_events_file(
            &dir,
            "main#0",
            &[json!({"type":4,"timestamp":11}), json!({"type":2,"timestamp":22})],
        )
        .unwrap();
        let last = crate::storage::read_segment_last_line(
            &dir.join("segments").join("main#0.jsonl"),
        )
        .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&last).unwrap()["timestamp"], 22);
    }

    /// 段索引对 gz 段同样给出 firstTs/lastTs（gz 无法逆序解压，走全文解压路径）。
    #[test]
    fn session_meta_index_covers_gzip_segments() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            &dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        crate::storage::append_events_file_with(
            &dir,
            "gz#0",
            &[json!({"type":4,"timestamp":7}), json!({"type":2,"timestamp":9})],
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();

        let idx = read_session_meta(&dir).unwrap()["segments"].clone();
        assert_eq!(idx[0]["firstTs"], 7);
        assert_eq!(idx[0]["lastTs"], 9);
    }

    /// 首行非 Meta / 缺宽高时仍产出条目，宽高为 null（回放侧退化为不缩放）。
    #[test]
    fn session_meta_index_tolerates_missing_meta() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            &dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        crate::storage::append_events_file(&dir, "main#0", &[json!({"type":2,"timestamp":2})])
            .unwrap();

        let idx = read_session_meta(&dir).unwrap()["segments"].clone();
        assert_eq!(idx[0]["width"], Value::Null);
        assert_eq!(idx[0]["height"], Value::Null);
    }

    /// P19 D2：直拼的段 JSON 与源 jsonl 语义一致（plain 与 gz 两路都走）。
    #[test]
    fn read_segment_json_roundtrips_plain_and_gzip() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        let rows = [
            json!({"data":{"width":800,"height":600},"timestamp":1,"type":4}),
            json!({"type":3,"timestamp":2}),
        ];
        crate::storage::append_events_file(&dir, "main#0", &rows).unwrap();
        crate::storage::append_events_file_with(
            &dir,
            "gz#0",
            &rows,
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();

        for seg in ["main#0", "gz#0"] {
            let txt = read_segment_json(&dir, seg).unwrap();
            let arr: Vec<Value> = serde_json::from_str(&txt).unwrap();
            assert_eq!(arr.len(), 2, "{seg} 直拼结果应可被 JSON 解析且条数一致");
            assert_eq!(arr[0]["data"]["width"], 800);
        }
    }

    /// 直拼路径的入参闸门：segmentId 校验（路径穿越）+ 段不存在。
    #[test]
    fn read_segment_json_rejects_bad_id_and_missing() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        assert!(read_segment_json(&dir, "../etc/passwd").is_err());
        assert!(read_segment_json(&dir, "nope#9").is_err());
    }

    /// P19 回归修复：全库信号只回 type:6 行（含 plain 与 gz 两路），DOM 事件不入选。
    #[test]
    fn session_signals_returns_only_plugin_events() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        crate::storage::append_events_file(
            &dir,
            "main#0",
            &[
                json!({"data":{"width":800,"height":600},"timestamp":1,"type":4}),
                json!({"data":{"plugin":"console","payload":{"level":"log","args":["hi"]}},
                        "timestamp":2,"type":6}),
                json!({"data":{"source":0,"adds":[]},"timestamp":3,"type":3}),
            ],
        )
        .unwrap();
        crate::storage::append_events_file_with(
            &dir,
            "gz#0",
            &[
                json!({"data":{"plugin":"error","payload":{"message":"boom","kind":"error"}},
                        "timestamp":4,"type":6}),
                json!({"data":{"source":1,"positions":[]},"timestamp":5,"type":3}),
            ],
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();

        let txt = read_session_signals_json(&dir).unwrap();
        let arr: Vec<Value> = serde_json::from_str(&txt).unwrap();
        assert_eq!(arr.len(), 2, "两段各一条 type:6");
        assert!(arr.iter().all(|e| e["type"] == 6));
        let plugins: Vec<&str> = arr
            .iter()
            .map(|e| e["data"]["plugin"].as_str().unwrap())
            .collect();
        assert!(plugins.contains(&"console") && plugins.contains(&"error"));
    }

    /// 配平校验不得误伤：字符串值里出现未配对的括号是合法 JSON。
    #[test]
    fn json_balanced_tolerates_braces_in_strings() {
        assert!(json_balanced(r#"{"a":"{ not a brace","b":1}"#));
        assert!(json_balanced(r#"{"a":"}","b":[1,2]}"#));
        assert!(json_balanced(r#"{"a":"esc \" quote {","b":1}"#));
        assert!(!json_balanced(r#"{"a":{"b":1}"#), "缺右括号");
        assert!(!json_balanced(r#"{"a":"unterminated}"#), "字符串未闭合");
        assert!(!json_balanced(r#"{"a":1}}"#), "多余右括号");
    }

    /// 截断点恰好落在 `}` 之后（能通过首尾检查但括号不配平）同样被排除——
    /// 否则整份直拼结果不可解析（独立复核实测到的损坏模式）。
    #[test]
    fn read_segment_json_skips_truncation_ending_with_brace() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("segments").join("main#0.jsonl"),
            "{\"type\":4,\"timestamp\":1}\n{\"type\":6,\"timestamp\":2,\"data\":{\"plugin\":\"console\"}\n",
        )
        .unwrap();
        let txt = read_segment_json(&dir, "main#0").unwrap();
        let arr: Vec<Value> = serde_json::from_str(&txt).unwrap();
        assert_eq!(arr.len(), 1, "括号不配平的残行被排除，其余仍可解析");
    }

    /// windows.jsonl 单行损坏只跳过该行——旧写法（map + collect::<Option<Vec<_>>>）会让
    /// 整个数组变空，而首屏的 label/hiddenAt 全靠它。
    #[test]
    fn session_windows_tolerates_bad_line() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            &dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        fs::write(
            dir.join("windows.jsonl"),
            "{not json}\n{\"type\":\"shown\",\"label\":\"main\",\"segmentId\":\"main#0\",\"t\":1}\n",
        )
        .unwrap();

        for data in [read_session_meta(&dir).unwrap(), read_session(&dir).unwrap()] {
            assert_eq!(data["windows"].as_array().unwrap().len(), 1);
        }
    }

    /// 截断残行（进程中途退出的典型损坏）被跳过，不产出非法 JSON。
    #[test]
    fn read_segment_json_skips_truncated_line() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("segments").join("main#0.jsonl"),
            "{\"type\":4,\"timestamp\":1}\n{\"type\":2,\"timesta",
        )
        .unwrap();

        let txt = read_segment_json(&dir, "main#0").unwrap();
        let arr: Vec<Value> = serde_json::from_str(&txt).unwrap();
        assert_eq!(arr.len(), 1, "残行被跳过，其余行可解析");
    }

    // ---- P25：write_export_bundle_streaming（流式导出） ----

    /// 流式导出收集器：返回（输出文本, 返回字节数）。
    fn streaming_to_string(dir: &Path) -> (String, u64) {
        let mut buf = Vec::new();
        let n = write_export_bundle_streaming(dir, &mut buf).unwrap();
        (String::from_utf8(buf).unwrap(), n)
    }

    /// 流式导出与 build_export_bundle 解压后 Value 级 canonical 相等（含混合明文+gz 段、
    /// 空行/残行/坏 JSON 行剔除行为一致、CRLF 残行、windows/annotations 还原）；
    /// 返回值 = 实际写出字节数（D7）。
    #[test]
    fn streaming_export_canonical_equivalence() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({"id":"s1","source":"tauri","startedAt":100,"endedAt":200}).to_string(),
        )
        .unwrap();
        fs::write(
            dir.join("windows.jsonl"),
            "{not json}\n{\"type\":\"shown\",\"label\":\"main\",\"segmentId\":\"main#0\",\"t\":1}\n",
        )
        .unwrap();
        // 明文段：合法行 + 空行 + 残行 + 坏 JSON 行 + CRLF 行
        fs::write(
            dir.join("segments").join("main#0.jsonl"),
            "{\"type\":4,\"timestamp\":1}\n\n{\"type\":2,\"timesta\nnot json\n{\"type\":3,\"timestamp\":3}\r\n",
        )
        .unwrap();
        // gz 段：两次追加 = 两个 member
        crate::storage::append_events_file_with(
            &dir,
            "gz#0",
            &[json!({"type":2,"timestamp":10}), json!({"type":6,"timestamp":11})],
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();
        crate::storage::append_events_file_with(
            &dir,
            "gz#0",
            &[json!({"type":3,"timestamp":12})],
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();
        crate::annotations::write_annotations(
            &dir,
            &[json!({"id":"a1","t":5,"text":"note","author":"local","createdAt":6})],
        )
        .unwrap();

        let (out, n) = streaming_to_string(&dir);
        let streamed: Value = serde_json::from_str(&out).expect("流式输出必为合法 JSON");
        let mut expect = build_export_bundle(&dir).unwrap();
        expect["exportedAt"] = streamed["exportedAt"].clone(); // 对齐时间戳后比 Value
        assert_eq!(
            streamed, expect,
            "流式导出与全量组装解压后 Value 级相等（剔除行为一致）"
        );
        // D7：返回值 = 输出字节数
        assert_eq!(n, out.len() as u64);
        // 剔除行为细化断言：明文段合法行 2 条（残行/空行/坏行剔除、CRLF 保留语义）
        assert_eq!(streamed["segments"]["main#0"].as_array().unwrap().len(), 2);
        assert_eq!(streamed["segments"]["gz#0"].as_array().unwrap().len(), 3);
        assert_eq!(streamed["windows"].as_array().unwrap().len(), 1);
        assert_eq!(streamed["annotations"].as_array().unwrap().len(), 1);
    }

    /// segments 输出按 segmentId 字典序（D4：对齐 build_export_bundle 的 BTreeMap 键序，
    /// 与目录枚举序无关——创建顺序故意乱序）。
    #[test]
    fn streaming_export_segments_sorted_by_id() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        for id in ["z#0", "a#0", "m#2", "b#0"] {
            fs::write(dir.join("segments").join(format!("{id}.jsonl")), "{\"type\":2}\n").unwrap();
        }
        let (out, _) = streaming_to_string(&dir);
        let (ia, ib, im, iz) = (
            out.find("\"a#0\"").unwrap(),
            out.find("\"b#0\"").unwrap(),
            out.find("\"m#2\"").unwrap(),
            out.find("\"z#0\"").unwrap(),
        );
        assert!(ia < ib && ib < im && im < iz, "segments 键须按字典序输出");
        // 与 build_export_bundle 的 segments 键序一致
        let build = build_export_bundle(&dir).unwrap();
        let build_keys: Vec<&String> = build["segments"].as_object().unwrap().keys().collect();
        let streamed_keys: Vec<String> = ["a#0", "b#0", "m#2", "z#0"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(build_keys, streamed_keys.iter().collect::<Vec<_>>());
    }

    /// gz 段截断（流级失败）：导出不失败，段键保留，保留已解出的行（D6 预期改进——
    /// build_export_bundle 整段为空，流式保留损坏点之前的行）。
    #[test]
    fn streaming_export_tolerates_truncated_gzip() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        let events: Vec<Value> = (0..50)
            .map(|i| json!({"type":2,"timestamp":i}))
            .collect();
        crate::storage::append_events_file_with(&dir, "trunc#0", &events, &crate::storage::WriteOpts::gzip())
            .unwrap();
        let gz_path = dir.join("segments").join("trunc#0.jsonl.gz");
        let full = fs::read(&gz_path).unwrap();
        fs::write(&gz_path, &full[..full.len() / 2]).unwrap(); // 截断一半

        let (out, _) = streaming_to_string(&dir);
        let v: Value = serde_json::from_str(&out).expect("截断 gz 段不得炸导出");
        let arr = v["segments"]["trunc#0"].as_array().expect("段键必保留");
        assert!(
            arr.len() > 0 && arr.len() < 50,
            "截断 gz：损坏点前已解出的行必保留（D6 预期改进，非空非全量）"
        );
        // 对照：build_export_bundle 对同一文件整段剔为空（D6 注明的差异，属预期）
        let build = build_export_bundle(&dir).unwrap();
        assert_eq!(build["segments"]["trunc#0"].as_array().unwrap().len(), 0);
    }

    /// 明文段含非 UTF-8 字节（行级失败）：剔该行继续，前后合法行保留（build_export_bundle
    /// 对整文件 read_to_string 失败 → 整段空——流式为预期改进）。
    #[test]
    fn streaming_export_skips_invalid_utf8_line() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        fs::write(
            dir.join("segments").join("bad#0.jsonl"),
            b"{\"type\":2,\"timestamp\":1}\n{\"type\":3,\xFF\xFE}\n{\"type\":6,\"timestamp\":3}\n",
        )
        .unwrap();

        let (out, _) = streaming_to_string(&dir);
        let v: Value = serde_json::from_str(&out).expect("非 UTF-8 行不得炸导出");
        let arr = v["segments"]["bad#0"].as_array().unwrap();
        assert_eq!(arr.len(), 2, "坏行剔除，前后合法行保留");
        assert_eq!(arr[0]["timestamp"], 1);
        assert_eq!(arr[1]["timestamp"], 3);
        // 对照：build 路径整段为空
        let build = build_export_bundle(&dir).unwrap();
        assert_eq!(build["segments"]["bad#0"].as_array().unwrap().len(), 0);
    }

    /// 缺 windows.jsonl / annotations.jsonl / segments 目录：默认空语义与 build_export_bundle
    /// 一致，输出仍为合法 bundle。
    #[test]
    fn streaming_export_missing_optional_files() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();

        let (out, _) = streaming_to_string(&dir);
        let streamed: Value = serde_json::from_str(&out).unwrap();
        let mut expect = build_export_bundle(&dir).unwrap();
        expect["exportedAt"] = streamed["exportedAt"].clone();
        assert_eq!(streamed, expect, "缺省文件默认空语义一致");
        assert_eq!(streamed["windows"].as_array().unwrap().len(), 0);
        assert_eq!(streamed["annotations"].as_array().unwrap().len(), 0);
        assert_eq!(streamed["segments"].as_object().unwrap().len(), 0);
    }

    /// segments/ 混入打不开的条目（子目录等）：open 失败降级为空段键——导出不因坏段失败
    /// （D6 + /regress 建议 1 收敛：旧实现此处 Err 冒泡会炸整个导出）。
    #[test]
    fn streaming_export_tolerates_unopenable_segment() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments").join("weird#0")).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        crate::storage::append_events_file(&dir, "ok#0", &[json!({"type":2,"timestamp":1})])
            .unwrap();

        let (out, _) = streaming_to_string(&dir);
        let v: Value = serde_json::from_str(&out).expect("打不开的段不得炸导出");
        assert_eq!(v["segments"]["ok#0"].as_array().unwrap().len(), 1);
        assert_eq!(
            v["segments"]["weird#0"].as_array().unwrap().len(),
            0,
            "打不开的条目降级为空段键"
        );
    }

    /// 消费链形态钉死（P25 论证裁决 d6-修订3）：截断 gz 会话 → 流式导出 → parse →
    /// import → read_session_meta 读回。断言：段条目存在、firstTs == 段首 Meta、
    /// 事件数非空非全量（前缀）。回放形态预期（usePlayer）：该段 hiddenAt 前恒 active，
    /// 播完前缀后提前定格——与 build 路径「整段缺席」是两种形态，site 手册已注明。
    #[test]
    fn streaming_truncated_gzip_survives_import_chain() {
        let root = tempdir().unwrap();
        let dir = root.path().join("s1");
        fs::create_dir_all(dir.join("segments")).unwrap();
        fs::write(
            dir.join("session.json"),
            json!({"id":"s1","startedAt":1}).to_string(),
        )
        .unwrap();
        let events: Vec<Value> = std::iter::once(
            json!({"data":{"width":800,"height":600},"timestamp":1,"type":4}),
        )
        .chain((1..50).map(|i| json!({"type":2,"timestamp":i})))
        .collect();
        crate::storage::append_events_file_with(
            &dir,
            "trunc#0",
            &events,
            &crate::storage::WriteOpts::gzip(),
        )
        .unwrap();
        let gz = dir.join("segments").join("trunc#0.jsonl.gz");
        let full = fs::read(&gz).unwrap();
        fs::write(&gz, &full[..full.len() / 2]).unwrap();

        // 流式导出 → parse → import → 读回
        let mut buf = Vec::new();
        write_export_bundle_streaming(&dir, &mut buf).unwrap();
        let bundle: Value = serde_json::from_slice(&buf).unwrap();
        let dst = tempdir().unwrap();
        let new_id = import_bundle(dst.path(), &bundle).unwrap();
        let meta = read_session_meta(&dst.path().join(&new_id)).unwrap();
        let idx = meta["segments"].as_array().unwrap();
        assert_eq!(idx.len(), 1, "段条目必存在");
        assert_eq!(idx[0]["segmentId"], "trunc#0");
        assert_eq!(idx[0]["firstTs"], 1, "段首 Meta 行经前缀保留仍可读");
        let last = idx[0]["lastTs"].as_i64().expect("lastTs 非空（前缀内）");
        assert!(
            last > 0 && last < 49,
            "lastTs 落在前缀内（非空非全量），got {last}"
        );
    }
}
