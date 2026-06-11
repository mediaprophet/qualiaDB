// lora_apply.wgsl — LoRA delta computation
//
// Implements the additive LoRA correction for a single linear layer:
//
//   output[i] += sum_k( lora_b[i, k] * sum_j( lora_a[k, j] * input[j] ) ) * scaling
//
// Both matrices are f32 row-major:
//   lora_a: [rank, n_in]   — down-projection (A matrix)
//   lora_b: [n_out, rank]  — up-projection   (B matrix)
//
// Dispatch: ceil(n_out / 64) workgroups of 64 threads.
// Each thread handles one output element.
//
// Workgroup memory is used to cache the intermediate z[rank] vector
// (A @ input) so it is only computed once per workgroup, not once per thread.
// This reduces the FLOP count from O(n_out * rank * n_in) to
// O(n_in * rank) + O(n_out * rank) — a significant saving when n_out >> rank.

struct LoraParams {
    n_in    : u32,
    n_out   : u32,
    rank    : u32,
    scaling : f32,
}

@group(0) @binding(0) var<storage, read_write> output : array<f32>;
@group(0) @binding(1) var<storage, read>       input  : array<f32>;
@group(0) @binding(2) var<storage, read>       lora_a : array<f32>;
@group(0) @binding(3) var<storage, read>       lora_b : array<f32>;
@group(0) @binding(4) var<uniform>             params : LoraParams;

// Workgroup-shared buffer for z = A @ input (length = rank).
// Rank is bounded by 256 in practice; 512 covers extreme cases.
var<workgroup> z_shared : array<f32, 512>;

@compute @workgroup_size(64)
fn apply_lora(
    @builtin(global_invocation_id) gid  : vec3<u32>,
    @builtin(local_invocation_id)  lid  : vec3<u32>,
    @builtin(workgroup_id)         wgid : vec3<u32>,
) {
    let local_id    = lid.x;
    let global_id   = gid.x;
    let n_in        = params.n_in;
    let n_out       = params.n_out;
    let rank        = params.rank;
    let scaling     = params.scaling;

    // ── Phase 1: z[k] = sum_j( A[k,j] * input[j] ) ──────────────────────────
    //
    // Each thread in the workgroup computes a contiguous slice of z.
    // With 64 threads and rank ≤ 512 this covers up to 8 rank-elements per thread.

    let elems_per_thread = (rank + 64u - 1u) / 64u;  // ceil(rank / 64)

    for (var e = 0u; e < elems_per_thread; e++) {
        let k = local_id + e * 64u;
        if (k < rank) {
            var acc = 0.0f;
            let row_base = k * n_in;
            for (var j = 0u; j < n_in; j++) {
                acc += lora_a[row_base + j] * input[j];
            }
            z_shared[k] = acc;
        }
    }

    // All threads must finish Phase 1 before Phase 2 reads z_shared.
    workgroupBarrier();

    // ── Phase 2: output[i] += sum_k( B[i,k] * z[k] ) * scaling ──────────────

    if (global_id >= n_out) {
        return;
    }

    var delta = 0.0f;
    let row_base = global_id * rank;
    for (var k = 0u; k < rank; k++) {
        delta += lora_b[row_base + k] * z_shared[k];
    }

    output[global_id] += delta * scaling;
}
