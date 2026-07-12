//! Self-Consistent Field (SCF) Iterative Driver
//!
//! Solves the generalized Roothaan-Hall equation (FC = SCE) to find the molecular
//! ground state electronic energy.
//!
//! Implements Direct Inversion in the Iterative Subspace (DIIS) to aggressively accelerate
//! convergence, strictly bounded within the zero-heap constraints.

use super::super::shared::zero_heap_algebra::ZeroHeapMatrix;

/// Subspace size for DIIS. Maximum historical Fock/Density/Error vectors kept.
pub const DIIS_SUBSPACE_SIZE: usize = 8;
pub const SCF_CONVERGENCE_THRESHOLD: f64 = 1e-8;
pub const MAX_SCF_ITERATIONS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScfFormalism {
    Restricted,   // RHF (Closed-shell)
    Unrestricted, // UHF (Open-shell)
}

#[derive(Debug)]
pub enum ScfError {
    ConvergenceFailed,
    SingularDiisMatrix,
    InvalidEigenvalueDecomposition,
}

/// Gaussian elimination solver for small DIIS mixing matrices.
/// Solves Ax = b where A is an N x N matrix, returning x.
/// All done strictly on the stack.
pub fn gaussian_elimination<const N: usize>(
    mut a: ZeroHeapMatrix<f64, N, N>,
    mut b: [f64; N],
) -> Result<[f64; N], ScfError> {
    // Forward elimination
    for i in 0..N {
        // Find pivot
        let mut max_row = i;
        let mut max_val = a.get(i, i).abs();
        for k in (i + 1)..N {
            let val = a.get(k, i).abs();
            if val > max_val {
                max_val = val;
                max_row = k;
            }
        }

        if max_val < 1e-14 {
            return Err(ScfError::SingularDiisMatrix);
        }

        // Swap rows
        if i != max_row {
            for j in i..N {
                let temp = a.get(i, j);
                a.set(i, j, a.get(max_row, j));
                a.set(max_row, j, temp);
            }
            let temp_b = b[i];
            b[i] = b[max_row];
            b[max_row] = temp_b;
        }

        // Eliminate
        for k in (i + 1)..N {
            let factor = a.get(k, i) / a.get(i, i);
            for j in i..N {
                let new_val = a.get(k, j) - factor * a.get(i, j);
                a.set(k, j, new_val);
            }
            b[k] -= factor * b[i];
        }
    }

    // Back substitution
    let mut x = [0.0; N];
    for i in (0..N).rev() {
        let mut sum = 0.0;
        for j in (i + 1)..N {
            sum += a.get(i, j) * x[j];
        }
        x[i] = (b[i] - sum) / a.get(i, i);
    }

    Ok(x)
}

/// Zero-heap Jacobi eigenvalue algorithm for real symmetric matrices.
/// Returns eigenvalues and eigenvectors.
pub fn jacobi_diagonalization<const N: usize>(
    matrix: &ZeroHeapMatrix<f64, N, N>,
) -> Result<([f64; N], ZeroHeapMatrix<f64, N, N>), ScfError> {
    let mut a = *matrix;
    let mut v = ZeroHeapMatrix::<f64, N, N>::zeros();
    for i in 0..N {
        v.set(i, i, 1.0);
    }

    let max_sweeps = 50;
    let eps = 1e-15;

    for _sweep in 0..max_sweeps {
        let mut max_off_diag: f64 = 0.0;

        for p in 0..N {
            for q in (p + 1)..N {
                max_off_diag = f64::max(max_off_diag, a.get(p, q).abs());
            }
        }

        if max_off_diag < eps {
            let mut eigenvalues = [0.0; N];
            for i in 0..N {
                eigenvalues[i] = a.get(i, i);
            }
            // Sort eigenvalues and eigenvectors
            for i in 0..N {
                for j in (i + 1)..N {
                    if eigenvalues[i] > eigenvalues[j] {
                        let temp_val = eigenvalues[i];
                        eigenvalues[i] = eigenvalues[j];
                        eigenvalues[j] = temp_val;

                        for k in 0..N {
                            let temp_v = v.get(k, i);
                            v.set(k, i, v.get(k, j));
                            v.set(k, j, temp_v);
                        }
                    }
                }
            }
            return Ok((eigenvalues, v));
        }

        for p in 0..N {
            for q in (p + 1)..N {
                let apq = a.get(p, q);
                if apq.abs() > eps {
                    let app = a.get(p, p);
                    let aqq = a.get(q, q);
                    let theta = 0.5 * (2.0 * apq).atan2(aqq - app);
                    let c = theta.cos();
                    let s = theta.sin();

                    for i in 0..N {
                        if i != p && i != q {
                            let aip = a.get(i, p);
                            let aiq = a.get(i, q);
                            a.set(i, p, c * aip - s * aiq);
                            a.set(p, i, a.get(i, p));

                            a.set(i, q, s * aip + c * aiq);
                            a.set(q, i, a.get(i, q));
                        }

                        let vip = v.get(i, p);
                        let viq = v.get(i, q);
                        v.set(i, p, c * vip - s * viq);
                        v.set(i, q, s * vip + c * viq);
                    }

                    let a_pp_new = c * c * app - 2.0 * s * c * apq + s * s * aqq;
                    let a_qq_new = s * s * app + 2.0 * s * c * apq + c * c * aqq;

                    a.set(p, p, a_pp_new);
                    a.set(q, q, a_qq_new);
                    a.set(p, q, 0.0);
                    a.set(q, p, 0.0);
                }
            }
        }
    }

    Err(ScfError::InvalidEigenvalueDecomposition)
}

/// Helper function to perform matrix transposition on zero heap.
pub fn transpose<const N: usize>(m: &ZeroHeapMatrix<f64, N, N>) -> ZeroHeapMatrix<f64, N, N> {
    let mut out = ZeroHeapMatrix::<f64, N, N>::zeros();
    for i in 0..N {
        for j in 0..N {
            out.set(i, j, m.get(j, i));
        }
    }
    out
}

/// Calculate symmetric orthogonalization matrix X = S^(-1/2)
pub fn orthogonalization_matrix<const N: usize>(
    s: &ZeroHeapMatrix<f64, N, N>,
) -> Result<ZeroHeapMatrix<f64, N, N>, ScfError> {
    let (evals, evecs) = jacobi_diagonalization(s)?;

    // Form s^(-1/2)
    let mut d_inv_sqrt = ZeroHeapMatrix::<f64, N, N>::zeros();
    for i in 0..N {
        if evals[i] < 1e-12 {
            // Drop linearly dependent basis functions or singular values
            d_inv_sqrt.set(i, i, 0.0);
        } else {
            d_inv_sqrt.set(i, i, 1.0 / evals[i].sqrt());
        }
    }

    // X = V * D^(-1/2) * V^T
    let evecs_t = transpose(&evecs);
    let x = evecs * d_inv_sqrt * evecs_t;
    Ok(x)
}

/// Perform a full Restricted Hartree-Fock SCF iteration with DIIS
pub fn solve_rhf_scf<const N: usize>(
    h_core: &ZeroHeapMatrix<f64, N, N>, // One-electron Hamiltonian
    s: &ZeroHeapMatrix<f64, N, N>,      // Overlap Matrix
    eri: &ZeroHeapMatrix<f64, N, N>, // Two-electron repulsion integrals (mock 2D mapped for test)
    num_electrons: usize,            // Total electrons
) -> Result<f64, ScfError> {
    let x = orthogonalization_matrix(s)?;
    let x_t = transpose(&x);
    let mut density = ZeroHeapMatrix::<f64, N, N>::zeros();
    let mut old_energy = 0.0;

    // DIIS History arrays (Zero heap constraint)
    let mut error_vectors = [ZeroHeapMatrix::<f64, N, N>::zeros(); DIIS_SUBSPACE_SIZE];
    let mut fock_history = [ZeroHeapMatrix::<f64, N, N>::zeros(); DIIS_SUBSPACE_SIZE];
    let mut diis_count = 0;
    let mut diis_index = 0;

    for iter in 0..MAX_SCF_ITERATIONS {
        // 1. Build Fock Matrix (F = H + G(P))
        let mut fock = *h_core;
        for mu in 0..N {
            for nu in 0..N {
                let mut g = 0.0;
                for lam in 0..N {
                    for sig in 0..N {
                        // Normally this would index a 4D ERI tensor (mu, nu | lam, sig).
                        // In this mock, we map it to 2D for demonstration by collapsing indices.
                        let eri_val = eri.get((mu + lam) % N, (nu + sig) % N);
                        // J - 0.5 * K (Coulomb - Exchange)
                        g += density.get(lam, sig)
                            * (eri_val - 0.5 * eri.get((mu + sig) % N, (nu + lam) % N));
                    }
                }
                fock.set(mu, nu, fock.get(mu, nu) + g);
            }
        }

        // 2. Compute DIIS error vector: e = FDS - SDF
        let fds = fock * density * (*s);
        let sdf = (*s) * density * fock;
        let mut err_vec = ZeroHeapMatrix::<f64, N, N>::zeros();
        for i in 0..N {
            for j in 0..N {
                err_vec.set(i, j, fds.get(i, j) - sdf.get(i, j));
            }
        }

        // Transform error vector to orthogonal basis: e' = X^T * e * X
        let err_ortho = x_t * err_vec * x;
        let mut max_err: f64 = 0.0;
        for i in 0..N {
            for j in 0..N {
                max_err = max_err.max(err_ortho.get(i, j).abs());
            }
        }

        // 3. DIIS Extrapolation
        fock_history[diis_index] = fock;
        error_vectors[diis_index] = err_ortho;
        if diis_count < DIIS_SUBSPACE_SIZE {
            diis_count += 1;
        }
        diis_index = (diis_index + 1) % DIIS_SUBSPACE_SIZE;

        let mut fock_extrapolated = fock;
        if diis_count > 1 {
            // Build Pulay matrix B
            let _b_size = diis_count + 1;
            let mut b_matrix = ZeroHeapMatrix::<
                f64,
                { DIIS_SUBSPACE_SIZE + 1 },
                { DIIS_SUBSPACE_SIZE + 1 },
            >::zeros();
            for i in 0..diis_count {
                for j in 0..diis_count {
                    let mut dot = 0.0;
                    for mu in 0..N {
                        for nu in 0..N {
                            dot += error_vectors[i].get(mu, nu) * error_vectors[j].get(mu, nu);
                        }
                    }
                    b_matrix.set(i, j, dot);
                }
                b_matrix.set(i, diis_count, -1.0);
                b_matrix.set(diis_count, i, -1.0);
            }
            b_matrix.set(diis_count, diis_count, 0.0);

            let mut rhs = [0.0; DIIS_SUBSPACE_SIZE + 1];
            rhs[diis_count] = -1.0;

            // Use our Gaussian elimination to solve B * c = rhs
            if let Ok(c) = gaussian_elimination(b_matrix, rhs) {
                fock_extrapolated = ZeroHeapMatrix::<f64, N, N>::zeros();
                for i in 0..diis_count {
                    for mu in 0..N {
                        for nu in 0..N {
                            fock_extrapolated.set(
                                mu,
                                nu,
                                fock_extrapolated.get(mu, nu) + c[i] * fock_history[i].get(mu, nu),
                            );
                        }
                    }
                }
            }
        }

        // 4. Transform Fock matrix to orthogonal basis: F' = X^T * F * X
        let f_prime = x_t * fock_extrapolated * x;

        // 5. Diagonalize F' to get eigenvalues and C'
        let (_evals, c_prime) = jacobi_diagonalization(&f_prime)?;

        // 6. Back transform C' to original basis: C = X * C'
        let c = x * c_prime;

        // 7. Build new density matrix P = 2 * C_occ * C_occ^T
        let mut new_density = ZeroHeapMatrix::<f64, N, N>::zeros();
        let num_occ = num_electrons / 2;
        for mu in 0..N {
            for nu in 0..N {
                let mut sum = 0.0;
                for a in 0..num_occ {
                    sum += c.get(mu, a) * c.get(nu, a);
                }
                new_density.set(mu, nu, 2.0 * sum);
            }
        }

        // 8. Calculate Electronic Energy
        let mut energy = 0.0;
        for mu in 0..N {
            for nu in 0..N {
                energy += 0.5
                    * new_density.get(mu, nu)
                    * (h_core.get(mu, nu) + fock_extrapolated.get(mu, nu));
            }
        }

        // Check both Energy and DIIS error convergence
        if iter > 0 && (energy - old_energy).abs() < SCF_CONVERGENCE_THRESHOLD && max_err < 1e-6 {
            return Ok(energy);
        }
        old_energy = energy;
        density = new_density;
    }

    Err(ScfError::ConvergenceFailed)
}

/// Converged RHF result: the electronic energy plus everything a caller needs to
/// compute post-SCF observables (orbital energies for HOMO/LUMO, the density and
/// overlap-consistent MO coefficients for Mulliken populations and the dipole).
#[derive(Debug, Clone, Copy)]
pub struct RhfResult<const N: usize> {
    /// Electronic energy E_elec = ½ Σ_μν P_μν (H_μν + F_μν), in Hartree.
    pub electronic_energy: f64,
    /// Orbital (eigen)energies ε, ascending. `orbital_energies[0..num_occ]` are
    /// occupied, the rest virtual.
    pub orbital_energies: [f64; N],
    /// Converged density matrix P_μν = 2 Σ_a^occ C_μa C_νa.
    pub density: ZeroHeapMatrix<f64, N, N>,
    /// MO coefficients C in the original (non-orthogonal) AO basis.
    pub coefficients: ZeroHeapMatrix<f64, N, N>,
    /// Number of doubly-occupied orbitals.
    pub num_occ: usize,
    /// SCF iterations taken to converge.
    pub iterations: usize,
}

/// Full Restricted Hartree-Fock SCF with a REAL 4-index two-electron contraction
/// and DIIS acceleration.
///
/// The Fock build is the genuine
///   G_μν = Σ_λσ P_λσ [ (μν|λσ) − ½ (μσ|λν) ]
/// over the supplied 4-index ERI tensor `eri[μ][ν][λ][σ] = (μν|λσ)` in chemists'
/// notation — not the 2-D index-collapse mock used by [`solve_rhf_scf`]. The core
/// Hamiltonian `h_core = T + V_nuc`, overlap `s`, and the ERI tensor must all be
/// assembled from real molecular integrals by the caller.
///
/// Returns the converged [`RhfResult`] (electronic energy only — add the nuclear
/// repulsion for the total). Requires an even electron count (closed shell).
pub fn solve_rhf_scf_4index<const N: usize>(
    h_core: &ZeroHeapMatrix<f64, N, N>,
    s: &ZeroHeapMatrix<f64, N, N>,
    eri: &[[[[f64; N]; N]; N]; N],
    num_electrons: usize,
) -> Result<RhfResult<N>, ScfError> {
    let x = orthogonalization_matrix(s)?;
    let x_t = transpose(&x);
    let mut density = ZeroHeapMatrix::<f64, N, N>::zeros();
    let mut old_energy = 0.0;
    let num_occ = num_electrons / 2;

    let mut error_vectors = [ZeroHeapMatrix::<f64, N, N>::zeros(); DIIS_SUBSPACE_SIZE];
    let mut fock_history = [ZeroHeapMatrix::<f64, N, N>::zeros(); DIIS_SUBSPACE_SIZE];
    let mut diis_count = 0;
    let mut diis_index = 0;

    for iter in 0..MAX_SCF_ITERATIONS {
        // 1. Build the Fock matrix F = H + G(P) with the TRUE 4-index contraction.
        let mut fock = *h_core;
        for mu in 0..N {
            for nu in 0..N {
                let mut g = 0.0;
                for lam in 0..N {
                    for sig in 0..N {
                        // Coulomb (μν|λσ) minus half exchange (μσ|λν).
                        let coulomb = eri[mu][nu][lam][sig];
                        let exchange = eri[mu][sig][lam][nu];
                        g += density.get(lam, sig) * (coulomb - 0.5 * exchange);
                    }
                }
                fock.set(mu, nu, fock.get(mu, nu) + g);
            }
        }

        // 2. DIIS error vector e = FDS − SDF, transformed to the orthogonal basis.
        let fds = fock * density * (*s);
        let sdf = (*s) * density * fock;
        let mut err_vec = ZeroHeapMatrix::<f64, N, N>::zeros();
        for i in 0..N {
            for j in 0..N {
                err_vec.set(i, j, fds.get(i, j) - sdf.get(i, j));
            }
        }
        let err_ortho = x_t * err_vec * x;
        let mut max_err: f64 = 0.0;
        for i in 0..N {
            for j in 0..N {
                max_err = max_err.max(err_ortho.get(i, j).abs());
            }
        }

        // 3. DIIS extrapolation of the Fock matrix.
        fock_history[diis_index] = fock;
        error_vectors[diis_index] = err_ortho;
        if diis_count < DIIS_SUBSPACE_SIZE {
            diis_count += 1;
        }
        diis_index = (diis_index + 1) % DIIS_SUBSPACE_SIZE;

        let mut fock_extrapolated = fock;
        if diis_count > 1 {
            let mut b_matrix = ZeroHeapMatrix::<
                f64,
                { DIIS_SUBSPACE_SIZE + 1 },
                { DIIS_SUBSPACE_SIZE + 1 },
            >::zeros();
            for i in 0..diis_count {
                for j in 0..diis_count {
                    let mut dot = 0.0;
                    for mu in 0..N {
                        for nu in 0..N {
                            dot += error_vectors[i].get(mu, nu) * error_vectors[j].get(mu, nu);
                        }
                    }
                    b_matrix.set(i, j, dot);
                }
                b_matrix.set(i, diis_count, -1.0);
                b_matrix.set(diis_count, i, -1.0);
            }
            b_matrix.set(diis_count, diis_count, 0.0);

            let mut rhs = [0.0; DIIS_SUBSPACE_SIZE + 1];
            rhs[diis_count] = -1.0;

            if let Ok(c) = gaussian_elimination(b_matrix, rhs) {
                fock_extrapolated = ZeroHeapMatrix::<f64, N, N>::zeros();
                for i in 0..diis_count {
                    for mu in 0..N {
                        for nu in 0..N {
                            fock_extrapolated.set(
                                mu,
                                nu,
                                fock_extrapolated.get(mu, nu) + c[i] * fock_history[i].get(mu, nu),
                            );
                        }
                    }
                }
            }
        }

        // 4. F' = X^T F X, diagonalize, back-transform C = X C'.
        let f_prime = x_t * fock_extrapolated * x;
        let (evals, c_prime) = jacobi_diagonalization(&f_prime)?;
        let c = x * c_prime;

        // 5. New density P = 2 Σ_a^occ C_μa C_νa.
        let mut new_density = ZeroHeapMatrix::<f64, N, N>::zeros();
        for mu in 0..N {
            for nu in 0..N {
                let mut sum = 0.0;
                for a in 0..num_occ {
                    sum += c.get(mu, a) * c.get(nu, a);
                }
                new_density.set(mu, nu, 2.0 * sum);
            }
        }

        // 6. Electronic energy E = ½ Σ P_μν (H_μν + F_μν) using the UN-extrapolated
        //    consistent Fock for the current density.
        let mut fock_for_energy = *h_core;
        for mu in 0..N {
            for nu in 0..N {
                let mut g = 0.0;
                for lam in 0..N {
                    for sig in 0..N {
                        let coulomb = eri[mu][nu][lam][sig];
                        let exchange = eri[mu][sig][lam][nu];
                        g += new_density.get(lam, sig) * (coulomb - 0.5 * exchange);
                    }
                }
                fock_for_energy.set(mu, nu, fock_for_energy.get(mu, nu) + g);
            }
        }
        let mut energy = 0.0;
        for mu in 0..N {
            for nu in 0..N {
                energy +=
                    0.5 * new_density.get(mu, nu) * (h_core.get(mu, nu) + fock_for_energy.get(mu, nu));
            }
        }

        if iter > 0 && (energy - old_energy).abs() < SCF_CONVERGENCE_THRESHOLD && max_err < 1e-6 {
            return Ok(RhfResult {
                electronic_energy: energy,
                orbital_energies: evals,
                density: new_density,
                coefficients: c,
                num_occ,
                iterations: iter + 1,
            });
        }
        old_energy = energy;
        density = new_density;
    }

    Err(ScfError::ConvergenceFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_elimination() {
        let mut a = ZeroHeapMatrix::<f64, 2, 2>::zeros();
        a.set(0, 0, 3.0);
        a.set(0, 1, 2.0);
        a.set(1, 0, 1.0);
        a.set(1, 1, 4.0);
        let b = [7.0, 9.0];

        let x = gaussian_elimination(a, b).unwrap();
        assert!((x[0] - 1.0).abs() < 1e-10); // x = 1
        assert!((x[1] - 2.0).abs() < 1e-10); // y = 2
    }

    #[test]
    fn test_jacobi_diagonalization() {
        let mut a = ZeroHeapMatrix::<f64, 2, 2>::zeros();
        a.set(0, 0, 2.0);
        a.set(0, 1, 1.0);
        a.set(1, 0, 1.0);
        a.set(1, 1, 2.0);

        let (evals, _) = jacobi_diagonalization(&a).unwrap();
        // Eigenvalues of [[2, 1], [1, 2]] are 1 and 3.
        // It sorts ascending, so 1.0 then 3.0
        assert!((evals[0] - 1.0).abs() < 1e-10);
        assert!((evals[1] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_rhf_scf_convergence() {
        // Extremely simple H2 minimal basis mock
        let mut h_core = ZeroHeapMatrix::<f64, 2, 2>::zeros();
        h_core.set(0, 0, -1.1);
        h_core.set(1, 1, -1.1);
        h_core.set(0, 1, -0.9);
        h_core.set(1, 0, -0.9);

        let mut s = ZeroHeapMatrix::<f64, 2, 2>::zeros();
        s.set(0, 0, 1.0);
        s.set(1, 1, 1.0);
        s.set(0, 1, 0.5);
        s.set(1, 0, 0.5);

        let eri = ZeroHeapMatrix::<f64, 2, 2>::zeros();
        let energy = solve_rhf_scf(&h_core, &s, &eri, 2).expect("SCF should converge");

        assert!(energy < 0.0);
    }
}
