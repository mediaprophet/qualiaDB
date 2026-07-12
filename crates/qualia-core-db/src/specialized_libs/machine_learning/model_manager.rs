//! Model manager, storage, catalog, search, and compression impls.

use super::*;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;

// Supporting implementations

impl ModelManager {
    pub fn new() -> Self {
        Self {
            model_storage: ModelStorage::new(),
            model_loader: ModelLoader::new(),
            model_converter: ModelConverter::new(),
            model_cache: ModelCache::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.model_storage.initialize()?;
        self.model_loader.initialize()?;
        self.model_converter.initialize()?;
        self.model_cache.initialize()?;
        Ok(())
    }

    pub fn load_model(&mut self, model_id: String, model_path: &str) -> Result<Model, MLError> {
        // Check cache first
        if let Some(cached_model) = self.model_cache.get(&model_id) {
            return Ok(cached_model);
        }

        // Load model from storage
        let model = self.model_storage.load_model(&model_id, model_path)?;

        // Cache the model
        self.model_cache.put(model_id.clone(), model.clone())?;

        Ok(model)
    }

    pub fn list_models(&self) -> Vec<String> {
        self.model_storage.list_models()
    }

    pub fn get_model_metadata(&self, model_id: &str) -> Option<ModelMetadata> {
        self.model_storage.get_model_metadata(model_id)
    }
}

impl ModelStorage {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            model_catalog: ModelCatalog::new(),
            compression_engine: ModelCompression::new(),
            version_control: ModelVersionControl::new(),
            model_store: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.create_zones()?;
        self.model_catalog.initialize()?;
        self.compression_engine.initialize()?;
        self.version_control.initialize()?;
        Ok(())
    }

    fn create_zones(&mut self) -> Result<(), MLError> {
        let zones = vec![
            ("llm", ModelZoneType::LargeLanguage),
            ("cv", ModelZoneType::ComputerVision),
            ("audio", ModelZoneType::AudioProcessing),
            ("multimodal", ModelZoneType::Multimodal),
            ("embedding", ModelZoneType::Embedding),
            ("transformer", ModelZoneType::Transformer),
            ("cnn", ModelZoneType::Convolutional),
            ("rnn", ModelZoneType::Recurrent),
        ];

        for (name, zone_type) in zones {
            let zone = ModelZone {
                zone_id: name.to_string(),
                zone_type,
                capacity: 10 * 1024 * 1024 * 1024, // 10GB
                models: HashMap::new(),
                access_pattern: AccessPattern::Adaptive,
            };
            self.zones.insert(name.to_string(), zone);
        }

        Ok(())
    }

    pub fn load_model(&mut self, model_id: &str, model_path: &str) -> Result<Model, MLError> {
        if let Some(model) = self.model_store.get(model_id) {
            return Ok(model.clone());
        }

        // Attempt a real GGUF load when the path points at an existing .gguf file.
        // On non-GGUF / missing / unreadable files we fall back to the mock scaffold
        // model so downstream inference still has something to operate on.
        let model = if model_path.to_ascii_lowercase().ends_with(".gguf")
            && std::path::Path::new(model_path).exists()
        {
            match Self::load_gguf_model(model_id, model_path) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!(
                        "ModelStorage::load_model: GGUF load failed for {} ({}); \
                         falling back to mock model",
                        model_path,
                        e
                    );
                    Self::mock_model(model_id)
                }
            }
        } else {
            if std::path::Path::new(model_path).exists() {
                log::warn!(
                    "ModelStorage::load_model: {} is not a .gguf file; \
                     falling back to mock model",
                    model_path
                );
            } else {
                log::warn!(
                    "ModelStorage::load_model: model file {} does not exist; \
                     falling back to mock model",
                    model_path
                );
            }
            Self::mock_model(model_id)
        };

        self.model_store.insert(model_id.to_string(), model.clone());
        Ok(model)
    }

    /// Build the mock scaffold model used when no real GGUF weights are available.
    fn mock_model(model_id: &str) -> Model {
        Model {
            model_id: model_id.to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture::new(),
            weights: vec![0.0; 1000],
            metadata: ModelMetadata::new(),
        }
    }

    /// Load a real GGUF file by memory-mapping it and extracting the `token_embd.weight`
    /// tensor via `GgufTensorIndex`. The embedding table can be many gigabytes for a full
    /// vocabulary, so only a bounded preview of per-token embeddings (first
    /// [`GGUF_EMBEDDING_PREVIEW_TOKENS`] tokens) is materialised into `Model.weights` to
    /// keep the in-memory `Vec<f64>` tractable. The `ModelArchitecture` is populated with a
    /// single `Linear` layer matching the embedding dimensions reported by the GGUF header.
    #[cfg(not(target_arch = "wasm32"))]
    fn load_gguf_model(model_id: &str, model_path: &str) -> Result<Model, MLError> {
        use memmap2::Mmap;

        let file = std::fs::File::open(model_path)
            .map_err(|e| MLError::ModelError(format!("open {}: {}", model_path, e)))?;
        let mmap = unsafe { Mmap::map(&file) }
            .map_err(|e| MLError::ModelError(format!("mmap {}: {}", model_path, e)))?;
        let mmap_bytes: &[u8] = &mmap;

        // `GgufTensorIndex::from_gguf` is infallible — it returns an empty index on a
        // malformed header — so validate that real tensor metadata was parsed.
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(mmap_bytes);
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            return Err(MLError::ModelError(
                "GGUF header parse failed or yielded no tensor metadata".to_string(),
            ));
        }

        let n_embd = index.emb_dim();
        let n_vocab = index.vocab_dim();
        if n_embd == 0 || n_vocab == 0 {
            return Err(MLError::ModelError(
                "GGUF has no token_embd.weight tensor".to_string(),
            ));
        }

        // Materialise a bounded preview of the embedding table into f64 weights.
        let token_cap = n_vocab.min(GGUF_EMBEDDING_PREVIEW_TOKENS);
        let mut weights = Vec::with_capacity(token_cap * n_embd);
        let mut row = vec![0.0f32; n_embd];
        for token_id in 0..token_cap as u32 {
            let n = index.dequantize_token_embedding_into(mmap_bytes, token_id, &mut row);
            if n == 0 {
                // Stop at the first token we cannot dequantize rather than emitting zeros.
                break;
            }
            for &v in &row[..n] {
                weights.push(v as f64);
            }
        }

        let loaded_rows = weights.len() / n_embd;
        let total_parameters = weights.len();

        let architecture = ModelArchitecture {
            layers: vec![LayerInfo {
                layer_id: "token_embd".to_string(),
                layer_type: LayerType::Linear,
                input_shape: vec![n_vocab],
                output_shape: vec![n_embd],
                parameters: total_parameters,
                activation: None,
            }],
            connections: vec![],
            input_shape: vec![n_vocab],
            output_shape: vec![n_embd],
            total_parameters,
        };

        let mut metadata = ModelMetadata::new();
        metadata.model_id = model_id.to_string();
        metadata.architecture = architecture.clone();
        metadata.parameters.weight_count = total_parameters;
        metadata.size = (total_parameters * std::mem::size_of::<f64>()) as u64;

        log::info!(
            "ModelStorage::load_model: loaded GGUF {} — n_embd={}, n_vocab={}, \
             materialised {} token embeddings ({} weights)",
            model_path,
            n_embd,
            n_vocab,
            loaded_rows,
            total_parameters
        );

        Ok(Model {
            model_id: model_id.to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::Custom("GGUF".to_string()),
            architecture,
            weights,
            metadata,
        })
    }

    /// WASM fallback: `memmap2` is unavailable, so a GGUF path cannot be mapped.
    #[cfg(target_arch = "wasm32")]
    fn load_gguf_model(_model_id: &str, model_path: &str) -> Result<Model, MLError> {
        Err(MLError::ModelError(format!(
            "GGUF loading via mmap is not supported on wasm32 ({})",
            model_path
        )))
    }

    pub fn list_models(&self) -> Vec<String> {
        let mut models = Vec::new();
        for zone in self.zones.values() {
            models.extend(zone.models.keys().cloned());
        }
        models
    }

    pub fn get_model_metadata(&self, model_id: &str) -> Option<ModelMetadata> {
        for zone in self.zones.values() {
            if let Some(metadata) = zone.models.get(model_id) {
                return Some(metadata.clone());
            }
        }
        None
    }
}

impl ModelCatalog {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            relationships: HashMap::new(),
            tags: HashMap::new(),
            search_index: ModelSearchIndex::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.search_index.initialize()?;
        Ok(())
    }

    /// Register a model in the catalog and add it to the search index.
    ///
    /// The model's id becomes both the catalog key and the index entry id. The model type
    /// and framework are added as keywords so the model is searchable by those terms, and
    /// the architecture's total parameter count is recorded in the index entry metadata.
    pub fn register_model(&mut self, model_id: &str, metadata: ModelMetadata) {
        // Build a search-index entry from the metadata before inserting it.
        let entry = ModelIndexEntry {
            entry_id: model_id.to_string(),
            keywords: vec![
                model_id.to_string(),
                format!("{:?}", metadata.model_type),
                format!("{:?}", metadata.framework),
            ],
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "model_type".to_string(),
                    format!("{:?}", metadata.model_type),
                );
                m.insert("framework".to_string(), format!("{:?}", metadata.framework));
                m.insert(
                    "total_parameters".to_string(),
                    metadata.architecture.total_parameters.to_string(),
                );
                m
            },
            relevance_score: 1.0,
        };
        self.search_index.index(entry);
        self.models.insert(model_id.to_string(), metadata);
    }

    /// Add a tag to a model for searchability.
    ///
    /// Tags are stored both in the catalog's `tags` map (tag → model ids) and as a keyword
    /// on the model's search-index entry, so a single `search()` call covers both paths.
    pub fn add_tag(&mut self, model_id: &str, tag: &str) {
        let tag_lower = tag.to_lowercase();
        self.tags
            .entry(tag_lower.clone())
            .or_default()
            .push(model_id.to_string());

        // Mirror the tag into the index entry's keywords so keyword search finds it too.
        if let Some(entry) = self.search_index.index_entries.get_mut(model_id) {
            if !entry.keywords.iter().any(|k| k == &tag_lower) {
                entry.keywords.push(tag_lower);
            }
        }
    }

    /// Search models by name, tags, or keywords (case-insensitive substring match).
    ///
    /// Returns matching model ids. A model matches if the query (lower-cased) is a substring
    /// of its id, any of its tags, or any keyword/metadata value on its index entry.
    pub fn search(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();

        // 1. Match by model id.
        for model_id in self.models.keys() {
            if model_id.to_lowercase().contains(&q) {
                matches.push(model_id.clone());
            }
        }

        // 2. Match by tag.
        for (tag, model_ids) in &self.tags {
            if tag.contains(&q) {
                for id in model_ids {
                    if !matches.contains(id) {
                        matches.push(id.clone());
                    }
                }
            }
        }

        // 3. Match by index entry keywords / metadata.
        for entry in self.search_index.search(&q) {
            if !matches.contains(&entry.entry_id) {
                matches.push(entry.entry_id.clone());
            }
        }

        matches
    }

    /// Find all models that carry a given tag (case-insensitive).
    pub fn get_by_tag(&self, tag: &str) -> Vec<String> {
        self.tags
            .get(&tag.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Record a relationship between two models (e.g. fine-tuned-from, quantized-from).
    ///
    /// The relationship is stored under the source model's id so all relationships
    /// originating from a given model can be retrieved together.
    pub fn add_relationship(&mut self, relationship: ModelRelationship) {
        self.relationships
            .entry(relationship.source_model.clone())
            .or_default()
            .push(relationship);
    }

    /// Return all relationships originating from `source_model`.
    pub fn get_relationships(&self, source_model: &str) -> Vec<&ModelRelationship> {
        self.relationships
            .get(source_model)
            .map(|rels| rels.iter().collect())
            .unwrap_or_default()
    }

    /// Remove all relationships originating from `source_model`. Returns the number removed.
    pub fn remove_relationships(&mut self, source_model: &str) -> usize {
        self.relationships
            .remove(source_model)
            .map(|rels| rels.len())
            .unwrap_or(0)
    }

    /// Return the total number of relationships recorded in the catalog.
    pub fn relationship_count(&self) -> usize {
        self.relationships.values().map(|rels| rels.len()).sum()
    }
}

impl ModelSearchIndex {
    pub fn new() -> Self {
        Self {
            index_entries: HashMap::new(),
            search_engine: ModelSearchEngine::new(),
            initialized: false,
        }
    }

    /// Actually initialize the search index: configure the engine for hybrid keyword
    /// search and mark the index as ready. Search calls before this return empty results.
    pub fn initialize(&mut self) -> Result<(), MLError> {
        // Configure a keyword/hybrid strategy suited to the catalog's text-based entries
        // (the default is a Semantic/Vector engine, which has no embedding backend here).
        self.search_engine.engine_type = SearchEngineType::Hybrid;
        self.search_engine.indexing_strategy = IndexingStrategy::Text;
        self.initialized = true;
        Ok(())
    }

    /// Add an entry to the search index. Replaces any existing entry with the same id.
    pub fn index(&mut self, entry: ModelIndexEntry) {
        self.index_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Keyword search across index entries (case-insensitive substring match on the
    /// entry id, keywords, and metadata values). Returns references to matching entries.
    pub fn search(&self, query: &str) -> Vec<&ModelIndexEntry> {
        if !self.initialized || query.is_empty() {
            return Vec::new();
        }
        let q = query.to_lowercase();
        self.index_entries
            .values()
            .filter(|entry| {
                entry.entry_id.to_lowercase().contains(&q)
                    || entry.keywords.iter().any(|k| k.to_lowercase().contains(&q))
                    || entry
                        .metadata
                        .values()
                        .any(|v| v.to_lowercase().contains(&q))
            })
            .collect()
    }
}

impl ModelSearchEngine {
    pub fn new() -> Self {
        Self {
            engine_type: SearchEngineType::Semantic,
            indexing_strategy: IndexingStrategy::Vector,
        }
    }
}

impl ModelCompression {
    pub fn new() -> Self {
        Self {
            compression_algorithms: HashMap::new(),
            compression_statistics: CompressionStatistics::new(),
            quality_metrics: CompressionQualityMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        // Register the standard set of compression algorithms.
        self.register_algorithm("QuantizationInt8", CompressionAlgorithm::Quantization);
        self.register_algorithm("QuantizationFP16", CompressionAlgorithm::Quantization);
        self.register_algorithm("Pruning", CompressionAlgorithm::Pruning);
        self.register_algorithm("Distillation", CompressionAlgorithm::KnowledgeDistillation);
        Ok(())
    }

    /// Register a compression algorithm under the given name.
    pub fn register_algorithm(&mut self, name: &str, algorithm: CompressionAlgorithm) {
        self.compression_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered compression algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&CompressionAlgorithm> {
        self.compression_algorithms.get(name)
    }

    /// List the names of all registered compression algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.compression_algorithms.keys().cloned().collect()
    }

    /// Number of bytes needed for a packed one-bit-per-weight pruning mask.
    pub const fn pruning_mask_bytes(weight_count: usize) -> usize {
        weight_count.div_ceil(PRUNING_MASK_BITS_PER_BYTE)
    }

    /// Return whether `weight_index` is present in a packed pruning mask.
    pub fn mask_keeps(mask: &[u8], weight_index: usize) -> bool {
        let byte = weight_index / PRUNING_MASK_BITS_PER_BYTE;
        let bit = weight_index % PRUNING_MASK_BITS_PER_BYTE;
        mask.get(byte)
            .map(|value| (value & (1u8 << bit)) != 0)
            .unwrap_or(false)
    }

    fn set_mask_bit(mask: &mut [u8], weight_index: usize) {
        let byte = weight_index / PRUNING_MASK_BITS_PER_BYTE;
        let bit = weight_index % PRUNING_MASK_BITS_PER_BYTE;
        mask[byte] |= 1u8 << bit;
    }

    /// Run per-tensor symmetric signed-int8 post-training quantization.
    ///
    /// `out` is caller-owned and receives the complete compressed payload.
    /// The returned scale is the only side metadata required to dequantize it.
    pub fn quantize_symmetric_int8_into(
        &mut self,
        weights: &[f64],
        out: &mut [i8],
    ) -> Result<QuantizationReport, MLError> {
        if weights.is_empty() {
            return Err(MLError::ValidationError(
                "cannot quantize an empty weight tensor".to_string(),
            ));
        }
        if out.len() < weights.len() {
            return Err(MLError::ResourceError(format!(
                "int8 output buffer needs {} elements, got {}",
                weights.len(),
                out.len()
            )));
        }

        let mut max_abs = 0.0f64;
        let mut signal_sq = 0.0f64;
        for &weight in weights {
            if !weight.is_finite() {
                return Err(MLError::ValidationError(
                    "quantization input contains a non-finite weight".to_string(),
                ));
            }
            max_abs = max_abs.max(weight.abs());
            signal_sq += weight * weight;
        }

        let scale = if max_abs == 0.0 {
            1.0
        } else {
            max_abs / i8::MAX as f64
        };
        let mut squared_error = 0.0f64;
        let mut max_abs_error = 0.0f64;
        for (index, &weight) in weights.iter().enumerate() {
            let quantized = (weight / scale)
                .round()
                .clamp(-(i8::MAX as f64), i8::MAX as f64) as i8;
            out[index] = quantized;
            let error = weight - quantized as f64 * scale;
            squared_error += error * error;
            max_abs_error = max_abs_error.max(error.abs());
        }

        let rmse = (squared_error / weights.len() as f64).sqrt();
        let signal_rms = (signal_sq / weights.len() as f64).sqrt();
        let preservation = if signal_rms == 0.0 {
            1.0
        } else {
            (1.0 - rmse / signal_rms).clamp(0.0, 1.0)
        };
        let original_bytes = std::mem::size_of_val(weights);
        // Portable payload: one byte per weight plus the f64 scale.
        let compressed_bytes = weights.len() + std::mem::size_of::<f64>();
        let report = QuantizationReport {
            parameters: QuantizationParameters {
                scheme: QuantizationScheme::SymmetricInt8,
                scale,
                zero_point: 0,
            },
            element_count: weights.len(),
            original_bytes,
            compressed_bytes,
            compression_ratio: original_bytes as f64 / compressed_bytes as f64,
            rmse,
            max_abs_error,
        };
        self.record_measured_compression(original_bytes, compressed_bytes, preservation);
        Ok(report)
    }

    /// Dequantize a symmetric-int8 tensor into caller-owned floating-point storage.
    pub fn dequantize_symmetric_int8_into(
        quantized: &[i8],
        parameters: QuantizationParameters,
        out: &mut [f64],
    ) -> Result<usize, MLError> {
        if parameters.scheme != QuantizationScheme::SymmetricInt8
            || !parameters.scale.is_finite()
            || parameters.scale <= 0.0
            || parameters.zero_point != 0
        {
            return Err(MLError::ValidationError(
                "invalid symmetric-int8 quantization parameters".to_string(),
            ));
        }
        if out.len() < quantized.len() {
            return Err(MLError::ResourceError(format!(
                "dequantization output needs {} elements, got {}",
                quantized.len(),
                out.len()
            )));
        }
        for (dst, &value) in out.iter_mut().zip(quantized.iter()) {
            *dst = value as f64 * parameters.scale;
        }
        Ok(quantized.len())
    }

    /// Exact unstructured magnitude pruning.
    ///
    /// The smallest-magnitude weights are removed. The result is a real sparse
    /// representation: a packed keep-mask and the retained values in original
    /// index order. `scratch_indices` is caller-provided sorting workspace.
    pub fn prune_unstructured_into(
        &mut self,
        weights: &[f64],
        sparsity: f64,
        mask_out: &mut [u8],
        values_out: &mut [f64],
        scratch_indices: &mut [usize],
    ) -> Result<PruningReport, MLError> {
        Self::validate_pruning_input(weights, sparsity)?;
        let count = weights.len();
        let mask_bytes = Self::pruning_mask_bytes(count);
        let pruned = ((count as f64 * sparsity).round() as usize).min(count);
        let kept = count - pruned;
        if mask_out.len() < mask_bytes {
            return Err(MLError::ResourceError(format!(
                "pruning mask needs {} bytes, got {}",
                mask_bytes,
                mask_out.len()
            )));
        }
        if values_out.len() < kept {
            return Err(MLError::ResourceError(format!(
                "sparse value buffer needs {} elements, got {}",
                kept,
                values_out.len()
            )));
        }
        if scratch_indices.len() < count {
            return Err(MLError::ResourceError(format!(
                "pruning scratch needs {} indices, got {}",
                count,
                scratch_indices.len()
            )));
        }

        mask_out[..mask_bytes].fill(0);
        for (index, slot) in scratch_indices[..count].iter_mut().enumerate() {
            *slot = index;
        }
        scratch_indices[..count].sort_unstable_by(|left, right| {
            weights[*left]
                .abs()
                .total_cmp(&weights[*right].abs())
                .then_with(|| left.cmp(right))
        });
        for &index in &scratch_indices[pruned..count] {
            Self::set_mask_bit(mask_out, index);
        }

        let mut write = 0usize;
        let mut original_energy = 0.0f64;
        let mut kept_energy = 0.0f64;
        for (index, &weight) in weights.iter().enumerate() {
            original_energy += weight * weight;
            if Self::mask_keeps(mask_out, index) {
                values_out[write] = weight;
                write += 1;
                kept_energy += weight * weight;
            }
        }

        let report = Self::make_pruning_report(
            count,
            pruned,
            count,
            pruned,
            sparsity,
            mask_bytes,
            original_energy,
            kept_energy,
        );
        self.record_measured_compression(
            report.original_bytes,
            report.compressed_bytes,
            report.l2_energy_preserved.sqrt(),
        );
        Ok(report)
    }

    /// Structured output-channel pruning for a row-major `rows × columns` matrix.
    ///
    /// Entire rows with the smallest L2 norm are removed and packed contiguously.
    /// `row_mask_out` contains one keep bit per row.
    pub fn prune_output_channels_into(
        &mut self,
        weights: &[f64],
        rows: usize,
        columns: usize,
        sparsity: f64,
        row_mask_out: &mut [u8],
        values_out: &mut [f64],
        score_scratch: &mut [f64],
        index_scratch: &mut [usize],
    ) -> Result<PruningReport, MLError> {
        Self::validate_pruning_input(weights, sparsity)?;
        if rows == 0 || columns == 0 || rows.checked_mul(columns) != Some(weights.len()) {
            return Err(MLError::ValidationError(format!(
                "structured pruning shape {}x{} does not match {} weights",
                rows,
                columns,
                weights.len()
            )));
        }
        let mask_bytes = Self::pruning_mask_bytes(rows);
        let pruned_rows = ((rows as f64 * sparsity).round() as usize).min(rows);
        let kept_rows = rows - pruned_rows;
        let kept_weights = kept_rows * columns;
        if row_mask_out.len() < mask_bytes
            || score_scratch.len() < rows
            || index_scratch.len() < rows
            || values_out.len() < kept_weights
        {
            return Err(MLError::ResourceError(
                "structured-pruning caller buffers are too small".to_string(),
            ));
        }

        row_mask_out[..mask_bytes].fill(0);
        let mut original_energy = 0.0f64;
        for row in 0..rows {
            let mut row_energy = 0.0f64;
            for &weight in &weights[row * columns..(row + 1) * columns] {
                row_energy += weight * weight;
            }
            score_scratch[row] = row_energy;
            index_scratch[row] = row;
            original_energy += row_energy;
        }
        index_scratch[..rows].sort_unstable_by(|left, right| {
            score_scratch[*left]
                .total_cmp(&score_scratch[*right])
                .then_with(|| left.cmp(right))
        });
        for &row in &index_scratch[pruned_rows..rows] {
            Self::set_mask_bit(row_mask_out, row);
        }

        let mut write = 0usize;
        let mut kept_energy = 0.0f64;
        for row in 0..rows {
            if Self::mask_keeps(row_mask_out, row) {
                let source = &weights[row * columns..(row + 1) * columns];
                values_out[write..write + columns].copy_from_slice(source);
                write += columns;
                kept_energy += score_scratch[row];
            }
        }

        let report = Self::make_pruning_report(
            weights.len(),
            pruned_rows * columns,
            rows,
            pruned_rows,
            sparsity,
            mask_bytes,
            original_energy,
            kept_energy,
        );
        self.record_measured_compression(
            report.original_bytes,
            report.compressed_bytes,
            report.l2_energy_preserved.sqrt(),
        );
        Ok(report)
    }

    /// Reconstruct an unstructured sparse tensor into caller-owned dense storage.
    pub fn unpack_pruned_weights_into(
        mask: &[u8],
        packed_values: &[f64],
        out: &mut [f64],
    ) -> Result<usize, MLError> {
        let needed_mask = Self::pruning_mask_bytes(out.len());
        if mask.len() < needed_mask {
            return Err(MLError::ResourceError(format!(
                "pruning mask needs {} bytes, got {}",
                needed_mask,
                mask.len()
            )));
        }
        let kept = (0..out.len())
            .filter(|&index| Self::mask_keeps(mask, index))
            .count();
        if packed_values.len() < kept {
            return Err(MLError::ResourceError(format!(
                "packed sparse tensor needs {} values, got {}",
                kept,
                packed_values.len()
            )));
        }

        let mut read = 0usize;
        for (index, value) in out.iter_mut().enumerate() {
            if Self::mask_keeps(mask, index) {
                *value = packed_values[read];
                read += 1;
            } else {
                *value = 0.0;
            }
        }
        Ok(read)
    }

    /// Distil any inference-supported teacher MLP into the existing single-linear
    /// SGD student. Teacher outputs are generated from real forward passes and
    /// optionally blended with hard targets in `target_buffer`.
    pub fn distill_linear_student(
        &mut self,
        training_engine: &mut TrainingEngine,
        teacher: &Model,
        student: &mut Model,
        training_data: &[f64],
        hard_targets: Option<&[f64]>,
        distillation: DistillationConfig,
        training: &TrainingConfig,
        target_buffer: &mut [f64],
    ) -> Result<DistillationReport, MLError> {
        if !distillation.teacher_weight.is_finite()
            || !(0.0..=1.0).contains(&distillation.teacher_weight)
        {
            return Err(MLError::ValidationError(
                "teacher_weight must be finite and in [0, 1]".to_string(),
            ));
        }
        let input_size = student
            .architecture
            .input_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("student input shape is empty".into()))?;
        let output_size = student
            .architecture
            .output_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("student output shape is empty".into()))?;
        if input_size == 0 || training_data.is_empty() || training_data.len() % input_size != 0 {
            return Err(MLError::DataError(
                "distillation training data has an invalid shape".to_string(),
            ));
        }
        if teacher.architecture.input_shape.first().copied() != Some(input_size)
            || teacher.architecture.output_shape.first().copied() != Some(output_size)
        {
            return Err(MLError::ValidationError(
                "teacher and student input/output shapes must match".to_string(),
            ));
        }
        let samples = training_data.len() / input_size;
        let target_count = samples * output_size;
        if target_buffer.len() < target_count {
            return Err(MLError::ResourceError(format!(
                "distillation target buffer needs {} elements, got {}",
                target_count,
                target_buffer.len()
            )));
        }
        if let Some(targets) = hard_targets {
            if targets.len() != target_count {
                return Err(MLError::DataError(format!(
                    "hard target length {} does not match {}",
                    targets.len(),
                    target_count
                )));
            }
        } else if distillation.teacher_weight < 1.0 {
            return Err(MLError::ValidationError(
                "hard targets are required when teacher_weight is below 1".to_string(),
            ));
        }

        for sample in 0..samples {
            let input = &training_data[sample * input_size..(sample + 1) * input_size];
            let teacher_output = InferenceEngine::forward_pass(teacher, input)?;
            if teacher_output.len() != output_size {
                return Err(MLError::ValidationError(
                    "teacher produced an unexpected output shape".to_string(),
                ));
            }
            for output in 0..output_size {
                let index = sample * output_size + output;
                let hard = hard_targets.map(|targets| targets[index]).unwrap_or(0.0);
                target_buffer[index] = distillation.teacher_weight * teacher_output[output]
                    + (1.0 - distillation.teacher_weight) * hard;
            }
        }

        let fidelity_mse_before =
            Self::teacher_student_fidelity_mse(teacher, student, training_data)?;
        let training_result = training_engine.start_training(
            student,
            training_data,
            &target_buffer[..target_count],
            training,
        )?;
        let fidelity_mse_after =
            Self::teacher_student_fidelity_mse(teacher, student, training_data)?;

        let teacher_bytes = std::mem::size_of_val(teacher.weights.as_slice());
        let student_bytes = std::mem::size_of_val(student.weights.as_slice());
        let compression_ratio = teacher_bytes as f64 / student_bytes.max(1) as f64;
        self.record_measured_compression(
            teacher_bytes,
            student_bytes,
            (1.0 / (1.0 + fidelity_mse_after.sqrt())).clamp(0.0, 1.0),
        );
        Ok(DistillationReport {
            teacher_parameters: teacher.weights.len(),
            student_parameters: student.weights.len(),
            compression_ratio,
            fidelity_mse_before,
            fidelity_mse_after,
            training: training_result,
        })
    }

    fn teacher_student_fidelity_mse(
        teacher: &Model,
        student: &Model,
        training_data: &[f64],
    ) -> Result<f64, MLError> {
        let input_size = student.architecture.input_shape[0];
        let samples = training_data.len() / input_size;
        let mut squared_error = 0.0f64;
        let mut outputs = 0usize;
        for sample in 0..samples {
            let input = &training_data[sample * input_size..(sample + 1) * input_size];
            let teacher_output = InferenceEngine::forward_pass(teacher, input)?;
            let student_output = InferenceEngine::forward_pass(student, input)?;
            if teacher_output.len() != student_output.len() {
                return Err(MLError::ValidationError(
                    "teacher and student output lengths differ".to_string(),
                ));
            }
            for (&teacher_value, &student_value) in teacher_output.iter().zip(student_output.iter())
            {
                let difference = teacher_value - student_value;
                squared_error += difference * difference;
                outputs += 1;
            }
        }
        Ok(if outputs == 0 {
            0.0
        } else {
            squared_error / outputs as f64
        })
    }

    fn validate_pruning_input(weights: &[f64], sparsity: f64) -> Result<(), MLError> {
        if weights.is_empty() {
            return Err(MLError::ValidationError(
                "cannot prune an empty weight tensor".to_string(),
            ));
        }
        if !sparsity.is_finite() || !(0.0..=1.0).contains(&sparsity) {
            return Err(MLError::ValidationError(
                "sparsity must be finite and in [0, 1]".to_string(),
            ));
        }
        if weights.iter().any(|weight| !weight.is_finite()) {
            return Err(MLError::ValidationError(
                "pruning input contains a non-finite weight".to_string(),
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn make_pruning_report(
        total_weights: usize,
        pruned_weights: usize,
        total_units: usize,
        pruned_units: usize,
        requested_sparsity: f64,
        mask_bytes: usize,
        original_energy: f64,
        kept_energy: f64,
    ) -> PruningReport {
        let kept_weights = total_weights - pruned_weights;
        let original_bytes = total_weights * std::mem::size_of::<f64>();
        let compressed_bytes = mask_bytes + kept_weights * std::mem::size_of::<f64>();
        PruningReport {
            total_weights,
            pruned_weights,
            kept_weights,
            total_units,
            pruned_units,
            requested_sparsity,
            achieved_sparsity: pruned_weights as f64 / total_weights as f64,
            original_bytes,
            compressed_bytes,
            compression_ratio: original_bytes as f64 / compressed_bytes.max(1) as f64,
            l2_energy_preserved: if original_energy == 0.0 {
                1.0
            } else {
                kept_energy / original_energy
            },
        }
    }

    fn record_measured_compression(
        &mut self,
        original_bytes: usize,
        compressed_bytes: usize,
        preservation: f64,
    ) {
        self.compression_statistics.original_size = original_bytes as u64;
        self.compression_statistics.compressed_size = compressed_bytes as u64;
        self.compression_statistics.compression_ratio =
            original_bytes as f64 / compressed_bytes.max(1) as f64;

        let count = self.quality_metrics.compression_count;
        let next = count + 1;
        let ratio = self.compression_statistics.compression_ratio;
        let reduction =
            (1.0 - compressed_bytes as f64 / original_bytes.max(1) as f64).clamp(0.0, 1.0);
        self.quality_metrics.compression_count = next;
        self.quality_metrics.compression_ratio =
            (self.quality_metrics.compression_ratio * count as f64 + ratio) / next as f64;
        self.quality_metrics.size_reduction =
            (self.quality_metrics.size_reduction * count as f64 + reduction) / next as f64;
        self.quality_metrics.memory_savings = self.quality_metrics.size_reduction;
        self.quality_metrics.accuracy_preservation = (self.quality_metrics.accuracy_preservation
            * count as f64
            + preservation.clamp(0.0, 1.0))
            / next as f64;
    }

    /// Record the result of a compression operation and update the aggregate
    /// quality metrics.
    pub fn record_compression(
        &mut self,
        algorithm_name: &str,
        original_size: usize,
        compressed_size: usize,
        accuracy_before: f64,
        accuracy_after: f64,
    ) -> Result<(), MLError> {
        if !self.compression_algorithms.contains_key(algorithm_name) {
            return Err(MLError::OptimizationError(format!(
                "unknown compression algorithm '{}'",
                algorithm_name
            )));
        }
        if original_size == 0 {
            return Err(MLError::ValidationError(
                "original_size must be greater than zero".to_string(),
            ));
        }

        let ratio = original_size as f64 / compressed_size.max(1) as f64;
        let size_reduction = 1.0 - (compressed_size as f64 / original_size as f64);
        let accuracy_preservation = if accuracy_before > 0.0 {
            (accuracy_after / accuracy_before).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Update the running aggregate statistics.
        let count = self.quality_metrics.compression_count;
        let prev_ratio = self.quality_metrics.compression_ratio;
        let prev_reduction = self.quality_metrics.size_reduction;
        let prev_accuracy = self.quality_metrics.accuracy_preservation;

        let new_count = count + 1;
        self.quality_metrics.compression_count = new_count;
        // Running average across all recorded compressions.
        self.quality_metrics.compression_ratio =
            (prev_ratio * count as f64 + ratio) / new_count as f64;
        self.quality_metrics.size_reduction =
            (prev_reduction * count as f64 + size_reduction) / new_count as f64;
        self.quality_metrics.accuracy_preservation =
            (prev_accuracy * count as f64 + accuracy_preservation) / new_count as f64;
        // Memory savings mirror the size reduction for this simple wiring.
        self.quality_metrics.memory_savings = self.quality_metrics.size_reduction;

        Ok(())
    }

    /// Access the aggregate compression quality metrics.
    pub fn get_quality_metrics(&self) -> &CompressionQualityMetrics {
        &self.quality_metrics
    }

    /// Access byte counts and the ratio from the most recent real compression.
    pub fn get_compression_statistics(&self) -> &CompressionStatistics {
        &self.compression_statistics
    }

    /// Return the overall compression ratio recorded so far.
    pub fn compression_ratio(&self) -> f64 {
        self.quality_metrics.compression_ratio
    }
}

impl CompressionStatistics {
    pub fn new() -> Self {
        Self {
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 0.0,
            compression_time: 0,
            decompression_time: 0,
        }
    }
}

impl CompressionQualityMetrics {
    pub fn new() -> Self {
        Self {
            accuracy_preservation: 0.0,
            performance_impact: 0.0,
            memory_savings: 0.0,
            compression_ratio: 0.0,
            size_reduction: 0.0,
            compression_count: 0,
        }
    }
}
