//! W5b Phase 4b — the KV-dictionary type and its sparse **codec**, in CORE.
//!
//! This is the "engine runs the certified artifact" half of the sparse-KV-dictionary work. A learned
//! per-layer dictionary `D` of `n_atoms` unit atoms represents each KV vector as a **k-sparse** linear
//! combination — `k` (atom_index, coefficient) pairs. The engine needs two operations at runtime, and
//! neither may depend on the forge feature:
//!   * [`KvDictionary::encode`] — Orthogonal Matching Pursuit: a vector → its k-sparse code (write path).
//!   * [`KvDictionary::reconstruct`] — code → vector (read path, in attention).
//!
//! The MOD **learner** that PRODUCES a dictionary is a forge/training step and lives in
//! `wgsl_forge::calibration::kv_dictionary` (which re-exports this type). Colocation would blur "forge
//! produces, engine runs", so only the data + codec + the small numeric helpers they share live here.
//! Pure CPU + `f32`; the GPU reconstruction shader (Phase 4b step 5) mirrors [`reconstruct`] in WGSL.

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
    pub(crate) fn atom(&self, a: usize) -> &[f32] {
        &self.atoms[a * self.dim..(a + 1) * self.dim]
    }

    /// Reconstruct a vector from its sparse code: `Σ coeffs[i] · atom(indices[i])`. The read-path
    /// operation the attention shader mirrors.
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

#[inline]
pub(crate) fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[inline]
pub(crate) fn l2(v: &[f32]) -> f32 {
    dot(v, v).sqrt()
}

#[inline]
pub(crate) fn normalize(v: &mut [f32]) {
    let n = l2(v);
    if n > 1e-12 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}
