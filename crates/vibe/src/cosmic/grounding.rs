//! Grounding status and granular collapse operator (OCS §10, Packet OCS-13).
//!
//! Reference: OCS Specification v2.2.0 §10 & Playbook v4 Packet OCS-13.

use crate::value::Value;
use std::collections::BTreeMap;

/// Epistemic grounding classification (OCS §10.1).
#[derive(Debug, Clone, PartialEq)]
pub enum GroundingStatus {
    /// Explicitly anchored to a physical, archaeological, geological, or astronomical artifact.
    EmpiricallyAnchored {
        anchor_iri: String,
        confidence: f32,
        stratum_or_epoch: Option<String>,
    },
    /// Historically or geographically plausible interpolation (unverified but consistent).
    PlausibleInterpolation,
    /// Known artistic, literary, or narrative embellishment.
    NarrativeFiction,
    /// Counterfactual element violating physical law or verified historical record.
    CounterfactualMythos,
}

impl GroundingStatus {
    /// `str` form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EmpiricallyAnchored { .. } => "EmpiricallyAnchored",
            Self::PlausibleInterpolation => "PlausibleInterpolation",
            Self::NarrativeFiction => "NarrativeFiction",
            Self::CounterfactualMythos => "CounterfactualMythos",
        }
    }

    /// Quantum context coordinate $q$ for `Tensor10D` integration (Packet OCS-13):
    /// - `EmpiricallyAnchored` $\implies q \in [0.0, 1.0)$ ($q = 0.0$ at full confidence $1.0$).
    /// - Uncollapsed narrative / counterfactual realms $\implies q \ge 1.0$.
    pub fn quantum_context_q(&self) -> f32 {
        match self {
            Self::EmpiricallyAnchored { confidence, .. } => {
                (1.0 - confidence.clamp(0.0, 1.0)).min(0.99)
            }
            Self::PlausibleInterpolation => 1.0,
            Self::NarrativeFiction => 2.0,
            Self::CounterfactualMythos => 3.0,
        }
    }

    /// Whether this element should be collapsed onto physical spacetime ($q < 1.0$).
    pub fn is_collapsible(&self) -> bool {
        matches!(self, Self::EmpiricallyAnchored { .. }) && self.quantum_context_q() < 1.0
    }

    /// Converts to value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("status".into(), Value::String(self.as_str().into()));
        rec.insert(
            "quantum_context_q".into(),
            Value::F64(self.quantum_context_q() as f64),
        );
        if let Self::EmpiricallyAnchored {
            anchor_iri,
            confidence,
            stratum_or_epoch,
        } = self
        {
            rec.insert("anchor_iri".into(), Value::String(anchor_iri.clone()));
            rec.insert("confidence".into(), Value::F64(*confidence as f64));
            if let Some(s) = stratum_or_epoch {
                rec.insert("stratum_or_epoch".into(), Value::String(s.clone()));
            }
        }
        Value::Record(rec)
    }
}

/// A narrative entity with grounding classification (OCS §10).
#[derive(Debug, Clone)]
pub struct NarrativeEntity {
    pub name: String,
    pub realm_usri: String,
    pub grounding: GroundingStatus,
}

impl NarrativeEntity {
    /// Create a collapsed (empirically anchored) entity.
    pub fn anchored(name: &str, realm: &str, anchor_iri: &str, confidence: f32) -> Self {
        Self {
            name: name.into(),
            realm_usri: realm.into(),
            grounding: GroundingStatus::EmpiricallyAnchored {
                anchor_iri: anchor_iri.into(),
                confidence,
                stratum_or_epoch: None,
            },
        }
    }

    /// Create a counterfactual/mythological entity.
    pub fn mythos(name: &str, realm: &str) -> Self {
        Self {
            name: name.into(),
            realm_usri: realm.into(),
            grounding: GroundingStatus::CounterfactualMythos,
        }
    }

    /// Converts to value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("realm_usri".into(), Value::String(self.realm_usri.clone()));
        rec.insert("grounding".into(), self.grounding.to_value());
        Value::Record(rec)
    }
}

/// Granular collapse operator Ĉ (OCS §10.2, Packet OCS-13).
///
/// Given a set of narrative entities, returns only those that are
/// empirically anchored ($q < 1.0$, collapsible onto physical spacetime),
/// leaving counterfactual or fictional entities uncollapsed in their narrative realm.
pub fn collapse_entities(entities: &[NarrativeEntity]) -> Vec<&NarrativeEntity> {
    entities
        .iter()
        .filter(|e| e.grounding.is_collapsible())
        .collect()
}

/// Normalized log-sum-exp distance metric subtracting $\ln(N)$ so $d(x, x) = 0.0$ (Packet OCS-13).
///
/// $$d_{\text{LSE}}(\mathbf{x}, \mathbf{y}) = \ln\!\left(\sum_{i=1}^{N} e^{|x_i - y_i|}\right) - \ln(N)$$
pub fn normalized_lse_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return f64::INFINITY;
    }
    let n = a.len() as f64;
    let max_diff = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0_f64, f64::max);
    let sum_exp: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| ((x - y).abs() - max_diff).exp())
        .sum();
    (max_diff + sum_exp.ln() - n.ln()).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empirically_anchored_is_collapsible() {
        let status = GroundingStatus::EmpiricallyAnchored {
            anchor_iri: "urn:omni:v1:physical:earth:hisarlik".into(),
            confidence: 0.95,
            stratum_or_epoch: Some("VIIa".into()),
        };
        assert!(status.is_collapsible());
        assert!(status.quantum_context_q() < 1.0);
    }

    #[test]
    fn counterfactual_not_collapsible() {
        assert!(!GroundingStatus::CounterfactualMythos.is_collapsible());
        assert!(GroundingStatus::CounterfactualMythos.quantum_context_q() >= 1.0);
    }

    #[test]
    fn narrative_fiction_not_collapsible() {
        assert!(!GroundingStatus::NarrativeFiction.is_collapsible());
        assert!(GroundingStatus::NarrativeFiction.quantum_context_q() >= 1.0);
    }

    #[test]
    fn collapse_troy_and_athena() {
        let troy = NarrativeEntity::anchored(
            "ancient_troy_citadel",
            "urn:omni:v1:narrative:homer:iliad",
            "urn:omni:v1:physical:earth:hisarlik",
            0.95,
        );
        let athena = NarrativeEntity::mythos("goddess_athena", "urn:omni:v1:narrative:homer:iliad");
        let entities = vec![troy.clone(), athena.clone()];
        let collapsed = collapse_entities(&entities);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].name, "ancient_troy_citadel");
        // Verify uncollapsed mythos entity remains in narrative realm with q >= 1.0
        assert_eq!(athena.realm_usri, "urn:omni:v1:narrative:homer:iliad");
        assert!(athena.grounding.quantum_context_q() >= 1.0);
    }

    #[test]
    fn normalized_lse_distance_zero_on_identical_vectors() {
        let x = [12.5, -4.2, 88.0];
        assert!(normalized_lse_distance(&x, &x).abs() < 1e-12);
        let y = [13.5, -4.2, 88.0];
        assert!(normalized_lse_distance(&x, &y) > 0.0);
    }

    #[test]
    fn grounding_to_value() {
        let status = GroundingStatus::EmpiricallyAnchored {
            anchor_iri: "test".into(),
            confidence: 0.8,
            stratum_or_epoch: None,
        };
        let v = status.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(
                    r.get("status"),
                    Some(&Value::String("EmpiricallyAnchored".into()))
                );
                if let Some(Value::F64(c)) = r.get("confidence") {
                    assert!((*c - 0.8).abs() < 1e-6, "confidence mismatch: {}", c);
                } else {
                    panic!("expected F64 confidence");
                }
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn narrative_entity_to_value() {
        let e = NarrativeEntity::anchored("test", "realm", "anchor", 0.9);
        let v = e.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("test".into())));
                assert!(r.contains_key("grounding"));
            }
            _ => panic!("expected Record"),
        }
    }
}
