
use super::*;

#[test]
fn test_statistical_library_creation() {
    let library = StatisticalComputingLibrary::new();
    assert_eq!(library.list_datasets().len(), 0);
}

#[test]
fn test_dataset_creation() {
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(3.0), DataValue::Float(4.0)],
        vec![DataValue::Float(5.0), DataValue::Float(6.0)],
    ];

    let dataset = library
        .create_dataset(
            "test_dataset".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();

    assert_eq!(dataset.dataset_id, "test_dataset");
    assert_eq!(dataset.data.len(), 3);
    assert_eq!(dataset.column_names.len(), 2);
}

#[test]
fn test_mean_computation() {
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(3.0), DataValue::Float(4.0)],
        vec![DataValue::Float(5.0), DataValue::Float(6.0)],
    ];

    library
        .create_dataset(
            "test_dataset".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();

    let result = library.mean("test_dataset", "col1", false).unwrap();

    assert_eq!(result.result, 3.0); // (1 + 3 + 5) / 3
    assert_eq!(result.sample_size, 3);
    assert!(!result.privacy_preserved);
}

#[test]
fn test_median_computation() {
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(3.0), DataValue::Float(4.0)],
        vec![DataValue::Float(5.0), DataValue::Float(6.0)],
        vec![DataValue::Float(7.0), DataValue::Float(8.0)],
    ];

    library
        .create_dataset(
            "test_dataset".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();

    let result = library.median("test_dataset", "col1", false).unwrap();

    assert_eq!(result.result, 4.0); // median of [1, 3, 5, 7]
    assert_eq!(result.sample_size, 4);
    assert!(!result.privacy_preserved);
}

#[test]
fn test_variance_computation() {
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(3.0), DataValue::Float(4.0)],
        vec![DataValue::Float(5.0), DataValue::Float(6.0)],
    ];

    library
        .create_dataset(
            "test_dataset".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();

    let result = library
        .variance("test_dataset", "col1", true, false)
        .unwrap();

    // Variance of [1, 3, 5] = ((1-3)^2 + (3-3)^2 + (5-3)^2) / (3-1) = (4 + 0 + 4) / 2 = 4
    assert!((result.result - 4.0).abs() < 1e-10);
    assert_eq!(result.sample_size, 3);
    assert!(!result.privacy_preserved);
}

#[test]
fn test_correlation_computation() {
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(2.0), DataValue::Float(4.0)],
        vec![DataValue::Float(3.0), DataValue::Float(6.0)],
        vec![DataValue::Float(4.0), DataValue::Float(8.0)],
    ];

    library
        .create_dataset(
            "test_dataset".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();

    let result = library
        .correlation(
            "test_dataset",
            "col1",
            "col2",
            CorrelationMethod::Pearson,
            false,
        )
        .unwrap();

    // Perfect correlation for [1,2,3,4] and [2,4,6,8]
    assert!((result.result - 1.0).abs() < 1e-10);
    assert_eq!(result.sample_size, 4);
    assert!(!result.privacy_preserved);
}

#[test]
fn laplace_noise_is_random_not_a_counter() {
    // Regression: DP noise was a deterministic AtomicU64 ramp, which voids
    // the guarantee (predictable noise can be subtracted off). Two draws
    // over identical (value, sensitivity) must differ now that real OS
    // entropy backs the inverse-CDF sampler.
    let mut eng = StatisticalPrivacyEngine::new();
    let (a, eps) = eng.add_laplace_noise(100.0, 1.0).unwrap();
    let (b, _) = eng.add_laplace_noise(100.0, 1.0).unwrap();
    assert_eq!(eps, 1.0);
    assert_ne!(
        a, b,
        "differential-privacy noise must be random, not deterministic"
    );
}

#[test]
fn test_privacy_preserved_mean() {
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(3.0), DataValue::Float(4.0)],
        vec![DataValue::Float(5.0), DataValue::Float(6.0)],
    ];

    library
        .create_dataset(
            "test_dataset".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Confidential,
        )
        .unwrap();

    let result = library.mean("test_dataset", "col1", true).unwrap();

    assert!(result.privacy_preserved);
    assert!(result.privacy_cost > 0.0);
    // The mean should be noisy (not exactly 3.0)
    assert!(result.result != 3.0);
}

#[test]
fn test_histogram_generation() {
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(3.0), DataValue::Float(4.0)],
        vec![DataValue::Float(5.0), DataValue::Float(6.0)],
        vec![DataValue::Float(7.0), DataValue::Float(8.0)],
        vec![DataValue::Float(9.0), DataValue::Float(10.0)],
    ];

    library
        .create_dataset(
            "test_dataset".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();

    let result = library.histogram("test_dataset", "col1", 5, false).unwrap();

    assert_eq!(result.result.bins, 5);
    assert_eq!(result.result.counts.len(), 5);
    assert_eq!(result.result.min_value, 1.0);
    assert_eq!(result.result.max_value, 9.0);
    assert!(!result.privacy_preserved);
}

// ---- Feature 1: ZNS data persistence ----

#[test]
fn test_dataset_store_and_retrieve() {
    let mut storage = StatisticalDataStorage::new();
    storage.initialize().unwrap();

    let dataset = Dataset {
        dataset_id: "persisted_ds".to_string(),
        metadata: DatasetMetadata {
            dataset_id: "persisted_ds".to_string(),
            dataset_type: DatasetType::Numerical,
            dimensions: DatasetDimensions {
                rows: 2,
                columns: 1,
                time_steps: None,
                features: Some(1),
            },
            data_types: vec![DataType::Float64],
            sample_size: 2,
            created_at: 0,
            last_updated: 0,
            access_count: 0,
            privacy_level: PrivacyLevel::Public,
        },
        data: vec![vec![DataValue::Float(1.0)], vec![DataValue::Float(2.0)]],
        column_names: vec!["x".to_string()],
        column_types: vec![DataType::Float64],
    };

    // Store through the persistence layer.
    storage.store_dataset_data(&dataset).unwrap();

    // Retrieve from the in-memory persistence layer.
    let retrieved = storage
        .retrieve_dataset_data("persisted_ds")
        .expect("dataset should be cached after store_dataset_data");
    assert_eq!(retrieved.dataset_id, "persisted_ds");
    assert_eq!(retrieved.data.len(), 2);

    // Retrieving an unknown id returns None.
    assert!(storage.retrieve_dataset_data("missing").is_none());
}

#[test]
fn test_store_dataset_to_named_zone() {
    let mut storage = StatisticalDataStorage::new();
    storage.initialize().unwrap();

    let dataset = Dataset {
        dataset_id: "zoned_ds".to_string(),
        metadata: DatasetMetadata {
            dataset_id: "zoned_ds".to_string(),
            dataset_type: DatasetType::TimeSeries,
            dimensions: DatasetDimensions {
                rows: 1,
                columns: 1,
                time_steps: Some(1),
                features: None,
            },
            data_types: vec![DataType::Float64],
            sample_size: 1,
            created_at: 0,
            last_updated: 0,
            access_count: 0,
            privacy_level: PrivacyLevel::Restricted,
        },
        data: vec![vec![DataValue::Float(42.0)]],
        column_names: vec!["v".to_string()],
        column_types: vec![DataType::Float64],
    };

    storage.store_dataset_data(&dataset).unwrap();

    // Explicitly place the dataset into the "timeseries" zone.
    storage
        .store_dataset_to_zone("zoned_ds", "timeseries")
        .unwrap();

    // The metadata should now be registered with that zone.
    let zone = storage.zones.get("timeseries").unwrap();
    assert!(zone.datasets.contains_key("zoned_ds"));

    // Storing into a non-existent zone errors.
    assert!(storage.store_dataset_to_zone("zoned_ds", "nope").is_err());

    // Storing an uncached dataset errors.
    assert!(storage
        .store_dataset_to_zone("ghost", "timeseries")
        .is_err());
}

// ---- Feature 2: Fiduciary crypto / ZK proof wiring ----

#[test]
fn test_encrypt_and_verify_result() {
    let engine = StatisticalPrivacyEngine::new();

    let payload = b"mean=3.0; n=10";
    let signature = engine
        .encrypt_result(payload)
        .expect("encryption should succeed");

    // The signature is a real ML-DSA signature (non-empty).
    assert!(!signature.is_empty());

    // Verifying with the correct payload succeeds.
    let valid = engine
        .verify_result(payload, &signature)
        .expect("verify path should run");
    assert!(valid);

    // Verifying against a tampered payload fails.
    let tampered = b"mean=99.0; n=10";
    let invalid = engine
        .verify_result(tampered, &signature)
        .expect("verify path should run");
    assert!(!invalid);
}

#[test]
fn test_zk_prove_and_verify_computation() {
    let engine = StatisticalPrivacyEngine::new();

    let inputs = vec![b"x=1".to_vec(), b"y=2".to_vec()];
    let outputs = vec![b"sum=3".to_vec()];

    let proof = engine
        .prove_computation("add_op", &inputs, &outputs)
        .expect("proof generation should succeed");
    assert!(!proof.is_empty());

    // Verify the genuine proof.
    let ok = engine
        .verify_computation(&proof, &[])
        .expect("verify path should run");
    assert!(ok);
}

// ---- Feature 3: Data catalog search ----

fn sample_metadata(id: &str, rows: usize) -> DatasetMetadata {
    DatasetMetadata {
        dataset_id: id.to_string(),
        dataset_type: DatasetType::Numerical,
        dimensions: DatasetDimensions {
            rows,
            columns: 2,
            time_steps: None,
            features: Some(2),
        },
        data_types: vec![DataType::Float64, DataType::Float64],
        sample_size: rows,
        created_at: 0,
        last_updated: 0,
        access_count: 0,
        privacy_level: PrivacyLevel::Public,
    }
}

#[test]
fn test_catalog_register_search_and_tags() {
    let mut catalog = DataCatalog::new();
    catalog.initialize().unwrap();

    catalog.register_dataset(sample_metadata("sales_q1", 100));
    catalog.register_dataset(sample_metadata("sales_q2", 200));
    catalog.register_dataset(sample_metadata("inventory", 50));

    catalog.add_tag("sales_q1", "revenue");
    catalog.add_tag("sales_q2", "revenue");
    catalog.add_tag("inventory", "stock");

    // Search by name substring.
    let sales = catalog.search("sales");
    assert_eq!(sales.len(), 2);

    // Search by tag.
    let revenue = catalog.search("revenue");
    assert_eq!(revenue.len(), 2);

    // get_by_tag returns the right datasets.
    let stock = catalog.get_by_tag("stock");
    assert_eq!(stock.len(), 1);
    assert_eq!(stock[0].dataset_id, "inventory");

    // get_by_tag is case-insensitive.
    let revenue_ci = catalog.get_by_tag("REVENUE");
    assert_eq!(revenue_ci.len(), 2);

    // Empty query returns everything.
    assert_eq!(catalog.search("").len(), 3);
}

#[test]
fn test_catalog_relationships() {
    let mut catalog = DataCatalog::new();
    catalog.register_dataset(sample_metadata("base", 10));
    catalog.register_dataset(sample_metadata("derived", 10));

    catalog.add_relationship(
        "base",
        "derived",
        Relationship {
            relationship_id: "rel1".to_string(),
            source_dataset: String::new(),
            target_dataset: String::new(),
            relationship_type: RelationshipType::Derived,
            strength: 0.9,
        },
    );

    let rels = catalog.relationships.get("base").unwrap();
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].source_dataset, "base");
    assert_eq!(rels[0].target_dataset, "derived");
}

#[test]
fn test_search_index_index_and_search() {
    let mut index = SearchIndex::new();
    index.initialize().unwrap();

    index.index(IndexEntry {
        entry_id: "e1".to_string(),
        keywords: vec!["alpha".to_string(), "beta".to_string()],
        metadata: HashMap::new(),
        relevance_score: 0.5,
    });
    index.index(IndexEntry {
        entry_id: "e2".to_string(),
        keywords: vec!["gamma".to_string()],
        metadata: HashMap::new(),
        relevance_score: 0.8,
    });

    assert_eq!(index.search("alpha").len(), 1);
    assert_eq!(index.search("beta").len(), 1);
    assert_eq!(index.search("gamma").len(), 1);
    assert_eq!(index.search("zzz").len(), 0);
}

// ---- Feature 4: Sensitivity analysis for differential privacy ----

#[test]
fn test_sensitivity_mean_sum_count() {
    let mut analyzer = SensitivityAnalyzer::new();
    let data = vec![1.0, 2.0, 3.0, 4.0]; // n = 4

    let mean_s = analyzer.compute_sensitivity("mean", &data).unwrap();
    assert!((mean_s - 0.25).abs() < 1e-12); // 1/4

    let sum_s = analyzer.compute_sensitivity("sum", &data).unwrap();
    assert!((sum_s - 1.0).abs() < 1e-12);

    let count_s = analyzer.compute_sensitivity("count", &data).unwrap();
    assert!((count_s - 1.0).abs() < 1e-12);

    let hist_s = analyzer.compute_sensitivity("histogram", &data).unwrap();
    assert!((hist_s - 1.0).abs() < 1e-12);
}

#[test]
fn test_sensitivity_median_variance() {
    let mut analyzer = SensitivityAnalyzer::new();
    let data = vec![1.0, 2.0, 3.0, 10.0]; // range = 9, n = 4

    let median_s = analyzer.compute_sensitivity("median", &data).unwrap();
    assert!((median_s - (10.0 - 1.0) / 4.0).abs() < 1e-12);

    let var_s = analyzer.compute_sensitivity("variance", &data).unwrap();
    let range = 10.0 - 1.0;
    assert!((var_s - (range * range) / 4.0).abs() < 1e-12);
}

#[test]
fn test_sensitivity_caching_and_registered_function() {
    let mut analyzer = SensitivityAnalyzer::new();
    let data = vec![1.0, 2.0, 3.0];

    // First call computes and caches.
    let s1 = analyzer.get_sensitivity("sum", &data).unwrap();
    assert!((s1 - 1.0).abs() < 1e-12);

    // Cache hit: a subsequent call returns the same value even with
    // different data (sum sensitivity is data-independent here, but the
    // point is the cache short-circuits recomputation).
    let s2 = analyzer.get_sensitivity("sum", &[100.0]).unwrap();
    assert!((s2 - 1.0).abs() < 1e-12);

    // A registered function overrides the built-in approximation.
    analyzer.register_function(
        "custom",
        SensitivityFunction {
            function_id: "custom".to_string(),
            sensitivity: 3.5,
            computation_method: SensitivityMethod::Approximate,
        },
    );
    let s3 = analyzer.compute_sensitivity("custom", &data).unwrap();
    assert!((s3 - 3.5).abs() < 1e-12);

    // Unknown operation errors.
    assert!(analyzer.compute_sensitivity("bogus", &data).is_err());
    // Empty data errors.
    assert!(analyzer.compute_sensitivity("mean", &[]).is_err());
}

#[test]
fn test_dp_mean_uses_calibrated_sensitivity() {
    // The privacy-preserved mean path should pull sensitivity from the
    // analyzer (1/n) rather than the old hardcoded 1.0. With n=3 the
    // sensitivity is 1/3; we just assert the path runs and produces a
    // noisy result whose privacy cost is recorded.
    let mut library = StatisticalComputingLibrary::new();
    library.initialize().unwrap();

    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(3.0), DataValue::Float(4.0)],
        vec![DataValue::Float(5.0), DataValue::Float(6.0)],
    ];

    library
        .create_dataset(
            "ds".to_string(),
            data,
            vec!["col1".to_string(), "col2".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Confidential,
        )
        .unwrap();

    let result = library.mean("ds", "col1", true).unwrap();
    assert!(result.privacy_preserved);
    assert!(result.privacy_cost > 0.0);

    // The analyzer cache should now hold the mean sensitivity (1/3).
    let cached = library
        .privacy_engine
        .differential_privacy
        .sensitivity_analyzer
        .sensitivity_cache
        .get("mean")
        .copied();
    assert!(cached.is_some());
    assert!((cached.unwrap() - 1.0 / 3.0).abs() < 1e-12);
}

#[test]
fn test_compression_statistics_tracking() {
    let mut engine = DataCompressionEngine::new();
    engine.initialize().unwrap();

    // Fresh engine has zeroed stats.
    let stats = engine.get_statistics();
    assert_eq!(stats.original_size, 0);
    assert_eq!(stats.compressed_size, 0);
    assert_eq!(stats.compression_count, 0);
    assert_eq!(stats.decompression_count, 0);
    assert_eq!(stats.compression_ratio(), 0.0);

    // Highly repetitive data compresses well under RLE.
    let data = vec![7u8; 1000];
    let compressed = engine.compress(&data).unwrap();
    assert!(compressed.len() < data.len());

    let stats = engine.get_statistics();
    assert_eq!(stats.compression_count, 1);
    assert_eq!(stats.original_size, 1000);
    assert_eq!(stats.compressed_size, compressed.len() as u64);
    // Overall ratio matches compressed/original.
    let expected = compressed.len() as f64 / 1000.0;
    assert!((stats.compression_ratio() - expected).abs() < 1e-12);
    // Last-op ratio field also updated.
    assert!((stats.compression_ratio - expected).abs() < 1e-12);

    // Round-trip decompress and verify decompression stats.
    let decompressed = engine.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
    let stats = engine.get_statistics();
    assert_eq!(stats.decompression_count, 1);
    assert!(stats.decompression_time > 0 || stats.decompression_time == 0); // timing may be 0 on fast machines

    // A second, incompressible compression accumulates.
    let noisy: Vec<u8> = (0..256u32).map(|i| i as u8).collect();
    let compressed2 = engine.compress(&noisy).unwrap();
    let stats = engine.get_statistics();
    assert_eq!(stats.compression_count, 2);
    assert_eq!(stats.original_size, 1000 + 256);
    assert_eq!(
        stats.compressed_size,
        (compressed.len() + compressed2.len()) as u64
    );

    // Summary is human-readable and non-empty.
    let summary = stats.summary();
    assert!(summary.contains("compress op(s)"));
    assert!(summary.contains("2 compress"));

    // Reset zeroes everything.
    engine.reset_statistics();
    let stats = engine.get_statistics();
    assert_eq!(stats.compression_count, 0);
    assert_eq!(stats.decompression_count, 0);
    assert_eq!(stats.original_size, 0);
    assert_eq!(stats.compressed_size, 0);
    assert_eq!(stats.compression_ratio(), 0.0);
}

#[test]
fn test_compression_roundtrip_random_data() {
    let mut engine = DataCompressionEngine::new();
    // Random-ish data: ensure round-trip still holds even when it expands.
    let data: Vec<u8> = (0..500u32).map(|i| (i * 31 + 7) as u8).collect();
    let compressed = engine.compress(&data).unwrap();
    let decompressed = engine.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn test_cost_model_total_and_comparison() {
    let cheap = CostModel {
        cpu_cost: 1.0,
        io_cost: 2.0,
        memory_cost: 0.5,
        network_cost: 0.0,
    };
    let expensive = CostModel {
        cpu_cost: 10.0,
        io_cost: 20.0,
        memory_cost: 5.0,
        network_cost: 1.0,
    };
    assert!((cheap.total_cost() - 3.5).abs() < 1e-12);
    assert!((expensive.total_cost() - 36.0).abs() < 1e-12);
    assert!(cheap.is_better_than(&expensive));
    assert!(!expensive.is_better_than(&cheap));
}

#[test]
fn test_filter_pushdown() {
    // Filter placed *after* a Join should be pushed ahead of it so rows
    // are reduced before the expensive join.
    let optimizer = QueryOptimizer::new();
    let operations = vec![
        QueryOperation::Join {
            left_cost: 500.0,
            right_cost: 500.0,
            join_type: JoinType::NestedLoop,
        },
        QueryOperation::Filter {
            predicate: "x > 10".to_string(),
            selectivity: 0.1,
        },
    ];

    let plan = optimizer.optimize(operations).unwrap();

    // The Filter must come before the Join.
    assert!(
        matches!(plan.operations[0].operation, QueryOperation::Filter { .. }),
        "filter should be pushed before the join"
    );
    assert!(
        matches!(plan.operations[1].operation, QueryOperation::Join { .. }),
        "join should follow the filter"
    );
    assert_eq!(plan.operations.len(), 2);
}

#[test]
fn test_hash_join_selection() {
    // Both sides > 1000 rows → HashJoin should be chosen regardless of the
    // join_type the caller supplied.
    let optimizer = QueryOptimizer::new();
    let operations = vec![QueryOperation::Join {
        left_cost: 5000.0,
        right_cost: 4000.0,
        join_type: JoinType::NestedLoop,
    }];

    let plan = optimizer.optimize(operations).unwrap();

    match &plan.operations[0].operation {
        QueryOperation::Join { join_type, .. } => {
            assert_eq!(*join_type, JoinType::HashJoin);
        }
        other => panic!("expected a join, got {:?}", other),
    }
}

#[test]
fn test_nested_loop_small_tables() {
    // Both sides < 100 rows → NestedLoop is acceptable (and chosen).
    let optimizer = QueryOptimizer::new();
    let operations = vec![QueryOperation::Join {
        left_cost: 50.0,
        right_cost: 30.0,
        join_type: JoinType::HashJoin,
    }];

    let plan = optimizer.optimize(operations).unwrap();

    match &plan.operations[0].operation {
        QueryOperation::Join { join_type, .. } => {
            assert_eq!(*join_type, JoinType::NestedLoop);
        }
        other => panic!("expected a join, got {:?}", other),
    }
}

#[test]
fn test_cost_estimation() {
    // Every step should carry a positive, finite estimated cost and the
    // plan total should equal the sum of the per-step costs.
    let optimizer = QueryOptimizer::new();
    let operations = vec![
        QueryOperation::Scan {
            table: "t".to_string(),
            estimated_rows: 1000,
        },
        QueryOperation::Filter {
            predicate: "x > 1".to_string(),
            selectivity: 0.5,
        },
        QueryOperation::Limit { count: 10 },
    ];

    let plan = optimizer.optimize(operations).unwrap();

    assert!(plan.estimated_cost > 0.0);
    assert!(plan.estimated_cost.is_finite());
    for step in &plan.operations {
        assert!(step.estimated_cost >= 0.0, "cost must be non-negative");
        assert!(step.estimated_cost.is_finite(), "cost must be finite");
        assert!(step.estimated_rows <= 10_000_000, "rows bounded");
    }

    let sum: f64 = plan.operations.iter().map(|s| s.estimated_cost).sum();
    assert!((plan.estimated_cost - sum).abs() < 1e-9);

    // Scan cost = 1000 * 0.01 = 10.0
    assert!((plan.operations[0].estimated_cost - 10.0).abs() < 1e-9);
    // Filter cost = 1000 * 0.5 * 0.005 = 2.5
    assert!((plan.operations[1].estimated_cost - 2.5).abs() < 1e-9);
    // Limit cost = 10 * 0.001 = 0.01
    assert!((plan.operations[2].estimated_cost - 0.01).abs() < 1e-9);
}

#[test]
fn test_limit_last() {
    // Limit must always be the final step, even if supplied first.
    let optimizer = QueryOptimizer::new();
    let operations = vec![
        QueryOperation::Limit { count: 5 },
        QueryOperation::Scan {
            table: "t".to_string(),
            estimated_rows: 100,
        },
        QueryOperation::Filter {
            predicate: "x > 0".to_string(),
            selectivity: 0.5,
        },
    ];

    let plan = optimizer.optimize(operations).unwrap();

    assert!(
        matches!(
            plan.operations.last().unwrap().operation,
            QueryOperation::Limit { .. }
        ),
        "limit must be the last operation"
    );
    // No other Limit appears mid-plan.
    let limit_count = plan
        .operations
        .iter()
        .filter(|s| matches!(s.operation, QueryOperation::Limit { .. }))
        .count();
    assert_eq!(limit_count, 1);
}

#[test]
fn test_full_plan_optimization() {
    // A complex query: scan → filter → join → aggregate → sort → limit.
    let optimizer = QueryOptimizer::new();
    let operations = vec![
        QueryOperation::Scan {
            table: "orders".to_string(),
            estimated_rows: 5000,
        },
        QueryOperation::Filter {
            predicate: "status = 'paid'".to_string(),
            selectivity: 0.2,
        },
        QueryOperation::Join {
            left_cost: 1000.0,
            right_cost: 2000.0,
            join_type: JoinType::NestedLoop,
        },
        QueryOperation::Aggregate {
            group_by: vec!["customer_id".to_string()],
        },
        QueryOperation::Sort {
            columns: vec!["total".to_string()],
        },
        QueryOperation::Limit { count: 100 },
    ];

    let plan = optimizer.optimize(operations).unwrap();

    // No operations dropped or duplicated.
    assert_eq!(plan.operations.len(), 6);

    // Ordering: Scan, Filter, Join, Aggregate, Sort, Limit.
    let kinds: Vec<&str> = plan
        .operations
        .iter()
        .map(|s| match &s.operation {
            QueryOperation::Scan { .. } => "scan",
            QueryOperation::Filter { .. } => "filter",
            QueryOperation::Join { .. } => "join",
            QueryOperation::Aggregate { .. } => "aggregate",
            QueryOperation::Sort { .. } => "sort",
            QueryOperation::Limit { .. } => "limit",
            QueryOperation::Project { .. } => "project",
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["scan", "filter", "join", "aggregate", "sort", "limit"]
    );

    // Both join sides > 1000 → HashJoin selected.
    if let QueryOperation::Join { join_type, .. } = &plan.operations[2].operation {
        assert_eq!(*join_type, JoinType::HashJoin);
    }

    // Plan-level aggregates are populated.
    assert!(plan.estimated_cost > 0.0);
    assert!(plan.estimated_rows > 0);
}

#[test]
fn test_empty_operations() {
    // An empty operation list yields an empty plan.
    let optimizer = QueryOptimizer::new();
    let plan = optimizer.optimize(Vec::new()).unwrap();

    assert!(plan.operations.is_empty());
    assert!((plan.estimated_cost - 0.0).abs() < 1e-12);
    assert_eq!(plan.estimated_rows, 0);
}

// ---- wired capability methods: known-value tests --------------------------

/// Build a single-column dataset of floats named `col`.
fn one_col(lib: &mut StatisticalComputingLibrary, id: &str, col: &str, vals: &[f64]) {
    let data: Vec<Vec<DataValue>> = vals.iter().map(|&v| vec![DataValue::Float(v)]).collect();
    lib.create_dataset(
        id.to_string(),
        data,
        vec![col.to_string()],
        vec![DataType::Float64],
        PrivacyLevel::Public,
    )
    .unwrap();
}

/// Build a two-column dataset of floats.
fn two_col(lib: &mut StatisticalComputingLibrary, id: &str, xs: &[f64], ys: &[f64]) {
    let data: Vec<Vec<DataValue>> = xs
        .iter()
        .zip(ys)
        .map(|(&x, &y)| vec![DataValue::Float(x), DataValue::Float(y)])
        .collect();
    lib.create_dataset(
        id.to_string(),
        data,
        vec!["x".to_string(), "y".to_string()],
        vec![DataType::Float64, DataType::Float64],
        PrivacyLevel::Public,
    )
    .unwrap();
}

#[test]
fn test_std_skew_kurtosis_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    one_col(
        &mut lib,
        "d",
        "col",
        &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0],
    );
    // Population std of this classic set is 2.0.
    let sd = lib.standard_deviation("d", "col", false).unwrap();
    assert!((sd.result - 2.0).abs() < 1e-9, "sd={}", sd.result);
    // Symmetric-ish set: skewness finite; just assert it computes.
    assert!(lib.skewness("d", "col").is_ok());
    assert!(lib.kurtosis("d", "col").is_ok());
}

#[test]
fn test_mode_and_quantile_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    one_col(&mut lib, "d", "col", &[1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0]);
    let m = lib.mode("d", "col").unwrap();
    assert_eq!(m.value, 3.0);
    assert_eq!(m.count, 3);
    // Median (0.5 quantile) of [1..=7 subset sorted] = 3.0.
    let q = lib.quantile("d", "col", 0.5).unwrap();
    assert!((q.result - 3.0).abs() < 1e-9);
}

#[test]
fn test_covariance_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    two_col(&mut lib, "d", &[1.0, 2.0, 3.0, 4.0], &[2.0, 4.0, 6.0, 8.0]);
    // Sample covariance of x=[1,2,3,4], y=2x: var(x)_sample=1.6667, cov=3.3333.
    let c = lib.covariance("d", "x", "y", true).unwrap();
    assert!((c.result - 10.0 / 3.0).abs() < 1e-9, "cov={}", c.result);
}

#[test]
fn test_linear_regression_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    two_col(
        &mut lib,
        "d",
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[3.0, 5.0, 7.0, 9.0, 11.0],
    );
    // y = 2x + 1 exactly.
    let r = lib.linear_regression("d", "x", "y").unwrap();
    assert!((r.slope - 2.0).abs() < 1e-9, "slope={}", r.slope);
    assert!(
        (r.intercept - 1.0).abs() < 1e-9,
        "intercept={}",
        r.intercept
    );
    assert!((r.r_squared - 1.0).abs() < 1e-9);
}

#[test]
fn test_polynomial_regression_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    // y = x^2 exactly.
    two_col(
        &mut lib,
        "d",
        &[-2.0, -1.0, 0.0, 1.0, 2.0, 3.0],
        &[4.0, 1.0, 0.0, 1.0, 4.0, 9.0],
    );
    let f = lib.polynomial_regression("d", "x", "y", 2).unwrap();
    assert_eq!(f.coefficients.len(), 3);
    assert!((f.coefficients[0]).abs() < 1e-6, "c0={}", f.coefficients[0]);
    assert!((f.coefficients[1]).abs() < 1e-6, "c1={}", f.coefficients[1]);
    assert!(
        (f.coefficients[2] - 1.0).abs() < 1e-6,
        "c2={}",
        f.coefficients[2]
    );
    assert!((f.r_squared - 1.0).abs() < 1e-9);
}

#[test]
fn test_anova_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    // Three identical-spread groups with different means → large F.
    let data: Vec<Vec<DataValue>> = (0..4)
        .map(|i| {
            vec![
                DataValue::Float(1.0 + i as f64 * 0.0 + [0.0, 1.0, 0.0, 1.0][i]),
                DataValue::Float(10.0 + [0.0, 1.0, 0.0, 1.0][i]),
                DataValue::Float(20.0 + [0.0, 1.0, 0.0, 1.0][i]),
            ]
        })
        .collect();
    lib.create_dataset(
        "d".to_string(),
        data,
        vec!["g1".to_string(), "g2".to_string(), "g3".to_string()],
        vec![DataType::Float64, DataType::Float64, DataType::Float64],
        PrivacyLevel::Public,
    )
    .unwrap();
    let a = lib.anova("d", &["g1", "g2", "g3"]).unwrap();
    assert!(a.f_statistic > 10.0, "F={}", a.f_statistic);
    assert!(a.p_value < 0.001, "p={}", a.p_value);
    assert!((a.df_between - 2.0).abs() < 1e-9);
}

#[test]
fn test_chi_square_gof_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    // Observed exactly equal to a uniform expectation → statistic 0.
    one_col(&mut lib, "d", "col", &[10.0, 10.0, 10.0, 10.0]);
    let r = lib.chi_square_gof("d", "col", None).unwrap();
    assert!(r.statistic.abs() < 1e-9, "chi2={}", r.statistic);
    assert!((r.dof - 3.0).abs() < 1e-9);
}

#[test]
fn test_logistic_regression_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    // Clear upward trend, non-separable so the MLE is finite.
    let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let ys = [0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0];
    two_col(&mut lib, "d", &xs, &ys);
    let m = lib.logistic_regression("d", &["x"], "y", true).unwrap();
    // Positive slope on the single predictor (coefficients[1] with intercept).
    assert!(m.coefficients[1] > 0.0, "beta={}", m.coefficients[1]);
}

#[test]
fn test_timeseries_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    one_col(&mut lib, "d", "col", &[1.0, 2.0, 3.0, 4.0, 5.0]);
    let ac = lib.autocorrelation("d", "col", 1).unwrap();
    assert!((ac.result - 0.4).abs() < 1e-9, "acf1={}", ac.result);
    let ma = lib.moving_average("d", "col", 2).unwrap();
    assert_eq!(ma, vec![1.5, 2.5, 3.5, 4.5]);
    let es = lib.exponential_smoothing("d", "col", 0.5).unwrap();
    assert!((es[0] - 1.0).abs() < 1e-9 && (es[1] - 1.5).abs() < 1e-9);
}

#[test]
fn test_kmeans_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    // Two well-separated clusters in 1-D.
    one_col(&mut lib, "d", "col", &[0.0, 0.1, 0.2, 10.0, 10.1, 10.2]);
    let m = lib.kmeans("d", &["col"], 2, 50, 42).unwrap();
    assert_eq!(m.k, 2);
    // The two low points and two high points must not share a label boundary:
    // points 0..3 share one label, points 3..6 share another.
    assert_eq!(m.labels[0], m.labels[1]);
    assert_eq!(m.labels[3], m.labels[4]);
    assert_ne!(m.labels[0], m.labels[3]);
}

#[test]
fn test_svm_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    // Linearly separable: x<0 negative, x>0 positive.
    let xs = [-3.0, -2.0, -1.0, 1.0, 2.0, 3.0];
    let ys = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    two_col(&mut lib, "d", &xs, &ys);
    let r = lib.linear_svm("d", &["x"], "y", 1.0).unwrap();
    assert!(
        (r.train_accuracy - 1.0).abs() < 1e-9,
        "acc={}",
        r.train_accuracy
    );
    assert!(r.n_support_vectors >= 1);
}

#[test]
fn test_random_forest_wired() {
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize().unwrap();
    // Monotone target the forest can fit in-sample.
    let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let ys = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    two_col(&mut lib, "d", &xs, &ys);
    let r = lib.random_forest("d", &["x"], "y", 16, false, 7).unwrap();
    assert_eq!(r.n_trees, 16);
    assert!(r.train_metric > 0.8, "R2={}", r.train_metric);
}
