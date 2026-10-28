//! Fungal mycelial network, nutrient transport, and sporocarp phenology.
//!
//! Models hyphal nodes with moisture and nutrient pools (C, P, N), inter-node
//! transport along conductive edges, sporocarp developmental stage transitions,
//! and zero-heap NQuin state emission.

use crate::foundation::frame_layout::pack_float_object;
use crate::query::resolver::INLINE_TAG_INTEGER;
use crate::{q_hash, NQuin};

/// Phenological stages of sporocarp (fruiting body) development.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SporocarpStage {
    MycelialKnot = 0,
    PrimordiumPinhead = 1,
    ButtonVeilIntact = 2,
    MaturePileusExpanded = 3,
    SporeRelease = 4,
    DeliquescentSenescent = 5,
}

/// A node in the mycelial network within substrate or soil matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct HyphalNode {
    pub node_id: u64,
    pub substrate_moisture: f32,
    pub carbon_mol_m3: f32,
    pub phosphorus_mol_m3: f32,
    pub nitrogen_mol_m3: f32,
}

/// Conductive edge connecting two hyphal nodes.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct HyphalEdge {
    pub from_idx: u16,
    pub to_idx: u16,
    pub conductance: f32,
    pub anastomosis_fused: bool,
}

/// Morphological and physiological state of a developing sporocarp.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct SporocarpState {
    pub stage: SporocarpStage,
    pub pileus_diameter_mm: f32,
    pub stipe_height_mm: f32,
    pub biomass_carbon_mol: f32,
    pub age_hours: f32,
}

/// Errors occurring during fungal network evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum FungaError {
    InvalidMoisture(f32),
    NegativeNutrientFlux(f32),
    NegativeConductance(f32),
    NegativeNutrient(f32),
    NegativeTimeStep(f32),
    InvalidNodeIndex { index: u16, len: usize },
    BufferOverflow { needed: usize, available: usize },
}

impl std::fmt::Display for FungaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMoisture(m) => write!(f, "Invalid moisture {m}; must be in [0.0, 1.0]"),
            Self::NegativeNutrientFlux(flux) => write!(f, "Negative nutrient flux: {flux}"),
            Self::NegativeConductance(c) => write!(f, "Negative conductance: {c}"),
            Self::NegativeNutrient(n) => write!(f, "Negative nutrient pool: {n}"),
            Self::NegativeTimeStep(dt) => write!(f, "Negative time step: {dt}"),
            Self::InvalidNodeIndex { index, len } => write!(f, "Node index {index} >= {len}"),
            Self::BufferOverflow { needed, available } => write!(f, "Overflow: needed {needed}, got {available}"),
        }
    }
}

impl std::error::Error for FungaError {}

pub const PRED_HYPHAL_MOISTURE: u64 = q_hash("qualia:biosphere#hyphalNodeMoisture");
pub const PRED_HYPHAL_CARBON: u64 = q_hash("qualia:biosphere#hyphalNodeCarbon");
pub const PRED_HYPHAL_PHOSPHORUS: u64 = q_hash("qualia:biosphere#hyphalNodePhosphorus");
pub const PRED_HYPHAL_NITROGEN: u64 = q_hash("qualia:biosphere#hyphalNodeNitrogen");
pub const PRED_SPOROCARP_STAGE: u64 = q_hash("qualia:biosphere#sporocarpStage");
pub const PRED_SPOROCARP_PILEUS_DIAMETER: u64 = q_hash("qualia:biosphere#sporocarpPileusDiameter");
pub const PRED_SPOROCARP_BIOMASS: u64 = q_hash("qualia:biosphere#sporocarpBiomass");
pub const METADATA_PROVENANCE_SIMULATION: u64 = 1u64 << 28;

/// Zero-heap step executing mycelial nutrient transport across network edges.
pub fn step_mycelial_nutrient_transport_zero_heap(
    nodes: &mut [HyphalNode],
    edges: &[HyphalEdge],
    dt_sec: f32,
) -> Result<(), FungaError> {
    if dt_sec < 0.0 || !dt_sec.is_finite() {
        return Err(FungaError::NegativeTimeStep(dt_sec));
    }
    for n in nodes.iter() {
        if n.substrate_moisture < 0.0 || n.substrate_moisture > 1.0 || !n.substrate_moisture.is_finite() {
            return Err(FungaError::InvalidMoisture(n.substrate_moisture));
        }
        if n.carbon_mol_m3 < 0.0 || !n.carbon_mol_m3.is_finite()
            || n.phosphorus_mol_m3 < 0.0 || !n.phosphorus_mol_m3.is_finite()
            || n.nitrogen_mol_m3 < 0.0 || !n.nitrogen_mol_m3.is_finite()
        {
            return Err(FungaError::NegativeNutrient(n.carbon_mol_m3));
        }
    }
    for e in edges.iter() {
        if e.conductance < 0.0 || !e.conductance.is_finite() {
            return Err(FungaError::NegativeConductance(e.conductance));
        }
        if e.from_idx as usize >= nodes.len() {
            return Err(FungaError::InvalidNodeIndex { index: e.from_idx, len: nodes.len() });
        }
        if e.to_idx as usize >= nodes.len() {
            return Err(FungaError::InvalidNodeIndex { index: e.to_idx, len: nodes.len() });
        }
    }

    for e in edges.iter() {
        let (u, v) = (e.from_idx as usize, e.to_idx as usize);
        if u == v { continue; }
        let cond = e.conductance * if e.anastomosis_fused { 1.5 } else { 1.0 };
        if u < v {
            let (left, right) = nodes.split_at_mut(v);
            transfer_nutrients(&mut left[u], &mut right[0], cond, dt_sec);
        } else {
            let (left, right) = nodes.split_at_mut(u);
            transfer_nutrients(&mut right[0], &mut left[v], cond, dt_sec);
        }
    }
    Ok(())
}

#[inline(always)]
fn transfer_pool(a: &mut f32, b: &mut f32, cond: f32, dt: f32) {
    let diff = *a - *b;
    let rate = diff * cond * dt;
    let flux = if rate >= 0.0 { rate.min(*a * 0.5) } else { rate.max(-*b * 0.5) };
    *a -= flux;
    *b += flux;
}

fn transfer_nutrients(u: &mut HyphalNode, v: &mut HyphalNode, cond: f32, dt: f32) {
    transfer_pool(&mut u.carbon_mol_m3, &mut v.carbon_mol_m3, cond, dt);
    transfer_pool(&mut u.phosphorus_mol_m3, &mut v.phosphorus_mol_m3, cond, dt);
    transfer_pool(&mut u.nitrogen_mol_m3, &mut v.nitrogen_mol_m3, cond, dt);
}

/// Evaluates phenological transitions of a sporocarp based on environmental factors.
pub fn evaluate_sporocarp_transition_zero_heap(
    state: &mut SporocarpState,
    soil_moisture: f32,
    temperature_c: f32,
    available_carbon_mol: f32,
    dt_hours: f32,
) -> Result<bool, FungaError> {
    if soil_moisture < 0.0 || soil_moisture > 1.0 || !soil_moisture.is_finite() {
        return Err(FungaError::InvalidMoisture(soil_moisture));
    }
    if available_carbon_mol < 0.0 || !available_carbon_mol.is_finite() {
        return Err(FungaError::NegativeNutrient(available_carbon_mol));
    }
    if dt_hours < 0.0 || !dt_hours.is_finite() {
        return Err(FungaError::NegativeTimeStep(dt_hours));
    }

    state.age_hours += dt_hours;
    match state.stage {
        SporocarpStage::MycelialKnot => {
            if soil_moisture >= 0.40 && (10.0..=28.0).contains(&temperature_c) && available_carbon_mol >= 0.05 {
                state.stage = SporocarpStage::PrimordiumPinhead;
                state.stipe_height_mm = 2.0;
                state.pileus_diameter_mm = 1.5;
                state.biomass_carbon_mol += 0.02;
                return Ok(true);
            }
        }
        SporocarpStage::PrimordiumPinhead => {
            state.stipe_height_mm += 0.5 * dt_hours;
            state.pileus_diameter_mm += 0.4 * dt_hours;
            if state.age_hours >= 12.0 && soil_moisture >= 0.45 && available_carbon_mol >= 0.10 {
                state.stage = SporocarpStage::ButtonVeilIntact;
                state.biomass_carbon_mol += 0.05;
                return Ok(true);
            }
        }
        SporocarpStage::ButtonVeilIntact => {
            state.stipe_height_mm += 1.0 * dt_hours;
            state.pileus_diameter_mm += 1.2 * dt_hours;
            if state.age_hours >= 36.0 && soil_moisture >= 0.35 && available_carbon_mol >= 0.15 {
                state.stage = SporocarpStage::MaturePileusExpanded;
                state.biomass_carbon_mol += 0.10;
                return Ok(true);
            }
        }
        SporocarpStage::MaturePileusExpanded => {
            if state.age_hours >= 60.0 && soil_moisture >= 0.25 {
                state.stage = SporocarpStage::SporeRelease;
                return Ok(true);
            }
        }
        SporocarpStage::SporeRelease => {
            if state.age_hours >= 96.0 {
                state.stage = SporocarpStage::DeliquescentSenescent;
                return Ok(true);
            }
        }
        SporocarpStage::DeliquescentSenescent => {}
    }
    Ok(false)
}

/// Emits parity-valid NQuins representing mycelial and sporocarp states into a caller buffer.
pub fn emit_funga_state_quins_zero_heap(
    network_did_hash: u64,
    nodes: &[HyphalNode],
    sporocarp: Option<&SporocarpState>,
    is_simulation: bool,
    out_quins: &mut [NQuin],
) -> Result<usize, FungaError> {
    let needed = nodes.len() * 4 + if sporocarp.is_some() { 3 } else { 0 };
    if out_quins.len() < needed {
        return Err(FungaError::BufferOverflow { needed, available: out_quins.len() });
    }

    let prov = if is_simulation { METADATA_PROVENANCE_SIMULATION } else { 0 };
    let metadata = (0b01u64 << 61) | prov;
    let mut idx = 0;

    for n in nodes.iter() {
        let subj = network_did_hash ^ n.node_id;
        let props: [(u64, u64); 4] = [
            (PRED_HYPHAL_MOISTURE, pack_float_object(n.substrate_moisture)),
            (PRED_HYPHAL_CARBON, pack_float_object(n.carbon_mol_m3)),
            (PRED_HYPHAL_PHOSPHORUS, pack_float_object(n.phosphorus_mol_m3)),
            (PRED_HYPHAL_NITROGEN, pack_float_object(n.nitrogen_mol_m3)),
        ];
        for (pred, obj) in props {
            let parity = NQuin::calculate_parity(subj, pred, obj, network_did_hash, metadata);
            out_quins[idx] = NQuin { subject: subj, predicate: pred, object: obj, context: network_did_hash, metadata, parity };
            idx += 1;
        }
    }

    if let Some(sp) = sporocarp {
        let subj = network_did_hash;
        let props: [(u64, u64); 3] = [
            (PRED_SPOROCARP_STAGE, INLINE_TAG_INTEGER | (sp.stage as u64)),
            (PRED_SPOROCARP_PILEUS_DIAMETER, pack_float_object(sp.pileus_diameter_mm)),
            (PRED_SPOROCARP_BIOMASS, pack_float_object(sp.biomass_carbon_mol)),
        ];
        for (pred, obj) in props {
            let parity = NQuin::calculate_parity(subj, pred, obj, network_did_hash, metadata);
            out_quins[idx] = NQuin { subject: subj, predicate: pred, object: obj, context: network_did_hash, metadata, parity };
            idx += 1;
        }
    }

    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn test_nutrient_transport_conservation() {
        let mut nodes = [
            HyphalNode { node_id: 1, substrate_moisture: 0.6, carbon_mol_m3: 10.0, phosphorus_mol_m3: 2.0, nitrogen_mol_m3: 5.0 },
            HyphalNode { node_id: 2, substrate_moisture: 0.5, carbon_mol_m3: 2.0, phosphorus_mol_m3: 0.5, nitrogen_mol_m3: 1.0 },
        ];
        let edges = [HyphalEdge { from_idx: 0, to_idx: 1, conductance: 0.1, anastomosis_fused: false }];

        let (c0, p0, n0) = (nodes[0].carbon_mol_m3 + nodes[1].carbon_mol_m3, nodes[0].phosphorus_mol_m3 + nodes[1].phosphorus_mol_m3, nodes[0].nitrogen_mol_m3 + nodes[1].nitrogen_mol_m3);
        step_mycelial_nutrient_transport_zero_heap(&mut nodes, &edges, 1.0).unwrap();
        assert!(nodes[0].carbon_mol_m3 < 10.0);
        assert!(nodes[1].carbon_mol_m3 > 2.0);
        let (c1, p1, n1) = (nodes[0].carbon_mol_m3 + nodes[1].carbon_mol_m3, nodes[0].phosphorus_mol_m3 + nodes[1].phosphorus_mol_m3, nodes[0].nitrogen_mol_m3 + nodes[1].nitrogen_mol_m3);
        assert!((c0 - c1).abs() < 1e-6);
        assert!((p0 - p1).abs() < 1e-6);
        assert!((n0 - n1).abs() < 1e-6);
    }

    #[test]
    fn test_anastomosis_enhanced_conductance() {
        let mut unfused = [
            HyphalNode { node_id: 1, substrate_moisture: 0.5, carbon_mol_m3: 10.0, phosphorus_mol_m3: 2.0, nitrogen_mol_m3: 2.0 },
            HyphalNode { node_id: 2, substrate_moisture: 0.5, carbon_mol_m3: 0.0, phosphorus_mol_m3: 0.0, nitrogen_mol_m3: 0.0 },
        ];
        let mut fused = unfused;
        let e_unfused = [HyphalEdge { from_idx: 0, to_idx: 1, conductance: 0.05, anastomosis_fused: false }];
        let e_fused = [HyphalEdge { from_idx: 0, to_idx: 1, conductance: 0.05, anastomosis_fused: true }];

        step_mycelial_nutrient_transport_zero_heap(&mut unfused, &e_unfused, 1.0).unwrap();
        step_mycelial_nutrient_transport_zero_heap(&mut fused, &e_fused, 1.0).unwrap();
        assert!(fused[1].carbon_mol_m3 > unfused[1].carbon_mol_m3);
    }

    #[test]
    fn test_negative_conductance_or_invalid_moisture_rejected() {
        let mut nodes = [HyphalNode { node_id: 1, substrate_moisture: 1.2, carbon_mol_m3: 1.0, phosphorus_mol_m3: 1.0, nitrogen_mol_m3: 1.0 }];
        let edges = [HyphalEdge { from_idx: 0, to_idx: 0, conductance: -0.1, anastomosis_fused: false }];
        assert!(step_mycelial_nutrient_transport_zero_heap(&mut nodes, &edges, 1.0).is_err());
    }

    #[test]
    fn test_sporocarp_phenology_progression() {
        let mut sp = SporocarpState { stage: SporocarpStage::MycelialKnot, pileus_diameter_mm: 0.0, stipe_height_mm: 0.0, biomass_carbon_mol: 0.01, age_hours: 0.0 };
        assert!(evaluate_sporocarp_transition_zero_heap(&mut sp, 0.55, 20.0, 0.1, 1.0).unwrap());
        assert_eq!(sp.stage, SporocarpStage::PrimordiumPinhead);
        evaluate_sporocarp_transition_zero_heap(&mut sp, 0.55, 20.0, 0.2, 12.0).unwrap();
        assert_eq!(sp.stage, SporocarpStage::ButtonVeilIntact);
    }

    #[test]
    fn test_emit_funga_state_quins_and_zero_heap() {
        let nodes = [HyphalNode { node_id: 42, substrate_moisture: 0.65, carbon_mol_m3: 8.5, phosphorus_mol_m3: 1.2, nitrogen_mol_m3: 3.4 }];
        let sp = SporocarpState { stage: SporocarpStage::MaturePileusExpanded, pileus_diameter_mm: 45.0, stipe_height_mm: 80.0, biomass_carbon_mol: 0.35, age_hours: 50.0 };
        let mut out = [NQuin::default(); 7];
        assert_zero_alloc("emit_funga_state_quins", || {
            let count = emit_funga_state_quins_zero_heap(1001, &nodes, Some(&sp), true, &mut out).unwrap();
            assert_eq!(count, 7);
        });
        for q in out.iter() {
            assert!(q.verify_ecc_parity());
            assert_eq!((q.metadata >> 28) & 1, 1);
        }
        let mut small_out = [NQuin::default(); 2];
        let err = emit_funga_state_quins_zero_heap(1001, &nodes, Some(&sp), true, &mut small_out);
        assert!(matches!(err, Err(FungaError::BufferOverflow { .. })));
    }
}
