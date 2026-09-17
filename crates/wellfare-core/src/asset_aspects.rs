//! Asset aspect graphs — sub-graphs for assets at spatial locations with
//! multiple temporal assertions (T71+ / user request 2026-08-20).
//!
//! An asset (recording, venue, event space, artwork, document, etc.) can be
//! anchored to a spatial location and have multiple named temporal assertions:
//! production date, recording date, publication date, event date, performance
//! date, etc. Each assertion has its own provenance, confidence, and evidence
//! type. These form a sub-graph that illustrates different aspects of the
//! asset.
//!
//! The asset aspect graph also supports topic/concept associations — different
//! topics or concepts in relation to different subjects — so the sub-graph can
//! express relationships like "this recording is about topic X, performed at
//! venue Y, published by subject Z."
//!
//! Architecture:
//! - `TemporalAspect` — a single named temporal assertion (e.g., "production_date")
//! - `AssetAspectGraph` — the full sub-graph for an asset
//! - `TopicAssociation` — a topic/concept in relation to a subject
//! - `SpatialAnchorLite` — lightweight geodetic position (no vibe dependency)
//!
//! The `SpatialAnchorLite` type mirrors the essential fields of
//! `vibe::cosmic::ar::SpatialAnchor` without pulling in vibe as a
//! hard dependency. Under the `qualia` feature, it converts to/from
//! `SpatialAnchor`.

use crate::record::{DurationBridge, EvidenceType, InstantBridge, NQuin, q_hash_str};
use serde::{Deserialize, Serialize};

// ── Spatial anchor lite ───────────────────────────────────────────────────

/// Lightweight geodetic spatial anchor for assets (no vibe dependency).
///
/// Stores WGS84 [lat, lon, alt] plus an optional ENU offset and confidence
/// radius. Under the `qualia` feature, converts to/from
/// `vibe::cosmic::ar::SpatialAnchor`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialAnchorLite {
    /// Unique anchor identifier
    pub anchor_id: String,
    /// WGS84 geodetic position [lat_deg, lon_deg, alt_m]
    pub geodetic: [f64; 3],
    /// Local ENU offset [East, North, Up] in meters
    #[serde(default)]
    pub enu_offset: [f32; 3],
    /// Confidence radius in millimeters
    #[serde(default = "default_confidence_mm")]
    pub confidence_radius_mm: f32,
}

fn default_confidence_mm() -> f32 {
    10.0
}

impl SpatialAnchorLite {
    /// Create a new spatial anchor at a geodetic position.
    pub fn new(anchor_id: &str, lat: f64, lon: f64, alt: f64) -> Self {
        Self {
            anchor_id: anchor_id.into(),
            geodetic: [lat, lon, alt],
            enu_offset: [0.0, 0.0, 0.0],
            confidence_radius_mm: 10.0,
        }
    }

    /// Set ENU offset.
    pub fn with_enu_offset(mut self, east: f32, north: f32, up: f32) -> Self {
        self.enu_offset = [east, north, up];
        self
    }

    /// Set confidence radius in mm.
    pub fn with_confidence(mut self, radius_mm: f32) -> Self {
        self.confidence_radius_mm = radius_mm;
        self
    }

    /// Whether this anchor is sub-millimeter precision.
    pub fn is_submillimeter(&self) -> bool {
        self.confidence_radius_mm < 1.0
    }
}

// ── Temporal aspect ───────────────────────────────────────────────────────

/// The kind of temporal assertion associated with an asset.
///
/// Assets can have multiple temporal aspects — e.g., a recording has both a
/// production date (when it was made) and a publication date (when it was
/// released). A venue has event dates. An artwork has creation and exhibition
/// dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemporalAspectKind {
    /// When the asset was produced/created
    Production,
    /// When the asset was recorded
    Recording,
    /// When the asset was published/released
    Publication,
    /// When an event occurred (venue, event space)
    Event,
    /// When a performance took place
    Performance,
    /// When the asset was exhibited
    Exhibition,
    /// When the asset was modified
    Modification,
    /// When the asset was archived
    Archival,
    /// When the asset was acquired/obtained
    Acquisition,
    /// When the asset was decommissioned/retired
    Decommission,
    /// A custom named temporal aspect
    Custom,
}

impl TemporalAspectKind {
    /// Canonical string representation for graph predicates.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Production => "q42:productionDate",
            Self::Recording => "q42:recordingDate",
            Self::Publication => "q42:publicationDate",
            Self::Event => "q42:eventDate",
            Self::Performance => "q42:performanceDate",
            Self::Exhibition => "q42:exhibitionDate",
            Self::Modification => "q42:modificationDate",
            Self::Archival => "q42:archivalDate",
            Self::Acquisition => "q42:acquisitionDate",
            Self::Decommission => "q42:decommissionDate",
            Self::Custom => "q42:customDate",
        }
    }
}

/// A single named temporal assertion about an asset.
///
/// Each aspect has its own instant, optional duration (for events/
/// performances), provenance (who asserted it), confidence, and evidence type.
/// This allows multiple conflicting or complementary temporal assertions to
/// coexist in the same sub-graph — e.g., a recording might have a production
/// date asserted by the producer and a different one asserted by a historian.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalAspect {
    /// The kind of temporal assertion
    pub kind: TemporalAspectKind,
    /// Custom label for Custom kind (ignored for other kinds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_label: Option<String>,
    /// When this aspect occurred
    pub instant: InstantBridge,
    /// Optional duration (for events, performances, exhibitions)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<DurationBridge>,
    /// Who asserted this temporal aspect (DID)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asserted_by: Option<String>,
    /// Confidence in this assertion [0.0, 1.0]
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    /// How this temporal assertion was determined
    pub evidence_type: EvidenceType,
}

fn default_confidence() -> f64 {
    1.0
}

impl TemporalAspect {
    /// Create a new temporal aspect.
    pub fn new(kind: TemporalAspectKind, instant: InstantBridge) -> Self {
        Self {
            kind,
            custom_label: None,
            instant,
            duration: None,
            asserted_by: None,
            confidence: 1.0,
            evidence_type: EvidenceType::SelfReported,
        }
    }

    /// Set a custom label (for Custom kind).
    pub fn with_label(mut self, label: &str) -> Self {
        self.custom_label = Some(label.into());
        self
    }

    /// Set the duration.
    pub fn with_duration(mut self, duration: DurationBridge) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set who asserted this.
    pub fn with_asserted_by(mut self, did: &str) -> Self {
        self.asserted_by = Some(did.into());
        self
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set evidence type.
    pub fn with_evidence(mut self, evidence: EvidenceType) -> Self {
        self.evidence_type = evidence;
        self
    }

    /// The predicate string for this aspect (for graph Quins).
    pub fn predicate(&self) -> &str {
        self.kind.as_str()
    }

    /// Compile this temporal aspect to a graph Quin.
    /// The subject is the asset ID hash, the predicate is the aspect kind,
    /// the object is the Unix nanoseconds, and the context is the asset's
    /// sub-graph context hash.
    pub fn to_quin(&self, asset_id_hash: u64, context_hash: u64) -> NQuin {
        let pred_hash = q_hash_str(self.kind.as_str());
        let obj = self.instant.to_unix_nanos() as u64;
        let metadata = (self.confidence * 1e6) as u64; // Pack confidence into metadata
        NQuin::new(asset_id_hash, pred_hash, obj, context_hash, metadata)
    }
}

// ── Topic association ─────────────────────────────────────────────────────

/// A topic/concept in relation to a subject within an asset sub-graph.
///
/// This allows the sub-graph to express relationships like "this recording
/// is about topic X" or "this venue is associated with subject Y" — where
/// the topic and subject can be any IRI or string identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicAssociation {
    /// The topic or concept (IRI or string)
    pub topic: String,
    /// The subject the topic relates to (IRI or string)
    pub subject: String,
    /// The relationship type (e.g., "q42:isAbout", "q42:associatedWith")
    pub relation: String,
    /// Optional confidence in this association
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

impl TopicAssociation {
    /// Create a new topic association.
    pub fn new(topic: &str, subject: &str, relation: &str) -> Self {
        Self {
            topic: topic.into(),
            subject: subject.into(),
            relation: relation.into(),
            confidence: 1.0,
        }
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Compile this topic association to a graph Quin.
    pub fn to_quin(&self, asset_id_hash: u64, context_hash: u64) -> NQuin {
        let pred_hash = q_hash_str(&self.relation);
        let obj_hash = q_hash_str(&self.topic);
        let metadata = (self.confidence * 1e6) as u64;
        NQuin::new(asset_id_hash, pred_hash, obj_hash, context_hash, metadata)
    }
}

// ── Asset aspect graph ────────────────────────────────────────────────────

/// The kind of asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetKind {
    Recording,
    Venue,
    EventSpace,
    Artwork,
    Document,
    Photograph,
    Performance,
    Artifact,
    Location,
    Custom,
}

impl AssetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Recording => "q42:Recording",
            Self::Venue => "q42:Venue",
            Self::EventSpace => "q42:EventSpace",
            Self::Artwork => "q42:Artwork",
            Self::Document => "q42:Document",
            Self::Photograph => "q42:Photograph",
            Self::Performance => "q42:Performance",
            Self::Artifact => "q42:Artifact",
            Self::Location => "q42:Location",
            Self::Custom => "q42:Asset",
        }
    }
}

/// A sub-graph for an asset at a spatial location with multiple temporal
/// assertions and topic/concept associations.
///
/// This is the core type for the "asset sub-graph" concept. Each asset gets
/// its own sub-graph (identified by a context hash) that contains:
/// - A spatial anchor (where the asset is/was)
/// - Multiple temporal aspects (when different things happened to it)
/// - Topic/concept associations (what the asset is about)
/// - Graph Quins (the compiled semantic graph entries)
///
/// Example: A live recording at a venue might have:
/// - Spatial anchor: the venue's geodetic position
/// - Temporal aspects:
///   - Production date (when the recording was made)
///   - Publication date (when it was released)
///   - Event date (when the live performance happened)
/// - Topic associations:
///   - "jazz music" isAbout "this recording"
///   - "Carnegie Hall" associatedWith "this recording"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetAspectGraph {
    /// Unique asset identifier
    pub asset_id: String,
    /// The kind of asset
    pub asset_kind: AssetKind,
    /// Optional spatial anchor (where the asset is/was)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial_anchor: Option<SpatialAnchorLite>,
    /// Multiple temporal assertions about this asset
    pub temporal_aspects: Vec<TemporalAspect>,
    /// Topic/concept associations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<TopicAssociation>,
    /// Optional owner DID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_did: Option<String>,
}

impl AssetAspectGraph {
    /// Create a new asset aspect graph.
    pub fn new(asset_id: &str, asset_kind: AssetKind) -> Self {
        Self {
            asset_id: asset_id.into(),
            asset_kind,
            spatial_anchor: None,
            temporal_aspects: Vec::new(),
            topics: Vec::new(),
            owner_did: None,
        }
    }

    /// Set the spatial anchor.
    pub fn with_spatial_anchor(mut self, anchor: SpatialAnchorLite) -> Self {
        self.spatial_anchor = Some(anchor);
        self
    }

    /// Add a temporal aspect.
    pub fn with_temporal_aspect(mut self, aspect: TemporalAspect) -> Self {
        self.temporal_aspects.push(aspect);
        self
    }

    /// Add a topic association.
    pub fn with_topic(mut self, topic: TopicAssociation) -> Self {
        self.topics.push(topic);
        self
    }

    /// Set the owner DID.
    pub fn with_owner(mut self, did: &str) -> Self {
        self.owner_did = Some(did.into());
        self
    }

    /// The context hash for this asset's sub-graph.
    /// This is derived from the asset ID and kind, so the same asset always
    /// maps to the same sub-graph context.
    pub fn context_hash(&self) -> u64 {
        let mut h = q_hash_str(self.asset_kind.as_str());
        h ^= q_hash_str(&self.asset_id);
        h = h.wrapping_mul(0x100000001b3);
        h
    }

    /// The asset ID hash (used as the subject in graph Quins).
    pub fn asset_id_hash(&self) -> u64 {
        q_hash_str(&self.asset_id)
    }

    /// Compile the full sub-graph to Quins.
    /// Returns a vector of NQuins representing all temporal aspects and
    /// topic associations in this asset's sub-graph.
    pub fn to_quins(&self) -> Vec<NQuin> {
        let asset_hash = self.asset_id_hash();
        let ctx_hash = self.context_hash();
        let mut quins = Vec::new();

        // Temporal aspect Quins
        for aspect in &self.temporal_aspects {
            quins.push(aspect.to_quin(asset_hash, ctx_hash));
        }

        // Topic association Quins
        for topic in &self.topics {
            quins.push(topic.to_quin(asset_hash, ctx_hash));
        }

        // Spatial anchor Quin (if present)
        if let Some(ref anchor) = self.spatial_anchor {
            let pred_hash = q_hash_str("q42:hasSpatialAnchor");
            let obj_hash = q_hash_str(&anchor.anchor_id);
            quins.push(NQuin::new(asset_hash, pred_hash, obj_hash, ctx_hash, 0));
        }

        // Asset kind Quin
        let kind_pred = q_hash_str("q42:hasAssetKind");
        let kind_obj = q_hash_str(self.asset_kind.as_str());
        quins.push(NQuin::new(asset_hash, kind_pred, kind_obj, ctx_hash, 0));

        quins
    }

    /// Find all temporal aspects of a specific kind.
    pub fn aspects_of_kind(&self, kind: TemporalAspectKind) -> Vec<&TemporalAspect> {
        self.temporal_aspects
            .iter()
            .filter(|a| a.kind == kind)
            .collect()
    }

    /// The earliest temporal aspect (by instant).
    pub fn earliest_aspect(&self) -> Option<&TemporalAspect> {
        self.temporal_aspects
            .iter()
            .min_by_key(|a| a.instant.to_unix_nanos())
    }

    /// The latest temporal aspect (by instant).
    pub fn latest_aspect(&self) -> Option<&TemporalAspect> {
        self.temporal_aspects
            .iter()
            .max_by_key(|a| a.instant.to_unix_nanos())
    }

    /// The duration between the earliest and latest temporal aspects.
    /// Useful for computing the span between production and publication, etc.
    pub fn temporal_span(&self) -> Option<DurationBridge> {
        let earliest = self.earliest_aspect()?;
        let latest = self.latest_aspect()?;
        Some(latest.instant.duration_since(&earliest.instant))
    }

    /// Find all topics with a specific relation.
    pub fn topics_with_relation(&self, relation: &str) -> Vec<&TopicAssociation> {
        self.topics
            .iter()
            .filter(|t| t.relation == relation)
            .collect()
    }

    /// Find all topics about a specific subject.
    pub fn topics_about(&self, subject: &str) -> Vec<&TopicAssociation> {
        self.topics
            .iter()
            .filter(|t| t.subject == subject)
            .collect()
    }
}

// ── vibe integration (qualia feature) ────────────────────────────────

#[cfg(feature = "qualia")]
impl From<&SpatialAnchorLite> for vibe::cosmic::ar::SpatialAnchor {
    fn from(lite: &SpatialAnchorLite) -> Self {
        vibe::cosmic::ar::SpatialAnchor::new(&lite.anchor_id, lite.geodetic)
            .with_enu_offset(lite.enu_offset[0], lite.enu_offset[1], lite.enu_offset[2])
            .with_confidence(lite.confidence_radius_mm)
    }
}

#[cfg(feature = "qualia")]
impl From<&vibe::cosmic::ar::SpatialAnchor> for SpatialAnchorLite {
    fn from(anchor: &vibe::cosmic::ar::SpatialAnchor) -> Self {
        Self {
            anchor_id: anchor.anchor_id.clone(),
            geodetic: anchor.geodetic_anchor,
            enu_offset: anchor.enu_offset,
            confidence_radius_mm: anchor.confidence_radius_mm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_anchor_lite_basic() {
        let a = SpatialAnchorLite::new("venue-1", 40.7128, -74.0060, 10.0);
        assert_eq!(a.anchor_id, "venue-1");
        assert_eq!(a.geodetic, [40.7128, -74.0060, 10.0]);
        assert_eq!(a.enu_offset, [0.0, 0.0, 0.0]);
        assert!(!a.is_submillimeter());
    }

    #[test]
    fn spatial_anchor_lite_submillimeter() {
        let a = SpatialAnchorLite::new("precise-1", 0.0, 0.0, 0.0).with_confidence(0.5);
        assert!(a.is_submillimeter());
    }

    #[test]
    fn temporal_aspect_kind_strings() {
        assert_eq!(
            TemporalAspectKind::Production.as_str(),
            "q42:productionDate"
        );
        assert_eq!(
            TemporalAspectKind::Publication.as_str(),
            "q42:publicationDate"
        );
        assert_eq!(TemporalAspectKind::Event.as_str(), "q42:eventDate");
        assert_eq!(
            TemporalAspectKind::Performance.as_str(),
            "q42:performanceDate"
        );
    }

    #[test]
    fn temporal_aspect_basic() {
        let instant = InstantBridge::unix(1_700_000_000, 0);
        let aspect = TemporalAspect::new(TemporalAspectKind::Production, instant);
        assert_eq!(aspect.kind, TemporalAspectKind::Production);
        assert_eq!(aspect.instant.secs, 1_700_000_000);
        assert_eq!(aspect.confidence, 1.0);
        assert!(aspect.duration.is_none());
        assert!(aspect.asserted_by.is_none());
    }

    #[test]
    fn temporal_aspect_with_fields() {
        let instant = InstantBridge::unix(1_700_000_000, 0);
        let duration = DurationBridge::from_secs(7200); // 2 hours
        let aspect = TemporalAspect::new(TemporalAspectKind::Performance, instant)
            .with_duration(duration)
            .with_asserted_by("did:alice")
            .with_confidence(0.95)
            .with_evidence(EvidenceType::ClinicianObserved);
        assert_eq!(aspect.duration, Some(duration));
        assert_eq!(aspect.asserted_by, Some("did:alice".into()));
        assert!((aspect.confidence - 0.95).abs() < 0.001);
        assert_eq!(aspect.evidence_type, EvidenceType::ClinicianObserved);
    }

    #[test]
    fn temporal_aspect_to_quin() {
        let instant = InstantBridge::unix(1_700_000_000, 0);
        let aspect = TemporalAspect::new(TemporalAspectKind::Publication, instant);
        let asset_hash = q_hash_str("asset-1");
        let ctx_hash = q_hash_str("ctx-1");
        let quin = aspect.to_quin(asset_hash, ctx_hash);
        assert_eq!(quin.subject, asset_hash);
        assert_eq!(quin.object, 1_700_000_000_000_000_000);
        assert_eq!(quin.context, ctx_hash);
    }

    #[test]
    fn topic_association_basic() {
        let t = TopicAssociation::new("jazz", "recording-1", "q42:isAbout");
        assert_eq!(t.topic, "jazz");
        assert_eq!(t.subject, "recording-1");
        assert_eq!(t.relation, "q42:isAbout");
        assert_eq!(t.confidence, 1.0);
    }

    #[test]
    fn topic_association_with_confidence() {
        let t = TopicAssociation::new("jazz", "recording-1", "q42:isAbout").with_confidence(0.8);
        assert!((t.confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn topic_association_to_quin() {
        let t = TopicAssociation::new("jazz", "recording-1", "q42:isAbout");
        let asset_hash = q_hash_str("asset-1");
        let ctx_hash = q_hash_str("ctx-1");
        let quin = t.to_quin(asset_hash, ctx_hash);
        assert_eq!(quin.subject, asset_hash);
        assert_eq!(quin.context, ctx_hash);
    }

    #[test]
    fn asset_aspect_graph_basic() {
        let graph = AssetAspectGraph::new("recording-1", AssetKind::Recording);
        assert_eq!(graph.asset_id, "recording-1");
        assert_eq!(graph.asset_kind, AssetKind::Recording);
        assert!(graph.spatial_anchor.is_none());
        assert!(graph.temporal_aspects.is_empty());
        assert!(graph.topics.is_empty());
    }

    #[test]
    fn asset_aspect_graph_with_spatial_anchor() {
        let anchor = SpatialAnchorLite::new("venue-1", 40.7128, -74.0060, 10.0);
        let graph =
            AssetAspectGraph::new("recording-1", AssetKind::Recording).with_spatial_anchor(anchor);
        assert!(graph.spatial_anchor.is_some());
        assert_eq!(graph.spatial_anchor.as_ref().unwrap().anchor_id, "venue-1");
    }

    #[test]
    fn asset_aspect_graph_with_multiple_temporal_aspects() {
        let production = InstantBridge::unix(1_699_000_000, 0);
        let publication = InstantBridge::unix(1_700_000_000, 0);
        let event = InstantBridge::unix(1_699_500_000, 0);

        let graph = AssetAspectGraph::new("recording-1", AssetKind::Recording)
            .with_temporal_aspect(TemporalAspect::new(
                TemporalAspectKind::Production,
                production,
            ))
            .with_temporal_aspect(TemporalAspect::new(
                TemporalAspectKind::Publication,
                publication,
            ))
            .with_temporal_aspect(TemporalAspect::new(TemporalAspectKind::Event, event));

        assert_eq!(graph.temporal_aspects.len(), 3);
        assert_eq!(
            graph.aspects_of_kind(TemporalAspectKind::Production).len(),
            1
        );
        assert_eq!(
            graph.aspects_of_kind(TemporalAspectKind::Publication).len(),
            1
        );
        assert_eq!(graph.aspects_of_kind(TemporalAspectKind::Event).len(), 1);
    }

    #[test]
    fn asset_aspect_graph_earliest_latest() {
        let production = InstantBridge::unix(1_699_000_000, 0);
        let publication = InstantBridge::unix(1_700_000_000, 0);
        let event = InstantBridge::unix(1_699_500_000, 0);

        let graph = AssetAspectGraph::new("recording-1", AssetKind::Recording)
            .with_temporal_aspect(TemporalAspect::new(
                TemporalAspectKind::Production,
                production,
            ))
            .with_temporal_aspect(TemporalAspect::new(
                TemporalAspectKind::Publication,
                publication,
            ))
            .with_temporal_aspect(TemporalAspect::new(TemporalAspectKind::Event, event));

        let earliest = graph.earliest_aspect().unwrap();
        assert_eq!(earliest.kind, TemporalAspectKind::Production);
        assert_eq!(earliest.instant.secs, 1_699_000_000);

        let latest = graph.latest_aspect().unwrap();
        assert_eq!(latest.kind, TemporalAspectKind::Publication);
        assert_eq!(latest.instant.secs, 1_700_000_000);
    }

    #[test]
    fn asset_aspect_graph_temporal_span() {
        let production = InstantBridge::unix(1_699_000_000, 0);
        let publication = InstantBridge::unix(1_700_000_000, 0);

        let graph = AssetAspectGraph::new("recording-1", AssetKind::Recording)
            .with_temporal_aspect(TemporalAspect::new(
                TemporalAspectKind::Production,
                production,
            ))
            .with_temporal_aspect(TemporalAspect::new(
                TemporalAspectKind::Publication,
                publication,
            ));

        let span = graph.temporal_span().unwrap();
        assert_eq!(span.secs, 1_000_000); // 1M seconds between production and publication
        assert!(span.is_positive());
    }

    #[test]
    fn asset_aspect_graph_with_topics() {
        let graph = AssetAspectGraph::new("recording-1", AssetKind::Recording)
            .with_topic(TopicAssociation::new("jazz", "recording-1", "q42:isAbout"))
            .with_topic(TopicAssociation::new(
                "Carnegie Hall",
                "recording-1",
                "q42:associatedWith",
            ));

        assert_eq!(graph.topics.len(), 2);
        assert_eq!(graph.topics_with_relation("q42:isAbout").len(), 1);
        assert_eq!(graph.topics_with_relation("q42:associatedWith").len(), 1);
        assert_eq!(graph.topics_about("recording-1").len(), 2);
    }

    #[test]
    fn asset_aspect_graph_to_quins() {
        let production = InstantBridge::unix(1_699_000_000, 0);
        let anchor = SpatialAnchorLite::new("venue-1", 40.7128, -74.0060, 10.0);

        let graph = AssetAspectGraph::new("recording-1", AssetKind::Recording)
            .with_spatial_anchor(anchor)
            .with_temporal_aspect(TemporalAspect::new(
                TemporalAspectKind::Production,
                production,
            ))
            .with_topic(TopicAssociation::new("jazz", "recording-1", "q42:isAbout"));

        let quins = graph.to_quins();
        // 1 temporal + 1 topic + 1 spatial + 1 kind = 4
        assert_eq!(quins.len(), 4);

        // All Quins should have the same context (sub-graph)
        let ctx = graph.context_hash();
        for q in &quins {
            assert_eq!(q.context, ctx);
        }

        // All Quins should have the same subject (asset ID hash)
        let subj = graph.asset_id_hash();
        for q in &quins {
            assert_eq!(q.subject, subj);
        }
    }

    #[test]
    fn asset_aspect_graph_context_hash_deterministic() {
        let g1 = AssetAspectGraph::new("recording-1", AssetKind::Recording);
        let g2 = AssetAspectGraph::new("recording-1", AssetKind::Recording);
        assert_eq!(g1.context_hash(), g2.context_hash());

        // Different assets have different contexts
        let g3 = AssetAspectGraph::new("recording-2", AssetKind::Recording);
        assert_ne!(g1.context_hash(), g3.context_hash());

        // Different kinds have different contexts
        let g4 = AssetAspectGraph::new("recording-1", AssetKind::Venue);
        assert_ne!(g1.context_hash(), g4.context_hash());
    }

    #[test]
    fn asset_aspect_graph_venue_example() {
        // Example: a venue with event dates
        let venue_anchor = SpatialAnchorLite::new("carnegie-hall", 40.7651, -73.9799, 0.0);
        let event1 = InstantBridge::unix(1_699_000_000, 0);
        let event2 = InstantBridge::unix(1_700_000_000, 0);

        let graph = AssetAspectGraph::new("carnegie-hall", AssetKind::Venue)
            .with_spatial_anchor(venue_anchor)
            .with_temporal_aspect(
                TemporalAspect::new(TemporalAspectKind::Event, event1)
                    .with_duration(DurationBridge::from_secs(7200))
                    .with_asserted_by("did:organizer-1")
                    .with_evidence(EvidenceType::ClinicianObserved),
            )
            .with_temporal_aspect(
                TemporalAspect::new(TemporalAspectKind::Event, event2)
                    .with_duration(DurationBridge::from_secs(9000))
                    .with_asserted_by("did:organizer-2"),
            )
            .with_topic(TopicAssociation::new(
                "classical music",
                "carnegie-hall",
                "q42:isAbout",
            ))
            .with_owner("did:venue-owner");

        assert_eq!(graph.asset_kind, AssetKind::Venue);
        assert_eq!(graph.temporal_aspects.len(), 2);
        assert_eq!(graph.aspects_of_kind(TemporalAspectKind::Event).len(), 2);
        assert!(graph.spatial_anchor.is_some());
        assert_eq!(graph.owner_did, Some("did:venue-owner".into()));

        // Both events have durations
        for aspect in &graph.temporal_aspects {
            assert!(aspect.duration.is_some());
        }

        // The quins compile
        let quins = graph.to_quins();
        assert!(quins.len() >= 4); // 2 events + 1 topic + 1 spatial + 1 kind
    }

    #[test]
    fn asset_aspect_graph_serde_roundtrip() {
        let anchor = SpatialAnchorLite::new("venue-1", 40.7128, -74.0060, 10.0);
        let graph = AssetAspectGraph::new("recording-1", AssetKind::Recording)
            .with_spatial_anchor(anchor)
            .with_temporal_aspect(
                TemporalAspect::new(
                    TemporalAspectKind::Production,
                    InstantBridge::unix(1_699_000_000, 0),
                )
                .with_asserted_by("did:alice"),
            )
            .with_topic(TopicAssociation::new("jazz", "recording-1", "q42:isAbout"));

        let json = serde_json::to_string(&graph).unwrap();
        let restored: AssetAspectGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph, restored);
    }

    #[test]
    fn asset_kind_strings() {
        assert_eq!(AssetKind::Recording.as_str(), "q42:Recording");
        assert_eq!(AssetKind::Venue.as_str(), "q42:Venue");
        assert_eq!(AssetKind::EventSpace.as_str(), "q42:EventSpace");
        assert_eq!(AssetKind::Artwork.as_str(), "q42:Artwork");
    }
}
