use super::*;

/// Dataset relationships
#[derive(Debug, Clone)]
pub struct Relationship {
    pub relationship_id: String,
    pub source_dataset: String,
    pub target_dataset: String,
    pub relationship_type: RelationshipType,
    pub strength: f64,
}

/// Relationship types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationshipType {
    Derived,
    Aggregated,
    Transformed,
    Merged,
    Linked,
    Hierarchical,
}

/// Search index for efficient dataset discovery
pub struct SearchIndex {
    index_entries: HashMap<String, IndexEntry>,
    search_engine: SearchEngine,
}

/// Index entry
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub entry_id: String,
    pub keywords: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub relevance_score: f64,
}

/// Search engine
pub struct SearchEngine {
    engine_type: SearchEngineType,
    indexing_strategy: IndexingStrategy,
}

/// Search engine types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchEngineType {
    FullText,
    Semantic,
    Hybrid,
    Vector,
}

/// Indexing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingStrategy {
    Inverted,
    Ngram,
    SkipGram,
    BM25,
    BTree,
    Custom,
}

impl DataCatalog {
    pub fn new() -> Self {
        Self {
            datasets: HashMap::new(),
            relationships: HashMap::new(),
            tags: HashMap::new(),
            search_index: SearchIndex::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.search_index.initialize()?;
        Ok(())
    }

    /// Register a dataset's metadata in the catalog and refresh the search
    /// index so the dataset is discoverable by name and by its metadata
    /// keywords.
    pub fn register_dataset(&mut self, metadata: DatasetMetadata) {
        let dataset_id = metadata.dataset_id.clone();

        // Build a search-index entry from the metadata. The dataset id and a
        // few derived keywords become the searchable surface.
        let mut keywords = vec![dataset_id.clone()];
        if let Some(features) = metadata.dimensions.features {
            keywords.push(format!("features_{}", features));
        }
        keywords.push(format!("rows_{}", metadata.dimensions.rows));
        keywords.push(format!("type_{:?}", metadata.dataset_type));

        let entry = IndexEntry {
            entry_id: dataset_id.clone(),
            keywords,
            metadata: HashMap::new(),
            relevance_score: 1.0,
        };
        self.search_index.index(entry);

        self.datasets.insert(dataset_id, metadata);
    }

    /// Record a relationship between two datasets, keyed by the source dataset.
    pub fn add_relationship(&mut self, source: &str, target: &str, relationship: Relationship) {
        // Keep the relationship record consistent with the requested endpoints.
        let mut rel = relationship;
        rel.source_dataset = source.to_string();
        rel.target_dataset = target.to_string();

        self.relationships
            .entry(source.to_string())
            .or_default()
            .push(rel);
    }

    /// Tag a dataset. Tags are stored as `dataset_id -> Vec<tag>` and also
    /// folded into the search index so tagged datasets are searchable by tag.
    pub fn add_tag(&mut self, dataset_id: &str, tag: &str) {
        self.tags
            .entry(dataset_id.to_string())
            .or_default()
            .push(tag.to_string());

        // Mirror the tag into the search index entry's keywords if present.
        if let Some(entry) = self.search_index.index_entries.get_mut(dataset_id) {
            if !entry.keywords.iter().any(|k| k == tag) {
                entry.keywords.push(tag.to_string());
            }
        }
    }

    /// Search datasets by name, tag, or indexed keyword. Matching is
    /// case-insensitive substring matching against the dataset id, the dataset's
    /// tags, and the search-index keywords.
    pub fn search(&self, query: &str) -> Vec<&DatasetMetadata> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return self.datasets.values().collect();
        }

        let mut matches: Vec<&DatasetMetadata> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Match by dataset id (name).
        for (id, metadata) in &self.datasets {
            if id.to_lowercase().contains(&q) {
                seen.insert(id.clone());
                matches.push(metadata);
            }
        }

        // Match by tag.
        for (id, tag_list) in &self.tags {
            if seen.contains(id) {
                continue;
            }
            if tag_list.iter().any(|t| t.to_lowercase().contains(&q)) {
                if let Some(metadata) = self.datasets.get(id) {
                    seen.insert(id.clone());
                    matches.push(metadata);
                }
            }
        }

        // Match by search-index keywords.
        for entry in self.search_index.search(&q) {
            if seen.contains(&entry.entry_id) {
                continue;
            }
            if let Some(metadata) = self.datasets.get(&entry.entry_id) {
                seen.insert(entry.entry_id.clone());
                matches.push(metadata);
            }
        }

        matches
    }

    /// Return metadata for every dataset carrying the given tag (case-insensitive).
    pub fn get_by_tag(&self, tag: &str) -> Vec<&DatasetMetadata> {
        let t = tag.to_lowercase();
        let mut result = Vec::new();
        for (id, tag_list) in &self.tags {
            if tag_list.iter().any(|x| x.to_lowercase() == t) {
                if let Some(metadata) = self.datasets.get(id) {
                    result.push(metadata);
                }
            }
        }
        result
    }
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            index_entries: HashMap::new(),
            search_engine: SearchEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Add (or replace) an entry in the search index, keyed by `entry_id`.
    pub fn index(&mut self, entry: IndexEntry) {
        self.index_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Simple keyword search: returns entries whose keywords or metadata
    /// values contain the query (case-insensitive substring match).
    pub fn search(&self, query: &str) -> Vec<&IndexEntry> {
        let q = query.to_lowercase();
        self.index_entries
            .values()
            .filter(|entry| {
                entry.keywords.iter().any(|k| k.to_lowercase().contains(&q))
                    || entry
                        .metadata
                        .values()
                        .any(|v| v.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Returns a reference to the underlying search engine configuration.
    pub fn search_engine(&self) -> &SearchEngine {
        &self.search_engine
    }

    /// Returns a mutable reference to the search engine so callers can
    /// reconfigure its type or indexing strategy.
    pub fn search_engine_mut(&mut self) -> &mut SearchEngine {
        &mut self.search_engine
    }

    /// Returns the number of entries currently held in the index.
    pub fn entry_count(&self) -> usize {
        self.index_entries.len()
    }

    /// Remove an entry from the index by its id. Returns the removed entry
    /// if it existed.
    pub fn remove_entry(&mut self, entry_id: &str) -> Option<IndexEntry> {
        self.index_entries.remove(entry_id)
    }

    /// Look up an entry by id.
    pub fn get_entry(&self, entry_id: &str) -> Option<&IndexEntry> {
        self.index_entries.get(entry_id)
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            engine_type: SearchEngineType::FullText,
            indexing_strategy: IndexingStrategy::Inverted,
        }
    }

    /// Returns the configured search engine type.
    pub fn engine_type(&self) -> &SearchEngineType {
        &self.engine_type
    }

    /// Reconfigure the search engine type.
    pub fn set_engine_type(&mut self, engine_type: SearchEngineType) {
        self.engine_type = engine_type;
    }

    /// Returns the configured indexing strategy.
    pub fn indexing_strategy(&self) -> &IndexingStrategy {
        &self.indexing_strategy
    }

    /// Reconfigure the indexing strategy.
    pub fn set_indexing_strategy(&mut self, strategy: IndexingStrategy) {
        self.indexing_strategy = strategy;
    }
}
