const COMMANDS: &[&str] = &[
    "start_session",
    "stop_session",
    "is_recording_active",
    "begin_segment",
    "append_events",
    "bind_session",
    "session_id",
    "notify_segment_start",
    "list_sessions",
    "export_session",
    "export_session_to_file",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
