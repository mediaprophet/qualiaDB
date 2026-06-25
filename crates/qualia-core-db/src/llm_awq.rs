//! W1/AWQ — activation-statistics capture for Activation-aware Weight Quantization (no external libs).
//!
//! AWQ's premise: a weight's salience is set by the magnitude of the *activation* it multiplies, so
//! scaling each input channel by `s_j = max|X_j|` before quantizing preserves the channels that matter
//! — letting aggressive (ternary) quantization survive. This module is AWQ **step 1, the forward hook**:
//! during a calibration forward over the eval corpus it records, per FFN layer, the per-input-channel
//! max |activation| at the FFN input (post-`ffn_norm`, the input to the gate/up projections).
//!
//! Lock-free + gated: off in production (one relaxed atomic load on the FFN path). The accumulator is
//! `fetch_max` over `|x|.to_bits()` — valid because `|x| >= 0`, so u32 bit-order matches float order.
//! Heap (the stats buffer) is calibration-only, allocated once, never on a production hot path.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;

/// Upper bounds for the fixed stats buffer (covers up to ~8B-param transformer shapes).
const MAX_LAYERS: usize = 80;
const MAX_CHAN: usize = 8192;

static ENABLED: AtomicBool = AtomicBool::new(false);
static N_LAYER: AtomicU32 = AtomicU32::new(0);
static N_CHAN: AtomicU32 = AtomicU32::new(0);
/// Per-forward layer index (reset by [`begin_forward`]); the FFN hook increments it 0..n_layer-1.
static LAYER_CURSOR: AtomicU32 = AtomicU32::new(0);

/// `max |activation|` bits at `[layer * N_CHAN + chan]`. Allocated once at first use.
fn stats() -> &'static Vec<AtomicU32> {
    static STATS: OnceLock<Vec<AtomicU32>> = OnceLock::new();
    STATS.get_or_init(|| (0..MAX_LAYERS * MAX_CHAN).map(|_| AtomicU32::new(0)).collect())
}

/// Begin an AWQ calibration capture for a model with `n_layer` FFN layers of `n_chan` input channels.
pub fn enable(n_layer: u32, n_chan: u32) -> Result<(), String> {
    if n_layer as usize > MAX_LAYERS || n_chan as usize > MAX_CHAN {
        return Err(format!(
            "AWQ capture exceeds bounds: n_layer={n_layer} (max {MAX_LAYERS}), n_chan={n_chan} (max {MAX_CHAN})"
        ));
    }
    N_LAYER.store(n_layer, Ordering::Relaxed);
    N_CHAN.store(n_chan, Ordering::Relaxed);
    reset();
    ENABLED.store(true, Ordering::Relaxed);
    Ok(())
}

pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
}

/// Zero the accumulators + reset the per-forward layer cursor.
pub fn reset() {
    LAYER_CURSOR.store(0, Ordering::Relaxed);
    for a in stats().iter() {
        a.store(0, Ordering::Relaxed);
    }
}

/// Call once at the start of each token's forward so the layer cursor tracks layers 0..n_layer-1.
/// No-op (one atomic load) when capture is off.
#[inline]
pub fn begin_forward() {
    if ENABLED.load(Ordering::Relaxed) {
        LAYER_CURSOR.store(0, Ordering::Relaxed);
    }
}

/// Record one FFN layer's post-norm input channels. Called from the FFN forward; no-op when off.
#[inline]
pub fn record_ffn_input(x: &[f32]) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let n_chan = N_CHAN.load(Ordering::Relaxed) as usize;
    let n_layer = N_LAYER.load(Ordering::Relaxed) as usize;
    if n_chan == 0 {
        return;
    }
    let layer = LAYER_CURSOR.fetch_add(1, Ordering::Relaxed) as usize;
    if layer >= n_layer {
        return;
    }
    let s = stats();
    let base = layer * n_chan;
    let lim = x.len().min(n_chan);
    for c in 0..lim {
        // |x| >= 0 → u32 bit pattern is monotone in the float value, so fetch_max is a true max.
        s[base + c].fetch_max(x[c].abs().to_bits(), Ordering::Relaxed);
    }
}

/// Per-layer per-channel max |activation| (`[layer][chan]`). Snapshot after the calibration pass.
pub fn snapshot() -> Vec<Vec<f32>> {
    let n_layer = N_LAYER.load(Ordering::Relaxed) as usize;
    let n_chan = N_CHAN.load(Ordering::Relaxed) as usize;
    if n_chan == 0 {
        return Vec::new();
    }
    let s = stats();
    (0..n_layer)
        .map(|l| {
            (0..n_chan)
                .map(|c| f32::from_bits(s[l * n_chan + c].load(Ordering::Relaxed)))
                .collect()
        })
        .collect()
}

#[inline]
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
