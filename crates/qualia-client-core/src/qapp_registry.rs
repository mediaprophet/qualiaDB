use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const QAPP_PACKAGE_MANIFEST: &str = "qapp.json";
pub const QAPPS_DIR: &str = "Qapps";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum QappTarget {
    LocalDevDirectory(PathBuf),
    LocalProxyPort(u16),
    IsolatedVault(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappLaunchRequirement {
    pub capability: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappWasmConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub engine_package: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_exports: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappDaemonConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub health_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub query_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub websocket_path: String,
    #[serde(default)]
    pub requires_token: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappChatIntegrationConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub intent: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub payload_version: String,
    #[serde(default)]
    pub supports_launch_from_chat: bool,
    #[serde(default)]
    pub supports_return_to_chat: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappRepresentationContract {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepts: Vec<String>,
}

/// Configuration for 1.58-bit ternary quantized PINN models
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappPinnModelConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model_type: String,
    /// Quantization precision in bits (e.g., "1.58")
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub quantization_bits: String,
    /// Compression ratio achieved through ternary quantization
    #[serde(default)]
    pub compression_ratio: f32,
    /// SMX format version for model serialization
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub smx_version: String,
    /// Physics domain for the PINN model
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub physics_domain: String,
    /// Input dimensions for the model
    #[serde(default)]
    pub input_dim: usize,
    /// Output dimensions for the model
    #[serde(default)]
    pub output_dim: usize,
    /// Whether the model uses ternary quantization
    #[serde(default)]
    pub uses_ternary_quantization: bool,
    /// Memory requirements in MB
    #[serde(default)]
    pub memory_requirement_mb: u32,
    /// VRAM requirements in MB
    #[serde(default)]
    pub vram_requirement_mb: Option<u32>,
    /// Supported operations for this model
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_operations: Vec<String>,
    /// Model metadata including training information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_metadata: Option<QappPinnTrainingMetadata>,
    /// Performance characteristics
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance_metrics: Option<QappPinnPerformanceMetrics>,
}

/// Training metadata for PINN models
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappPinnTrainingMetadata {
    #[serde(default)]
    pub epochs: u32,
    #[serde(default)]
    pub final_loss: f64,
    #[serde(default)]
    pub convergence_rate: f64,
    #[serde(default)]
    pub validation_accuracy: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub training_dataset: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub training_date: String,
}

/// Performance metrics for PINN models
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappPinnPerformanceMetrics {
    /// Inference speedup from ternary quantization
    #[serde(default)]
    pub inference_speedup: f32,
    /// Memory savings percentage
    #[serde(default)]
    pub memory_savings: f32,
    /// Quantization error rate
    #[serde(default)]
    pub quantization_error: f64,
    /// Sparsity ratio (percentage of zero weights)
    #[serde(default)]
    pub sparsity_ratio: f32,
    /// Average inference time in milliseconds
    #[serde(default)]
    pub avg_inference_time_ms: f32,
    /// Throughput in operations per second
    #[serde(default)]
    pub throughput_ops_per_sec: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QappHostExtension {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub app_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launch_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preferred_launch_mode: String,
    #[serde(default)]
    pub supports_offline: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<QappLaunchRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_ontologies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_remote_endpoints: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub max_sensitivity_clearance: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_models: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub entrypoints: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm: Option<QappWasmConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_daemon: Option<QappDaemonConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_integration: Option<QappChatIntegrationConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub representation_contract: Option<QappRepresentationContract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinn_model: Option<QappPinnModelConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ui_surfaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Developer-authored on-disk package manifest (`qapp.json`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QappPackageManifest {
    pub name: String,
    pub version: String,
    pub required_shapes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dev_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_qualia: Option<QappHostExtension>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisteredQapp {
    pub did: String,
    pub manifest: QappPackageManifest,
    pub target: QappTarget,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct QappRegistryState {
    pub installed_qapps: HashMap<String, RegisteredQapp>,
}

impl QappRegistryState {
    pub fn new() -> Self {
        Self {
            installed_qapps: HashMap::new(),
        }
    }
}
