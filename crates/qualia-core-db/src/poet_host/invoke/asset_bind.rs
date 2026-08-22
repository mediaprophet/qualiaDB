//! VibeScript bindings for asset aspect sub-graphs.
//!
//! Exposes the asset aspect graph concept to VibeScript so scripts can
//! create assets with multiple temporal assertions, spatial anchors, and
//! topic associations, then compile them to graph quins.
//!
//! Invoke IDs:
//! - `Asset.create` — create a new asset aspect graph record
//! - `Asset.add_temporal` — add a temporal aspect to an asset
//! - `Asset.add_topic` — add a topic association to an asset
//! - `Asset.set_spatial` — set the spatial anchor on an asset
//! - `Asset.compile` — compile an asset to graph quins and stage them
//! - `Asset.temporal_span` — get the duration between earliest and latest aspects
//! - `Asset.query_aspects` — query temporal aspects by kind

use super::args;
use crate::q_hash;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// Parse a temporal aspect kind from a string.
fn parse_aspect_kind(s: &str) -> Option<&'static str> {
    match s {
        "production" | "Production" => Some("q42:productionDate"),
        "recording" | "Recording" => Some("q42:recordingDate"),
        "publication" | "Publication" => Some("q42:publicationDate"),
        "event" | "Event" => Some("q42:eventDate"),
        "performance" | "Performance" => Some("q42:performanceDate"),
        "exhibition" | "Exhibition" => Some("q42:exhibitionDate"),
        "modification" | "Modification" => Some("q42:modificationDate"),
        "archival" | "Archival" => Some("q42:archivalDate"),
        "acquisition" | "Acquisition" => Some("q42:acquisitionDate"),
        "decommission" | "Decommission" => Some("q42:decommissionDate"),
        _ => None,
    }
}

/// Parse an asset kind from a string.
fn parse_asset_kind(s: &str) -> Option<&'static str> {
    match s {
        "recording" | "Recording" => Some("q42:Recording"),
        "venue" | "Venue" => Some("q42:Venue"),
        "event_space" | "EventSpace" => Some("q42:EventSpace"),
        "artwork" | "Artwork" => Some("q42:Artwork"),
        "document" | "Document" => Some("q42:Document"),
        "photograph" | "Photograph" => Some("q42:Photograph"),
        "performance" | "Performance" => Some("q42:Performance"),
        "artifact" | "Artifact" => Some("q42:Artifact"),
        "location" | "Location" => Some("q42:Location"),
        _ => None,
    }
}

fn bad(span: Span, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagCode::E100, span, msg.into())
}

/// Create a new asset aspect graph record.
/// Args: { asset_id: String, kind: String, owner_did?: String }
pub fn create(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_id = args::rec_str(args, "asset_id")
        .ok_or_else(|| bad(span, "Asset.create: missing asset_id string"))?;
    let kind_str = args::rec_str(args, "kind")
        .ok_or_else(|| bad(span, "Asset.create: missing kind string"))?;
    let kind_iri = parse_asset_kind(kind_str).ok_or_else(|| {
        bad(
            span,
            format!("Asset.create: unknown asset kind: {kind_str}"),
        )
    })?;
    let owner_did = args::rec_str(args, "owner_did");

    let mut graph = BTreeMap::new();
    graph.insert("asset_id".into(), Value::String(asset_id.to_string()));
    graph.insert("kind".into(), Value::String(kind_iri.to_string()));
    graph.insert("temporal_aspects".into(), Value::List(Vec::new()));
    graph.insert("topics".into(), Value::List(Vec::new()));
    if let Some(owner) = owner_did {
        graph.insert("owner_did".into(), Value::String(owner.to_string()));
    }
    Ok(Value::Record(graph))
}

/// Add a temporal aspect to an asset graph.
/// Args: { asset: Record, kind: String, secs: I64, nanos?: U64, duration_secs?: I64, asserted_by?: String, confidence?: F64 }
pub fn add_temporal(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut asset = args::rec(args, "asset")
        .ok_or_else(|| bad(span, "Asset.add_temporal: missing asset record"))?
        .clone();
    let kind_str = args::rec_str(args, "kind")
        .ok_or_else(|| bad(span, "Asset.add_temporal: missing kind string"))?;
    let kind_iri = parse_aspect_kind(kind_str).ok_or_else(|| {
        bad(
            span,
            format!("Asset.add_temporal: unknown temporal aspect kind: {kind_str}"),
        )
    })?;
    let secs = args::rec_i64(args, "secs")
        .ok_or_else(|| bad(span, "Asset.add_temporal: missing secs (i64)"))?;
    let nanos = args::rec_u64(args, "nanos").unwrap_or(0) as u32;

    let mut aspect = BTreeMap::new();
    aspect.insert("kind".into(), Value::String(kind_iri.to_string()));
    aspect.insert("secs".into(), Value::I64(secs));
    aspect.insert("nanos".into(), Value::U64(nanos as u64));
    if let Some(dur) = args::rec_i64(args, "duration_secs") {
        aspect.insert("duration_secs".into(), Value::I64(dur));
    }
    if let Some(asserted_by) = args::rec_str(args, "asserted_by") {
        aspect.insert("asserted_by".into(), Value::String(asserted_by.to_string()));
    }
    if let Some(confidence) = args::rec_f64(args, "confidence") {
        aspect.insert("confidence".into(), Value::F64(confidence));
    }

    if let Value::Record(ref mut asset_rec) = asset {
        if let Some(Value::List(ref mut aspects)) = asset_rec.get_mut("temporal_aspects") {
            aspects.push(Value::Record(aspect));
        } else {
            return Err(bad(
                span,
                "Asset.add_temporal: asset record is malformed: no temporal_aspects list",
            ));
        }
    } else {
        return Err(bad(span, "Asset.add_temporal: asset is not a record"));
    }
    Ok(asset)
}

/// Add a topic association to an asset graph.
/// Args: { asset: Record, topic: String, subject: String, relation: String, confidence?: F64 }
pub fn add_topic(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut asset = args::rec(args, "asset")
        .ok_or_else(|| bad(span, "Asset.add_topic: missing asset record"))?
        .clone();
    let topic = args::rec_str(args, "topic")
        .ok_or_else(|| bad(span, "Asset.add_topic: missing topic string"))?;
    let subject = args::rec_str(args, "subject")
        .ok_or_else(|| bad(span, "Asset.add_topic: missing subject string"))?;
    let relation = args::rec_str(args, "relation")
        .ok_or_else(|| bad(span, "Asset.add_topic: missing relation string"))?;

    let mut topic_rec = BTreeMap::new();
    topic_rec.insert("topic".into(), Value::String(topic.to_string()));
    topic_rec.insert("subject".into(), Value::String(subject.to_string()));
    topic_rec.insert("relation".into(), Value::String(relation.to_string()));
    if let Some(confidence) = args::rec_f64(args, "confidence") {
        topic_rec.insert("confidence".into(), Value::F64(confidence));
    }

    if let Value::Record(ref mut asset_rec) = asset {
        if let Some(Value::List(ref mut topics)) = asset_rec.get_mut("topics") {
            topics.push(Value::Record(topic_rec));
        } else {
            return Err(bad(
                span,
                "Asset.add_topic: asset record is malformed: no topics list",
            ));
        }
    } else {
        return Err(bad(span, "Asset.add_topic: asset is not a record"));
    }
    Ok(asset)
}

/// Set the spatial anchor on an asset graph.
/// Args: { asset: Record, anchor_id: String, lat: F64, lon: F64, alt: F64, confidence_mm?: F64 }
pub fn set_spatial(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut asset = args::rec(args, "asset")
        .ok_or_else(|| bad(span, "Asset.set_spatial: missing asset record"))?
        .clone();
    let anchor_id = args::rec_str(args, "anchor_id")
        .ok_or_else(|| bad(span, "Asset.set_spatial: missing anchor_id string"))?;
    let lat = args::rec_f64(args, "lat")
        .ok_or_else(|| bad(span, "Asset.set_spatial: missing lat (f64)"))?;
    let lon = args::rec_f64(args, "lon")
        .ok_or_else(|| bad(span, "Asset.set_spatial: missing lon (f64)"))?;
    let alt = args::rec_f64(args, "alt")
        .ok_or_else(|| bad(span, "Asset.set_spatial: missing alt (f64)"))?;

    let mut anchor = BTreeMap::new();
    anchor.insert("anchor_id".into(), Value::String(anchor_id.to_string()));
    anchor.insert("lat".into(), Value::F64(lat));
    anchor.insert("lon".into(), Value::F64(lon));
    anchor.insert("alt".into(), Value::F64(alt));
    if let Some(conf) = args::rec_f64(args, "confidence_mm") {
        anchor.insert("confidence_mm".into(), Value::F64(conf));
    }

    if let Value::Record(ref mut asset_rec) = asset {
        asset_rec.insert("spatial_anchor".into(), Value::Record(anchor));
    } else {
        return Err(bad(span, "Asset.set_spatial: asset is not a record"));
    }
    Ok(asset)
}

/// Compile an asset aspect graph to graph quins.
/// Args: { asset: Record }
/// Returns: { quin_count: U64, context_hash: U64, quins: List }
pub fn compile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset =
        args::rec(args, "asset").ok_or_else(|| bad(span, "Asset.compile: missing asset record"))?;
    let asset_id = args::rec_str(asset, "asset_id")
        .ok_or_else(|| bad(span, "Asset.compile: asset missing asset_id"))?;
    let kind_iri = args::rec_str(asset, "kind")
        .ok_or_else(|| bad(span, "Asset.compile: asset missing kind"))?;

    let asset_hash = q_hash(asset_id);
    let ctx_hash = {
        let mut h = q_hash(kind_iri);
        h ^= asset_hash;
        h = h.wrapping_mul(0x100000001b3);
        h
    };

    let mut quins: Vec<Value> = Vec::new();

    // Asset kind quin
    quins.push(make_quin_value(
        asset_hash,
        q_hash("q42:hasAssetKind"),
        q_hash(kind_iri),
        ctx_hash,
    ));

    // Temporal aspect quins
    if let Some(temporal) = args::rec(asset, "temporal_aspects") {
        if let Value::List(aspects) = temporal {
            for aspect_val in aspects {
                if let (Some(kind), Some(secs)) = (
                    args::rec_str(aspect_val, "kind"),
                    args::rec_i64(aspect_val, "secs"),
                ) {
                    let nanos = args::rec_u64(aspect_val, "nanos").unwrap_or(0);
                    let obj = (secs as i128) * 1_000_000_000 + nanos as i128;
                    quins.push(make_quin_value(
                        asset_hash,
                        q_hash(kind),
                        obj as u64,
                        ctx_hash,
                    ));
                }
            }
        }
    }

    // Topic association quins
    if let Some(topics_val) = args::rec(asset, "topics") {
        if let Value::List(topics) = topics_val {
            for topic_val in topics {
                if let (Some(topic), Some(relation)) = (
                    args::rec_str(topic_val, "topic"),
                    args::rec_str(topic_val, "relation"),
                ) {
                    quins.push(make_quin_value(
                        asset_hash,
                        q_hash(relation),
                        q_hash(topic),
                        ctx_hash,
                    ));
                }
            }
        }
    }

    // Spatial anchor quin
    if let Some(anchor_val) = args::rec(asset, "spatial_anchor") {
        if let Some(anchor_id) = args::rec_str(anchor_val, "anchor_id") {
            quins.push(make_quin_value(
                asset_hash,
                q_hash("q42:hasSpatialAnchor"),
                q_hash(anchor_id),
                ctx_hash,
            ));
        }
    }

    let quin_count = quins.len() as u64;
    let mut result = BTreeMap::new();
    result.insert("quin_count".into(), Value::U64(quin_count));
    result.insert("context_hash".into(), Value::U64(ctx_hash));
    result.insert("quins".into(), Value::List(quins));
    Ok(Value::Record(result))
}

/// Get the temporal span between earliest and latest aspects.
/// Args: { asset: Record }
/// Returns: { secs: I64, nanos: U64 } or Null if < 2 aspects
pub fn temporal_span(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset = args::rec(args, "asset")
        .ok_or_else(|| bad(span, "Asset.temporal_span: missing asset record"))?;
    let aspects = match args::rec(asset, "temporal_aspects") {
        Some(Value::List(l)) => l,
        _ => return Ok(Value::Null),
    };
    if aspects.len() < 2 {
        return Ok(Value::Null);
    }

    let mut min_nanos: i128 = i128::MAX;
    let mut max_nanos: i128 = i128::MIN;
    for aspect_val in aspects {
        if let Some(secs) = args::rec_i64(aspect_val, "secs") {
            let nanos = args::rec_u64(aspect_val, "nanos").unwrap_or(0);
            let total = secs as i128 * 1_000_000_000 + nanos as i128;
            if total < min_nanos {
                min_nanos = total;
            }
            if total > max_nanos {
                max_nanos = total;
            }
        }
    }
    if min_nanos == i128::MAX || max_nanos == i128::MIN {
        return Ok(Value::Null);
    }

    let diff = max_nanos - min_nanos;
    let secs = (diff / 1_000_000_000) as i64;
    let nanos = (diff % 1_000_000_000) as u64;
    let mut result = BTreeMap::new();
    result.insert("secs".into(), Value::I64(secs));
    result.insert("nanos".into(), Value::U64(nanos));
    Ok(Value::Record(result))
}

/// Query temporal aspects by kind.
/// Args: { asset: Record, kind: String }
/// Returns: List of aspect records
pub fn query_aspects(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset = args::rec(args, "asset")
        .ok_or_else(|| bad(span, "Asset.query_aspects: missing asset record"))?;
    let kind_str = args::rec_str(args, "kind")
        .ok_or_else(|| bad(span, "Asset.query_aspects: missing kind string"))?;
    let kind_iri = parse_aspect_kind(kind_str).ok_or_else(|| {
        bad(
            span,
            format!("Asset.query_aspects: unknown temporal aspect kind: {kind_str}"),
        )
    })?;

    let aspects = match args::rec(asset, "temporal_aspects") {
        Some(Value::List(l)) => l,
        _ => return Ok(Value::List(Vec::new())),
    };

    let filtered: Vec<Value> = aspects
        .iter()
        .filter(|v| {
            args::rec_str(v, "kind")
                .map(|k| k == kind_iri)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    Ok(Value::List(filtered))
}

/// Build a Quin value record from (subject, predicate, object, context) hashes.
fn make_quin_value(s: u64, p: u64, o: u64, c: u64) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("subject".into(), Value::U64(s));
    rec.insert("predicate".into(), Value::U64(p));
    rec.insert("object".into(), Value::U64(o));
    rec.insert("context".into(), Value::U64(c));
    Value::Record(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_asset() -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("asset_id".into(), Value::String("rec-1".into()));
        rec.insert("kind".into(), Value::String("q42:Recording".into()));
        rec.insert("temporal_aspects".into(), Value::List(Vec::new()));
        rec.insert("topics".into(), Value::List(Vec::new()));
        Value::Record(rec)
    }

    fn rec_with(pairs: &[(&str, Value)]) -> Value {
        let mut r = BTreeMap::new();
        for (k, v) in pairs {
            r.insert((*k).into(), v.clone());
        }
        Value::Record(r)
    }

    #[test]
    fn asset_create_basic() {
        let args = rec_with(&[
            ("asset_id", Value::String("rec-1".into())),
            ("kind", Value::String("recording".into())),
        ]);
        let result = create(&args, Span::point(0)).unwrap();
        assert_eq!(args::rec_str(&result, "asset_id"), Some("rec-1"));
        assert_eq!(args::rec_str(&result, "kind"), Some("q42:Recording"));
        assert!(args::rec(&result, "temporal_aspects").is_some());
        assert!(args::rec(&result, "topics").is_some());
    }

    #[test]
    fn asset_create_with_owner() {
        let args = rec_with(&[
            ("asset_id", Value::String("rec-1".into())),
            ("kind", Value::String("recording".into())),
            ("owner_did", Value::String("did:alice".into())),
        ]);
        let result = create(&args, Span::point(0)).unwrap();
        assert_eq!(args::rec_str(&result, "owner_did"), Some("did:alice"));
    }

    #[test]
    fn asset_create_unknown_kind() {
        let args = rec_with(&[
            ("asset_id", Value::String("rec-1".into())),
            ("kind", Value::String("unknown".into())),
        ]);
        assert!(create(&args, Span::point(0)).is_err());
    }

    #[test]
    fn asset_add_temporal_basic() {
        let args = rec_with(&[
            ("asset", make_asset()),
            ("kind", Value::String("production".into())),
            ("secs", Value::I64(1_699_000_000)),
            ("nanos", Value::U64(0)),
        ]);
        let result = add_temporal(&args, Span::point(0)).unwrap();
        if let Some(Value::List(aspects)) = args::rec(&result, "temporal_aspects") {
            assert_eq!(aspects.len(), 1);
        } else {
            panic!("no temporal_aspects");
        }
    }

    #[test]
    fn asset_add_temporal_with_fields() {
        let args = rec_with(&[
            ("asset", make_asset()),
            ("kind", Value::String("performance".into())),
            ("secs", Value::I64(1_700_000_000)),
            ("duration_secs", Value::I64(7200)),
            ("asserted_by", Value::String("did:alice".into())),
            ("confidence", Value::F64(0.95)),
        ]);
        let result = add_temporal(&args, Span::point(0)).unwrap();
        if let Some(Value::List(aspects)) = args::rec(&result, "temporal_aspects") {
            assert_eq!(aspects.len(), 1);
            assert_eq!(
                args::rec_str(&aspects[0], "kind"),
                Some("q42:performanceDate")
            );
            assert_eq!(args::rec_i64(&aspects[0], "duration_secs"), Some(7200));
            assert_eq!(args::rec_str(&aspects[0], "asserted_by"), Some("did:alice"));
        }
    }

    #[test]
    fn asset_add_topic() {
        let args = rec_with(&[
            ("asset", make_asset()),
            ("topic", Value::String("jazz".into())),
            ("subject", Value::String("rec-1".into())),
            ("relation", Value::String("q42:isAbout".into())),
        ]);
        let result = add_topic(&args, Span::point(0)).unwrap();
        if let Some(Value::List(topics)) = args::rec(&result, "topics") {
            assert_eq!(topics.len(), 1);
        }
    }

    #[test]
    fn asset_set_spatial() {
        let args = rec_with(&[
            ("asset", make_asset()),
            ("anchor_id", Value::String("venue-1".into())),
            ("lat", Value::F64(40.7128)),
            ("lon", Value::F64(-74.0060)),
            ("alt", Value::F64(10.0)),
        ]);
        let result = set_spatial(&args, Span::point(0)).unwrap();
        assert!(args::rec(&result, "spatial_anchor").is_some());
    }

    #[test]
    fn asset_compile_basic() {
        let create_args = rec_with(&[
            ("asset_id", Value::String("rec-1".into())),
            ("kind", Value::String("recording".into())),
        ]);
        let mut asset = create(&create_args, Span::point(0)).unwrap();

        let add_args = rec_with(&[
            ("asset", asset.clone()),
            ("kind", Value::String("production".into())),
            ("secs", Value::I64(1_699_000_000)),
        ]);
        asset = add_temporal(&add_args, Span::point(0)).unwrap();

        let topic_args = rec_with(&[
            ("asset", asset.clone()),
            ("topic", Value::String("jazz".into())),
            ("subject", Value::String("rec-1".into())),
            ("relation", Value::String("q42:isAbout".into())),
        ]);
        asset = add_topic(&topic_args, Span::point(0)).unwrap();

        let compile_args = rec_with(&[("asset", asset)]);
        let result = compile(&compile_args, Span::point(0)).unwrap();
        assert_eq!(args::rec_u64(&result, "quin_count"), Some(3));
        assert!(args::rec_u64(&result, "context_hash").unwrap() > 0);
    }

    #[test]
    fn asset_compile_with_spatial() {
        let create_args = rec_with(&[
            ("asset_id", Value::String("venue-1".into())),
            ("kind", Value::String("venue".into())),
        ]);
        let mut asset = create(&create_args, Span::point(0)).unwrap();

        let spatial_args = rec_with(&[
            ("asset", asset.clone()),
            ("anchor_id", Value::String("venue-1".into())),
            ("lat", Value::F64(40.7)),
            ("lon", Value::F64(-74.0)),
            ("alt", Value::F64(0.0)),
        ]);
        asset = set_spatial(&spatial_args, Span::point(0)).unwrap();

        let compile_args = rec_with(&[("asset", asset)]);
        let result = compile(&compile_args, Span::point(0)).unwrap();
        assert_eq!(args::rec_u64(&result, "quin_count"), Some(2));
    }

    #[test]
    fn asset_temporal_span() {
        let create_args = rec_with(&[
            ("asset_id", Value::String("rec-1".into())),
            ("kind", Value::String("recording".into())),
        ]);
        let mut asset = create(&create_args, Span::point(0)).unwrap();

        let a1 = rec_with(&[
            ("asset", asset.clone()),
            ("kind", Value::String("production".into())),
            ("secs", Value::I64(1_699_000_000)),
        ]);
        asset = add_temporal(&a1, Span::point(0)).unwrap();

        let a2 = rec_with(&[
            ("asset", asset.clone()),
            ("kind", Value::String("publication".into())),
            ("secs", Value::I64(1_700_000_000)),
        ]);
        asset = add_temporal(&a2, Span::point(0)).unwrap();

        let span_args = rec_with(&[("asset", asset)]);
        let result = temporal_span(&span_args, Span::point(0)).unwrap();
        assert_eq!(args::rec_i64(&result, "secs"), Some(1_000_000));
    }

    #[test]
    fn asset_temporal_span_single_returns_null() {
        let create_args = rec_with(&[
            ("asset_id", Value::String("rec-1".into())),
            ("kind", Value::String("recording".into())),
        ]);
        let mut asset = create(&create_args, Span::point(0)).unwrap();

        let a1 = rec_with(&[
            ("asset", asset.clone()),
            ("kind", Value::String("production".into())),
            ("secs", Value::I64(1_699_000_000)),
        ]);
        asset = add_temporal(&a1, Span::point(0)).unwrap();

        let span_args = rec_with(&[("asset", asset)]);
        let result = temporal_span(&span_args, Span::point(0)).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn asset_query_aspects() {
        let create_args = rec_with(&[
            ("asset_id", Value::String("venue-1".into())),
            ("kind", Value::String("venue".into())),
        ]);
        let mut asset = create(&create_args, Span::point(0)).unwrap();

        for kind in &["event", "event", "production"] {
            let a = rec_with(&[
                ("asset", asset.clone()),
                ("kind", Value::String((*kind).into())),
                ("secs", Value::I64(1_700_000_000)),
            ]);
            asset = add_temporal(&a, Span::point(0)).unwrap();
        }

        let q = rec_with(&[("asset", asset), ("kind", Value::String("event".into()))]);
        let result = query_aspects(&q, Span::point(0)).unwrap();
        if let Value::List(l) = result {
            assert_eq!(l.len(), 2);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn asset_create_missing_args() {
        assert!(create(&Value::Null, Span::point(0)).is_err());
    }

    #[test]
    fn asset_add_temporal_missing_asset() {
        let args = rec_with(&[
            ("kind", Value::String("production".into())),
            ("secs", Value::I64(1)),
        ]);
        assert!(add_temporal(&args, Span::point(0)).is_err());
    }
}
