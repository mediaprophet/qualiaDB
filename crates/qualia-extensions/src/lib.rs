//! QualiaDB Advanced Extensions
//! 
//! This crate provides the extension interface for heavy computational workloads
//! that cannot run in the zero-allocation core QualiaDB engine.
//! 
//! # Architecture
//! 
//! - Core QualiaDB: Semantic orchestrator (48-byte Super-Quin logic)
//! - Extensions: Heavy computational work (std, GPU, external APIs)
//! - Communication: FFI bridge with strict memory boundaries
//! 
//! # Extension Types
//! 
//! - QPU Extension: Quantum computing via remote APIs
//! - PINN Extension: Physics-Informed Neural Networks (uses native Qualia LLM pipeline with wgpu + WGSL)
//! - SNN Extension: Spiking Neural Networks with CRDT synchronization
//! - Fluid Extension: WebGPU-based fluid dynamics
//! - Math Extension: Advanced mathematical solvers
//!
//! # Native Pipeline Integration
//!
//! The PINN extension now uses the native Qualia LLM pipeline:
//! - wgpu for GPU compute (not Candle)
//! - Custom WGSL compute shaders
//! - memmap2 for GGUF model loading
//! - Same infrastructure as the core LLM agent

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

/// Extension capability descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCapability {
    pub name: String,
    pub version: String,
    pub description: String,
    pub required_resources: ResourceRequirements,
    pub supported_operations: Vec<String>,
}

/// Resource requirements for an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_memory_mb: u64,
    pub min_vram_mb: Option<u64>,
    pub requires_gpu: bool,
    pub requires_network: bool,
    pub max_concurrent_jobs: u32,
}

/// Extension job request from core QualiaDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionJob {
    pub job_id: String,
    pub extension_name: String,
    pub operation: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub boundary_conditions: Vec<NQuin>,
}

/// Extension job result for core QualiaDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionResult {
    pub job_id: String,
    pub success: bool,
    pub result_quins: Vec<NQuin>,
    pub metadata: HashMap<String, String>,
    pub execution_time_ms: u64,
}

/// NQuin representation for extension communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

/// Extension registry and manager
pub struct ExtensionManager {
    extensions: HashMap<String, Box<dyn Extension>>,
    active_jobs: Arc<Mutex<HashMap<String, ExtensionJob>>>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_extension(&mut self, extension: Box<dyn Extension>) {
        let capability = extension.capability();
        self.extensions.insert(capability.name.clone(), extension);
    }

    pub async fn execute_job(&self, job: ExtensionJob) -> Result<ExtensionResult, ExtensionError> {
        let extension = self.extensions.get(&job.extension_name)
            .ok_or(ExtensionError::ExtensionNotFound(job.extension_name.clone()))?;

        // Track active job
        {
            let mut active = self.active_jobs.lock().unwrap();
            active.insert(job.job_id.clone(), job.clone());
        }

        // Execute the job
        let result = extension.execute(job).await;

        // Remove from active jobs
        {
            let mut active = self.active_jobs.lock().unwrap();
            active.remove(&result.as_ref().map(|r| r.job_id.clone()).unwrap_or_default());
        }

        result
    }

    pub fn list_capabilities(&self) -> Vec<ExtensionCapability> {
        self.extensions.values()
            .map(|ext| ext.capability())
            .collect()
    }
}

/// Extension trait for all computational extensions
#[async_trait::async_trait]
pub trait Extension: Send + Sync {
    fn capability(&self) -> ExtensionCapability;
    async fn execute(&self, job: ExtensionJob) -> Result<ExtensionResult, ExtensionError>;
    fn shutdown(&self) -> Result<(), ExtensionError>;
}

/// Extension error types
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Extension '{0}' not found")]
    ExtensionNotFound(String),
    
    #[error("Operation '{0}' not supported by extension")]
    OperationNotSupported(String),
    
    #[error("Insufficient resources: {0}")]
    InsufficientResources(String),
    
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("GPU error: {0}")]
    GpuError(String),
}

/// FFI bridge for core QualiaDB communication
#[repr(C)]
pub struct ExtensionBridge {
    manager: *mut ExtensionManager,
}

#[repr(C)]
pub struct CExtensionJob {
    pub job_id: *const u8,
    pub job_id_len: usize,
    pub extension_name: *const u8,
    pub extension_name_len: usize,
    pub operation: *const u8,
    pub operation_len: usize,
    /// UTF-8 JSON object of `parameters` (may be null/0 ⇒ no parameters).
    pub parameters: *const u8,
    pub parameters_len: usize,
}

#[repr(C)]
pub struct CExtensionResult {
    pub success: bool,
    /// UTF-8 JSON of the [`ExtensionResult`] on success (else null).
    /// Free with [`extension_result_free`].
    pub result_data: *const u8,
    pub result_len: usize,
    /// UTF-8 error message on failure (else null). Free with
    /// [`extension_result_free`].
    pub error_msg: *const u8,
    pub error_len: usize,
}

#[no_mangle]
pub extern "C" fn extension_manager_new() -> *mut ExtensionManager {
    Box::into_raw(Box::new(ExtensionManager::new()))
}

/// Process-wide runtime used to drive async extension jobs across the (sync)
/// FFI boundary. Built once; `None` only if Tokio fails to initialise.
fn ffi_runtime() -> Option<&'static tokio::runtime::Runtime> {
    use std::sync::OnceLock;
    static RT: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .ok()
    })
    .as_ref()
}

/// SAFETY: `ptr`/`len` must describe a valid byte range for `'a`, or `ptr` may
/// be null with `len == 0`.
unsafe fn ffi_str<'a>(ptr: *const u8, len: usize) -> Option<&'a str> {
    if ptr.is_null() {
        return if len == 0 { Some("") } else { None };
    }
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).ok()
}

fn heap_bytes(bytes: Vec<u8>) -> (*const u8, usize) {
    if bytes.is_empty() {
        return (std::ptr::null(), 0);
    }
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    (Box::into_raw(boxed) as *const u8, len)
}

fn ffi_ok(json: Vec<u8>) -> CExtensionResult {
    let (ptr, len) = heap_bytes(json);
    CExtensionResult {
        success: true,
        result_data: ptr,
        result_len: len,
        error_msg: std::ptr::null(),
        error_len: 0,
    }
}

fn ffi_err(msg: impl Into<String>) -> CExtensionResult {
    let (ptr, len) = heap_bytes(msg.into().into_bytes());
    CExtensionResult {
        success: false,
        result_data: std::ptr::null(),
        result_len: 0,
        error_msg: ptr,
        error_len: len,
    }
}

/// Execute an extension job submitted over the C ABI.
///
/// Marshals the [`CExtensionJob`] (UTF-8 fields + JSON `parameters`) into an
/// [`ExtensionJob`], runs it on the shared runtime, and returns the
/// JSON-serialised [`ExtensionResult`] (on success) or an error message. The
/// returned buffers are heap-owned and must be released with
/// [`extension_result_free`].
///
/// # Safety
/// `manager` must be a pointer returned by [`extension_manager_new`] (and not
/// yet freed); `job` must point to a valid `CExtensionJob` whose byte ranges
/// are valid for the duration of the call. Must not be called from within an
/// existing Tokio runtime on the same thread.
#[no_mangle]
pub extern "C" fn extension_manager_execute_job(
    manager: *mut ExtensionManager,
    job: *const CExtensionJob,
) -> CExtensionResult {
    if manager.is_null() {
        return ffi_err("null manager pointer");
    }
    if job.is_null() {
        return ffi_err("null job pointer");
    }

    let mgr = unsafe { &*manager };
    let cjob = unsafe { &*job };

    let job_id = match unsafe { ffi_str(cjob.job_id, cjob.job_id_len) } {
        Some(s) => s.to_string(),
        None => return ffi_err("job_id is not valid UTF-8"),
    };
    let extension_name = match unsafe { ffi_str(cjob.extension_name, cjob.extension_name_len) } {
        Some(s) => s.to_string(),
        None => return ffi_err("extension_name is not valid UTF-8"),
    };
    let operation = match unsafe { ffi_str(cjob.operation, cjob.operation_len) } {
        Some(s) => s.to_string(),
        None => return ffi_err("operation is not valid UTF-8"),
    };
    let parameters: HashMap<String, serde_json::Value> =
        if cjob.parameters.is_null() || cjob.parameters_len == 0 {
            HashMap::new()
        } else {
            let raw = unsafe { std::slice::from_raw_parts(cjob.parameters, cjob.parameters_len) };
            match serde_json::from_slice(raw) {
                Ok(v) => v,
                Err(e) => return ffi_err(format!("invalid parameters JSON: {e}")),
            }
        };

    let ext_job = ExtensionJob {
        job_id,
        extension_name,
        operation,
        parameters,
        boundary_conditions: Vec::new(),
    };

    let rt = match ffi_runtime() {
        Some(rt) => rt,
        None => return ffi_err("failed to initialise async runtime"),
    };

    match rt.block_on(mgr.execute_job(ext_job)) {
        Ok(result) => match serde_json::to_vec(&result) {
            Ok(json) => ffi_ok(json),
            Err(e) => ffi_err(format!("failed to serialise result: {e}")),
        },
        Err(e) => ffi_err(e.to_string()),
    }
}

/// Release the heap buffers owned by a [`CExtensionResult`].
///
/// # Safety
/// `result` must be a value previously returned by
/// [`extension_manager_execute_job`] and not already freed.
#[no_mangle]
pub extern "C" fn extension_result_free(result: CExtensionResult) {
    unsafe {
        if !result.result_data.is_null() && result.result_len > 0 {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(
                result.result_data as *mut u8,
                result.result_len,
            ));
        }
        if !result.error_msg.is_null() && result.error_len > 0 {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(
                result.error_msg as *mut u8,
                result.error_len,
            ));
        }
    }
}

#[no_mangle]
pub extern "C" fn extension_manager_free(manager: *mut ExtensionManager) {
    if !manager.is_null() {
        unsafe { let _ = Box::from_raw(manager); }
    }
}

// Module declarations
pub mod pinn_extension;
pub mod snn_extension;
pub mod qpu_extension;
pub mod webgpu_extension;

/// Helper function for hashing strings to 64-bit integers
pub fn q_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quin_conversion() {
        let core_quin = crate::NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 4,
            metadata: 5,
            parity: 6,
        };

        let ext_quin = NQuin::from(core_quin.clone());
        let converted_back = crate::NQuin::from(ext_quin);

        assert_eq!(core_quin.subject, converted_back.subject);
        assert_eq!(core_quin.predicate, converted_back.predicate);
        assert_eq!(core_quin.object, converted_back.object);
        assert_eq!(core_quin.context, converted_back.context);
        assert_eq!(core_quin.metadata, converted_back.metadata);
        assert_eq!(core_quin.parity, converted_back.parity);
    }

    /// Build a manager with the (real) webgpu extension registered and leak it
    /// to a raw pointer for the FFI calls.
    fn manager_with_webgpu() -> *mut ExtensionManager {
        let mut manager = ExtensionManager::new();
        manager.register_extension(Box::new(crate::webgpu_extension::WebGpuExtension::new()));
        Box::into_raw(Box::new(manager))
    }

    fn cjob(job_id: &[u8], ext: &[u8], op: &[u8], params: &[u8]) -> CExtensionJob {
        CExtensionJob {
            job_id: job_id.as_ptr(),
            job_id_len: job_id.len(),
            extension_name: ext.as_ptr(),
            extension_name_len: ext.len(),
            operation: op.as_ptr(),
            operation_len: op.len(),
            parameters: params.as_ptr(),
            parameters_len: params.len(),
        }
    }

    #[test]
    fn ffi_bridge_executes_a_real_job() {
        let mgr = manager_with_webgpu();
        let job = cjob(b"ffi-1", b"webgpu", b"tensor_operations", b"{}");
        let res = extension_manager_execute_job(mgr, &job);

        assert!(res.success, "FFI job did not succeed");
        assert!(!res.result_data.is_null());
        let bytes = unsafe { std::slice::from_raw_parts(res.result_data, res.result_len) };
        let parsed: ExtensionResult = serde_json::from_slice(bytes).expect("result JSON");
        assert_eq!(parsed.job_id, "ffi-1");
        assert!(parsed.success);
        assert!(!parsed.result_quins.is_empty());

        extension_result_free(res);
        unsafe { extension_manager_free(mgr) };
    }

    #[test]
    fn ffi_bridge_reports_unknown_extension() {
        let mgr = manager_with_webgpu();
        let job = cjob(b"ffi-2", b"nonexistent", b"tensor_operations", b"{}");
        let res = extension_manager_execute_job(mgr, &job);

        assert!(!res.success);
        assert!(res.result_data.is_null());
        let msg = unsafe { std::slice::from_raw_parts(res.error_msg, res.error_len) };
        assert!(std::str::from_utf8(msg).unwrap().contains("nonexistent"));

        extension_result_free(res);
        unsafe { extension_manager_free(mgr) };
    }

    #[test]
    fn ffi_bridge_rejects_null_manager() {
        let job = cjob(b"ffi-3", b"webgpu", b"tensor_operations", b"{}");
        let res = extension_manager_execute_job(std::ptr::null_mut(), &job);
        assert!(!res.success);
        assert!(res.error_len > 0);
        extension_result_free(res);
    }

    #[test]
    fn ffi_bridge_rejects_bad_parameters_json() {
        let mgr = manager_with_webgpu();
        let job = cjob(b"ffi-4", b"webgpu", b"tensor_operations", b"{not json");
        let res = extension_manager_execute_job(mgr, &job);
        assert!(!res.success);
        let msg = unsafe { std::slice::from_raw_parts(res.error_msg, res.error_len) };
        assert!(std::str::from_utf8(msg).unwrap().contains("parameters JSON"));
        extension_result_free(res);
        unsafe { extension_manager_free(mgr) };
    }
}
