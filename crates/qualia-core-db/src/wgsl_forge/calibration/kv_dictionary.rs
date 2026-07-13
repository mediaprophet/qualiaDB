//! W5b — the sparse KV-dictionary **learner** (the forge/training step) + its rate-distortion metrics.
//!
//! The dictionary type ([`KvDictionary`]) and its runtime codec (`encode`/`reconstruct`) live in CORE
//! (`crate::inference::kv_dict`) so the ENGINE can run a certified artifact without the forge feature;
//! they are re-exported here so existing paths (`wgsl_forge::calibration::kv_dictionary::*`, the CBOR
//! packaging, the tests) resolve unchanged. This module keeps the parts that are purely forge-side:
//!   * [`learn_dictionary`] — MOD (Method of Optimal Directions) + OMP training over captured KV;
//!   * the reconstruction-error / footprint metrics used by the go/no-go analysis.
//!
//! Algorithm — **MOD + OMP**, the robust, simple cousin of k-SVD:
//! 1. init the dictionary from `n_atoms` distinct training vectors (normalized);
//! 2. repeat `iters` times: (a) sparse-code every vector with OMP to `k` atoms; (b) least-squares refit
//!    each atom given the current codes, renormalize, drop/reseed any unused atom.

#![cfg(not(target_arch = "wasm32"))]

use crate::inference::kv_dict::{l2, normalize};
pub use crate::inference::kv_dict::{KvDictionary, SparseCode};

impl KvDictionary {
    /// Mean relative reconstruction error over `vectors`: `mean_i ‖v_i − D·code_i‖ / ‖v_i‖`. The
    /// quality proxy compared against uniform quantization in the go/no-go.
    pub fn reconstruction_error(&self, vectors: &[Vec<f32>], k: usize) -> f64 {
        if vectors.is_empty() {
            return 0.0;
        }
        let mut acc = 0f64;
        let mut n = 0usize;
        for v in vectors {
            let vn = l2(v);
            if vn <= 1e-12 {
                continue;
            }
            let recon = self.reconstruct(&self.encode(v, k));
            let mut err = 0f32;
            for (a, b) in v.iter().zip(&recon) {
                let d = a - b;
                err += d * d;
            }
            acc += (err.sqrt() / vn) as f64;
            n += 1;
        }
        if n == 0 {
            0.0
        } else {
            acc / n as f64
        }
    }
}

/// Learn a `KvDictionary` from `vectors` (all length `dim`) via MOD+OMP. `n_atoms` = dictionary size,
/// `k` = sparsity (atoms per coded vector), `iters` = training passes. Deterministic given the inputs
/// (atom init = evenly-spaced distinct training vectors), so a re-run reproduces the artifact.
pub fn learn_dictionary(
    vectors: &[Vec<f32>],
    dim: usize,
    n_atoms: usize,
    k: usize,
    iters: usize,
) -> KvDictionary {
    assert!(dim > 0 && n_atoms > 0, "dim and n_atoms must be > 0");
    let m = vectors.len();
    let n_atoms = n_atoms.min(m.max(1));
    let k = k.min(n_atoms).max(1);

    // Init: evenly-spaced distinct training vectors, L2-normalized. Fallback to a unit e_i basis when
    // fewer vectors than atoms.
    let mut atoms = vec![0f32; n_atoms * dim];
    for a in 0..n_atoms {
        let dst = &mut atoms[a * dim..(a + 1) * dim];
        if m > 0 {
            let src = &vectors[(a * m) / n_atoms.max(1)];
            dst.copy_from_slice(&src[..dim]);
        } else {
            dst[a % dim] = 1.0;
        }
        normalize(dst);
    }
    let mut dict = KvDictionary {
        dim,
        n_atoms,
        atoms,
        sparsity: k,
    };
    if m == 0 {
        return dict;
    }

    for _ in 0..iters {
        // a. Sparse-code every training vector.
        let codes: Vec<SparseCode> = vectors.iter().map(|v| dict.encode(v, k)).collect();

        // b. Dictionary update (MOD): for each atom, refit it as the LS-optimal direction given the
        //    vectors that use it. Atom_a ≈ (Σ_{i uses a} coeff_ia · residual_excluding_a) / Σ coeff².
        let mut new_atoms = vec![0f32; n_atoms * dim];
        let mut denom = vec![0f32; n_atoms];
        for (v, code) in vectors.iter().zip(&codes) {
            // residual excluding each selected atom = v − Σ_{j≠a} c_j·atom_j.
            for (pos, (&idx, &c)) in code.indices.iter().zip(&code.coeffs).enumerate() {
                let a = idx as usize;
                // e_a = v − Σ_{j≠pos} c_j·atom_j
                let mut e = v.clone();
                for (pos2, (&idx2, &c2)) in code.indices.iter().zip(&code.coeffs).enumerate() {
                    if pos2 == pos {
                        continue;
                    }
                    let atom2 = dict.atom(idx2 as usize);
                    for (ev, &av) in e.iter_mut().zip(atom2) {
                        *ev -= c2 * av;
                    }
                }
                let dst = &mut new_atoms[a * dim..(a + 1) * dim];
                for (d, &ev) in dst.iter_mut().zip(&e) {
                    *d += c * ev;
                }
                denom[a] += c * c;
            }
        }
        for a in 0..n_atoms {
            let dst = &mut new_atoms[a * dim..(a + 1) * dim];
            if denom[a] > 1e-9 {
                for d in dst.iter_mut() {
                    *d /= denom[a];
                }
                normalize(dst);
            } else {
                // Unused atom → reseed from the current worst-reconstructed vector (keeps the
                // dictionary expressive instead of leaving a dead atom).
                let worst = worst_vector(&dict, vectors, k);
                dst.copy_from_slice(&vectors[worst][..dim]);
                normalize(dst);
            }
        }
        dict.atoms = new_atoms;
    }
    dict
}

/// Index of the vector with the largest relative reconstruction error (for reseeding a dead atom).
fn worst_vector(dict: &KvDictionary, vectors: &[Vec<f32>], k: usize) -> usize {
    let mut worst = 0usize;
    let mut worst_err = -1f32;
    for (i, v) in vectors.iter().enumerate() {
        let vn = l2(v);
        if vn <= 1e-12 {
            continue;
        }
        let recon = dict.reconstruct(&dict.encode(v, k));
        let mut err = 0f32;
        for (a, b) in v.iter().zip(&recon) {
            let d = a - b;
            err += d * d;
        }
        let rel = err.sqrt() / vn;
        if rel > worst_err {
            worst_err = rel;
            worst = i;
        }
    }
    worst
}

/// Mean relative L2 error of **per-vector symmetric uniform** quantization to `bits` bits: one f32
/// scale per vector, `scale = max|v| / (2^(bits-1) − 1)`, `q_i = round(v_i / scale)` clamped, dequant
/// `v'_i = q_i · scale`. This is the "naive scalar quantizer" baseline a learned codebook must beat *at
/// the same bit rate* to be worthwhile. Vectors with ~0 norm are skipped (they quantize exactly).
pub fn uniform_reconstruction_error(vectors: &[Vec<f32>], bits: u32) -> f64 {
    let levels = ((1u32 << bits.clamp(1, 16).saturating_sub(1)) - 1).max(1) as f32;
    let mut acc = 0f64;
    let mut n = 0usize;
    for v in vectors {
        let vn = l2(v);
        if vn <= 1e-12 {
            continue;
        }
        let amax = v.iter().fold(0f32, |m, &x| m.max(x.abs()));
        if amax <= 1e-12 {
            continue;
        }
        let scale = amax / levels;
        let mut err = 0f32;
        for &x in v {
            let q = (x / scale).round().clamp(-levels, levels);
            let d = x - q * scale;
            err += d * d;
        }
        acc += (err.sqrt() / vn) as f64;
        n += 1;
    }
    if n == 0 {
        0.0
    } else {
        acc / n as f64
    }
}

/// The W5a incumbent: per-vector symmetric **int8** KV reconstruction error (uniform at 8 bits).
pub fn int8_reconstruction_error(vectors: &[Vec<f32>]) -> f64 {
    uniform_reconstruction_error(vectors, 8)
}

/// Footprint of `bits`-bit uniform KV per vector: `dim` codes + one f32 scale.
pub fn uniform_bits_per_vector(dim: usize, bits: u32) -> f64 {
    (dim * bits as usize + 32) as f64
}

/// Footprint of int8 KV per vector (uniform at 8 bits): `dim` int8 elements + one f32 scale.
pub fn int8_bits_per_vector(dim: usize) -> f64 {
    uniform_bits_per_vector(dim, 8)
}

/// Bits per stored index for a dictionary of `n_atoms` atoms: `ceil(log2 n_atoms)`.
pub fn index_bits(n_atoms: usize) -> f64 {
    (usize::BITS - n_atoms.saturating_sub(1).leading_zeros()).max(1) as f64
}

/// **Asymptotic** (code-only) footprint of a k-sparse dictionary code per vector: `k` × (index +
/// coefficient). At deployment the shared dictionary amortizes over millions of cached vectors → ~0,
/// so this is the rate that matters for the rate-distortion comparison against uniform quantization.
pub fn dict_code_bits_per_vector(n_atoms: usize, k: usize, coeff_bits: usize) -> f64 {
    k as f64 * (index_bits(n_atoms) + coeff_bits as f64)
}

/// **Full** footprint of a k-sparse dictionary over exactly `n_vectors` vectors: the code plus the
/// per-vector amortized share of the shared `n_atoms · dim` f32 dictionary. Use this to report the
/// realized footprint of a finite capture (where the dictionary is NOT yet amortized away).
pub fn dict_bits_per_vector(
    n_atoms: usize,
    k: usize,
    dim: usize,
    n_vectors: usize,
    coeff_bits: usize,
) -> f64 {
    let dict_amortized = if n_vectors == 0 {
        0.0
    } else {
        (n_atoms * dim * 32) as f64 / n_vectors as f64
    };
    dict_code_bits_per_vector(n_atoms, k, coeff_bits) + dict_amortized
}
