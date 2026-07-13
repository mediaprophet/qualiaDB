//! Companion-qapp packaging: define a qapp, and generate an installable PWA scaffold for it.
//!
//! This is the transport-neutral **foundation** for the companion PWA feature — the means by which
//! a person defines a qapp (a cooperative front, a health tracker, a journal, a directory, …) and
//! obtains an installable, wasm-backed web app for their phone. It has two halves:
//!
//! - [`manifest`] — [`QappManifest`], the definition of a qapp: identity, semver, the extensible
//!   [`QappKind`], the **least-privilege** [`Capability`] scopes it requests, a content-addressed
//!   [`WasmRef`] to its wasm bundle, and PWA presentation metadata (icons, colours, display mode).
//! - [`pwa`] — [`generate_pwa`], which turns a manifest into a [`PwaBundle`]: a
//!   `manifest.webmanifest` (W3C Web App Manifest), a cache-first `sw.js` service worker for
//!   offline capability, and an `index.html` loader that registers the service worker and
//!   instantiates the qapp's wasm.
//!
//! ## Honest scope — what this module does and does NOT do
//!
//! It **does**:
//! - define a qapp in a transport-neutral way ([`QappManifest`]); and
//! - generate a **correct, standards-compliant, installable-in-principle** PWA scaffold
//!   ([`generate_pwa`]) — valid Web App Manifest, a working cache-first service worker, and a
//!   loader page with the iOS/Android install meta tags.
//!
//! It **does NOT**:
//! - **serve** anything. Browsers only offer to *install* a PWA when it is delivered from a
//!   **secure origin** (HTTPS, or `localhost` in development) with the manifest and service worker
//!   reachable under scope. That secure-origin delivery/pairing layer is a **separate, later
//!   piece** — this module produces the bytes, not the transport. A scaffold opened from a `file://`
//!   URL or a plain-HTTP origin will not register a service worker and will not be installable; that
//!   is a property of the delivery environment, not a defect here.
//! - **compile wasm.** The wasm bundle is referenced by path + SHA-256 hash + size
//!   ([`WasmRef`]); building that bundle is out of scope for this module.
//!
//! Nothing here is a stub: the manifest model, validation, and PWA generation are fully
//! implemented and tested. The two "does NOT" items above are genuine adjacent layers, not
//! deferred work inside this module.

pub mod manifest;
pub mod pwa;

pub use manifest::{Capability, IconRef, QappKind, QappManifest, WasmRef};
pub use pwa::{generate_pwa, PwaBundle, PwaContent, PwaFile};
