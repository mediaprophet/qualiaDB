//! Model mapping and precision optimization pipeline.
//!
//! Provides model operational envelope mapping (`ModelPrecisionTarget`),
//! candidate strategy generation (B0–B5), split-isolated evaluation,
//! and registry promotion with reviewable rollback.

pub mod model_target;
pub mod optimizer;
pub mod receipt;
#[cfg(not(target_arch = "wasm32"))]
pub mod receipt_store;
pub mod runtime_contract;
pub mod strategy_generator;

pub use model_target::{
    global_model_precision_registry, resolve_global_active_contract, ModelFamily,
    ModelPrecisionRegistry, ModelPrecisionTarget,
};
pub use optimizer::{ModelPrecisionOptimizer, OptimizationError};
pub use receipt::ModelOptimizationReceipt;
#[cfg(not(target_arch = "wasm32"))]
pub use receipt_store::{
    ModelOptimizationReceiptStore, ReceiptStoreError, SignedModelOptimizationReceipt,
    MAX_MODEL_OPTIMIZATION_RECEIPT_BYTES, MODEL_OPTIMIZATION_RECEIPT_SCHEMA_VERSION,
};
pub use runtime_contract::{CompressionStrategy, ModelPrecisionContract, PrefixConfiguration};
pub use strategy_generator::{CandidateRequirement, CandidateStrategy, StrategyGenerator};
