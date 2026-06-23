// topk_reduction.wgsl — STELLAR §A A1a: GPU block top-K reduction (decision D4/D5).
//
// Each workgroup reduces a contiguous BLOCK of `logits` to its local top-K
// (value, global-index) candidates via K iterative parallel-argmax passes
// (NOT a full-vocab bitonic sort — we need top-K with K << N). The host merges
// every block's K candidates into the global top-K. This replaces the per-token
// 196 KB full-logit readback + CPU argmax with a ~(num_blocks × K)-pair readback.
//
// Semantics: NaN → -inf (never selected); ties broken toward the LOWER index
// (deterministic). K=1 degenerates to a parallel argmax.
//
// Bindings (auto layout, native): 0=logits(read) 1=params(uniform)
//   2=cand_val(read_write) 3=cand_idx(read_write).

const WG: u32 = 64u;
const MAX_BLOCK: u32 = 1024u;
const NEG_INF: f32 = -3.4028235e38;

struct Params {
    n: u32,           // total number of logits
    k: u32,           // top-K per block (caller guarantees k >= 1)
    block_size: u32,  // elements per workgroup (<= MAX_BLOCK)
    _pad: u32,
};

@group(0) @binding(0) var<storage, read> logits: array<f32>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<storage, read_write> cand_val: array<f32>;
@group(0) @binding(3) var<storage, read_write> cand_idx: array<u32>;

// Block logits, mutated (masked) across the K rounds.
var<workgroup> s_val: array<f32, MAX_BLOCK>;
// Per-thread reduction scratch.
var<workgroup> r_val: array<f32, WG>;
var<workgroup> r_idx: array<u32, WG>;

@compute @workgroup_size(WG)
fn topk_block(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let tid = lid.x;
    let blk = wg.x;
    let base = blk * params.block_size;
    let bsize = min(params.block_size, MAX_BLOCK);

    // Load block into shared memory; NaN and out-of-range → -inf.
    var i = tid;
    loop {
        if (i >= bsize) { break; }
        let g = base + i;
        var v = NEG_INF;
        if (g < params.n) {
            let raw = logits[g];
            if (raw == raw) { v = raw; } // NaN != NaN ⇒ leaves -inf
        }
        s_val[i] = v;
        i = i + WG;
    }
    workgroupBarrier();

    // K iterative parallel-argmax passes.
    var round = 0u;
    loop {
        if (round >= params.k) { break; }

        // Each thread scans its strided slice for a local best (lowest idx on ties).
        var best_v = NEG_INF;
        var best_i = 0u;
        var j = tid;
        loop {
            if (j >= bsize) { break; }
            let v = s_val[j];
            if (v > best_v) { best_v = v; best_i = j; }
            j = j + WG;
        }
        r_val[tid] = best_v;
        r_idx[tid] = best_i;
        workgroupBarrier();

        // Tree reduction; on equal values keep the lower index.
        var stride = WG / 2u;
        loop {
            if (stride == 0u) { break; }
            if (tid < stride) {
                let ov = r_val[tid + stride];
                let oi = r_idx[tid + stride];
                let cv = r_val[tid];
                let ci = r_idx[tid];
                if (ov > cv || (ov == cv && oi < ci)) {
                    r_val[tid] = ov;
                    r_idx[tid] = oi;
                }
            }
            workgroupBarrier();
            stride = stride / 2u;
        }

        // Thread 0 emits the winner (global index) and masks it for the next round.
        if (tid == 0u) {
            let widx = r_idx[0];
            let out = blk * params.k + round;
            cand_val[out] = r_val[0];
            cand_idx[out] = base + widx;
            s_val[widx] = NEG_INF;
        }
        workgroupBarrier();
        round = round + 1u;
    }
}
