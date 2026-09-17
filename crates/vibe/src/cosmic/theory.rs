//! Theory packages, law nature, and assurance hierarchy (OCS §11).
//!
//! Extends the law_package module with OCS-specific theory metadata.
//!
//! Reference: OCS Specification v2.2.0 §11.

use crate::value::Value;
use std::collections::BTreeMap;

/// Law nature — the epistemic classification of a law (OCS §11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LawNature {
    /// Empirically verified or standard physical law (Maxwell, Einstein GR, Standard Model).
    Physical,
    /// Formal scientific hypothesis under active investigation (MOND, String Compactification).
    TheoreticalHypothesis,
    /// Mathematically consistent solution requiring hypothetical conditions (Alcubierre, Wormholes).
    HypotheticalExotic,
    /// Fictional lore for games, literature, or worldbuilding (Star Trek Warp).
    Fictional,
}

impl LawNature {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Physical => "Physical",
            Self::TheoreticalHypothesis => "TheoreticalHypothesis",
            Self::HypotheticalExotic => "HypotheticalExotic",
            Self::Fictional => "Fictional",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Physical" => Some(Self::Physical),
            "TheoreticalHypothesis" => Some(Self::TheoreticalHypothesis),
            "HypotheticalExotic" => Some(Self::HypotheticalExotic),
            "Fictional" => Some(Self::Fictional),
            _ => None,
        }
    }
}

/// Progressive assurance hierarchy (OCS §11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AssuranceLevel {
    /// Speculative / Fictional: unconstrained conjecture, zero empirical anchors.
    A0,
    /// Mathematically Formalized: self-consistent equations and action principles.
    A1,
    /// Computationally Verified: numerically stable, validated against CPU/GPU oracles.
    A2,
    /// Empirically Calibrated: validated against real datasets (Gaia, JWST, LIGO). χ² ≈ 1.0.
    A3,
    /// Empirical Metrology Standard: fundamental SI definitions and consensus constants.
    A4,
}

impl AssuranceLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A0 => "A0",
            Self::A1 => "A1",
            Self::A2 => "A2",
            Self::A3 => "A3",
            Self::A4 => "A4",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "A0" => Some(Self::A0),
            "A1" => Some(Self::A1),
            "A2" => Some(Self::A2),
            "A3" => Some(Self::A3),
            "A4" => Some(Self::A4),
            _ => None,
        }
    }
}

/// Theory lineage relationship (OCS §11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TheoryLineage {
    Refines,
    Supersedes,
    Approximates,
    CompetesWith,
}

impl TheoryLineage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Refines => "Refines",
            Self::Supersedes => "Supersedes",
            Self::Approximates => "Approximates",
            Self::CompetesWith => "CompetesWith",
        }
    }
}

/// Applicability envelope — the domain where a theory applies (OCS §11.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicabilityEnvelope {
    /// Minimum scale (m) where the theory is valid.
    pub min_scale_m: f64,
    /// Maximum scale (m) where the theory is valid.
    pub max_scale_m: f64,
    /// Whether the theory applies in strong-field gravity.
    pub strong_field: bool,
    /// Whether the theory applies in weak-field gravity.
    pub weak_field: bool,
}

/// A theory package — a signed, classified law with provenance (OCS §11.1).
#[derive(Debug, Clone)]
pub struct TheoryPackage {
    pub theory_id: String,
    pub name: String,
    pub author_did: String,
    pub nature: LawNature,
    pub assurance_level: AssuranceLevel,
    /// Evidential interval (μ: support, λ: refutation) in E_τ
    pub evidential_interval: (f32, f32),
    pub lineage: Option<(TheoryLineage, String)>,
    pub empirical_anchors: Vec<String>,
    pub residual_chi_squared: f64,
    pub applicability: ApplicabilityEnvelope,
    pub signature: Vec<u8>,
}

impl TheoryPackage {
    /// Create a new theory package.
    pub fn new(
        id: &str,
        name: &str,
        author_did: &str,
        nature: LawNature,
        assurance: AssuranceLevel,
    ) -> Self {
        Self {
            theory_id: id.into(),
            name: name.into(),
            author_did: author_did.into(),
            nature,
            assurance_level: assurance,
            evidential_interval: (0.0, 0.0),
            lineage: None,
            empirical_anchors: Vec::new(),
            residual_chi_squared: 0.0,
            applicability: ApplicabilityEnvelope {
                min_scale_m: 0.0,
                max_scale_m: f64::INFINITY,
                strong_field: false,
                weak_field: true,
            },
            signature: Vec::new(),
        }
    }

    /// Set evidential interval (μ, λ).
    pub fn with_evidence(mut self, mu: f32, lambda: f32) -> Self {
        self.evidential_interval = (mu, lambda);
        self
    }

    /// Add an empirical anchor IRI.
    pub fn with_anchor(mut self, iri: &str) -> Self {
        self.empirical_anchors.push(iri.into());
        self
    }

    /// Set residual chi-squared.
    pub fn with_chi_squared(mut self, chi2: f64) -> Self {
        self.residual_chi_squared = chi2;
        self
    }

    /// Set lineage.
    pub fn with_lineage(mut self, relation: TheoryLineage, target: &str) -> Self {
        self.lineage = Some((relation, target.into()));
        self
    }

    /// Whether this theory is empirically calibrated (A3+).
    pub fn is_empirically_calibrated(&self) -> bool {
        self.assurance_level >= AssuranceLevel::A3
    }

    /// Whether this theory is fictional.
    pub fn is_fictional(&self) -> bool {
        self.nature == LawNature::Fictional
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("theory_id".into(), Value::String(self.theory_id.clone()));
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("author_did".into(), Value::String(self.author_did.clone()));
        rec.insert("nature".into(), Value::String(self.nature.as_str().into()));
        rec.insert(
            "assurance_level".into(),
            Value::String(self.assurance_level.as_str().into()),
        );
        rec.insert("mu".into(), Value::F64(self.evidential_interval.0 as f64));
        rec.insert(
            "lambda".into(),
            Value::F64(self.evidential_interval.1 as f64),
        );
        rec.insert(
            "residual_chi_squared".into(),
            Value::F64(self.residual_chi_squared),
        );
        if let Some((rel, target)) = &self.lineage {
            rec.insert(
                "lineage_relation".into(),
                Value::String(rel.as_str().into()),
            );
            rec.insert("lineage_target".into(), Value::String(target.clone()));
        }
        if !self.empirical_anchors.is_empty() {
            rec.insert(
                "empirical_anchors".into(),
                Value::List(
                    self.empirical_anchors
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn law_nature_round_trip() {
        for n in [
            LawNature::Physical,
            LawNature::TheoreticalHypothesis,
            LawNature::Fictional,
        ] {
            assert_eq!(LawNature::from_str(n.as_str()), Some(n));
        }
    }

    #[test]
    fn assurance_ordering() {
        assert!(AssuranceLevel::A4 > AssuranceLevel::A3);
        assert!(AssuranceLevel::A3 > AssuranceLevel::A0);
    }

    #[test]
    fn theory_package_basic() {
        let t = TheoryPackage::new(
            "lcdm",
            "ΛCDM Standard",
            "did:q42:person:author",
            LawNature::Physical,
            AssuranceLevel::A3,
        )
        .with_evidence(0.85, 0.15)
        .with_chi_squared(1.04)
        .with_anchor("urn:omni:v1:physical:gaia-dr3");
        assert!(t.is_empirically_calibrated());
        assert!(!t.is_fictional());
        assert_eq!(t.evidential_interval, (0.85, 0.15));
    }

    #[test]
    fn theory_package_fictional() {
        let t = TheoryPackage::new(
            "warp",
            "Warp Drive",
            "did:q42:person:author",
            LawNature::Fictional,
            AssuranceLevel::A0,
        );
        assert!(t.is_fictional());
        assert!(!t.is_empirically_calibrated());
    }

    #[test]
    fn theory_package_lineage() {
        let t = TheoryPackage::new(
            "mond",
            "MOND",
            "did:q42:person:author",
            LawNature::TheoreticalHypothesis,
            AssuranceLevel::A1,
        )
        .with_lineage(TheoryLineage::CompetesWith, "lcdm");
        assert!(t.lineage.is_some());
        let (rel, target) = t.lineage.unwrap();
        assert_eq!(rel, TheoryLineage::CompetesWith);
        assert_eq!(target, "lcdm");
    }

    #[test]
    fn theory_to_value() {
        let t = TheoryPackage::new(
            "test",
            "Test Theory",
            "did:q42:person:x",
            LawNature::Physical,
            AssuranceLevel::A2,
        )
        .with_evidence(0.5, 0.3);
        let v = t.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("theory_id"), Some(&Value::String("test".into())));
                assert_eq!(r.get("nature"), Some(&Value::String("Physical".into())));
                assert_eq!(r.get("assurance_level"), Some(&Value::String("A2".into())));
                assert_eq!(r.get("mu"), Some(&Value::F64(0.5)));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn assurance_from_str() {
        assert_eq!(AssuranceLevel::from_str("A3"), Some(AssuranceLevel::A3));
        assert_eq!(AssuranceLevel::from_str("A0"), Some(AssuranceLevel::A0));
        assert_eq!(AssuranceLevel::from_str("invalid"), None);
    }
}
