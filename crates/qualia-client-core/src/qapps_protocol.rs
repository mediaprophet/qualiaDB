//! Loopback HTTP server mirroring Tauri's `qualia://localhost/` custom protocol.
//!
//! Serves installed qapp assets from `{storage_path}/Qapps/`
//! so embedded WebViews can load sandboxed HTML without `file://` CORS restrictions.

use crate::qapp_paths::{ensure_qapps_dir, qapps_dir};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;

static QAPPS_SERVER_PORT: AtomicU16 = AtomicU16::new(0);

fn find_open_port(host: &str, start: u16) -> u16 {
    for port in start..=4600 {
        if std::net::TcpListener::bind((host, port)).is_ok() {
            return port;
        }
    }
    start
}

fn safe_path(base: &Path, request_path: &str) -> Option<PathBuf> {
    let trimmed = request_path.trim_start_matches('/');
    let mut out = base.to_path_buf();
    for part in Path::new(trimmed).components() {
        match part {
            Component::Normal(name) => out.push(name),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.starts_with(base) {
        Some(out)
    } else {
        None
    }
}

fn guess_mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html",
        "js" => "application/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "wasm" => "application/wasm",
        "woff" | "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn qapps_root() -> Result<PathBuf, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    Ok(qapps_dir(&data_dir))
}

/// Start the qualia asset server on 127.0.0.1 (idempotent).
pub fn start_qualia_protocol() -> Result<u16, String> {
    let existing = QAPPS_SERVER_PORT.load(Ordering::SeqCst);
    if existing != 0 {
        return Ok(existing);
    }

    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let root = ensure_qapps_dir(&data_dir).map_err(|e| e.to_string())?;
    let port = find_open_port("127.0.0.1", 4567);
    let serve_root = root.clone();

    let server = tiny_http::Server::http(format!("127.0.0.1:{port}"))
        .map_err(|e| format!("Bind qualia protocol: {e}"))?;
    QAPPS_SERVER_PORT.store(port, Ordering::SeqCst);
    eprintln!("Qualia qapps protocol listening on 127.0.0.1:{port}");

    thread::spawn(move || {
        for request in server.incoming_requests() {
            let url_path = request.url().to_string();
            let path_only = url_path.split('?').next().unwrap_or("/");
            let mut file_path = match safe_path(&serve_root, path_only) {
                Some(p) => p,
                None => {
                    let _ = request.respond(tiny_http::Response::empty(403));
                    continue;
                }
            };
            if file_path.is_dir() {
                file_path.push("index.html");
            }
            match std::fs::read(&file_path) {
                Ok(data) => {
                    let mime = guess_mime(&file_path);
                    let _ = request.respond(
                        tiny_http::Response::from_data(data)
                            .with_status_code(200)
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    mime.as_bytes(),
                                )
                                .unwrap(),
                            ),
                    );
                }
                Err(_) => {
                    let _ = request.respond(tiny_http::Response::empty(404));
                }
            }
        }
    });

    Ok(port)
}

pub fn qualia_protocol_port() -> u16 {
    QAPPS_SERVER_PORT.load(Ordering::SeqCst)
}

/// `http://127.0.0.1:{port}/{qapp}/index.html` — WebView-safe launch URL.
pub fn qualia_qapp_asset_url(qapp_name: &str, asset_path: &str) -> Result<String, String> {
    let port = qualia_protocol_port();
    if port == 0 {
        return Err("Qualia protocol server not started".into());
    }
    let root = qapps_root()?;
    let qapp_dir = root.join(qapp_name);
    if !qapp_dir.exists() {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }
    let trimmed = asset_path.trim_start_matches('/');
    let resolved = safe_path(&qapp_dir, trimmed)
        .ok_or_else(|| format!("Invalid qapp asset path: {asset_path}"))?;
    if !resolved.exists() {
        return Err(format!("Qapp asset not found: {qapp_name}/{trimmed}"));
    }
    let launch_path = if trimmed.is_empty() {
        "index.html"
    } else {
        trimmed
    };
    Ok(format!("http://127.0.0.1:{port}/{qapp_name}/{launch_path}"))
}

pub fn qualia_qapp_launch_url(qapp_name: &str) -> Result<String, String> {
    qualia_qapp_asset_url(qapp_name, "index.html")
}

#[cfg(windows)]
pub fn register_qualia_uri_handler(exe_path: &str) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (classes, _) = hkcu
        .create_subkey("Software\\Classes\\qualia")
        .map_err(|e| e.to_string())?;
    classes
        .set_value("", &"URL:QualiaDB Protocol")
        .map_err(|e| e.to_string())?;
    classes
        .set_value("URL Protocol", &"")
        .map_err(|e| e.to_string())?;

    let (icon, _) = classes
        .create_subkey("DefaultIcon")
        .map_err(|e| e.to_string())?;
    icon.set_value("", &format!("{exe_path},0"))
        .map_err(|e| e.to_string())?;

    let (shell, _) = classes
        .create_subkey("shell\\open\\command")
        .map_err(|e| e.to_string())?;
    shell
        .set_value("", &format!("\"{exe_path}\" \"%1\""))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(windows))]
pub fn register_qualia_uri_handler(_exe_path: &str) -> Result<(), String> {
    Ok(())
}
