//! W5b Phase 4b step 3 — the sparse-dictionary KV cache LAYOUT math.
//!
//! The compressed arena stores `dict_k` code-words per K/V vector (each word = `u16 atom-index |
//! f16 coefficient`) instead of `head_dim` floats. `code_index` must be a **dense, non-overlapping
//! bijection** into the buffer (no two distinct code slots collide, everything in bounds, no wasted
//! space) — get this wrong and the write/read paths (steps 4-5) corrupt the cache. Also checks the dict
//! layout is dramatically smaller than f32.

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::gguf_bridge::KvCacheLayout;
use std::collections::HashSet;

/// Hand-build a dict-mode layout (mirrors `from_hyperparams_mode`'s dict branch) so the addressing math
/// can be tested without a model / GPU / the installed-dictionary machinery.
fn dict_layout(n_layer: u32, max_context: u32, n_kv: u32, head_dim: u32, dict_k: u32) -> KvCacheLayout {
    let layer_stride = max_context * 2 * n_kv * dict_k;
    KvCacheLayout {
        max_context,
        n_layer,
        n_kv_head: n_kv,
        head_dim,
        slot_kv_elems: n_kv * head_dim,
        layer_stride,
        total_f32_elems: (n_layer as usize) * (layer_stride as usize),
        int8: false,
        dict_k,
    }
}

#[test]
fn code_index_is_a_dense_bijection() {
    let (n_layer, max_context, n_kv, head_dim, k) = (2u32, 16u32, 3u32, 8u32, 4u32);
    let l = dict_layout(n_layer, max_context, n_kv, head_dim, k);

    let mut seen = HashSet::new();
    for layer in 0..n_layer {
        for slot in 0..max_context {
            for h in 0..n_kv {
                for k_not_v in [true, false] {
                    for i in 0..k {
                        let idx = l.code_index(layer, slot, h, k_not_v, i);
                        assert!(
                            idx < l.total_f32_elems,
                            "code_index out of bounds: {idx} >= {}",
                            l.total_f32_elems
                        );
                        assert!(
                            seen.insert(idx),
                            "code_index collision at (l{layer} s{slot} h{h} k_not_v={k_not_v} i{i}) → {idx}"
                        );
                    }
                }
            }
        }
    }
    let expected = (n_layer * max_context * n_kv * 2 * k) as usize;
    assert_eq!(seen.len(), expected, "addressing must fill every code word (dense)");
    assert_eq!(l.total_f32_elems, expected, "buffer is exactly sized to the codes");

    // K and V regions of the same (layer, slot, head) must be disjoint.
    let k0 = l.code_index(0, 0, 0, true, 0);
    let v0 = l.code_index(0, 0, 0, false, 0);
    assert_ne!(k0, v0, "K and V code regions must not overlap");
}

#[test]
fn dict_layout_is_far_smaller_than_f32() {
    // SmolLM2-ish: head_dim 64, n_kv 5, k 5. Per slot: dict = 2·5·5 = 50 words vs f32 2·5·64 = 640.
    let (n_kv, head_dim, k, ctx, layers) = (5u32, 64u32, 5u32, 1024u32, 32u32);
    let dict = dict_layout(layers, ctx, n_kv, head_dim, k);
    let f32_stride = ctx * 2 * n_kv * head_dim;
    let f32_total = (layers as usize) * (f32_stride as usize);
    assert!(
        dict.total_f32_elems * 12 < f32_total,
        "dict arena must be ≥ ~12.8× smaller than f32 ({} vs {} words)",
        dict.total_f32_elems,
        f32_total
    );
}
