//! **Webizen trust store** — user-controlled trust anchors (P1).
//!
//! The software provides the *means* to import, enable, disable, and evaluate roots;
//! it does not silently redefine the OS PKI. Default suggested set is empty until the
//! principal coins a bundle (AU community roots, etc.).
//!
//! Honest scope of this module:
//! - Persist anchors (PEM certs, DID / front-door identifiers, labels).
//! - Produce a **trust verdict** for a URL (scheme + store membership + notes).
//! - Supply PEM material for *our* TLS clients (agent fetch via rustls/reqwest).
//! - OS WebView cert-override (WebView2 `ServerCertificateErrorDetected`) is a
//!   platform hook layered in `webizen-desktop`; this store is the policy source.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TRUST_STORE_FILE: &str = "webizen/trust_store.json";
/// Suggested catalog (empty until principal curates). Relative to storage or bundled.
pub const SUGGESTED_CATALOG_FILE: &str = "webizen/suggested_trust_catalog.json";
pub const CATALOG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnchorKind {
    /// X.509 root or intermediate PEM.
    PemRoot,
    /// Front-door / connection DID or WebID.
    Did,
    /// Opaque label the principal trusts by policy (e.g. micro-commons id).
    PolicyLabel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAnchor {
    pub id: String,
    pub label: String,
    pub kind: AnchorKind,
    /// PEM body for PemRoot; DID URI for Did; free text for PolicyLabel.
    pub material: String,
    pub enabled: bool,
    pub notes: String,
    pub added_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustStore {
    pub version: u32,
    pub anchors: Vec<TrustAnchor>,
    /// When true, http(s) sites with no matching custom policy are labelled "os-default".
    pub defer_unknown_https_to_os: bool,
}

impl TrustStore {
    pub fn new() -> Self {
        Self {
            version: 1,
            anchors: Vec::new(),
            defer_unknown_https_to_os: true,
        }
    }

    pub fn path(storage_root: &Path) -> PathBuf {
        storage_root.join(TRUST_STORE_FILE)
    }

    pub fn load(storage_root: &Path) -> Self {
        let p = Self::path(storage_root);
        match fs::read_to_string(&p) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self, storage_root: &Path) -> Result<(), String> {
        let p = Self::path(storage_root);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp = p.with_extension("json.tmp");
        fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &p).map_err(|e| e.to_string())
    }

    pub fn add_pem_root(
        &mut self,
        label: &str,
        pem: &str,
        notes: &str,
        now: u64,
    ) -> Result<TrustAnchor, String> {
        let pem = pem.trim();
        if !pem.contains("BEGIN CERTIFICATE") {
            return Err("expected PEM certificate (BEGIN CERTIFICATE)".into());
        }
        let id = format!("pem:{}", short_hash(pem.as_bytes()));
        if self.anchors.iter().any(|a| a.id == id) {
            return Err("anchor already present".into());
        }
        let a = TrustAnchor {
            id: id.clone(),
            label: if label.trim().is_empty() {
                format!("Root {id}")
            } else {
                label.trim().into()
            },
            kind: AnchorKind::PemRoot,
            material: pem.into(),
            enabled: true,
            notes: notes.into(),
            added_unix: now,
        };
        self.anchors.push(a.clone());
        Ok(a)
    }

    pub fn add_did(&mut self, label: &str, did: &str, notes: &str, now: u64) -> Result<TrustAnchor, String> {
        let did = did.trim();
        if !did.starts_with("did:") {
            return Err("DID must start with did:".into());
        }
        let id = format!("did:{}", short_hash(did.as_bytes()));
        if self.anchors.iter().any(|a| a.material == did) {
            return Err("DID already present".into());
        }
        let a = TrustAnchor {
            id,
            label: if label.trim().is_empty() {
                did.to_string()
            } else {
                label.trim().into()
            },
            kind: AnchorKind::Did,
            material: did.into(),
            enabled: true,
            notes: notes.into(),
            added_unix: now,
        };
        self.anchors.push(a.clone());
        Ok(a)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        let a = self
            .anchors
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| format!("unknown anchor {id}"))?;
        a.enabled = enabled;
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.anchors.len();
        self.anchors.retain(|a| a.id != id);
        self.anchors.len() < before
    }

    /// Enabled PEM roots concatenated for rustls/custom clients.
    pub fn enabled_pem_bundle(&self) -> String {
        self.anchors
            .iter()
            .filter(|a| a.enabled && a.kind == AnchorKind::PemRoot)
            .map(|a| a.material.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn enabled_dids(&self) -> Vec<&str> {
        self.anchors
            .iter()
            .filter(|a| a.enabled && a.kind == AnchorKind::Did)
            .map(|a| a.material.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustVerdict {
    pub url: String,
    pub scheme: String,
    pub host: String,
    /// os_default | custom_root_available | did_match | local_scheme | untrusted_policy | unknown
    pub level: String,
    pub summary: String,
    pub matching_anchors: Vec<String>,
    pub notes: Vec<String>,
}

/// Evaluate how *our* store thinks about this URL (does not replace OS TLS for WebView).
pub fn evaluate_url(store: &TrustStore, url: &str) -> TrustVerdict {
    let url = url.trim();
    let (scheme, host) = parse_scheme_host(url);
    let mut matching = Vec::new();
    let mut notes = Vec::new();

    if scheme == "qualia" || scheme == "webizen" {
        return TrustVerdict {
            url: url.into(),
            scheme: scheme.clone(),
            host: host.clone(),
            level: "local_scheme".into(),
            summary: "Local Qualia/Webizen scheme — rendered under your device policy, not public CA trust."
                .into(),
            matching_anchors: Vec::new(),
            notes: vec!["Native protocol handler; not subject to public web PKI.".into()],
        };
    }

    // DID anchors: match if host or path contains the DID, or URL is a did:
    for a in store.anchors.iter().filter(|a| a.enabled && a.kind == AnchorKind::Did) {
        if url.contains(&a.material) || host.contains(&a.material) {
            matching.push(a.label.clone());
        }
    }
    if !matching.is_empty() {
        return TrustVerdict {
            url: url.into(),
            scheme,
            host,
            level: "did_match".into(),
            summary: format!(
                "Matches DID/front-door anchor(s) in your store: {}.",
                matching.join(", ")
            ),
            matching_anchors: matching,
            notes: vec![
                "DID trust is policy-level in Webizen; OS TLS may still apply for https.".into(),
            ],
        };
    }

    let pem_count = store
        .anchors
        .iter()
        .filter(|a| a.enabled && a.kind == AnchorKind::PemRoot)
        .count();
    if pem_count > 0 {
        notes.push(format!(
            "{pem_count} custom PEM root(s) enabled — used for agent HTTPS fetch; WebView still uses OS store unless platform cert-override is wired."
        ));
    }

    if scheme == "https" || scheme == "http" {
        let level = if store.defer_unknown_https_to_os {
            "os_default"
        } else {
            "untrusted_policy"
        };
        let summary = if store.defer_unknown_https_to_os {
            "No custom DID match. HTTPS validation defers to the OS trust store (WebView)."
                .to_string()
        } else {
            "defer_unknown_https_to_os=false: treat unknown public sites as untrusted by policy."
                .to_string()
        };
        return TrustVerdict {
            url: url.into(),
            scheme,
            host,
            level: level.into(),
            summary,
            matching_anchors: matching,
            notes,
        };
    }

    TrustVerdict {
        url: url.into(),
        scheme,
        host,
        level: "unknown".into(),
        summary: "Scheme not classified by the Webizen trust policy.".into(),
        matching_anchors: matching,
        notes,
    }
}

fn parse_scheme_host(url: &str) -> (String, String) {
    let u = url.trim();
    if let Some(rest) = u.strip_prefix("https://") {
        let host = rest.split(['/', '?', '#']).next().unwrap_or("").to_string();
        return ("https".into(), host);
    }
    if let Some(rest) = u.strip_prefix("http://") {
        let host = rest.split(['/', '?', '#']).next().unwrap_or("").to_string();
        return ("http".into(), host);
    }
    if let Some(rest) = u.strip_prefix("qualia://") {
        return ("qualia".into(), rest.to_string());
    }
    if let Some(rest) = u.strip_prefix("webizen://") {
        return ("webizen".into(), rest.to_string());
    }
    if u.starts_with("did:") {
        return ("did".into(), u.to_string());
    }
    ("unknown".into(), u.to_string())
}

fn short_hash(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    hex::encode(&d[..8])
}

// ── Suggested trust catalog (T0/T1) — means only; no invented roots ──────────

/// One **suggested** anchor. Never auto-enabled unless `enabled_by_default` is true
/// (must stay false until the principal explicitly curates a default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAnchor {
    pub id: String,
    pub label: String,
    /// Jurisdiction or community tag (e.g. "AU", "micro-commons") — free text.
    #[serde(default)]
    pub jurisdiction: String,
    pub kind: AnchorKind,
    /// Inline PEM / DID / label material. Prefer inline for small catalogs.
    #[serde(default)]
    pub material: String,
    /// Optional path relative to catalog dir (e.g. `roots/example.pem`). Loaded if material empty.
    #[serde(default)]
    pub material_path: Option<String>,
    /// Must be false for ship defaults until principal curates.
    #[serde(default)]
    pub enabled_by_default: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedTrustCatalog {
    pub version: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entries: Vec<SuggestedAnchor>,
}

impl Default for SuggestedTrustCatalog {
    fn default() -> Self {
        Self {
            version: CATALOG_VERSION,
            description: "Empty suggested trust catalog. Principal curates content; software provides means only.".into(),
            entries: Vec::new(),
        }
    }
}

impl SuggestedTrustCatalog {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load catalog from JSON bytes. Malformed → Err (fail closed).
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() {
            return Ok(Self::empty());
        }
        serde_json::from_slice(bytes).map_err(|e| format!("suggested catalog parse: {e}"))
    }

    pub fn from_json_str(s: &str) -> Result<Self, String> {
        Self::from_json_bytes(s.as_bytes())
    }

    /// Load from a file path; missing file → empty catalog (not an error).
    pub fn load_path(path: &Path) -> Result<Self, String> {
        match fs::read(path) {
            Ok(b) => Self::from_json_bytes(&b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Prefer storage override, else bundled path next to catalog default location.
    pub fn load_for_storage(storage_root: &Path) -> Result<Self, String> {
        let storage_cat = storage_root.join(SUGGESTED_CATALOG_FILE);
        if storage_cat.is_file() {
            return Self::load_path(&storage_cat);
        }
        // Bundled empty catalog (repo / package).
        if let Some(bundled) = bundled_catalog_path() {
            return Self::load_path(&bundled);
        }
        Ok(Self::empty())
    }

    pub fn get(&self, id: &str) -> Option<&SuggestedAnchor> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Resolve material for an entry (inline or file relative to `base_dir`).
    pub fn resolve_material(&self, entry: &SuggestedAnchor, base_dir: &Path) -> Result<String, String> {
        let inline = entry.material.trim();
        if !inline.is_empty() {
            return Ok(inline.to_string());
        }
        if let Some(rel) = entry.material_path.as_deref() {
            let p = base_dir.join(rel);
            return fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()));
        }
        Err(format!("suggested anchor {} has no material", entry.id))
    }
}

/// Path to repo/package empty catalog when present.
pub fn bundled_catalog_path() -> Option<PathBuf> {
    // Desktop: next to exe resources or relative to CARGO_MANIFEST of client-core.
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bundled/trust/catalog.json"),
        PathBuf::from("bundled/trust/catalog.json"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// Import a suggested entry into the live store.
/// `force_enabled` overrides `enabled_by_default` (UI "Enable now").
pub fn import_suggested_into_store(
    store: &mut TrustStore,
    catalog: &SuggestedTrustCatalog,
    entry_id: &str,
    base_dir: &Path,
    now: u64,
    force_enabled: bool,
) -> Result<TrustAnchor, String> {
    let entry = catalog
        .get(entry_id)
        .ok_or_else(|| format!("unknown suggested id {entry_id}"))?;
    let material = catalog.resolve_material(entry, base_dir)?;
    let enabled = force_enabled || entry.enabled_by_default;
    match entry.kind {
        AnchorKind::PemRoot => {
            let a = store.add_pem_root(&entry.label, &material, &entry.notes, now)?;
            if !enabled {
                let _ = store.set_enabled(&a.id, false);
            }
            Ok(store
                .anchors
                .iter()
                .find(|x| x.id == a.id)
                .cloned()
                .unwrap_or(a))
        }
        AnchorKind::Did => {
            let a = store.add_did(&entry.label, &material, &entry.notes, now)?;
            if !enabled {
                let _ = store.set_enabled(&a.id, false);
            }
            Ok(store
                .anchors
                .iter()
                .find(|x| x.id == a.id)
                .cloned()
                .unwrap_or(a))
        }
        AnchorKind::PolicyLabel => {
            let id = format!("policy:{}", short_hash(material.as_bytes()));
            if store.anchors.iter().any(|a| a.id == id) {
                return Err("anchor already present".into());
            }
            let a = TrustAnchor {
                id: id.clone(),
                label: entry.label.clone(),
                kind: AnchorKind::PolicyLabel,
                material,
                enabled,
                notes: entry.notes.clone(),
                added_unix: now,
            };
            store.anchors.push(a.clone());
            Ok(a)
        }
    }
}

/// Host/SPKI allowlist decision for cert-override (C1 pure helper).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertOverrideDecision {
    /// No policy match — deny by default.
    Deny,
    /// PEM material present; platform must still verify chain against it.
    AllowIfChainMatchesStore,
    /// Explicit host allow entry (policy label `host-allow:example.com`).
    AllowHostPinned,
}

/// Policy for ServerCertificateErrorDetected handlers.
pub fn cert_override_decision(store: &TrustStore, host: &str) -> CertOverrideDecision {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return CertOverrideDecision::Deny;
    }
    for a in store.anchors.iter().filter(|a| a.enabled) {
        if a.kind == AnchorKind::PolicyLabel {
            let m = a.material.trim().to_ascii_lowercase();
            if m == format!("host-allow:{host}") || m == host {
                return CertOverrideDecision::AllowHostPinned;
            }
        }
    }
    let pem_n = store
        .anchors
        .iter()
        .filter(|a| a.enabled && a.kind == AnchorKind::PemRoot)
        .count();
    if pem_n > 0 {
        return CertOverrideDecision::AllowIfChainMatchesStore;
    }
    CertOverrideDecision::Deny
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pem_and_did_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = TrustStore::new();
        s.add_did("Front door", "did:web:example.org", "test", 1)
            .unwrap();
        let pem = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";
        s.add_pem_root("Test root", pem, "", 2).unwrap();
        s.save(dir.path()).unwrap();
        let loaded = TrustStore::load(dir.path());
        assert_eq!(loaded.anchors.len(), 2);
        let v = evaluate_url(&loaded, "https://example.org/x?ref=did:web:example.org");
        assert_eq!(v.level, "did_match");
        let v2 = evaluate_url(&loaded, "https://google.com/");
        assert_eq!(v2.level, "os_default");
        let v3 = evaluate_url(&loaded, "qualia://webid/did:q42:local");
        assert_eq!(v3.level, "local_scheme");
    }

    #[test]
    fn add_did_then_remove_reverts_verdict() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = TrustStore::new();
        let a = s
            .add_did("Peer", "did:web:trusted.example", "", 10)
            .unwrap();
        s.save(dir.path()).unwrap();
        let loaded = TrustStore::load(dir.path());
        let url = "https://site.test/?id=did:web:trusted.example";
        assert_eq!(evaluate_url(&loaded, url).level, "did_match");
        let mut s2 = TrustStore::load(dir.path());
        assert!(s2.remove(&a.id));
        s2.save(dir.path()).unwrap();
        let loaded2 = TrustStore::load(dir.path());
        assert_eq!(evaluate_url(&loaded2, url).level, "os_default");
    }

    #[test]
    fn disable_anchor_skips_match() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = TrustStore::new();
        let a = s
            .add_did("Peer", "did:web:off.example", "", 1)
            .unwrap();
        s.set_enabled(&a.id, false).unwrap();
        s.save(dir.path()).unwrap();
        let loaded = TrustStore::load(dir.path());
        let v = evaluate_url(&loaded, "https://x/?did:web:off.example");
        assert_eq!(v.level, "os_default");
    }

    #[test]
    fn empty_catalog_loads() {
        let c = SuggestedTrustCatalog::from_json_str(
            r#"{"version":1,"description":"test","entries":[]}"#,
        )
        .unwrap();
        assert!(c.entries.is_empty());
        assert_eq!(SuggestedTrustCatalog::empty().entries.len(), 0);
    }

    #[test]
    fn malformed_catalog_fails_closed() {
        assert!(SuggestedTrustCatalog::from_json_str("{not json").is_err());
    }

    #[test]
    fn import_suggested_did_disabled_by_default() {
        let cat = SuggestedTrustCatalog {
            version: 1,
            description: "fixture".into(),
            entries: vec![SuggestedAnchor {
                id: "sug-did-1".into(),
                label: "Suggested peer".into(),
                jurisdiction: "test".into(),
                kind: AnchorKind::Did,
                material: "did:web:suggested.example".into(),
                material_path: None,
                enabled_by_default: false,
                notes: "fixture only".into(),
                source_url: None,
                license: None,
            }],
        };
        let mut store = TrustStore::new();
        let a = import_suggested_into_store(
            &mut store,
            &cat,
            "sug-did-1",
            Path::new("."),
            1,
            false,
        )
        .unwrap();
        assert!(!a.enabled);
        assert_eq!(
            cert_override_decision(&store, "evil.example"),
            CertOverrideDecision::Deny
        );
    }

    #[test]
    fn cert_override_host_pin() {
        let mut s = TrustStore::new();
        s.anchors.push(TrustAnchor {
            id: "policy:host".into(),
            label: "Pin".into(),
            kind: AnchorKind::PolicyLabel,
            material: "host-allow:intranet.local".into(),
            enabled: true,
            notes: "".into(),
            added_unix: 1,
        });
        assert_eq!(
            cert_override_decision(&s, "intranet.local"),
            CertOverrideDecision::AllowHostPinned
        );
        assert_eq!(
            cert_override_decision(&s, "other.local"),
            CertOverrideDecision::Deny
        );
    }
}
