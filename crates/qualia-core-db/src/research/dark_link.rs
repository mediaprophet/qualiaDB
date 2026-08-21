//! Dark link inference — provenance gaps, concealment patterns, confirmation/refutation.

/// Status of a dark link hypothesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DarkLinkStatus {
    /// Inferred but not yet confirmed.
    Inferred,
    /// Confirmed by evidence.
    Confirmed,
    /// Refuted by evidence.
    Refuted,
    /// Insufficient evidence.
    Inconclusive,
}

/// A dark link — a hypothesised causal or informational connection that is
/// not directly visible in the available data.
#[derive(Debug, Clone)]
pub struct DarkLink {
    pub id: String,
    pub source: String,
    pub target: String,
    pub link_type: String,
    pub confidence: f64,
    pub status: DarkLinkStatus,
    pub evidence: Vec<String>,
    pub provenance_gaps: Vec<String>,
}

impl DarkLink {
    pub fn new(id: &str, source: &str, target: &str, link_type: &str) -> Self {
        Self {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            link_type: link_type.to_string(),
            confidence: 0.0,
            status: DarkLinkStatus::Inferred,
            evidence: Vec::new(),
            provenance_gaps: Vec::new(),
        }
    }

    pub fn add_evidence(&mut self, evidence: &str) {
        self.evidence.push(evidence.to_string());
        self.confidence = (self.confidence + 0.2).min(1.0);
    }

    pub fn confirm(&mut self) {
        self.status = DarkLinkStatus::Confirmed;
        self.confidence = 1.0;
    }

    pub fn refute(&mut self) {
        self.status = DarkLinkStatus::Refuted;
        self.confidence = 0.0;
    }

    pub fn add_provenance_gap(&mut self, gap: &str) {
        self.provenance_gaps.push(gap.to_string());
    }
}

/// Detect provenance gaps in a list of items — missing source references,
/// unexplained temporal gaps, or attribution chains with breaks.
pub fn detect_provenance_gaps(items: &[(String, String, Option<String>)]) -> Vec<String> {
    let mut gaps = Vec::new();
    for (id, content, source) in items {
        if source.is_none() {
            gaps.push(format!("{id}: missing source attribution"));
        }
        // Check for unexplained temporal gaps (simplified: look for date markers).
        if !content.contains("date") && !content.contains("year") && !content.contains("time") {
            gaps.push(format!("{id}: no temporal reference found"));
        }
    }
    gaps
}

/// Detect concealment patterns — redacted content, missing sections,
/// or suspiciously uniform language across sources.
pub fn detect_concealment_patterns(items: &[(String, String)]) -> Vec<String> {
    let mut patterns = Vec::new();
    for (id, content) in items {
        if content.contains("[REDACTED]") || content.contains("...") {
            patterns.push(format!("{id}: contains redaction markers"));
        }
        if content.len() < 50 {
            patterns.push(format!(
                "{id}: suspiciously short content ({}) bytes",
                content.len()
            ));
        }
    }

    // Check for suspiciously uniform language (all items identical).
    if items.len() > 2 {
        let first = &items[0].1;
        let all_same = items.iter().all(|(_, c)| c == first);
        if all_same {
            patterns.push("all items have identical content — possible templating".to_string());
        }
    }

    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_link_creation() {
        let dl = DarkLink::new("dl1", "entity_a", "entity_b", "causal");
        assert_eq!(dl.source, "entity_a");
        assert_eq!(dl.status, DarkLinkStatus::Inferred);
        assert_eq!(dl.confidence, 0.0);
    }

    #[test]
    fn dark_link_add_evidence() {
        let mut dl = DarkLink::new("dl1", "a", "b", "causal");
        dl.add_evidence("Document X confirms the link");
        assert!(dl.confidence > 0.0);
        assert_eq!(dl.evidence.len(), 1);
    }

    #[test]
    fn dark_link_confirm_refute() {
        let mut dl = DarkLink::new("dl1", "a", "b", "causal");
        dl.confirm();
        assert_eq!(dl.status, DarkLinkStatus::Confirmed);
        assert_eq!(dl.confidence, 1.0);

        let mut dl2 = DarkLink::new("dl2", "c", "d", "informational");
        dl2.refute();
        assert_eq!(dl2.status, DarkLinkStatus::Refuted);
        assert_eq!(dl2.confidence, 0.0);
    }

    #[test]
    fn detect_provenance_gaps_missing_source() {
        let items = vec![
            (
                "i1".to_string(),
                "Some content with date".to_string(),
                Some("src1".to_string()),
            ),
            (
                "i2".to_string(),
                "Content without date ref".to_string(),
                None,
            ),
        ];
        let gaps = detect_provenance_gaps(&items);
        assert!(gaps.iter().any(|g| g.contains("i2: missing source")));
    }

    #[test]
    fn detect_concealment_redacted() {
        let items = vec![(
            "i1".to_string(),
            "Some [REDACTED] content here that is long enough".to_string(),
        )];
        let patterns = detect_concealment_patterns(&items);
        assert!(patterns.iter().any(|p| p.contains("redaction")));
    }

    #[test]
    fn detect_concealment_uniform() {
        let items = vec![
            ("i1".to_string(), "identical".to_string()),
            ("i2".to_string(), "identical".to_string()),
            ("i3".to_string(), "identical".to_string()),
        ];
        let patterns = detect_concealment_patterns(&items);
        assert!(patterns.iter().any(|p| p.contains("templating")));
    }
}
