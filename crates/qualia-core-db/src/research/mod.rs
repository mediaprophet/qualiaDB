//! Research / Investigation / Epistemics module.
//!
//! Build-new research workflow layer: research enquiries, corpus management,
//! dark-link inference, inference chains, investigations, hypothesis graphs,
//! epistemic assessment, perspective analysis, intentionality, fiction
//! classification, sentiment analysis, and ungrounded generation diagnosis.

pub mod assessment;
pub mod corpus;
pub mod dark_link;
pub mod dynamics;
pub mod enquiry;
pub mod hypothesis;
pub mod inference_chain;
pub mod intentionality;
pub mod investigation;
pub mod perspective;
pub mod sentiment;

pub use assessment::{EpistemicAssessment, EpistemicMode, RealityCategory};
pub use corpus::{Corpus, CorpusConfidence, CorpusItem};
pub use dark_link::{
    detect_concealment_patterns, detect_provenance_gaps, DarkLink, DarkLinkStatus,
};
pub use dynamics::{
    analyse_diffusion, analyse_inequality, analyse_social_network, InequalityAnalysis,
    NetworkAnalysis,
};
pub use enquiry::{ResearchConstraint, ResearchEnquiry, ResearchQuestion};
pub use hypothesis::{HypothesisGraph, HypothesisNode, HypothesisRevision};
pub use inference_chain::{InferenceChain, InferenceStep};
pub use intentionality::{assess_intentionality, classify_mistake, Intentionality, MistakeType};
pub use investigation::{Evidence, EvidenceReliability, Hypothesis, Investigation, TimelineEntry};
pub use perspective::{
    add_bias, compare_perspectives, detect_perspective_conflict, reconcile_perspectives,
    register_perspective, Bias, Perspective, PerspectiveConflict,
};
pub use sentiment::{SentimentAssessment, SentimentDimension, SentimentTrend};
