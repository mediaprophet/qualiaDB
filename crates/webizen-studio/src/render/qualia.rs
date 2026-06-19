//! QualiaDB-driven scenes — the binding that makes this a *semantic* 3D engine.
//!
//! A [`SemanticScene`] is the data-driven scene description QualiaDB produces from
//! a graph query: SPARQL/SPARQL-star results, or N3/`modalities` inference output
//! (e.g. the anatomy use-case — health observations → organ highlights with an
//! intensity score). Each [`SceneItem`] carries its **provenance** so geometry is
//! traceable back to the NQuin that justified it.
//!
//! The split is deliberate:
//! - **Semantics** (what to show, how hot, why) come from QualiaDB.
//! - **Geometry** (where each item sits, what mesh) comes from a `layout` the
//!   consumer supplies (e.g. an anatomical atlas), so the same semantic scene can
//!   drive a body map, a knowledge graph, or a physics field.
//!
//! On native, the heavy math/physics behind a scene belongs to QualiaDB
//! (`geometric_algebra` SIMD, `domains::physical`, `quantum_dft`, `ode_solver`,
//! diffusion `shaders/`); this module is the UI-side binding, not a math engine.

use super::graph::{Node, Scene, Style};
use super::mesh::Mesh;
use super::scene::{Camera, Vec3};
use serde::{Deserialize, Serialize};

/// Visual state of a semantic item. Matches the anatomy representation contract
/// (`"highlighted"`, `"active"`, …) so a QualiaDB payload deserializes directly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemState {
    #[default]
    Default,
    Active,
    Highlighted,
    Alert,
}

/// One element of a semantic scene, produced by a QualiaDB query/inference.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneItem {
    /// Stable id used by the layout to look up geometry (e.g. `"heart"`).
    pub id: String,
    #[serde(default)]
    pub state: ItemState,
    /// Inference confidence / heat, 0.0–1.0.
    #[serde(default)]
    pub intensity: f64,
    /// `q42:` provenance hash linking back to the justifying NQuin(s).
    #[serde(default)]
    pub provenance: Option<String>,
    /// Human-readable reasons ("Why is this highlighted").
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// A semantic, backend-neutral scene description from QualiaDB. Deserializes from
/// the representation-contract payload returned by a Tauri command / SPARQL query.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticScene {
    #[serde(default)]
    pub items: Vec<SceneItem>,
    #[serde(default)]
    pub explanations: Vec<String>,
}

/// Anything that yields a [`SemanticScene`] from the graph — a Tauri command
/// result wrapper, a SPARQL binding, or the offline stub. Lets UI code stay
/// agnostic about whether the engine answered or a preview did.
pub trait SceneSource {
    fn semantic_scene(&self) -> SemanticScene;
}

impl SceneSource for SemanticScene {
    fn semantic_scene(&self) -> SemanticScene {
        self.clone()
    }
}

/// Map a semantic state + intensity to a themeable CSS color. Higher intensity
/// pushes toward the warm accent; lower stays cool/muted. Deterministic.
pub fn item_color(state: ItemState, intensity: f64) -> String {
    let t = intensity.clamp(0.0, 1.0);
    // Cool (muted blue-grey) → warm (alert orange-red) interpolation.
    let (r0, g0, b0) = (96.0, 132.0, 168.0);
    let (r1, g1, b1) = match state {
        ItemState::Alert => (239.0, 68.0, 68.0),
        ItemState::Highlighted => (245.0, 158.0, 11.0),
        ItemState::Active => (34.0, 197.0, 94.0),
        ItemState::Default => (148.0, 163.0, 184.0),
    };
    let lerp = |a: f64, b: f64| (a + (b - a) * t).round() as u8;
    let alpha = 0.35 + 0.6 * t;
    format!(
        "rgba({}, {}, {}, {:.2})",
        lerp(r0, r1),
        lerp(g0, g1),
        lerp(b0, b1),
        alpha
    )
}

/// Build a renderable [`Scene`] from a [`SemanticScene`] plus a `layout` that maps
/// each item id to its world position and mesh. Items the layout doesn't recognise
/// are skipped. Each item becomes a [`Node`] colored by [`item_color`]; its
/// provenance and reasons ride along on the node label for inspection/tooltips.
pub fn build_scene<L>(sem: &SemanticScene, camera: Camera, layout: L) -> Scene
where
    L: Fn(&str) -> Option<(Vec3, Mesh)>,
{
    let mut scene = Scene::new(camera).with_background("var(--qualia-bg)");
    for item in &sem.items {
        let Some((position, mesh)) = layout(&item.id) else {
            continue;
        };
        let color = item_color(item.state, item.intensity);
        let style = match item.state {
            ItemState::Default => Style::wire(color),
            _ => Style {
                stroke: Some(color.clone()),
                fill: Some(color),
                ..Default::default()
            },
        };
        let label = match &item.provenance {
            Some(p) => format!("{} [{}]", item.id, p),
            None => item.id.clone(),
        };
        scene = scene.add(
            Node::new(label)
                .at(position)
                .with_mesh(mesh)
                .with_style(style),
        );
    }
    scene
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_color_intensity_monotonic_alpha() {
        let lo = item_color(ItemState::Highlighted, 0.0);
        let hi = item_color(ItemState::Highlighted, 1.0);
        assert_ne!(lo, hi);
        assert!(hi.contains("0.95")); // alpha ramps with intensity
    }

    #[test]
    fn build_scene_uses_layout_and_skips_unknown() {
        let sem = SemanticScene {
            items: vec![
                SceneItem {
                    id: "heart".into(),
                    state: ItemState::Highlighted,
                    intensity: 0.8,
                    provenance: Some("q42:abc".into()),
                    reasons: vec!["elevated BP".into()],
                },
                SceneItem {
                    id: "unknown".into(),
                    ..SceneItem {
                        id: String::new(),
                        state: ItemState::Default,
                        intensity: 0.0,
                        provenance: None,
                        reasons: vec![],
                    }
                },
            ],
            explanations: vec![],
        };
        let scene = build_scene(&sem, Camera::default(), |id| {
            (id == "heart").then(|| (Vec3::new(0.0, 0.0, 0.0), Mesh::uv_sphere(1.0, 6, 8)))
        });
        assert_eq!(scene.roots.len(), 1, "unknown id skipped");
        assert!(scene.roots[0].label.contains("q42:abc"));
    }

    #[test]
    fn semantic_scene_deserializes_from_contract_json() {
        let json = r#"{"items":[{"id":"heart","state":"highlighted","intensity":0.82,
            "provenance":"q42:deadbeef","reasons":["elevated BP"]}],
            "explanations":["cardiovascular load"]}"#;
        let s: SemanticScene = serde_json::from_str(json).unwrap();
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].state, ItemState::Highlighted);
        assert_eq!(s.explanations[0], "cardiovascular load");
    }
}
