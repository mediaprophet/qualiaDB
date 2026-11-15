//! Canonical Ethogram Registry & Behavioral State Machine Kernel (WP-2.1).
//!
//! Formalizes the binding between:
//! 1. `EONT1` binary affordance tables ([`crate::container_10d::entity_ontology_section`])
//! 2. `.vibe` HFSM behavioral rules (stimulus events, internal drive thresholds)
//! 3. `SKL1` skeletal animation motor clips ([`crate::container_10d::skeletal_section`])
//!
//! Zero-heap on hot query and tick paths.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// Maximum behaviors registered in a single compact entity ethogram.
pub const MAX_ETHOGRAM_BEHAVIORS: usize = 32;

/// Stimulus topic categories driving animal ethology and plant tropisms.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StimulusCategory {
    #[default]
    AmbientProximity = 0,
    PredatorAlert = 1,
    PreyOrFoodDetected = 2,
    MatingCall = 3,
    WeatherDrought = 4,
    WeatherDeluge = 5,
    WildfireSmoke = 6,
    PhysicalContact = 7,
    HumanObserverApproaching = 8,
}

/// An incoming environmental or sensory stimulus event.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct StimulusEvent {
    pub topic_hash: u64,
    pub source_did_hash: u64,
    pub intensity_permill: u16,
    pub category: u8,
    pub reserved_u8: u8,
    pub distance_meters: f32,
}

/// Dynamic physiological internal state meters of an active entity.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize, Default)]
pub struct EntityPhysiologicalState {
    pub energy_permill: u16,
    pub damage_permill: u16,
    pub hydration_permill: u16,
    pub wear_permill: u16,
    pub active_state_id: u16,
    pub lifecycle_stage: u8,
    pub reserved_u8: u8,
    pub active_motor_clip_id: u32,
}

/// A fixed-stride binary entry representing one state-transition behavior in the ethogram.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize, Default)]
pub struct EthogramBehaviorEntry {
    /// Action identifier (e.g. `q_hash("action:flee")`).
    pub action_id: u64,
    /// Stimulus topic hash that triggers this behavior.
    pub trigger_stimulus_hash: u64,
    /// Target behavioral state ID upon transition.
    pub target_state_id: u16,
    /// Required precondition flags bitmask.
    pub precondition_mask: u16,
    /// Target SKL1 skeletal motor clip to play.
    pub animation_clip_id: u32,
    /// Energy change delta permill (+/-).
    pub energy_delta_permill: i16,
    /// Hydration change delta permill (+/-).
    pub hydration_delta_permill: i16,
    /// Damage change delta permill (+/-).
    pub damage_delta_permill: i16,
    /// Behavioral priority (higher values preempt lower priority behaviors).
    pub priority: u8,
    /// Category of affordance (Physiological, Locomotion, Social, etc.).
    pub category: u8,
}

/// Evaluated transition verdict when a stimulus triggers a behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionVerdict {
    /// True when a behavior transition fired (vs. no matching rule).
    pub triggered: bool,
    /// State id entered after the transition (unchanged when not triggered).
    pub target_state_id: u16,
    /// Animation clip to play for the transition (0 = none).
    pub animation_clip_id: u32,
    /// Energy level after applying the behavior's delta (0..1000 permill).
    pub new_energy_permill: u16,
    /// Hydration level after the transition.
    pub new_hydration_permill: u16,
    /// Damage level after the transition.
    pub new_damage_permill: u16,
}

/// A fixed-capacity, zero-heap Ethogram Registry for an entity or taxon family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthogramRegistry {
    /// `q_hash` of the taxon/entity-family IRI this registry describes.
    pub taxon_iri_hash: u64,
    /// Number of live entries in `behaviors` (≤ `MAX_ETHOGRAM_BEHAVIORS`).
    pub behavior_count: usize,
    /// Fixed-capacity entry table; only the first `behavior_count` are live.
    pub behaviors: [EthogramBehaviorEntry; MAX_ETHOGRAM_BEHAVIORS],
}

impl Default for EthogramRegistry {
    fn default() -> Self {
        Self {
            taxon_iri_hash: 0,
            behavior_count: 0,
            behaviors: [EthogramBehaviorEntry::default(); MAX_ETHOGRAM_BEHAVIORS],
        }
    }
}

impl EthogramRegistry {
    /// Create an empty registry for a taxon (all `MAX_ETHOGRAM_BEHAVIORS`
    /// slots zeroed); `const` so it can live in statics.
    pub const fn new(taxon_iri_hash: u64) -> Self {
        Self {
            taxon_iri_hash,
            behavior_count: 0,
            behaviors: [EthogramBehaviorEntry {
                action_id: 0,
                trigger_stimulus_hash: 0,
                target_state_id: 0,
                precondition_mask: 0,
                animation_clip_id: 0,
                energy_delta_permill: 0,
                hydration_delta_permill: 0,
                damage_delta_permill: 0,
                priority: 0,
                category: 0,
            }; MAX_ETHOGRAM_BEHAVIORS],
        }
    }

    /// Add a behavior entry to the registry. Returns false if registry is full.
    pub fn add_behavior(&mut self, entry: EthogramBehaviorEntry) -> bool {
        if self.behavior_count >= MAX_ETHOGRAM_BEHAVIORS {
            return false;
        }
        self.behaviors[self.behavior_count] = entry;
        self.behavior_count += 1;
        true
    }

    /// Evaluate an incoming stimulus against current state in $O(1)$ zero-heap time.
    /// Updates `current_state` in-place and returns the transition verdict.
    pub fn evaluate_stimulus(
        &self,
        stimulus: &StimulusEvent,
        current_state: &mut EntityPhysiologicalState,
    ) -> TransitionVerdict {
        let mut best_idx: Option<usize> = None;
        let mut best_priority = 0u8;

        for i in 0..self.behavior_count {
            let b = &self.behaviors[i];
            if b.trigger_stimulus_hash == stimulus.topic_hash {
                if best_idx.is_none() || b.priority >= best_priority {
                    best_idx = Some(i);
                    best_priority = b.priority;
                }
            }
        }

        if let Some(idx) = best_idx {
            let b = &self.behaviors[idx];

            let new_energy = apply_delta(current_state.energy_permill, b.energy_delta_permill);
            let new_hydration =
                apply_delta(current_state.hydration_permill, b.hydration_delta_permill);
            let new_damage = apply_delta(current_state.damage_permill, b.damage_delta_permill);

            current_state.energy_permill = new_energy;
            current_state.hydration_permill = new_hydration;
            current_state.damage_permill = new_damage;
            current_state.active_state_id = b.target_state_id;
            current_state.active_motor_clip_id = b.animation_clip_id;

            TransitionVerdict {
                triggered: true,
                target_state_id: b.target_state_id,
                animation_clip_id: b.animation_clip_id,
                new_energy_permill: new_energy,
                new_hydration_permill: new_hydration,
                new_damage_permill: new_damage,
            }
        } else {
            TransitionVerdict {
                triggered: false,
                target_state_id: current_state.active_state_id,
                animation_clip_id: current_state.active_motor_clip_id,
                new_energy_permill: current_state.energy_permill,
                new_hydration_permill: current_state.hydration_permill,
                new_damage_permill: current_state.damage_permill,
            }
        }
    }
}

#[inline]
fn apply_delta(base: u16, delta: i16) -> u16 {
    let result = (base as i32) + (delta as i32);
    if result < 0 {
        0
    } else if result > 1000 {
        1000
    } else {
        result as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_view::entity_ontologies::{AffordanceCategory, LifecycleStage};

    #[test]
    fn test_ethogram_behavior_evaluation() {
        let mut ethogram = EthogramRegistry::new(12345);
        ethogram.add_behavior(EthogramBehaviorEntry {
            action_id: 101,
            trigger_stimulus_hash: 999, // Predator detected
            target_state_id: 3,         // Fleeing
            precondition_mask: 0,
            animation_clip_id: 12, // Gallop/flee clip
            energy_delta_permill: -50,
            hydration_delta_permill: -30,
            damage_delta_permill: 0,
            priority: 10,
            category: AffordanceCategory::Physical as u8,
        });

        let mut state = EntityPhysiologicalState {
            energy_permill: 800,
            damage_permill: 50,
            hydration_permill: 600,
            wear_permill: 0,
            active_state_id: 1, // Grazing
            lifecycle_stage: LifecycleStage::Adult as u8,
            reserved_u8: 0,
            active_motor_clip_id: 2,
        };

        let stimulus = StimulusEvent {
            topic_hash: 999,
            source_did_hash: 777,
            intensity_permill: 900,
            category: StimulusCategory::PredatorAlert as u8,
            reserved_u8: 0,
            distance_meters: 15.0,
        };

        let verdict = ethogram.evaluate_stimulus(&stimulus, &mut state);
        assert!(verdict.triggered);
        assert_eq!(verdict.target_state_id, 3);
        assert_eq!(verdict.animation_clip_id, 12);
        assert_eq!(state.energy_permill, 750);
        assert_eq!(state.hydration_permill, 570);
        assert_eq!(state.active_state_id, 3);
    }
}
