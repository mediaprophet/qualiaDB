//! W5b Phase 4 — runtime KV-dictionary reconstruction (the "engine runs the certified artifact" side).
//!
//! Holds the learned per-layer K/V dictionaries and, when enabled, reconstructs each K/V vector on the
//! KV-cache **write** path: encode the vector against its layer's dictionary (OMP → k-sparse code) and
//! replace it with the reconstruction. Attention then reads the lossy vectors, so a perplexity run with
//! this on measures the dictionary's EXACT quality impact — reconstruct-on-write and store-code-then-
//! reconstruct-on-read produce the identical reconstructed vector, so ΔPPL here is faithful to a real
//! compressed KV cache (the compressed *storage* layout that realizes the memory saving is Phase 4b).
//!
//! Gated + calibration/forge-side for now (behind the `wgsl-forge` feature): certification runs with the
//! feature on. Shipping this in the feature-off engine is Phase 4b (move the dictionary apply into core
//! + a compressed cache layout). Tapped from `gguf_bridge::…::cpu_attention_pass` before the cache write.

#![cfg(not(target_arch = "wasm32"))]

use super::kv_dictionary::KvDictionary;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(false);

struct Rt {
    /// Per-layer K dictionaries (`None` = layer not certified / too few vectors → passthrough).
    k: Vec<Option<KvDictionary>>,
    v: Vec<Option<KvDictionary>>,
    sparsity: usize,
}

fn rt() -> &'static Mutex<Option<Rt>> {
    static R: OnceLock<Mutex<Option<Rt>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(None))
}

/// Install the per-layer dictionaries and turn reconstruction ON.
pub fn enable(k: Vec<Option<KvDictionary>>, v: Vec<Option<KvDictionary>>, sparsity: usize) {
    if let Ok(mut g) = rt().lock() {
        *g = Some(Rt { k, v, sparsity });
    }
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
}

#[inline]
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Free the installed dictionaries.
pub fn clear() {
    if let Ok(mut g) = rt().lock() {
        *g = None;
    }
}

/// Reconstruct each of the `n_kv` head vectors in `proj` (length ≥ `n_kv * head_dim`) through this
/// layer's dictionary, in place. No-op (one atomic load) when disabled, when the layer has no
/// dictionary, or on a head_dim mismatch — so the caller stores the original vector unchanged.
#[inline]
pub fn reconstruct_kv(layer: usize, k_not_v: bool, proj: &mut [f32], n_kv: usize, head_dim: usize) {
    if !ENABLED.load(Ordering::Relaxed) || head_dim == 0 {
        return;
    }
    let Ok(g) = rt().lock() else {
        return;
    };
    let Some(rt) = g.as_ref() else {
        return;
    };
    let dicts = if k_not_v { &rt.k } else { &rt.v };
    let Some(Some(dict)) = dicts.get(layer) else {
        return;
    };
    if dict.dim != head_dim {
        return;
    }
    for h in 0..n_kv {
        let s = h * head_dim;
        if s + head_dim > proj.len() {
            break;
        }
        let code = dict.encode(&proj[s..s + head_dim], rt.sparsity);
        let recon = dict.reconstruct(&code);
        proj[s..s + head_dim].copy_from_slice(&recon);
    }
}
