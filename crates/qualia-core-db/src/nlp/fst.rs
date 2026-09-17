//! Finite-State Transducer for morphology.
//!
//! A simple trie of surface forms → `lemma|features`, walked for lookups.
//! When an exact surface is absent, suffix-based morphology rules strip
//! common English inflections (`-s`, `-ed`, `-ing`, `-es`) and re-lookup the
//! stem, annotating the result with the corresponding feature tag.
//!
//! Deterministic, allocation-free in the hot lookup walk (results own their
//! `String`s, which is Tier-2 authoring output). WASM-compatible.

use super::span::DocSpan;
use std::collections::HashMap;

/// One morphology analysis result for a surface form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FstResult {
    pub lemma: String,
    pub features: String,
    /// Byte span of the matched surface within the queried word (always
    /// `0..word.len()` for a standalone lookup; included for provenance
    /// symmetry with the rest of the NLP pipeline).
    pub span: DocSpan,
}

/// Suffix rule: strip `suffix` from the surface, re-lookup the stem, and
/// annotate with `feature` when the stem resolves.
struct SuffixRule {
    suffix: &'static str,
    feature: &'static str,
}

const SUFFIX_RULES: &[SuffixRule] = &[
    SuffixRule {
        suffix: "ies",
        feature: "PL",
    },
    SuffixRule {
        suffix: "es",
        feature: "PL",
    },
    SuffixRule {
        suffix: "s",
        feature: "PL",
    },
    SuffixRule {
        suffix: "ied",
        feature: "PAST",
    },
    SuffixRule {
        suffix: "ed",
        feature: "PAST",
    },
    SuffixRule {
        suffix: "ing",
        feature: "VING",
    },
];

/// Compiled FST dictionary: a trie mapping surface bytes to an entry id.
pub struct FstDict {
    /// Trie nodes: `children[byte] = node_id`.
    nodes: Vec<HashMap<u8, usize>>,
    /// `out[node_id] = entry_id` when a surface terminates at that node.
    out: Vec<Option<usize>>,
    /// Entries: `(lemma, features)`.
    entries: Vec<(String, String)>,
}

impl FstDict {
    /// Build a dictionary from `(surface, "lemma|features")` pairs. If the
    /// second element contains no `|`, the whole string is treated as the
    /// lemma with empty features.
    pub fn from_entries(entries: &[(String, String)]) -> Self {
        let mut nodes: Vec<HashMap<u8, usize>> = vec![HashMap::new()];
        let mut out: Vec<Option<usize>> = vec![None];
        let mut stored: Vec<(String, String)> = Vec::with_capacity(entries.len());

        for (surface, payload) in entries {
            let (lemma, features) = split_payload(payload);
            let id = stored.len();
            stored.push((lemma, features));
            let mut state = 0usize;
            for &b in surface.as_bytes() {
                let next = nodes[state].get(&b).copied();
                state = match next {
                    Some(n) => n,
                    None => {
                        let n = nodes.len();
                        nodes.push(HashMap::new());
                        out.push(None);
                        nodes[state].insert(b, n);
                        n
                    }
                };
            }
            out[state] = Some(id);
        }

        FstDict {
            nodes,
            out,
            entries: stored,
        }
    }

    /// Walk the trie for `word`. Returns exact matches first, then suffix-rule
    /// derivations. The span covers the whole queried word.
    pub fn lookup(&self, word: &str) -> Vec<FstResult> {
        let mut results = Vec::new();
        let span = DocSpan::new(0, word.len() as u32);
        if let Some(id) = self.walk(word.as_bytes()) {
            let (lemma, features) = &self.entries[id];
            results.push(FstResult {
                lemma: lemma.clone(),
                features: features.clone(),
                span,
            });
            return results;
        }
        // Suffix-based morphology: try stripping each suffix and re-walking.
        for rule in SUFFIX_RULES {
            if let Some(stem) = word.strip_suffix(rule.suffix) {
                if stem.is_empty() {
                    continue;
                }
                // Candidate stems to try, in priority order. For "ies" we also
                // try the "y" re-write ("cities" → "city").
                let mut candidates: Vec<String> = vec![stem.to_string()];
                if rule.suffix == "ies" {
                    candidates.push(format!("{}y", stem));
                }
                for cand in candidates {
                    if let Some(id) = self.walk(cand.as_bytes()) {
                        let (lemma, base_features) = &self.entries[id];
                        let features = if base_features.is_empty() {
                            rule.feature.to_string()
                        } else {
                            format!("{}|{}", base_features, rule.feature)
                        };
                        results.push(FstResult {
                            lemma: lemma.clone(),
                            features,
                            span,
                        });
                    }
                }
                if !results.is_empty() {
                    return results;
                }
            }
        }
        results
    }

    /// Pure trie walk — no allocation. Returns the entry id at the terminal
    /// node, or `None` if the surface is absent.
    fn walk(&self, bytes: &[u8]) -> Option<usize> {
        let mut state = 0usize;
        for &b in bytes {
            state = self.nodes[state].get(&b).copied()?;
        }
        self.out[state]
    }

    /// Number of stored entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

fn split_payload(payload: &str) -> (String, String) {
    match payload.find('|') {
        Some(idx) => (payload[..idx].to_string(), payload[idx + 1..].to_string()),
        None => (payload.to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dict() -> FstDict {
        FstDict::from_entries(&[
            ("cat".into(), "cat|N".into()),
            ("dog".into(), "dog|N".into()),
            ("walk".into(), "walk|V".into()),
            ("carry".into(), "carry|V".into()),
            ("city".into(), "city|N".into()),
        ])
    }

    #[test]
    fn lookup_known_word() {
        let d = sample_dict();
        let r = d.lookup("cat");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].lemma, "cat");
        assert_eq!(r[0].features, "N");
        assert_eq!(r[0].span, DocSpan::new(0, 3));
    }

    #[test]
    fn lookup_unknown_word_returns_empty() {
        let d = sample_dict();
        let r = d.lookup("xyzzy");
        assert!(r.is_empty());
    }

    #[test]
    fn suffix_plural_s() {
        let d = sample_dict();
        let r = d.lookup("cats");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].lemma, "cat");
        assert!(r[0].features.contains("PL"));
    }

    #[test]
    fn suffix_past_ed() {
        let d = sample_dict();
        let r = d.lookup("walked");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].lemma, "walk");
        assert!(r[0].features.contains("PAST"));
    }

    #[test]
    fn suffix_ies_to_y() {
        let d = sample_dict();
        let r = d.lookup("cities");
        assert!(!r.is_empty());
        assert_eq!(r[0].lemma, "city");
        assert!(r[0].features.contains("PL"));
    }

    #[test]
    fn suffix_ing() {
        let d = sample_dict();
        let r = d.lookup("walking");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].lemma, "walk");
        assert!(r[0].features.contains("VING"));
    }

    #[test]
    fn entry_count() {
        let d = sample_dict();
        assert_eq!(d.entry_count(), 5);
    }
}
