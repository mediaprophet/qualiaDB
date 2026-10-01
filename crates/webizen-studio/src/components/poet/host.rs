//! Webview host bridge — clipboard, localStorage, prompt.
//!
//! wasm32: real DOM calls. Native build: held / not yet (no-op fallbacks —
//! the Studio crate compiles natively for `webizen-desktop` but the
//! HyperCanvas only runs inside the webview).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::store::Workbench;
use dioxus::prelude::*;

/// Copy text to the OS clipboard. Native: held / not yet.
pub fn copy_to_clipboard(text: String) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(win) = web_sys::window() {
                let _ = wasm_bindgen_futures::JsFuture::from(
                    win.navigator().clipboard().write_text(&text),
                )
                .await;
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
    }
}

/// Prompt the operator for a text value (`window.prompt`).
/// Returns None on native or when cancelled/empty.
pub fn prompt_text(message: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.prompt_with_message(message).ok().flatten())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
        None
    }
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

/// Write a localStorage key. Returns false when storage is unavailable.
pub fn storage_set(key: &str, value: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        storage()
            .map(|s| s.set_item(key, value).is_ok())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, value);
        false
    }
}

/// Read a localStorage key. None when storage is unavailable or unset.
pub fn storage_get(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        storage().and_then(|s| s.get_item(key).ok().flatten())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

fn checkpoint_key(w: &Workbench) -> String {
    format!("poet:desk:{}", w.active.id())
}

/// Persist the desk to localStorage (`poet:desk:{manifold}`).
pub fn save_checkpoint(mut wb: Signal<Workbench>) {
    let mut s = wb();
    if storage_set(&checkpoint_key(&s), &s.to_hcf_json()) {
        s.note("Desk checkpoint saved (localStorage)");
    } else {
        s.note("Checkpoint save held — localStorage unavailable");
    }
    s.menu = None;
    wb.set(s);
}

/// Restore the desk from its localStorage checkpoint.
pub fn load_checkpoint(mut wb: Signal<Workbench>) {
    let mut s = wb();
    match storage_get(&checkpoint_key(&s)) {
        Some(json) if s.restore_hcf_json(&json) => s.note("Desk checkpoint loaded"),
        Some(_) => s.note("Checkpoint payload not recognised"),
        None => s.note("No checkpoint saved for this manifold"),
    }
    s.menu = None;
    wb.set(s);
}

/// Open a `.hcf` desk document from the clipboard (replaces the desk).
pub fn open_hcf_from_clipboard(mut wb: Signal<Workbench>) {
    let mut s = wb();
    s.menu = None;
    wb.set(s);
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        let text = match web_sys::window() {
            Some(win) => {
                wasm_bindgen_futures::JsFuture::from(win.navigator().clipboard().read_text())
                    .await
                    .ok()
                    .and_then(|js| js.as_string())
            }
            None => None,
        };
        let mut s = wb();
        match text {
            Some(t) if s.restore_hcf_json(&t) => s.note("Opened .hcf desk from clipboard"),
            Some(_) => s.note("Clipboard isn't a webizen.hcf/1 document"),
            None => s.note("Clipboard read held — permission denied or empty"),
        }
        wb.set(s);
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut s = wb();
        s.note(".hcf clipboard open held — native build");
        wb.set(s);
    }
}
