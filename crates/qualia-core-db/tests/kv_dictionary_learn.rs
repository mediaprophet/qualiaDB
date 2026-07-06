//! W5b — integration test for the KV-dictionary learner (`wgsl_forge::calibration::kv_dictionary`).
//!
//! Runs as an INTEGRATION test (links the lib compiled normally), so it executes even while a lib
//! `#[cfg(test)]` unit test elsewhere fails to compile. Validates the MOD+OMP learner on synthetic
//! data: (1) data that genuinely lives on a union of low-dim subspaces is reconstructed with small
//! error, (2) k>1 sparse coding beats k=1 (VQ), (3) incompressible (full-rank random) data yields a
//! high error — so the reconstruction metric is meaningful, not vacuous.

#![cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]

use qualia_core_db::wgsl_forge::calibration::kv_dictionary::{
    dict_code_bits_per_vector, int8_bits_per_vector, int8_reconstruction_error, learn_dictionary,
    uniform_reconstruction_error, KvDictionary,
};

/// Deterministic LCG → f32 in [-1, 1).
struct Rng(u64);
impl Rng {
    fn f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

fn unit(mut v: Vec<f32>) -> Vec<f32> {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 1e-12 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
    v
}

/// Build `m` vectors, each a `k`-sparse combination of `true_atoms` (a union of k-dim subspaces) plus
/// a little noise.
fn synth_subspace_data(rng: &mut Rng, true_atoms: &[Vec<f32>], dim: usize, m: usize, k: usize, noise: f32) -> Vec<Vec<f32>> {
    let d = true_atoms.len();
    (0..m)
        .map(|_| {
            let mut v = vec![0f32; dim];
            for _ in 0..k {
                let a = (rng.f32().abs() * d as f32) as usize % d;
                let c = rng.f32() * 2.0;
                for (vv, &av) in v.iter_mut().zip(&true_atoms[a]) {
                    *vv += c * av;
                }
            }
            for vv in v.iter_mut() {
                *vv += rng.f32() * noise;
            }
            v
        })
        .collect()
}

#[test]
fn learns_a_dictionary_that_reconstructs_subspace_data() {
    let mut rng = Rng(0x5eed_1234);
    let dim = 32usize;
    let n_true = 12usize; // the data lives on 12 atoms
    let k = 3usize; // 3-sparse
    let true_atoms: Vec<Vec<f32>> = (0..n_true)
        .map(|_| unit((0..dim).map(|_| rng.f32()).collect()))
        .collect();
    let data = synth_subspace_data(&mut rng, &true_atoms, dim, 600, k, 0.01);

    // Learn a dictionary with a few spare atoms over the true count.
    let dict: KvDictionary = learn_dictionary(&data, dim, 16, k, 40);
    let err = dict.reconstruction_error(&data, k);
    println!("[w5b] {n_true}-subspace data, dict=16 atoms k={k}: mean rel recon err = {err:.4}");
    // Reconstructs genuinely low-rank data well. The strict, meaningful claims below (beats VQ +
    // discriminates incompressible data) carry the real weight; this absolute bar just guards against
    // a non-converging learner (a MOD learner on noisy multi-subspace data lands well inside this).
    assert!(
        err < 0.20,
        "learner should reconstruct genuinely low-rank data reasonably (got {err:.4})"
    );

    // k>1 must beat k=1 (single-atom VQ) on this multi-subspace data.
    let err_k1 = dict.reconstruction_error(&data, 1);
    println!("[w5b] same dict, k=1 (VQ): mean rel recon err = {err_k1:.4}");
    assert!(
        err < err_k1,
        "k={k} sparse coding must beat k=1 VQ ({err:.4} vs {err_k1:.4})"
    );

    // Sanity: full-rank incompressible random data is NOT well reconstructed at the same footprint —
    // the metric is meaningful, not vacuously small.
    let mut rng2 = Rng(0xabcd_ef01);
    let rand_data: Vec<Vec<f32>> = (0..600)
        .map(|_| (0..dim).map(|_| rng2.f32()).collect())
        .collect();
    let rand_dict = learn_dictionary(&rand_data, dim, 16, k, 25);
    let rand_err = rand_dict.reconstruction_error(&rand_data, k);
    println!("[w5b] incompressible random data, dict=16 atoms k={k}: mean rel recon err = {rand_err:.4}");
    assert!(
        rand_err > err,
        "incompressible data must reconstruct worse than low-rank data ({rand_err:.4} vs {err:.4})"
    );

    println!("[w5b] PASS — MOD+OMP learner recovers subspace structure; metric discriminates.");
}

/// The go/no-go is a RATE-DISTORTION test, and it must discriminate in both directions. int8 (8
/// bits/elem) is a strong, accurate baseline that a low-rate sparse code will NOT beat on accuracy — so
/// the honest question is whether the learned dictionary beats NAIVE uniform quantization *at its own
/// bit rate*. On genuinely low-rank data it should (the learned basis captures the subspace a coarse
/// scalar grid can't); on incompressible data it should NOT (nothing to learn). If it "won" everywhere
/// the metric would be vacuous.
#[test]
fn dictionary_beats_uniform_at_matched_rate_only_on_low_rank_data() {
    let dim = 64usize;
    // 3-sparse over 64 atoms (6-bit index) + f16 coeff = 3*(6+16) = 66 bits/vec ≈ 1 bit/elem → matched
    // uniform baseline is ~1-2 bits/elem. int8 (544 bits) is far more accurate but ~8× the footprint.
    let n_atoms = 64usize;
    let k = 3usize;
    let code_bits = dict_code_bits_per_vector(n_atoms, k, 16);
    let matched_bits = ((code_bits / dim as f64).round() as u32).clamp(2, 8);

    // (a) Low-rank data (8 atoms, 3-sparse) — the regime a dictionary is designed for.
    let mut rng = Rng(0x1122_3344);
    let atoms: Vec<Vec<f32>> = (0..8)
        .map(|_| unit((0..dim).map(|_| rng.f32()).collect()))
        .collect();
    let low = synth_subspace_data(&mut rng, &atoms, dim, 1200, 3, 0.01);
    let dict = learn_dictionary(&low, dim, n_atoms, k, 30);
    let recon_dict = dict.reconstruction_error(&low, k);
    let recon_unif = uniform_reconstruction_error(&low, matched_bits);
    let recon_int8 = int8_reconstruction_error(&low);
    println!(
        "[w5b/gonogo] low-rank @ {code_bits:.0}b: dict {recon_dict:.4} vs uniform@{matched_bits}b {recon_unif:.4} (int8 {recon_int8:.4} @ {:.0}b)",
        int8_bits_per_vector(dim)
    );
    assert!(
        recon_dict < recon_unif,
        "on low-rank data the learned dictionary should beat matched-rate uniform quant ({recon_dict:.4} vs {recon_unif:.4})"
    );

    // (b) Incompressible data: nothing to learn → naive uniform quant should be no worse.
    let mut rng2 = Rng(0x9911_2244);
    let rand_data: Vec<Vec<f32>> = (0..1200)
        .map(|_| (0..dim).map(|_| rng2.f32()).collect())
        .collect();
    let rdict = learn_dictionary(&rand_data, dim, n_atoms, k, 20);
    let r_dict = rdict.reconstruction_error(&rand_data, k);
    let r_unif = uniform_reconstruction_error(&rand_data, matched_bits);
    println!("[w5b/gonogo] random @ {code_bits:.0}b: dict {r_dict:.4} vs uniform@{matched_bits}b {r_unif:.4}");
    assert!(
        r_unif <= r_dict,
        "on incompressible data uniform quant must be no worse than the dictionary ({r_unif:.4} vs {r_dict:.4}) — else the gate is vacuous"
    );

    // int8 footprint sanity: dim int8 elems + one f32 scale.
    assert_eq!(int8_bits_per_vector(64), (64 * 8 + 32) as f64);
    println!("[w5b/gonogo] PASS — the rate-distortion decision discriminates in both directions.");
}

/// Phase 4 runtime: `kv_dict_runtime::reconstruct_kv` replaces vectors in place with exactly what the
/// dictionary's encode→reconstruct produces (so a PPL run through it measures the dictionary's real
/// quality), is a no-op when disabled, and the dictionaries survive a CBOR round-trip (packaging).
#[test]
fn runtime_reconstruct_matches_dict_and_is_gated() {
    use qualia_core_db::wgsl_forge::calibration::kv_dict_runtime;

    let dim = 32usize;
    let k = 3usize;
    let mut rng = Rng(0x5151_2626);
    let atoms: Vec<Vec<f32>> = (0..6)
        .map(|_| unit((0..dim).map(|_| rng.f32()).collect()))
        .collect();
    let data = synth_subspace_data(&mut rng, &atoms, dim, 400, k, 0.01);
    let dict = learn_dictionary(&data, dim, 16, k, 25);

    // Two heads' worth of test vectors in one proj slice.
    let v0 = data[7].clone();
    let v1 = data[42].clone();
    let mut proj: Vec<f32> = v0.iter().chain(v1.iter()).copied().collect();
    let expected: Vec<f32> = [&v0, &v1]
        .iter()
        .flat_map(|v| dict.reconstruct(&dict.encode(v, k)))
        .collect();

    // Disabled → no-op.
    kv_dict_runtime::reconstruct_kv(0, true, &mut proj, 2, dim);
    assert_eq!(proj, [v0.clone(), v1.clone()].concat(), "no-op while disabled");

    // Enabled for layer 0 K → in-place reconstruction equals the dictionary's own encode→reconstruct.
    kv_dict_runtime::enable(vec![Some(dict.clone())], vec![None], k);
    kv_dict_runtime::reconstruct_kv(0, true, &mut proj, 2, dim);
    kv_dict_runtime::disable();
    kv_dict_runtime::clear();
    for (got, want) in proj.iter().zip(&expected) {
        assert!((got - want).abs() < 1e-5, "reconstruct_kv must match dict path: {got} vs {want}");
    }
    // Layer with no dictionary (V here) stays passthrough.
    let mut untouched = vec![1.0f32; 2 * dim];
    let snap = untouched.clone();
    kv_dict_runtime::enable(vec![Some(dict.clone())], vec![None], k);
    kv_dict_runtime::reconstruct_kv(0, false, &mut untouched, 2, dim); // V has None → passthrough
    kv_dict_runtime::disable();
    kv_dict_runtime::clear();
    assert_eq!(untouched, snap, "a layer with no dictionary must be passthrough");
    println!("[w5b/phase4] PASS — runtime reconstruct matches the dictionary and is properly gated.");
}
