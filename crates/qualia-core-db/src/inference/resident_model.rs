//! Process-wide resident GGUF mmap — released explicitly on model eviction.

#[cfg(not(target_arch = "wasm32"))]
use crate::gguf_bridge::{GgufLoadReport, QTensorEngine};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(all(unix, not(target_arch = "wasm32")))]
fn apply_mlock(mmap: &memmap2::Mmap, mlock: bool) {
    if mlock {
        unsafe {
            libc::mlock(mmap.as_ptr() as *const libc::c_void, mmap.len());
        }
    }
}

#[cfg(not(all(unix, not(target_arch = "wasm32"))))]
fn apply_mlock<T>(_mmap: &T, _mlock: bool) {}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct ResidentModelSlot {
    pub model_id: u64,
    pub gguf_path: String,
    pub mmap: Arc<memmap2::Mmap>,
    pub report: GgufLoadReport,
}

#[cfg(not(target_arch = "wasm32"))]
fn slot() -> &'static Arc<Mutex<Option<ResidentModelSlot>>> {
    static SLOT: OnceLock<Arc<Mutex<Option<ResidentModelSlot>>>> = OnceLock::new();
    SLOT.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// Memory-map `path` and retain until [`clear_resident_model`].
#[cfg(not(target_arch = "wasm32"))]
pub fn mount_resident_gguf(model_id: u64, path: &str, mlock: bool) -> Result<GgufLoadReport, String> {
    clear_resident_model();
    let mut engine = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(QTensorEngine::try_new())
    })?;
    let report = engine.load_gguf_checked(path)?;
    let mmap = engine
        .gguf_mmap
        .take()
        .ok_or_else(|| "Internal error: GGUF mmap missing after load".to_string())?;
    apply_mlock(&mmap, mlock);
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

/// Memory-map a P64 weight container and retain it as the resident model.
///
/// The function name is retained for source compatibility. New format-neutral
/// callers should use [`mount_resident_model`].
#[cfg(not(target_arch = "wasm32"))]
pub fn mount_resident_q42(model_id: u64, path: &str, mlock: bool) -> Result<GgufLoadReport, String> {
    clear_resident_model();
    let file = std::fs::File::open(path).map_err(|e| format!("open {path}: {e}"))?;
    let mmap_raw = unsafe { memmap2::MmapOptions::new().populate().map(&file) }.map_err(|e| e.to_string())?;
    apply_mlock(&mmap_raw, mlock);
    let mmap = Arc::new(mmap_raw);
    let mut engine = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(QTensorEngine::try_new())
    })?;
    let report = engine.adopt_resident_p64_mmap(Arc::clone(&mmap))?;
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

/// Memory-map a local model and select P64 or GGUF by canonical magic.
///
/// This is the preferred format-neutral entry point. The historical
/// `mount_resident_q42` function remains as a compatibility alias for callers
/// that already know they have a P64 container.
#[cfg(not(target_arch = "wasm32"))]
pub fn mount_resident_model(model_id: u64, path: &str, mlock: bool) -> Result<GgufLoadReport, String> {
    clear_resident_model();
    let mut engine = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(QTensorEngine::try_new())
    })?;
    let report = engine.load_model_checked(path)?;
    let mmap = engine
        .gguf_mmap
        .take()
        .ok_or_else(|| "Internal error: model mmap missing after load".to_string())?;
    apply_mlock(&mmap, mlock);
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
pub fn resident_mmap_for_path(path: &str) -> Option<Arc<memmap2::Mmap>> {
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

#[cfg(not(target_arch = "wasm32"))]
pub fn resident_gguf_path() -> Option<String> {
    slot()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.gguf_path.clone()))
}

#[cfg(target_arch = "wasm32")]
pub fn resident_gguf_path() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn resident_mmap_for_path(_path: &str) -> Option<()> {
    None
}
