//! p64 → forge bridge: read a transcoded model's role-tagged weights out of a p64 volume as f32,
//! laid out for the forge decode-layer graph.
//!
//! GGUF/p64 store a projection weight as `[out,in]` row-major (ne[0]=in is the contiguous dim).
//! The forge's plain `MatMul(m=1,n=out,k=in)` wants B as `[in,out]` = `[k,n]`, so the bridge
//! transposes each 2-D projection weight once at load (a cheap one-time cost; the result is then
//! uploaded resident). The certified `MatMul.trans_b` path *can* consume `[out,in]` with no copy
//! for the full-matmul weights, but the per-head output projection still wants `[in,out]` for
//! contiguous per-head slicing, so this first version transposes uniformly. Norms are 1-D (no
//! transpose).

use crate::q42::p64_weight::{
    P64TensorIndex, P64_ROLE_ATTN_NORM, P64_ROLE_ATTN_OUTPUT, P64_ROLE_ATTN_Q, P64_ROLE_FFN_DOWN,
    P64_ROLE_FFN_GATE, P64_ROLE_FFN_NORM, P64_ROLE_FFN_UP,
};

/// A dequantized weight tensor with its (row-major) dims.
#[derive(Debug, Clone)]
pub struct P64Tensor {
    pub data: Vec<f32>,
    pub dims: Vec<u32>,
}

/// Dequantize one role's tensor for layer `layer` to f32, exactly as stored (row-major).
pub fn read_role(
    index: &P64TensorIndex,
    data: &[u8],
    role: u16,
    layer: u32,
) -> Result<P64Tensor, String> {
    let entry = index
        .entries
        .iter()
        .find(|e| e.role_id == role && e.manifold_idx == layer)
        .ok_or_else(|| format!("p64 bridge: role {role} layer {layer} not found"))?;
    let rank = entry.rank as usize;
    let dims: Vec<u32> = entry.dimensions[..rank].to_vec();
    let n_elems: usize = dims.iter().map(|&d| d as usize).product::<usize>().max(1);
    let blob = index.blob(data, entry);
    let mut out = vec![0.0f32; n_elems];
    crate::inference::ggml_quants::dequantize_row_into(blob, entry.dtype as u32, n_elems, &mut out)
        .map_err(|e| format!("p64 bridge dequant role {role} layer {layer}: {e:?}"))?;
    Ok(P64Tensor { data: out, dims })
}

/// Transpose a `[d0,d1]` row-major tensor to `[d1,d0]` row-major.
fn transpose_2d(t: &P64Tensor) -> Result<Vec<f32>, String> {
    if t.dims.len() != 2 {
        return Err(format!("transpose_2d: expected rank-2, got dims {:?}", t.dims));
    }
    let (r, c) = (t.dims[0] as usize, t.dims[1] as usize);
    let mut out = vec![0.0f32; r * c];
    for i in 0..r {
        for j in 0..c {
            out[j * r + i] = t.data[i * c + j];
        }
    }
    Ok(out)
}

/// The per-layer projection / FFN / norm weights a forge decode layer needs, in the `[in,out]`
/// layout (2-D projections transposed from the native `[out,in]`); norms are 1-D.
#[derive(Debug, Clone)]
pub struct ForgeLayerWeights {
    pub wq: Vec<f32>,        // [in=d, out=d]
    pub wo: Vec<f32>,        // [in=d, out=d]
    pub wg: Vec<f32>,        // [in=d, out=ffn]
    pub wu: Vec<f32>,        // [in=d, out=ffn]
    pub wd: Vec<f32>,        // [in=ffn, out=d]
    pub attn_norm: Vec<f32>, // [d]
    pub ffn_norm: Vec<f32>,  // [d]
}

/// Read + transpose layer `layer`'s decode weights from a p64 volume, ready for
/// `decode_layer_graph` externals (`Wq,Wo,Wg,Wu,Wd,attn_norm,ffn_norm`). K/V projection weights are
/// not read here — attention reads K/V from the cache, and the current token's K/V projection +
/// cache append is the decode-loop integration step.
pub fn read_forge_layer_weights(
    index: &P64TensorIndex,
    data: &[u8],
    layer: u32,
) -> Result<ForgeLayerWeights, String> {
    Ok(ForgeLayerWeights {
        wq: transpose_2d(&read_role(index, data, P64_ROLE_ATTN_Q, layer)?)?,
        wo: transpose_2d(&read_role(index, data, P64_ROLE_ATTN_OUTPUT, layer)?)?,
        wg: transpose_2d(&read_role(index, data, P64_ROLE_FFN_GATE, layer)?)?,
        wu: transpose_2d(&read_role(index, data, P64_ROLE_FFN_UP, layer)?)?,
        wd: transpose_2d(&read_role(index, data, P64_ROLE_FFN_DOWN, layer)?)?,
        attn_norm: read_role(index, data, P64_ROLE_ATTN_NORM, layer)?.data,
        ffn_norm: read_role(index, data, P64_ROLE_FFN_NORM, layer)?.data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::graph_ops::executor::{
        decode_layer_graph, execute_graph, execute_graph_cpu,
    };

    /// Locate a SmolLM2-360M GGUF (env override, Timothy's model dir, or docs/models).
    fn find_smollm_gguf() -> Option<std::path::PathBuf> {
        if let Ok(p) = std::env::var("QUALIA_TEST_MODEL") {
            let pb = std::path::PathBuf::from(p);
            if pb.exists() {
                return Some(pb);
            }
        }
        [
            "C:/LLM_Models/GGUF/smollm2-360m-instruct-q8_0.gguf",
            "C:/LLM_Models/GGUF/lmstudio-community/smollm2-360m-instruct-q8_0.gguf",
            "C:/LLM_Models/GGUF/lmstudio-community/SmolLM2-360M-Instruct-GGUF/SmolLM2-360M-Instruct-Q8_0.gguf",
            "docs/models/smollm2-360m-instruct-q8_0.gguf",
            "docs/models/SmolLM2-360M-Instruct-Q8_0.gguf",
        ]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.exists())
    }

    /// The real bake-off (correctness half): the forge decode layer running on **actual
    /// SmolLM2-360M layer-0 weights** (read from p64 via the bridge) matches the composed CPU
    /// oracle. The weights, dims (n_embd 960, 15 heads / 5 kv-heads / head_dim 64, ffn 2560) and
    /// RoPE base (100k) are the model's real values; x and the KV cache are synthetic (the layer
    /// *compute* on real weights is what's certified). Skips cleanly if no model is on disk.
    #[test]
    #[ignore = "requires a SmolLM2 GGUF on disk"]
    fn forge_decode_layer_on_real_p64_weights_matches_oracle() {
        let Some(path) = find_smollm_gguf() else {
            eprintln!("[bridge] no SmolLM2 GGUF found — skipping real-weights cert");
            return;
        };
        let gguf = std::fs::read(&path).expect("read gguf");
        let p64 = crate::q42::p64_weight::compile_gguf_to_p64(&gguf, 14).expect("compile gguf->p64");
        drop(gguf);
        let index = P64TensorIndex::from_p64(&p64).expect("from_p64");
        let h = &index.hparams;
        let n_head = h.n_head;
        let n_kv = if h.n_kv_head > 0 { h.n_kv_head } else { h.n_head };
        let n_embd = h.n_embd;
        let head_dim = n_embd / n_head;
        let d = n_head * head_dim;
        let theta_base = if h.rope_freq_base > 0.0 { h.rope_freq_base } else { 10000.0 };

        let w = read_forge_layer_weights(&index, &p64, 0).expect("read layer 0 weights");
        let ffn = (w.wg.len() as u32) / d; // wg is [in=d, out=ffn]
        assert_eq!(w.wq.len(), (d * d) as usize, "Wq shape");
        assert_eq!(w.attn_norm.len(), d as usize, "attn_norm shape");

        let (seq, pos) = (8u32, 3u32);
        let inv_scale = 1.0f32 / (head_dim as f32).sqrt();
        let gen = |len: usize, salt: usize| -> Vec<f32> {
            (0..len)
                .map(|i| (((i * 7 + salt * 13) % 23) as f32) * 0.02 - 0.23)
                .collect()
        };
        let ext = vec![
            gen(d as usize, 1),                            // x
            gen((n_kv * head_dim * seq) as usize, 2),      // Kt [n_kv, head_dim, seq]
            gen((n_kv * seq * head_dim) as usize, 3),      // V  [n_kv, seq, head_dim]
            w.wq,
            w.wo,
            w.wg,
            w.wu,
            w.wd,
            w.attn_norm,
            w.ffn_norm,
            vec![inv_scale],
            vec![1e-5],
        ];
        let g = decode_layer_graph(n_head, n_kv, head_dim, seq, ffn, pos, 0, theta_base).unwrap();
        let gpu = execute_graph(&g, &ext).expect("forge decode layer on real p64 weights");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        assert_eq!(gpu.len(), d as usize);
        let mut max_rel = 0.0f32;
        for (a, b) in gpu.iter().zip(&cpu) {
            let rel = (a - b).abs() / b.abs().max(1.0);
            max_rel = max_rel.max(rel);
            assert!(rel <= 2e-2, "real-weights forge vs oracle: {a} vs {b} (rel {rel})");
        }
        println!(
            "[bridge] SmolLM2-360M layer-0 on the forge: n_embd={n_embd} heads={n_head} kv={n_kv} \
             head_dim={head_dim} ffn={ffn} rope_base={theta_base} | forge==oracle (max rel \
             {max_rel:.2e}) on REAL p64-dequantized weights"
        );
    }

    /// Profiles the forge **certification executor's** per-layer cost on real SmolLM2-360M weights
    /// (resident weights, warm pipeline cache, single-encoder submit). IMPORTANT framing: this is the
    /// forge's oracle-diff *certification* harness running the decode graph node-by-node — it is **not**
    /// the inference runtime, and this number is **not** a forge-vs-engine runtime comparison. The
    /// engine (`gguf_bridge`) is the runtime (18.32 tok/s decode on this A2000, Vulkan); the forge's job
    /// is to *produce + certify* kernels, not to run them. Recorded only as a forge profiling datum (it
    /// also omits the current-token K/V projection + cache append). Skips if no model is on disk.
    #[test]
    #[ignore = "requires a SmolLM2 GGUF on disk"]
    fn forge_decode_layer_real_weights_ms_per_layer() {
        use crate::wgsl_forge::graph_ops::executor::{decode_layer_graph, ForgeGraphExecutor};
        use std::time::Instant;
        let Some(path) = find_smollm_gguf() else {
            eprintln!("[forge-bench] no SmolLM2 GGUF found — skipping");
            return;
        };
        let gguf = std::fs::read(&path).expect("read gguf");
        let p64 = crate::q42::p64_weight::compile_gguf_to_p64(&gguf, 14).expect("compile gguf->p64");
        drop(gguf);
        let index = P64TensorIndex::from_p64(&p64).expect("from_p64");
        let h = &index.hparams;
        let n_head = h.n_head;
        let n_kv = if h.n_kv_head > 0 { h.n_kv_head } else { h.n_head };
        let head_dim = h.n_embd / n_head;
        let d = n_head * head_dim;
        let theta = if h.rope_freq_base > 0.0 { h.rope_freq_base } else { 10000.0 };
        let w = read_forge_layer_weights(&index, &p64, 0).expect("read layer 0");
        let ffn = (w.wg.len() as u32) / d;
        let (seq, pos) = (24u32, 23u32);
        let inv_scale = 1.0f32 / (head_dim as f32).sqrt();
        let gen = |len: usize, salt: usize| -> Vec<f32> {
            (0..len).map(|i| (((i * 7 + salt * 13) % 23) as f32) * 0.02 - 0.23).collect()
        };

        let g = decode_layer_graph(n_head, n_kv, head_dim, seq, ffn, pos, 0, theta).unwrap();
        let mut exec = ForgeGraphExecutor::on_shared_gpu().expect("forge on shared gpu");
        // Big matrices resident (indices 3..=9); activations uploaded per call (0,1,2,10,11).
        let resident = exec
            .load_weights(&[
                (3, w.wq), (4, w.wo), (5, w.wg), (6, w.wu), (7, w.wd),
                (8, w.attn_norm), (9, w.ffn_norm),
            ])
            .expect("load_weights");
        let acts = vec![
            gen(d as usize, 1),                       // x
            gen((n_kv * head_dim * seq) as usize, 2), // Kt
            gen((n_kv * seq * head_dim) as usize, 3), // V
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], // resident slots
            vec![inv_scale],
            vec![1e-5],
        ];

        for _ in 0..15 {
            let _ = exec.run_resident(&g, &acts, &resident).unwrap();
        }
        let iters = 100;
        let t = Instant::now();
        for _ in 0..iters {
            let _ = exec.run_resident(&g, &acts, &resident).unwrap();
        }
        let ms_layer = t.elapsed().as_secs_f64() * 1e3 / iters as f64;
        println!(
            "[forge-bench] SmolLM2-360M decode LAYER on the forge: {ms_layer:.3} ms/layer \
             (resident weights, warm cache, single-encoder submit, seq={seq}, d={d}, ffn={ffn}) \
             | engine baseline (measured, same A2000): 1.63 ms/layer forward, 18.32 tok/s decode. \
             PER-LAYER compute — forge omits current-token K/V projection (decode-loop seam); NOT \
             end-to-end tok/s."
        );
    }
}
