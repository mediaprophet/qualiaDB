//! Investigation — evidence collection, hypotheses, timelines, links.

use std::collections::BTreeMap;

/// Reliability level for a piece of evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceReliability {
    Confirmed,
    Probable,
    Possible,
    Doubtful,
    Discredited,
}

impl EvidenceReliability {
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Confirmed => 1.0,
            Self::Probable => 0.75,
            Self::Possible => 0.5,
            Self::Doubtful => 0.25,
            Self::Discredited => 0.0,
        }
    }
}

/// A piece of evidence in an investigation.
#[derive(Debug, Clone)]
pub struct Evidence {
    pub id: String,
    pub description: String,
    pub source: String,
    pub reliability: EvidenceReliability,
    pub timestamp: Option<String>,
    pub supports: Vec<String>,    // Hypothesis IDs this supports
    pub contradicts: Vec<String>, // Hypothesis IDs this contradicts
}

impl Evidence {
    pub fn new(id: &str, description: &str, source: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            source: source.to_string(),
            reliability: EvidenceReliability::Possible,
            timestamp: None,
            supports: Vec::new(),
            contradicts: Vec::new(),
        }
    }

    pub fn set_reliability(&mut self, reliability: EvidenceReliability) {
        self.reliability = reliability;
    }
}

/// A hypothesis proposed during an investigation.
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub id: String,
    pub statement: String,
    pub supporting_evidence: Vec<String>,
    pub contradicting_evidence: Vec<String>,
    pub confidence: f64,
    pub status: HypothesisStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    Proposed,
    UnderEvaluation,
    Supported,
    Refuted,
    Inconclusive,
}

impl Hypothesis {
    pub fn new(id: &str, statement: &str) -> Self {
        Self {
            id: id.to_string(),
            statement: statement.to_string(),
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
            confidence: 0.0,
            status: HypothesisStatus::Proposed,
        }
    }

    /// Evaluate the hypothesis based on collected evidence.
    pub fn evaluate(&mut self) {
        let support_count = self.supporting_evidence.len();
        let contra_count = self.contradicting_evidence.len();
        let total = support_count + contra_count;
        if total == 0 {
            self.status = HypothesisStatus::Inconclusive;
            return;
        }
        self.confidence = support_count as f64 / total as f64;
        self.status = if self.confidence > 0.7 {
            HypothesisStatus::Supported
        } else if self.confidence < 0.3 {
            HypothesisStatus::Refuted
        } else {
            HypothesisStatus::UnderEvaluation
        };
    }
}

/// A timeline entry for the investigation.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub timestamp: String,
    pub event: String,
    pub evidence_id: Option<String>,
}

/// An investigation — evidence, hypotheses, timeline, and links.
#[derive(Debug, Clone)]
pub struct Investigation {
    pub id: String,
    pub evidence: BTreeMap<String, Evidence>,
    pub hypotheses: BTreeMap<String, Hypothesis>,
    pub timeline: Vec<TimelineEntry>,
    pub links: Vec<(String, String, String)>, // (source, target, link_type)
}

impl Investigation {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            evidence: BTreeMap::new(),
            hypotheses: BTreeMap::new(),
            timeline: Vec::new(),
            links: Vec::new(),
        }
    }

    pub fn collect_evidence(&mut self, evidence: Evidence) {
        self.evidence.insert(evidence.id.clone(), evidence);
    }

    pub fn set_reliability(&mut self, evidence_id: &str, reliability: EvidenceReliability) -> bool {
        if let Some(ev) = self.evidence.get_mut(evidence_id) {
            ev.set_reliability(reliability);
            true
        } else {
            false
        }
    }

    pub fn propose_hypothesis(&mut self, hypothesis: Hypothesis) {
        self.hypotheses.insert(hypothesis.id.clone(), hypothesis);
    }

    /// Evaluate evidence for a hypothesis — links evidence and re-evaluates.
    pub fn evaluate_evidence(&mut self, hypothesis_id: &str) -> bool {
        let ev_ids: Vec<String> = self.evidence.keys().cloned().collect();
        for ev_id in ev_ids {
            let (supports, contradicts) = {
                let ev = self.evidence.get(&ev_id).unwrap();
                (ev.supports.clone(), ev.contradicts.clone())
            };
            if let Some(hyp) = self.hypotheses.get_mut(hypothesis_id) {
                if supports.contains(&hyp.id.to_string())
                    && !hyp.supporting_evidence.contains(&ev_id)
                {
                    hyp.supporting_evidence.push(ev_id.clone());
                }
                if contradicts.contains(&hyp.id.to_string())
                    && !hyp.contradicting_evidence.contains(&ev_id)
                {
                    hyp.contradicting_evidence.push(ev_id);
                }
            }
        }
        if let Some(hyp) = self.hypotheses.get_mut(hypothesis_id) {
            hyp.evaluate();
            true
        } else {
            false
        }
    }

    pub fn create_timeline(&mut self, timestamp: &str, event: &str, evidence_id: Option<&str>) {
        self.timeline.push(TimelineEntry {
            timestamp: timestamp.to_string(),
            event: event.to_string(),
            evidence_id: evidence_id.map(|s| s.to_string()),
        });
    }

    pub fn add_link(&mut self, source: &str, target: &str, link_type: &str) {
        self.links.push((
            source.to_string(),
            target.to_string(),
            link_type.to_string(),
        ));
    }

    /// Find a path between two entities through the link graph (BFS).
    pub fn find_path(&self, start: &str, end: &str) -> Option<Vec<String>> {
        if start == end {
            return Some(vec![start.to_string()]);
        }
        let mut visited = std::collections::BTreeSet::new();
        let mut queue: Vec<(String, Vec<String>)> =
            vec![(start.to_string(), vec![start.to_string()])];
        visited.insert(start.to_string());

        while let Some((current, path)) = queue.pop() {
            for (src, tgt, _) in &self.links {
                if src == &current && !visited.contains(tgt) {
                    let mut new_path = path.clone();
                    new_path.push(tgt.clone());
                    if tgt == end {
                        return Some(new_path);
                    }
                    visited.insert(tgt.clone());
                    queue.push((tgt.clone(), new_path));
                }
                if tgt == &current && !visited.contains(src) {
                    let mut new_path = path.clone();
                    new_path.push(src.clone());
                    if src == end {
                        return Some(new_path);
                    }
                    visited.insert(src.clone());
                    queue.push((src.clone(), new_path));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn investigation_basic() {
        let inv = Investigation::new("inv1");
        assert_eq!(inv.id, "inv1");
        assert!(inv.evidence.is_empty());
    }

    #[test]
    fn investigation_collect_evidence() {
        let mut inv = Investigation::new("inv1");
        inv.collect_evidence(Evidence::new("e1", "Document found", "source_a"));
        assert!(inv.evidence.contains_key("e1"));
    }

    #[test]
    fn investigation_propose_hypothesis() {
        let mut inv = Investigation::new("inv1");
        inv.propose_hypothesis(Hypothesis::new("h1", "X caused Y"));
        assert!(inv.hypotheses.contains_key("h1"));
    }

    #[test]
    fn hypothesis_evaluate_supported() {
        let mut inv = Investigation::new("inv1");
        let mut ev1 = Evidence::new("e1", "Evidence A", "src");
        ev1.supports = vec!["h1".to_string()];
        let mut ev2 = Evidence::new("e2", "Evidence B", "src");
        ev2.supports = vec!["h1".to_string()];
        let mut ev3 = Evidence::new("e3", "Evidence C", "src");
        ev3.contradicts = vec!["h1".to_string()];
        inv.collect_evidence(ev1);
        inv.collect_evidence(ev2);
        inv.collect_evidence(ev3);
        inv.propose_hypothesis(Hypothesis::new("h1", "X caused Y"));
        inv.evaluate_evidence("h1");
        let hyp = inv.hypotheses.get("h1").unwrap();
        assert_eq!(hyp.status, HypothesisStatus::Supported);
    }

    #[test]
    fn hypothesis_evaluate_refuted() {
        let mut inv = Investigation::new("inv1");
        let mut ev1 = Evidence::new("e1", "Evidence A", "src");
        ev1.contradicts = vec!["h1".to_string()];
        let mut ev2 = Evidence::new("e2", "Evidence B", "src");
        ev2.contradicts = vec!["h1".to_string()];
        let mut ev3 = Evidence::new("e3", "Evidence C", "src");
        ev3.supports = vec!["h1".to_string()];
        inv.collect_evidence(ev1);
        inv.collect_evidence(ev2);
        inv.collect_evidence(ev3);
        inv.propose_hypothesis(Hypothesis::new("h1", "X caused Y"));
        inv.evaluate_evidence("h1");
        let hyp = inv.hypotheses.get("h1").unwrap();
        assert_eq!(hyp.status, HypothesisStatus::Refuted);
    }

    #[test]
    fn investigation_timeline() {
        let mut inv = Investigation::new("inv1");
        inv.create_timeline("2024-01-01", "Event A", Some("e1"));
        inv.create_timeline("2024-02-01", "Event B", None);
        assert_eq!(inv.timeline.len(), 2);
    }

    #[test]
    fn investigation_find_path() {
        let mut inv = Investigation::new("inv1");
        inv.add_link("a", "b", "causal");
        inv.add_link("b", "c", "temporal");
        inv.add_link("c", "d", "informational");
        let path = inv.find_path("a", "d");
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn investigation_find_path_none() {
        let mut inv = Investigation::new("inv1");
        inv.add_link("a", "b", "causal");
        assert!(inv.find_path("a", "z").is_none());
    }
}
