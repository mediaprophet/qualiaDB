//! The installable-PWA artifact generator.
//!
//! [`generate_pwa`] turns a [`QappManifest`] into a [`PwaBundle`] — an in-memory set of files
//! (`manifest.webmanifest`, `sw.js`, `index.html`) forming a standards-compliant PWA scaffold
//! that wraps and loads the qapp's wasm bundle. It is pure domain code: it produces bytes/strings,
//! it does not write to disk and it does not serve anything.
//!
//! **Scope honesty:** the scaffold is correct and *installable in principle*, but a browser will
//! only offer to install a PWA when it is served from a **secure origin** (HTTPS, or `localhost`
//! for development) with the manifest and service worker reachable under scope. That secure-origin
//! delivery layer is a separate, later piece. This module also does not compile wasm — it wires up
//! a loader for the bundle referenced by [`QappManifest::entry_wasm`].

use serde_json::json;

use super::manifest::QappManifest;

/// A generated file's contents — text (UTF-8) or opaque bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PwaContent {
    Text(String),
    Bytes(Vec<u8>),
}

impl PwaContent {
    /// Borrow the text if this file is textual.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            PwaContent::Text(s) => Some(s.as_str()),
            PwaContent::Bytes(_) => None,
        }
    }
}

/// One file in a generated PWA bundle, addressed by its path relative to the served scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PwaFile {
    /// Path relative to the served scope, e.g. `"index.html"` or `"manifest.webmanifest"`.
    pub path: String,
    /// The file's contents.
    pub content: PwaContent,
}

/// A generated, installable PWA scaffold: an ordered collection of [`PwaFile`]s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PwaBundle {
    pub files: Vec<PwaFile>,
}

impl PwaBundle {
    /// Look up a file by exact path.
    pub fn get(&self, path: &str) -> Option<&PwaFile> {
        self.files.iter().find(|f| f.path == path)
    }

    /// Borrow the text of a file by path, if it exists and is textual.
    pub fn text_of(&self, path: &str) -> Option<&str> {
        self.get(path).and_then(|f| f.content.as_text())
    }
}

/// Escape a string for safe interpolation into an HTML text/attribute context (double-quoted).
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape a string for safe interpolation inside a single-quoted JavaScript string literal.
/// Also escapes `<` / `>` so a `</script>` sequence in data cannot terminate the inline script,
/// and escapes `\u{2028}` / `\u{2029}` which are line terminators in JS.
fn js_string_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\x3C"),
            '>' => out.push_str("\\x3E"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out
}

/// The versioned cache name a service worker uses: `"<id>-<version>"`. Stamping the version in
/// means a new qapp version installs a fresh cache and the old one is cleaned up on activate.
fn cache_name(manifest: &QappManifest) -> String {
    format!("{}-{}", manifest.id, manifest.version)
}

/// Build the W3C Web App Manifest JSON (`manifest.webmanifest`).
fn build_webmanifest(manifest: &QappManifest) -> String {
    let icons: Vec<serde_json::Value> = manifest
        .icons
        .iter()
        .map(|icon| {
            json!({
                "src": icon.src,
                "sizes": icon.sizes,
                "type": guess_icon_type(&icon.src),
                "purpose": icon.purpose,
            })
        })
        .collect();

    let value = json!({
        "id": manifest.id,
        "name": manifest.name,
        "short_name": manifest.short_name,
        "description": manifest.description,
        "start_url": ".",
        "scope": ".",
        "display": manifest.display,
        "theme_color": manifest.theme_color,
        "background_color": manifest.background_color,
        "icons": icons,
    });

    // Pretty output is fine — this file is fetched once and cached.
    serde_json::to_string_pretty(&value).expect("web app manifest is always serializable")
}

/// Best-effort MIME type for an icon `src` by extension. Defaults to `image/png`.
fn guess_icon_type(src: &str) -> &'static str {
    let lower = src.to_ascii_lowercase();
    if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else {
        "image/png"
    }
}

/// Build the cache-first service worker (`sw.js`).
fn build_service_worker(manifest: &QappManifest) -> String {
    let cache = js_string_escape(&cache_name(manifest));

    // Precache list: shell + the wasm bundle + every icon. Deduplicated, JS-escaped.
    let mut precache: Vec<String> = vec![
        "./".to_string(),
        "./index.html".to_string(),
        "./manifest.webmanifest".to_string(),
        manifest.entry_wasm.path.clone(),
    ];
    for icon in &manifest.icons {
        precache.push(icon.src.clone());
    }
    precache.dedup();

    let precache_js = precache
        .iter()
        .map(|p| format!("  '{}'", js_string_escape(p)))
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "// Auto-generated cache-first service worker for qapp '{cache}'.\n\
         // Offline capability: precache the shell + wasm on install, serve cache-first on fetch.\n\
         const CACHE = '{cache}';\n\
         const PRECACHE = [\n{precache_js}\n];\n\
         \n\
         self.addEventListener('install', (event) => {{\n\
         \x20 event.waitUntil(\n\
         \x20\x20\x20 caches.open(CACHE).then((cache) => cache.addAll(PRECACHE)).then(() => self.skipWaiting())\n\
         \x20 );\n\
         }});\n\
         \n\
         self.addEventListener('activate', (event) => {{\n\
         \x20 event.waitUntil(\n\
         \x20\x20\x20 caches.keys().then((keys) => Promise.all(\n\
         \x20\x20\x20\x20\x20 keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))\n\
         \x20\x20\x20 )).then(() => self.clients.claim())\n\
         \x20 );\n\
         }});\n\
         \n\
         self.addEventListener('fetch', (event) => {{\n\
         \x20 if (event.request.method !== 'GET') return;\n\
         \x20 event.respondWith(\n\
         \x20\x20\x20 caches.match(event.request).then((cached) => {{\n\
         \x20\x20\x20\x20\x20 if (cached) return cached;\n\
         \x20\x20\x20\x20\x20 return fetch(event.request).then((response) => {{\n\
         \x20\x20\x20\x20\x20\x20\x20 if (response && response.ok && response.type === 'basic') {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20 const copy = response.clone();\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20 caches.open(CACHE).then((cache) => cache.put(event.request, copy));\n\
         \x20\x20\x20\x20\x20\x20\x20 }}\n\
         \x20\x20\x20\x20\x20\x20\x20 return response;\n\
         \x20\x20\x20\x20\x20 }});\n\
         \x20\x20\x20 }})\n\
         \x20 );\n\
         }});\n",
        cache = cache,
        precache_js = precache_js,
    )
}

/// Build the minimal loader page (`index.html`).
fn build_index_html(manifest: &QappManifest) -> String {
    let title_html = html_escape(&manifest.name);
    let short_name_html = html_escape(&manifest.short_name);
    let theme_html = html_escape(&manifest.theme_color);
    let bg_html = html_escape(&manifest.background_color);
    let description_html = html_escape(&manifest.description);
    let wasm_path_js = js_string_escape(&manifest.entry_wasm.path);

    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         \x20 <meta charset=\"utf-8\">\n\
         \x20 <meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\">\n\
         \x20 <title>{title}</title>\n\
         \x20 <meta name=\"description\" content=\"{description}\">\n\
         \x20 <link rel=\"manifest\" href=\"manifest.webmanifest\">\n\
         \x20 <meta name=\"theme-color\" content=\"{theme}\">\n\
         \x20 <meta name=\"apple-mobile-web-app-capable\" content=\"yes\">\n\
         \x20 <meta name=\"apple-mobile-web-app-status-bar-style\" content=\"black-translucent\">\n\
         \x20 <meta name=\"apple-mobile-web-app-title\" content=\"{short_name}\">\n\
         \x20 <style>\n\
         \x20\x20\x20 html, body {{ margin: 0; height: 100%; background: {bg}; color: #e6edf3;\n\
         \x20\x20\x20\x20\x20 font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; }}\n\
         \x20\x20\x20 #app {{ display: flex; align-items: center; justify-content: center; height: 100%; }}\n\
         \x20 </style>\n\
         </head>\n\
         <body>\n\
         \x20 <main id=\"app\">Loading {title}…</main>\n\
         \x20 <script>\n\
         \x20\x20\x20 (function () {{\n\
         \x20\x20\x20\x20\x20 'use strict';\n\
         \x20\x20\x20\x20\x20 var WASM_PATH = '{wasm_path}';\n\
         \x20\x20\x20\x20\x20 if ('serviceWorker' in navigator) {{\n\
         \x20\x20\x20\x20\x20\x20\x20 window.addEventListener('load', function () {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20 navigator.serviceWorker.register('./sw.js').catch(function (err) {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20 console.warn('service worker registration failed:', err);\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20 }});\n\
         \x20\x20\x20\x20\x20\x20\x20 }});\n\
         \x20\x20\x20\x20\x20 }}\n\
         \x20\x20\x20\x20\x20 var importObject = {{ env: {{}} }};\n\
         \x20\x20\x20\x20\x20 function fallbackInstantiate() {{\n\
         \x20\x20\x20\x20\x20\x20\x20 return fetch(WASM_PATH)\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20 .then(function (resp) {{ return resp.arrayBuffer(); }})\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20 .then(function (bytes) {{ return WebAssembly.instantiate(bytes, importObject); }});\n\
         \x20\x20\x20\x20\x20 }}\n\
         \x20\x20\x20\x20\x20 var loader;\n\
         \x20\x20\x20\x20\x20 if (WebAssembly.instantiateStreaming) {{\n\
         \x20\x20\x20\x20\x20\x20\x20 loader = WebAssembly.instantiateStreaming(fetch(WASM_PATH), importObject)\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20 .catch(fallbackInstantiate);\n\
         \x20\x20\x20\x20\x20 }} else {{\n\
         \x20\x20\x20\x20\x20\x20\x20 loader = fallbackInstantiate();\n\
         \x20\x20\x20\x20\x20 }}\n\
         \x20\x20\x20\x20\x20 loader.then(function (result) {{\n\
         \x20\x20\x20\x20\x20\x20\x20 var start = result.instance.exports.start || result.instance.exports.main;\n\
         \x20\x20\x20\x20\x20\x20\x20 if (typeof start === 'function') {{ start(); }}\n\
         \x20\x20\x20\x20\x20 }}).catch(function (err) {{\n\
         \x20\x20\x20\x20\x20\x20\x20 console.error('failed to load qapp wasm:', err);\n\
         \x20\x20\x20\x20\x20\x20\x20 var el = document.getElementById('app');\n\
         \x20\x20\x20\x20\x20\x20\x20 if (el) {{ el.textContent = 'Failed to load. Check your connection and reload.'; }}\n\
         \x20\x20\x20\x20\x20 }});\n\
         \x20\x20\x20 }})();\n\
         \x20 </script>\n\
         </body>\n\
         </html>\n",
        title = title_html,
        description = description_html,
        theme = theme_html,
        short_name = short_name_html,
        bg = bg_html,
        wasm_path = wasm_path_js,
    )
}

/// Generate a standards-compliant, installable PWA scaffold for a qapp.
///
/// Emits three files: `manifest.webmanifest` (W3C Web App Manifest), `sw.js` (cache-first
/// service worker for offline capability), and `index.html` (loader that registers the service
/// worker and instantiates the qapp's wasm bundle). See the module doc for the secure-origin
/// caveat and the fact that this does not compile wasm.
pub fn generate_pwa(manifest: &QappManifest) -> PwaBundle {
    let files = vec![
        PwaFile {
            path: "manifest.webmanifest".to_string(),
            content: PwaContent::Text(build_webmanifest(manifest)),
        },
        PwaFile {
            path: "sw.js".to_string(),
            content: PwaContent::Text(build_service_worker(manifest)),
        },
        PwaFile {
            path: "index.html".to_string(),
            content: PwaContent::Text(build_index_html(manifest)),
        },
    ];
    PwaBundle { files }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qapp_package::manifest::{IconRef, QappManifest, WasmRef};

    fn sample() -> QappManifest {
        QappManifest::new("coop.qualia.journal", "Qualia Journal")
            .with_version("1.2.3")
            .with_description("A private journal")
            .with_entry(WasmRef {
                path: "./journal.wasm".to_string(),
                sha256_hex: "abc123".to_string(),
                size_bytes: 8192,
            })
            .with_icon(IconRef {
                src: "./icons/icon-192.png".to_string(),
                sizes: "192x192".to_string(),
                purpose: "any".to_string(),
            })
    }

    #[test]
    fn generates_the_three_expected_files() {
        let bundle = generate_pwa(&sample());
        assert!(bundle.get("manifest.webmanifest").is_some());
        assert!(bundle.get("sw.js").is_some());
        assert!(bundle.get("index.html").is_some());
        assert_eq!(bundle.files.len(), 3);
    }

    #[test]
    fn get_and_text_of_helpers_work() {
        let bundle = generate_pwa(&sample());
        assert!(bundle.text_of("sw.js").is_some());
        assert!(bundle.get("nope.txt").is_none());
        assert!(bundle.text_of("nope.txt").is_none());
    }

    #[test]
    fn webmanifest_has_required_keys_and_values() {
        let bundle = generate_pwa(&sample());
        let text = bundle.text_of("manifest.webmanifest").expect("manifest present");

        // Raw string assertions the task asks for.
        assert!(text.contains("\"display\": \"standalone\""), "text: {text}");
        assert!(text.contains("\"start_url\": \".\""), "text: {text}");
        assert!(text.contains("Qualia Journal"));
        assert!(text.contains("#101418"), "theme color present");

        // Parse it back and assert structurally.
        let v: serde_json::Value = serde_json::from_str(text).expect("valid JSON");
        assert_eq!(v["display"], "standalone");
        assert_eq!(v["start_url"], ".");
        assert_eq!(v["scope"], ".");
        assert_eq!(v["name"], "Qualia Journal");
        assert_eq!(v["short_name"], "Qualia Journal");
        assert_eq!(v["theme_color"], "#101418");
        assert_eq!(v["background_color"], "#0b0d10");
        assert_eq!(v["id"], "coop.qualia.journal");
        let icons = v["icons"].as_array().expect("icons array");
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0]["src"], "./icons/icon-192.png");
        assert_eq!(icons[0]["type"], "image/png");
        assert_eq!(icons[0]["sizes"], "192x192");
    }

    #[test]
    fn service_worker_references_wasm_and_versioned_cache() {
        let bundle = generate_pwa(&sample());
        let sw = bundle.text_of("sw.js").expect("sw present");
        // Version-stamped cache name.
        assert!(sw.contains("coop.qualia.journal-1.2.3"), "sw: {sw}");
        // References the wasm bundle path in the precache.
        assert!(sw.contains("./journal.wasm"), "sw: {sw}");
        // Standard SW lifecycle + cache-first fetch handler.
        assert!(sw.contains("addEventListener('install'"));
        assert!(sw.contains("addEventListener('fetch'"));
        assert!(sw.contains("caches.match"));
        // Icons are precached too.
        assert!(sw.contains("./icons/icon-192.png"));
    }

    #[test]
    fn index_html_references_manifest_sw_and_wasm() {
        let bundle = generate_pwa(&sample());
        let html = bundle.text_of("index.html").expect("index present");
        assert!(html.contains("manifest.webmanifest"), "html: {html}");
        assert!(html.contains("./sw.js"), "html: {html}");
        assert!(html.contains("./journal.wasm"), "html: {html}");
        // iOS installability meta tags.
        assert!(html.contains("apple-mobile-web-app-capable"));
        assert!(html.contains("apple-mobile-web-app-title"));
        // theme-color meta + manifest link.
        assert!(html.contains("name=\"theme-color\""));
        assert!(html.contains("rel=\"manifest\""));
        // wasm loader.
        assert!(html.contains("WebAssembly.instantiateStreaming"));
        assert!(html.contains("serviceWorker"));
    }

    #[test]
    fn html_escapes_dangerous_strings() {
        // A name containing HTML/script metacharacters must be escaped in index.html.
        let m = QappManifest::new("coop.qualia.evil", "Journal <script>alert('x')</script> & \"co\"")
            .with_icon(IconRef {
                src: "./i.png".to_string(),
                sizes: "any".to_string(),
                purpose: "any".to_string(),
            });
        let bundle = generate_pwa(&m);
        let html = bundle.text_of("index.html").unwrap();
        assert!(!html.contains("<script>alert('x')</script>"), "raw script leaked: {html}");
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&amp;"));
    }

    #[test]
    fn js_escapes_wasm_path_with_quote() {
        // A wasm path containing a single quote must not break out of the JS string literal.
        let m = QappManifest::new("coop.qualia.q", "Q")
            .with_entry(WasmRef {
                path: "./a'b.wasm".to_string(),
                sha256_hex: String::new(),
                size_bytes: 0,
            })
            .with_icon(IconRef {
                src: "./i.png".to_string(),
                sizes: "any".to_string(),
                purpose: "any".to_string(),
            });
        let bundle = generate_pwa(&m);
        let html = bundle.text_of("index.html").unwrap();
        assert!(html.contains("./a\\'b.wasm"), "quote not escaped: {html}");
    }

    #[test]
    fn custom_display_mode_propagates() {
        let mut m = sample();
        m.display = "fullscreen".to_string();
        let bundle = generate_pwa(&m);
        let v: serde_json::Value =
            serde_json::from_str(bundle.text_of("manifest.webmanifest").unwrap()).unwrap();
        assert_eq!(v["display"], "fullscreen");
    }

    #[test]
    fn icon_type_inferred_from_extension() {
        let m = QappManifest::new("coop.q.svg", "S").with_icon(IconRef {
            src: "./icon.svg".to_string(),
            sizes: "any".to_string(),
            purpose: "maskable".to_string(),
        });
        let bundle = generate_pwa(&m);
        let v: serde_json::Value =
            serde_json::from_str(bundle.text_of("manifest.webmanifest").unwrap()).unwrap();
        assert_eq!(v["icons"][0]["type"], "image/svg+xml");
    }
}
