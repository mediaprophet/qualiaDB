//! Inference chain — sequential inferences with confidence tracking.

/// A single step in an inference chain.
#[derive(Debug, Clone)]
pub struct InferenceStep {
    pub id: String,
    pub premise: String,
    pub conclusion: String,
    pub confidence: f64,
    pub validated: bool,
}

impl InferenceStep {
    pub fn new(id: &str, premise: &str, conclusion: &str, confidence: f64) -> Self {
        Self {
            id: id.to_string(),
            premise: premise.to_string(),
            conclusion: conclusion.to_string(),
            confidence: confidence.clamp(0.0, 1.0),
            validated: false,
        }
    }

    pub fn validate(&mut self) {
        self.validated = true;
    }
}

/// A chain of inferences where each step's conclusion feeds the next.
#[derive(Debug, Clone)]
pub struct InferenceChain {
    pub id: String,
    pub steps: Vec<InferenceStep>,
}

impl InferenceChain {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            steps: Vec::new(),
        }
    }

    pub fn make_inference(&mut self, step: InferenceStep) {
        self.steps.push(step);
    }

    /// Chain a new inference from the previous step's conclusion.
    pub fn chain_inference(&mut self, id: &str, new_conclusion: &str, confidence: f64) -> bool {
        let premise = self.steps.last().map(|s| s.conclusion.clone());
        if let Some(premise) = premise {
            self.steps
                .push(InferenceStep::new(id, &premise, new_conclusion, confidence));
            true
        } else {
            false
        }
    }

    pub fn set_confidence(&mut self, step_id: &str, confidence: f64) -> bool {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == step_id) {
            step.confidence = confidence.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    pub fn validate(&mut self, step_id: &str) -> bool {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == step_id) {
            step.validate();
            true
        } else {
            false
        }
    }

    /// Combined confidence of the chain (product of step confidences).
    pub fn combined_confidence(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        self.steps.iter().map(|s| s.confidence).product()
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_basic() {
        let mut chain = InferenceChain::new("ch1");
        chain.make_inference(InferenceStep::new("s1", "A is true", "B is likely", 0.8));
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn chain_inference_links() {
        let mut chain = InferenceChain::new("ch1");
        chain.make_inference(InferenceStep::new("s1", "A", "B", 0.8));
        assert!(chain.chain_inference("s2", "C", 0.7));
        assert_eq!(chain.steps[1].premise, "B");
    }

    #[test]
    fn chain_combined_confidence() {
        let mut chain = InferenceChain::new("ch1");
        chain.make_inference(InferenceStep::new("s1", "A", "B", 0.8));
        chain.chain_inference("s2", "C", 0.5);
        let combined = chain.combined_confidence();
        assert!((combined - 0.4).abs() < 0.01);
    }

    #[test]
    fn chain_validate_step() {
        let mut chain = InferenceChain::new("ch1");
        chain.make_inference(InferenceStep::new("s1", "A", "B", 0.8));
        assert!(chain.validate("s1"));
        assert!(chain.steps[0].validated);
    }

    #[test]
    fn chain_set_confidence() {
        let mut chain = InferenceChain::new("ch1");
        chain.make_inference(InferenceStep::new("s1", "A", "B", 0.8));
        chain.set_confidence("s1", 0.5);
        assert!((chain.steps[0].confidence - 0.5).abs() < 0.01);
    }

    #[test]
    fn chain_empty_confidence() {
        let chain = InferenceChain::new("ch1");
        assert_eq!(chain.combined_confidence(), 0.0);
    }
}
