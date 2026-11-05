//! Request-scoped lifetime for U1 producer work and its host ring envelopes.
//! The work mutex is held through actual device/search completion and result
//! publication. Closing waits for in-flight work, revokes the epoch, and clears
//! the published mask before another request can use the fabric.
use crate::tensor::resident_substrate::ResidentSnapshot;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
use std::{cell::Cell, sync::Arc};
static SOURCE: Mutex<Option<Arc<ResidentSnapshot>>> = Mutex::new(None);
static KV: Mutex<Option<crate::tensor::kv_provenance::KvProvenanceMap>> = Mutex::new(None);
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
static GPU_SOURCE: Mutex<Option<crate::tensor::volume_gpu::owned::OwnedTensorSearch>> =
    Mutex::new(None);
thread_local! {
    static OWNER: Cell<u64> = const { Cell::new(0) };
    static PRODUCING: Cell<bool> = const { Cell::new(false) };
}
pub(super) fn source() -> Option<Arc<ResidentSnapshot>> {
    SOURCE.lock().ok()?.as_ref().cloned()
}
pub(super) fn can_publish() -> bool {
    source().is_none() || OWNER.with(|v| v.get() == current()) || PRODUCING.with(Cell::get)
}

static WORK: Mutex<()> = Mutex::new(());
static EPOCH: AtomicU64 = AtomicU64::new(1);
static ACTIVE: AtomicBool = AtomicBool::new(true);

pub(super) fn current() -> u64 {
    EPOCH.load(Ordering::Acquire)
}
fn advance() -> u64 {
    EPOCH
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .expect("compute request epoch exhausted")
        + 1
}
pub(super) fn accepts(epoch: u64) -> bool {
    ACTIVE.load(Ordering::Acquire) && current() == epoch
}
pub(super) fn produce(work: impl FnOnce() -> usize) -> usize {
    let _guard = WORK.lock().unwrap_or_else(|e| e.into_inner());
    if !ACTIVE.load(Ordering::Acquire) {
        return 0;
    }
    PRODUCING.with(|flag| {
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        flag.set(true);
        let _reset = Reset(flag);
        work()
    })
}

pub struct ComputeRequestEpoch {
    epoch: u64,
    previous_owner: u64,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
pub fn begin_request_epoch() -> ComputeRequestEpoch {
    let _guard = WORK.lock().unwrap_or_else(|e| e.into_inner());
    ACTIVE.store(false, Ordering::Release);
    let epoch = advance();
    *SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *KV.lock().unwrap_or_else(|e| e.into_inner()) = None;
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    {
        *GPU_SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
    let previous_owner = OWNER.with(|owner| owner.replace(epoch));
    // Envelopes from older requests remain bounded and are skipped on pop.
    while super::context_inject_ring().try_pop().is_some() {}
    while super::topology_draft_ring().try_pop().is_some() {}
    super::publish_attention_route_mask(super::AttentionRouteMask::default());
    super::publish_query_tensor(crate::tensor::Tensor10D::default(), 0);
    super::publish_decode_hint(0, 0);
    ComputeRequestEpoch {
        epoch,
        previous_owner,
        _thread: std::marker::PhantomData,
    }
}
#[allow(dead_code)]
impl ComputeRequestEpoch {
    #[allow(unused_variables)]
    pub(crate) fn bind_source(&self, source: Arc<ResidentSnapshot>, gpu_budget: usize) -> bool {
        let _guard = WORK.lock().unwrap_or_else(|e| e.into_inner());
        if current() != self.epoch || ACTIVE.load(Ordering::Acquire) {
            return false;
        }
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        {
            let Ok(gpu) =
                crate::tensor::volume_gpu::owned::OwnedTensorSearch::prepare(&source, gpu_budget)
            else {
                return false;
            };
            *GPU_SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = gpu;
        }
        *SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = Some(source);
        true
    }
    pub(crate) fn identity(&self) -> u64 {
        self.epoch
    }
    /// Freeze producer work and ring publication across input admission and the
    /// complete numerical step. Device work finishes before the guard releases.
    pub(crate) fn pause(&self) -> Option<std::sync::MutexGuard<'static, ()>> {
        let guard = WORK.lock().ok()?;
        (current() == self.epoch).then_some(guard)
    }
    /// Publish this request's query/mask before allowing producer work. The
    /// initializer must not call the producer recursively or start another epoch.
    pub(crate) fn activate(&self, initialize: impl FnOnce()) -> bool {
        let _guard = WORK.lock().unwrap_or_else(|e| e.into_inner());
        if current() != self.epoch {
            return false;
        }
        initialize();
        ACTIVE.store(true, Ordering::Release);
        true
    }
}
impl Drop for ComputeRequestEpoch {
    fn drop(&mut self) {
        let _guard = WORK.lock().unwrap_or_else(|e| e.into_inner());
        if current() == self.epoch {
            ACTIVE.store(false, Ordering::Release);
            super::publish_attention_route_mask(super::AttentionRouteMask::default());
            super::publish_query_tensor(crate::tensor::Tensor10D::default(), 0);
            super::publish_decode_hint(0, 0);
            *SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = None;
            *KV.lock().unwrap_or_else(|e| e.into_inner()) = None;
            #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
            {
                *GPU_SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = None;
            }
            advance();
        }
        OWNER.with(|owner| owner.set(self.previous_owner));
    }
}
pub(crate) fn scoped_kv_slot(index: u32) -> Option<Option<u32>> {
    KV.lock()
        .ok()?
        .as_ref()
        .map(|kv| kv.kv_slot_for_tensor(index))
}
#[allow(dead_code)]
pub(crate) fn rebuild_kv(prompt: u32, page: u64) -> bool {
    let Some(source) = source() else {
        return false;
    };
    if !can_publish() {
        return true;
    }
    let mut kv = crate::tensor::kv_provenance::KvProvenanceMap::new();
    kv.build_prompt_alignment(prompt, source.node_count(), page);
    *KV.lock().unwrap_or_else(|e| e.into_inner()) = Some(kv);
    true
}
#[allow(dead_code)]
pub(crate) fn record_kv(tensor: u32, slot: u32, page: u64) -> bool {
    if source().is_none() {
        return false;
    }
    if can_publish() {
        if let Some(kv) = KV.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            kv.record(tensor, slot, page);
        }
    }
    true
}
#[allow(dead_code)]
pub(super) fn kv_digest() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"qualia:context-kv-provenance:v1\0");
    if let Some(kv) = KV.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        for i in 0..crate::tensor::kv_provenance::MAX_KV_PROVENANCE as u32 {
            h.update(kv.kv_slot_for_tensor(i).unwrap_or(u32::MAX).to_le_bytes());
            h.update(kv.page_id_for_tensor(i).unwrap_or(0).to_le_bytes());
        }
    }
    h.finalize().into()
}
pub(super) fn owned_gpu_search(
    query: &crate::tensor::Tensor10D,
    distance: f32,
    out: &mut [usize],
) -> Option<usize> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    {
        return GPU_SOURCE
            .lock()
            .ok()?
            .as_ref()
            .map(|gpu| gpu.search(query, distance, out));
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "gpu-runtime")))]
    {
        let _ = (query, distance, out);
        None
    }
}
#[allow(dead_code)]
pub(crate) fn live(identity: u64, digest: [u8; 32]) -> bool {
    current() == identity && source().is_some_and(|s| s.digest() == digest)
}
