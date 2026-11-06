//! Observer fiber, affective status, correlated atlas uncertainty, presheaf gluing,
//! and epistemic divergence (OCS §8, Packet OCS-13 & §6.2.5.1).
//!
//! Reference: OCS Specification v2.2.0 §8 & Playbook v4 §6.2.5.1.

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

    /// Whether this observer is in a hyper-vigilant state (OCS-T11, Packet OCS-13).
    ///
    /// Note: `safety_threat_index` is in `[-1.0: Severe Threat/Panic, +1.0: Deep Safety/Trust]`,
    /// so severe threat is `< -0.8` (not `> 0.8`), or high arousal `> 0.9` (or `>= 0.9` with active
    /// trauma reactivity).
    pub fn is_hyper_vigilant(&self) -> bool {
        self.safety_threat_index < -0.8
            || self.arousal_level > 0.9
            || (self.arousal_level >= 0.9 && self.trauma_reactivity > 0.0)
    }

    /// Converts to value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert(
            "safety_threat_index".into(),
            Value::F64(self.safety_threat_index as f64),
        );
        rec.insert(
            "emotional_valence".into(),
            Value::F64(self.emotional_valence as f64),
        );
        rec.insert(
            "arousal_level".into(),
            Value::F64(self.arousal_level as f64),
        );
        rec.insert(
            "dissociation_index".into(),
            Value::F64(self.dissociation_index as f64),
        );
        rec.insert(
            "trauma_reactivity".into(),
            Value::F64(self.trauma_reactivity as f64),
        );
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

impl DoxasticBelief {
    /// Converts to `Value::Record`.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert(
            "proposition_iri".into(),
            Value::String(self.proposition_iri.clone()),
        );
        rec.insert("mu".into(), Value::F64(self.mu as f64));
        rec.insert("lambda".into(), Value::F64(self.lambda as f64));
        Value::Record(rec)
    }
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
/// perceives, filters, and constructs their experienced reality (OCS §8.2, §6.2.5.1).
///
/// Includes split correlated uncertainty (`shared_anchor_var` vs `local_chart_var`) so
/// relative displacements between two observers sharing a coarse anchor cancel the
/// coarse anchor variance rather than inflating local relative error.
#[derive(Debug, Clone)]
pub struct ObserverFiber {
    pub observer_did: String,
    pub semantic_cohort: Option<String>,
    pub affective_state: AffectiveStatus,
    pub epistemic_beliefs: Vec<DoxasticBelief>,
    pub cognitive_lens: CognitiveLens,
    pub perceived_frame_usi: String,
    /// Optional shared coarse anchor identifier (e.g. sector / spacecraft / habitat USRI hash).
    pub shared_anchor_id: Option<u64>,
    /// Positional variance ($\text{m}^2$) of the coarse anchor relative to a cosmic root frame.
    pub shared_anchor_var: f64,
    /// Positional variance ($\text{m}^2$) within the local chart relative to `shared_anchor_id`.
    pub local_chart_var: f64,
}

impl ObserverFiber {
    /// Creates a new `ObserverFiber`.
    pub fn new(observer_did: &str, perceived_frame: &str) -> Self {
        Self {
            observer_did: observer_did.into(),
            semantic_cohort: None,
            affective_state: AffectiveStatus::default(),
            epistemic_beliefs: Vec::new(),
            cognitive_lens: CognitiveLens::default(),
            perceived_frame_usi: perceived_frame.into(),
            shared_anchor_id: None,
            shared_anchor_var: 0.0,
            local_chart_var: 0.0,
        }
    }

    /// Configure correlated anchor variance vs local chart variance (§6.2.5.1).
    pub fn with_correlated_uncertainty(
        mut self,
        shared_anchor_id: u64,
        shared_anchor_var: f64,
        local_chart_var: f64,
    ) -> Self {
        self.shared_anchor_id = Some(shared_anchor_id);
        self.shared_anchor_var = shared_anchor_var.max(0.0);
        self.local_chart_var = local_chart_var.max(0.0);
        self
    }

    /// Whether this observer fiber is in a hyper-vigilant state (Packet OCS-13).
    pub fn is_hyper_vigilant(&self) -> bool {
        self.affective_state.safety_threat_index < -0.8
            || self.affective_state.arousal_level > 0.9
            || self.affective_state.is_hyper_vigilant()
    }

    /// Total absolute positional variance ($\text{m}^2$) relative to the cosmic root frame.
    pub fn total_positional_variance(&self) -> f64 {
        self.shared_anchor_var.max(0.0) + self.local_chart_var.max(0.0)
    }

    /// Relative displacement variance ($\text{m}^2$) between `self` and `other` (§6.2.5.1).
    ///
    /// When both observers share the same `shared_anchor_id`, the common-mode coarse anchor
    /// variance cancels in $\Delta\mathbf{x} = \mathbf{x}_A - \mathbf{x}_B$, leaving only
    /// `self.local_chart_var + other.local_chart_var`.
    pub fn relative_displacement_variance(&self, other: &Self) -> f64 {
        let local_sum = self.local_chart_var.max(0.0) + other.local_chart_var.max(0.0);
        match (self.shared_anchor_id, other.shared_anchor_id) {
            (Some(a), Some(b)) if a == b => local_sum,
            _ => local_sum + self.shared_anchor_var.max(0.0) + other.shared_anchor_var.max(0.0),
        }
    }

    /// Standard deviation ($\text{m}$) of the relative displacement between `self` and `other`.
    pub fn relative_displacement_std(&self, other: &Self) -> f64 {
        self.relative_displacement_variance(other).sqrt()
    }

    /// Check presheaf overlap compatibility on $U_i \cap U_j$ before gluing/merging two
    /// observer fibers (§6.2.5.1).
    ///
    /// Two fibers are compatible on their overlap if:
    /// 1. For every shared `proposition_iri`, $|\mu_A - \mu_B| \le \text{max\_belief\_divergence}$
    ///    and $|\lambda_A - \lambda_B| \le \text{max\_belief\_divergence}$.
    /// 2. If both fibers reference the same `shared_anchor_id`, their `shared_anchor_var`
    ///    estimates agree within relative tolerance.
    pub fn check_presheaf_overlap(
        &self,
        other: &Self,
        max_belief_divergence: f32,
    ) -> Result<(), String> {
        if let (Some(a), Some(b)) = (self.shared_anchor_id, other.shared_anchor_id) {
            if a == b {
                let diff = (self.shared_anchor_var - other.shared_anchor_var).abs();
                let scale = self
                    .shared_anchor_var
                    .abs()
                    .max(other.shared_anchor_var.abs())
                    .max(1.0);
                if diff / scale > 1e-3 {
                    return Err(format!(
                        "Presheaf overlap mismatch on shared_anchor_id {}: anchor variance {} vs {}",
                        a, self.shared_anchor_var, other.shared_anchor_var
                    ));
                }
            }
        }

        for b1 in &self.epistemic_beliefs {
            if let Some(b2) = other
                .epistemic_beliefs
                .iter()
                .find(|cand| cand.proposition_iri == b1.proposition_iri)
            {
                let d_mu = (b1.mu - b2.mu).abs();
                let d_lambda = (b1.lambda - b2.lambda).abs();
                if d_mu > max_belief_divergence || d_lambda > max_belief_divergence {
                    return Err(format!(
                        "Presheaf section conflict on proposition '{}': (mu={}, lambda={}) vs (mu={}, lambda={})",
                        b1.proposition_iri, b1.mu, b1.lambda, b2.mu, b2.lambda
                    ));
                }
            }
        }
        Ok(())
    }

    /// Glue two compatible observer fibers over their overlap $U_i \cap U_j$ (§6.2.5.1).
    ///
    /// Fails closed if `check_presheaf_overlap` detects a contradiction on shared propositions
    /// or anchor metadata.
    pub fn merge_presheaf_compatible(
        &self,
        other: &Self,
        max_belief_divergence: f32,
    ) -> Result<ObserverFiber, String> {
        self.check_presheaf_overlap(other, max_belief_divergence)?;

        let mut merged_beliefs = self.epistemic_beliefs.clone();
        for b2 in &other.epistemic_beliefs {
            if let Some(existing) = merged_beliefs
                .iter_mut()
                .find(|cand| cand.proposition_iri == b2.proposition_iri)
            {
                existing.mu = 0.5 * (existing.mu + b2.mu);
                existing.lambda = 0.5 * (existing.lambda + b2.lambda);
            } else {
                merged_beliefs.push(b2.clone());
            }
        }

        let merged_local_var = if self.local_chart_var > 0.0 && other.local_chart_var > 0.0 {
            1.0 / (1.0 / self.local_chart_var + 1.0 / other.local_chart_var)
        } else {
            self.local_chart_var.max(other.local_chart_var)
        };

        Ok(ObserverFiber {
            observer_did: self.observer_did.clone(),
            semantic_cohort: self
                .semantic_cohort
                .clone()
                .or_else(|| other.semantic_cohort.clone()),
            affective_state: self.affective_state,
            epistemic_beliefs: merged_beliefs,
            cognitive_lens: self.cognitive_lens.clone(),
            perceived_frame_usi: self.perceived_frame_usi.clone(),
            shared_anchor_id: self.shared_anchor_id.or(other.shared_anchor_id),
            shared_anchor_var: self.shared_anchor_var.max(other.shared_anchor_var),
            local_chart_var: merged_local_var,
        })
    }

    /// Clinical epistemic divergence tensor Δ_epistemic (OCS §8.3).
    ///
    /// Simple scalar version: the L2 norm of the difference between
    /// the perceived frame and the empirical frame, weighted by
    /// dissociation and trauma reactivity.
    pub fn epistemic_divergence(
        &self,
        empirical_frame_coords: &[f64],
        perceived_coords: &[f64],
    ) -> f64 {
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
        let weight = 1.0
            + self.affective_state.dissociation_index as f64
            + self.affective_state.trauma_reactivity as f64;
        base * weight
    }

    /// Whether this observer needs grounding intervention (OCS-T11).
    pub fn needs_grounding(&self) -> bool {
        self.is_hyper_vigilant() || self.affective_state.dissociation_index >= 0.7
    }

    /// Project the observer's phenomenological and doxastic state onto an explicit
    /// **Claim/Opinion** record (`plane = "ClaimOpinion"`) without mutating physical
    /// coordinates or `NaturalAgent` identity (Packet OCS-13).
    pub fn to_epistemic_claim_record(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("plane".into(), Value::String("ClaimOpinion".into()));
        rec.insert(
            "observer_handle".into(),
            Value::String(self.observer_did.clone()),
        );
        rec.insert(
            "perceived_frame_usi".into(),
            Value::String(self.perceived_frame_usi.clone()),
        );
        rec.insert(
            "is_hyper_vigilant".into(),
            Value::Bool(self.is_hyper_vigilant()),
        );
        rec.insert(
            "needs_grounding".into(),
            Value::Bool(self.needs_grounding()),
        );
        rec.insert("affective_state".into(), self.affective_state.to_value());
        rec.insert(
            "beliefs".into(),
            Value::List(
                self.epistemic_beliefs
                    .iter()
                    .map(DoxasticBelief::to_value)
                    .collect(),
            ),
        );
        rec.insert(
            "shared_anchor_var".into(),
            Value::F64(self.shared_anchor_var),
        );
        rec.insert("local_chart_var".into(), Value::F64(self.local_chart_var));
        Value::Record(rec)
    }

    /// Converts to value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert(
            "observer_did".into(),
            Value::String(self.observer_did.clone()),
        );
        if let Some(cohort) = &self.semantic_cohort {
            rec.insert("semantic_cohort".into(), Value::String(cohort.clone()));
        }
        rec.insert("affective_state".into(), self.affective_state.to_value());
        rec.insert(
            "perceived_frame_usi".into(),
            Value::String(self.perceived_frame_usi.clone()),
        );
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
        // Severe threat (-0.9 < -0.8) triggers hyper-vigilance
        let threat = AffectiveStatus {
            safety_threat_index: -0.9,
            ..Default::default()
        };
        assert!(threat.is_hyper_vigilant());

        // High arousal (0.95 > 0.9) triggers hyper-vigilance
        let high_arousal = AffectiveStatus {
            arousal_level: 0.95,
            ..Default::default()
        };
        assert!(high_arousal.is_hyper_vigilant());

        // Deep safety (+0.9) with calm arousal must NOT be flagged as hyper-vigilant
        let safe = AffectiveStatus {
            safety_threat_index: 0.9,
            ..Default::default()
        };
        assert!(!safe.is_hyper_vigilant());
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
        obs.affective_state.safety_threat_index = -0.9;
        assert!(obs.is_hyper_vigilant());
        assert!(obs.needs_grounding());
    }

    #[test]
    fn needs_grounding_dissociation() {
        let mut obs = ObserverFiber::new("did:q42:person:test", "frame");
        obs.affective_state.dissociation_index = 0.8;
        assert!(obs.needs_grounding());
    }

    #[test]
    fn correlated_anchor_variance_cancels_for_shared_anchor() {
        // Two observers inside the same deep-space habitat (anchor 42):
        // Coarse habitat position relative to BCRS has huge variance (1e12 m^2 -> 1000 km std),
        // while each observer's local chart variance inside the room is 1e-6 m^2 (1 mm std).
        let obs_a = ObserverFiber::new("did:q42:obs:a", "urn:omni:v1:phenomenology:a")
            .with_correlated_uncertainty(42, 1.0e12, 1.0e-6);
        let obs_b = ObserverFiber::new("did:q42:obs:b", "urn:omni:v1:phenomenology:b")
            .with_correlated_uncertainty(42, 1.0e12, 1.0e-6);
        let obs_unshared = ObserverFiber::new("did:q42:obs:c", "urn:omni:v1:phenomenology:c")
            .with_correlated_uncertainty(99, 1.0e12, 1.0e-6);

        let rel_var_shared = obs_a.relative_displacement_variance(&obs_b);
        assert!(
            (rel_var_shared - 2.0e-6).abs() < 1e-12,
            "shared anchor variance must cancel: got {}",
            rel_var_shared
        );
        assert!((obs_a.relative_displacement_std(&obs_b) - (2.0e-6_f64).sqrt()).abs() < 1e-9);

        let rel_var_unshared = obs_a.relative_displacement_variance(&obs_unshared);
        assert!(rel_var_unshared > 1.9e12);
    }

    #[test]
    fn presheaf_overlap_compatibility_and_epistemic_claim_projection() {
        let mut obs_a = ObserverFiber::new("did:q42:obs:a", "urn:omni:v1:phenomenology:a")
            .with_correlated_uncertainty(7, 100.0, 0.04);
        obs_a.epistemic_beliefs.push(DoxasticBelief {
            proposition_iri: "urn:qualia:prop:door-locked".into(),
            mu: 0.85,
            lambda: 0.05,
        });

        let mut obs_b = ObserverFiber::new("did:q42:obs:b", "urn:omni:v1:phenomenology:b")
            .with_correlated_uncertainty(7, 100.0, 0.04);
        obs_b.epistemic_beliefs.push(DoxasticBelief {
            proposition_iri: "urn:qualia:prop:door-locked".into(),
            mu: 0.80,
            lambda: 0.08,
        });

        let merged = obs_a.merge_presheaf_compatible(&obs_b, 0.10).unwrap();
        assert_eq!(merged.epistemic_beliefs.len(), 1);
        assert!((merged.local_chart_var - 0.02).abs() < 1e-9);

        // Contradictory belief on overlap must fail closed
        let mut obs_conflict = obs_b.clone();
        obs_conflict.epistemic_beliefs[0].mu = 0.10;
        obs_conflict.epistemic_beliefs[0].lambda = 0.85;
        assert!(obs_a.merge_presheaf_compatible(&obs_conflict, 0.10).is_err());

        // Verify ClaimOpinion plane projection
        let claim = obs_a.to_epistemic_claim_record();
        match claim {
            Value::Record(r) => {
                assert_eq!(
                    r.get("plane"),
                    Some(&Value::String("ClaimOpinion".into()))
                );
                assert_eq!(
                    r.get("observer_handle"),
                    Some(&Value::String("did:q42:obs:a".into()))
                );
            }
            _ => panic!("expected ClaimOpinion Record"),
        }
    }

    #[test]
    fn observer_to_value() {
        let obs = ObserverFiber::new("did:q42:person:alice", "urn:omni:v1:phenomenology:alice");
        let v = obs.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(
                    r.get("observer_did"),
                    Some(&Value::String("did:q42:person:alice".into()))
                );
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
