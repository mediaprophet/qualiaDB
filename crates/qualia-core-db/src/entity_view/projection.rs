//! Flat + scene projection IR for mindware HID (cold-path friendly serde types).

use super::entity_id::{EntityId, EntityKind};
use super::observer::{AffordanceBits, RepresentationWing};
use serde::{Deserialize, Serialize};

/// Presentation morphology level P0-P6 (see presentation-morphology plan).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum PresentationLevel {
    #[default]
    Document = 0,
    AppHabitat = 1,
    SpatialDesk = 2,
    StageWorld = 3,
    EmbodiedWorld = 4,
    Infosphere = 5,
    MultiSensory = 6,
}

impl PresentationLevel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::AppHabitat,
            2 => Self::SpatialDesk,
            3 => Self::StageWorld,
            4 => Self::EmbodiedWorld,
            5 => Self::Infosphere,
            6 => Self::MultiSensory,
            _ => Self::Document,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Card descriptor for flat HID (Dioxus / browser chrome lists).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatCard {
    pub entity_id: u64,
    pub kind: EntityKind,
    pub title: String,
    pub excerpt: String,
    pub wing: RepresentationWing,
    pub affordance_bits: u8,
    pub honesty: String,
    pub uri: String,
}

/// Scene node projection (maps to webizen-render SceneNode fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNodeProj {
    pub entity_id: u64,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub color: String,
    pub radius: f64,
    pub alpha: f64,
    pub affordance_bits: u8,
}

/// Combined projection result for one observer session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectionResult {
    pub observer: String,
    pub presentation_level: u8,
    pub flat: Vec<FlatCard>,
    pub scene_nodes: Vec<SceneNodeProj>,
    pub hidden_count: u32,
}

/// Layout entity with optional geo for pin placement.
#[derive(Debug, Clone, Copy)]
pub struct LayoutInput {
    pub entity_id: EntityId,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub affordances: AffordanceBits,
    pub wing: RepresentationWing,
}

/// Wing → presentation colour (cinema-readable, not neon random).
pub fn wing_color(wing: RepresentationWing) -> &'static str {
    match wing {
        RepresentationWing::Private => "#a78bfa",  // violet
        RepresentationWing::Offered => "#34d399",  // emerald
        RepresentationWing::Commons => "#38bdf8",  // sky
    }
}

/// Place nodes: geo pins when lat/lon present; otherwise golden-angle manifold field.
/// Writes up to `out.len()` nodes; returns count.
pub fn layout_scene_nodes(inputs: &[LayoutInput], out: &mut [SceneNodeProj]) -> usize {
    let mut n = 0;
    let mut field_i = 0usize;
    let field_total = inputs
        .iter()
        .filter(|i| i.lat.is_none() || i.lon.is_none())
        .count()
        .max(1);
    for inp in inputs {
        if n >= out.len() {
            break;
        }
        let (x, y, z) = if let (Some(lat), Some(lon)) = (inp.lat, inp.lon) {
            // Equirectangular sketch in 0..1 for 2D map morph.
            let xn = ((lon as f64) + 180.0) / 360.0;
            let yn = 1.0 - ((lat as f64) + 90.0) / 180.0;
            (xn.clamp(0.05, 0.95), yn.clamp(0.05, 0.95), 0.15)
        } else {
            // Golden-angle disk → slight depth for spatial field (prestige morph).
            let i = field_i as f64;
            let tot = field_total as f64;
            field_i += 1;
            let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
            let r = ((i + 0.5) / tot).sqrt() * 0.38;
            let theta = i * golden;
            let xn = 0.5 + r * theta.cos();
            let yn = 0.5 + r * theta.sin() * 0.72;
            let zn = 0.2 + (i / tot) * 0.55;
            (xn.clamp(0.08, 0.92), yn.clamp(0.08, 0.92), zn.clamp(0.05, 0.95))
        };
        let bits = inp.affordances.pack();
        let radius = 5.5
            + if inp.affordances.can_edit { 2.0 } else { 0.0 }
            + if inp.affordances.can_share { 1.0 } else { 0.0 };
        out[n] = SceneNodeProj {
            entity_id: inp.entity_id.raw(),
            id: format!("{:016x}", inp.entity_id.raw()),
            x,
            y,
            z,
            color: wing_color(inp.wing).into(),
            radius,
            alpha: (0.5 + z * 0.5).clamp(0.45, 1.0),
            affordance_bits: bits,
        };
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_view::entity_id::EntityId;
    use crate::entity_view::observer::AffordanceBits;

    #[test]
    fn layout_uses_geo_when_present() {
        let inputs = [LayoutInput {
            entity_id: EntityId::from_uri("urn:pin"),
            lat: Some(0.0),
            lon: Some(0.0),
            affordances: AffordanceBits::FULL,
            wing: RepresentationWing::Commons,
        }];
        let mut out: Vec<SceneNodeProj> = (0..2)
            .map(|_| SceneNodeProj {
                entity_id: 0,
                id: String::new(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                color: String::new(),
                radius: 0.0,
                alpha: 0.0,
                affordance_bits: 0,
            })
            .collect();
        let n = layout_scene_nodes(&inputs, &mut out);
        assert_eq!(n, 1);
        assert!((out[0].x - 0.5).abs() < 0.01);
    }
}
