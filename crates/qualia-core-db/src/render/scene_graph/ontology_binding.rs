//! `.10d` `EONT1` Living Ontology & Affordance Scene-Graph Binding (`ontology_binding.rs`).
//!
//! Bridges [`EntityOntologyMiniHeader`], [`AffordanceBinaryRecord`], and
//! [`DevelopmentalKeyframeRecord`] from `container_10d::entity_ontology_section` and
//! [`evaluate_allometric_dimension`] from `container_10d::procedural_synthesis` directly into
//! [`SceneNodeRecord`].
//!
//! All evaluation functions are strictly `HotZeroHeap` (zero heap allocations).

use crate::container_10d::entity_ontology_section::{
    AffordanceBinaryRecord, DevelopmentalKeyframeRecord, EntityOntologyMiniHeader,
};
use crate::container_10d::procedural_synthesis::evaluate_allometric_dimension;
use crate::entity_view::entity_ontologies::LifecycleStage;
use crate::render::scene_graph::pool::SceneNodeRecord;

/// Ontogenetic developmental lifecycle stage of a living `.10d` entity.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum DevelopmentalStage {
    /// Dormant seed, spore, or egg.
    SeedOrSpore = 0,
    /// Growing seedling, sapling, or juvenile organism.
    Juvenile = 1,
    /// Morphologically mature adult organism.
    #[default]
    Adult = 2,
    /// Post-reproductive senescent stage.
    Senescent = 3,
    /// Deceased / decomposing snags or remains.
    Deceased = 4,
}

impl DevelopmentalStage {
    /// Converts a raw `u8` stage code into a [`DevelopmentalStage`].
    #[inline]
    pub const fn from_u8(raw: u8) -> Self {
        match raw {
            0 => Self::SeedOrSpore,
            1 => Self::Juvenile,
            2 => Self::Adult,
            3 => Self::Senescent,
            4 => Self::Deceased,
            _ => Self::Adult,
        }
    }

    /// Converts from the general [`LifecycleStage`] enum.
    #[inline]
    pub const fn from_lifecycle_stage(stage: LifecycleStage) -> Self {
        match stage {
            LifecycleStage::SeedOrSpore => Self::SeedOrSpore,
            LifecycleStage::Juvenile => Self::Juvenile,
            LifecycleStage::Adult | LifecycleStage::Manufactured | LifecycleStage::Operational => {
                Self::Adult
            }
            LifecycleStage::Senescent | LifecycleStage::Degraded => Self::Senescent,
            LifecycleStage::Deceased | LifecycleStage::Decommissioned => Self::Deceased,
        }
    }
}

/// Live runtime metabolic and ontogenetic state bound to a scene node via `EONT1`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LivingOntologyState {
    /// Current developmental stage.
    pub stage: DevelopmentalStage,
    /// Normalized ontogenetic growth progress in `[0.0, 1.0]`.
    pub growth_progress_01: f32,
    /// Current age in days since germination/birth.
    pub age_days: f32,
    /// Internal energy / biomass reserve in permill (`0..=1000`).
    pub energy_permill: u16,
    /// Physical or structural damage in permill (`0..=1000`).
    pub damage_permill: u16,
    /// Cellular hydration / turgor pressure in permill (`0..=1000`).
    pub hydration_permill: u16,
    /// Accumulated mechanical wear in permill (`0..=1000`).
    pub wear_permill: u16,
    /// Active behavioral state identifier.
    pub active_state_id: u32,
    /// Huxley allometric base constant $b$ ($y = b \cdot x^k$).
    pub allometry_base_b: f32,
    /// Huxley allometric power exponent $k$ ($y = b \cdot x^k$).
    pub allometry_exponent_k: f32,
}

impl Default for LivingOntologyState {
    #[inline]
    fn default() -> Self {
        Self {
            stage: DevelopmentalStage::Adult,
            growth_progress_01: 1.0,
            age_days: 365.0,
            energy_permill: 1000,
            damage_permill: 0,
            hydration_permill: 1000,
            wear_permill: 0,
            active_state_id: 0,
            allometry_base_b: 1.0,
            allometry_exponent_k: 1.0,
        }
    }
}

impl LivingOntologyState {
    /// Initializes a live state directly from a decoded `.10d` [`EntityOntologyMiniHeader`].
    #[inline]
    pub const fn from_mini_header(header: &EntityOntologyMiniHeader) -> Self {
        let stage = DevelopmentalStage::from_u8(header.stage);
        let progress = match stage {
            DevelopmentalStage::SeedOrSpore => 0.10,
            DevelopmentalStage::Juvenile => 0.45,
            DevelopmentalStage::Adult => 1.0,
            DevelopmentalStage::Senescent => 0.95,
            DevelopmentalStage::Deceased => 0.85,
        };
        Self {
            stage,
            growth_progress_01: progress,
            age_days: progress * 365.0,
            energy_permill: header.energy_permill,
            damage_permill: header.damage_permill,
            hydration_permill: header.hydration_permill,
            wear_permill: header.wear_permill,
            active_state_id: header.stage as u32,
            allometry_base_b: 1.0,
            allometry_exponent_k: 1.0,
        }
    }
}

/// Interpolated ontogenetic keyframe scales `(crown_scale, trunk_scale, lai_multiplier, mesh_asset_hash)`.
#[inline]
pub fn interpolate_ontogenetic_scales(
    keyframes: &[DevelopmentalKeyframeRecord],
    growth_progress_01: f32,
) -> (f32, f32, f32, u64) {
    if keyframes.is_empty() {
        let p = growth_progress_01.clamp(0.05, 1.5);
        return (p, p, p, 0);
    }
    if keyframes.len() == 1 {
        let k = &keyframes[0];
        return (
            k.allometric_crown_scale,
            k.allometric_trunk_scale,
            k.lai_multiplier,
            k.mesh_asset_hash,
        );
    }

    let p = growth_progress_01.clamp(0.0, 1.0);
    let first_age = keyframes[0].age_days;
    let last_age = keyframes[keyframes.len() - 1].age_days.max(first_age + 1e-4);
    let target_age = first_age + p * (last_age - first_age);

    let mut idx = 0usize;
    while idx + 1 < keyframes.len() && keyframes[idx + 1].age_days < target_age {
        idx += 1;
    }

    if idx + 1 >= keyframes.len() {
        let k = &keyframes[keyframes.len() - 1];
        return (
            k.allometric_crown_scale,
            k.allometric_trunk_scale,
            k.lai_multiplier,
            k.mesh_asset_hash,
        );
    }

    let k0 = &keyframes[idx];
    let k1 = &keyframes[idx + 1];
    let span = (k1.age_days - k0.age_days).max(1e-5);
    let t = ((target_age - k0.age_days) / span).clamp(0.0, 1.0);

    let crown = k0.allometric_crown_scale + t * (k1.allometric_crown_scale - k0.allometric_crown_scale);
    let trunk = k0.allometric_trunk_scale + t * (k1.allometric_trunk_scale - k0.allometric_trunk_scale);
    let lai = k0.lai_multiplier + t * (k1.lai_multiplier - k0.lai_multiplier);
    let mesh_hash = if t < 0.5 {
        k0.mesh_asset_hash
    } else {
        k1.mesh_asset_hash
    };

    (crown, trunk, lai, mesh_hash)
}

/// Applies `.10d` `EONT1` ontogenetic keyframe interpolation and Huxley power-law allometry
/// ($y = b \cdot x^k$) directly to `node.local_scale`, stamping `node.changed_tick`.
#[inline]
pub fn apply_ontogenetic_allometry(
    node: &mut SceneNodeRecord,
    state: &LivingOntologyState,
    _header: &EntityOntologyMiniHeader,
    keyframes: &[DevelopmentalKeyframeRecord],
    current_tick: u32,
) {
    let (crown_kf, trunk_kf, _lai_kf, _mesh_hash) =
        interpolate_ontogenetic_scales(keyframes, state.growth_progress_01);

    let huxley_factor = evaluate_allometric_dimension(
        state.growth_progress_01.max(0.05),
        state.allometry_base_b.max(0.05),
        state.allometry_exponent_k.max(0.1),
    );

    // Turgor/hydration factor: severe dehydration (< 300‰) causes slight crown droop.
    let hydration_factor = if state.hydration_permill < 300 {
        0.85 + 0.15 * (state.hydration_permill as f32 / 300.0)
    } else {
        1.0
    };

    let radial_scale = (trunk_kf * huxley_factor).max(0.01);
    let vertical_scale = (crown_kf * huxley_factor * hydration_factor).max(0.01);

    node.local_scale = [radial_scale, vertical_scale, radial_scale];
    node.changed_tick = current_tick;
}

/// Outcome returned when a deontic- and capability-gated `.10d` affordance is dispatched.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AffordanceDispatchOutcome {
    /// Action identifier hash that was triggered.
    pub action_id: u64,
    /// Topic hash emitted to the semantic journal.
    pub emitted_topic_hash: u64,
    /// Skeletal/procedural animation clip ID triggered (`0` = none).
    pub animation_clip_id: u32,
    /// New behavioral state ID transitioned into.
    pub target_state_id: u32,
    /// Physical impulse magnitude generated by the affordance.
    pub impulse_magnitude: f32,
    /// Updated energy permill (`0..=1000`).
    pub energy_permill: u16,
    /// Updated damage permill (`0..=1000`).
    pub damage_permill: u16,
    /// Updated hydration permill (`0..=1000`).
    pub hydration_permill: u16,
    /// Updated wear permill (`0..=1000`).
    pub wear_permill: u16,
}

/// Evaluates and dispatches a matching [`AffordanceBinaryRecord`] against `state`.
///
/// Enforces:
/// 1. `deontic_permitted == true`,
/// 2. `actor_capability_mask` satisfies `required_capability_hash` (when non-zero),
/// 3. Metabolic preconditions (`min_energy_permill`, `max_damage_permill`, `min_hydration_permill`, `max_wear_permill`).
#[inline]
pub fn dispatch_affordance(
    state: &mut LivingOntologyState,
    affordances: &[AffordanceBinaryRecord],
    trigger_predicate_hash: u64,
    actor_capability_mask: u32,
    deontic_permitted: bool,
) -> Option<AffordanceDispatchOutcome> {
    if !deontic_permitted {
        return None;
    }

    let mut i = 0usize;
    while i < affordances.len() {
        let rec = &affordances[i];
        if rec.action_id == trigger_predicate_hash {
            // 1. Check capability mask against required capability hash
            if rec.required_capability_hash != 0 {
                let req_mask = (rec.required_capability_hash & 0xFFFF_FFFF) as u32;
                if (actor_capability_mask & req_mask) != req_mask {
                    return None;
                }
            }

            // 2. Check metabolic & structural preconditions
            if state.energy_permill < rec.min_energy_permill
                || state.damage_permill > rec.max_damage_permill
                || state.hydration_permill < rec.min_hydration_permill
                || state.wear_permill > rec.max_wear_permill
            {
                return None;
            }

            // 3. Apply metabolic state transitions
            state.energy_permill = ((state.energy_permill as i32) + (rec.energy_delta as i32))
                .clamp(0, 1000) as u16;
            state.damage_permill = ((state.damage_permill as i32) + (rec.damage_delta as i32))
                .clamp(0, 1000) as u16;
            state.hydration_permill = ((state.hydration_permill as i32)
                + (rec.hydration_delta as i32))
                .clamp(0, 1000) as u16;
            state.wear_permill =
                ((state.wear_permill as i32) + (rec.wear_delta as i32)).clamp(0, 1000) as u16;

            if state.damage_permill >= 1000 {
                state.stage = DevelopmentalStage::Deceased;
            } else if state.damage_permill >= 750 {
                state.stage = DevelopmentalStage::Senescent;
            }

            let target_state_id = if rec.reserved_u16 != 0 {
                rec.reserved_u16 as u32
            } else {
                (rec.category as u32) + 1
            };
            state.active_state_id = target_state_id;

            let impulse_magnitude = ((rec.damage_delta.unsigned_abs() as f32) * 0.5
                + (rec.energy_delta.unsigned_abs() as f32) * 0.1)
                .max(1.0);

            return Some(AffordanceDispatchOutcome {
                action_id: rec.action_id,
                emitted_topic_hash: rec.emitted_topic_hash,
                animation_clip_id: rec.animation_clip_id,
                target_state_id,
                impulse_magnitude,
                energy_permill: state.energy_permill,
                damage_permill: state.damage_permill,
                hydration_permill: state.hydration_permill,
                wear_permill: state.wear_permill,
            });
        }
        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::entity_ontology_section::MAGIC_EONT;
    use crate::render::scene_graph::pool::NodeKind;

    #[test]
    fn ontogenetic_allometry_scales_node_and_stamps_tick() {
        let header = EntityOntologyMiniHeader {
            magic: MAGIC_EONT,
            version: 1,
            stage: DevelopmentalStage::Juvenile as u8,
            reserved_u8: 0,
            entity_did_hash: 0x1234,
            taxon_iri_hash: 0x5678,
            energy_permill: 900,
            damage_permill: 0,
            hydration_permill: 900,
            wear_permill: 0,
            affordance_count: 0,
            reserved_u32: 0,
        };
        let mut state = LivingOntologyState::from_mini_header(&header);
        state.growth_progress_01 = 0.5;
        state.allometry_base_b = 1.0;
        state.allometry_exponent_k = 1.0;

        let keyframes = [
            DevelopmentalKeyframeRecord {
                age_days: 0.0,
                stage: 0,
                reserved_u8: 0,
                reserved_u16: 0,
                allometric_crown_scale: 0.2,
                allometric_trunk_scale: 0.2,
                lai_multiplier: 0.1,
                reserved_u32: 0,
                mesh_asset_hash: 1,
            },
            DevelopmentalKeyframeRecord {
                age_days: 100.0,
                stage: 2,
                reserved_u8: 0,
                reserved_u16: 0,
                allometric_crown_scale: 1.0,
                allometric_trunk_scale: 0.8,
                lai_multiplier: 1.0,
                reserved_u32: 0,
                mesh_asset_hash: 2,
            },
        ];

        let mut node = SceneNodeRecord::new(0x1234, NodeKind::MeshBRep);
        apply_ontogenetic_allometry(&mut node, &state, &header, &keyframes, 77);

        assert_eq!(node.changed_tick, 77);
        assert!((node.local_scale[0] - 0.25).abs() < 1e-4);
        assert!((node.local_scale[1] - 0.30).abs() < 1e-4);
    }

    #[test]
    fn affordance_dispatch_enforces_deontic_and_capability_gates() {
        let mut state = LivingOntologyState::default();
        let affordances = [AffordanceBinaryRecord {
            action_id: 0xCAFE,
            required_capability_hash: 0b0101,
            emitted_topic_hash: 0xBEEF,
            category: 3,
            min_observer_status: 0,
            min_energy_permill: 200,
            max_damage_permill: 800,
            min_hydration_permill: 100,
            max_wear_permill: 900,
            energy_delta: -150,
            damage_delta: 40,
            hydration_delta: -50,
            wear_delta: 25,
            reserved_u16: 9,
            animation_clip_id: 14,
        }];

        // Denied when deontic_permitted is false.
        assert!(dispatch_affordance(&mut state, &affordances, 0xCAFE, 0b0111, false).is_none());

        // Denied when actor lacks capability bit.
        assert!(dispatch_affordance(&mut state, &affordances, 0xCAFE, 0b0001, true).is_none());

        // Permitted when both deontic and capability gates pass.
        let outcome =
            dispatch_affordance(&mut state, &affordances, 0xCAFE, 0b0111, true).unwrap();
        assert_eq!(outcome.action_id, 0xCAFE);
        assert_eq!(outcome.emitted_topic_hash, 0xBEEF);
        assert_eq!(outcome.animation_clip_id, 14);
        assert_eq!(outcome.target_state_id, 9);
        assert_eq!(state.energy_permill, 850);
        assert_eq!(state.damage_permill, 40);
        assert_eq!(state.hydration_permill, 950);
        assert_eq!(state.wear_permill, 25);
    }
}
