//! Grounding status and granular collapse operator (OCS §10).
//!
//! Reference: OCS Specification v2.2.0 §10.

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
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EmpiricallyAnchored { .. } => "EmpiricallyAnchored",
            Self::PlausibleInterpolation => "PlausibleInterpolation",
            Self::NarrativeFiction => "NarrativeFiction",
            Self::CounterfactualMythos => "CounterfactualMythos",
        }
    }

    /// Whether this element should be collapsed onto physical spacetime.
    pub fn is_collapsible(&self) -> bool {
        matches!(self, Self::EmpiricallyAnchored { .. })
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("status".into(), Value::String(self.as_str().into()));
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

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("realm_usri".into(), Value::String(self.realm_usri.clone()));
        rec.insert("grounding".into(), self.grounding.to_value());
        Value::Record(rec)
    }
}

/// Granular collapse operator Ĉ (OCS §10.2).
///
/// Given a set of narrative entities, returns only those that are
/// empirically anchored (collapsible onto physical spacetime).
pub fn collapse_entities(entities: &[NarrativeEntity]) -> Vec<&NarrativeEntity> {
    entities
        .iter()
        .filter(|e| e.grounding.is_collapsible())
        .collect()
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
    }

    #[test]
    fn counterfactual_not_collapsible() {
        assert!(!GroundingStatus::CounterfactualMythos.is_collapsible());
    }

    #[test]
    fn narrative_fiction_not_collapsible() {
        assert!(!GroundingStatus::NarrativeFiction.is_collapsible());
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
        let entities = vec![troy.clone(), athena];
        let collapsed = collapse_entities(&entities);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].name, "ancient_troy_citadel");
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
                // f32 0.8 → f64 may not be exactly 0.8
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
