//! W5b — KV-vector capture hook for sparse-dictionary calibration.
//!
//! Sibling of [`crate::llm_awq`] (AWQ activation capture): a gated forward-pass hook that records the
//! engine's real per-layer K and V vectors during a calibration forward over the eval corpus. The
//! sparse-dictionary go/no-go ([`crate::wgsl_forge::calibration`]) needs OUR engine's actual KV
//! geometry — GQA layout, RoPE convention, and layer shapes are engine-specific, so this cannot come
//! from synthetic data or another runtime.
//!
//! Capture point: native attention runs through `gguf_bridge::…::cpu_attention_pass` (the wasm-proven
//! CPU SDPA the native path routes through), which writes each token's post-RoPE K and pre-RoPE V into
//! the KV cache. We tap those exact vectors there — post-RoPE K is what the int8 KV cache would
//! quantize, so this is an apples-to-apples source for the int8-vs-dictionary comparison.
//!
//! Gated + bounded: off in production (one relaxed atomic load on the attention path). When on, it
//! appends into a per-layer buffer under a mutex, capped at `max_per_layer` vectors per layer per
//! stream (K, V) so memory stays bounded regardless of corpus length. Calibration-only — never a
//! production hot path.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(false);

struct KvBuf {
    /// Head-dim of the captured vectors; 0 until the first record self-sizes it.
    head_dim: usize,
    /// Cap on vectors per layer per stream.
    max_per_layer: usize,
    n_layer: usize,
    /// Per-layer flat blobs; `k[layer]` = captured K vectors concatenated (len = count * head_dim).
    k: Vec<Vec<f32>>,
    v: Vec<Vec<f32>>,
}

fn buf() -> &'static Mutex<Option<KvBuf>> {
    static B: OnceLock<Mutex<Option<KvBuf>>> = OnceLock::new();
    B.get_or_init(|| Mutex::new(None))
}

/// Begin a KV capture for a model with up to `n_layer` layers, keeping at most `max_per_layer` K and
/// `max_per_layer` V vectors per layer. `head_dim` self-sizes on the first recorded vector.
pub fn enable(n_layer: usize, max_per_layer: usize) {
    if let Ok(mut g) = buf().lock() {
        *g = Some(KvBuf {
            head_dim: 0,
            max_per_layer,
            n_layer,
            k: (0..n_layer).map(|_| Vec::new()).collect(),
            v: (0..n_layer).map(|_| Vec::new()).collect(),
        });
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

/// Record every head's K (`k_not_v = true`) or V vector from a projection slice. `proj` holds `n_kv`
/// contiguous head vectors of `head_dim` each (`[h0…][h1…]…`). No-op (one atomic load) when disabled;
/// stops appending to a layer/stream once its `max_per_layer` cap is hit.
#[inline]
pub fn record(layer: usize, k_not_v: bool, proj: &[f32], n_kv: usize, head_dim: usize) {
    if !ENABLED.load(Ordering::Relaxed) || head_dim == 0 {
        return;
    }
    let Ok(mut g) = buf().lock() else {
        return;
    };
    let Some(b) = g.as_mut() else {
        return;
    };
    if b.head_dim == 0 {
        b.head_dim = head_dim;
    }
    if b.head_dim != head_dim || layer >= b.n_layer {
        return;
    }
    let cap_floats = b.max_per_layer.saturating_mul(head_dim);
    let dst = if k_not_v {
        &mut b.k[layer]
    } else {
        &mut b.v[layer]
    };
    for h in 0..n_kv {
        if dst.len() >= cap_floats {
            break;
        }
        let s = h * head_dim;
        if s + head_dim > proj.len() {
            break;
        }
        dst.extend_from_slice(&proj[s..s + head_dim]);
    }
}

/// One layer's captured vectors, split into `head_dim`-length rows.
pub struct KvCapture {
    pub head_dim: usize,
    /// `k[layer]` = list of captured K vectors (each `head_dim` long).
    pub k: Vec<Vec<Vec<f32>>>,
    pub v: Vec<Vec<Vec<f32>>>,
}

impl KvCapture {
    /// Total K vectors captured across all layers.
    pub fn total_k(&self) -> usize {
        self.k.iter().map(|l| l.len()).sum()
    }
    /// Total V vectors captured across all layers.
    pub fn total_v(&self) -> usize {
        self.v.iter().map(|l| l.len()).sum()
    }
}

/// Copy out the captured vectors (splitting the flat per-layer blobs into rows). Returns `None` if
/// nothing was captured (capture never enabled, or the forward never hit the CPU attention path).
pub fn snapshot() -> Option<KvCapture> {
    let g = buf().lock().ok()?;
    let b = g.as_ref()?;
    if b.head_dim == 0 {
        return None;
    }
    let hd = b.head_dim;
    let rows = |flat: &Vec<f32>| -> Vec<Vec<f32>> {
        flat.chunks_exact(hd).map(|c| c.to_vec()).collect()
    };
    Some(KvCapture {
        head_dim: hd,
        k: b.k.iter().map(rows).collect(),
        v: b.v.iter().map(rows).collect(),
    })
}

/// Drop the capture buffer (free the calibration-only heap).
pub fn clear() {
    if let Ok(mut g) = buf().lock() {
        *g = None;
    }
}
