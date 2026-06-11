//! Context detection for LoRA adapter selection.
//!
//! Classifies a natural-language prompt (and optionally the NQuin
//! metadata vector) into one of the six `ContextType` domains using
//! weighted keyword scoring and bigram analysis.

use std::collections::HashMap;

// ─── ContextType ─────────────────────────────────────────────────────────────

/// Domain classification used to select the correct LoRA adapter.
///
/// The 4-bit encoding in `NQuin.metadata` bits 63–60 must stay stable;
/// do not reorder these variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ContextType {
    General    = 0x0,
    Medical    = 0x1,
    Legal      = 0x2,
    Chemical   = 0x3,
    Biological = 0x4,
    Technical  = 0x5,
}

impl ContextType {
    /// Decode from the 4-bit metadata field (bits 63–60).
    pub fn from_metadata_bits(bits: u8) -> Self {
        match bits & 0xF {
            0x1 => ContextType::Medical,
            0x2 => ContextType::Legal,
            0x3 => ContextType::Chemical,
            0x4 => ContextType::Biological,
            0x5 => ContextType::Technical,
            _   => ContextType::General,
        }
    }

    pub fn to_metadata_bits(self) -> u8 {
        self as u8
    }

    pub fn adapter_filename(self) -> &'static str {
        match self {
            ContextType::General    => "general_v1.lora",
            ContextType::Medical    => "medical_v1.lora",
            ContextType::Legal      => "legal_v1.lora",
            ContextType::Chemical   => "chemical_v1.lora",
            ContextType::Biological => "biological_v1.lora",
            ContextType::Technical  => "technical_v1.lora",
        }
    }

    pub fn all() -> &'static [ContextType] {
        &[
            ContextType::General,
            ContextType::Medical,
            ContextType::Legal,
            ContextType::Chemical,
            ContextType::Biological,
            ContextType::Technical,
        ]
    }
}

impl std::fmt::Display for ContextType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextType::General    => write!(f, "general"),
            ContextType::Medical    => write!(f, "medical"),
            ContextType::Legal      => write!(f, "legal"),
            ContextType::Chemical   => write!(f, "chemical"),
            ContextType::Biological => write!(f, "biological"),
            ContextType::Technical  => write!(f, "technical"),
        }
    }
}

// ─── NGramAnalyzer ────────────────────────────────────────────────────────────

struct NGramAnalyzer {
    bigram_weights: HashMap<(&'static str, &'static str), (ContextType, f32)>,
}

impl NGramAnalyzer {
    fn new() -> Self {
        let mut bigram_weights = HashMap::new();
        // Medical bigrams
        for pair in [
            ("patient", "diagnosis"), ("medical", "record"), ("clinical", "trial"),
            ("emergency", "room"), ("blood", "pressure"), ("heart", "rate"),
            ("drug", "dosage"), ("surgical", "procedure"),
        ] {
            bigram_weights.insert(pair, (ContextType::Medical, 0.85));
        }
        // Legal bigrams
        for pair in [
            ("legal", "contract"), ("court", "order"), ("intellectual", "property"),
            ("due", "diligence"), ("breach", "contract"), ("statute", "limitations"),
            ("case", "law"), ("legal", "counsel"),
        ] {
            bigram_weights.insert(pair, (ContextType::Legal, 0.85));
        }
        // Chemical bigrams
        for pair in [
            ("chemical", "reaction"), ("organic", "compound"), ("synthesis", "reaction"),
            ("chemical", "bond"), ("molecular", "weight"), ("oxidation", "reduction"),
            ("acid", "base"), ("reaction", "mechanism"),
        ] {
            bigram_weights.insert(pair, (ContextType::Chemical, 0.85));
        }
        // Biological bigrams
        for pair in [
            ("gene", "expression"), ("cell", "division"), ("protein", "synthesis"),
            ("natural", "selection"), ("dna", "replication"), ("immune", "system"),
            ("metabolic", "pathway"), ("stem", "cell"),
        ] {
            bigram_weights.insert(pair, (ContextType::Biological, 0.85));
        }
        // Technical bigrams
        for pair in [
            ("machine", "learning"), ("neural", "network"), ("api", "endpoint"),
            ("data", "structure"), ("time", "complexity"), ("memory", "management"),
            ("software", "architecture"), ("distributed", "system"),
        ] {
            bigram_weights.insert(pair, (ContextType::Technical, 0.85));
        }
        Self { bigram_weights }
    }

    fn score(&self, tokens: &[&str]) -> HashMap<ContextType, f32> {
        let mut scores: HashMap<ContextType, f32> = HashMap::new();
        for window in tokens.windows(2) {
            if let [a, b] = window {
                if let Some(&(ctx, w)) = self.bigram_weights.get(&(*a, *b)) {
                    *scores.entry(ctx).or_insert(0.0) += w;
                }
            }
        }
        scores
    }
}

// ─── ContextDetector ─────────────────────────────────────────────────────────

/// Classifies text into one of the `ContextType` domains.
///
/// Uses a two-phase approach:
/// 1. Unigram keyword scoring (fast, O(n) where n = token count).
/// 2. Bigram analysis for disambiguation of overlapping domains.
///
/// The resulting confidence is normalised to [0, 1]; a score below
/// `confidence_threshold` falls back to `ContextType::General`.
pub struct ContextDetector {
    /// Per-domain keyword → weight table.
    keyword_weights: HashMap<&'static str, Vec<(ContextType, f32)>>,
    ngrams: NGramAnalyzer,
    /// Minimum normalised confidence before the detector commits to a domain.
    pub confidence_threshold: f32,
}

impl ContextDetector {
    pub fn new() -> Self {
        let mut kw: HashMap<&'static str, Vec<(ContextType, f32)>> = HashMap::new();

        macro_rules! add {
            ($word:expr, $( ($ctx:expr, $w:expr) ),+) => {
                kw.entry($word).or_default().extend([$( ($ctx, $w) ),+]);
            };
        }

        // ── Medical ──
        for w in ["diagnosis", "symptom", "treatment", "medication", "patient",
                  "clinical", "prescription", "therapy", "disease", "anatomy",
                  "physiology", "pharmacology", "surgery", "emergency", "vaccine",
                  "oncology", "radiology", "pathology", "neurology", "cardiology",
                  "prognosis", "aetiology", "comorbidity", "triage", "dosage"] {
            add!(w, (ContextType::Medical, 0.9), (ContextType::Biological, 0.4));
        }

        // ── Legal ──
        for w in ["contract", "agreement", "liability", "legal", "court", "law",
                  "jurisdiction", "statute", "regulation", "compliance", "litigation",
                  "plaintiff", "defendant", "attorney", "verdict", "evidence",
                  "indemnity", "arbitration", "injunction", "tort", "precedent",
                  "affidavit", "deposition", "fiduciary", "subpoena"] {
            add!(w, (ContextType::Legal, 0.9));
        }

        // ── Chemical ──
        for w in ["molecule", "compound", "reaction", "chemical", "synthesis", "bond",
                  "catalyst", "reagent", "solvent", "stoichiometry", "organic",
                  "inorganic", "polymer", "spectroscopy", "titration", "oxidation",
                  "reduction", "isomer", "alkyl", "hydroxyl", "carbonyl", "ester",
                  "molar", "entropy", "enthalpy"] {
            add!(w, (ContextType::Chemical, 0.9), (ContextType::Technical, 0.2));
        }

        // ── Biological ──
        for w in ["cell", "gene", "protein", "dna", "rna", "organism", "species",
                  "evolution", "ecosystem", "metabolism", "genetics", "biology",
                  "immunology", "mitosis", "meiosis", "chromosome", "allele",
                  "phenotype", "genotype", "ribosome", "enzyme", "chlorophyll",
                  "photosynthesis", "fermentation", "microbiome"] {
            add!(w, (ContextType::Biological, 0.9), (ContextType::Chemical, 0.3));
        }

        // ── Technical ──
        for w in ["algorithm", "software", "hardware", "programming", "code", "system",
                  "database", "network", "protocol", "interface", "api", "encryption",
                  "authentication", "optimization", "latency", "throughput", "compiler",
                  "runtime", "kernel", "concurrency", "async", "cache", "shader",
                  "tensor", "gradient"] {
            add!(w, (ContextType::Technical, 0.9));
        }

        Self {
            keyword_weights: kw,
            ngrams: NGramAnalyzer::new(),
            confidence_threshold: 0.55,
        }
    }

    /// Classify `text` and return `(domain, confidence)`.
    ///
    /// Confidence of 0.0 means no relevant keywords found.
    pub fn analyze_text(&self, text: &str) -> (ContextType, f32) {
        let tokens: Vec<&str> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| {
                // &'static trick: we only look up keys that are &'static str.
                // For matching, we lowercase into a stack buffer via a scratch approach.
                // Here we accept the pointer-equality limitation and use a raw compare.
                s
            })
            .collect();

        // Lowercase comparison via a small scratch allocation (adapter init path, not hot).
        let lowered: Vec<String> = tokens.iter().map(|t| t.to_lowercase()).collect();
        let lower_refs: Vec<&str> = lowered.iter().map(|s| s.as_str()).collect();

        let mut scores: HashMap<ContextType, f32> = HashMap::new();

        // Unigram pass
        for tok in &lower_refs {
            if let Some(weights) = self.keyword_weights.get(tok) {
                for &(ctx, w) in weights {
                    *scores.entry(ctx).or_insert(0.0) += w;
                }
            }
        }

        // Bigram pass (half weight)
        for (ctx, w) in self.ngrams.score(&lower_refs) {
            *scores.entry(ctx).or_insert(0.0) += w * 0.5;
        }

        if scores.is_empty() {
            return (ContextType::General, 0.0);
        }

        let best = scores.iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&ctx, &s)| (ctx, s))
            .unwrap();

        let total: f32 = scores.values().sum();
        let confidence = if total > 0.0 { best.1 / total } else { 0.0 };

        if confidence < self.confidence_threshold {
            (ContextType::General, confidence)
        } else {
            (best.0, confidence)
        }
    }

    /// Combine text-derived and metadata-derived contexts.
    ///
    /// The higher-confidence signal wins; ties go to metadata.
    pub fn combine(
        &self,
        text_result: (ContextType, f32),
        meta_result: (ContextType, f32),
    ) -> (ContextType, f32) {
        if text_result.1 > meta_result.1 { text_result } else { meta_result }
    }
}

impl Default for ContextDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_medical_detection() {
        let det = ContextDetector::new();
        let (ctx, conf) = det.analyze_text("The patient requires medication for the diagnosis");
        assert_eq!(ctx, ContextType::Medical, "expected Medical, got {:?} (conf={conf:.2})", ctx);
        assert!(conf > 0.5);
    }

    #[test]
    fn test_legal_detection() {
        let det = ContextDetector::new();
        let (ctx, conf) = det.analyze_text("The plaintiff filed a contract litigation case");
        assert_eq!(ctx, ContextType::Legal, "expected Legal, got {:?} (conf={conf:.2})", ctx);
        assert!(conf > 0.5);
    }

    #[test]
    fn test_chemical_detection() {
        let det = ContextDetector::new();
        let (ctx, conf) = det.analyze_text("Organic synthesis of the catalyst compound via reaction");
        assert_eq!(ctx, ContextType::Chemical, "expected Chemical, got {:?} (conf={conf:.2})", ctx);
        assert!(conf > 0.4);
    }

    #[test]
    fn test_biological_detection() {
        let det = ContextDetector::new();
        let (ctx, conf) = det.analyze_text("Gene expression in the cell affects protein synthesis via RNA");
        assert_eq!(ctx, ContextType::Biological, "expected Biological, got {:?} (conf={conf:.2})", ctx);
        assert!(conf > 0.4);
    }

    #[test]
    fn test_technical_detection() {
        let det = ContextDetector::new();
        let (ctx, conf) = det.analyze_text("Optimizing the algorithm latency in the distributed system");
        assert_eq!(ctx, ContextType::Technical, "expected Technical, got {:?} (conf={conf:.2})", ctx);
        assert!(conf > 0.5);
    }

    #[test]
    fn test_general_fallback() {
        let det = ContextDetector::new();
        let (ctx, _conf) = det.analyze_text("Hello world");
        assert_eq!(ctx, ContextType::General);
    }

    #[test]
    fn test_roundtrip_metadata_bits() {
        for &ct in ContextType::all() {
            let bits = ct.to_metadata_bits();
            let decoded = ContextType::from_metadata_bits(bits);
            assert_eq!(decoded, ct, "roundtrip failed for {:?}", ct);
        }
    }
}
