//! `QappManifest` — the transport-neutral definition of a companion qapp.
//!
//! A qapp is a person-defined mini-application (a cooperative front, a health tracker, a
//! journal, a directory, …) that is ultimately delivered to a phone as an installable PWA
//! wrapping a WebAssembly bundle. This module defines *what a qapp is*: its identity, the
//! **least-privilege** capabilities it requests, a content-addressed reference to its wasm
//! bundle, and the presentation metadata a PWA needs (icons, colours, display mode).
//!
//! It does **not** build wasm and it does not deliver anything — it only describes. The wasm
//! bundle is referenced by path + SHA-256 hash + size (see [`WasmRef`]); producing that bundle
//! and serving it over a secure origin are separate, later pieces (see the module doc in
//! `qapp_package/mod.rs`).

use serde::{Deserialize, Serialize};

/// The kind of qapp. **Extensible**: [`QappKind::Custom`] carries an arbitrary id so new kinds
/// need no code change. Serializes in `snake_case`; `Custom("x")` serializes as its inner string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QappKind {
    Cooperative,
    Health,
    Journal,
    Directory,
    Custom(String),
}

impl QappKind {
    /// A stable string form. Well-known kinds return their snake_case name; `Custom` returns
    /// its inner id verbatim.
    pub fn as_str(&self) -> &str {
        match self {
            QappKind::Cooperative => "cooperative",
            QappKind::Health => "health",
            QappKind::Journal => "journal",
            QappKind::Directory => "directory",
            QappKind::Custom(id) => id.as_str(),
        }
    }
}

impl Default for QappKind {
    fn default() -> Self {
        QappKind::Cooperative
    }
}

/// A least-privilege scope a qapp requests. An empty `capabilities` list is the safest default
/// (the qapp is granted nothing). **Extensible** via [`Capability::Custom`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadRecords,
    WriteRecords,
    Sync,
    BlobStore,
    Notifications,
    Camera,
    Custom(String),
}

impl Capability {
    /// A stable string form; `Custom` returns its inner id verbatim.
    pub fn as_str(&self) -> &str {
        match self {
            Capability::ReadRecords => "read_records",
            Capability::WriteRecords => "write_records",
            Capability::Sync => "sync",
            Capability::BlobStore => "blob_store",
            Capability::Notifications => "notifications",
            Capability::Camera => "camera",
            Capability::Custom(id) => id.as_str(),
        }
    }
}

/// A content-addressed reference to the qapp's compiled wasm bundle. This module references the
/// bundle; it does not produce it. `sha256_hex` binds the manifest to an exact artifact so a
/// later delivery layer can verify integrity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRef {
    /// Path (relative to the served PWA scope) at which the wasm bundle is fetched, e.g.
    /// `"./qapp.wasm"`.
    pub path: String,
    /// Lower-case hex SHA-256 of the wasm bytes. Empty until a bundle is attached.
    pub sha256_hex: String,
    /// Size of the wasm bundle in bytes.
    pub size_bytes: u64,
}

impl Default for WasmRef {
    fn default() -> Self {
        Self {
            path: "./qapp.wasm".to_string(),
            sha256_hex: String::new(),
            size_bytes: 0,
        }
    }
}

/// An icon reference for the PWA manifest / home-screen install.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconRef {
    /// Icon source path (relative to scope), e.g. `"./icons/icon-192.png"`.
    pub src: String,
    /// Space-separated size list, e.g. `"192x192"` or `"any"`.
    pub sizes: String,
    /// `"any"` or `"maskable"` (may be a space-separated combination).
    pub purpose: String,
}

/// The transport-neutral definition of a qapp. Everything a PWA generator or a delivery layer
/// needs to describe, present, and integrity-check the qapp — without knowing how it is served.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QappManifest {
    /// Reverse-DNS-ish stable id, e.g. `"coop.qualia.journal"`.
    pub id: String,
    /// Human-facing full name.
    pub name: String,
    /// Short name for constrained surfaces (home screen), defaults to `name`.
    pub short_name: String,
    /// Semver string, e.g. `"0.1.0"`.
    pub version: String,
    /// One-line description.
    pub description: String,
    /// The kind of qapp (extensible).
    pub kind: QappKind,
    /// Least-privilege scopes requested. Empty = nothing granted.
    pub capabilities: Vec<Capability>,
    /// Content-addressed reference to the wasm bundle.
    pub entry_wasm: WasmRef,
    /// Icons for install / launch surfaces.
    pub icons: Vec<IconRef>,
    /// PWA theme colour (CSS colour string).
    pub theme_color: String,
    /// PWA background colour (CSS colour string).
    pub background_color: String,
    /// PWA display mode, e.g. `"standalone"`, `"fullscreen"`, `"minimal-ui"`, `"browser"`.
    pub display: String,
    /// Whether the qapp is expected to work offline (drives service-worker precache intent).
    pub offline: bool,
}

impl QappManifest {
    /// Create a manifest with sane defaults. `short_name` defaults to `name`; the wasm ref,
    /// icons, and capabilities start empty/least-privilege.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: id.into(),
            short_name: name.clone(),
            name,
            version: "0.1.0".to_string(),
            description: String::new(),
            kind: QappKind::default(),
            capabilities: Vec::new(),
            entry_wasm: WasmRef::default(),
            icons: Vec::new(),
            theme_color: "#101418".to_string(),
            background_color: "#0b0d10".to_string(),
            display: "standalone".to_string(),
            offline: true,
        }
    }

    /// Set the kind.
    pub fn with_kind(mut self, kind: QappKind) -> Self {
        self.kind = kind;
        self
    }

    /// Add a requested capability (least-privilege: only add what is needed).
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Set the content-addressed wasm reference.
    pub fn with_entry(mut self, entry_wasm: WasmRef) -> Self {
        self.entry_wasm = entry_wasm;
        self
    }

    /// Add an icon reference.
    pub fn with_icon(mut self, icon: IconRef) -> Self {
        self.icons.push(icon);
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the version (semver string).
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Validate the manifest, returning the full list of problems (empty = valid).
    ///
    /// Hard errors: empty id, id without a dot (not reverse-DNS-ish), empty name, empty version.
    /// A manifest with no icons is reported as a problem (an icon is required for a usable
    /// install), consistent with the task's rule that a manifest with no icon *and* no name
    /// must error — here we are stricter and always flag a missing icon, but the message is
    /// clearly an install-quality issue.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut problems = Vec::new();

        if self.id.trim().is_empty() {
            problems.push("id must not be empty".to_string());
        } else if !self.id.contains('.') {
            problems.push(format!(
                "id \"{}\" must be reverse-DNS-ish (contain at least one '.')",
                self.id
            ));
        }

        if self.name.trim().is_empty() {
            problems.push("name must not be empty".to_string());
        }

        if self.version.trim().is_empty() {
            problems.push("version must not be empty".to_string());
        }

        if self.icons.is_empty() {
            problems
                .push("at least one icon is required for a usable home-screen install".to_string());
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_sane_defaults() {
        let m = QappManifest::new("coop.qualia.journal", "Qualia Journal");
        assert_eq!(m.id, "coop.qualia.journal");
        assert_eq!(m.name, "Qualia Journal");
        assert_eq!(m.short_name, "Qualia Journal");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.display, "standalone");
        assert_eq!(m.theme_color, "#101418");
        assert_eq!(m.background_color, "#0b0d10");
        assert!(m.offline);
        assert!(
            m.capabilities.is_empty(),
            "least privilege = empty by default"
        );
        assert_eq!(m.kind, QappKind::Cooperative);
    }

    #[test]
    fn builders_compose() {
        let m = QappManifest::new("coop.qualia.journal", "Qualia Journal")
            .with_kind(QappKind::Journal)
            .with_version("1.2.3")
            .with_description("A private journal")
            .with_capability(Capability::ReadRecords)
            .with_capability(Capability::WriteRecords)
            .with_entry(WasmRef {
                path: "./journal.wasm".to_string(),
                sha256_hex: "deadbeef".to_string(),
                size_bytes: 4096,
            })
            .with_icon(IconRef {
                src: "./icons/icon-192.png".to_string(),
                sizes: "192x192".to_string(),
                purpose: "any".to_string(),
            });
        assert_eq!(m.kind, QappKind::Journal);
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.description, "A private journal");
        assert_eq!(m.capabilities.len(), 2);
        assert_eq!(m.entry_wasm.path, "./journal.wasm");
        assert_eq!(m.icons.len(), 1);
    }

    #[test]
    fn manifest_serde_round_trips() {
        let m = QappManifest::new("coop.qualia.directory", "Directory")
            .with_kind(QappKind::Directory)
            .with_capability(Capability::Sync)
            .with_icon(IconRef {
                src: "./i.png".to_string(),
                sizes: "any".to_string(),
                purpose: "maskable".to_string(),
            });
        let json = serde_json::to_string(&m).expect("serialize");
        let back: QappManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, back);
    }

    #[test]
    fn well_known_kind_serializes_snake_case() {
        let json = serde_json::to_string(&QappKind::Cooperative).unwrap();
        assert_eq!(json, "\"cooperative\"");
        let json = serde_json::to_string(&QappKind::Health).unwrap();
        assert_eq!(json, "\"health\"");
    }

    #[test]
    fn custom_kind_serializes_to_its_string() {
        let json =
            serde_json::to_string(&QappKind::Custom("coop.custom.kind".to_string())).unwrap();
        assert_eq!(json, "{\"custom\":\"coop.custom.kind\"}");
        assert_eq!(QappKind::Custom("x".to_string()).as_str(), "x");
    }

    #[test]
    fn custom_capability_serializes_to_its_string() {
        let json = serde_json::to_string(&Capability::Custom("geolocation".to_string())).unwrap();
        assert_eq!(json, "{\"custom\":\"geolocation\"}");
        assert_eq!(
            Capability::Custom("geolocation".to_string()).as_str(),
            "geolocation"
        );
        // Well-known snake_case check.
        assert_eq!(
            serde_json::to_string(&Capability::ReadRecords).unwrap(),
            "\"read_records\""
        );
    }

    #[test]
    fn validate_ok_for_complete_manifest() {
        let m = QappManifest::new("coop.qualia.journal", "Journal").with_icon(IconRef {
            src: "./i.png".to_string(),
            sizes: "192x192".to_string(),
            purpose: "any".to_string(),
        });
        assert_eq!(m.validate(), Ok(()));
    }

    #[test]
    fn validate_catches_empty_id() {
        let m = QappManifest::new("", "Journal").with_icon(IconRef {
            src: "./i.png".to_string(),
            sizes: "any".to_string(),
            purpose: "any".to_string(),
        });
        let problems = m.validate().unwrap_err();
        assert!(problems.iter().any(|p| p.contains("id must not be empty")));
    }

    #[test]
    fn validate_catches_missing_dot() {
        let m = QappManifest::new("journal", "Journal").with_icon(IconRef {
            src: "./i.png".to_string(),
            sizes: "any".to_string(),
            purpose: "any".to_string(),
        });
        let problems = m.validate().unwrap_err();
        assert!(problems.iter().any(|p| p.contains("reverse-DNS-ish")));
    }

    #[test]
    fn validate_catches_empty_name() {
        let m = QappManifest::new("coop.qualia.journal", "").with_icon(IconRef {
            src: "./i.png".to_string(),
            sizes: "any".to_string(),
            purpose: "any".to_string(),
        });
        let problems = m.validate().unwrap_err();
        assert!(problems
            .iter()
            .any(|p| p.contains("name must not be empty")));
    }

    #[test]
    fn validate_reports_missing_icon() {
        let m = QappManifest::new("coop.qualia.journal", "Journal");
        let problems = m.validate().unwrap_err();
        assert!(problems.iter().any(|p| p.contains("icon")));
    }

    #[test]
    fn validate_accumulates_all_problems() {
        // Empty id, empty name, empty version, no icon → four problems at once.
        let mut m = QappManifest::new("", "");
        m.version = String::new();
        let problems = m.validate().unwrap_err();
        assert_eq!(problems.len(), 4, "problems: {problems:?}");
    }
}
