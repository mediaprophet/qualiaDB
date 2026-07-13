// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// The structured key search engine: engine/indexing configuration, the query
// and result value types, and the relevance-scored search implementation.
use super::*;

/// Key search engine
pub struct KeySearchEngine {
    pub(in crate::specialized_libs::cryptographic_library) engine_type: SearchEngineType,
    pub(in crate::specialized_libs::cryptographic_library) indexing_strategy: IndexingStrategy,
    /// Indexed key metadata keyed by key id.
    key_metadata: HashMap<String, KeyMetadata>,
    /// Tags attached to each key.
    key_tags: HashMap<String, Vec<String>>,
    /// Purpose assigned to each key.
    key_purposes: HashMap<String, KeyPurpose>,
}

/// Search engine types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchEngineType {
    FullText,
    Semantic,
    Hybrid,
    Encrypted,
}

/// Indexing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingStrategy {
    Inverted,
    Ngram,
    SkipGram,
    BM25,
    Encrypted,
}

/// Key purposes for search filtering.
///
/// A key's purpose describes the cryptographic role it is intended for
/// (signing, encryption, key exchange, etc.). This is orthogonal to the
/// raw [`KeyAlgorithm`] — e.g. an `AES` key may be used for either
/// `Encryption` or `Decryption`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyPurpose {
    /// Digital signature generation
    Signing,
    /// Signature verification
    Verification,
    /// Data encryption
    Encryption,
    /// Data decryption
    Decryption,
    /// Key exchange / key agreement
    KeyExchange,
    /// Authentication / identity proof
    Authentication,
    /// Key derivation
    Derivation,
    /// Hashing / fingerprinting
    Hashing,
}

/// Structured query against the [`KeySearchIndex`].
///
/// Every field is optional; a `None`/empty field is treated as a wildcard
/// (i.e. it does not constrain the result set). When multiple fields are
/// populated they are combined as a logical AND — only keys satisfying every
/// constraint are returned.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// Free-form text matched as a case-insensitive substring against the
    /// key id and metadata fields (algorithm, key type, security level, …).
    pub text: Option<String>,
    /// Exact algorithm filter.
    pub algorithm: Option<KeyAlgorithm>,
    /// Exact purpose filter.
    pub purpose: Option<KeyPurpose>,
    /// Tag filter — a key matches if it carries *any* of the listed tags.
    pub tags: Vec<String>,
    /// Inclusive lower bound on the key creation timestamp.
    pub created_after: Option<u64>,
    /// Inclusive upper bound on the key creation timestamp.
    pub created_before: Option<u64>,
}

impl SearchQuery {
    /// Build an empty query (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the free-form text filter.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set the algorithm filter.
    pub fn with_algorithm(mut self, algorithm: KeyAlgorithm) -> Self {
        self.algorithm = Some(algorithm);
        self
    }

    /// Set the purpose filter.
    pub fn with_purpose(mut self, purpose: KeyPurpose) -> Self {
        self.purpose = Some(purpose);
        self
    }

    /// Add a tag to the tag filter.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the inclusive creation-date lower bound.
    pub fn with_created_after(mut self, ts: u64) -> Self {
        self.created_after = Some(ts);
        self
    }

    /// Set the inclusive creation-date upper bound.
    pub fn with_created_before(mut self, ts: u64) -> Self {
        self.created_before = Some(ts);
        self
    }
}

/// A single search hit returned by [`KeySearchIndex::search`].
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The id of the matching key.
    pub key_id: String,
    /// Aggregate relevance score in the range `0.0..=1.0+`. Higher is more
    /// relevant; exact matches contribute `1.0` and partial matches `0.5`,
    /// with multiple matching fields summed.
    pub relevance_score: f64,
    /// Names of the fields that contributed to the match (e.g. `"key_id"`,
    /// `"algorithm"`, `"tag:production"`).
    pub matched_fields: Vec<String>,
}

impl KeySearchEngine {
    pub fn new(engine_type: SearchEngineType) -> Self {
        let indexing_strategy = match engine_type {
            SearchEngineType::Encrypted => IndexingStrategy::Encrypted,
            _ => IndexingStrategy::Inverted,
        };
        Self {
            engine_type,
            indexing_strategy,
            key_metadata: HashMap::new(),
            key_tags: HashMap::new(),
            key_purposes: HashMap::new(),
        }
    }

    /// Index a key together with its metadata.
    ///
    /// The metadata is stored verbatim so that subsequent [`Self::search`]
    /// calls can filter on algorithm, creation date, and the textual
    /// representation of the metadata fields.
    pub fn index_key(&mut self, key_id: &str, metadata: &KeyMetadata) {
        self.key_metadata
            .insert(key_id.to_string(), metadata.clone());
    }

    /// Attach a tag to a previously indexed key. Tags are case-sensitive but
    /// compared case-insensitively during search.
    pub fn add_tag(&mut self, key_id: &str, tag: &str) {
        self.key_tags
            .entry(key_id.to_string())
            .or_default()
            .push(tag.to_string());
    }

    /// Assign a purpose to a previously indexed key.
    pub fn set_purpose(&mut self, key_id: &str, purpose: KeyPurpose) {
        self.key_purposes.insert(key_id.to_string(), purpose);
    }

    /// Number of indexed keys.
    pub fn indexed_key_count(&self) -> usize {
        self.key_metadata.len()
    }

    /// Run a structured [`SearchQuery`] against the indexed keys.
    ///
    /// Each populated query field acts as both a hard filter (non-matching
    /// keys are excluded) and a relevance signal. Relevance is accumulated:
    /// an exact match contributes `1.0`, a partial (substring) match
    /// contributes `0.5`. Results are returned sorted by descending
    /// relevance score.
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let q_text = query.text.as_deref().map(|t| t.trim().to_lowercase());
        let q_text = q_text.filter(|t| !t.is_empty());

        let mut results: Vec<SearchResult> = self
            .key_metadata
            .iter()
            .filter_map(|(key_id, metadata)| {
                let mut score: f64 = 0.0;
                let mut matched_fields: Vec<String> = Vec::new();

                // --- Text filter (substring on key_id + metadata fields) ---
                if let Some(ref q) = q_text {
                    let mut text_matched = false;

                    // key_id
                    let key_id_lc = key_id.to_lowercase();
                    if key_id_lc == *q {
                        score += 1.0;
                        matched_fields.push("key_id".to_string());
                        text_matched = true;
                    } else if key_id_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("key_id".to_string());
                        text_matched = true;
                    }

                    // algorithm
                    let algo_lc = format!("{:?}", metadata.key_algorithm).to_lowercase();
                    if algo_lc == *q {
                        score += 1.0;
                        matched_fields.push("algorithm".to_string());
                        text_matched = true;
                    } else if algo_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("algorithm".to_string());
                        text_matched = true;
                    }

                    // key_type
                    let kt_lc = format!("{:?}", metadata.key_type).to_lowercase();
                    if kt_lc == *q {
                        score += 1.0;
                        matched_fields.push("key_type".to_string());
                        text_matched = true;
                    } else if kt_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("key_type".to_string());
                        text_matched = true;
                    }

                    // security_level
                    let sl_lc = format!("{:?}", metadata.security_level).to_lowercase();
                    if sl_lc == *q {
                        score += 1.0;
                        matched_fields.push("security_level".to_string());
                        text_matched = true;
                    } else if sl_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("security_level".to_string());
                        text_matched = true;
                    }

                    // tags
                    if let Some(tags) = self.key_tags.get(key_id) {
                        for tag in tags {
                            let tag_lc = tag.to_lowercase();
                            if tag_lc == *q {
                                score += 1.0;
                                matched_fields.push(format!("tag:{}", tag));
                                text_matched = true;
                            } else if tag_lc.contains(q) {
                                score += 0.5;
                                matched_fields.push(format!("tag:{}", tag));
                                text_matched = true;
                            }
                        }
                    }

                    if !text_matched {
                        return None;
                    }
                }

                // --- Algorithm filter (exact match) ---
                if let Some(ref algo) = query.algorithm {
                    if metadata.key_algorithm != *algo {
                        return None;
                    }
                    score += 1.0;
                    matched_fields.push("algorithm".to_string());
                }

                // --- Purpose filter (exact match) ---
                if let Some(ref purpose) = query.purpose {
                    match self.key_purposes.get(key_id) {
                        Some(p) if p == purpose => {
                            score += 1.0;
                            matched_fields.push("purpose".to_string());
                        }
                        _ => return None,
                    }
                }

                // --- Tag filter (any tag matches, case-insensitive) ---
                if !query.tags.is_empty() {
                    let tags = self.key_tags.get(key_id);
                    let mut tag_matched = false;
                    if let Some(tags) = tags {
                        for query_tag in &query.tags {
                            let qt_lc = query_tag.to_lowercase();
                            if tags.iter().any(|t| t.to_lowercase() == qt_lc) {
                                score += 0.5;
                                matched_fields.push(format!("tag:{}", query_tag));
                                tag_matched = true;
                            }
                        }
                    }
                    if !tag_matched {
                        return None;
                    }
                }

                // --- Date range filter (inclusive) ---
                if let Some(after) = query.created_after {
                    if metadata.created_at < after {
                        return None;
                    }
                    matched_fields.push("created_after".to_string());
                }
                if let Some(before) = query.created_before {
                    if metadata.created_at > before {
                        return None;
                    }
                    matched_fields.push("created_before".to_string());
                }

                // A key that satisfied only filter constraints (no text) still
                // gets a baseline score so it is represented in the output.
                if score == 0.0 {
                    score = 1.0;
                }

                Some(SearchResult {
                    key_id: key_id.clone(),
                    relevance_score: score,
                    matched_fields,
                })
            })
            .collect();

        // Sort by descending relevance score, then by key_id for determinism.
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.key_id.cmp(&b.key_id))
        });
        results
    }
}
