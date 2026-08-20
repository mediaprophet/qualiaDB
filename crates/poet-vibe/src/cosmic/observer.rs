//! Observer fiber, affective status, and epistemic divergence (OCS §8).
//!
//! Reference: OCS Specification v2.2.0 §8.

use crate::value::Value;
use std::collections::BTreeMap;

/// Affective status vector (OCS §8.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffectiveStatus {
    /// [-1.0: Severe Threat/Panic, +1.0: Deep Safety/Trust]
    pub safety_threat_index: f32,
    /// [-1.0: Extreme Grief/Depression, +1.0: Joy/Euphoria]
    pub emotional_valence: f32,
    /// [0.0: Comatose/Sedated, 1.0: Hyper-vigilant]
    pub arousal_level: f32,
    /// [0.0: Fully Grounded, 1.0: Complete Depersonalization]
    pub dissociation_index: f32,
    /// [0.0: Baseline, 1.0: Active Flashback/Trigger]
    pub trauma_reactivity: f32,
}

impl Default for AffectiveStatus {
    fn default() -> Self {
        Self {
            safety_threat_index: 0.0,
            emotional_valence: 0.0,
            arousal_level: 0.5,
            dissociation_index: 0.0,
            trauma_reactivity: 0.0,
        }
    }
}

impl AffectiveStatus {
    /// Clamp all values to their valid ranges.
    pub fn clamped(&self) -> Self {
        Self {
            safety_threat_index: self.safety_threat_index.clamp(-1.0, 1.0),
            emotional_valence: self.emotional_valence.clamp(-1.0, 1.0),
            arousal_level: self.arousal_level.clamp(0.0, 1.0),
            dissociation_index: self.dissociation_index.clamp(0.0, 1.0),
            trauma_reactivity: self.trauma_reactivity.clamp(0.0, 1.0),
        }
    }

    /// Whether this observer is in a hyper-vigilant state (OCS-T11).
    pub fn is_hyper_vigilant(&self) -> bool {
        self.safety_threat_index > 0.8 || self.arousal_level > 0.9
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("safety_threat_index".into(), Value::F64(self.safety_threat_index as f64));
        rec.insert("emotional_valence".into(), Value::F64(self.emotional_valence as f64));
        rec.insert("arousal_level".into(), Value::F64(self.arousal_level as f64));
        rec.insert("dissociation_index".into(), Value::F64(self.dissociation_index as f64));
        rec.insert("trauma_reactivity".into(), Value::F64(self.trauma_reactivity as f64));
        Value::Record(rec)
    }
}

/// A doxastic belief with evidential weight (OCS §8.2).
#[derive(Debug, Clone, PartialEq)]
pub struct DoxasticBelief {
    pub proposition_iri: String,
    /// Evidential support weight μ ∈ [0, 1]
    pub mu: f32,
    /// Evidential refutation weight λ ∈ [0, 1]
    pub lambda: f32,
}

/// Cognitive lens — perceptual/sensory transfer function (OCS §8.2).
#[derive(Debug, Clone, PartialEq)]
pub struct CognitiveLens {
    pub lens_id: String,
    /// Sensory acuity multiplier (1.0 = normal)
    pub sensory_acuity: f32,
    /// Whether this lens applies indoctrinated worldview filtering
    pub worldview_filter: bool,
}

impl Default for CognitiveLens {
    fn default() -> Self {
        Self {
            lens_id: "default".into(),
            sensory_acuity: 1.0,
            worldview_filter: false,
        }
    }
}

/// Observer fiber — the inward inversion modeling how an observer
/// perceives, filters, and constructs their experienced reality (OCS §8.2).
#[derive(Debug, Clone)]
pub struct ObserverFiber {
    pub observer_did: String,
    pub semantic_cohort: Option<String>,
    pub affective_state: AffectiveStatus,
    pub epistemic_beliefs: Vec<DoxasticBelief>,
    pub cognitive_lens: CognitiveLens,
    pub perceived_frame_usi: String,
}

impl ObserverFiber {
    pub fn new(observer_did: &str, perceived_frame: &str) -> Self {
        Self {
            observer_did: observer_did.into(),
            semantic_cohort: None,
            affective_state: AffectiveStatus::default(),
            epistemic_beliefs: Vec::new(),
            cognitive_lens: CognitiveLens::default(),
            perceived_frame_usi: perceived_frame.into(),
        }
    }

    /// Clinical epistemic divergence tensor Δ_epistemic (OCS §8.3).
    ///
    /// Simple scalar version: the L2 norm of the difference between
    /// the perceived frame and the empirical frame, weighted by
    /// dissociation and trauma reactivity.
    pub fn epistemic_divergence(&self, empirical_frame_coords: &[f64], perceived_coords: &[f64]) -> f64 {
        if empirical_frame_coords.len() != perceived_coords.len() {
            return f64::INFINITY;
        }
        let sum_sq: f64 = empirical_frame_coords
            .iter()
            .zip(perceived_coords.iter())
            .map(|(e, p)| {
                let d = e - p;
                d * d
            })
            .sum();
        let base = sum_sq.sqrt();
        // Weight by dissociation + trauma reactivity
        let weight = 1.0 + self.affective_state.dissociation_index as f64
            + self.affective_state.trauma_reactivity as f64;
        base * weight
    }

    /// Whether this observer needs grounding intervention (OCS-T11).
    pub fn needs_grounding(&self) -> bool {
        self.affective_state.is_hyper_vigilant()
            || self.affective_state.dissociation_index > 0.7
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("observer_did".into(), Value::String(self.observer_did.clone()));
        if let Some(cohort) = &self.semantic_cohort {
            rec.insert("semantic_cohort".into(), Value::String(cohort.clone()));
        }
        rec.insert("affective_state".into(), self.affective_state.to_value());
        rec.insert("perceived_frame_usi".into(), Value::String(self.perceived_frame_usi.clone()));
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_affective_is_neutral() {
        let a = AffectiveStatus::default();
        assert_eq!(a.safety_threat_index, 0.0);
        assert_eq!(a.emotional_valence, 0.0);
        assert_eq!(a.arousal_level, 0.5);
    }

    #[test]
    fn hyper_vigilant_detection() {
        let a = AffectiveStatus {
            safety_threat_index: 0.9,
            ..Default::default()
        };
        assert!(a.is_hyper_vigilant());
    }

    #[test]
    fn not_hyper_vigilant() {
        let a = AffectiveStatus::default();
        assert!(!a.is_hyper_vigilant());
    }

    #[test]
    fn affective_clamp() {
        let a = AffectiveStatus {
            safety_threat_index: 5.0,
            emotional_valence: -3.0,
            arousal_level: 2.0,
            dissociation_index: -1.0,
            trauma_reactivity: 10.0,
        };
        let c = a.clamped();
        assert_eq!(c.safety_threat_index, 1.0);
        assert_eq!(c.emotional_valence, -1.0);
        assert_eq!(c.arousal_level, 1.0);
        assert_eq!(c.dissociation_index, 0.0);
        assert_eq!(c.trauma_reactivity, 1.0);
    }

    #[test]
    fn epistemic_divergence_zero() {
        let obs = ObserverFiber::new("did:q42:person:test", "urn:omni:v1:phenomenology:test");
        let coords = [1.0, 2.0, 3.0];
        let d = obs.epistemic_divergence(&coords, &coords);
        assert!(d.abs() < 1e-10);
    }

    #[test]
    fn epistemic_divergence_nonzero() {
        let obs = ObserverFiber::new("did:q42:person:test", "urn:omni:v1:phenomenology:test");
        let empirical = [0.0, 0.0, 0.0];
        let perceived = [3.0, 4.0, 0.0]; // distance = 5
        let d = obs.epistemic_divergence(&empirical, &perceived);
        assert!((d - 5.0).abs() < 1e-10, "got {} expected 5.0", d);
    }

    #[test]
    fn epistemic_divergence_weighted_by_dissociation() {
        let mut obs = ObserverFiber::new("did:q42:person:test", "urn:omni:v1:phenomenology:test");
        obs.affective_state.dissociation_index = 1.0;
        let empirical = [0.0, 0.0, 0.0];
        let perceived = [3.0, 4.0, 0.0]; // distance = 5
        let d = obs.epistemic_divergence(&empirical, &perceived);
        // weight = 1 + 1.0 + 0.0 = 2.0 → 5 * 2 = 10
        assert!((d - 10.0).abs() < 1e-10, "got {} expected 10.0", d);
    }

    #[test]
    fn needs_grounding_hyper_vigilant() {
        let mut obs = ObserverFiber::new("did:q42:person:test", "frame");
        obs.affective_state.safety_threat_index = 0.9;
        assert!(obs.needs_grounding());
    }

    #[test]
    fn needs_grounding_dissociation() {
        let mut obs = ObserverFiber::new("did:q42:person:test", "frame");
        obs.affective_state.dissociation_index = 0.8;
        assert!(obs.needs_grounding());
    }

    #[test]
    fn observer_to_value() {
        let obs = ObserverFiber::new("did:q42:person:alice", "urn:omni:v1:phenomenology:alice");
        let v = obs.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("observer_did"), Some(&Value::String("did:q42:person:alice".into())));
                assert!(r.contains_key("affective_state"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn affective_to_value() {
        let a = AffectiveStatus::default();
        let v = a.to_value();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("safety_threat_index"));
                assert!(r.contains_key("emotional_valence"));
            }
            _ => panic!("expected Record"),
        }
    }
}
