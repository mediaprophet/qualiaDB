//! QualiaPortal theatre adapter: evaluate a VibeScript scene, pack GPU nodes.
//!
//! Synth produce is `capability.invoke("hypermedia.*")` (148 morphisms) into
//! HMC / .10d / .emf. This file does not mint morphisms and is not a produce
//! source. Scene construction lives in `docs/vibe/cinema-graph.vibe`.
//!
//! Ingest (parse RDF, intern names, eval vibe) may allocate. GPU packing uses
//! [`super::rdf_holograph::layout_placed_into`] on the caller-owned scene.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use vibe::Value;

use crate::render::rdf_holograph::{
    classify_resource_kind, display_local_name, fill_label_names, intern_kind, layout_nquins_into,
    layout_placed_into, PlacedHolograph, RdfHolographScene,
};
use crate::sparql_library::rdf_formats::{parse_rdf, QuinCollector, RdfFormat};
use crate::vibe_host::PoetSnapshot;
use crate::{q_hash, NQuin};

const RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
const FLAG_SUBJECT: u8 = 1;
const FLAG_CLASS: u8 = 2;

/// Bundled theatre scene. Hot-edit fetches `docs/vibe/cinema-graph.vibe` as text
/// and passes it in; this is the fallback when the fetch is empty.
pub const CINEMA_GRAPH_VIBE: &str = include_str!("../../../../docs/vibe/cinema-graph.vibe");

/// Camera pose returned with a vibe-constructed theatre.
#[derive(Debug, Clone, Copy)]
pub struct TheatreCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}

impl Default for TheatreCamera {
    fn default() -> Self {
        Self {
            yaw: 0.55,
            pitch: 0.48,
            zoom: 0.0,
        }
    }
}

/// How the theatre was constructed. `"vibe"` is the authoring path.
pub type TheatreLayoutKind = &'static str;

struct InternedRdf {
    names: BTreeMap<u64, String>,
    quoted: BTreeSet<u64>,
}

fn intern_names_from_rdf_text(payload: &str) -> InternedRdf {
    let mut prefixes: BTreeMap<String, String> = BTreeMap::new();
    let mut quoted: BTreeSet<u64> = BTreeSet::new();
    for raw in payload.lines() {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        let rest = if let Some(r) = lower.strip_prefix("@prefix ") {
            &line[line.len() - r.len()..]
        } else if let Some(r) = lower.strip_prefix("prefix ") {
            &line[line.len() - r.len()..]
        } else {
            continue;
        };
        let Some((pfx, iri_part)) = rest.split_once(':') else {
            continue;
        };
        let pfx = pfx.trim();
        let Some(start) = iri_part.find('<') else {
            continue;
        };
        let Some(end) = iri_part[start + 1..].find('>') else {
            continue;
        };
        let iri = &iri_part[start + 1..start + 1 + end];
        prefixes.insert(pfx.to_string(), iri.to_string());
    }

    let mut names: BTreeMap<u64, String> = BTreeMap::new();
    let put = |map: &mut BTreeMap<u64, String>, lexical: &str| {
        if lexical.is_empty() {
            return;
        }
        let h = q_hash(lexical);
        let local = display_local_name(lexical);
        if local.is_empty() {
            return;
        }
        map.entry(h).or_insert_with(|| local.chars().take(24).collect());
    };

    let bytes = payload.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'<' {
            if let Some(rel) = payload[i + 1..].find('>') {
                put(&mut names, &payload[i + 1..i + 1 + rel]);
                i += rel + 2;
                continue;
            }
        }
        if c == b'"' {
            if let Some(rel) = payload[i + 1..].find('"') {
                let lex = &payload[i + 1..i + 1 + rel];
                put(&mut names, lex);
                if !lex.is_empty() {
                    quoted.insert(q_hash(lex));
                }
                i += rel + 2;
                continue;
            }
        }
        i += 1;
    }

    let mut token = String::new();
    for ch in payload.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' || ch == '-' {
            token.push(ch);
            continue;
        }
        if let Some((pfx, local)) = token.split_once(':') {
            if !pfx.is_empty()
                && !local.is_empty()
                && pfx != "http"
                && pfx != "https"
                && !pfx.starts_with('@')
            {
                if let Some(base) = prefixes.get(pfx) {
                    put(&mut names, &format!("{base}{local}"));
                } else {
                    put(&mut names, local);
                }
            }
        }
        token.clear();
    }
    InternedRdf { names, quoted }
}

fn resource_records(quins: &[NQuin], intern: &InternedRdf) -> (Value, Value) {
    let mut ids: BTreeMap<u64, (u32, u8)> = BTreeMap::new();
    for q in quins {
        let e = ids.entry(q.subject).or_insert((0, 0));
        e.0 = e.0.saturating_add(1);
        e.1 |= FLAG_SUBJECT;
        let e = ids.entry(q.object).or_insert((0, 0));
        e.0 = e.0.saturating_add(1);
        if q.predicate == RDF_TYPE {
            e.1 |= FLAG_CLASS;
        }
    }
    let mut resources = Vec::with_capacity(ids.len());
    for (id, (degree, flags)) in &ids {
        let kind = classify_resource_kind(
            *flags & FLAG_SUBJECT != 0,
            *flags & FLAG_CLASS != 0,
            *degree,
            intern.quoted.contains(id),
        );
        let name = intern.names.get(id).cloned().unwrap_or_default();
        let mut rec = BTreeMap::new();
        rec.insert("id".into(), Value::U64(*id));
        rec.insert("kind".into(), Value::String(kind.into()));
        rec.insert("name".into(), Value::String(name));
        rec.insert("degree".into(), Value::F64(*degree as f64));
        resources.push(Value::Record(rec));
    }
    let mut triples = Vec::with_capacity(quins.len());
    for q in quins {
        let mut rec = BTreeMap::new();
        rec.insert("subject".into(), Value::U64(q.subject));
        rec.insert("predicate".into(), Value::U64(q.predicate));
        rec.insert("object".into(), Value::U64(q.object));
        rec.insert("is_type".into(), Value::Bool(q.predicate == RDF_TYPE));
        rec.insert(
            "name".into(),
            Value::String(intern.names.get(&q.predicate).cloned().unwrap_or_default()),
        );
        triples.push(Value::Record(rec));
    }
    (Value::List(resources), Value::List(triples))
}

fn rec_f64(v: &Value, key: &str) -> f64 {
    match v {
        Value::Record(m) => match m.get(key) {
            Some(Value::F64(n)) => *n,
            Some(Value::I64(n)) => *n as f64,
            Some(Value::U64(n)) => *n as f64,
            _ => 0.0,
        },
        _ => 0.0,
    }
}

fn rec_u64(v: &Value, key: &str) -> u64 {
    match v {
        Value::Record(m) => match m.get(key) {
            Some(Value::U64(n)) => *n,
            Some(Value::I64(n)) if *n >= 0 => *n as u64,
            Some(Value::F64(n)) if *n >= 0.0 && n.fract() == 0.0 => *n as u64,
            _ => 0,
        },
        _ => 0,
    }
}

fn rec_bool(v: &Value, key: &str) -> bool {
    match v {
        Value::Record(m) => matches!(m.get(key), Some(Value::Bool(true))),
        _ => false,
    }
}

fn rec_str<'a>(v: &'a Value, key: &str) -> &'a str {
    match v {
        Value::Record(m) => match m.get(key) {
            Some(Value::String(s)) => s.as_str(),
            _ => "",
        },
        _ => "",
    }
}

fn rec_list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    match v {
        Value::Record(m) => match m.get(key) {
            Some(Value::List(xs)) => xs.as_slice(),
            _ => &[],
        },
        _ => &[],
    }
}

fn placed_from_record(v: &Value) -> PlacedHolograph {
    let mut p = PlacedHolograph {
        id: rec_u64(v, "id"),
        kind: intern_kind(rec_str(v, "kind")),
        degree: rec_f64(v, "degree") as u32,
        position: [
            rec_f64(v, "x") as f32,
            rec_f64(v, "y") as f32,
            rec_f64(v, "z") as f32,
        ],
        endpoint: [
            rec_f64(v, "x2") as f32,
            rec_f64(v, "y2") as f32,
            rec_f64(v, "z2") as f32,
        ],
        scale: rec_f64(v, "scale") as f32,
        is_type: rec_bool(v, "is_type"),
        ..PlacedHolograph::default()
    };
    if p.scale <= 0.0 {
        p.scale = match p.kind {
            "class" => 0.46,
            "literal" => 0.20,
            "predicate" => 0.05,
            "plane" => 0.85,
            _ => 0.30,
        };
    }
    p.set_name(rec_str(v, "name"));
    p
}

fn camera_from_value(v: &Value) -> TheatreCamera {
    let mut cam = TheatreCamera::default();
    if matches!(v, Value::Record(_)) {
        cam.yaw = rec_f64(v, "yaw") as f32;
        cam.pitch = rec_f64(v, "pitch") as f32;
        cam.zoom = rec_f64(v, "zoom") as f32;
    }
    cam
}

fn fill_scene_names(scene: &mut RdfHolographScene, names: &BTreeMap<u64, String>) {
    fill_label_names(scene, |id| names.get(&id).map(String::as_str));
}

fn eval_construct_theatre(src: &str, resources: Value, triples: Value) -> Result<Value, String> {
    let mut host = PoetSnapshot::default();
    host.eval_fn(src, "construct_theatre", vec![resources, triples])
        .map_err(|d| d.message)
}

/// Parse RDF, evaluate `construct_theatre`, pack the placed scene.
/// Falls back to [`layout_nquins_into`] when the vibe fails, so the holograph
/// GPU path stays available.
pub fn layout_rdf_with_vibe_into(
    content_type: &str,
    payload: &str,
    vibe_src: &str,
    scene: &mut RdfHolographScene,
) -> Result<(TheatreCamera, TheatreLayoutKind), &'static str> {
    let format = RdfFormat::from_media_type(content_type)
        .or_else(|| RdfFormat::from_str(content_type))
        .ok_or("unsupported RDF Content-Type / format")?;
    let mut collector = QuinCollector::new();
    parse_rdf(format, Cursor::new(payload.as_bytes()), 0, &mut collector)
        .map_err(|_| "rdf_holograph_parse_failed")?;
    let quins = collector.as_slice();
    if quins.is_empty() {
        return Err("rdf_holograph_empty_graph");
    }
    let intern = intern_names_from_rdf_text(payload);
    let (resources, triples) = resource_records(quins, &intern);
    let src = if vibe_src.trim().is_empty() {
        CINEMA_GRAPH_VIBE
    } else {
        vibe_src
    };
    match eval_construct_theatre(src, resources, triples) {
        Ok(theatre) => {
            let mut placed: Vec<PlacedHolograph> = Vec::new();
            for key in ["nodes", "edges", "planes"] {
                for item in rec_list(&theatre, key) {
                    placed.push(placed_from_record(item));
                }
            }
            if placed.is_empty() {
                layout_nquins_into(quins, scene);
                fill_scene_names(scene, &intern.names);
                scene.truncated = scene.truncated || collector.truncated;
                return Ok((TheatreCamera::default(), "nquins"));
            }
            let triple_count = rec_list(&theatre, "edges").len() as u32;
            layout_placed_into(&placed, triple_count.max(quins.len() as u32), scene);
            fill_scene_names(scene, &intern.names);
            scene.truncated = scene.truncated || collector.truncated;
            let mut cam = camera_from_value(match &theatre {
                Value::Record(m) => m.get("camera").unwrap_or(&Value::Null),
                _ => &Value::Null,
            });
            if cam.yaw == 0.0 && cam.pitch == 0.0 {
                cam = TheatreCamera::default();
            }
            Ok((cam, "vibe"))
        }
        Err(_) => {
            layout_nquins_into(quins, scene);
            fill_scene_names(scene, &intern.names);
            scene.truncated = scene.truncated || collector.truncated;
            Ok((TheatreCamera::default(), "nquins"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOAF: &str = r#"@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix ex: <http://ex/> .
ex:alice foaf:knows ex:bob .
ex:bob foaf:knows ex:carol .
ex:carol foaf:knows ex:alice .
ex:alice foaf:name "Alice" .
"#;

    #[test]
    fn cinema_graph_vibe_parses_checks_and_evals() {
        let prog = vibe::parse_program(CINEMA_GRAPH_VIBE)
            .unwrap_or_else(|e| panic!("parse cinema-graph.vibe: {e}"));
        vibe::check_program(&prog).unwrap_or_else(|e| panic!("check cinema-graph.vibe: {e}"));
        let mut resources = Vec::new();
        let mut rec = BTreeMap::new();
        rec.insert("id".into(), Value::U64(1));
        rec.insert("kind".into(), Value::String("instance".into()));
        rec.insert("name".into(), Value::String("alice".into()));
        rec.insert("degree".into(), Value::F64(1.0));
        resources.push(Value::Record(rec));
        let mut host = PoetSnapshot::default();
        host.eval_fn(
            CINEMA_GRAPH_VIBE,
            "construct_theatre",
            vec![Value::List(resources), Value::List(vec![])],
        )
        .unwrap_or_else(|e| panic!("eval construct_theatre: {e}"));
    }

    #[test]
    fn intern_names_recovers_local_and_literal() {
        let intern = intern_names_from_rdf_text(FOAF);
        assert!(intern.names.values().any(|n| n == "alice"));
        assert!(intern.names.values().any(|n| n == "Alice"));
        assert!(intern.names.values().any(|n| n == "knows"));
        assert!(intern.quoted.contains(&q_hash("Alice")));
        assert!(!intern.quoted.contains(&q_hash("http://ex/alice")));
    }

    const STAR: &str = r#"@prefix ex: <http://ex/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:nucleus rdf:type ex:Core .
ex:nucleus ex:emits ex:photon .
ex:nucleus ex:binds ex:electron .
"#;

    #[test]
    fn cinema_vibe_star_types_iri_objects_as_instances() {
        let mut scene = Box::new(RdfHolographScene::empty());
        let (_, kind) =
            layout_rdf_with_vibe_into("text/turtle", STAR, CINEMA_GRAPH_VIBE, &mut scene)
                .expect("layout");
        assert_eq!(kind, "vibe");
        assert!(scene
            .labels()
            .iter()
            .any(|l| l.name_str() == "electron" && l.kind == "instance"));
        assert!(scene
            .labels()
            .iter()
            .any(|l| l.name_str() == "Core" && l.kind == "class"));
        assert!(scene.labels().iter().any(|l| l.kind == "plane"));
    }

    #[test]
    fn cinema_vibe_places_foaf_as_named_graph() {
        let mut scene = Box::new(RdfHolographScene::empty());
        let (cam, kind) =
            layout_rdf_with_vibe_into("text/turtle", FOAF, CINEMA_GRAPH_VIBE, &mut scene)
                .expect("layout");
        assert_eq!(kind, "vibe", "construct_theatre must author the theatre");
        assert!(scene.node_count() >= 5);
        assert!(scene.labels().iter().any(|l| l.name_str() == "alice"));
        assert!(scene.labels().iter().any(|l| l.kind == "predicate"));
        let resources: Vec<_> = scene
            .labels()
            .iter()
            .filter(|l| l.kind != "predicate" && l.kind != "plane")
            .collect();
        assert!(resources.len() >= 3);
        let mut min_sep = f32::MAX;
        for i in 0..resources.len() {
            for j in (i + 1)..resources.len() {
                let d = (resources[i].position[0] - resources[j].position[0]).hypot(
                    (resources[i].position[2] - resources[j].position[2])
                        .hypot(resources[i].position[1] - resources[j].position[1]),
                );
                min_sep = min_sep.min(d);
            }
        }
        assert!(min_sep > 0.35, "FOAF resources must sit apart (sep={min_sep})");
        assert!(cam.yaw > 0.3 && cam.pitch > 0.3);
    }
}
