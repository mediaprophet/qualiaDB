//! W5b — integration test for the KV-dictionary learner (`wgsl_forge::calibration::kv_dictionary`).
//!
//! Runs as an INTEGRATION test (links the lib compiled normally), so it executes even while a lib
//! `#[cfg(test)]` unit test elsewhere fails to compile. Validates the MOD+OMP learner on synthetic
//! data: (1) data that genuinely lives on a union of low-dim subspaces is reconstructed with small
//! error, (2) k>1 sparse coding beats k=1 (VQ), (3) incompressible (full-rank random) data yields a
//! high error — so the reconstruction metric is meaningful, not vacuous.

#![cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]

use qualia_core_db::wgsl_forge::calibration::kv_dictionary::{learn_dictionary, KvDictionary};

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
