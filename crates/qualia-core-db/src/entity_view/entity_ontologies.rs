//! Entity Behavioral & Affordance Ontologies for Digital Twins.
//!
//! Digital twin representations model entities across distinct ontological domains:
//! - **Darwinian / Ecological**: Natural organisms, species, and ecological actors governed
//!   by evolutionary biology and life history (not moral or legal sovereigns).
//! - **Manufactured Property**: Engineered artifacts, vehicles, machinery, and constructed
//!   infrastructure subject to property rights, operational ratings, and maintenance regimes.
//! - **Abiotic Aggregates**: Geological, soil, hydrological, and atmospheric systems.
//! - **Liberal Arts & Scientific Works**: The authored descriptive model itself (3D geometry,
//!   semantic graphs, audio, textures) as an expressive creative and scientific work.
//!
//! This module declares an entity's intrinsic operational profile:
//! - Discoverable affordances (`wot:ActionAffordance`, `schema:Action`)
//! - Machine-readable preconditions (required state/inventory/energy/biomass/permissions)
//! - State transition effects (energy delta, wear delta, damage delta, animation clips)
//! - Dynamic AI tool manifest generation (standard JSON schema for LLMs and autonomous agents)
//! - Deterministic, bounded intent evaluation emitting 48-byte `NQuin` action records within
//!   the 42MB Sentinel.

use serde::{Deserialize, Serialize};

use crate::entity_view::entity_id::EntityId;
use crate::entity_view::observer::ObserverStatus;
use crate::q_hash;
use crate::NQuin;

/// Ontological origin and domain of the represented entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum OntologicalOrigin {
    /// Darwinian organism or evolutionary taxon (part of the natural world / biosphere).
    #[default]
    DarwinianTaxon = 0,
    /// Manufactured physical property, engineering asset, or fabricated machine.
    ManufacturedProperty = 1,
    /// Abiotic physical/environmental feature (rock, water body, terrain aggregate).
    AbioticEnvironment = 2,
    /// Expressive, liberal arts, cultural heritage, or open scientific dataset work.
    LiberalArtsWork = 3,
}

/// Biological or mechanical lifecycle stage of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum LifecycleStage {
    /// Botanical / fungal seed, spore, or dormant embryonic stage.
    SeedOrSpore = 0,
    /// Developing juvenile, nymph, larva, seedling, or sapling.
    Juvenile = 1,
    /// Morphologically mature, fully functional organism.
    #[default]
    Adult = 2,
    /// Post-reproductive or aged organism exhibiting degradation.
    Senescent = 3,
    /// Organic remains undergoing decomposition.
    Deceased = 4,
    /// Assembled / fabricated mechanical or architectural asset before commissioning.
    Manufactured = 5,
    /// Commissioned, fully operational constructed entity.
    Operational = 6,
    /// Degraded operational state requiring maintenance or repair.
    Degraded = 7,
    /// Decommissioned, scrap, or inert salvage.
    Decommissioned = 8,
}

impl LifecycleStage {
    pub const fn from_u8(raw: u8) -> Self {
        match raw {
            0 => Self::SeedOrSpore,
            1 => Self::Juvenile,
            2 => Self::Adult,
            3 => Self::Senescent,
            4 => Self::Deceased,
            5 => Self::Manufactured,
            6 => Self::Operational,
            7 => Self::Degraded,
            8 => Self::Decommissioned,
            _ => Self::Adult,
        }
    }
}

/// Category of an affordance for behavioral dispatch and UI classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum AffordanceCategory {
    /// Ingestion, resting, reproduction, phototropism, respiration.
    #[default]
    Biological = 0,
    /// Locomotion, manipulation, collision response, physical carrying.
    Physical = 1,
    /// Communication, herding, vocalization, mutualism, grooming, trading.
    Social = 2,
    /// Lubrication, fueling, pruning, weeding, structural repair, parts replacement.
    Maintenance = 3,
    /// Non-invasive sensor probing, sampling, telemetry read, spectral scan.
    Sensing = 4,
    /// Deontic admission, quarantine, guardianship, lease or ownership transfer.
    Governance = 5,
}

impl AffordanceCategory {
    pub const fn from_u8(raw: u8) -> Self {
        match raw {
            0 => Self::Biological,
            1 => Self::Physical,
            2 => Self::Social,
            3 => Self::Maintenance,
            4 => Self::Sensing,
            5 => Self::Governance,
            _ => Self::Biological,
        }
    }
}

/// Operational preconditions that must be satisfied before an affordance can be invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPrecondition {
    /// Minimum energy/biomass level in parts-per-thousand (0..1000).
    pub min_energy_permill: u16,
    /// Maximum allowed physical damage before affordance is disabled (0..1000).
    pub max_damage_permill: u16,
    /// Minimum hydration level required (0..1000).
    pub min_hydration_permill: u16,
    /// Maximum wear level allowed (0..1000).
    pub max_wear_permill: u16,
    /// Optional hash of required tool or capability (0 = none).
    pub required_capability_hash: u64,
    /// Minimum observer status / role required to invoke (e.g. Principal or Steward).
    pub min_observer_status: ObserverStatus,
}

impl Default for ActionPrecondition {
    fn default() -> Self {
        Self {
            min_energy_permill: 0,
            max_damage_permill: 1000,
            min_hydration_permill: 0,
            max_wear_permill: 1000,
            required_capability_hash: 0,
            min_observer_status: ObserverStatus::Public,
        }
    }
}

/// State transition effects emitted upon successful execution of an affordance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ActionEffect {
    /// Energy change delta (-1000..+1000 permill).
    pub energy_delta: i16,
    /// Damage change delta (-1000..+1000 permill, negative means repaired).
    pub damage_delta: i16,
    /// Hydration change delta.
    pub hydration_delta: i16,
    /// Wear change delta (positive means accrued wear).
    pub wear_delta: i16,
    /// Hash of the event topic or produced resource emitted to the session journal.
    pub emitted_topic_hash: u64,
    /// Visual/skeletal animation clip ID to trigger (0 = none).
    pub animation_clip_id: u32,
}

/// A discoverable affordance exposed by a digital twin entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionAffordance {
    /// Stable 64-bit identifier (usually `q_hash(action_name)`).
    pub action_id: u64,
    /// Machine-readable identifier (e.g. `"feed"`, `"prune"`, `"start_pump"`, `"inspect"`).
    pub name: String,
    /// Human and LLM readable semantic documentation of what the action accomplishes.
    pub description: String,
    /// Categorical taxonomic grouping.
    pub category: AffordanceCategory,
    /// Preconditions guarding execution.
    pub preconditions: ActionPrecondition,
    /// Intended state effects upon execution.
    pub effects: ActionEffect,
    /// JSON Schema defining parameters for LLM tool calling (e.g. `{"type":"object",...}`).
    pub input_schema_json: String,
}

impl ActionAffordance {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: AffordanceCategory,
    ) -> Self {
        let name_str = name.into();
        let action_id = q_hash(&name_str);
        Self {
            action_id,
            name: name_str,
            description: description.into(),
            category,
            preconditions: ActionPrecondition::default(),
            effects: ActionEffect::default(),
            input_schema_json: "{\"type\":\"object\",\"properties\":{}}".to_string(),
        }
    }
}

/// Entity behavioral and affordance ontology profile for an entity instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityOntology {
    /// Entity ID of the referent digital twin.
    pub entity_id: EntityId,
    /// Ontological origin (Darwinian taxon, manufactured property, abiotic aggregate, liberal arts work).
    pub origin: OntologicalOrigin,
    /// IRI or ontological class (e.g. `qualia:EucalyptusRegnans`, `qualia:CentrifugalPump`).
    pub taxon_or_system_iri: String,
    /// Current developmental or functional lifecycle stage.
    pub stage: LifecycleStage,
    /// Current internal energy / biomass reserve (0..1000).
    pub energy_permill: u16,
    /// Current damage / degradation level (0..1000).
    pub damage_permill: u16,
    /// Current hydration or fluid reserve (0..1000).
    pub hydration_permill: u16,
    /// Current mechanical / biological wear level (0..1000).
    pub wear_permill: u16,
    /// List of discoverable affordances published by this entity.
    pub affordances: Vec<ActionAffordance>,
}

/// Compatibility type alias.
pub type PersonalOntology = EntityOntology;

impl EntityOntology {
    pub fn new(entity_id: EntityId, taxon_or_system_iri: impl Into<String>) -> Self {
        Self {
            entity_id,
            origin: OntologicalOrigin::DarwinianTaxon,
            taxon_or_system_iri: taxon_or_system_iri.into(),
            stage: LifecycleStage::Adult,
            energy_permill: 1000,
            damage_permill: 0,
            hydration_permill: 1000,
            wear_permill: 0,
            affordances: Vec::new(),
        }
    }

    /// Add an affordance to the entity ontology.
    pub fn add_affordance(&mut self, affordance: ActionAffordance) {
        if let Some(pos) = self
            .affordances
            .iter()
            .position(|a| a.action_id == affordance.action_id)
        {
            self.affordances[pos] = affordance;
        } else {
            self.affordances.push(affordance);
        }
    }

    /// Generate a standard JSON tool manifest compatible with LLM function-calling APIs
    /// (OpenAI, Anthropic, Gemini, local Ollama/vLLM) for AI agent triggering.
    pub fn export_ai_tools_json(&self) -> String {
        let mut out = String::from("[\n");
        for (i, aff) in self.affordances.iter().enumerate() {
            if i > 0 {
                out.push_str(",\n");
            }
            out.push_str("  {\n");
            out.push_str("    \"type\": \"function\",\n");
            out.push_str("    \"function\": {\n");
            out.push_str(&format!("      \"name\": \"{}\",\n", aff.name));
            out.push_str(&format!(
                "      \"description\": \"{}\",\n",
                aff.description.replace('"', "\\\"")
            ));
            out.push_str(&format!(
                "      \"parameters\": {}\n",
                aff.input_schema_json
            ));
            out.push_str("    }\n");
            out.push_str("  }");
        }
        out.push_str("\n]");
        out
    }

    /// Evaluate an incoming action intent against this entity ontology.
    /// Pure, zero-heap evaluation that validates preconditions, enforces permissions,
    /// updates internal state registers, and generates an auditable 48-byte `NQuin`.
    pub fn evaluate_intent(
        &self,
        intent: &ActionIntent,
        caller_status: ObserverStatus,
    ) -> Result<ActionEvaluationVerdict, IntentError> {
        let aff = self
            .affordances
            .iter()
            .find(|a| a.action_id == intent.action_id)
            .ok_or(IntentError::AffordanceNotFound {
                action_id: intent.action_id,
            })?;

        // 1. Permission check
        if (caller_status as u8) > (aff.preconditions.min_observer_status as u8) {
            return Err(IntentError::PermissionDenied {
                required: aff.preconditions.min_observer_status,
                provided: caller_status,
            });
        }

        // 2. Preconditions check
        if self.energy_permill < aff.preconditions.min_energy_permill {
            return Err(IntentError::InsufficientEnergy {
                have: self.energy_permill,
                need: aff.preconditions.min_energy_permill,
            });
        }
        if self.damage_permill > aff.preconditions.max_damage_permill {
            return Err(IntentError::ExcessiveDamage {
                have: self.damage_permill,
                max_allowed: aff.preconditions.max_damage_permill,
            });
        }
        if self.hydration_permill < aff.preconditions.min_hydration_permill {
            return Err(IntentError::InsufficientHydration {
                have: self.hydration_permill,
                need: aff.preconditions.min_hydration_permill,
            });
        }
        if self.wear_permill > aff.preconditions.max_wear_permill {
            return Err(IntentError::ExcessiveWear {
                have: self.wear_permill,
                max_allowed: aff.preconditions.max_wear_permill,
            });
        }

        // 3. Compute updated state registers
        let new_energy = ((self.energy_permill as i32) + (aff.effects.energy_delta as i32))
            .clamp(0, 1000) as u16;
        let new_damage = ((self.damage_permill as i32) + (aff.effects.damage_delta as i32))
            .clamp(0, 1000) as u16;
        let new_hydration = ((self.hydration_permill as i32) + (aff.effects.hydration_delta as i32))
            .clamp(0, 1000) as u16;
        let new_wear =
            ((self.wear_permill as i32) + (aff.effects.wear_delta as i32)).clamp(0, 1000) as u16;

        // 4. Construct 48-byte Super-Quin representing the executed action event
        let subject = self.entity_id.0;
        let predicate = aff.action_id;
        let object = intent.parameters_hash;
        let context = intent.caller_did_hash;
        // Metadata: PermissiveCommons lane (0x01) + Lamport clock in lower 32 bits
        let metadata = (0x01u64 << 61) | ((intent.lamport_clock as u64) & 0xFFFF_FFFF);
        // Universal Quin XOR fold parity
        let parity = subject ^ predicate ^ object ^ context ^ metadata;

        let record_quin = NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        };

        Ok(ActionEvaluationVerdict {
            new_energy,
            new_damage,
            new_hydration,
            new_wear,
            animation_clip_id: aff.effects.animation_clip_id,
            emitted_topic_hash: aff.effects.emitted_topic_hash,
            record_quin,
        })
    }

    /// Serialise the ontology's affordance structure into caller-supplied `NQuin` slices.
    /// Returns the number of quins written. Zero-heap operation.
    pub fn to_nquins(&self, out: &mut [NQuin]) -> Result<usize, IntentError> {
        let needed = 1 + self.affordances.len();
        if out.len() < needed {
            return Err(IntentError::OutputBufferTooSmall {
                needed,
                have: out.len(),
            });
        }

        let taxon_hash = q_hash(&self.taxon_or_system_iri);
        let entity_sub = self.entity_id.0;

        // Base quin: Entity is-a Taxon
        let is_a_pred = q_hash("rdf:type");
        let meta_clock = 1u64;
        let p0 = entity_sub ^ is_a_pred ^ taxon_hash ^ 0 ^ meta_clock;
        out[0] = NQuin {
            subject: entity_sub,
            predicate: is_a_pred,
            object: taxon_hash,
            context: (self.stage as u64) << 56,
            metadata: meta_clock,
            parity: p0,
        };

        let has_affordance_pred = q_hash("wot:hasActionAffordance");
        for (i, aff) in self.affordances.iter().enumerate() {
            let cat_byte = aff.category as u64;
            let ctx = (cat_byte << 56) | (aff.preconditions.min_observer_status as u64);
            let meta = ((aff.effects.animation_clip_id as u64) << 32)
                | (aff.preconditions.min_energy_permill as u64);
            let parity = entity_sub ^ has_affordance_pred ^ aff.action_id ^ ctx ^ meta;
            out[1 + i] = NQuin {
                subject: entity_sub,
                predicate: has_affordance_pred,
                object: aff.action_id,
                context: ctx,
                metadata: meta,
                parity,
            };
        }

        Ok(needed)
    }
}

/// An action invocation intent submitted by an AI agent or player controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionIntent {
    /// DID hash of the acting agent or player.
    pub caller_did_hash: u64,
    /// Target entity DID hash.
    pub target_entity_hash: u64,
    /// Action affordance ID being invoked.
    pub action_id: u64,
    /// Hash of input parameter payload (or 0 for parameterless actions).
    pub parameters_hash: u64,
    /// Lamport clock tick of the caller's session.
    pub lamport_clock: u32,
}

/// The result of a successfully validated and executed action intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionEvaluationVerdict {
    pub new_energy: u16,
    pub new_damage: u16,
    pub new_hydration: u16,
    pub new_wear: u16,
    pub animation_clip_id: u32,
    pub emitted_topic_hash: u64,
    pub record_quin: NQuin,
}

/// Errors returned when an action intent cannot be admitted or executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentError {
    AffordanceNotFound {
        action_id: u64,
    },
    PermissionDenied {
        required: ObserverStatus,
        provided: ObserverStatus,
    },
    InsufficientEnergy {
        have: u16,
        need: u16,
    },
    ExcessiveDamage {
        have: u16,
        max_allowed: u16,
    },
    InsufficientHydration {
        have: u16,
        need: u16,
    },
    ExcessiveWear {
        have: u16,
        max_allowed: u16,
    },
    MissingCapability {
        required_hash: u64,
    },
    OutputBufferTooSmall {
        needed: usize,
        have: usize,
    },
}

impl std::fmt::Display for IntentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AffordanceNotFound { action_id } => {
                write!(f, "affordance {action_id:#018x} not found on entity")
            }
            Self::PermissionDenied { required, provided } => {
                write!(
                    f,
                    "permission denied: required {required:?}, got {provided:?}"
                )
            }
            Self::InsufficientEnergy { have, need } => {
                write!(f, "insufficient energy: have {have}‰, need {need}‰")
            }
            Self::ExcessiveDamage { have, max_allowed } => {
                write!(
                    f,
                    "excessive damage: have {have}‰, max allowed {max_allowed}‰"
                )
            }
            Self::InsufficientHydration { have, need } => {
                write!(f, "insufficient hydration: have {have}‰, need {need}‰")
            }
            Self::ExcessiveWear { have, max_allowed } => {
                write!(
                    f,
                    "excessive wear: have {have}‰, max allowed {max_allowed}‰"
                )
            }
            Self::MissingCapability { required_hash } => {
                write!(f, "missing required capability {required_hash:#018x}")
            }
            Self::OutputBufferTooSmall { needed, have } => {
                write!(f, "output buffer too small: need {needed}, have {have}")
            }
        }
    }
}

impl std::error::Error for IntentError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_ontology_evaluates_intent_and_emits_valid_quin() {
        let entity_id = EntityId::from_u64(0x1234_5678);
        let mut onto = EntityOntology::new(entity_id, "qualia:EucalyptusRegnans");
        onto.energy_permill = 800;
        onto.damage_permill = 50;

        let mut prune = ActionAffordance::new(
            "prune",
            "Remove dead or hazardous foliage to stimulate growth",
            AffordanceCategory::Maintenance,
        );
        prune.preconditions.min_observer_status = ObserverStatus::Steward;
        prune.preconditions.min_energy_permill = 200;
        prune.effects.energy_delta = -100;
        prune.effects.damage_delta = -20;
        prune.effects.animation_clip_id = 42;
        onto.add_affordance(prune);

        let intent = ActionIntent {
            caller_did_hash: 0xABCD_EF01,
            target_entity_hash: entity_id.0,
            action_id: q_hash("prune"),
            parameters_hash: 0,
            lamport_clock: 10,
        };

        // Unauthorized caller (Public < Steward)
        let fail = onto.evaluate_intent(&intent, ObserverStatus::Public);
        assert!(matches!(fail, Err(IntentError::PermissionDenied { .. })));

        // Authorized caller (Steward)
        let verdict = onto
            .evaluate_intent(&intent, ObserverStatus::Steward)
            .unwrap();
        assert_eq!(verdict.new_energy, 700);
        assert_eq!(verdict.new_damage, 30);
        assert_eq!(verdict.animation_clip_id, 42);

        // Verify emitted NQuin parity
        let q = &verdict.record_quin;
        let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
        assert_eq!(q.parity, expected_parity);

        // AI tool export contains JSON function definition
        let tools_json = onto.export_ai_tools_json();
        assert!(tools_json.contains("\"name\": \"prune\""));
        assert!(tools_json.contains("\"type\": \"function\""));

        // NQuin export fits within bounded caller array
        let mut buffer = [NQuin::default(); 8];
        let count = onto.to_nquins(&mut buffer).unwrap();
        assert_eq!(count, 2);
    }
}
