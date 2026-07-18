// Mel-filterbank apply — project a power spectrum onto a mel filterbank.
//
// This is a CERTIFIED forge kernel: its exact CPU oracle and the naga-validation /
// GPU-certify tests live in `wgsl_forge::audio::mel`, which embeds this file via
// `include_str!` so there is a single source of truth.
//
// Row-major buffers (no `vec`/matrix padding):
//   spectrum : n_frames × n_bins  (power spectrum; frame-major)
//   mel_fb   : n_mel    × n_bins  (triangular filterbank weights; mel-band-major)
//   mel_out  : n_frames × n_mel   (result; frame-major)
//   params   : [ n_frames, n_bins, n_mel ]   (as f32, converted to u32)
//
// One invocation per OUTPUT element `i = gid.x` (guarded against the output length).
// The output index decomposes as `frame = i / n_mel`, `m = i % n_mel`, and the value is
//   mel_out[frame*n_mel + m] = Σ_b spectrum[frame*n_bins + b] * mel_fb[m*n_bins + b].
// This is a plain matrix–matrix contraction over the shared `n_bins` axis; there is no
// data-dependent branch, so the result is bit-for-bit reproducible by the CPU oracle
// (accumulation in increasing-`b` order).

@group(0) @binding(0) var<storage, read> spectrum: array<f32>;
@group(0) @binding(1) var<storage, read> mel_fb: array<f32>;
@group(0) @binding(2) var<storage, read_write> mel_out: array<f32>;
@group(0) @binding(3) var<storage, read> params: array<f32>;

@compute @workgroup_size(64)
fn mel_apply(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&mel_out)) {
        return;
    }
    let n_bins = u32(params[1]);
    let n_mel = u32(params[2]);

    let frame = i / n_mel;
    let m = i - frame * n_mel;

    let spec_base = frame * n_bins;
    let fb_base = m * n_bins;

    var acc = 0.0;
    for (var b: u32 = 0u; b < n_bins; b = b + 1u) {
        acc = acc + spectrum[spec_base + b] * mel_fb[fb_base + b];
    }
    mel_out[i] = acc;
}
