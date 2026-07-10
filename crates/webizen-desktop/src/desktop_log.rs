use serde::Serialize;
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const MAX_RECENT_LINES: usize = 500;

#[derive(Clone, Debug, Serialize)]
pub struct DesktopLogEntry {
    pub ts: String,
    pub level: String,
    pub message: String,
}

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static RECENT: OnceLock<Mutex<VecDeque<DesktopLogEntry>>> = OnceLock::new();

fn default_log_path() -> PathBuf {
    if let Ok(path) = std::env::var("WEBIZEN_DESKTOP_LOG") {
        return PathBuf::from(path);
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join("Webizen")
            .join("logs")
            .join("desktop.log");
    }
    std::env::temp_dir().join("webizen-desktop.log")
}

pub fn init() -> PathBuf {
    let path = LOG_PATH.get_or_init(default_log_path).clone();
    let _ = RECENT.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECENT_LINES)));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    record(
        "info",
        format!("Webizen desktop logging to {}", path.display()),
    );
    path
}

pub fn log_path() -> PathBuf {
    LOG_PATH.get_or_init(default_log_path).clone()
}

pub fn record(level: impl Into<String>, message: impl Into<String>) {
    let entry = DesktopLogEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        level: level.into(),
        message: message.into(),
    };

    let recent = RECENT.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECENT_LINES)));
    if let Ok(mut guard) = recent.lock() {
        if guard.len() >= MAX_RECENT_LINES {
            guard.pop_front();
        }
        guard.push_back(entry.clone());
    }

    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{} [{}] {}", entry.ts, entry.level, entry.message);
    }
}

pub fn recent_entries() -> Vec<DesktopLogEntry> {
    RECENT
        .get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECENT_LINES)))
        .lock()
        .map(|guard| guard.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn recent_text() -> String {
    let mut lines = Vec::new();
    let path = log_path();
    lines.push(format!("log_file={}", path.display()));
    for entry in recent_entries() {
        lines.push(format!("{} [{}] {}", entry.ts, entry.level, entry.message));
    }
    lines.join("\n")
}
