//! Observer context, heraldic lexicon, multi-scale sheaves, civic time,
//! and agent characteristics (W7, W9, W14, W15, T59).
//!
//! ## W7: Measurement context / observer stalk
//!
//! A measurement context captures *who* observed *what*, *when*, and
//! *with what instrument*. It is the observer's stalk — the local view
//! from which a measurement was made. This is distinct from the raw
//! measurement value: the same value observed from different contexts
//! carries different epistemic weight.
//!
//! ## W9: Oral / heraldic lexicon modalities as identifier views
//!
//! An identifier can have multiple *views* — visual (IRI), oral
//! (spoken name), heraldic (emblem/sigil). These are not separate
//! identifiers; they are presentations of the same identity. The
//! heraldic lexicon maps an IRI to its modality-specific views.
//!
//! ## W14: Multi-scale / filtered sheaves (LOD as physics)
//!
//! A multi-scale sheaf is a sheaf filtered at a given level of detail
//! (LOD). The LOD is not a rendering parameter — it is a physical
//! scale at which the sheaf's sections are defined. Different scales
//! reveal different structure (e.g. continuum vs. molecular vs.
//! atomic).
//!
//! ## W15: Civic time + authority to assert it
//!
//! Civic time is time asserted by a civic authority (e.g. NIST,
//! GPS, a national metrology institute). A `CivicInstant` carries
//! the instant value AND the authority that asserted it, so the
//! provenance of the time claim is explicit.
//!
//! ## T59: Agent characteristics KB
//!
//! A knowledge base of agent characteristics — logged from behaviour,
//! not declared. Records what an agent *does*, not what it *claims*.
//! Used for governance, routing, and trust assessment.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` wish list W7, W9,
//! W14, W15, and §8.12 T59.

use crate::value::Value;
use std::collections::BTreeMap;

// ── W7: Measurement context / observer stalk ─────────────────────────────────

/// A measurement context — the observer's stalk (W7).
///
/// Captures who observed, with what instrument, at what instant, and
/// the frame of reference. The measurement value itself is separate;
/// this is the epistemic context.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementContext {
    /// The observer's DID or IRI.
    pub observer: String,
    /// The instrument identifier (e.g. "sensor:thermometer-0").
    pub instrument: String,
    /// Unix seconds when the measurement was taken.
    pub instant_secs: i64,
    /// Nanoseconds within the second.
    pub instant_nanos: u32,
    /// The frame of reference IRI (e.g. "frame:lab-inertial").
    pub frame: Option<String>,
    /// The measurement uncertainty (± in the same units as the value).
    pub uncertainty: Option<f64>,
    /// The stalk ID this measurement belongs to.
    pub stalk_id: Option<u64>,
}

impl MeasurementContext {
    pub fn new(observer: &str, instrument: &str, instant_secs: i64) -> Self {
        Self {
            observer: observer.into(),
            instrument: instrument.into(),
            instant_secs,
            instant_nanos: 0,
            frame: None,
            uncertainty: None,
            stalk_id: None,
        }
    }

    pub fn with_frame(mut self, frame: &str) -> Self {
        self.frame = Some(frame.into());
        self
    }

    pub fn with_uncertainty(mut self, u: f64) -> Self {
        self.uncertainty = Some(u);
        self
    }

    pub fn with_stalk(mut self, stalk_id: u64) -> Self {
        self.stalk_id = Some(stalk_id);
        self
    }

    /// Convert to a Value::Record.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("observer".into(), Value::String(self.observer.clone()));
        rec.insert("instrument".into(), Value::String(self.instrument.clone()));
        rec.insert("instant_secs".into(), Value::I64(self.instant_secs));
        rec.insert("instant_nanos".into(), Value::U64(self.instant_nanos as u64));
        if let Some(ref f) = self.frame {
            rec.insert("frame".into(), Value::String(f.clone()));
        }
        if let Some(u) = self.uncertainty {
            rec.insert("uncertainty".into(), Value::F64(u));
        }
        if let Some(s) = self.stalk_id {
            rec.insert("stalk_id".into(), Value::U64(s));
        }
        Value::Record(rec)
    }

    /// Extract from a Value::Record, if possible.
    pub fn from_value(val: &Value) -> Option<Self> {
        let rec = match val {
            Value::Record(r) => r,
            _ => return None,
        };
        let observer = match rec.get("observer") {
            Some(Value::String(s)) => s.clone(),
            _ => return None,
        };
        let instrument = match rec.get("instrument") {
            Some(Value::String(s)) => s.clone(),
            _ => return None,
        };
        let instant_secs = match rec.get("instant_secs") {
            Some(Value::I64(n)) => *n,
            _ => return None,
        };
        let instant_nanos = match rec.get("instant_nanos") {
            Some(Value::U64(n)) => *n as u32,
            _ => 0,
        };
        let frame = match rec.get("frame") {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        };
        let uncertainty = match rec.get("uncertainty") {
            Some(Value::F64(n)) => Some(*n),
            _ => None,
        };
        let stalk_id = match rec.get("stalk_id") {
            Some(Value::U64(n)) => Some(*n),
            _ => None,
        };
        Some(Self {
            observer,
            instrument,
            instant_secs,
            instant_nanos,
            frame,
            uncertainty,
            stalk_id,
        })
    }
}

// ── W9: Oral / heraldic lexicon modalities ───────────────────────────────────

/// The modality of an identifier view (W9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentifierModality {
    /// Visual: the canonical IRI or text representation.
    Visual,
    /// Oral: a spoken name, for audio output.
    Oral,
    /// Heraldic: an emblem, sigil, or icon.
    Heraldic,
    /// Tactile: a Braille or haptic representation.
    Tactile,
}

impl IdentifierModality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Visual => "visual",
            Self::Oral => "oral",
            Self::Heraldic => "heraldic",
            Self::Tactile => "tactile",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "visual" => Some(Self::Visual),
            "oral" => Some(Self::Oral),
            "heraldic" => Some(Self::Heraldic),
            "tactile" => Some(Self::Tactile),
            _ => None,
        }
    }
}

/// An identifier view — one modality's presentation of an identity (W9).
#[derive(Debug, Clone, PartialEq)]
pub struct IdentifierView {
    /// The canonical IRI this view represents.
    pub iri: String,
    /// The modality of this view.
    pub modality: IdentifierModality,
    /// The view's content (spoken name, emblem description, etc.).
    pub content: String,
    /// Optional locale code (e.g. "en", "zh").
    pub locale: Option<String>,
}

impl IdentifierView {
    pub fn new(iri: &str, modality: IdentifierModality, content: &str) -> Self {
        Self {
            iri: iri.into(),
            modality,
            content: content.into(),
            locale: None,
        }
    }

    pub fn with_locale(mut self, locale: &str) -> Self {
        self.locale = Some(locale.into());
        self
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("iri".into(), Value::String(self.iri.clone()));
        rec.insert("modality".into(), Value::String(self.modality.as_str().into()));
        rec.insert("content".into(), Value::String(self.content.clone()));
        if let Some(ref l) = self.locale {
            rec.insert("locale".into(), Value::String(l.clone()));
        }
        Value::Record(rec)
    }
}

// ── W14: Multi-scale / filtered sheaves ───────────────────────────────────────

/// A level of detail (LOD) for multi-scale sheaves (W14).
///
/// The LOD is a physical scale, not a rendering parameter. Different
/// scales reveal different structure.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LevelOfDetail {
    /// Continuum scale (macroscopic, field equations).
    Continuum,
    /// Mesoscopic scale (grain-level, homogenization).
    Mesoscopic,
    /// Molecular scale (individual molecules).
    Molecular,
    /// Atomic scale (individual atoms).
    Atomic,
    /// Subatomic scale (nucleons, quarks).
    Subatomic,
}

impl LevelOfDetail {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Continuum => "continuum",
            Self::Mesoscopic => "mesoscopic",
            Self::Molecular => "molecular",
            Self::Atomic => "atomic",
            Self::Subatomic => "subatomic",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "continuum" => Some(Self::Continuum),
            "mesoscopic" => Some(Self::Mesoscopic),
            "molecular" => Some(Self::Molecular),
            "atomic" => Some(Self::Atomic),
            "subatomic" => Some(Self::Subatomic),
            _ => None,
        }
    }

    /// Returns true if this LOD is coarser (larger scale) than `other`.
    pub fn is_coarser_than(&self, other: &Self) -> bool {
        self < other
    }
}

/// A multi-scale sheaf filter — a sheaf viewed at a specific LOD (W14).
#[derive(Debug, Clone, PartialEq)]
pub struct MultiScaleSheaf {
    /// The sheaf name/identifier.
    pub sheaf_id: String,
    /// The level of detail at which this sheaf is filtered.
    pub lod: LevelOfDetail,
    /// The scale parameter (e.g. characteristic length in meters).
    pub scale: f64,
    /// Whether this is the finest available LOD.
    pub is_finest: bool,
}

impl MultiScaleSheaf {
    pub fn new(sheaf_id: &str, lod: LevelOfDetail, scale: f64) -> Self {
        Self {
            sheaf_id: sheaf_id.into(),
            lod,
            scale,
            is_finest: false,
        }
    }

    pub fn finest(sheaf_id: &str, lod: LevelOfDetail, scale: f64) -> Self {
        Self {
            sheaf_id: sheaf_id.into(),
            lod,
            scale,
            is_finest: true,
        }
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("sheaf_id".into(), Value::String(self.sheaf_id.clone()));
        rec.insert("lod".into(), Value::String(self.lod.as_str().into()));
        rec.insert("scale".into(), Value::F64(self.scale));
        rec.insert("is_finest".into(), Value::Bool(self.is_finest));
        Value::Record(rec)
    }
}

// ── W15: Civic time + authority ───────────────────────────────────────────────

/// A civic instant — time asserted by a civic authority (W15).
///
/// The authority is the metrology institute or time source that
/// asserts the instant. This makes the provenance of time claims
/// explicit — "NIST says it's 12:00" is different from "my local
/// clock says it's 12:00".
#[derive(Debug, Clone, PartialEq)]
pub struct CivicInstant {
    /// Unix seconds.
    pub secs: i64,
    /// Nanoseconds within the second.
    pub nanos: u32,
    /// The authority IRI that asserted this time (e.g.
    /// "did:civic:nist", "did:civic:gps").
    pub authority: String,
    /// The uncertainty in the asserted time (nanoseconds).
    pub uncertainty_nanos: Option<u64>,
}

impl CivicInstant {
    pub fn new(secs: i64, nanos: u32, authority: &str) -> Self {
        Self {
            secs,
            nanos,
            authority: authority.into(),
            uncertainty_nanos: None,
        }
    }

    pub fn with_uncertainty(mut self, uncertainty_nanos: u64) -> Self {
        self.uncertainty_nanos = Some(uncertainty_nanos);
        self
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("secs".into(), Value::I64(self.secs));
        rec.insert("nanos".into(), Value::U64(self.nanos as u64));
        rec.insert("authority".into(), Value::String(self.authority.clone()));
        if let Some(u) = self.uncertainty_nanos {
            rec.insert("uncertainty_nanos".into(), Value::U64(u));
        }
        Value::Record(rec)
    }
}

// ── T59: Agent characteristics KB ─────────────────────────────────────────────

/// A characteristic of an agent, logged from behaviour (T59).
#[derive(Debug, Clone, PartialEq)]
pub struct AgentCharacteristic {
    /// The agent's DID or IRI.
    pub agent_id: String,
    /// The characteristic name (e.g. "latency_p50_ms", "success_rate").
    pub name: String,
    /// The measured value.
    pub value: f64,
    /// When this characteristic was observed (Unix seconds).
    pub observed_at: i64,
    /// Number of observations that contributed to this value.
    pub sample_count: u64,
}

impl AgentCharacteristic {
    pub fn new(agent_id: &str, name: &str, value: f64, observed_at: i64) -> Self {
        Self {
            agent_id: agent_id.into(),
            name: name.into(),
            value,
            observed_at,
            sample_count: 1,
        }
    }

    pub fn with_sample_count(mut self, count: u64) -> Self {
        self.sample_count = count;
        self
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("agent_id".into(), Value::String(self.agent_id.clone()));
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("value".into(), Value::F64(self.value));
        rec.insert("observed_at".into(), Value::I64(self.observed_at));
        rec.insert("sample_count".into(), Value::U64(self.sample_count));
        Value::Record(rec)
    }
}

/// An agent characteristics KB — a log of agent characteristics
/// observed from behaviour (T59).
#[derive(Debug, Clone, Default)]
pub struct AgentCharacteristicsKb {
    entries: Vec<AgentCharacteristic>,
}

impl AgentCharacteristicsKb {
    pub fn new() -> Self {
        Self::default()
    }

    /// Log a characteristic observation.
    pub fn log(&mut self, entry: AgentCharacteristic) -> &mut Self {
        self.entries.push(entry);
        self
    }

    /// Get all characteristics for a given agent.
    pub fn for_agent(&self, agent_id: &str) -> Vec<&AgentCharacteristic> {
        self.entries.iter().filter(|e| e.agent_id == agent_id).collect()
    }

    /// Get the latest value of a named characteristic for an agent.
    pub fn latest(&self, agent_id: &str, name: &str) -> Option<&AgentCharacteristic> {
        self.entries
            .iter()
            .filter(|e| e.agent_id == agent_id && e.name == name)
            .max_by_key(|e| e.observed_at)
    }

    /// Get the mean value of a named characteristic for an agent.
    pub fn mean(&self, agent_id: &str, name: &str) -> Option<f64> {
        let matches: Vec<&AgentCharacteristic> = self.entries
            .iter()
            .filter(|e| e.agent_id == agent_id && e.name == name)
            .collect();
        if matches.is_empty() {
            return None;
        }
        let total: f64 = matches.iter().map(|e| e.value).sum();
        Some(total / matches.len() as f64)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the KB empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All entries as a List of Records.
    pub fn to_value_list(&self) -> Value {
        Value::List(self.entries.iter().map(|e| e.to_value()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── W7: MeasurementContext tests ─────────────────────────────────

    #[test]
    fn w7_measurement_context_basic() {
        let mc = MeasurementContext::new("did:alice", "sensor:therm-0", 1000);
        assert_eq!(mc.observer, "did:alice");
        assert_eq!(mc.instrument, "sensor:therm-0");
        assert_eq!(mc.instant_secs, 1000);
        assert_eq!(mc.instant_nanos, 0);
        assert!(mc.frame.is_none());
        assert!(mc.uncertainty.is_none());
    }

    #[test]
    fn w7_measurement_context_with_options() {
        let mc = MeasurementContext::new("did:bob", "sensor:accel-1", 2000)
            .with_frame("frame:lab")
            .with_uncertainty(0.01)
            .with_stalk(42);
        assert_eq!(mc.frame, Some("frame:lab".into()));
        assert_eq!(mc.uncertainty, Some(0.01));
        assert_eq!(mc.stalk_id, Some(42));
    }

    #[test]
    fn w7_measurement_context_round_trip() {
        let mc = MeasurementContext::new("did:alice", "sensor:therm-0", 1000)
            .with_uncertainty(0.5)
            .with_stalk(7);
        let v = mc.to_value();
        let restored = MeasurementContext::from_value(&v).unwrap();
        assert_eq!(restored.observer, "did:alice");
        assert_eq!(restored.instrument, "sensor:therm-0");
        assert_eq!(restored.instant_secs, 1000);
        assert_eq!(restored.uncertainty, Some(0.5));
        assert_eq!(restored.stalk_id, Some(7));
    }

    #[test]
    fn w7_measurement_context_from_non_record() {
        let v = Value::I64(42);
        assert!(MeasurementContext::from_value(&v).is_none());
    }

    // ── W9: IdentifierView tests ──────────────────────────────────────

    #[test]
    fn w9_identifier_modality_round_trip() {
        for m in &[
            IdentifierModality::Visual,
            IdentifierModality::Oral,
            IdentifierModality::Heraldic,
            IdentifierModality::Tactile,
        ] {
            let s = m.as_str();
            assert_eq!(IdentifierModality::from_str(s), Some(*m));
        }
        assert_eq!(IdentifierModality::from_str("unknown"), None);
    }

    #[test]
    fn w9_identifier_view_basic() {
        let v = IdentifierView::new("did:alice", IdentifierModality::Oral, "Alice");
        assert_eq!(v.iri, "did:alice");
        assert_eq!(v.modality, IdentifierModality::Oral);
        assert_eq!(v.content, "Alice");
        assert!(v.locale.is_none());
    }

    #[test]
    fn w9_identifier_view_with_locale() {
        let v = IdentifierView::new("did:alice", IdentifierModality::Oral, "爱丽丝")
            .with_locale("zh");
        assert_eq!(v.locale, Some("zh".into()));
    }

    #[test]
    fn w9_identifier_view_to_value() {
        let v = IdentifierView::new("did:alice", IdentifierModality::Heraldic, "rose sigil")
            .with_locale("en");
        let val = v.to_value();
        match &val {
            Value::Record(r) => {
                assert_eq!(r.len(), 4);
                assert!(r.contains_key("iri"));
                assert!(r.contains_key("modality"));
                assert!(r.contains_key("content"));
                assert!(r.contains_key("locale"));
            }
            _ => panic!("expected Record"),
        }
    }

    // ── W14: MultiScaleSheaf tests ────────────────────────────────────

    #[test]
    fn w14_lod_round_trip() {
        for l in &[
            LevelOfDetail::Continuum,
            LevelOfDetail::Mesoscopic,
            LevelOfDetail::Molecular,
            LevelOfDetail::Atomic,
            LevelOfDetail::Subatomic,
        ] {
            let s = l.as_str();
            assert_eq!(LevelOfDetail::from_str(s), Some(*l));
        }
    }

    #[test]
    fn w14_lod_ordering() {
        assert!(LevelOfDetail::Continuum.is_coarser_than(&LevelOfDetail::Molecular));
        assert!(LevelOfDetail::Molecular.is_coarser_than(&LevelOfDetail::Atomic));
        assert!(!LevelOfDetail::Atomic.is_coarser_than(&LevelOfDetail::Continuum));
    }

    #[test]
    fn w14_multi_scale_sheaf_basic() {
        let s = MultiScaleSheaf::new("sheaf:fluid", LevelOfDetail::Continuum, 1.0);
        assert_eq!(s.sheaf_id, "sheaf:fluid");
        assert_eq!(s.lod, LevelOfDetail::Continuum);
        assert_eq!(s.scale, 1.0);
        assert!(!s.is_finest);
    }

    #[test]
    fn w14_multi_scale_sheaf_finest() {
        let s = MultiScaleSheaf::finest("sheaf:fluid", LevelOfDetail::Atomic, 1e-10);
        assert!(s.is_finest);
    }

    #[test]
    fn w14_multi_scale_sheaf_to_value() {
        let s = MultiScaleSheaf::new("sheaf:fluid", LevelOfDetail::Molecular, 1e-9);
        let v = s.to_value();
        match &v {
            Value::Record(r) => {
                assert_eq!(r.len(), 4);
                assert!(r.contains_key("sheaf_id"));
                assert!(r.contains_key("lod"));
                assert!(r.contains_key("scale"));
                assert!(r.contains_key("is_finest"));
            }
            _ => panic!("expected Record"),
        }
    }

    // ── W15: CivicInstant tests ───────────────────────────────────────

    #[test]
    fn w15_civic_instant_basic() {
        let ci = CivicInstant::new(1000, 500_000_000, "did:civic:nist");
        assert_eq!(ci.secs, 1000);
        assert_eq!(ci.nanos, 500_000_000);
        assert_eq!(ci.authority, "did:civic:nist");
        assert!(ci.uncertainty_nanos.is_none());
    }

    #[test]
    fn w15_civic_instant_with_uncertainty() {
        let ci = CivicInstant::new(1000, 0, "did:civic:gps").with_uncertainty(100);
        assert_eq!(ci.uncertainty_nanos, Some(100));
    }

    #[test]
    fn w15_civic_instant_to_value() {
        let ci = CivicInstant::new(2000, 0, "did:civic:nist").with_uncertainty(50);
        let v = ci.to_value();
        match &v {
            Value::Record(r) => {
                assert_eq!(r.len(), 4);
                assert!(r.contains_key("secs"));
                assert!(r.contains_key("nanos"));
                assert!(r.contains_key("authority"));
                assert!(r.contains_key("uncertainty_nanos"));
            }
            _ => panic!("expected Record"),
        }
    }

    // ── T59: AgentCharacteristicsKb tests ─────────────────────────────

    #[test]
    fn t59_agent_characteristic_basic() {
        let c = AgentCharacteristic::new("did:agent-1", "latency_p50_ms", 42.5, 1000);
        assert_eq!(c.agent_id, "did:agent-1");
        assert_eq!(c.name, "latency_p50_ms");
        assert_eq!(c.value, 42.5);
        assert_eq!(c.sample_count, 1);
    }

    #[test]
    fn t59_agent_characteristic_with_sample_count() {
        let c = AgentCharacteristic::new("did:agent-1", "success_rate", 0.95, 1000)
            .with_sample_count(100);
        assert_eq!(c.sample_count, 100);
    }

    #[test]
    fn t59_kb_log_and_query() {
        let mut kb = AgentCharacteristicsKb::new();
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 42.0, 1000));
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 38.0, 2000));
        kb.log(AgentCharacteristic::new("did:agent-2", "latency_ms", 55.0, 1000));
        assert_eq!(kb.len(), 3);
        let a1 = kb.for_agent("did:agent-1");
        assert_eq!(a1.len(), 2);
        let a2 = kb.for_agent("did:agent-2");
        assert_eq!(a2.len(), 1);
    }

    #[test]
    fn t59_kb_latest() {
        let mut kb = AgentCharacteristicsKb::new();
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 42.0, 1000));
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 38.0, 2000));
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 40.0, 1500));
        let latest = kb.latest("did:agent-1", "latency_ms").unwrap();
        assert_eq!(latest.value, 38.0);
        assert_eq!(latest.observed_at, 2000);
    }

    #[test]
    fn t59_kb_mean() {
        let mut kb = AgentCharacteristicsKb::new();
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 40.0, 1000));
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 60.0, 2000));
        let mean = kb.mean("did:agent-1", "latency_ms").unwrap();
        assert!((mean - 50.0).abs() < 1e-9);
    }

    #[test]
    fn t59_kb_latest_missing() {
        let kb = AgentCharacteristicsKb::new();
        assert!(kb.latest("did:unknown", "anything").is_none());
    }

    #[test]
    fn t59_kb_mean_missing() {
        let kb = AgentCharacteristicsKb::new();
        assert!(kb.mean("did:unknown", "anything").is_none());
    }

    #[test]
    fn t59_kb_empty() {
        let kb = AgentCharacteristicsKb::new();
        assert!(kb.is_empty());
        assert_eq!(kb.len(), 0);
    }

    #[test]
    fn t59_kb_to_value_list() {
        let mut kb = AgentCharacteristicsKb::new();
        kb.log(AgentCharacteristic::new("did:agent-1", "latency_ms", 42.0, 1000));
        let v = kb.to_value_list();
        match &v {
            Value::List(xs) => assert_eq!(xs.len(), 1),
            _ => panic!("expected List"),
        }
    }
}
