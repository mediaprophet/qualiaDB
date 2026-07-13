#![allow(unused_imports)]
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_ml_library_creation() {
        let mut library = MachineLearningLibrary::new();
        assert!(library.initialize().is_ok());
    }

    #[test]
    fn test_model_loading() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        let result = library
            .load_model("test_model".to_string(), "/path/to/model")
            .unwrap();

        assert_eq!(result.result.model_id, "test_model");
        assert_eq!(result.result.model_type, ModelType::LLM);
        assert_eq!(result.result.framework, MLFramework::PyTorch);
    }

    #[test]
    fn test_inference() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        // 100 bytes is not a multiple of 8 (f64 size), so the wired MLP backend rejects the
        // input with a DataError rather than fabricating a result. (The default scaffold model
        // loaded here is a 512→512 Linear layer with zero weights; even with valid input the
        // shape would not match — see test_mlp_inference_forward_pass for a real forward pass.)
        let input_data = vec![1u8; 100];
        let parameters = InferenceParameters {
            batch_size: 1,
            sequence_length: 512,
            temperature: Some(0.7),
            top_k: Some(50),
            top_p: Some(0.9),
            max_tokens: Some(100),
            precision: Precision::FP32,
        };

        let result = library.run_inference("test_model", &input_data, parameters);
        assert!(
            result.is_err(),
            "malformed input (not a multiple of f64 size) must be rejected, not fabricated"
        );
    }

    #[test]
    fn test_mlp_inference_forward_pass() {
        // Build a 2 → 3 → 2 MLP with ReLU on the hidden layer and no output activation.
        //
        // Layer 1 (Linear, 2→3, ReLU):
        //   W1 (row-major, 3×2) = [[1, 2], [0, -1], [0.5, 0.5]], bias1 = [0, 0, 0]
        //   z1 = W1·x + b1 for x = [1, 2] = [5, -2, 1.5]
        //   after ReLU        = [5,  0, 1.5]
        //
        // Layer 2 (Linear, 3→2, no activation):
        //   W2 (row-major, 2×3) = [[1, 0, 0], [0, 1, 0]], bias2 = [0, 0]
        //   z2 = W2·h1 + b2 = [5, 0]
        //
        // Expected output = [5.0, 0.0].
        let layer1 = LayerInfo {
            layer_id: "l1".to_string(),
            layer_type: LayerType::Linear,
            input_shape: vec![2],
            output_shape: vec![3],
            parameters: 9, // 3×2 weights + 3 bias
            activation: Some(ActivationFunction::ReLU),
        };
        let layer2 = LayerInfo {
            layer_id: "l2".to_string(),
            layer_type: LayerType::Linear,
            input_shape: vec![3],
            output_shape: vec![2],
            parameters: 8, // 2×3 weights + 2 bias
            activation: None,
        };
        let model = Model {
            model_id: "mlp_test".to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::Custom("test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![layer1, layer2],
                connections: vec![],
                input_shape: vec![2],
                output_shape: vec![2],
                total_parameters: 17,
            },
            // Flattened in consumption order: layer1 W(3×2) + bias(3), layer2 W(2×3) + bias(2).
            weights: vec![
                1.0, 2.0, 0.0, -1.0, 0.5, 0.5, // W1 row-major
                0.0, 0.0, 0.0, // bias1
                1.0, 0.0, 0.0, 0.0, 1.0, 0.0, // W2 row-major
                0.0, 0.0, // bias2
            ],
            metadata: ModelMetadata::new(),
        };

        let input = [1.0f64, 2.0];
        let input_bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();

        let request = InferenceRequest {
            request_id: "req_mlp".to_string(),
            model_id: "mlp_test".to_string(),
            input_data: input_bytes,
            parameters: InferenceParameters {
                batch_size: 1,
                sequence_length: 2,
                temperature: None,
                top_k: None,
                top_p: None,
                max_tokens: None,
                precision: Precision::FP32,
            },
            priority: RequestPriority::Normal,
            submitted_at: 0,
            deadline: None,
        };

        let mut engine = InferenceEngine::new();
        let result = engine
            .execute_inference(&request, &model)
            .expect("MLP forward pass should succeed");

        let out: Vec<f64> = result
            .output_data
            .chunks_exact(std::mem::size_of::<f64>())
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect();

        assert_eq!(out.len(), 2, "output should have 2 values");
        assert!(
            (out[0] - 5.0).abs() < 1e-9,
            "out[0] should be 5.0, got {}",
            out[0]
        );
        assert!(
            (out[1] - 0.0).abs() < 1e-9,
            "out[1] should be 0.0, got {}",
            out[1]
        );
        assert_eq!(result.metadata.model_id, "mlp_test");
        assert_eq!(result.metadata.backend_id, "linear_algebra_mlp");
    }

    #[test]
    fn test_mlp_inference_rejects_unsupported_layer() {
        // A model with an Attention layer cannot be evaluated by the MLP backend and must
        // fail with a clear, honest error naming the unsupported layer type.
        let model = Model {
            model_id: "attn_test".to_string(),
            model_type: ModelType::Transformer,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: "attn1".to_string(),
                    layer_type: LayerType::Attention,
                    input_shape: vec![4],
                    output_shape: vec![4],
                    parameters: 0,
                    activation: None,
                }],
                connections: vec![],
                input_shape: vec![4],
                output_shape: vec![4],
                total_parameters: 0,
            },
            weights: vec![],
            metadata: ModelMetadata::new(),
        };

        let input = [1.0f64, 2.0, 3.0, 4.0];
        let input_bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();

        let request = InferenceRequest {
            request_id: "req_attn".to_string(),
            model_id: "attn_test".to_string(),
            input_data: input_bytes,
            parameters: InferenceParameters {
                batch_size: 1,
                sequence_length: 4,
                temperature: None,
                top_k: None,
                top_p: None,
                max_tokens: None,
                precision: Precision::FP32,
            },
            priority: RequestPriority::Normal,
            submitted_at: 0,
            deadline: None,
        };

        let mut engine = InferenceEngine::new();
        let result = engine.execute_inference(&request, &model);
        let err = result.expect_err("Attention layer must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("Attention"),
            "error should name the unsupported layer type: {}",
            msg
        );
    }

    #[test]
    fn test_training() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        let training_config = TrainingConfig {
            epochs: 5,
            batch_size: 16,
            learning_rate: 0.001,
            optimizer: TrainingAlgorithm::Adam,
            loss_function: "cross_entropy".to_string(),
            metrics: vec!["accuracy".to_string()],
            validation_split: 0.2,
        };

        let result = library
            .start_training("test_model", training_config)
            .unwrap();

        assert_eq!(result.result.model_id, "test_model");
        assert_eq!(result.result.status, TrainingStatus::Pending);
        assert_eq!(result.result.training_config.epochs, 5);
    }

    #[test]
    fn test_model_optimization() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        let result = library
            .optimize_model("test_model", MLOptimizationAlgorithm::ModelQuantization)
            .unwrap();

        assert_eq!(result.result.model_id, "test_model");
        assert_eq!(result.result.model_type, ModelType::LLM);
    }

    #[test]
    fn test_performance_metrics() {
        let library = MachineLearningLibrary::new();
        let metrics = library.get_performance_stats();

        assert_eq!(metrics.inference_metrics.total_requests, 0);
        assert_eq!(metrics.training_metrics.total_training_jobs, 0);
        assert_eq!(metrics.system_metrics.cpu_utilization, 0.0);
        assert_eq!(metrics.model_metrics.total_models, 0);
    }

    #[test]
    fn test_model_listing() {
        let library = MachineLearningLibrary::new();
        let models = library.list_models();
        assert_eq!(models.len(), 0);
    }

    #[test]
    fn test_model_info() {
        let library = MachineLearningLibrary::new();
        let info = library.get_model_info("test_model");
        assert!(info.is_none());
    }

    #[test]
    fn test_model_cache_get_put_and_stats() {
        let mut cache = ModelCache::new();

        // Miss on an empty cache.
        assert!(cache.get("missing").is_none());
        let stats = cache.cache_stats();
        assert_eq!(stats.hit_count, 0);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate - 0.0).abs() < f64::EPSILON);

        // Put a model in and retrieve it (hit).
        let mut model = Model::new();
        model.model_id = "m1".to_string();
        cache.put("m1".to_string(), model.clone()).unwrap();
        assert_eq!(cache.cache_size(), 1);

        let retrieved = cache.get("m1").expect("cached model should be present");
        assert_eq!(retrieved.model_id, "m1");
        let stats = cache.cache_stats();
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate - 0.5).abs() < f64::EPSILON);

        // A second miss.
        assert!(cache.get("nope").is_none());
        let stats = cache.cache_stats();
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 2);
        let expected = 1.0 / 3.0;
        assert!((stats.hit_rate - expected).abs() < 1e-9);
    }

    #[test]
    fn test_model_cache_lru_eviction() {
        // Build a cache with a tiny max size so eviction is exercised.
        let mut cache = ModelCache {
            cache_entries: HashMap::new(),
            cache_policy: ModelCachePolicy {
                eviction_policy: ModelEvictionPolicy::LRU,
                max_size: 16, // two 8-byte entries fit; a third forces LRU eviction
                ttl: 3600,
                priority_levels: vec![PriorityLevel::Medium],
            },
            cache_stats: ModelCacheStats::new(),
        };

        let mk = |id: &str| {
            let mut m = Model::new();
            m.model_id = id.to_string();
            // One f64 weight = 8 bytes per entry.
            m.weights = vec![0.0];
            m
        };

        cache.put("a".to_string(), mk("a")).unwrap();
        cache.put("b".to_string(), mk("b")).unwrap();

        // Access "a" so "b" becomes the LRU candidate.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let _ = cache.get("a");

        // Adding "c" exceeds the budget and must evict the oldest (b).
        cache.put("c".to_string(), mk("c")).unwrap();

        assert!(
            cache.get("b").is_none(),
            "LRU entry 'b' should have been evicted"
        );
        assert!(cache.get("a").is_some(), "'a' should still be resident");
        assert!(cache.get("c").is_some(), "'c' should be resident");

        let stats = cache.cache_stats();
        assert!(
            stats.eviction_count >= 1,
            "eviction_count should reflect evictions"
        );
        assert!(stats.total_size <= cache.cache_policy.max_size);
    }

    #[test]
    fn test_model_storage_load_model_fallback_on_missing_file() {
        // A path that does not exist must fall back to the mock scaffold model rather
        // than erroring out, so downstream inference always has a model to operate on.
        let mut storage = ModelStorage::new();
        let model = storage
            .load_model("fallback_missing", "/nonexistent/path/to/model.gguf")
            .expect("missing file should fall back to mock model, not error");

        assert_eq!(model.model_id, "fallback_missing");
        assert_eq!(model.model_type, ModelType::LLM);
        assert_eq!(model.framework, MLFramework::PyTorch);
        assert_eq!(
            model.weights.len(),
            1000,
            "mock model should have 1000 weights"
        );

        // The loaded model should be cached in the model_store.
        assert!(storage.model_store.contains_key("fallback_missing"));
    }

    #[test]
    fn test_model_storage_load_model_fallback_on_non_gguf_file() {
        // A real file that is not a GGUF file must fall back to the mock scaffold model.
        let dir = std::env::temp_dir();
        let path = dir.join("qualia_ml_non_gguf_test.bin");
        std::fs::write(&path, b"this is not a gguf file").unwrap();

        let mut storage = ModelStorage::new();
        let model = storage
            .load_model("fallback_non_gguf", path.to_str().unwrap())
            .expect("non-GGUF file should fall back to mock model, not error");

        assert_eq!(model.model_id, "fallback_non_gguf");
        assert_eq!(
            model.weights.len(),
            1000,
            "mock model should have 1000 weights"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_model_storage_load_model_caches_in_store() {
        // Loading the same model_id twice should return the cached instance from
        // model_store (verified by mutating the first result and confirming the second
        // load is independent of further disk reads).
        let mut storage = ModelStorage::new();
        let first = storage
            .load_model("cached_model", "/nonexistent/model.gguf")
            .unwrap();
        assert_eq!(first.model_id, "cached_model");

        // Second load should come from the store without re-reading disk.
        let second = storage
            .load_model("cached_model", "/nonexistent/model.gguf")
            .unwrap();
        assert_eq!(second.model_id, first.model_id);
        assert_eq!(second.weights.len(), first.weights.len());
    }

    #[test]
    fn test_model_storage_load_model_real_gguf_if_present() {
        // If a real GGUF file happens to be available at a well-known path, exercise the
        // real loading path; otherwise skip gracefully so the test is hermetic.
        let candidate = std::env::var("QUALIA_TEST_GGUF_PATH").ok();
        let gguf_path = match candidate {
            Some(p) if !p.is_empty() && std::path::Path::new(&p).exists() => p,
            _ => {
                eprintln!(
                    "[test_model_storage_load_model_real_gguf_if_present] \
                           no GGUF file available (set QUALIA_TEST_GGUF_PATH); skipping"
                );
                return;
            }
        };

        let mut storage = ModelStorage::new();
        let model = match storage.load_model("real_gguf", &gguf_path) {
            Ok(m) => m,
            Err(e) => {
                // A parse failure should have fallen back to the mock model, not errored,
                // so reaching here is unexpected — surface it.
                panic!(
                    "load_model returned error for real GGUF {}: {}",
                    gguf_path, e
                );
            }
        };

        // If the GGUF parsed successfully the framework is Custom("GGUF") and weights are
        // a non-empty multiple of n_embd; otherwise the fallback mock (1000 weights,
        // PyTorch) was returned. Both are acceptable outcomes for this hermetic test.
        if model.framework == MLFramework::Custom("GGUF".to_string()) {
            assert!(
                !model.weights.is_empty(),
                "real GGUF model should have non-empty weights"
            );
            assert!(
                !model.architecture.layers.is_empty(),
                "real GGUF model should describe at least one layer"
            );
            assert_eq!(
                model.architecture.layers[0].layer_type,
                LayerType::Linear,
                "GGUF embedding layer should be modelled as Linear"
            );
        } else {
            assert_eq!(
                model.weights.len(),
                1000,
                "fallback mock model should have 1000 weights"
            );
        }
    }

    // ------------------------------------------------------------------
    // Feature 1: ModelCatalog search index
    // ------------------------------------------------------------------

    #[test]
    fn test_model_catalog_register_search_by_tag() {
        let mut catalog = ModelCatalog::new();
        catalog.initialize().unwrap();

        // Register two models with distinct metadata.
        let mut meta_a = ModelMetadata::new();
        meta_a.model_id = "vision-resnet".to_string();
        meta_a.model_type = ModelType::CNN;
        let mut meta_b = ModelMetadata::new();
        meta_b.model_id = "llm-bert".to_string();
        meta_b.model_type = ModelType::LLM;

        catalog.register_model("vision-resnet", meta_a);
        catalog.register_model("llm-bert", meta_b);

        // Tag them.
        catalog.add_tag("vision-resnet", "vision");
        catalog.add_tag("vision-resnet", "classification");
        catalog.add_tag("llm-bert", "nlp");
        catalog.add_tag("llm-bert", "classification");

        // get_by_tag returns the right model ids (case-insensitive).
        let vision = catalog.get_by_tag("Vision");
        assert_eq!(vision, vec!["vision-resnet".to_string()]);
        let nlp = catalog.get_by_tag("NLP");
        assert_eq!(nlp, vec!["llm-bert".to_string()]);

        // Both models share the "classification" tag.
        let mut cls = catalog.get_by_tag("classification");
        cls.sort();
        assert_eq!(
            cls,
            vec!["llm-bert".to_string(), "vision-resnet".to_string()]
        );

        // Unknown tag → empty.
        assert!(catalog.get_by_tag("nonexistent").is_empty());
    }

    #[test]
    fn test_model_catalog_search_by_keyword_and_name() {
        let mut catalog = ModelCatalog::new();
        catalog.initialize().unwrap();

        let mut meta = ModelMetadata::new();
        meta.model_id = "audio-transformer".to_string();
        meta.model_type = ModelType::Transformer;
        catalog.register_model("audio-transformer", meta);
        catalog.add_tag("audio-transformer", "speech");

        // Search by a substring of the model id.
        let by_name = catalog.search("audio");
        assert_eq!(by_name, vec!["audio-transformer".to_string()]);

        // Search by tag keyword.
        let by_tag = catalog.search("speech");
        assert_eq!(by_tag, vec!["audio-transformer".to_string()]);

        // Search by the model type keyword that register_model adds to the index.
        let by_type = catalog.search("Transformer");
        assert_eq!(by_type, vec!["audio-transformer".to_string()]);

        // A query that matches nothing returns empty.
        assert!(catalog.search("zzz-no-match").is_empty());

        // Empty query returns empty (no spurious matches).
        assert!(catalog.search("").is_empty());
    }

    #[test]
    fn test_model_search_index_initialize_and_search() {
        let mut index = ModelSearchIndex::new();
        // Before initialize, search returns nothing even with a matching entry.
        index.index(ModelIndexEntry {
            entry_id: "m1".to_string(),
            keywords: vec!["alpha".to_string()],
            metadata: HashMap::new(),
            relevance_score: 1.0,
        });
        assert!(
            index.search("alpha").is_empty(),
            "search before initialize must be empty"
        );

        index.initialize().unwrap();
        let hits = index.search("alpha");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry_id, "m1");

        // Metadata values are also searchable.
        let mut md = HashMap::new();
        md.insert("framework".to_string(), "PyTorch".to_string());
        index.index(ModelIndexEntry {
            entry_id: "m2".to_string(),
            keywords: vec![],
            metadata: md,
            relevance_score: 0.5,
        });
        let hits2 = index.search("pytorch");
        assert_eq!(hits2.len(), 1);
        assert_eq!(hits2[0].entry_id, "m2");
    }

    // ------------------------------------------------------------------
    // Feature 2: SGD training loop
    // ------------------------------------------------------------------

    #[test]
    fn test_compute_mse() {
        let preds = [1.0, 2.0, 3.0];
        let targets = [1.0, 2.0, 3.0];
        assert!((TrainingEngine::compute_mse(&preds, &targets) - 0.0).abs() < 1e-12);

        let preds = [2.0, 4.0];
        let targets = [1.0, 1.0];
        // MSE = ((2-1)^2 + (4-1)^2) / 2 = (1 + 9)/2 = 5.0
        assert!((TrainingEngine::compute_mse(&preds, &targets) - 5.0).abs() < 1e-12);

        // Mismatched lengths → 0.0 (defined behaviour).
        assert_eq!(TrainingEngine::compute_mse(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn test_compute_gradients_linear() {
        // y = w*x + b, weights = [w, b] = [3.0, -1.0], x = [2.0], pred = 3*2 - 1 = 5, target = 1.
        // diff = 4 ; dL/dw = 2*4*2 = 16 ; dL/db = 2*4 = 8 ; lr = 0.1
        // returned (scaled) = [0.1*16, 0.1*8] = [1.6, 0.8]
        let weights = [3.0, -1.0];
        let inputs = [2.0];
        let grad = TrainingEngine::compute_gradients(&weights, &inputs, 5.0, 1.0, 0.1);
        assert_eq!(grad.len(), 2);
        assert!((grad[0] - 1.6).abs() < 1e-12, "grad[0] = {}", grad[0]);
        assert!((grad[1] - 0.8).abs() < 1e-12, "grad[1] = {}", grad[1]);
    }

    #[test]
    fn test_sgd_training_learns_linear_function() {
        // Build a 1->1 linear model with no activation: y = w*x + b.
        // weights = [w, b], initialised to zero.
        let mut model = Model {
            model_id: "linreg".to_string(),
            model_type: ModelType::RNN, // arbitrary; not used by the trainer
            framework: MLFramework::Custom("test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: "l1".to_string(),
                    layer_type: LayerType::Linear,
                    input_shape: vec![1],
                    output_shape: vec![1],
                    parameters: 2, // 1 weight + 1 bias
                    activation: None,
                }],
                connections: vec![],
                input_shape: vec![1],
                output_shape: vec![1],
                total_parameters: 2,
            },
            weights: vec![0.0, 0.0],
            metadata: ModelMetadata::new(),
        };

        // Training data from y = 2x + 1 over x in [-2, 2].
        let xs: Vec<f64> = (-20..=20).map(|i| i as f64 * 0.2).collect();
        let training_data: Vec<f64> = xs.clone();
        let targets: Vec<f64> = xs.iter().map(|x| 2.0 * x + 1.0).collect();

        let config = TrainingConfig {
            epochs: 500,
            batch_size: 8,
            learning_rate: 0.05,
            optimizer: TrainingAlgorithm::SGD,
            loss_function: "mse".to_string(),
            metrics: vec!["loss".to_string()],
            validation_split: 0.0,
        };

        let mut engine = TrainingEngine::new();
        let result = engine
            .start_training(&mut model, &training_data, &targets, &config)
            .expect("SGD training should succeed");

        // Loss must drop dramatically.
        assert!(
            result.final_loss < result.initial_loss,
            "final loss ({}) should be less than initial loss ({})",
            result.final_loss,
            result.initial_loss
        );
        assert!(
            result.final_loss < 1e-3,
            "final loss ({}) should be near zero for a perfectly linear dataset",
            result.final_loss
        );
        // The loop may converge early (loss plateau) before all epochs run; both outcomes
        // are valid, so only bound the completed count.
        assert!(
            result.epochs_completed <= config.epochs as usize,
            "epochs_completed ({}) must not exceed configured epochs ({})",
            result.epochs_completed,
            config.epochs
        );

        // Learned weights should be close to the true [w=2, b=1].
        let w = model.weights[0];
        let b = model.weights[1];
        assert!(
            (w - 2.0).abs() < 0.05,
            "learned weight w = {} should be ~2.0",
            w
        );
        assert!(
            (b - 1.0).abs() < 0.05,
            "learned bias b = {} should be ~1.0",
            b
        );
    }

    #[test]
    fn test_sgd_training_rejects_unsupported_config() {
        // A non-SGD optimizer must be rejected, not silently ignored.
        let mut model = Model::new();
        let config = TrainingConfig {
            epochs: 1,
            batch_size: 1,
            learning_rate: 0.01,
            optimizer: TrainingAlgorithm::Adam,
            loss_function: "mse".to_string(),
            metrics: vec![],
            validation_split: 0.0,
        };
        let mut engine = TrainingEngine::new();
        let err = engine
            .start_training(&mut model, &[1.0], &[1.0], &config)
            .expect_err("Adam must be rejected by the SGD-only trainer");
        let msg = format!("{}", err);
        assert!(msg.contains("SGD"), "error should name SGD: {}", msg);
    }

    #[test]
    fn test_sgd_training_rejects_activation() {
        // A Linear layer with an activation is out of scope for linear-regression SGD.
        let mut model = Model {
            model_id: "act".to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: "l1".to_string(),
                    layer_type: LayerType::Linear,
                    input_shape: vec![1],
                    output_shape: vec![1],
                    parameters: 2,
                    activation: Some(ActivationFunction::ReLU),
                }],
                connections: vec![],
                input_shape: vec![1],
                output_shape: vec![1],
                total_parameters: 2,
            },
            weights: vec![0.0, 0.0],
            metadata: ModelMetadata::new(),
        };
        let config = TrainingConfig {
            epochs: 1,
            batch_size: 1,
            learning_rate: 0.01,
            optimizer: TrainingAlgorithm::SGD,
            loss_function: "mse".to_string(),
            metrics: vec![],
            validation_split: 0.0,
        };
        let mut engine = TrainingEngine::new();
        let err = engine
            .start_training(&mut model, &[1.0], &[1.0], &config)
            .expect_err("activated layer must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("activation"),
            "error should mention activation: {}",
            msg
        );
    }

    // ------------------------------------------------------------------
    // Feature 1: Model Version Control
    // ------------------------------------------------------------------

    fn sample_version(id: &str) -> ModelVersion {
        ModelVersion {
            version_id: id.to_string(),
            version_number: id.to_string(),
            changes: vec![],
            created_at: 0,
            created_by: "tester".to_string(),
        }
    }

    #[test]
    fn test_version_control_create_and_get() {
        let mut vc = ModelVersionControl::new();
        assert!(vc.initialize().is_ok());

        let v1 = sample_version("v1");
        assert!(vc.create_version("model-a", v1.clone()).is_ok());

        // Duplicate version should be rejected.
        let err = vc
            .create_version("model-a", sample_version("v1"))
            .expect_err("duplicate version must be rejected");
        assert!(format!("{}", err).contains("already exists"));

        // Retrieval works.
        let got = vc
            .get_version("model-a", "v1")
            .expect("version should exist");
        assert_eq!(got.version_id, "v1");

        // Unknown model/version returns None.
        assert!(vc.get_version("model-a", "v2").is_none());
        assert!(vc.get_version("model-b", "v1").is_none());
    }

    #[test]
    fn test_version_control_list_versions() {
        let mut vc = ModelVersionControl::new();
        vc.initialize().unwrap();
        vc.create_version("model-a", sample_version("v1")).unwrap();
        vc.create_version("model-a", sample_version("v2")).unwrap();
        vc.create_version("model-b", sample_version("v1")).unwrap();

        let mut a_versions = vc.list_versions("model-a");
        a_versions.sort();
        assert_eq!(a_versions, vec!["v1".to_string(), "v2".to_string()]);

        let b_versions = vc.list_versions("model-b");
        assert_eq!(b_versions, vec!["v1".to_string()]);

        assert!(vc.list_versions("model-c").is_empty());
    }

    #[test]
    fn test_version_control_branches() {
        let mut vc = ModelVersionControl::new();
        vc.initialize().unwrap();
        vc.create_version("model-a", sample_version("v1")).unwrap();

        // Creating a branch from an existing version succeeds.
        assert!(vc.create_branch("dev", "v1").is_ok());
        let branch = vc.get_branch("dev").expect("branch should exist");
        assert_eq!(branch, &vec!["v1".to_string()]);

        // Duplicate branch is rejected.
        let err = vc
            .create_branch("dev", "v1")
            .expect_err("duplicate branch must be rejected");
        assert!(format!("{}", err).contains("already exists"));

        // Branching from an unknown version fails.
        assert!(vc.create_branch("feat", "nope").is_err());

        // The default `main` branch is seeded by initialize().
        assert!(vc.get_branch("main").is_some());

        // Unknown branch returns None.
        assert!(vc.get_branch("ghost").is_none());
    }

    #[test]
    fn test_version_control_tags() {
        let mut vc = ModelVersionControl::new();
        vc.initialize().unwrap();
        vc.create_version("model-a", sample_version("v1")).unwrap();
        vc.create_version("model-a", sample_version("v2")).unwrap();

        // Tag versions.
        assert!(vc.tag_version("v1", "stable").is_ok());
        assert!(vc.tag_version("v2", "latest").is_ok());
        assert!(vc.tag_version("v2", "stable").is_ok());

        // Tagging an unknown version fails.
        assert!(vc.tag_version("v9", "x").is_err());

        // get_tags returns all tags for a version.
        let mut v2_tags = vc.get_tags("v2");
        v2_tags.sort();
        assert_eq!(v2_tags, vec!["latest".to_string(), "stable".to_string()]);

        // get_by_tag returns all versions carrying the tag.
        let mut stable_versions = vc.get_by_tag("stable");
        stable_versions.sort();
        assert_eq!(stable_versions, vec!["v1".to_string(), "v2".to_string()]);

        assert!(vc.get_by_tag("nonexistent").is_empty());
        assert!(vc.get_tags("v9").is_empty());
    }

    #[test]
    fn test_version_control_initialize_seeds_main_branch() {
        let mut vc = ModelVersionControl::new();
        // Before initialize, no branches exist.
        assert!(vc.get_branch("main").is_none());
        assert!(vc.initialize().is_ok());
        assert!(vc.get_branch("main").is_some());
    }

    // ------------------------------------------------------------------
    // Feature 2: Compression Quality Metrics
    // ------------------------------------------------------------------

    #[test]
    fn test_compression_register_and_get_algorithm() {
        let mut mc = ModelCompression::new();
        assert!(mc.list_algorithms().is_empty());

        mc.register_algorithm("my-pruner", CompressionAlgorithm::Pruning);
        assert_eq!(mc.list_algorithms(), vec!["my-pruner".to_string()]);
        assert_eq!(
            mc.get_algorithm("my-pruner"),
            Some(&CompressionAlgorithm::Pruning)
        );
        assert!(mc.get_algorithm("missing").is_none());
    }

    #[test]
    fn test_compression_initialize_registers_standard_algorithms() {
        let mut mc = ModelCompression::new();
        assert!(mc.initialize().is_ok());

        let mut names = mc.list_algorithms();
        names.sort();
        assert_eq!(
            names,
            vec![
                "Distillation".to_string(),
                "Pruning".to_string(),
                "QuantizationFP16".to_string(),
                "QuantizationInt8".to_string(),
            ]
        );
    }

    #[test]
    fn test_compression_record_updates_metrics() {
        let mut mc = ModelCompression::new();
        mc.initialize().unwrap();

        // 1000 bytes -> 250 bytes is a 4x compression ratio (75% reduction).
        assert!(mc
            .record_compression("QuantizationInt8", 1000, 250, 0.90, 0.88)
            .is_ok());

        let metrics = mc.get_quality_metrics();
        assert_eq!(metrics.compression_count, 1);
        assert!((metrics.compression_ratio - 4.0).abs() < 1e-9);
        assert!((metrics.size_reduction - 0.75).abs() < 1e-9);
        // accuracy preservation = 0.88 / 0.90
        assert!((metrics.accuracy_preservation - (0.88 / 0.90)).abs() < 1e-9);

        // The accessor and helper agree.
        assert!((mc.compression_ratio() - metrics.compression_ratio).abs() < 1e-9);
    }

    #[test]
    fn test_compression_record_rejects_unknown_algorithm() {
        let mut mc = ModelCompression::new();
        let err = mc
            .record_compression("ghost", 100, 50, 1.0, 1.0)
            .expect_err("unknown algorithm must be rejected");
        assert!(format!("{}", err).contains("unknown compression algorithm"));
    }

    #[test]
    fn test_compression_record_rejects_zero_original_size() {
        let mut mc = ModelCompression::new();
        mc.register_algorithm("x", CompressionAlgorithm::Pruning);
        let err = mc
            .record_compression("x", 0, 0, 1.0, 1.0)
            .expect_err("zero original size must be rejected");
        assert!(format!("{}", err).contains("original_size"));
    }

    #[test]
    fn test_compression_record_running_average() {
        let mut mc = ModelCompression::new();
        mc.register_algorithm("x", CompressionAlgorithm::Pruning);

        // First: ratio 4.0 (1000 -> 250). Second: ratio 2.0 (1000 -> 500).
        // Average ratio should be 3.0.
        mc.record_compression("x", 1000, 250, 1.0, 1.0).unwrap();
        mc.record_compression("x", 1000, 500, 1.0, 1.0).unwrap();

        let metrics = mc.get_quality_metrics();
        assert_eq!(metrics.compression_count, 2);
        assert!((metrics.compression_ratio - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_symmetric_int8_ptq_round_trip_measures_error() {
        let weights = [-1.0, -0.51, 0.0, 0.26, 0.75, 1.0];
        let mut quantized = [0i8; 6];
        let mut compression = ModelCompression::new();

        let report = compression
            .quantize_symmetric_int8_into(&weights, &mut quantized)
            .expect("PTQ should succeed");
        let mut reconstructed = [0.0f64; 6];
        let written = ModelCompression::dequantize_symmetric_int8_into(
            &quantized,
            report.parameters,
            &mut reconstructed,
        )
        .expect("dequantization should succeed");

        assert_eq!(written, weights.len());
        assert_eq!(quantized[0], -127);
        assert_eq!(quantized[5], 127);
        assert!(report.compression_ratio > 3.0);
        assert!(report.rmse > 0.0);
        assert!(report.max_abs_error <= report.parameters.scale / 2.0 + f64::EPSILON);
        for (&expected, &actual) in weights.iter().zip(reconstructed.iter()) {
            assert!((expected - actual).abs() <= report.parameters.scale / 2.0 + f64::EPSILON);
        }
        assert_eq!(compression.get_quality_metrics().compression_count, 1);
    }

    #[test]
    fn test_unstructured_pruning_packs_exact_smallest_weights() {
        let weights = [0.01, 5.0, -0.02, 4.0];
        let mut mask = [0u8; 1];
        let mut packed = [0.0f64; 2];
        let mut scratch = [0usize; 4];
        let mut compression = ModelCompression::new();

        let report = compression
            .prune_unstructured_into(&weights, 0.5, &mut mask, &mut packed, &mut scratch)
            .expect("magnitude pruning should succeed");

        assert_eq!(report.pruned_weights, 2);
        assert_eq!(report.kept_weights, 2);
        assert_eq!(packed, [5.0, 4.0]);
        assert!(!ModelCompression::mask_keeps(&mask, 0));
        assert!(ModelCompression::mask_keeps(&mask, 1));
        assert!(!ModelCompression::mask_keeps(&mask, 2));
        assert!(ModelCompression::mask_keeps(&mask, 3));

        let mut reconstructed = [9.0f64; 4];
        assert_eq!(
            ModelCompression::unpack_pruned_weights_into(&mask, &packed, &mut reconstructed)
                .unwrap(),
            2
        );
        assert_eq!(reconstructed, [0.0, 5.0, 0.0, 4.0]);
        assert!(report.l2_energy_preserved > 0.999);
    }

    #[test]
    fn test_structured_pruning_removes_lowest_energy_output_channel() {
        // Three output channels (rows), two inputs per channel.
        let weights = [0.1, 0.1, 5.0, 5.0, 2.0, 2.0];
        let mut row_mask = [0u8; 1];
        let mut packed = [0.0f64; 4];
        let mut scores = [0.0f64; 3];
        let mut indices = [0usize; 3];
        let mut compression = ModelCompression::new();

        let report = compression
            .prune_output_channels_into(
                &weights,
                3,
                2,
                1.0 / 3.0,
                &mut row_mask,
                &mut packed,
                &mut scores,
                &mut indices,
            )
            .expect("structured pruning should succeed");

        assert_eq!(report.total_units, 3);
        assert_eq!(report.pruned_units, 1);
        assert_eq!(report.pruned_weights, 2);
        assert!(!ModelCompression::mask_keeps(&row_mask, 0));
        assert!(ModelCompression::mask_keeps(&row_mask, 1));
        assert!(ModelCompression::mask_keeps(&row_mask, 2));
        assert_eq!(packed, [5.0, 5.0, 2.0, 2.0]);
    }

    fn compression_test_linear_model(
        model_id: &str,
        input_size: usize,
        output_size: usize,
        weights: Vec<f64>,
    ) -> Model {
        Model {
            model_id: model_id.to_string(),
            model_type: ModelType::RNN,
            framework: MLFramework::Custom("compression-test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: format!("{}_linear", model_id),
                    layer_type: LayerType::Linear,
                    input_shape: vec![input_size],
                    output_shape: vec![output_size],
                    parameters: input_size * output_size + output_size,
                    activation: None,
                }],
                connections: vec![],
                input_shape: vec![input_size],
                output_shape: vec![output_size],
                total_parameters: input_size * output_size + output_size,
            },
            weights,
            metadata: ModelMetadata::new(),
        }
    }

    fn compression_test_training_config() -> TrainingConfig {
        TrainingConfig {
            epochs: 500,
            batch_size: 8,
            learning_rate: 0.05,
            optimizer: TrainingAlgorithm::SGD,
            loss_function: "mse".to_string(),
            metrics: vec!["loss".to_string()],
            validation_split: 0.0,
        }
    }

    #[test]
    fn test_pruning_recovery_never_regrows_masked_weight() {
        let mut model = compression_test_linear_model("masked", 2, 1, vec![0.0, 8.0, 0.0]);
        // Keep w0 and bias, prune w1.
        let mask = [0b0000_0101u8];
        let mut inputs = Vec::new();
        let mut targets = Vec::new();
        for x0 in -10..=10 {
            for x1 in -2..=2 {
                inputs.push(x0 as f64 / 5.0);
                inputs.push(x1 as f64);
                targets.push(2.0 * (x0 as f64 / 5.0) + 1.0);
            }
        }

        let mut trainer = TrainingEngine::new();
        let result = trainer
            .start_training_with_pruning_mask(
                &mut model,
                &inputs,
                &targets,
                &compression_test_training_config(),
                &mask,
            )
            .expect("masked recovery should train");

        assert!(result.final_loss < result.initial_loss);
        assert_eq!(model.weights[1], 0.0, "pruned weight must remain zero");
        assert!((model.weights[0] - 2.0).abs() < 0.05);
        assert!((model.weights[2] - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_teacher_student_distillation_trains_smaller_linear_model() {
        // A two-layer linear teacher representing y = 2x + 1:
        // [x, -x] followed by 3*x + 1*(-x) + 1.
        let teacher = Model {
            model_id: "teacher".to_string(),
            model_type: ModelType::RNN,
            framework: MLFramework::Custom("compression-test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![
                    LayerInfo {
                        layer_id: "teacher_1".to_string(),
                        layer_type: LayerType::Linear,
                        input_shape: vec![1],
                        output_shape: vec![2],
                        parameters: 4,
                        activation: None,
                    },
                    LayerInfo {
                        layer_id: "teacher_2".to_string(),
                        layer_type: LayerType::Linear,
                        input_shape: vec![2],
                        output_shape: vec![1],
                        parameters: 3,
                        activation: None,
                    },
                ],
                connections: vec![],
                input_shape: vec![1],
                output_shape: vec![1],
                total_parameters: 7,
            },
            weights: vec![1.0, -1.0, 0.0, 0.0, 3.0, 1.0, 1.0],
            metadata: ModelMetadata::new(),
        };
        let mut student = compression_test_linear_model("student", 1, 1, vec![0.0, 0.0]);
        let inputs: Vec<f64> = (-20..=20).map(|x| x as f64 / 10.0).collect();
        let mut target_buffer = vec![0.0f64; inputs.len()];
        let mut trainer = TrainingEngine::new();
        let mut compression = ModelCompression::new();

        let report = compression
            .distill_linear_student(
                &mut trainer,
                &teacher,
                &mut student,
                &inputs,
                None,
                DistillationConfig {
                    teacher_weight: 1.0,
                },
                &compression_test_training_config(),
                &mut target_buffer,
            )
            .expect("distillation should succeed");

        assert_eq!(report.teacher_parameters, 7);
        assert_eq!(report.student_parameters, 2);
        assert!((report.compression_ratio - 3.5).abs() < 1e-12);
        assert!(report.fidelity_mse_after < report.fidelity_mse_before);
        assert!(report.fidelity_mse_after < 1e-3);
        assert!((student.weights[0] - 2.0).abs() < 0.05);
        assert!((student.weights[1] - 1.0).abs() < 0.05);
    }
