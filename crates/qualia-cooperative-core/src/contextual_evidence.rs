//! Evidence detail for contextual assertions about subjects such as taxa, specimens and parts.
//!
//! A [`ContextualAssertion`](crate::contextual_assertion::ContextualAssertion) records who asserted
//! what, from which root source, with what lifecycle and verification status. [`EvidenceDetail`]
//! holds the content of a measured or described fact: property, value and unit, uncertainty,
//! method, observation/validity time, biological scope, the record or media region it came from,
//! and the basis on which it is held (measured, described, derived, model assumption, artistic
//! choice or unknown). [`EvidencedAssertion`] pairs the two without changing the existing
//! assertion type. Competing values for one property coexist as separate assertions.

use serde::{Deserialize, Serialize};

use crate::record::{q_hash_str, NQuin};

/// Basis on which an assertion's value is held. Keeps model assumptions and artistic choices
/// distinguishable from source evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimBasis {
    /// Measured by a stated method.
    Measured,
    /// Descriptive statement from a source (treatment, flora, description).
    Described,
    /// Derived by inference or calculation from other assertions.
    Derived,
    /// Model or simulation assumption, not evidence about the subject.
    ModelAssumption,
    /// Authored artistic or illustrative choice.
    ArtisticChoice,
    /// The value is unknown; the assertion records the gap.
    Unknown,
}

impl ClaimBasis {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Described => "described",
            Self::Derived => "derived",
            Self::ModelAssumption => "model_assumption",
            Self::ArtisticChoice => "artistic_choice",
            Self::Unknown => "unknown",
        }
    }
}

/// Asserted value. Numeric values carry a unit IRI (for example a QUDT unit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EvidenceValue {
    Quantity { value: f64, unit_iri: String },
    Range { min: f64, max: f64, unit_iri: String },
    Text { text: String, language: Option<String> },
    Iri { iri: String },
}

/// Statistical uncertainty or sample description for a value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Uncertainty {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_deviation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<(f64, f64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_size: Option<u32>,
}

/// What the assertion is about biologically. Type-level facts leave `individual_iri` empty;
/// observations of particular organisms set it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BiologicalScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxon_concept_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_stage_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sex_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual_iri: Option<String>,
}

/// Content of a measured or described fact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceDetail {
    pub property_iri: String,
    pub basis: ClaimBasis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<EvidenceValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Uncertainty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_iri: Option<String>,
    /// ISO 8601 instant or interval of observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    /// ISO 8601 interval during which the assertion is held valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_during: Option<String>,
    #[serde(default)]
    pub scope: BiologicalScope,
    /// Identifier of the record within the root source (for example a trait observation id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_record_id: Option<String>,
    /// Region of a media item supporting the assertion (for example an Audiovisual Core ROI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_region: Option<String>,
}

const INLINE_TAG_DECIMAL: u64 = 0b010 << 60;
const PAYLOAD_60: u64 = 0x0FFF_FFFF_FFFF_FFFF;

pub const P_PROPERTY: &str = "https://ns.webizen.org/evidence#property";
pub const P_BASIS: &str = "https://ns.webizen.org/evidence#basis";
pub const P_VALUE: &str = "https://ns.webizen.org/evidence#value";
pub const P_VALUE_MIN: &str = "https://ns.webizen.org/evidence#valueMin";
pub const P_VALUE_MAX: &str = "https://ns.webizen.org/evidence#valueMax";
pub const P_UNIT: &str = "https://ns.webizen.org/evidence#unit";
pub const P_METHOD: &str = "https://ns.webizen.org/evidence#method";
pub const P_TAXON: &str = "https://ns.webizen.org/evidence#taxonConcept";
pub const P_PART: &str = "https://ns.webizen.org/evidence#part";
pub const P_LIFE_STAGE: &str = "https://ns.webizen.org/evidence#lifeStage";
pub const P_SEX: &str = "https://ns.webizen.org/evidence#sex";
pub const P_POPULATION: &str = "https://ns.webizen.org/evidence#population";
pub const P_INDIVIDUAL: &str = "https://ns.webizen.org/evidence#individual";
pub const P_SOURCE_RECORD: &str = "https://ns.webizen.org/evidence#sourceRecord";

/// Encodes a number as an inline `xsd:decimal` object (value x 10^6) when it is finite,
/// non-negative and fits 60 bits; otherwise hashes its canonical lexical form.
pub fn encode_decimal_object(v: f64) -> u64 {
    let scaled = v * 1_000_000.0;
    if v.is_finite() && v >= 0.0 && scaled <= PAYLOAD_60 as f64 {
        INLINE_TAG_DECIMAL | (scaled.round() as u64 & PAYLOAD_60)
    } else {
        q_hash_str(&format!("{v:?}^^xsd:decimal"))
    }
}

impl EvidenceDetail {
    /// Validates internal consistency. Numeric values must be finite, ranges ordered, and a
    /// `Measured` assertion must name its method.
    pub fn validate(&self) -> Result<(), String> {
        if self.property_iri.trim().is_empty() {
            return Err("evidence property_iri is empty".into());
        }
        match &self.value {
            Some(EvidenceValue::Quantity { value, unit_iri }) => {
                if !value.is_finite() {
                    return Err("evidence quantity is not finite".into());
                }
                if unit_iri.trim().is_empty() {
                    return Err("evidence quantity requires unit_iri".into());
                }
            }
            Some(EvidenceValue::Range { min, max, unit_iri }) => {
                if !min.is_finite() || !max.is_finite() || min > max {
                    return Err("evidence range must be finite with min <= max".into());
                }
                if unit_iri.trim().is_empty() {
                    return Err("evidence range requires unit_iri".into());
                }
            }
            Some(EvidenceValue::Text { text, .. }) if text.trim().is_empty() => {
                return Err("evidence text is empty".into());
            }
            Some(EvidenceValue::Iri { iri }) if iri.trim().is_empty() => {
                return Err("evidence iri is empty".into());
            }
            _ => {}
        }
        if self.basis == ClaimBasis::Unknown && self.value.is_some() {
            return Err("an unknown-basis assertion must not carry a value".into());
        }
        if self.basis == ClaimBasis::Measured && self.method_iri.is_none() {
            return Err("a measured assertion requires method_iri".into());
        }
        if let Some(u) = &self.uncertainty {
            if let Some((lo, hi)) = u.interval {
                if !lo.is_finite() || !hi.is_finite() || lo > hi {
                    return Err("uncertainty interval must be finite with lo <= hi".into());
                }
            }
            if let Some(c) = u.interval_confidence {
                if !(0.0..=1.0).contains(&c) {
                    return Err("interval_confidence must be in 0.0..=1.0".into());
                }
            }
        }
        Ok(())
    }

    /// Appends evidence quins for subject hash `s` into `out[start..]`; returns the new length.
    pub fn compile_to_quins(&self, s: u64, ctx: u64, meta: u64, out: &mut [NQuin], start: usize) -> usize {
        let mut n = start;
        let mut push = |p: &str, o: u64, n: &mut usize| {
            if *n < out.len() {
                out[*n] = NQuin::new(s, q_hash_str(p), o, ctx, meta);
                *n += 1;
            }
        };
        push(P_PROPERTY, q_hash_str(&self.property_iri), &mut n);
        push(P_BASIS, q_hash_str(self.basis.as_str()), &mut n);
        match &self.value {
            Some(EvidenceValue::Quantity { value, unit_iri }) => {
                push(P_VALUE, encode_decimal_object(*value), &mut n);
                push(P_UNIT, q_hash_str(unit_iri), &mut n);
            }
            Some(EvidenceValue::Range { min, max, unit_iri }) => {
                push(P_VALUE_MIN, encode_decimal_object(*min), &mut n);
                push(P_VALUE_MAX, encode_decimal_object(*max), &mut n);
                push(P_UNIT, q_hash_str(unit_iri), &mut n);
            }
            Some(EvidenceValue::Text { text, .. }) => push(P_VALUE, q_hash_str(text), &mut n),
            Some(EvidenceValue::Iri { iri }) => push(P_VALUE, q_hash_str(iri), &mut n),
            None => {}
        }
        let optional = [
            (P_METHOD, &self.method_iri),
            (P_TAXON, &self.scope.taxon_concept_iri),
            (P_PART, &self.scope.part_iri),
            (P_LIFE_STAGE, &self.scope.life_stage_iri),
            (P_SEX, &self.scope.sex_iri),
            (P_POPULATION, &self.scope.population_iri),
            (P_INDIVIDUAL, &self.scope.individual_iri),
            (P_SOURCE_RECORD, &self.source_record_id),
        ];
        for (p, v) in optional {
            if let Some(v) = v {
                push(p, q_hash_str(v), &mut n);
            }
        }
        n
    }
}

/// A contextual assertion together with the content of the fact it asserts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidencedAssertion {
    pub assertion: crate::contextual_assertion::ContextualAssertion,
    pub evidence: EvidenceDetail,
}

impl EvidencedAssertion {
    /// Validates the evidence and its consistency with the assertion's statement type.
    pub fn validate(&self) -> Result<(), String> {
        use crate::contextual_assertion::StatementType;
        let a = &self.assertion;
        if a.id.trim().is_empty() || a.subject_uri.trim().is_empty() {
            return Err("assertion id and subject_uri are required".into());
        }
        self.evidence.validate()?;
        match self.evidence.basis {
            ClaimBasis::Measured | ClaimBasis::Described if a.root_source_id.trim().is_empty() => {
                Err("measured or described evidence requires root_source_id".into())
            }
            ClaimBasis::ModelAssumption | ClaimBasis::ArtisticChoice
                if matches!(
                    a.statement_type,
                    StatementType::Observation | StatementType::AdjudicatedFinding
                ) =>
            {
                Err("a model assumption or artistic choice cannot be an observation or finding".into())
            }
            ClaimBasis::Derived
                if a.statement_type != StatementType::Inference
                    && a.derived_from_assertion_ids.is_empty() =>
            {
                Err("derived evidence must be an inference or name derived_from_assertion_ids".into())
            }
            _ => Ok(()),
        }
    }

    /// Writes the assertion quins followed by the evidence quins; returns the count written.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        let a = &self.assertion;
        let n = a.compile_to_quins(out);
        let s = q_hash_str(&a.id);
        let ctx = q_hash_str(&a.scope_domain);
        let meta = a.sensitivity.to_metadata_mask() | (a.asserted_at_unix as u64);
        self.evidence.compile_to_quins(s, ctx, meta, out, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contextual_assertion::{
        AssertionStatus, ContextualAssertion, StatementType, VerificationStatus,
    };
    use crate::record::SensitivityClass;

    fn assertion(st: StatementType, root: &str) -> ContextualAssertion {
        ContextualAssertion {
            id: "ea-1".into(),
            subject_uri: "https://biodiversity.org.au/afd/taxa/Wallabia_bicolor".into(),
            statement_type: st,
            assertion_status: AssertionStatus::Active,
            verification_status: VerificationStatus::Proposed,
            claim_summary: "head-body length".into(),
            author_did: "did:example:curator".into(),
            root_source_id: root.into(),
            derived_from_assertion_ids: Vec::new(),
            scope_domain: "species:wallabia-bicolor".into(),
            sensitivity: SensitivityClass::Public,
            asserted_at_unix: 1_700_000_000,
            expires_at_unix: None,
            superseded_by_id: None,
            adjudicated_by_human_did: None,
        }
    }

    fn measured() -> EvidenceDetail {
        EvidenceDetail {
            property_iri: "http://purl.obolibrary.org/obo/VT_0002257".into(),
            basis: ClaimBasis::Measured,
            value: Some(EvidenceValue::Range {
                min: 0.665,
                max: 0.847,
                unit_iri: "http://qudt.org/vocab/unit/M".into(),
            }),
            uncertainty: None,
            method_iri: Some("urn:example:method:caliper".into()),
            observed_at: None,
            valid_during: None,
            scope: BiologicalScope {
                life_stage_iri: Some("http://purl.obolibrary.org/obo/UBERON_0000113".into()),
                ..Default::default()
            },
            source_record_id: Some("record-42".into()),
            media_region: None,
        }
    }

    #[test]
    fn measured_evidence_validates_and_compiles() {
        let ea = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, "src:example-dataset"),
            evidence: measured(),
        };
        ea.validate().unwrap();
        let mut out = [NQuin::default(); 32];
        let n = ea.compile_to_quins(&mut out);
        // 5 assertion + property, basis, min, max, unit, method, life stage, source record.
        assert_eq!(n, 13);
        let min = out[7];
        assert_eq!(min.predicate, q_hash_str(P_VALUE_MIN));
        assert_eq!(min.object, INLINE_TAG_DECIMAL | 665_000);
        assert!(out[..n].iter().all(|q| q.parity == q.subject ^ q.predicate ^ q.object ^ q.context));
    }

    #[test]
    fn evidence_validation_fails_closed() {
        let mut e = measured();
        e.method_iri = None;
        assert!(e.validate().unwrap_err().contains("method"));

        let mut e = measured();
        e.basis = ClaimBasis::Unknown;
        e.method_iri = None;
        assert!(e.validate().unwrap_err().contains("unknown"));

        let mut e = measured();
        e.value = Some(EvidenceValue::Range { min: 2.0, max: 1.0, unit_iri: "u".into() });
        assert!(e.validate().is_err());

        let mut e = measured();
        e.value = Some(EvidenceValue::Quantity { value: 1.0, unit_iri: String::new() });
        assert!(e.validate().unwrap_err().contains("unit"));
    }

    #[test]
    fn assertion_consistency_is_enforced() {
        let no_source = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, ""),
            evidence: measured(),
        };
        assert!(no_source.validate().unwrap_err().contains("root_source_id"));

        let mut assumption = measured();
        assumption.basis = ClaimBasis::ModelAssumption;
        let bad = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, "src"),
            evidence: assumption.clone(),
        };
        assert!(bad.validate().unwrap_err().contains("model assumption"));
        let ok = EvidencedAssertion {
            assertion: assertion(StatementType::Attestation, "src"),
            evidence: assumption,
        };
        ok.validate().unwrap();

        let mut derived = measured();
        derived.basis = ClaimBasis::Derived;
        let bad = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, "src"),
            evidence: derived,
        };
        assert!(bad.validate().unwrap_err().contains("derived"));
    }

    #[test]
    fn negative_values_fall_back_to_lexical_hash() {
        let o = encode_decimal_object(-0.2);
        assert_eq!(o, q_hash_str("-0.2^^xsd:decimal"));
        assert_eq!(encode_decimal_object(0.95), INLINE_TAG_DECIMAL | 950_000);
    }
}
