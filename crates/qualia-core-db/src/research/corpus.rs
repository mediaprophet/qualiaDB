//! Corpus management — items, confidence, extraction.

use std::collections::BTreeMap;

/// Confidence level for a corpus item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusConfidence {
    High,
    Medium,
    Low,
    Unverified,
}

impl CorpusConfidence {
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::High => 0.9,
            Self::Medium => 0.6,
            Self::Low => 0.3,
            Self::Unverified => 0.0,
        }
    }
}

/// A corpus item — a document, dataset, or other source.
#[derive(Debug, Clone)]
pub struct CorpusItem {
    pub id: String,
    pub source_type: String, // "literature", "dataset", "web", "interview"
    pub title: String,
    pub content: String,
    pub confidence: CorpusConfidence,
    pub metadata: BTreeMap<String, String>,
    pub extracted_facts: Vec<String>,
}

impl CorpusItem {
    pub fn new(id: &str, source_type: &str, title: &str, content: &str) -> Self {
        Self {
            id: id.to_string(),
            source_type: source_type.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            confidence: CorpusConfidence::Unverified,
            metadata: BTreeMap::new(),
            extracted_facts: Vec::new(),
        }
    }

    pub fn set_confidence(&mut self, confidence: CorpusConfidence) {
        self.confidence = confidence;
    }

    pub fn add_fact(&mut self, fact: &str) {
        self.extracted_facts.push(fact.to_string());
    }

    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }
}

/// A research corpus — a collection of items with confidence tracking.
#[derive(Debug, Clone, Default)]
pub struct Corpus {
    pub items: Vec<CorpusItem>,
}

impl Corpus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_item(&mut self, item: CorpusItem) {
        self.items.push(item);
    }

    pub fn import_literature(&mut self, id: &str, title: &str, content: &str) -> &mut CorpusItem {
        let item = CorpusItem::new(id, "literature", title, content);
        self.items.push(item);
        self.items.last_mut().unwrap()
    }

    pub fn import_dataset(&mut self, id: &str, title: &str, content: &str) -> &mut CorpusItem {
        let item = CorpusItem::new(id, "dataset", title, content);
        self.items.push(item);
        self.items.last_mut().unwrap()
    }

    pub fn set_confidence(&mut self, item_id: &str, confidence: CorpusConfidence) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == item_id) {
            item.set_confidence(confidence);
            true
        } else {
            false
        }
    }

    /// Extract simple facts from corpus items matching a keyword.
    pub fn extract_facts(&self, keyword: &str) -> Vec<(String, String)> {
        let mut results = Vec::new();
        let kw_lower = keyword.to_lowercase();
        for item in &self.items {
            for sentence in item.content.split('.') {
                if sentence.to_lowercase().contains(&kw_lower) {
                    results.push((item.id.clone(), sentence.trim().to_string()));
                }
            }
        }
        results
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn average_confidence(&self) -> f64 {
        if self.items.is_empty() {
            return 0.0;
        }
        self.items
            .iter()
            .map(|i| i.confidence.as_f64())
            .sum::<f64>()
            / self.items.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_add_item() {
        let mut corpus = Corpus::new();
        corpus.add_item(CorpusItem::new(
            "c1",
            "literature",
            "Paper A",
            "Content here.",
        ));
        assert_eq!(corpus.len(), 1);
    }

    #[test]
    fn corpus_import_literature() {
        let mut corpus = Corpus::new();
        corpus.import_literature("l1", "Title", "Some content about climate.");
        assert_eq!(corpus.items[0].source_type, "literature");
    }

    #[test]
    fn corpus_set_confidence() {
        let mut corpus = Corpus::new();
        corpus.import_literature("l1", "T", "C");
        assert!(corpus.set_confidence("l1", CorpusConfidence::High));
        assert_eq!(corpus.items[0].confidence, CorpusConfidence::High);
    }

    #[test]
    fn corpus_extract_facts() {
        let mut corpus = Corpus::new();
        corpus.import_literature("l1", "T", "Climate change is real. The sky is blue.");
        let facts = corpus.extract_facts("climate");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].0, "l1");
    }

    #[test]
    fn corpus_average_confidence() {
        let mut corpus = Corpus::new();
        corpus.import_literature("l1", "T", "C");
        corpus.set_confidence("l1", CorpusConfidence::High);
        corpus.import_literature("l2", "T", "C");
        corpus.set_confidence("l2", CorpusConfidence::Low);
        let avg = corpus.average_confidence();
        assert!((avg - 0.6).abs() < 0.01);
    }
}
