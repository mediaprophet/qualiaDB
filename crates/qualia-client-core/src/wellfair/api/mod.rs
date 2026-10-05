use super::policy::PolicyDecisionService;
use super::vault::VaultService;
use ed25519_dalek::SigningKey;
use wellfare_core::projects::Contribution;

const QAPP_SHELL: &str = "wellfair-shell";
const QAPP_LIFE: &str = "wellfair-life";
const QAPP_WELLBEING: &str = "wellfair-wellbeing";
const QAPP_FINANCE: &str = "wellfair-finance";
const QAPP_PROJECTS: &str = "wellfair-projects";
const QAPP_CREDENTIALS: &str = "wellfair-credentials";
const QAPP_CLINICAL: &str = "wellfair-clinical";
const QAPP_WELFARE: &str = "wellfair-welfare";
const SOURCE_PERSONAL: &str = "wellfair:personal";
const SOURCE_LIFE: &str = "wellfair:life";
const SOURCE_WELLBEING: &str = "wellfair:wellbeing";
const SOURCE_FINANCE: &str = "wellfair:finance";
const SOURCE_PROJECTS: &str = "wellfair:projects";
const SOURCE_CREDENTIALS: &str = "wellfair:credentials";
const SOURCE_CLINICAL: &str = "wellfair:clinical";
const SOURCE_WELFARE: &str = "wellfair:welfare";
const QAPP_COOPERATIVE: &str = "qualia-cooperative";
const SOURCE_COOPERATIVE: &str = "qualia:cooperative";
const QAPP_GUARDIANSHIP: &str = "wellfair-guardianship";
const SOURCE_GUARDIANSHIP: &str = "wellfair:guardianship";

/// Reconstruct a `Contribution` from a stored/transmitted summary JSON. The record id (which
/// is the dedup anchor for obligation derivation) is supplied by the caller — the journal row
/// id locally, or the sync operation's `record_id` for an inbound op.
fn contribution_from_summary(
    id: String,
    summary: &str,
    occurred_at_unix: u32,
) -> Option<Contribution> {
    let v: serde_json::Value = serde_json::from_str(summary).ok()?;
    Some(Contribution {
        id,
        project_id: v
            .get("project_id")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        contributor_did: v
            .get("contributor_did")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        description: String::new(),
        effort_minutes: v
            .get("effort_minutes")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as u32,
        capital_cents: v.get("capital_cents").and_then(|x| x.as_u64()).unwrap_or(0),
        roi_multiplier: v
            .get("roi_multiplier")
            .and_then(|x| x.as_f64())
            .map(|f| f as f32)
            .unwrap_or(1.0),
        privacy_level: Default::default(),
        occurred_at_unix,
        occurred_at_instant: None,
        predecessor_id: None,
    })
}

/// A per-entry summary of a hypermedia library entry for the UI (drops the raw quins).
pub(crate) fn library_summary(e: &super::hypermedia_store::LibraryEntry) -> serde_json::Value {
    serde_json::json!({
        "asset_uri": e.asset_uri,
        "media_type": e.media_type,
        "topics": e.topics,
        "projects": e.projects,
        "purposes": e.purposes,
        "place": e.place,
        "occurred_at": e.occurred_at,
        "lat": e.lat,
        "lon": e.lon,
        "flags": e.flags,
        "ingested_unix": e.ingested_unix,
        "excerpt": e.excerpt,
        "sensitivity": e.sensitivity,
        "section": e.section,
        "commons_visibility": e.commons_visibility,
        "is_secret": e.is_secret(),
        "cml_signals": e.cml_signals,
        "cml_concept_count": e.cml_concept_count,
        "cml_n3_chars": e.cml_n3.len(),
        "quin_count": e.quins.len(),
        "cof_segment_count": e.cof_segment_count,
        "cof_segment_index": e.cof_segment_index,
        "cof_profile": e.cof_profile,
        "cof_html_chars": e.cof_html.len(),
        "has_cof": !e.cof_html.is_empty(),
    })
}

/// Facets a **person** attaches to an asset at ingest — the "software provides the means, the person
/// authors the meaning" path. These merge *on top of* whatever a processor derived automatically (a photo's
/// EXIF still wins for its own time/place); they let a plain document be placed on the **timeline** (a date)
/// or the **map** (coordinates), or collected under a **project** / **purpose** — none of it imposed.
#[derive(Debug, Clone, Default)]
pub struct ManualFacets {
    pub occurred_at: Option<i64>,
    pub place_label: Option<String>,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub projects: Vec<String>,
    pub purposes: Vec<String>,
    /// `public` | `restricted` | `classified` — high sensitivity forces Secret section.
    pub sensitivity: Option<String>,
    /// Preferred product section: secret | wellfair | personal | work | tools | software | commons.
    pub section: Option<String>,
    /// `none` | `peers` | `commons` — social / micro-commons visibility.
    pub commons_visibility: Option<String>,
}

impl ManualFacets {
    fn is_empty(&self) -> bool {
        self.occurred_at.is_none()
            && self.place_label.is_none()
            && self.lat.is_none()
            && self.projects.is_empty()
            && self.purposes.is_empty()
            && self.sensitivity.is_none()
            && self.section.is_none()
            && self.commons_visibility.is_none()
    }
}

/// Decode a lowercase/uppercase hex string to bytes (the desktop passes binary assets — a JPEG is not utf-8 —
/// as hex across the command boundary). Dependency-free; rejects odd length / non-hex.
fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("odd-length hex".to_string());
    }
    let val = |c: u8| -> Result<u8, String> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(format!("non-hex byte {:#x}", c)),
        }
    };
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i < b.len() {
        out.push((val(b[i])? << 4) | val(b[i + 1])?);
        i += 2;
    }
    Ok(out)
}

/// Parse a model string (`"male"` / `"female"`, case-insensitive) into an `AnatomyModel()`.
pub fn parse_anatomy_model(s: &str) -> Result<wellfare_core::anatomy::AnatomyModel, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "male" | "m" | "xy" => Ok(wellfare_core::anatomy::AnatomyModel::Male),
        "female" | "f" | "xx" => Ok(wellfare_core::anatomy::AnatomyModel::Female),
        _ => Err(format!(
            "unknown anatomy model '{s}' (expected male/female)"
        )),
    }
}

/// Bridge decoupled `qualia_cooperative_core::RecordEnvelope` to `wellfare_core::record::RecordEnvelope`.
pub(crate) fn coop_envelope_to_wellfare(
    env: qualia_cooperative_core::RecordEnvelope,
) -> wellfare_core::record::RecordEnvelope {
    wellfare_core::record::RecordEnvelope {
        id: env.id,
        owner_did: env.owner_did,
        author_did: env.author_did,
        proxy_did: env.proxy_did,
        epistemic_status: match env.epistemic_status {
            qualia_cooperative_core::record::EpistemicStatus::Asserted => {
                wellfare_core::record::EpistemicStatus::Asserted
            }
            qualia_cooperative_core::record::EpistemicStatus::Hypothesis => {
                wellfare_core::record::EpistemicStatus::Hypothesis
            }
            qualia_cooperative_core::record::EpistemicStatus::Disputed => {
                wellfare_core::record::EpistemicStatus::Disputed
            }
            qualia_cooperative_core::record::EpistemicStatus::Refuted => {
                wellfare_core::record::EpistemicStatus::Refuted
            }
        },
        evidence_type: match env.evidence_type {
            qualia_cooperative_core::record::EvidenceType::SelfReported => {
                wellfare_core::record::EvidenceType::SelfReported
            }
            qualia_cooperative_core::record::EvidenceType::DeviceMeasured => {
                wellfare_core::record::EvidenceType::DeviceMeasured
            }
            qualia_cooperative_core::record::EvidenceType::ClinicianObserved => {
                wellfare_core::record::EvidenceType::ClinicianObserved
            }
            qualia_cooperative_core::record::EvidenceType::Inferred => {
                wellfare_core::record::EvidenceType::Inferred
            }
        },
        sensitivity: match env.sensitivity {
            qualia_cooperative_core::record::SensitivityClass::Public => {
                wellfare_core::record::SensitivityClass::Public
            }
            qualia_cooperative_core::record::SensitivityClass::Restricted => {
                wellfare_core::record::SensitivityClass::Restricted
            }
            qualia_cooperative_core::record::SensitivityClass::Classified => {
                wellfare_core::record::SensitivityClass::Classified
            }
        },
        asserted_time_unix: env.asserted_time_unix,
        asserted_instant: env
            .asserted_instant
            .map(|i| wellfare_core::record::InstantBridge {
                secs: i.secs,
                nanos: i.nanos,
            }),
        valid_time_start_unix: env.valid_time_start_unix,
        valid_time_start_instant: env
            .valid_time_start_instant
            .map(|i| wellfare_core::record::InstantBridge {
                secs: i.secs,
                nanos: i.nanos,
            }),
        valid_time_end_unix: env.valid_time_end_unix,
        valid_time_end_instant: env
            .valid_time_end_instant
            .map(|i| wellfare_core::record::InstantBridge {
                secs: i.secs,
                nanos: i.nanos,
            }),
        predecessor_id: env.predecessor_id,
        blob_hash: env.blob_hash,
        tombstone: env.tombstone,
    }
}

/// Transport-neutral Host API exported for UI and qApps.
pub struct WebizenHostApi {
    vault: VaultService,
    policy: PolicyDecisionService,
    signing_key: SigningKey,
    owner_did: String,
    author_did: String,
    storage_root: std::path::PathBuf,
}

mod accountability;
mod anatomy;
mod host_core;
mod library;
/// Vault-free hypermedia library reads (storage path; no Sanctuary HostApi required).
pub use library::{
    library_stats_at, list_library_section_at, query_library_faceted_at, search_library_at,
    search_library_text_at, search_library_time_at,
};
mod agency;
mod backup_clinical;
mod coop;
mod disclosure;
mod encryption;
mod guardianship;
mod pwa;
mod sanctuary_vault;
mod sync;
mod types;
mod welfare_work;

pub use types::*;
