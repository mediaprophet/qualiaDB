//! Epistemic assessment — mode, reality category, context.

use std::collections::BTreeMap;

/// Epistemic mode — how reality is being assessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicMode {
    Empirical,
    Theoretical,
    Speculative,
    Fictional,
    Hypothetical,
}

/// Reality category — classification of the assessed content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealityCategory {
    Factual,
    Fictional,
    Blended,
    Uncertain,
    Deceptive,
}

impl RealityCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Factual => "factual",
            Self::Fictional => "fictional",
            Self::Blended => "blended",
            Self::Uncertain => "uncertain",
            Self::Deceptive => "deceptive",
        }
    }
}

/// An epistemic assessment of a piece of content.
#[derive(Debug, Clone)]
pub struct EpistemicAssessment {
    pub id: String,
    pub content_ref: String,
    pub mode: EpistemicMode,
    pub reality_category: RealityCategory,
    pub spatio_temporal_context: Option<String>,
    pub social_context: Option<String>,
    pub grounding_score: f64,
    pub metadata: BTreeMap<String, String>,
}

impl EpistemicAssessment {
    pub fn new(id: &str, content_ref: &str) -> Self {
        Self {
            id: id.to_string(),
            content_ref: content_ref.to_string(),
            mode: EpistemicMode::Empirical,
            reality_category: RealityCategory::Uncertain,
            spatio_temporal_context: None,
            social_context: None,
            grounding_score: 0.0,
            metadata: BTreeMap::new(),
        }
    }

    pub fn set_epistemic_mode(&mut self, mode: EpistemicMode) {
        self.mode = mode;
    }

    pub fn set_reality_category(&mut self, category: RealityCategory) {
        self.reality_category = category;
    }

    pub fn set_spatio_temporal_context(&mut self, context: &str) {
        self.spatio_temporal_context = Some(context.to_string());
    }

    pub fn set_social_context(&mut self, context: &str) {
        self.social_context = Some(context.to_string());
    }

    /// Classify reality based on content markers.
    pub fn classify_reality(content: &str) -> RealityCategory {
        let lower = content.to_lowercase();
        let fiction_markers = [
            "novel",
            "story",
            "fiction",
            "narrative",
            "character",
            "plot",
        ];
        let factual_markers = [
            "study", "research", "data", "evidence", "measured", "observed",
        ];
        let deceptive_markers = ["allegedly", "supposedly", "claim", "unverified"];

        let fiction_count = fiction_markers
            .iter()
            .filter(|m| lower.contains(*m))
            .count();
        let factual_count = factual_markers
            .iter()
            .filter(|m| lower.contains(*m))
            .count();
        let deceptive_count = deceptive_markers
            .iter()
            .filter(|m| lower.contains(*m))
            .count();

        if deceptive_count > 0 && factual_count == 0 {
            RealityCategory::Deceptive
        } else if fiction_count > 0 && factual_count > 0 {
            RealityCategory::Blended
        } else if fiction_count > 0 {
            RealityCategory::Fictional
        } else if factual_count > 0 {
            RealityCategory::Factual
        } else {
            RealityCategory::Uncertain
        }
    }

    /// Detect blended content (mix of factual and fictional markers).
    pub fn detect_blended_content(content: &str) -> bool {
        Self::classify_reality(content) == RealityCategory::Blended
    }

    /// Detect deceptive fiction (fiction presented as fact).
    pub fn detect_deceptive_fiction(content: &str, claimed_category: RealityCategory) -> bool {
        let actual = Self::classify_reality(content);
        claimed_category == RealityCategory::Factual && actual == RealityCategory::Fictional
    }

    /// Trace fiction elements back to their reality basis.
    pub fn trace_fiction_to_reality(
        fiction_content: &str,
        reality_corpus: &[String],
    ) -> Vec<String> {
        let fiction_words: Vec<&str> = fiction_content.split_whitespace().collect();
        let mut traces = Vec::new();
        for corpus_item in reality_corpus {
            let corpus_lower = corpus_item.to_lowercase();
            let overlap = fiction_words
                .iter()
                .filter(|w| corpus_lower.contains(&w.to_lowercase()))
                .count();
            if overlap > 3 {
                traces.push(corpus_item.clone());
            }
        }
        traces
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assessment_creation() {
        let a = EpistemicAssessment::new("a1", "content_ref_1");
        assert_eq!(a.mode, EpistemicMode::Empirical);
        assert_eq!(a.reality_category, RealityCategory::Uncertain);
    }

    #[test]
    fn classify_factual() {
        let cat = EpistemicAssessment::classify_reality(
            "The study measured evidence from observed data.",
        );
        assert_eq!(cat, RealityCategory::Factual);
    }

    #[test]
    fn classify_fictional() {
        let cat = EpistemicAssessment::classify_reality(
            "The novel tells a story about a character in a plot.",
        );
        assert_eq!(cat, RealityCategory::Fictional);
    }

    #[test]
    fn classify_blended() {
        let cat = EpistemicAssessment::classify_reality(
            "The study research measured data about a novel story character.",
        );
        assert_eq!(cat, RealityCategory::Blended);
    }

    #[test]
    fn classify_deceptive() {
        let cat =
            EpistemicAssessment::classify_reality("The allegedly true events supposedly happened.");
        assert_eq!(cat, RealityCategory::Deceptive);
    }

    #[test]
    fn detect_deceptive_fiction() {
        let content = "The novel tells a story about a character.";
        assert!(EpistemicAssessment::detect_deceptive_fiction(
            content,
            RealityCategory::Factual
        ));
    }

    #[test]
    fn trace_fiction_to_reality() {
        let fiction = "The climate study measured temperature data";
        let corpus = vec![
            "The climate study was conducted in 2024 with temperature data".to_string(),
            "Unrelated content about cooking".to_string(),
        ];
        let traces = EpistemicAssessment::trace_fiction_to_reality(fiction, &corpus);
        assert!(!traces.is_empty());
    }

    #[test]
    fn set_contexts() {
        let mut a = EpistemicAssessment::new("a1", "ref");
        a.set_spatio_temporal_context("2024, London");
        a.set_social_context("Academic community");
        assert!(a.spatio_temporal_context.is_some());
        assert!(a.social_context.is_some());
    }
}
