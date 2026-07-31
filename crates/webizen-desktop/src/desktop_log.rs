use serde::Serialize;
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::{Mutex, OnceLock};

const MAX_RECENT_LINES: usize = 500;
const LOG_CHANNEL_CAPACITY: usize = 2_048;
const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct DesktopLogEntry {
    pub ts: String,
    pub level: String,
    pub message: String,
    pub session_id: String,
    pub thread: String,
}

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static RECENT: OnceLock<Mutex<VecDeque<DesktopLogEntry>>> = OnceLock::new();
static LOG_SENDER: OnceLock<SyncSender<DesktopLogEntry>> = OnceLock::new();
static SESSION_ID: OnceLock<String> = OnceLock::new();
static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);
static LOGGER_INSTALLED: OnceLock<bool> = OnceLock::new();

struct DesktopFacadeLogger;

static DESKTOP_FACADE_LOGGER: DesktopFacadeLogger = DesktopFacadeLogger;

impl log::Log for DesktopFacadeLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level()
            <= if DEBUG_ENABLED.load(Ordering::Relaxed) {
                log::Level::Debug
            } else {
                log::Level::Info
            }
    }

    fn log(&self, entry: &log::Record<'_>) {
        if !self.enabled(entry.metadata()) {
            return;
        }
        record(
            entry.level().as_str().to_ascii_lowercase(),
            format!("{}: {}", entry.target(), entry.args()),
        );
    }

    fn flush(&self) {}
}

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

fn session_id() -> &'static str {
    SESSION_ID
        .get_or_init(|| uuid::Uuid::new_v4().to_string())
        .as_str()
}

fn ensure_writer() -> &'static SyncSender<DesktopLogEntry> {
    LOG_SENDER.get_or_init(|| {
        let path = log_path();
        let (tx, rx) = sync_channel::<DesktopLogEntry>(LOG_CHANNEL_CAPACITY);
        let spawn_result = std::thread::Builder::new()
            .name("webizen-log-writer".to_string())
            .spawn(move || {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut file = open_log_file(&path);
                while let Ok(entry) = rx.recv() {
                    if should_rotate(&path) {
                        file.take();
                        rotate_log(&path);
                        file = open_log_file(&path);
                    }
                    if file.is_none() {
                        file = open_log_file(&path);
                    }
                    if let Some(writer) = file.as_mut() {
                        let _ = writeln!(
                            writer,
                            "{} [{}] [{}] [{}] {}",
                            entry.ts, entry.level, entry.session_id, entry.thread, entry.message
                        );
                    }
                }
            });
        if let Err(error) = spawn_result {
            eprintln!("Webizen could not start the log writer: {error}");
        }
        tx
    })
}

fn open_log_file(path: &Path) -> Option<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
}

fn should_rotate(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.len() >= MAX_LOG_BYTES)
        .unwrap_or(false)
}

fn rotate_log(path: &Path) {
    let rotated = path.with_extension("log.1");
    let _ = std::fs::remove_file(&rotated);
    let _ = std::fs::rename(path, rotated);
}

pub fn init() -> PathBuf {
    let path = log_path();
    let debug = std::env::var("WEBIZEN_DEBUG")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or_else(|_| {
            std::fs::read_to_string(debug_mode_path())
                .map(|value| value.trim() == "true")
                .unwrap_or(false)
        });
    DEBUG_ENABLED.store(debug, Ordering::Relaxed);
    let installed =
        *LOGGER_INSTALLED.get_or_init(|| log::set_logger(&DESKTOP_FACADE_LOGGER).is_ok());
    if installed {
        apply_log_level(debug);
    }
    let _ = RECENT.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECENT_LINES)));
    let _ = session_id();
    let _ = ensure_writer();
    record(
        "info",
        format!("Webizen desktop logging to {}", path.display()),
    );
    path
}

fn debug_mode_path() -> PathBuf {
    log_path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("debug-mode")
}

fn apply_log_level(enabled: bool) {
    log::set_max_level(if enabled {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    });
}

pub fn debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

/// Enable verbose native logging across the `log` facade. The preference persists beside the logs
/// and takes effect immediately; no restart is required.
pub fn set_debug_enabled(enabled: bool) -> Result<(), String> {
    let path = debug_mode_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, if enabled { "true" } else { "false" })
        .map_err(|error| error.to_string())?;
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);
    apply_log_level(enabled);
    record(
        "info",
        if enabled {
            "Debug mode enabled; verbose native events will be recorded"
        } else {
            "Debug mode disabled; native logging returned to information level"
        },
    );
    Ok(())
}

pub fn install_panic_hook() {
    let _ = PANIC_HOOK_INSTALLED.get_or_init(|| {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let current_thread = std::thread::current();
            let thread_name = current_thread.name().unwrap_or("unnamed");
            let payload = panic_info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".to_string());
            let location = panic_info
                .location()
                .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
                .unwrap_or_else(|| "unknown location".to_string());
            let summary = format!("panic on thread '{thread_name}' at {location}: {payload}");

            record("panic", &summary);
            write_crash_marker(&summary);
            eprintln!("Webizen desktop {summary}");
            previous_hook(panic_info);
        }));
    });
}

fn write_crash_marker(summary: &str) {
    let path = log_path().with_extension("crash");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(
            file,
            "{} [{}] {}",
            chrono::Utc::now().to_rfc3339(),
            session_id(),
            summary
        );
        let _ = file.flush();
    }
}

pub fn log_path() -> PathBuf {
    LOG_PATH.get_or_init(default_log_path).clone()
}

pub fn record(level: impl Into<String>, message: impl Into<String>) {
    let current_thread = std::thread::current();
    let thread = current_thread
        .name()
        .map(str::to_string)
        .unwrap_or_else(|| format!("{:?}", current_thread.id()));
    let entry = DesktopLogEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        level: level.into(),
        message: message.into(),
        session_id: session_id().to_string(),
        thread,
    };

    let recent = RECENT.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RECENT_LINES)));
    if let Ok(mut guard) = recent.lock() {
        if guard.len() >= MAX_RECENT_LINES {
            guard.pop_front();
        }
        guard.push_back(entry.clone());
    }

    match ensure_writer().try_send(entry) {
        Ok(()) => {}
        Err(TrySendError::Full(entry)) => eprintln!(
            "Webizen log queue full; dropped [{}] {}",
            entry.level, entry.message
        ),
        Err(TrySendError::Disconnected(entry)) => eprintln!(
            "Webizen log writer stopped; dropped [{}] {}",
            entry.level, entry.message
        ),
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
        lines.push(format!(
            "{} [{}] [{}] [{}] {}",
            entry.ts, entry.level, entry.session_id, entry.thread, entry.message
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_entries_include_session_and_thread_context() {
        let _ = init();
        record("info", "logger context test");
        let entry = recent_entries()
            .into_iter()
            .rev()
            .find(|entry| entry.message == "logger context test")
            .expect("test log entry");
        assert!(!entry.session_id.is_empty());
        assert!(!entry.thread.is_empty());
    }
}
