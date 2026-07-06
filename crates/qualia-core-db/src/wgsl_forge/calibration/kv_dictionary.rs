//! W5b — sparse KV-dictionary learning (the "training" step of the forge calibration pipeline).
//!
//! Lexico / Top-K-SAE style compression: instead of storing each KV vector raw (or int8), learn a
//! per-layer **dictionary** `D` of `n_atoms` unit atoms and represent every KV vector as a **K-sparse**
//! linear combination — `k` (atom_index, coefficient) pairs. Store the small dictionary once per layer
//! + `k` indices/coeffs per slot. This is a learned basis (unlike int8's fixed per-element grid), so at
//! the same footprint it can preserve the reference model's behaviour better — *if* the KV vectors of a
//! layer actually live near a low-dimensional union of subspaces, which this learner measures.
//!
//! Algorithm — **MOD (Method of Optimal Directions) + OMP**, the robust, simple cousin of k-SVD:
//! 1. init the dictionary from `n_atoms` distinct training vectors (normalized);
//! 2. repeat `iters` times:
//!    a. **sparse-code** every vector with Orthogonal Matching Pursuit to `k` atoms (greedy atom pick
//!       by max |⟨atom, residual⟩|, then re-solve all selected coefficients by least squares);
//!    b. **dictionary update** — least-squares refit `D = X · A⁺` (each atom = the LS-optimal direction
//!       given the current codes), then renormalize atoms; drop/reseed any unused atom.
//!
//! Pure CPU + `f32`, no GPU, unit-testable in isolation (see `tests/kv_dictionary_learn.rs`). The KV
//! **capture** (dumping the engine's per-layer K/V) and the ΔPPL certify wire this into
//! `run_calibration`; this module is just the learner + its reconstruction metric.

#![cfg(not(target_arch = "wasm32"))]

/// A learned per-layer KV dictionary: `n_atoms` atoms, each of length `dim`, row-major in `atoms`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KvDictionary {
    pub dim: usize,
    pub n_atoms: usize,
    /// `n_atoms × dim`, row-major; each atom is L2-normalized.
    pub atoms: Vec<f32>,
    /// Sparsity `k` this dictionary was trained for (atoms per coded vector).
    pub sparsity: usize,
}

/// A K-sparse code of one vector: `indices[i]` selects an atom, `coeffs[i]` its weight.
#[derive(Debug, Clone, PartialEq)]
pub struct SparseCode {
    pub indices: Vec<u32>,
    pub coeffs: Vec<f32>,
}

impl KvDictionary {
    #[inline]
    fn atom(&self, a: usize) -> &[f32] {
        &self.atoms[a * self.dim..(a + 1) * self.dim]
    }

    /// Reconstruct a vector from its sparse code: `Σ coeffs[i] · atom(indices[i])`.
    pub fn reconstruct(&self, code: &SparseCode) -> Vec<f32> {
        let mut out = vec![0f32; self.dim];
        for (idx, &c) in code.indices.iter().zip(&code.coeffs) {
            let atom = self.atom(*idx as usize);
            for (o, &a) in out.iter_mut().zip(atom) {
                *o += c * a;
            }
        }
        out
    }

    /// Orthogonal Matching Pursuit: encode `v` with at most `k` atoms. Greedy selection by maximum
    /// absolute correlation with the residual, re-solving all selected coefficients by least squares
    /// each step (the "orthogonal" in OMP). Stops early if the residual is already ~0.
    pub fn encode(&self, v: &[f32], k: usize) -> SparseCode {
        debug_assert_eq!(v.len(), self.dim);
        let k = k.min(self.n_atoms).max(1);
        let mut residual = v.to_vec();
        let mut selected: Vec<usize> = Vec::with_capacity(k);
        let mut coeffs: Vec<f32> = Vec::with_capacity(k);

        for _ in 0..k {
            // Pick the atom most correlated with the current residual (excluding already-picked).
            let mut best = usize::MAX;
            let mut best_abs = 0f32;
            for a in 0..self.n_atoms {
                if selected.contains(&a) {
                    continue;
                }
                let corr = dot(self.atom(a), &residual);
                if corr.abs() > best_abs {
                    best_abs = corr.abs();
                    best = a;
                }
            }
            if best == usize::MAX || best_abs <= 1e-12 {
                break;
            }
            selected.push(best);
            // Re-solve coefficients for ALL selected atoms by least squares (normal equations on the
            // small |S|×|S| Gram matrix), then recompute the residual.
            coeffs = least_squares_coeffs(&selected, self, v);
            residual = v.to_vec();
            for (&s, &c) in selected.iter().zip(&coeffs) {
                let atom = self.atom(s);
                for (r, &a) in residual.iter_mut().zip(atom) {
                    *r -= c * a;
                }
            }
            if l2(&residual) <= 1e-8 {
                break;
            }
        }
        SparseCode {
            indices: selected.iter().map(|&s| s as u32).collect(),
            coeffs,
        }
    }

    /// Mean relative reconstruction error over `vectors`: `mean_i ‖v_i − D·code_i‖ / ‖v_i‖`. The
    /// quality proxy compared against int8's quantization error before wiring the runtime.
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

/// Least-squares coefficients for the selected atoms fitting `v`: solve `(DₛᵀDₛ) c = Dₛᵀv` via the
/// normal equations (Gaussian elimination on the small `|S|×|S|` system).
fn least_squares_coeffs(selected: &[usize], dict: &KvDictionary, v: &[f32]) -> Vec<f32> {
    let s = selected.len();
    // Gram matrix G = Dₛᵀ Dₛ (s×s) and rhs = Dₛᵀ v (s).
    let mut g = vec![0f32; s * s];
    let mut rhs = vec![0f32; s];
    for i in 0..s {
        let ai = dict.atom(selected[i]);
        rhs[i] = dot(ai, v);
        for j in i..s {
            let aj = dict.atom(selected[j]);
            let val = dot(ai, aj);
            g[i * s + j] = val;
            g[j * s + i] = val;
        }
    }
    solve_spd(&mut g, &mut rhs, s);
    rhs
}

/// Solve a small dense symmetric system `G c = b` in place via Gaussian elimination with partial
/// pivoting (SPD in exact arithmetic; the pivoting + tiny ridge keep it robust to near-singular Gram
/// matrices from correlated atoms). Result written back into `b`.
fn solve_spd(g: &mut [f32], b: &mut [f32], n: usize) {
    // Tiny ridge for numerical stability against duplicate/near-duplicate atoms.
    for i in 0..n {
        g[i * n + i] += 1e-6;
    }
    for col in 0..n {
        // Partial pivot.
        let mut piv = col;
        let mut piv_abs = g[col * n + col].abs();
        for r in (col + 1)..n {
            let a = g[r * n + col].abs();
            if a > piv_abs {
                piv_abs = a;
                piv = r;
            }
        }
        if piv != col {
            for c in 0..n {
                g.swap(col * n + c, piv * n + c);
            }
            b.swap(col, piv);
        }
        let d = g[col * n + col];
        if d.abs() <= 1e-12 {
            continue;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = g[r * n + col] / d;
            if factor == 0.0 {
                continue;
            }
            for c in col..n {
                g[r * n + c] -= factor * g[col * n + c];
            }
            b[r] -= factor * b[col];
        }
    }
    for i in 0..n {
        let d = g[i * n + i];
        if d.abs() > 1e-12 {
            b[i] /= d;
        } else {
            b[i] = 0.0;
        }
    }
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

#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[inline]
fn l2(v: &[f32]) -> f32 {
    dot(v, v).sqrt()
}

#[inline]
fn normalize(v: &mut [f32]) {
    let n = l2(v);
    if n > 1e-12 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}
