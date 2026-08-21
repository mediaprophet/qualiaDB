//! Persistent asset aspect-graph store.
//!
//! Provides a process-local registry that retains asset records across
//! VibeScript invoke calls. Assets are keyed by asset ID and can be resolved
//! by ID, spatial anchor, topic, or temporal aspect kind.
//!
//! The store preserves:
//! - Asset identity (ID, kind, owner DID).
//! - Independent temporal aspect sub-graphs (each with kind, time, duration,
//!   asserting agent, confidence).
//! - Topic associations.
//! - Spatial anchor (lat, lon, altitude, anchor IRI).
//!
//! All operations are deterministic and bounded. The store uses `BTreeMap`
//! for stable iteration order.

use crate::q_hash;
use crate::NQuin;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// A temporal aspect — an independent sub-graph describing when the asset
/// existed/was created/was present in some form.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalAspect {
    pub kind_iri: String,
    pub seconds: i64,
    pub nanoseconds: u32,
    pub duration_seconds: Option<i64>,
    pub asserting_agent: Option<String>,
    pub confidence: Option<f64>,
}

/// A spatial anchor — where the asset is located.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialAnchor {
    pub anchor_iri: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
}

/// A persisted asset with its aspect sub-graphs.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PersistedAsset {
    pub asset_id: String,
    pub asset_id_hash: u64,
    pub kind_iri: String,
    pub owner_did: Option<String>,
    pub temporal_aspects: Vec<TemporalAspect>,
    pub topics: Vec<String>,
    pub spatial_anchor: Option<SpatialAnchor>,
}

impl PersistedAsset {
    pub fn new(asset_id: &str, kind_iri: &str, owner_did: Option<&str>) -> Self {
        Self {
            asset_id: asset_id.to_string(),
            asset_id_hash: q_hash(asset_id),
            kind_iri: kind_iri.to_string(),
            owner_did: owner_did.map(|s| s.to_string()),
            temporal_aspects: Vec::new(),
            topics: Vec::new(),
            spatial_anchor: None,
        }
    }

    pub fn add_temporal(&mut self, aspect: TemporalAspect) {
        self.temporal_aspects.push(aspect);
    }

    pub fn add_topic(&mut self, topic: &str) {
        if !self.topics.iter().any(|t| t == topic) {
            self.topics.push(topic.to_string());
        }
    }

    pub fn set_spatial(&mut self, anchor: SpatialAnchor) {
        self.spatial_anchor = Some(anchor);
    }

    /// Calculate the temporal span (in seconds) between the earliest and
    /// latest temporal aspects.
    pub fn temporal_span_seconds(&self) -> Option<i64> {
        if self.temporal_aspects.len() < 2 {
            return None;
        }
        let mut earliest = i64::MAX;
        let mut latest = i64::MIN;
        for aspect in &self.temporal_aspects {
            let t = aspect.seconds;
            if t < earliest {
                earliest = t;
            }
            if t > latest {
                latest = t;
            }
        }
        if earliest == i64::MAX || latest == i64::MIN {
            return None;
        }
        Some(latest - earliest)
    }

    /// Query temporal aspects by kind IRI.
    pub fn query_aspects_by_kind(&self, kind_iri: &str) -> Vec<&TemporalAspect> {
        self.temporal_aspects
            .iter()
            .filter(|a| a.kind_iri == kind_iri)
            .collect()
    }

    /// Compile the asset to graph quins. Each temporal aspect, topic, and the
    /// spatial anchor become a quin relating to the asset.
    pub fn compile_to_quins(&self) -> Vec<NQuin> {
        let subject = self.asset_id_hash;
        let mut quins = Vec::new();
        // Asset kind quin.
        quins.push(NQuin {
            subject,
            predicate: q_hash("q42:hasKind"),
            object: q_hash(&self.kind_iri),
            context: 0,
            metadata: 0,
            parity: 0,
        });
        // Owner quin if present.
        if let Some(owner) = &self.owner_did {
            quins.push(NQuin {
                subject,
                predicate: q_hash("q42:hasOwner"),
                object: q_hash(owner),
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }
        // Temporal aspect quins.
        for aspect in &self.temporal_aspects {
            quins.push(NQuin {
                subject,
                predicate: q_hash(&aspect.kind_iri),
                object: aspect.seconds as u64,
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }
        // Topic quins.
        for topic in &self.topics {
            quins.push(NQuin {
                subject,
                predicate: q_hash("q42:hasTopic"),
                object: q_hash(topic),
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }
        // Spatial anchor quin.
        if let Some(anchor) = &self.spatial_anchor {
            quins.push(NQuin {
                subject,
                predicate: q_hash("q42:hasSpatialAnchor"),
                object: q_hash(&anchor.anchor_iri),
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }
        quins
    }
}

/// The process-local asset store.
static ASSET_STORE: Mutex<Option<BTreeMap<String, PersistedAsset>>> = Mutex::new(None);

fn with_store<F, R>(f: F) -> R
where
    F: FnOnce(&mut BTreeMap<String, PersistedAsset>) -> R,
{
    let mut guard = ASSET_STORE.lock().expect("asset store mutex poisoned");
    if guard.is_none() {
        *guard = Some(BTreeMap::new());
    }
    f(guard.as_mut().expect("asset store map"))
}

/// Persist a new asset. Returns false if an asset with the same ID already exists.
pub fn persist_asset(asset: PersistedAsset) -> bool {
    with_store(|store| {
        if store.contains_key(&asset.asset_id) {
            return false;
        }
        store.insert(asset.asset_id.clone(), asset);
        true
    })
}

/// Get a clone of a persisted asset by ID.
pub fn get_asset(asset_id: &str) -> Option<PersistedAsset> {
    with_store(|store| store.get(asset_id).cloned())
}

/// Update a persisted asset. Returns false if the asset doesn't exist.
#[allow(dead_code)]
pub fn update_asset<F>(asset_id: &str, f: F) -> bool
where
    F: FnOnce(&mut PersistedAsset),
{
    with_store(|store| {
        if let Some(asset) = store.get_mut(asset_id) {
            f(asset);
            true
        } else {
            false
        }
    })
}

/// Add a temporal aspect to a persisted asset.
#[allow(dead_code)]
pub fn add_temporal_aspect(asset_id: &str, aspect: TemporalAspect) -> bool {
    update_asset(asset_id, |a| a.add_temporal(aspect))
}

/// Add a topic to a persisted asset.
#[allow(dead_code)]
pub fn add_topic_to_asset(asset_id: &str, topic: &str) -> bool {
    update_asset(asset_id, |a| a.add_topic(topic))
}

/// Set the spatial anchor on a persisted asset.
#[allow(dead_code)]
pub fn set_spatial_anchor(asset_id: &str, anchor: SpatialAnchor) -> bool {
    update_asset(asset_id, |a| a.set_spatial(anchor))
}

/// Resolve assets by spatial anchor IRI.
pub fn resolve_by_spatial(anchor_iri: &str) -> Vec<PersistedAsset> {
    with_store(|store| {
        store
            .values()
            .filter(|a| {
                a.spatial_anchor
                    .as_ref()
                    .map(|s| s.anchor_iri == anchor_iri)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    })
}

/// Resolve assets by topic.
pub fn resolve_by_topic(topic: &str) -> Vec<PersistedAsset> {
    with_store(|store| {
        store
            .values()
            .filter(|a| a.topics.iter().any(|t| t == topic))
            .cloned()
            .collect()
    })
}

/// Resolve assets by temporal aspect kind.
pub fn resolve_by_temporal_kind(kind_iri: &str) -> Vec<PersistedAsset> {
    with_store(|store| {
        store
            .values()
            .filter(|a| a.temporal_aspects.iter().any(|t| t.kind_iri == kind_iri))
            .cloned()
            .collect()
    })
}

/// Get the temporal span of a persisted asset.
#[allow(dead_code)]
pub fn temporal_span(asset_id: &str) -> Option<i64> {
    with_store(|store| store.get(asset_id).and_then(|a| a.temporal_span_seconds()))
}

/// Query temporal aspects of a persisted asset by kind.
#[allow(dead_code)]
pub fn query_aspects(asset_id: &str, kind_iri: &str) -> Vec<TemporalAspect> {
    with_store(|store| {
        store
            .get(asset_id)
            .map(|a| {
                a.query_aspects_by_kind(kind_iri)
                    .into_iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Compile a persisted asset to graph quins.
#[allow(dead_code)]
pub fn compile_asset(asset_id: &str) -> Option<Vec<NQuin>> {
    with_store(|store| store.get(asset_id).map(|a| a.compile_to_quins()))
}

/// List all persisted asset IDs.
pub fn list_asset_ids() -> Vec<String> {
    with_store(|store| store.keys().cloned().collect())
}

/// Clear the store (for testing).
#[allow(dead_code)]
pub fn clear_store() {
    with_store(|store| store.clear());
}

/// Count of persisted assets.
pub fn asset_count() -> usize {
    with_store(|store| store.len())
}

// ── VibeScript invoke seams ─────────────────────────────────────────────────

use super::args;
use poet_vibe::{Diagnostic, Span, Value};

/// `Asset.persist` — persist an asset record to the store.
/// Args: { asset_id: String, kind: String, owner_did?: String }
pub fn persist_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist needs asset_id"))?;
    let kind =
        args::rec_str(args, "kind").ok_or_else(|| args::bad(span, "Asset.persist needs kind"))?;
    let owner_did = args::rec_str(args, "owner_did");
    let asset = PersistedAsset::new(asset_id, kind, owner_did);
    let created = persist_asset(asset);
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("persisted", Value::Bool(created)),
    ]))
}

/// `Asset.persist_create` — create AND persist an asset in one call.
/// Args: { asset_id: String, kind: String, owner_did?: String }
pub fn persist_create_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist_create needs asset_id"))?;
    let kind = args::rec_str(args, "kind")
        .ok_or_else(|| args::bad(span, "Asset.persist_create needs kind"))?;
    let owner_did = args::rec_str(args, "owner_did");
    let asset = PersistedAsset::new(asset_id, kind, owner_did);
    let created = persist_asset(asset);
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("kind", Value::String(kind.to_string())),
        ("persisted", Value::Bool(created)),
    ]))
}

/// `Asset.persist_add_temporal` — add a temporal aspect to a persisted asset.
/// Args: { asset_id: String, kind_iri: String, seconds: I64, nanos?: U64, duration_secs?: I64, asserted_by?: String, confidence?: F64 }
pub fn persist_add_temporal_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist_add_temporal needs asset_id"))?;
    let kind_iri = args::rec_str(args, "kind_iri")
        .ok_or_else(|| args::bad(span, "Asset.persist_add_temporal needs kind_iri"))?;
    let seconds = args::rec_i64(args, "seconds")
        .ok_or_else(|| args::bad(span, "Asset.persist_add_temporal needs seconds"))?;
    let nanoseconds = args::rec_u64(args, "nanos").unwrap_or(0) as u32;
    let duration_seconds = args::rec_i64(args, "duration_secs");
    let asserting_agent = args::rec_str(args, "asserted_by").map(|s| s.to_string());
    let confidence = args::rec_f64(args, "confidence");
    let aspect = TemporalAspect {
        kind_iri: kind_iri.to_string(),
        seconds,
        nanoseconds,
        duration_seconds,
        asserting_agent,
        confidence,
    };
    let added = add_temporal_aspect(asset_id, aspect);
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("added", Value::Bool(added)),
    ]))
}

/// `Asset.persist_add_topic` — add a topic to a persisted asset.
/// Args: { asset_id: String, topic: String }
pub fn persist_add_topic_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist_add_topic needs asset_id"))?;
    let topic = args::rec_str(args, "topic")
        .ok_or_else(|| args::bad(span, "Asset.persist_add_topic needs topic"))?;
    let added = add_topic_to_asset(asset_id, topic);
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("topic", Value::String(topic.to_string())),
        ("added", Value::Bool(added)),
    ]))
}

/// `Asset.persist_set_spatial` — set the spatial anchor on a persisted asset.
/// Args: { asset_id: String, anchor_iri: String, latitude?: F64, longitude?: F64, altitude?: F64 }
pub fn persist_set_spatial_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist_set_spatial needs asset_id"))?;
    let anchor_iri = args::rec_str(args, "anchor_iri")
        .ok_or_else(|| args::bad(span, "Asset.persist_set_spatial needs anchor_iri"))?;
    let latitude = args::rec_f64(args, "latitude");
    let longitude = args::rec_f64(args, "longitude");
    let altitude = args::rec_f64(args, "altitude");
    let anchor = SpatialAnchor {
        anchor_iri: anchor_iri.to_string(),
        latitude,
        longitude,
        altitude,
    };
    let set = set_spatial_anchor(asset_id, anchor);
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("anchor_iri", Value::String(anchor_iri.to_string())),
        ("set", Value::Bool(set)),
    ]))
}

/// `Asset.persist_compile` — compile a persisted asset to graph quins.
/// Args: { asset_id: String }
pub fn persist_compile_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist_compile needs asset_id"))?;
    let quins = compile_asset(asset_id)
        .ok_or_else(|| args::bad(span, format!("asset {asset_id} not found")))?;
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("quin_count", Value::U64(quins.len() as u64)),
    ]))
}

/// `Asset.persist_temporal_span` — get the temporal span of a persisted asset.
/// Args: { asset_id: String }
pub fn persist_temporal_span_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist_temporal_span needs asset_id"))?;
    let span_secs = temporal_span(asset_id).ok_or_else(|| {
        args::bad(
            span,
            format!("asset {asset_id} not found or has < 2 aspects"),
        )
    })?;
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("span_seconds", Value::I64(span_secs)),
    ]))
}

/// `Asset.persist_query_aspects` — query temporal aspects of a persisted asset by kind.
/// Args: { asset_id: String, kind_iri: String }
pub fn persist_query_aspects_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.persist_query_aspects needs asset_id"))?;
    let kind_iri = args::rec_str(args, "kind_iri")
        .ok_or_else(|| args::bad(span, "Asset.persist_query_aspects needs kind_iri"))?;
    let aspects = query_aspects(asset_id, kind_iri);
    let aspect_records: Vec<Value> = aspects
        .iter()
        .map(|a| {
            let mut r = BTreeMap::new();
            r.insert("kind_iri".into(), Value::String(a.kind_iri.clone()));
            r.insert("seconds".into(), Value::I64(a.seconds));
            r.insert("nanoseconds".into(), Value::U64(a.nanoseconds as u64));
            if let Some(d) = a.duration_seconds {
                r.insert("duration_seconds".into(), Value::I64(d));
            }
            if let Some(agent) = &a.asserting_agent {
                r.insert("asserting_agent".into(), Value::String(agent.clone()));
            }
            if let Some(conf) = a.confidence {
                r.insert("confidence".into(), Value::F64(conf));
            }
            Value::Record(r)
        })
        .collect();
    Ok(args::record([
        ("asset_id", Value::String(asset_id.to_string())),
        ("kind_iri", Value::String(kind_iri.to_string())),
        ("aspects", Value::List(aspect_records)),
        ("count", Value::U64(aspects.len() as u64)),
    ]))
}

/// `Asset.resolve` — resolve an asset by ID, returning its full record.
pub fn resolve_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| args::bad(span, "Asset.resolve needs asset_id"))?;
    let asset = get_asset(asset_id)
        .ok_or_else(|| args::bad(span, format!("asset {asset_id} not found")))?;
    Ok(asset_to_value(&asset))
}

/// `Asset.resolve_by_spatial` — resolve assets by spatial anchor IRI.
pub fn resolve_by_spatial_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let anchor_iri = args::rec_str(args, "anchor_iri")
        .ok_or_else(|| args::bad(span, "Asset.resolve_by_spatial needs anchor_iri"))?;
    let assets = resolve_by_spatial(anchor_iri);
    Ok(args::record([
        ("anchor_iri", Value::String(anchor_iri.to_string())),
        (
            "assets",
            Value::List(assets.iter().map(asset_to_value).collect()),
        ),
        ("count", Value::U64(assets.len() as u64)),
    ]))
}

/// `Asset.resolve_by_topic` — resolve assets by topic.
pub fn resolve_by_topic_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let topic = args::rec_str(args, "topic")
        .ok_or_else(|| args::bad(span, "Asset.resolve_by_topic needs topic"))?;
    let assets = resolve_by_topic(topic);
    Ok(args::record([
        ("topic", Value::String(topic.to_string())),
        (
            "assets",
            Value::List(assets.iter().map(asset_to_value).collect()),
        ),
        ("count", Value::U64(assets.len() as u64)),
    ]))
}

/// `Asset.resolve_by_temporal` — resolve assets by temporal aspect kind.
pub fn resolve_by_temporal_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let kind_iri = args::rec_str(args, "kind_iri")
        .ok_or_else(|| args::bad(span, "Asset.resolve_by_temporal needs kind_iri"))?;
    let assets = resolve_by_temporal_kind(kind_iri);
    Ok(args::record([
        ("kind_iri", Value::String(kind_iri.to_string())),
        (
            "assets",
            Value::List(assets.iter().map(asset_to_value).collect()),
        ),
        ("count", Value::U64(assets.len() as u64)),
    ]))
}

/// `Asset.list` — list all persisted asset IDs.
pub fn list_seam(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let ids = list_asset_ids();
    Ok(args::record([
        (
            "asset_ids",
            Value::List(ids.into_iter().map(Value::String).collect()),
        ),
        ("count", Value::U64(asset_count() as u64)),
    ]))
}

/// `Asset.count` — count persisted assets.
pub fn count_seam(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    Ok(args::record([("count", Value::U64(asset_count() as u64))]))
}

/// Convert a PersistedAsset to a VibeScript Value::Record.
fn asset_to_value(asset: &PersistedAsset) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("asset_id".into(), Value::String(asset.asset_id.clone()));
    rec.insert("kind_iri".into(), Value::String(asset.kind_iri.clone()));
    if let Some(owner) = &asset.owner_did {
        rec.insert("owner_did".into(), Value::String(owner.clone()));
    }
    // Temporal aspects.
    let temporal: Vec<Value> = asset
        .temporal_aspects
        .iter()
        .map(|a| {
            let mut r = BTreeMap::new();
            r.insert("kind_iri".into(), Value::String(a.kind_iri.clone()));
            r.insert("seconds".into(), Value::I64(a.seconds));
            r.insert("nanoseconds".into(), Value::U64(a.nanoseconds as u64));
            if let Some(d) = a.duration_seconds {
                r.insert("duration_seconds".into(), Value::I64(d));
            }
            if let Some(agent) = &a.asserting_agent {
                r.insert("asserting_agent".into(), Value::String(agent.clone()));
            }
            if let Some(conf) = a.confidence {
                r.insert("confidence".into(), Value::F64(conf));
            }
            Value::Record(r)
        })
        .collect();
    rec.insert("temporal_aspects".into(), Value::List(temporal));
    // Topics.
    rec.insert(
        "topics".into(),
        Value::List(
            asset
                .topics
                .iter()
                .map(|t| Value::String(t.clone()))
                .collect(),
        ),
    );
    // Spatial anchor.
    if let Some(anchor) = &asset.spatial_anchor {
        let mut r = BTreeMap::new();
        r.insert(
            "anchor_iri".into(),
            Value::String(anchor.anchor_iri.clone()),
        );
        if let Some(lat) = anchor.latitude {
            r.insert("latitude".into(), Value::F64(lat));
        }
        if let Some(lon) = anchor.longitude {
            r.insert("longitude".into(), Value::F64(lon));
        }
        if let Some(alt) = anchor.altitude {
            r.insert("altitude".into(), Value::F64(alt));
        }
        rec.insert("spatial_anchor".into(), Value::Record(r));
    }
    Value::Record(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persist_and_get() {
        clear_store();
        let asset = PersistedAsset::new("asset1", "q42:Recording", Some("did:q42:alice"));
        assert!(persist_asset(asset));
        assert!(get_asset("asset1").is_some());
        assert!(!persist_asset(PersistedAsset::new(
            "asset1",
            "q42:Recording",
            None
        )));
    }

    #[test]
    fn add_temporal_and_query() {
        clear_store();
        persist_asset(PersistedAsset::new("a1", "q42:Recording", None));
        let aspect = TemporalAspect {
            kind_iri: "q42:recordingDate".into(),
            seconds: 1000,
            nanoseconds: 0,
            duration_seconds: Some(3600),
            asserting_agent: Some("did:q42:bob".into()),
            confidence: Some(0.95),
        };
        assert!(add_temporal_aspect("a1", aspect));
        let aspects = query_aspects("a1", "q42:recordingDate");
        assert_eq!(aspects.len(), 1);
        assert_eq!(aspects[0].seconds, 1000);
    }

    #[test]
    fn add_topic_and_resolve() {
        clear_store();
        persist_asset(PersistedAsset::new("a1", "q42:Recording", None));
        assert!(add_topic_to_asset("a1", "music"));
        let results = resolve_by_topic("music");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn set_spatial_and_resolve() {
        clear_store();
        persist_asset(PersistedAsset::new("a1", "q42:Recording", None));
        let anchor = SpatialAnchor {
            anchor_iri: "geo:51.5,-0.1".into(),
            latitude: Some(51.5),
            longitude: Some(-0.1),
            altitude: None,
        };
        assert!(set_spatial_anchor("a1", anchor));
        let results = resolve_by_spatial("geo:51.5,-0.1");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn temporal_span_calculation() {
        clear_store();
        persist_asset(PersistedAsset::new("a1", "q42:Recording", None));
        add_temporal_aspect(
            "a1",
            TemporalAspect {
                kind_iri: "q42:recordingDate".into(),
                seconds: 1000,
                nanoseconds: 0,
                duration_seconds: None,
                asserting_agent: None,
                confidence: None,
            },
        );
        add_temporal_aspect(
            "a1",
            TemporalAspect {
                kind_iri: "q42:publicationDate".into(),
                seconds: 2000,
                nanoseconds: 0,
                duration_seconds: None,
                asserting_agent: None,
                confidence: None,
            },
        );
        let span = temporal_span("a1");
        assert_eq!(span, Some(1000));
    }

    #[test]
    fn compile_to_quins() {
        clear_store();
        persist_asset(PersistedAsset::new(
            "a1",
            "q42:Recording",
            Some("did:q42:alice"),
        ));
        add_temporal_aspect(
            "a1",
            TemporalAspect {
                kind_iri: "q42:recordingDate".into(),
                seconds: 1000,
                nanoseconds: 0,
                duration_seconds: None,
                asserting_agent: None,
                confidence: None,
            },
        );
        add_topic_to_asset("a1", "music");
        set_spatial_anchor(
            "a1",
            SpatialAnchor {
                anchor_iri: "geo:51.5,-0.1".into(),
                latitude: Some(51.5),
                longitude: Some(-0.1),
                altitude: None,
            },
        );
        let quins = compile_asset("a1").unwrap();
        // kind + owner + temporal + topic + spatial = 5 quins
        assert_eq!(quins.len(), 5);
    }

    #[test]
    fn resolve_by_temporal_kind_test() {
        clear_store();
        persist_asset(PersistedAsset::new("a1", "q42:Recording", None));
        persist_asset(PersistedAsset::new("a2", "q42:Recording", None));
        add_temporal_aspect(
            "a1",
            TemporalAspect {
                kind_iri: "q42:recordingDate".into(),
                seconds: 1000,
                nanoseconds: 0,
                duration_seconds: None,
                asserting_agent: None,
                confidence: None,
            },
        );
        let results = resolve_by_temporal_kind("q42:recordingDate");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_and_count() {
        clear_store();
        persist_asset(PersistedAsset::new("a1", "q42:Recording", None));
        persist_asset(PersistedAsset::new("a2", "q42:Recording", None));
        assert_eq!(asset_count(), 2);
        assert_eq!(list_asset_ids().len(), 2);
    }

    #[test]
    fn independent_aspects_preserved() {
        clear_store();
        persist_asset(PersistedAsset::new("a1", "q42:Recording", None));
        add_temporal_aspect(
            "a1",
            TemporalAspect {
                kind_iri: "q42:recordingDate".into(),
                seconds: 1000,
                nanoseconds: 0,
                duration_seconds: None,
                asserting_agent: Some("did:q42:bob".into()),
                confidence: Some(0.9),
            },
        );
        add_temporal_aspect(
            "a1",
            TemporalAspect {
                kind_iri: "q42:publicationDate".into(),
                seconds: 2000,
                nanoseconds: 0,
                duration_seconds: None,
                asserting_agent: Some("did:q42:carol".into()),
                confidence: Some(0.8),
            },
        );
        let asset = get_asset("a1").unwrap();
        assert_eq!(asset.temporal_aspects.len(), 2);
        // Both aspects preserved independently.
        assert_eq!(
            asset.temporal_aspects[0].asserting_agent,
            Some("did:q42:bob".into())
        );
        assert_eq!(
            asset.temporal_aspects[1].asserting_agent,
            Some("did:q42:carol".into())
        );
    }
}
