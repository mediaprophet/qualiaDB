//! Natural-language design documents → NQuin graph + Tensor10D layout.
//!
//! General-purpose product/assembly representation for the Qualia Portal demo.
//! Geometry stays as tensor coordinates; semantics live in parts, relations, and quins.

use crate::tensor::Tensor10D;
use crate::{q_hash, NQuin};
use serde::{Deserialize, Serialize};

pub const DESIGN_TYPE: &str = "qualia.design";
pub const DESIGN_VERSION: &str = "1.0.0";
pub const MAX_DESIGN_PARTS: usize = 64;
pub const MAX_DESIGN_RELATIONS: usize = 128;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignPart {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub installer: String,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub pos: Option<[f32; 3]>,
    #[serde(default = "default_state")]
    pub state: String,
    #[serde(default = "default_intensity")]
    pub intensity: f32,
    #[serde(default)]
    pub reasons: Vec<String>,
}

fn default_state() -> String {
    "default".to_string()
}

fn default_intensity() -> f32 {
    0.65
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignRelation {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub relation_type: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparqlContextHit {
    pub endpoint: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub bindings: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignDocument {
    #[serde(rename = "type", default = "default_design_type")]
    pub doc_type: String,
    #[serde(default = "default_design_version")]
    pub version: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub parts: Vec<DesignPart>,
    #[serde(default)]
    pub relations: Vec<DesignRelation>,
    #[serde(default)]
    pub explanations: Vec<String>,
    #[serde(default)]
    pub sparql_context: Vec<SparqlContextHit>,
}

fn default_design_type() -> String {
    DESIGN_TYPE.to_string()
}

fn default_design_version() -> String {
    DESIGN_VERSION.to_string()
}

#[derive(Debug, PartialEq, Eq)]
pub enum DesignEncodeError {
    TooManyParts,
    TooManyRelations,
    UnknownPartId,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesignEncodeStats {
    pub part_count: usize,
    pub relation_count: usize,
    pub tensor_count: usize,
    pub quin_count: usize,
    pub design_hash: String,
}

fn manifold_w(label: &str, role: &str) -> f32 {
    let key = format!("{label}:{role}").to_lowercase();
    let h = q_hash(&key);
    (h % 5) as f32
}

fn topology_v(role: &str, relation_count: usize) -> f32 {
    let r = role.to_lowercase();
    if r.contains("interface") || r.contains("mate") || r.contains("connector") {
        return 3.2;
    }
    if relation_count > 2 {
        return 2.5;
    }
    if r.contains("sensor") || r.contains("compute") || r.contains("smart") {
        return 1.5;
    }
    0.0
}

fn epistemic_q(state: &str, installer: &str) -> f32 {
    let s = state.to_lowercase();
    if s == "alert" || s == "uncertain" {
        return 0.42;
    }
    if !installer.is_empty() && installer != "user" && installer != "owner" {
        return 0.0;
    }
    if s == "highlighted" || s == "active" {
        return 0.12;
    }
    0.08
}

fn spectral_sigma(id: &str, idx: usize, total: usize) -> f32 {
    let base = (q_hash(id) % 10_000) as f32 / 10_000.0;
    base + (idx as f32 / total.max(1) as f32) * 0.15
}

fn auto_position(index: usize, total: usize) -> [f32; 3] {
    if total <= 1 {
        return [0.0, 0.0, 0.0];
    }
    let t = index as f32 / total as f32;
    let angle = t * std::f32::consts::TAU;
    let radius = 4.0 + (total as f32 * 0.15);
    [
        radius * angle.cos(),
        (index as f32 * 0.35) - (total as f32 * 0.15),
        radius * angle.sin(),
    ]
}

fn relation_midpoint(
    from: &[f32; 3],
    to: &[f32; 3],
) -> [f32; 3] {
    [
        (from[0] + to[0]) * 0.5,
        (from[1] + to[1]) * 0.5 + 0.25,
        (from[2] + to[2]) * 0.5,
    ]
}

/// Lay out parts in 10D tensor space and emit optional relation anchor tensors.
pub fn design_to_tensors(doc: &DesignDocument) -> Result<Vec<Tensor10D>, DesignEncodeError> {
    if doc.parts.len() > MAX_DESIGN_PARTS {
        return Err(DesignEncodeError::TooManyParts);
    }
    if doc.relations.len() > MAX_DESIGN_RELATIONS {
        return Err(DesignEncodeError::TooManyRelations);
    }

    let total = doc.parts.len();
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total);
    for (i, part) in doc.parts.iter().enumerate() {
        positions.push(part.pos.unwrap_or_else(|| auto_position(i, total)));
    }

    let mut id_to_index = std::collections::BTreeMap::new();
    for (i, part) in doc.parts.iter().enumerate() {
        id_to_index.insert(part.id.clone(), i);
    }

    let mut out = Vec::new();
    for (i, part) in doc.parts.iter().enumerate() {
        let [x, y, z] = positions[i];
        let rels = doc
            .relations
            .iter()
            .filter(|r| r.from == part.id || r.to == part.id)
            .count();
        let nx = (x / 10.0).clamp(-1.0, 1.0);
        let ny = (y / 10.0).clamp(-1.0, 1.0);
        let nz = (z / 10.0).clamp(-1.0, 1.0);
        let label = if part.label.is_empty() {
            &part.id
        } else {
            &part.label
        };
        out.push(Tensor10D::new(
            epistemic_q(&part.state, &part.installer),
            topology_v(&part.role, rels),
            manifold_w(label, &part.role),
            nx,
            ny,
            nz,
            i as f32 / total.max(1) as f32,
            part.intensity.clamp(0.0, 1.0),
            if part.installer.is_empty() { 0.0 } else { 2.0 },
            spectral_sigma(&part.id, i, total),
        ));
    }

    for (ri, rel) in doc.relations.iter().enumerate() {
        let Some(&fi) = id_to_index.get(&rel.from) else {
            return Err(DesignEncodeError::UnknownPartId);
        };
        let Some(&ti) = id_to_index.get(&rel.to) else {
            return Err(DesignEncodeError::UnknownPartId);
        };
        let mid = relation_midpoint(&positions[fi], &positions[ti]);
        let nx = (mid[0] / 10.0).clamp(-1.0, 1.0);
        let ny = (mid[1] / 10.0).clamp(-1.0, 1.0);
        let nz = (mid[2] / 10.0).clamp(-1.0, 1.0);
        out.push(Tensor10D::new(
            0.18,
            3.2,
            manifold_w(&rel.relation_type, "relation"),
            nx,
            ny,
            nz,
            0.5 + (ri as f32 * 0.01),
            0.55,
            1.0,
            spectral_sigma(&rel.relation_type, ri, doc.relations.len()),
        ));
    }

    Ok(out)
}

/// Lower design semantics to NQuin triples (parts + relations + design root).
pub fn design_to_quins(doc: &DesignDocument) -> Result<Vec<NQuin>, DesignEncodeError> {
    if doc.parts.len() > MAX_DESIGN_PARTS {
        return Err(DesignEncodeError::TooManyParts);
    }
    if doc.relations.len() > MAX_DESIGN_RELATIONS {
        return Err(DesignEncodeError::TooManyRelations);
    }

    let design_id = if doc.title.is_empty() {
        q_hash(&doc.prompt)
    } else {
        q_hash(&doc.title)
    };
    let ctx = q_hash("ctx:qualia-design");
    let pred_has_part = q_hash("q42:hasPart");
    let pred_relation = q_hash("q42:designRelation");
    let pred_type = q_hash("rdf:type");
    let type_design = q_hash("q42:Design");

    let mut quins = Vec::new();

    let mut root = NQuin::default();
    root.subject = design_id;
    root.predicate = pred_type;
    root.object = type_design;
    root.context = ctx;
    root.parity = root.subject ^ root.predicate ^ root.object ^ root.context;
    quins.push(root);

    for part in &doc.parts {
        let part_hash = q_hash(&part.id);
        let mut q = NQuin::default();
        q.subject = design_id;
        q.predicate = pred_has_part;
        q.object = part_hash;
        q.context = ctx;
        q.metadata = (part.intensity.clamp(0.0, 1.0) * 255.0) as u64;
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
        quins.push(q);
    }

    for rel in &doc.relations {
        let _ = doc
            .parts
            .iter()
            .find(|p| p.id == rel.from)
            .ok_or(DesignEncodeError::UnknownPartId)?;
        let _ = doc
            .parts
            .iter()
            .find(|p| p.id == rel.to)
            .ok_or(DesignEncodeError::UnknownPartId)?;

        let from_hash = q_hash(&rel.from);
        let to_hash = q_hash(&rel.to);
        let rel_hash = q_hash(&rel.relation_type);
        let packed = (from_hash & 0xFFFF_FFFF) | ((to_hash & 0xFFFF) << 32);

        let mut q = NQuin::default();
        q.subject = design_id;
        q.predicate = pred_relation;
        q.object = packed ^ rel_hash;
        q.context = ctx;
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        quins.push(q);
    }

    Ok(quins)
}

pub fn design_context_hash(doc: &DesignDocument) -> u64 {
    if doc.title.is_empty() {
        q_hash(&doc.prompt)
    } else {
        q_hash(&doc.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_switch() -> DesignDocument {
        DesignDocument {
            doc_type: DESIGN_TYPE.to_string(),
            version: DESIGN_VERSION.to_string(),
            title: "Smart switch".to_string(),
            summary: "Two-part assembly".to_string(),
            prompt: "smart switch".to_string(),
            parts: vec![
                DesignPart {
                    id: "base".into(),
                    label: "Wall base".into(),
                    role: "housing".into(),
                    installer: "electrician".into(),
                    components: vec![],
                    pos: Some([0.0, 0.0, 0.0]),
                    state: "active".into(),
                    intensity: 0.9,
                    reasons: vec!["mains wiring".into()],
                },
                DesignPart {
                    id: "face".into(),
                    label: "Smart face".into(),
                    role: "smart-module".into(),
                    installer: "user".into(),
                    components: vec!["mcu".into(), "motion-sensor".into()],
                    pos: Some([0.0, 0.4, 0.0]),
                    state: "highlighted".into(),
                    intensity: 0.75,
                    reasons: vec![],
                },
            ],
            relations: vec![DesignRelation {
                from: "face".into(),
                to: "base".into(),
                relation_type: "matesWith".into(),
                label: String::new(),
            }],
            explanations: vec!["Electrician installs base first".into()],
            sparql_context: vec![],
        }
    }

    #[test]
    fn design_to_tensors_includes_relation_anchor() {
        let doc = sample_switch();
        let tensors = design_to_tensors(&doc).unwrap();
        assert_eq!(tensors.len(), 3, "2 parts + 1 relation");
    }

    #[test]
    fn design_to_quins_emits_root_and_relations() {
        let doc = sample_switch();
        let quins = design_to_quins(&doc).unwrap();
        assert!(quins.len() >= 4);
    }

    #[test]
    fn rejects_unknown_relation_endpoint() {
        let mut doc = sample_switch();
        doc.relations.push(DesignRelation {
            from: "ghost".into(),
            to: "base".into(),
            relation_type: "matesWith".into(),
            label: String::new(),
        });
        assert_eq!(
            design_to_tensors(&doc),
            Err(DesignEncodeError::UnknownPartId)
        );
    }
}