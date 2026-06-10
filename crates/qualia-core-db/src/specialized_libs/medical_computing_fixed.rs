/// Clinical guidelines
#[derive(Debug, Clone)]
pub struct ClinicalGuideline {
    pub guideline_id: String,
    pub guideline_name: String,
    pub guideline_type: GuidelineType,
    pub recommendations: Vec<GuidelineRecommendation>,
}

/// Guideline recommendations
#[derive(Debug, Clone)]
pub struct GuidelineRecommendation {
    pub recommendation_id: String,
    pub recommendation_text: String,
    pub recommendation_type: RecommendationType,
    pub evidence_level: EvidenceLevel,
    pub strength: RecommendationStrength,
}
