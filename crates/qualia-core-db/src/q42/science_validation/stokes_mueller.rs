//! Stokes vector and Mueller $4\times 4$ matrix polarimetric realizability verifiers (C02 Mode 1 & Mode 4).
//!
//! All reference calculations are evaluated from first principles in independent `f64` arithmetic:
//! - Overflow-safe scaled 3D hypotenuse $\operatorname{hypot}_3(S_1, S_2, S_3)$.
//! - Physical Stokes admission: $S_0 \ge 0$ and
//!   $\operatorname{hypot}_3(S_1, S_2, S_3) \le S_0 + 64\,\varepsilon \max(S_0, \operatorname{hypot}_3(S_1, S_2, S_3))$.
//! - Degree of Polarization $\text{DoP} = \operatorname{hypot}_3(S_1, S_2, S_3) / S_0 \in [0, 1]$,
//!   never clamping source components to mask over-polarized states.
//! - Cloude $4\times 4$ Hermitian coherency matrix $\mathbf{C}(\mathbf{M})$ constructed from the 16
//!   Pauli-basis Dirac-Kronig projections in the candidate C02 $V$-sign convention, with
//!   deterministic complex Hermitian Jacobi eigenvalue verification ($\lambda_i \ge -64\,\varepsilon \|\mathbf{M}\|_F$).
//! - Fry-Kattawar trace/transmittance passivity conditions and Jones-to-Mueller exact equivalence.

/// Machine epsilon for IEEE-754 `f64` ($2^{-52}$).
pub const F64_EPS: f64 = f64::EPSILON;

/// Standard C02 small-operation rounding allowance factor ($64 \times 2^{-52}$).
pub const C02_ROUNDING_FACTOR: f64 = 64.0 * F64_EPS;

/// Exact defined speed of light in vacuum $c_0$ ($\text{m/s}$).
pub const VACUUM_SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;

/// Structured rejection reason for Stokes or Mueller physical realizability failures.
#[derive(Debug, Clone, PartialEq)]
pub enum PolarimetricValidationError {
    NonFiniteStokesComponent { index: usize, value: f64 },
    NegativeTotalIntensity { s0: f64 },
    OverPolarizedState { s0: f64, polarized_norm: f64, allowance: f64, excess: f64 },
    InvalidPropagationDirection { norm_sq: f64 },
    InvalidFrequencyHz { frequency_hz: f64 },
    InvalidBandwidthHz { bandwidth_hz: f64 },
    InvalidGeometricPathM { path_m: f64 },
    NonFiniteMuellerEntry { row: usize, col: usize, value: f64 },
    NegativeMuellerM00 { m00: f64 },
    ActiveGainViolation { max_forward_gain: f64, max_column_gain: f64, tolerance: f64 },
    CloudeNegativeEigenvalue { min_eigenvalue: f64, eigenvalues: [f64; 4], tolerance: f64 },
    JonesMuellerMismatch { row: usize, col: usize, actual: f64, reference: f64, abs_error: f64, budget: f64 },
}

/// Verified Stokes physical state summary (retaining original unmodified components).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerifiedStokesState {
    pub stokes: [f64; 4],
    pub polarized_norm: f64,
    pub raw_degree_of_polarization: f64,
    pub degree_of_polarization: f64,
    pub realizability_margin: f64,
}

/// Verified Mueller $4\times 4$ physical realizability report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerifiedMuellerReport {
    pub frobenius_norm: f64,
    pub cloude_trace: f64,
    pub cloude_eigenvalues: [f64; 4],
    pub max_transmittance_gain: f64,
    pub max_polarizance_gain: f64,
    pub depolarization_index: f64,
}

/// Complex scalar in `f64` precision for independent Jones & Cloude algebra.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    #[inline]
    pub const fn new(re: f64, im: f64) -> Self { Self { re, im } }
    #[inline]
    pub const fn conj(self) -> Self { Self { re: self.re, im: -self.im } }
    #[inline]
    pub fn norm_sq(self) -> f64 { self.re * self.re + self.im * self.im }
    #[inline]
    pub fn abs(self) -> f64 { self.re.hypot(self.im) }
    #[inline]
    pub fn add(self, rhs: Self) -> Self { Self { re: self.re + rhs.re, im: self.im + rhs.im } }
    #[inline]
    pub fn sub(self, rhs: Self) -> Self { Self { re: self.re - rhs.re, im: self.im - rhs.im } }
    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        Self { re: self.re * rhs.re - self.im * rhs.im, im: self.re * rhs.im + self.im * rhs.re }
    }
    #[inline]
    pub fn scale(self, s: f64) -> Self { Self { re: self.re * s, im: self.im * s } }
}

/// Overflow- and underflow-safe 3D Euclidean norm $\sqrt{x^2 + y^2 + z^2}$.
#[inline]
pub fn scaled_hypot3(x: f64, y: f64, z: f64) -> f64 {
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let max_c = ax.max(ay).max(az);
    if max_c == 0.0 || !max_c.is_finite() {
        return max_c;
    }
    let (sx, sy, sz) = (ax / max_c, ay / max_c, az / max_c);
    max_c * ((sx * sx + sy * sy) + sz * sz).sqrt()
}

/// Verify physical Stokes realizability per C02 Mode 1.
/// Never clamps or modifies input components.
pub fn verify_stokes_realizability(
    stokes: [f64; 4],
) -> Result<VerifiedStokesState, PolarimetricValidationError> {
    for (index, &value) in stokes.iter().enumerate() {
        if !value.is_finite() {
            return Err(PolarimetricValidationError::NonFiniteStokesComponent { index, value });
        }
    }
    let s0 = stokes[0];
    if s0 < 0.0 {
        return Err(PolarimetricValidationError::NegativeTotalIntensity { s0 });
    }
    let polarized_norm = scaled_hypot3(stokes[1], stokes[2], stokes[3]);
    if !polarized_norm.is_finite() {
        return Err(PolarimetricValidationError::NonFiniteStokesComponent {
            index: 1,
            value: polarized_norm,
        });
    }
    let scale = s0.max(polarized_norm);
    let allowance = C02_ROUNDING_FACTOR * scale;
    let margin = s0 - polarized_norm;
    if polarized_norm > s0 + allowance {
        return Err(PolarimetricValidationError::OverPolarizedState {
            s0,
            polarized_norm,
            allowance,
            excess: polarized_norm - s0,
        });
    }
    let (raw_dop, dop) = if s0 == 0.0 {
        (0.0, 0.0)
    } else {
        let raw = polarized_norm / s0;
        (raw, raw.min(1.0))
    };
    Ok(VerifiedStokesState {
        stokes,
        polarized_norm,
        raw_degree_of_polarization: raw_dop,
        degree_of_polarization: dop,
        realizability_margin: margin,
    })
}

/// Convert a complex Jones vector $(E_x, E_y)$ to a Stokes vector $(S_0, S_1, S_2, S_3)$
/// in the C02 Mode 1 convention where $(1, i)/\sqrt{2} \mapsto (1, 0, 0, -1)$.
pub fn jones_vector_to_stokes(ex: Complex64, ey: Complex64) -> [f64; 4] {
    let (ex2, ey2, cross) = (ex.norm_sq(), ey.norm_sq(), ex.mul(ey.conj()));
    [ex2 + ey2, ex2 - ey2, 2.0 * cross.re, 2.0 * cross.im]
}

/// Passive transverse basis rotation of a Stokes vector by angle $\theta$ ($\text{rad}$).
pub fn rotate_stokes_transverse_basis(stokes: [f64; 4], theta_rad: f64) -> [f64; 4] {
    let (c, s) = ((2.0 * theta_rad).cos(), (2.0 * theta_rad).sin());
    let (q, u) = (stokes[1], stokes[2]);
    [stokes[0], q * c + u * s, -q * s + u * c, stokes[3]]
}

/// Deterministic transverse basis $(\mathbf{e}_1, \mathbf{e}_2)$ for propagation unit vector $\mathbf{k}$.
pub fn deterministic_transverse_basis(
    direction: [f64; 3],
) -> Result<([f64; 3], [f64; 3]), PolarimetricValidationError> {
    let norm = scaled_hypot3(direction[0], direction[1], direction[2]);
    if !norm.is_finite() || (norm - 1.0).abs() > 1e-12 {
        return Err(PolarimetricValidationError::InvalidPropagationDirection { norm_sq: norm * norm });
    }
    let k = [direction[0] / norm, direction[1] / norm, direction[2] / norm];
    if (k[2] - 1.0).abs() <= 1e-12 && k[0].abs() <= 1e-12 && k[1].abs() <= 1e-12 {
        return Ok(([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
    }
    if (k[2] + 1.0).abs() <= 1e-12 && k[0].abs() <= 1e-12 && k[1].abs() <= 1e-12 {
        return Ok(([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]));
    }
    let seed = if k[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let dot = seed[0] * k[0] + seed[1] * k[1] + seed[2] * k[2];
    let raw_e1 = [seed[0] - dot * k[0], seed[1] - dot * k[1], seed[2] - dot * k[2]];
    let e1_norm = scaled_hypot3(raw_e1[0], raw_e1[1], raw_e1[2]);
    let e1 = [raw_e1[0] / e1_norm, raw_e1[1] / e1_norm, raw_e1[2] / e1_norm];
    let e2 = [
        k[1] * e1[2] - k[2] * e1[1],
        k[2] * e1[0] - k[0] * e1[2],
        k[0] * e1[1] - k[1] * e1[0],
    ];
    Ok((e1, e2))
}

/// Compute geometric propagation time $\Delta t = \frac{n \cdot d}{c_0}$ in a homogeneous medium.
pub fn vacuum_or_homogeneous_propagation_delay_s(
    geometric_path_m: f64,
    refractive_index: f64,
) -> Result<f64, PolarimetricValidationError> {
    if !geometric_path_m.is_finite() || geometric_path_m < 0.0 {
        return Err(PolarimetricValidationError::InvalidGeometricPathM { path_m: geometric_path_m });
    }
    if !refractive_index.is_finite() || refractive_index <= 0.0 {
        return Err(PolarimetricValidationError::InvalidGeometricPathM { path_m: refractive_index });
    }
    Ok((geometric_path_m * refractive_index) / VACUUM_SPEED_OF_LIGHT_M_PER_S)
}

/// Apply a $4\times 4$ Mueller matrix to a Stokes vector: $\mathbf{S}_{\text{out}} = \mathbf{M}\mathbf{S}_{\text{in}}$.
pub fn apply_mueller_matrix(mueller: &[[f64; 4]; 4], stokes: [f64; 4]) -> [f64; 4] {
    let mut out = [0.0f64; 4];
    for r in 0..4 {
        out[r] = (mueller[r][0] * stokes[0] + mueller[r][1] * stokes[1])
            + (mueller[r][2] * stokes[2] + mueller[r][3] * stokes[3]);
    }
    out
}

/// Convert a $2\times 2$ complex Jones matrix $\mathbf{J}$ to its exact $4\times 4$ Mueller matrix
/// $\mathbf{M}(\mathbf{J}) = \mathbf{A}(\mathbf{J} \otimes \mathbf{J}^*)\mathbf{A}^{-1}$ in the C02 Mode 1/4 convention.
pub fn jones_to_mueller_matrix(jones: &[[Complex64; 2]; 2]) -> [[f64; 4]; 4] {
    let taus = pauli_tau_basis();
    let j_dag = [[jones[0][0].conj(), jones[1][0].conj()], [jones[0][1].conj(), jones[1][1].conj()]];
    let mut m = [[0.0f64; 4]; 4];
    for mu in 0..4 {
        for nu in 0..4 {
            let prod = mat2_mul(&taus[mu], &mat2_mul(&mat2_mul(jones, &taus[nu]), &j_dag));
            m[mu][nu] = 0.5 * prod[0][0].add(prod[1][1]).re;
        }
    }
    m
}

/// Construct the $4\times 4$ Hermitian Cloude coherency matrix $\mathbf{C}(\mathbf{M})$ from a $4\times 4$
/// Mueller matrix $\mathbf{M}$ using the 16 Hermitian Pauli-basis projections:
///
/// $$C_{\alpha\beta} = \frac{1}{8} \sum_{\mu,\nu=0}^3 M_{\mu\nu} \operatorname{tr}(\tau_\alpha \tau_\mu \tau_\beta \tau_\nu).$$
///
/// Normalization is chosen such that $\operatorname{tr}(\mathbf{C}(\mathbf{M})) = M_{00}$ and a pure
/// Jones-Mueller matrix $\mathbf{M}(\mathbf{J})$ has eigenvalues $(M_{00}, 0, 0, 0)$.
pub fn compute_cloude_coherency_matrix(mueller: &[[f64; 4]; 4]) -> [[Complex64; 4]; 4] {
    let taus = pauli_tau_basis();
    let mut cloude = [[Complex64::ZERO; 4]; 4];
    for alpha in 0..4 {
        for beta in 0..4 {
            let mut sum = Complex64::ZERO;
            for mu in 0..4 {
                for nu in 0..4 {
                    let prod = mat2_mul(&mat2_mul(&mat2_mul(&taus[alpha], &taus[mu]), &taus[beta]), &taus[nu]);
                    let tr = prod[0][0].add(prod[1][1]);
                    sum = sum.add(tr.scale(mueller[mu][nu]));
                }
            }
            cloude[alpha][beta] = sum.scale(0.125);
        }
    }
    for i in 0..4 {
        cloude[i][i].im = 0.0;
        for j in (i + 1)..4 {
            let avg = cloude[i][j].add(cloude[j][i].conj()).scale(0.5);
            cloude[i][j] = avg;
            cloude[j][i] = avg.conj();
        }
    }
    cloude
}

/// Compute all 4 real eigenvalues (sorted ascending) of a $4\times 4$ complex Hermitian matrix
/// using deterministic two-stage complex-phase clearing + real Jacobi rotations.
pub fn hermitian_4x4_eigenvalues(h_in: &[[Complex64; 4]; 4]) -> [f64; 4] {
    let mut a = *h_in;
    for _sweep in 0..64 {
        let mut max_off = 0.0f64;
        for p in 0..4 {
            for q in (p + 1)..4 {
                max_off = max_off.max(a[p][q].abs());
            }
        }
        if max_off <= 1e-15 {
            break;
        }
        for p in 0..4 {
            for q in (p + 1)..4 {
                let apq = a[p][q];
                let mag = apq.abs();
                if mag <= 1e-16 {
                    continue;
                }
                let phase = Complex64::new(apq.re / mag, apq.im / mag);
                let (app, aqq) = (a[p][p].re, a[q][q].re);
                let diff = aqq - app;
                let t = if diff.abs() <= 1e-30 {
                    1.0
                } else {
                    let tau = diff / (2.0 * mag);
                    let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
                    sign / (tau.abs() + (1.0 + tau * tau).sqrt())
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;
                let (s_phase, s_phase_conj) = (phase.scale(s), phase.conj().scale(s));
                for k in 0..4 {
                    if k != p && k != q {
                        let (akp, akq) = (a[k][p], a[k][q]);
                        let new_akp = akp.scale(c).sub(akq.mul(s_phase_conj));
                        let new_akq = akp.mul(s_phase).add(akq.scale(c));
                        a[k][p] = new_akp;
                        a[p][k] = new_akp.conj();
                        a[k][q] = new_akq;
                        a[q][k] = new_akq.conj();
                    }
                }
                a[p][p] = Complex64::new(app - t * mag, 0.0);
                a[q][q] = Complex64::new(aqq + t * mag, 0.0);
                a[p][q] = Complex64::ZERO;
                a[q][p] = Complex64::ZERO;
            }
        }
    }
    let mut eigs = [a[0][0].re, a[1][1].re, a[2][2].re, a[3][3].re];
    if eigs[0] > eigs[1] { eigs.swap(0, 1); }
    if eigs[2] > eigs[3] { eigs.swap(2, 3); }
    if eigs[0] > eigs[2] { eigs.swap(0, 2); }
    if eigs[1] > eigs[3] { eigs.swap(1, 3); }
    if eigs[1] > eigs[2] { eigs.swap(1, 2); }
    eigs
}

/// Verify physical realizability and passivity of a $4\times 4$ Mueller matrix (C02 Mode 4).
pub fn verify_mueller_realizability(
    mueller: &[[f64; 4]; 4],
    require_passive: bool,
) -> Result<VerifiedMuellerReport, PolarimetricValidationError> {
    let mut frob_sq = 0.0f64;
    for r in 0..4 {
        for c in 0..4 {
            let v = mueller[r][c];
            if !v.is_finite() {
                return Err(PolarimetricValidationError::NonFiniteMuellerEntry { row: r, col: c, value: v });
            }
            frob_sq += v * v;
        }
    }
    let frobenius_norm = frob_sq.sqrt();
    let m00 = mueller[0][0];
    if m00 < 0.0 {
        return Err(PolarimetricValidationError::NegativeMuellerM00 { m00 });
    }
    let diattenuation_norm = scaled_hypot3(mueller[0][1], mueller[0][2], mueller[0][3]);
    let polarizance_norm = scaled_hypot3(mueller[1][0], mueller[2][0], mueller[3][0]);
    let max_transmittance_gain = m00 + diattenuation_norm;
    let max_polarizance_gain = m00 + polarizance_norm;

    if require_passive {
        let passive_limit = 1.0 + C02_ROUNDING_FACTOR;
        if max_transmittance_gain > passive_limit || max_polarizance_gain > passive_limit {
            return Err(PolarimetricValidationError::ActiveGainViolation {
                max_forward_gain: max_transmittance_gain,
                max_column_gain: max_polarizance_gain,
                tolerance: passive_limit,
            });
        }
    }

    let cloude = compute_cloude_coherency_matrix(mueller);
    let cloude_trace = cloude[0][0].re + cloude[1][1].re + cloude[2][2].re + cloude[3][3].re;
    let cloude_eigenvalues = hermitian_4x4_eigenvalues(&cloude);
    let eig_tol = C02_ROUNDING_FACTOR * frobenius_norm.max(1.0);
    if cloude_eigenvalues[0] < -eig_tol {
        return Err(PolarimetricValidationError::CloudeNegativeEigenvalue {
            min_eigenvalue: cloude_eigenvalues[0],
            eigenvalues: cloude_eigenvalues,
            tolerance: eig_tol,
        });
    }

    let depolarization_index = if m00 == 0.0 {
        0.0
    } else {
        ((frob_sq - m00 * m00).max(0.0).sqrt()) / (3.0f64.sqrt() * m00)
    };

    Ok(VerifiedMuellerReport {
        frobenius_norm,
        cloude_trace,
        cloude_eigenvalues,
        max_transmittance_gain,
        max_polarizance_gain,
        depolarization_index,
    })
}

/// Verify that a candidate $4\times 4$ Mueller matrix agrees with the exact Jones-to-Mueller
/// transformation $\mathbf{M}(\mathbf{J}) = \mathbf{A}(\mathbf{J} \otimes \mathbf{J}^*)\mathbf{A}^{-1}$
/// within the C02 closed-form budget $64 \cdot 2^{-52} Q$.
pub fn verify_jones_to_mueller_equivalence(
    candidate_mueller: &[[f64; 4]; 4],
    jones: &[[Complex64; 2]; 2],
    scale_q: f64,
) -> Result<[[f64; 4]; 4], PolarimetricValidationError> {
    let reference = jones_to_mueller_matrix(jones);
    let budget = C02_ROUNDING_FACTOR * scale_q.abs().max(F64_EPS);
    for r in 0..4 {
        for c in 0..4 {
            let (actual, expected) = (candidate_mueller[r][c], reference[r][c]);
            let err = (actual - expected).abs();
            if !actual.is_finite() || err > budget {
                return Err(PolarimetricValidationError::JonesMuellerMismatch {
                    row: r, col: c, actual, reference: expected, abs_error: err, budget,
                });
            }
        }
    }
    Ok(reference)
}

#[inline]
fn pauli_tau_basis() -> [[[Complex64; 2]; 2]; 4] {
    [
        [[Complex64::ONE, Complex64::ZERO], [Complex64::ZERO, Complex64::ONE]],
        [[Complex64::ONE, Complex64::ZERO], [Complex64::ZERO, Complex64::new(-1.0, 0.0)]],
        [[Complex64::ZERO, Complex64::ONE], [Complex64::ONE, Complex64::ZERO]],
        [[Complex64::ZERO, Complex64::I], [Complex64::new(0.0, -1.0), Complex64::ZERO]],
    ]
}

#[inline]
fn mat2_mul(a: &[[Complex64; 2]; 2], b: &[[Complex64; 2]; 2]) -> [[Complex64; 2]; 2] {
    [
        [
            a[0][0].mul(b[0][0]).add(a[0][1].mul(b[1][0])),
            a[0][0].mul(b[0][1]).add(a[0][1].mul(b[1][1])),
        ],
        [
            a[1][0].mul(b[0][0]).add(a[1][1].mul(b[1][0])),
            a[1][0].mul(b[0][1]).add(a[1][1].mul(b[1][1])),
        ],
    ]
}
