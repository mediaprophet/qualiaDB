// "Mock Fused Contraction" — the placeholder GPU kernel used by the decode path
// ONLY when there is no real model mmap (i.e. tests). It is dispatched through
// the SAME 4-binding group-0 interface as the real `fused_transformer.wgsl`
// (input, weights, uniform params, output), so it must declare and reference
// all four bindings — otherwise naga's auto-derived layout prunes the unused
// ones and the shared 4-entry bind group no longer matches ("number of bindings
// … does not match"). It makes NO assumption about tensor dimensions: every
// index is taken modulo / clamped to the real buffer length, so it is
// bounds-safe on a tiny mock model. Output is shaped and position-varying so
// the plumbing (sample → tokens → grounded output) has something non-degenerate
// to run on; the values are meaningless (there is no real model).

struct GemmParams {
    n_in: u32,
    n_out: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32,
    n_batch: u32,
    in_row_stride: u32,
    out_row_stride: u32,
}

@group(0) @binding(0) var<storage, read> input_activations: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<f32>;
@group(0) @binding(2) var<uniform> params: GemmParams;
@group(0) @binding(3) var<storage, read_write> output_logits: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n_out = arrayLength(&output_logits);
    if i >= n_out {
        return;
    }
    let n_in = arrayLength(&input_activations);
    let n_w = arrayLength(&weights);
    if n_in == 0u || n_w == 0u {
        output_logits[i] = 0.0;
        return;
    }

    // A cheap bounds-safe contraction: sum input·weight over the real input
    // length (clamped by the declared n_in, so a huge/garbage param can't
    // over-read), touching `weights` and `params` so all four bindings stay
    // live in the auto layout.
    var limit = params.n_in;
    if limit == 0u || limit > n_in {
        limit = n_in;
    }
    let w = weights[i % n_w];
    var sum = 0.0;
    for (var k = 0u; k < limit; k = k + 1u) {
        sum = sum + input_activations[k] * w;
    }
    // Position-varying so argmax isn't trivially constant.
    output_logits[i] = sum + input_activations[i % n_in];
}
