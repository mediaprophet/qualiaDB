//! Statistical Computing Library - Privacy-Preserving Statistical Analysis
//!
//! This module provides high-performance statistical computing operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for secure statistical computations
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy statistical data
//! - Zero-Knowledge Semantic Proofs for privacy-preserving statistics
//! - NVMe Computational Storage (CSD) for accelerated statistical operations

use crate::fiduciary_crypto::{FiduciaryCrypto, MlDsaSignature};
use crate::zk_proofs::{CircuitExpression, FieldElement, VariableType, ZkProof, ZkProofSystem};
use crate::zns_storage::ZnsZoneManager;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Statistical Computing Library Manager
pub struct StatisticalComputingLibrary {
    data_storage: StatisticalDataStorage,
    computation_engine: StatisticalComputationEngine,
    privacy_engine: StatisticalPrivacyEngine,
    analysis_engine: StatisticalAnalysisEngine,
    performance_monitor: StatisticalPerformanceMonitor,
}

/// Statistical data storage using ZNS for efficient data management
pub struct StatisticalDataStorage {
    zones: HashMap<String, StatisticalZone>,
    data_catalog: DataCatalog,
    compression_engine: DataCompressionEngine,
    indexing_engine: DataIndexingEngine,
    dataset_cache: HashMap<String, Dataset>,
    /// Optional ZNS zone manager. When `Some`, dataset persistence delegates
    /// to the real ZNS device; otherwise the in-memory `dataset_cache` acts as
    /// the always-available persistence layer.
    zns_manager: Option<Arc<Mutex<ZnsZoneManager>>>,
}
/// Data catalog for dataset management
pub struct DataCatalog {
    datasets: HashMap<String, DatasetMetadata>,
    relationships: HashMap<String, Vec<Relationship>>,
    tags: HashMap<String, Vec<String>>,
    search_index: SearchIndex,
}
/// Statistical privacy engine
pub struct StatisticalPrivacyEngine {
    fiduciary_crypto: Arc<Mutex<FiduciaryCrypto>>,
    zk_proofs: Arc<Mutex<ZkProofSystem>>,
    differential_privacy: DifferentialPrivacy,
    secure_aggregation: SecureAggregation,
    privacy_budget: PrivacyBudget,
}

/// Differential privacy
pub struct DifferentialPrivacy {
    noise_mechanisms: Vec<NoiseMechanism>,
    privacy_accountant: PrivacyAccountant,
    sensitivity_analyzer: SensitivityAnalyzer,
}
/// Sensitivity analyzer
pub struct SensitivityAnalyzer {
    sensitivity_functions: HashMap<String, SensitivityFunction>,
    sensitivity_cache: HashMap<String, f64>,
}

mod accelerator;
mod analytics;
mod catalog;
mod compression;
mod computation;
mod datasets;
mod errors;
mod indexing;
mod library;
mod privacy;
mod scheduler;
mod types;

pub use accelerator::*;
pub use analytics::*;
pub use catalog::*;
pub use compression::*;
pub use computation::*;
pub use datasets::*;
pub use errors::*;
pub use indexing::*;
pub use privacy::*;
pub use scheduler::*;
pub use types::*;

#[cfg(test)]
mod tests;
