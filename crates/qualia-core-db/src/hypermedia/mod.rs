//! **Hypermedia semantic library** — an asset ⊕ its analytics ⊕ its related/associated assets, bound as a
//! **semantic graph (NQuins), not a directory structure.**
//!
//! The Qualia line is *"context is the asset"*: an asset is never a bare file; it is a first-class entity
//! that carries, inseparably, what it was derived *from* (`prov:wasDerivedFrom`), the analytics computed
//! *about* it (`analysisTarget`/`analysisResult`), the related assets bound *with* it (`bundledWith` + a
//! role), and its provenance (`hasProvenance`). Because the binding is a **graph of edges over the one
//! identity space** (`q_hash`/`fnv` subjects, shared with [`crate::render::assets`]), you browse and query it
//! by **meaning and lineage** — "what was this derived from / what analytics belong to it / what's related" —
//! never by folder path.
//!
//! P0 (this module): the semantic model + [`container_to_nquins`] (emit the whole edge-graph) + the
//! query helpers that read the relationships back out. It composes the real primitives — [`NQuin`],
//! `q_hash`, and the same 60-bit FNV subject-hashing that `render/assets.rs::mesh_to_nquins` uses, so a
//! container's reference to a mesh asset resolves to *the same subject* that asset's own manifest emits.
//! P1 adds the in-`.10d` provenance sidecar + a validate-before-use gate; P2 re-points the anatomy pipeline
//! through this (an organ = mesh ⊕ systemic-burden analytics ⊕ source-GLB / provenance). See
//! `docs/plans/hypermedia-semantic-library.md`.

use std::collections::HashMap;

use crate::frame_layout::pack_float_object;
use crate::{q_hash, NQuin};

/// Content-processor implementations that derive searchability at ingest. The
/// model-free [`TextProcessor`] lives in this file (it is the framework
/// reference); the heavier [`ImageProcessor`] (EXIF time/place → the
/// timeline/map facets) and [`WavProcessor`] (STFT spectral summary) live in
/// [`processors`] as their own units (§11: split as the library grows).
pub mod processors;
pub use processors::{AudioSpectralSummary, ImageProcessor, WavProcessor};

/// The named graph the hypermedia relationship edges live in.
pub const HYPERMEDIA_CONTEXT: u64 = q_hash("urn:qualia:context:hypermedia");

const P_RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
const C_CONTAINER: u64 = q_hash("urn:qualia:hypermedia:Container");
const C_ANALYTICS: u64 = q_hash("urn:qualia:hypermedia:Analytics");
/// container → its primary asset.
const P_HAS_PRIMARY: u64 = q_hash("urn:qualia:hypermedia:hasPrimary");
/// container → each asset it bundles (primary + related).
const P_BUNDLED_WITH: u64 = q_hash("urn:qualia:hypermedia:bundledWith");
/// asset → its role class within the container.
const P_HAS_ROLE: u64 = q_hash("urn:qualia:hypermedia:hasRole");
/// asset → the source asset it was derived from (W3C PROV).
const P_WAS_DERIVED_FROM: u64 = q_hash("http://www.w3.org/ns/prov#wasDerivedFrom");
/// asset → its provenance record asset.
const P_HAS_PROVENANCE: u64 = q_hash("urn:qualia:hypermedia:hasProvenance");
/// analytics → the asset it is *about*.
const P_ANALYSIS_TARGET: u64 = q_hash("urn:qualia:hypermedia:analysisTarget");
/// container → an analytics result it carries.
const P_ANALYSIS_RESULT: u64 = q_hash("urn:qualia:hypermedia:analysisResult");
/// analytics → the method that produced it.
const P_ANALYSIS_METHOD: u64 = q_hash("urn:qualia:hypermedia:analysisMethod");
const P_MEDIA_TYPE: u64 = q_hash("urn:qualia:hypermedia:mediaType");
const P_DIGEST: u64 = q_hash("urn:qualia:hypermedia:digest");
const P_LICENCE: u64 = q_hash("http://purl.org/dc/terms/license");
const P_CREATOR: u64 = q_hash("http://purl.org/dc/terms/creator");

/// The role a bundled asset plays relative to the container's primary asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetRole {
    /// The container's principal asset (the thing it is *about*).
    Primary,
    /// An immutable original source the primary/derivations came from.
    Source,
    /// A derivation of the primary (e.g. the compiled `.10d`, a transcode).
    Derivation,
    /// A level-of-detail variant.
    Lod,
    /// An analysis-result asset.
    Analysis,
    /// A provenance record (source bytes / licence / VC).
    Provenance,
    /// Any other associated asset.
    Related,
}

impl AssetRole {
    const fn uri(self) -> &'static str {
        match self {
            AssetRole::Primary => "urn:qualia:hypermedia:role:primary",
            AssetRole::Source => "urn:qualia:hypermedia:role:source",
            AssetRole::Derivation => "urn:qualia:hypermedia:role:derivation",
            AssetRole::Lod => "urn:qualia:hypermedia:role:lod",
            AssetRole::Analysis => "urn:qualia:hypermedia:role:analysis",
            AssetRole::Provenance => "urn:qualia:hypermedia:role:provenance",
            AssetRole::Related => "urn:qualia:hypermedia:role:related",
        }
    }

    /// The `q_hash` of this role's class IRI — the object of a `hasRole` edge.
    pub fn class(self) -> u64 {
        q_hash(self.uri())
    }

    /// Map a role-class hash back to the role (for reading edges).
    pub fn from_class(class: u64) -> Option<AssetRole> {
        const ALL: [AssetRole; 7] = [
            AssetRole::Primary,
            AssetRole::Source,
            AssetRole::Derivation,
            AssetRole::Lod,
            AssetRole::Analysis,
            AssetRole::Provenance,
            AssetRole::Related,
        ];
        ALL.into_iter().find(|r| r.class() == class)
    }
}

/// A content-addressed reference to an asset (its stable subject is `fnv60(uri)`, the same subject its own
/// geometry manifest emits, so container edges join to the asset's facts in the one identity space).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRef {
    pub uri: String,
    /// Content digest (CRC-32C or SHA-256 truncated) — the anti-tamper / dedup anchor.
    pub digest: u64,
    pub media_type: String,
    pub role: AssetRole,
    /// URIs of the source asset(s) this one was derived from (emits `prov:wasDerivedFrom`).
    pub derived_from: Vec<String>,
    /// Optional licence / creator (dcterms) — the never-strip-context fields.
    pub licence: Option<String>,
    pub creator: Option<String>,
}

impl AssetRef {
    pub fn new(
        uri: impl Into<String>,
        digest: u64,
        media_type: impl Into<String>,
        role: AssetRole,
    ) -> Self {
        Self {
            uri: uri.into(),
            digest,
            media_type: media_type.into(),
            role,
            derived_from: Vec::new(),
            licence: None,
            creator: None,
        }
    }

    pub fn derived_from(mut self, source_uri: impl Into<String>) -> Self {
        self.derived_from.push(source_uri.into());
        self
    }
    pub fn with_licence(mut self, licence: impl Into<String>) -> Self {
        self.licence = Some(licence.into());
        self
    }
    pub fn subject(&self) -> u64 {
        fnv60(self.uri.as_bytes())
    }
}

/// An analytics result *about* an asset — the derived data bound back to the geometry it concerns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyticsRef {
    pub id: String,
    /// The method/tool that produced it (e.g. `wellfare:systemic-burden`).
    pub method: String,
    /// The URI of the asset this analysis is *about* (defaults to the container primary if empty).
    pub target_uri: String,
    /// A short serialized summary of the result (e.g. a JSON burden roll-up).
    pub summary: String,
}

impl AnalyticsRef {
    pub fn subject(&self) -> u64 {
        fnv60(self.id.as_bytes())
    }
}

/// A hypermedia container: a primary asset bundled with its related assets and its analytics, as one
/// addressable semantic unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypermediaContainer {
    pub uri: String,
    pub primary: AssetRef,
    pub related: Vec<AssetRef>,
    pub analytics: Vec<AnalyticsRef>,
}

impl HypermediaContainer {
    pub fn new(uri: impl Into<String>, primary: AssetRef) -> Self {
        Self {
            uri: uri.into(),
            primary,
            related: Vec::new(),
            analytics: Vec::new(),
        }
    }
    pub fn with_related(mut self, asset: AssetRef) -> Self {
        self.related.push(asset);
        self
    }
    pub fn with_analytics(mut self, analytics: AnalyticsRef) -> Self {
        self.analytics.push(analytics);
        self
    }
    pub fn subject(&self) -> u64 {
        fnv60(self.uri.as_bytes())
    }
}

/// The same 60-bit FNV-1a subject hash `render/assets.rs` uses — so a container's reference to a mesh asset
/// resolves to the *identical* subject that asset's own `mesh_to_nquins` manifest emits (one identity space).
pub(crate) fn fnv60(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

/// A content digest for an asset's bytes, in the same 60-bit identity space as
/// asset subjects — the anti-tamper / dedup anchor a caller stores as
/// [`AssetRef::digest`]. (Public wrapper so client-core ingest can compute it.)
pub fn content_digest(bytes: &[u8]) -> u64 {
    fnv60(bytes)
}

fn edge(subject: u64, predicate: u64, object: u64) -> NQuin {
    let context = HYPERMEDIA_CONTEXT;
    let metadata = 0;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: NQuin::calculate_parity(subject, predicate, object, context, metadata),
    }
}

/// Emit the **whole relationship edge-graph** of a container as NQuins (+ a lexicon of the string values),
/// in the [`HYPERMEDIA_CONTEXT`] named graph. This is the semantic library's core: the edges *are* the
/// container. Every emitted quin carries valid field parity.
pub fn container_to_nquins(c: &HypermediaContainer) -> (Vec<NQuin>, HashMap<u64, String>) {
    let mut quins = Vec::new();
    let mut lexicon: HashMap<u64, String> = HashMap::new();
    let cs = c.subject();
    lexicon.insert(cs, c.uri.clone());

    // The container node.
    quins.push(edge(cs, P_RDF_TYPE, C_CONTAINER));
    quins.push(edge(cs, P_HAS_PRIMARY, c.primary.subject()));

    // Every asset (primary first, then related) is bundled and described.
    let assets = std::iter::once(&c.primary).chain(c.related.iter());
    for a in assets {
        let asu = a.subject();
        lexicon.insert(asu, a.uri.clone());
        quins.push(edge(cs, P_BUNDLED_WITH, asu));
        quins.push(edge(asu, P_HAS_ROLE, a.role.class()));

        let mt = fnv60(a.media_type.as_bytes());
        lexicon.insert(mt, a.media_type.clone());
        quins.push(edge(asu, P_MEDIA_TYPE, mt));
        quins.push(edge(asu, P_DIGEST, a.digest));

        for src in &a.derived_from {
            let ss = fnv60(src.as_bytes());
            lexicon.insert(ss, src.clone());
            quins.push(edge(asu, P_WAS_DERIVED_FROM, ss));
        }
        if a.role == AssetRole::Provenance {
            quins.push(edge(c.primary.subject(), P_HAS_PROVENANCE, asu));
        }
        if let Some(lic) = &a.licence {
            let lh = fnv60(lic.as_bytes());
            lexicon.insert(lh, lic.clone());
            quins.push(edge(asu, P_LICENCE, lh));
        }
        if let Some(cr) = &a.creator {
            let ch = fnv60(cr.as_bytes());
            lexicon.insert(ch, cr.clone());
            quins.push(edge(asu, P_CREATOR, ch));
        }
    }

    // Analytics bound back to the asset they are *about*.
    for an in &c.analytics {
        let ansu = an.subject();
        lexicon.insert(ansu, an.id.clone());
        let target = if an.target_uri.is_empty() {
            c.primary.subject()
        } else {
            fnv60(an.target_uri.as_bytes())
        };
        quins.push(edge(ansu, P_RDF_TYPE, C_ANALYTICS));
        quins.push(edge(ansu, P_ANALYSIS_TARGET, target));
        quins.push(edge(cs, P_ANALYSIS_RESULT, ansu));
        let mh = fnv60(an.method.as_bytes());
        lexicon.insert(mh, an.method.clone());
        quins.push(edge(ansu, P_ANALYSIS_METHOD, mh));
    }

    (quins, lexicon)
}

// ── Query the graph by *meaning and lineage* (not by path) ──────────────────────────────────────

fn objects(quins: &[NQuin], subject: u64, predicate: u64) -> Vec<u64> {
    quins
        .iter()
        .filter(|q| {
            q.context == HYPERMEDIA_CONTEXT && q.subject == subject && q.predicate == predicate
        })
        .map(|q| q.object)
        .collect()
}

/// The container's primary asset subject.
pub fn primary_of(quins: &[NQuin], container_subject: u64) -> Option<u64> {
    objects(quins, container_subject, P_HAS_PRIMARY)
        .first()
        .copied()
}

/// Every asset subject a container bundles (primary + related).
pub fn bundled(quins: &[NQuin], container_subject: u64) -> Vec<u64> {
    objects(quins, container_subject, P_BUNDLED_WITH)
}

/// The role of an asset within the container.
pub fn role_of(quins: &[NQuin], asset_subject: u64) -> Option<AssetRole> {
    objects(quins, asset_subject, P_HAS_ROLE)
        .first()
        .copied()
        .and_then(AssetRole::from_class)
}

/// The source asset subjects an asset was derived from — its lineage.
pub fn derived_from(quins: &[NQuin], asset_subject: u64) -> Vec<u64> {
    objects(quins, asset_subject, P_WAS_DERIVED_FROM)
}

/// The provenance-record subject bound to an asset, if any.
pub fn provenance_of(quins: &[NQuin], asset_subject: u64) -> Option<u64> {
    objects(quins, asset_subject, P_HAS_PROVENANCE)
        .first()
        .copied()
}

/// The analytics subjects that are *about* a given asset — the derived data belonging to it.
pub fn analytics_for(quins: &[NQuin], asset_subject: u64) -> Vec<u64> {
    quins
        .iter()
        .filter(|q| {
            q.context == HYPERMEDIA_CONTEXT
                && q.predicate == P_ANALYSIS_TARGET
                && q.object == asset_subject
        })
        .map(|q| q.subject)
        .collect()
}

// ── Descriptors (facets): find assets by meaning / time / place / project / content, not by path ──
//
// Ingest *derives* searchability: a processor attaches these facets to an asset, and search is a query over
// the edges — "files about a topic", "what an image depicts", "events in a period" (timeline), "photos at a
// place" (map), "everything for a project", "documents that support a tax/expenses claim". None is a folder.

const P_TOPIC: u64 = q_hash("urn:qualia:hypermedia:topic");
const P_DEPICTS: u64 = q_hash("urn:qualia:hypermedia:depicts");
const P_OCCURRED_AT: u64 = q_hash("urn:qualia:hypermedia:occurredAt");
const P_OCCURRED_START: u64 = q_hash("urn:qualia:hypermedia:occurredStart");
const P_OCCURRED_END: u64 = q_hash("urn:qualia:hypermedia:occurredEnd");
const P_AT_PLACE: u64 = q_hash("urn:qualia:hypermedia:atPlace");
const P_AT_LAT: u64 = q_hash("urn:qualia:hypermedia:atLat");
const P_AT_LON: u64 = q_hash("urn:qualia:hypermedia:atLon");
const P_IN_PROJECT: u64 = q_hash("urn:qualia:hypermedia:inProject");
const P_DOCUMENT_TYPE: u64 = q_hash("urn:qualia:hypermedia:documentType");
const P_PURPOSE: u64 = q_hash("urn:qualia:hypermedia:purpose");
const P_HAS_FLAG: u64 = q_hash("urn:qualia:hypermedia:hasFlag");
const P_FLAG_KIND: u64 = q_hash("urn:qualia:hypermedia:flagKind");
const P_FLAG_SEVERITY: u64 = q_hash("urn:qualia:hypermedia:flagSeverity");
const P_FLAG_DETAIL: u64 = q_hash("urn:qualia:hypermedia:flagDetail");
const C_FLAG: u64 = q_hash("urn:qualia:hypermedia:Flag");

/// A place an asset is bound to — a human label plus coordinates (for the map view).
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub label: String,
    pub lat: f32,
    pub lon: f32,
}

/// The semantic facets that make an asset findable — topic, what it depicts, when/where it happened, which
/// project it belongs to, and its document-type / purpose (e.g. tax-support). All edges; none a folder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Descriptors {
    pub topics: Vec<String>,
    /// Subjects depicted in an image (the "representation of something in the image").
    pub depicts: Vec<String>,
    /// A single event instant (unix seconds) — the timeline anchor.
    pub occurred_at: Option<i64>,
    /// A period the asset covers (unix seconds).
    pub occurred_interval: Option<(i64, i64)>,
    pub place: Option<Place>,
    pub projects: Vec<String>,
    pub document_type: Option<String>,
    /// Purposes the asset serves (e.g. `tax-return-2025`, `expenses-claim`).
    pub purposes: Vec<String>,
}

/// Emit descriptor edges for an asset subject. Each string facet's object is `fnv60(value)` (so a search for
/// that value matches); event times are stored as their `u64` bit pattern for range scans.
pub fn descriptors_to_nquins(
    asset_subject: u64,
    d: &Descriptors,
) -> (Vec<NQuin>, HashMap<u64, String>) {
    let mut quins = Vec::new();
    let mut lex = HashMap::new();
    let str_edge =
        |quins: &mut Vec<NQuin>, lex: &mut HashMap<u64, String>, pred: u64, val: &str| {
            let o = fnv60(val.as_bytes());
            lex.insert(o, val.to_string());
            quins.push(edge(asset_subject, pred, o));
        };
    for t in &d.topics {
        str_edge(&mut quins, &mut lex, P_TOPIC, t);
    }
    for s in &d.depicts {
        str_edge(&mut quins, &mut lex, P_DEPICTS, s);
    }
    for p in &d.projects {
        str_edge(&mut quins, &mut lex, P_IN_PROJECT, p);
    }
    for p in &d.purposes {
        str_edge(&mut quins, &mut lex, P_PURPOSE, p);
    }
    if let Some(dt) = &d.document_type {
        str_edge(&mut quins, &mut lex, P_DOCUMENT_TYPE, dt);
    }
    if let Some(t) = d.occurred_at {
        quins.push(edge(asset_subject, P_OCCURRED_AT, t as u64));
    }
    if let Some((s, e)) = d.occurred_interval {
        quins.push(edge(asset_subject, P_OCCURRED_START, s as u64));
        quins.push(edge(asset_subject, P_OCCURRED_END, e as u64));
    }
    if let Some(pl) = &d.place {
        let lh = fnv60(pl.label.as_bytes());
        lex.insert(lh, pl.label.clone());
        quins.push(edge(asset_subject, P_AT_PLACE, lh));
        quins.push(edge(asset_subject, P_AT_LAT, pack_float_object(pl.lat)));
        quins.push(edge(asset_subject, P_AT_LON, pack_float_object(pl.lon)));
    }
    (quins, lex)
}

fn subjects_with(quins: &[NQuin], predicate: u64, object: u64) -> Vec<u64> {
    quins
        .iter()
        .filter(|q| {
            q.context == HYPERMEDIA_CONTEXT && q.predicate == predicate && q.object == object
        })
        .map(|q| q.subject)
        .collect()
}

/// Assets *about* a topic. (biology / engineering / policy / software / law / …)
pub fn by_topic(quins: &[NQuin], topic: &str) -> Vec<u64> {
    subjects_with(quins, P_TOPIC, fnv60(topic.as_bytes()))
}
/// Assets whose image *depicts* a subject (the "representation in the image").
pub fn by_depiction(quins: &[NQuin], subject: &str) -> Vec<u64> {
    subjects_with(quins, P_DEPICTS, fnv60(subject.as_bytes()))
}
/// Assets at a place (map view).
pub fn by_place(quins: &[NQuin], place_label: &str) -> Vec<u64> {
    subjects_with(quins, P_AT_PLACE, fnv60(place_label.as_bytes()))
}
/// Assets collected under a project.
pub fn in_project(quins: &[NQuin], project: &str) -> Vec<u64> {
    subjects_with(quins, P_IN_PROJECT, fnv60(project.as_bytes()))
}
/// Assets serving a purpose (e.g. `tax-return-2025`, `expenses-claim`).
pub fn for_purpose(quins: &[NQuin], purpose: &str) -> Vec<u64> {
    subjects_with(quins, P_PURPOSE, fnv60(purpose.as_bytes()))
}
/// Assets whose event instant falls within `[start, end]` (unix seconds) — the timeline query.
pub fn in_time_range(quins: &[NQuin], start: i64, end: i64) -> Vec<u64> {
    quins
        .iter()
        .filter(|q| {
            q.context == HYPERMEDIA_CONTEXT
                && q.predicate == P_OCCURRED_AT
                && (q.object as i64) >= start
                && (q.object as i64) <= end
        })
        .map(|q| q.subject)
        .collect()
}

/// Severity of an ingest flag. Higher = more likely to warrant a guardian notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagSeverity {
    Info,
    Notice,
    Concern,
    Urgent,
}
impl FlagSeverity {
    pub fn level(self) -> u64 {
        match self {
            FlagSeverity::Info => 0,
            FlagSeverity::Notice => 1,
            FlagSeverity::Concern => 2,
            FlagSeverity::Urgent => 3,
        }
    }
}

/// A flag raised while processing an ingested asset (e.g. concerning content). The flag is a **semantic
/// descriptor** on the asset; if the principal is under a guardianship relation, the ingest path
/// (client-core / host) reads these and notifies the guardian (and records who was notified — the
/// accountability fabric). Defining flags here keeps them queryable; the notification wiring lives where
/// guardianship + notifications do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    pub kind: String,
    pub severity: FlagSeverity,
    pub detail: String,
}

/// Emit flag edges bound to an asset subject.
pub fn flags_to_nquins(
    asset_subject: u64,
    asset_uri: &str,
    flags: &[Flag],
) -> (Vec<NQuin>, HashMap<u64, String>) {
    let mut quins = Vec::new();
    let mut lex = HashMap::new();
    for f in flags {
        let fs = fnv60(format!("{asset_uri}#flag:{}", f.kind).as_bytes());
        lex.insert(fs, format!("{asset_uri}#flag:{}", f.kind));
        quins.push(edge(asset_subject, P_HAS_FLAG, fs));
        quins.push(edge(fs, P_RDF_TYPE, C_FLAG));
        let kh = fnv60(f.kind.as_bytes());
        lex.insert(kh, f.kind.clone());
        quins.push(edge(fs, P_FLAG_KIND, kh));
        quins.push(edge(fs, P_FLAG_SEVERITY, f.severity.level()));
        if !f.detail.is_empty() {
            let dh = fnv60(f.detail.as_bytes());
            lex.insert(dh, f.detail.clone());
            quins.push(edge(fs, P_FLAG_DETAIL, dh));
        }
    }
    (quins, lex)
}

/// The flag subjects raised on an asset — what the guardian-notify path reads.
pub fn flags_on(quins: &[NQuin], asset_subject: u64) -> Vec<u64> {
    objects(quins, asset_subject, P_HAS_FLAG)
}
/// A flag's severity level (0 Info .. 3 Urgent), if present.
pub fn flag_severity(quins: &[NQuin], flag_subject: u64) -> Option<u64> {
    objects(quins, flag_subject, P_FLAG_SEVERITY)
        .first()
        .copied()
}

// ── Ingest processors: ingest DERIVES searchability (P3) ─────────────────────────────────────────
//
// A document/image/asset goes in; a processor produces the derived searchable files (text / transcript /
// depicted-subjects / thumbnail) + descriptor facets + any flags — which fold into the asset's container so
// the *original* becomes findable. Heavy content processors (image→depicted-subjects/OCR, audio→transcript)
// compose the parked `qualia-vision` / `qualia-audio` engines; this is the framework + a real model-free
// text processor. (§11: this module is ~800 lines — split `hypermedia.rs` → `hypermedia/{container,descriptors,
// processors}.rs` in a dedicated library-ization pass.)

/// What a processor derives from an ingested asset — the searchable representations + facets + flags.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcessorOutput {
    /// Derived searchable assets (role Derivation / Analysis) to bundle into the container.
    pub derived: Vec<AssetRef>,
    /// The bytes of each derived asset, keyed by its uri (e.g. the extracted plain text).
    pub derived_bytes: HashMap<String, Vec<u8>>,
    /// Descriptor facets extracted (topics, depicts, …) — bound to the primary asset.
    pub descriptors: Descriptors,
    /// Flags raised (→ the guardian-notify path when the principal is under guardianship).
    pub flags: Vec<Flag>,
}

/// A processor: ingest an asset (bytes + media-type) → derive its searchable representations + facets.
pub trait Processor {
    /// Whether this processor handles the given media type.
    fn handles(&self, media_type: &str) -> bool;
    /// Derive searchable content + descriptors + flags from the asset.
    fn process(&self, asset_uri: &str, bytes: &[u8], media_type: &str) -> ProcessorOutput;
}

/// A real, **model-free text / markdown** processor: derives a plain-text representation, assigns **topics**
/// from a keyword map (biology / engineering / policy / software / law / finance-for-tax-&-expenses), and
/// raises a **flag** for any watch-word present. Proves "ingest derives searchability" with no model runtime;
/// the semantic-content processors compose `qualia-vision` / `qualia-audio`.
pub struct TextProcessor {
    /// topic → trigger words (any present ⇒ the topic is assigned).
    pub topic_keywords: Vec<(String, Vec<String>)>,
    /// watch-word → (flag kind, severity) (any present ⇒ a flag is raised).
    pub flag_words: Vec<(String, (String, FlagSeverity))>,
}

impl Default for TextProcessor {
    fn default() -> Self {
        let kw = |t: &str, ws: &[&str]| (t.to_string(), ws.iter().map(|s| s.to_string()).collect());
        Self {
            topic_keywords: vec![
                kw(
                    "biology",
                    &[
                        "cell",
                        "organ",
                        "gene",
                        "protein",
                        "hepatocyte",
                        "liver",
                        "anatomy",
                    ],
                ),
                kw(
                    "engineering",
                    &["stress", "load", "circuit", "tolerance", "mechanical"],
                ),
                kw(
                    "policy",
                    &["policy", "regulation", "governance", "legislation"],
                ),
                kw(
                    "software",
                    &["function", "compiler", "api", "struct", "runtime"],
                ),
                kw(
                    "law",
                    &["contract", "statute", "liability", "clause", "jurisdiction"],
                ),
                kw(
                    "finance",
                    &["invoice", "expense", "tax", "receipt", "deduction"],
                ),
            ],
            flag_words: Vec::new(),
        }
    }
}

impl Processor for TextProcessor {
    fn handles(&self, media_type: &str) -> bool {
        media_type.starts_with("text/")
    }

    fn process(&self, asset_uri: &str, bytes: &[u8], _media_type: &str) -> ProcessorOutput {
        let text = String::from_utf8_lossy(bytes).to_lowercase();
        let mut topics = Vec::new();
        for (topic, words) in &self.topic_keywords {
            if words.iter().any(|w| text.contains(&w.to_lowercase())) {
                topics.push(topic.clone());
            }
        }
        let mut flags = Vec::new();
        for (word, (kind, sev)) in &self.flag_words {
            if text.contains(&word.to_lowercase()) {
                flags.push(Flag {
                    kind: kind.clone(),
                    severity: *sev,
                    detail: format!("matched '{word}'"),
                });
            }
        }
        // A plain-text derivation of the original (what makes it searchable), derived from the primary.
        let text_uri = format!("{asset_uri}#text");
        let derived =
            vec![
                AssetRef::new(&text_uri, fnv60(bytes), "text/plain", AssetRole::Derivation)
                    .derived_from(asset_uri),
            ];
        let mut derived_bytes = HashMap::new();
        derived_bytes.insert(text_uri, bytes.to_vec());
        ProcessorOutput {
            derived,
            derived_bytes,
            descriptors: Descriptors {
                topics,
                ..Default::default()
            },
            flags,
        }
    }
}

/// The result of ingesting an asset through a processor: the container, its quin graph (edges + descriptors +
/// flags), the lexicon, and the flags (which the caller checks against a guardianship relation).
pub struct IngestResult {
    pub container: HypermediaContainer,
    pub quins: Vec<NQuin>,
    pub lexicon: HashMap<u64, String>,
    pub flags: Vec<Flag>,
}

/// **Ingest an asset through a processor** and fold its output into a fresh container — the original plus its
/// derived searchable representations, its facets, and any flags, all as edges. `digest` is the primary
/// asset's content digest.
pub fn ingest_with(
    processor: &dyn Processor,
    asset_uri: &str,
    media_type: &str,
    digest: u64,
    bytes: &[u8],
) -> IngestResult {
    let out = processor.process(asset_uri, bytes, media_type);
    let primary = AssetRef::new(asset_uri, digest, media_type, AssetRole::Primary);
    let mut container = HypermediaContainer::new(format!("{asset_uri}#container"), primary.clone());
    for d in &out.derived {
        container = container.with_related(d.clone());
    }
    let (mut quins, mut lexicon) = container_to_nquins(&container);
    let (dq, dl) = descriptors_to_nquins(primary.subject(), &out.descriptors);
    quins.extend(dq);
    for (k, v) in dl {
        lexicon.entry(k).or_insert(v);
    }
    let (fq, fl) = flags_to_nquins(primary.subject(), asset_uri, &out.flags);
    quins.extend(fq);
    for (k, v) in fl {
        lexicon.entry(k).or_insert(v);
    }
    IngestResult {
        container,
        quins,
        lexicon,
        flags: out.flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a real anatomy-shaped container (an organ mesh ⊕ its source GLB ⊕ a systemic-burden analysis ⊕
    /// a provenance record) and read every relationship back out of the quin graph — proving the container is
    /// a **semantic graph of edges**, not a directory.
    #[test]
    fn container_round_trips_as_a_semantic_graph_not_a_directory() {
        let primary = AssetRef::new(
            "urn:qualia:organ:liver.10d",
            0xABCD,
            "model/qualia-10d",
            AssetRole::Primary,
        )
        .derived_from("urn:hra:ccf:liver.glb");
        let source = AssetRef::new(
            "urn:hra:ccf:liver.glb",
            0x1234,
            "model/gltf-binary",
            AssetRole::Source,
        )
        .with_licence("CC-BY-4.0");
        let provenance = AssetRef::new(
            "urn:qualia:prov:liver",
            0x5555,
            "application/ld+json",
            AssetRole::Provenance,
        );
        let analysis = AnalyticsRef {
            id: "urn:qualia:analysis:liver-burden".into(),
            method: "wellfare:systemic-burden".into(),
            target_uri: String::new(), // → the primary
            summary: r#"{"digestive":420,"circulatory":180}"#.into(),
        };

        let c = HypermediaContainer::new("urn:qualia:container:liver", primary.clone())
            .with_related(source.clone())
            .with_related(provenance.clone())
            .with_analytics(analysis.clone());

        let (quins, _lex) = container_to_nquins(&c);
        assert!(
            quins.iter().all(|q| q.verify_ecc_parity()),
            "every emitted quin has valid parity"
        );

        let cs = c.subject();
        // The primary edge resolves to the SAME subject the asset's own manifest would use.
        assert_eq!(primary_of(&quins, cs), Some(primary.subject()));
        // The container bundles all three assets.
        let bundled = bundled(&quins, cs);
        assert_eq!(bundled.len(), 3);
        assert!(bundled.contains(&source.subject()) && bundled.contains(&provenance.subject()));
        // Roles are readable per asset.
        assert_eq!(role_of(&quins, primary.subject()), Some(AssetRole::Primary));
        assert_eq!(role_of(&quins, source.subject()), Some(AssetRole::Source));
        // Lineage: the primary was derived from the source GLB.
        assert_eq!(
            derived_from(&quins, primary.subject()),
            vec![source.subject()]
        );
        // Provenance is bound to the primary.
        assert_eq!(
            provenance_of(&quins, primary.subject()),
            Some(provenance.subject())
        );
        // The analysis is bound *back to the mesh it is about* — not a sibling file, an edge.
        assert_eq!(
            analytics_for(&quins, primary.subject()),
            vec![analysis.subject()]
        );
    }

    #[test]
    fn role_class_round_trips() {
        for r in [
            AssetRole::Primary,
            AssetRole::Source,
            AssetRole::Derivation,
            AssetRole::Lod,
            AssetRole::Analysis,
            AssetRole::Provenance,
            AssetRole::Related,
        ] {
            assert_eq!(AssetRole::from_class(r.class()), Some(r));
        }
    }

    #[test]
    fn subject_hash_matches_the_asset_identity_space() {
        // A container's reference to a URI hashes to the same 60-bit FNV subject that render/assets uses,
        // so container edges join to the asset's own geometry facts.
        let a = AssetRef::new(
            "urn:qualia:organ:heart.10d",
            1,
            "model/qualia-10d",
            AssetRole::Primary,
        );
        assert_eq!(
            a.subject() & 0xF000_0000_0000_0000,
            0,
            "subject stays in the 60-bit identity space"
        );
        assert_eq!(a.subject(), fnv60(b"urn:qualia:organ:heart.10d"));
    }

    #[test]
    fn descriptors_make_assets_findable_by_facet_not_folder() {
        let liver = fnv60(b"urn:qualia:organ:liver.10d");
        let heart = fnv60(b"urn:qualia:organ:heart.10d");
        let d_liver = Descriptors {
            topics: vec!["biology".into(), "anatomy".into()],
            projects: vec!["med-course".into()],
            purposes: vec!["study".into()],
            occurred_at: Some(1_700_000_000),
            place: Some(Place {
                label: "Sydney".into(),
                lat: -33.87,
                lon: 151.21,
            }),
            ..Default::default()
        };
        let d_heart = Descriptors {
            topics: vec!["biology".into()],
            occurred_at: Some(1_700_100_000),
            ..Default::default()
        };
        let (mut q, _) = descriptors_to_nquins(liver, &d_liver);
        let (q2, _) = descriptors_to_nquins(heart, &d_heart);
        q.extend(q2);
        assert!(
            q.iter().all(|x| x.verify_ecc_parity()),
            "descriptor quins have valid parity"
        );

        // By topic: both are biology; only the liver is anatomy — search by MEANING, not path.
        let bio = by_topic(&q, "biology");
        assert!(bio.contains(&liver) && bio.contains(&heart));
        assert_eq!(by_topic(&q, "anatomy"), vec![liver]);
        // By project / place / purpose.
        assert_eq!(in_project(&q, "med-course"), vec![liver]);
        assert_eq!(by_place(&q, "Sydney"), vec![liver]);
        assert_eq!(for_purpose(&q, "study"), vec![liver]);
        // Timeline: a window that excludes the liver's instant catches only the heart.
        assert_eq!(in_time_range(&q, 1_700_050_000, 1_700_200_000), vec![heart]);
    }

    #[test]
    fn a_flag_is_bound_to_the_asset_for_the_guardian_path() {
        let uri = "urn:qualia:doc:xray.10d";
        let asset = fnv60(uri.as_bytes());
        let (q, _) = flags_to_nquins(
            asset,
            uri,
            &[Flag {
                kind: "sensitive-medical".into(),
                severity: FlagSeverity::Concern,
                detail: "radiograph".into(),
            }],
        );
        assert!(q.iter().all(|x| x.verify_ecc_parity()));
        let flags = flags_on(&q, asset);
        assert_eq!(
            flags.len(),
            1,
            "the flag is bound to the asset (what the guardian-notify path reads)"
        );
        assert_eq!(flag_severity(&q, flags[0]), Some(2), "Concern = level 2");
    }

    #[test]
    fn text_processor_derives_topics_and_a_searchable_text_derivation() {
        let proc = TextProcessor::default();
        let doc = b"The human liver is an organ; hepatocytes secrete bile.";
        let out = proc.process("urn:doc:liver-notes", doc, "text/markdown");
        assert!(
            out.descriptors.topics.contains(&"biology".to_string()),
            "topic derived from content"
        );
        assert_eq!(
            out.derived.len(),
            1,
            "a searchable text derivation is produced"
        );
        assert_eq!(out.derived[0].role, AssetRole::Derivation);
    }

    #[test]
    fn ingest_makes_the_original_findable_and_raises_a_flag() {
        // A processor with a topic (law) and a watch-word that raises a flag (for the guardian path).
        let proc = TextProcessor {
            topic_keywords: vec![("law".into(), vec!["contract".into(), "statute".into()])],
            flag_words: vec![(
                "confidential".into(),
                ("sensitive".into(), FlagSeverity::Concern),
            )],
        };
        let doc = b"This CONFIDENTIAL contract is governed by statute.";
        let r = ingest_with(&proc, "urn:doc:nda", "text/plain", 0xAA, doc);
        let primary = r.container.primary.subject();
        // The original is now findable by meaning.
        assert!(
            by_topic(&r.quins, "law").contains(&primary),
            "original findable by derived topic"
        );
        // The flag is raised AND bound to the asset — the guardian-notify path reads it.
        assert_eq!(r.flags.len(), 1);
        assert!(!flags_on(&r.quins, primary).is_empty());
        assert!(r.quins.iter().all(|q| q.verify_ecc_parity()));
    }
}
