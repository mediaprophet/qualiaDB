// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// Key catalog: registered key metadata, inter-key relationships, tags, and the
// keyword search index that fronts the search engine.
use super::*;

/// Key catalog for key management
pub struct KeyCatalog {
    keys: HashMap<String, KeyMetadata>,
    relationships: HashMap<String, Vec<KeyRelationship>>,
    tags: HashMap<String, Vec<String>>,
    pub(in crate::specialized_libs::cryptographic_library) search_index: KeySearchIndex,
}

/// Key relationships
#[derive(Debug, Clone)]
pub struct KeyRelationship {
    pub relationship_id: String,
    pub source_key: String,
    pub target_key: String,
    pub relationship_type: KeyRelationshipType,
    pub created_at: u64,
}

/// Key relationship types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyRelationshipType {
    /// Public-private key pair
    KeyPair,
    /// Derived from master key
    DerivedFrom,
    /// Backup of original key
    BackupOf,
    /// Rotated version of key
    RotatedFrom,
    /// Shared between parties
    SharedWith,
    /// Hierarchical relationship
    ChildOf,
}

/// Key search index
pub struct KeySearchIndex {
    index_entries: HashMap<String, KeyIndexEntry>,
    pub(in crate::specialized_libs::cryptographic_library) search_engine: KeySearchEngine,
}

/// Key index entry
#[derive(Debug, Clone)]
pub struct KeyIndexEntry {
    pub entry_id: String,
    pub keywords: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub relevance_score: f64,
}

impl KeyCatalog {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            relationships: HashMap::new(),
            tags: HashMap::new(),
            search_index: KeySearchIndex::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.search_index.initialize()?;
        Ok(())
    }

    /// Register a relationship between two keys (e.g. KeyPair, RotatedFrom, DerivedFrom).
    pub fn add_relationship(
        &mut self,
        source_key: &str,
        target_key: &str,
        rel_type: KeyRelationshipType,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let rel = KeyRelationship {
            relationship_id: format!("rel_{}_{}_{}", source_key, target_key, now),
            source_key: source_key.to_string(),
            target_key: target_key.to_string(),
            relationship_type: rel_type,
            created_at: now,
        };
        self.relationships
            .entry(source_key.to_string())
            .or_default()
            .push(rel);
    }

    /// Get all relationships for a given key (as source).
    pub fn get_relationships(&self, key_id: &str) -> &[KeyRelationship] {
        match self.relationships.get(key_id) {
            Some(rels) => rels,
            None => &[],
        }
    }

    /// Find the related key of a given type (e.g. find the public key paired with a private key).
    pub fn find_related(
        &self,
        key_id: &str,
        rel_type: KeyRelationshipType,
    ) -> Option<&KeyRelationship> {
        self.relationships
            .get(key_id)
            .and_then(|rels| rels.iter().find(|r| r.relationship_type == rel_type))
    }

    /// Register key metadata in the catalog.
    pub fn register_key(&mut self, metadata: KeyMetadata) {
        // Populate the search index so the key is discoverable by keyword/metadata.
        let mut index_metadata = HashMap::new();
        index_metadata.insert("key_type".to_string(), format!("{:?}", metadata.key_type));
        index_metadata.insert(
            "algorithm".to_string(),
            format!("{:?}", metadata.key_algorithm),
        );
        index_metadata.insert(
            "security_level".to_string(),
            format!("{:?}", metadata.security_level),
        );
        index_metadata.insert("key_size".to_string(), metadata.key_size.to_string());

        let entry = KeyIndexEntry {
            entry_id: metadata.key_id.clone(),
            keywords: vec![
                metadata.key_id.clone(),
                format!("{:?}", metadata.key_algorithm),
                format!("{:?}", metadata.key_type),
                format!("{:?}", metadata.security_level),
            ],
            metadata: index_metadata,
            relevance_score: 1.0,
        };
        self.search_index.index(entry);

        self.keys.insert(metadata.key_id.clone(), metadata);
    }

    /// Add a tag to a key for searchability.
    pub fn add_tag(&mut self, key_id: &str, tag: &str) {
        self.tags
            .entry(key_id.to_string())
            .or_default()
            .push(tag.to_string());
    }

    /// Get tags for a key.
    pub fn get_tags(&self, key_id: &str) -> &[String] {
        match self.tags.get(key_id) {
            Some(tags) => tags,
            None => &[],
        }
    }

    /// Search keys by keyword, tag, or metadata (case-insensitive substring).
    /// Returns the matching key IDs.
    pub fn search(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        let mut matches = std::collections::HashSet::new();

        // 1. Match against registered key metadata (key_id, algorithm, type, level).
        for (key_id, metadata) in &self.keys {
            if key_id.to_lowercase().contains(&q)
                || format!("{:?}", metadata.key_algorithm)
                    .to_lowercase()
                    .contains(&q)
                || format!("{:?}", metadata.key_type)
                    .to_lowercase()
                    .contains(&q)
                || format!("{:?}", metadata.security_level)
                    .to_lowercase()
                    .contains(&q)
            {
                matches.insert(key_id.clone());
            }
        }

        // 2. Match against tags (case-insensitive).
        for (key_id, tags) in &self.tags {
            if tags.iter().any(|t| t.to_lowercase().contains(&q)) {
                matches.insert(key_id.clone());
            }
        }

        // 3. Match against the search index entries.
        for entry in self.search_index.search_by_keyword(query) {
            matches.insert(entry.entry_id.clone());
        }

        matches.into_iter().collect()
    }

    /// Find all keys with a given tag (case-insensitive).
    pub fn get_by_tag(&self, tag: &str) -> Vec<String> {
        let t = tag.to_lowercase();
        self.tags
            .iter()
            .filter(|(_, tags)| tags.iter().any(|x| x.to_lowercase() == t))
            .map(|(key_id, _)| key_id.clone())
            .collect()
    }

    /// Number of registered keys.
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Number of tracked relationships.
    pub fn relationship_count(&self) -> usize {
        self.relationships.values().map(|v| v.len()).sum()
    }
}

impl KeySearchIndex {
    pub fn new() -> Self {
        Self {
            index_entries: HashMap::new(),
            search_engine: KeySearchEngine::new(SearchEngineType::Encrypted),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        // Actually configure the search engine rather than returning Ok(()) blindly.
        self.search_engine.engine_type = SearchEngineType::Hybrid;
        self.search_engine.indexing_strategy = IndexingStrategy::Inverted;
        Ok(())
    }

    /// Add an entry to the search index, keyed by its `entry_id`.
    pub fn index(&mut self, entry: KeyIndexEntry) {
        self.index_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Index a key together with its metadata so it becomes discoverable via
    /// [`Self::search`] with a structured [`SearchQuery`].
    pub fn index_key(&mut self, key_id: &str, metadata: &KeyMetadata) {
        self.search_engine.index_key(key_id, metadata);
    }

    /// Attach a tag to an indexed key.
    pub fn add_tag(&mut self, key_id: &str, tag: &str) {
        self.search_engine.add_tag(key_id, tag);
    }

    /// Assign a purpose to an indexed key.
    pub fn set_purpose(&mut self, key_id: &str, purpose: KeyPurpose) {
        self.search_engine.set_purpose(key_id, purpose);
    }

    /// Keyword search across index entries (case-insensitive substring match).
    /// Returns references to every entry whose `entry_id` or any keyword contains
    /// the query substring.
    pub fn search_by_keyword(&self, query: &str) -> Vec<&KeyIndexEntry> {
        let q = query.to_lowercase();
        self.index_entries
            .values()
            .filter(|entry| {
                entry.entry_id.to_lowercase().contains(&q)
                    || entry.keywords.iter().any(|k| k.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Structured search over the indexed keys. Delegates to the underlying
    /// [`KeySearchEngine`].
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        self.search_engine.search(query)
    }

    /// Number of indexed entries.
    pub fn entry_count(&self) -> usize {
        self.index_entries.len()
    }

    /// Number of keys indexed via [`Self::index_key`].
    pub fn indexed_key_count(&self) -> usize {
        self.search_engine.indexed_key_count()
    }
}
