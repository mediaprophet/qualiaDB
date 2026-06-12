//! Process-wide resident GGUF mmap — released explicitly on model eviction.

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
use crate::gguf_bridge::{GgufLoadReport, QTensorEngine, GgufBuffer};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct ResidentModelSlot {
    pub model_id: u64,
    pub gguf_path: String,
    pub mmap: GgufBuffer,
    pub report: GgufLoadReport,
}

#[cfg(not(target_arch = "wasm32"))]
fn slot() -> &'static Arc<Mutex<Option<ResidentModelSlot>>> {
    static SLOT: OnceLock<Arc<Mutex<Option<ResidentModelSlot>>>> = OnceLock::new();
    SLOT.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// Memory-map `path` and retain until [`clear_resident_model`].
#[cfg(not(target_arch = "wasm32"))]
pub fn mount_resident_gguf(model_id: u64, path: &str) -> Result<GgufLoadReport, String> {
    clear_resident_model();
    let mut engine = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(QTensorEngine::try_new())
    })?;
    let report = engine.load_gguf_checked(path)?;
    let mmap = engine
        .gguf_mmap
        .take()
        .ok_or_else(|| "Internal error: GGUF mmap missing after load".to_string())?;
    let normalized = Path::new(path)
        .canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string());
    *slot().lock().map_err(|e| e.to_string())? = Some(ResidentModelSlot {
        model_id,
        gguf_path: normalized,
        mmap,
        report,
    });
    Ok(report)
}

#[cfg(target_arch = "wasm32")]
pub fn mount_resident_gguf(_model_id: u64, _path: &str) -> Result<(), String> {
    Ok(())
}

/// Drop resident mmap (called from orchestrator eviction scrub).
#[cfg(not(target_arch = "wasm32"))]
pub fn clear_resident_model() {
    if let Ok(mut guard) = slot().lock() {
        if guard.take().is_some() {
            log::info!("LLM_LOAD|evict-mmap|1.00|Released resident GGUF mmap");
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn clear_resident_model() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn resident_mmap_for_path(path: &str) -> Option<GgufBuffer> {
    let guard = slot().lock().ok()?;
    let slot = guard.as_ref()?;
    let requested = Path::new(path);
    let slot_path = Path::new(&slot.gguf_path);
    if requested == slot_path {
        return Some(Arc::clone(&slot.mmap));
    }
    let req_canon = requested.canonicalize().ok();
    let slot_canon = slot_path.canonicalize().ok();
    if req_canon.is_some() && req_canon == slot_canon {
        return Some(Arc::clone(&slot.mmap));
    }
    if requested.file_name().is_some() && requested.file_name() == slot_path.file_name() {
        return Some(Arc::clone(&slot.mmap));
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resident_model_id() -> Option<u64> {
    slot()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.model_id))
}

#[cfg(target_arch = "wasm32")]
pub fn resident_mmap_for_path(_path: &str) -> Option<()> {
    None
}
