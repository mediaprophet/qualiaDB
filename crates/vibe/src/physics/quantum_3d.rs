//! 3D Hydrogenic Quantum Orbitals and Real Spherical Harmonics.
//!
//! Provides zero-heap analytical evaluators for Hydrogenic wavefunctions $\psi_{nlm}(r, \theta, \phi)$,
//! generalized Laguerre polynomials $L_{n-l-1}^{2l+1}(\rho)$, associated Legendre polynomials
//! $P_l^m(\cos\theta)$, and real spherical harmonics $Y_l^m(\theta, \phi)$.
//!
//! Reference: `AGENTS.md` Workstream W5 (§5.5).

use std::f64::consts::PI;

/// Evaluates generalized Laguerre polynomial $L_p^k(x)$ using an iterative three-term recurrence.
///
/// $$(i + 1) L_{i+1}^k(x) = (2i + 1 + k - x) L_i^k(x) - (i + k) L_{i-1}^k(x)$$
///
/// Completely zero-heap and non-recursive.
pub fn associated_laguerre(p: u32, k: u32, x: f64) -> f64 {
    if p == 0 {
        return 1.0;
    }
    let k_f = k as f64;
    let mut l_prev = 1.0; // L_0^k
    let mut l_curr = 1.0 + k_f - x; // L_1^k
    if p == 1 {
        return l_curr;
    }

    for i in 1..p {
        let i_f = i as f64;
        let l_next = ((2.0 * i_f + 1.0 + k_f - x) * l_curr - (i_f + k_f) * l_prev) / (i_f + 1.0);
        l_prev = l_curr;
        l_curr = l_next;
    }

    l_curr
}

/// Evaluates associated Legendre polynomial $P_l^m(\cos\theta)$ for $0 \le m \le l$.
///
/// Includes the standard Condon-Shortley phase $(-1)^m$. Non-recursive and zero-heap.
pub fn associated_legendre(l: u32, m_abs: u32, cos_theta: f64) -> f64 {
    if m_abs > l {
        return 0.0;
    }

    let x = cos_theta.clamp(-1.0, 1.0);
    let sin_theta = (1.0 - x * x).max(0.0).sqrt();

    // Compute P_m^m(x) = (-1)^m * (2m - 1)!! * (1 - x^2)^(m / 2)
    let mut pmm = 1.0;
    let mut fact = 1.0;
    for _ in 1..=m_abs {
        pmm *= -fact * sin_theta;
        fact += 2.0;
    }

    if l == m_abs {
        return pmm;
    }

    // Compute P_{m+1}^m(x) = x * (2m + 1) * P_m^m(x)
    let mut pmmp1 = x * (2.0 * (m_abs as f64) + 1.0) * pmm;
    if l == m_abs + 1 {
        return pmmp1;
    }

    // Upward recurrence for l >= m + 2
    for ll in (m_abs + 2)..=l {
        let ll_f = ll as f64;
        let m_f = m_abs as f64;
        let pll = (x * (2.0 * ll_f - 1.0) * pmmp1 - (ll_f + m_f - 1.0) * pmm) / (ll_f - m_f);
        pmm = pmmp1;
        pmmp1 = pll;
    }

    pmmp1
}

/// Evaluates real spherical harmonic $Y_l^m(\theta, \phi)$ for $l \ge 0, -l \le m \le l$.
///
/// Normalized such that $\int_{0}^{2\pi}\int_{0}^{\pi} |Y_l^m|^2 \sin\theta \, d\theta d\phi = 1.0$.
pub fn real_spherical_harmonic(l: u32, m: i32, theta: f64, phi: f64) -> f64 {
    let m_abs = m.unsigned_abs();
    if m_abs > l {
        return 0.0;
    }

    let cos_theta = theta.cos();
    let plm = associated_legendre(l, m_abs, cos_theta);

    // Compute ratio (l - m)! / (l + m)! without factorial overflow
    let mut ratio = 1.0;
    for k in (l - m_abs + 1)..=(l + m_abs) {
        ratio /= k as f64;
    }

    let norm = (((2.0 * (l as f64) + 1.0) / (4.0 * PI)) * ratio).sqrt();

    if m == 0 {
        norm * plm
    } else {
        let sqrt2 = std::f64::consts::SQRT_2;
        // (-1)^m_abs cancels Condon-Shortley phase for positive lobes along coordinate axes
        let phase = if m_abs % 2 == 1 { -1.0 } else { 1.0 };
        let base = sqrt2 * norm * phase * plm;

        if m > 0 {
            base * (m as f64 * phi).cos()
        } else {
            base * (m_abs as f64 * phi).sin()
        }
    }
}

/// Evaluates radial hydrogenic wavefunction $R_{nl}(r)$ in atomic units ($a_0 = 1$).
///
/// $$R_{nl}(r) = \sqrt{ \left(\frac{2Z}{n}\right)^3 \frac{(n - l - 1)!}{2n (n + l)!} } \cdot e^{-\rho / 2} \cdot \rho^l \cdot L_{n-l-1}^{2l+1}(\rho)$$
/// where $\rho = \frac{2 Z r}{n}$.
pub fn hydrogenic_radial_wavefunction(n: u32, l: u32, z_atomic: f64, r_bohr: f64) -> f64 {
    if n == 0 || l >= n || z_atomic <= 0.0 || r_bohr < 0.0 {
        return 0.0;
    }

    let n_f = n as f64;
    let rho = (2.0 * z_atomic * r_bohr) / n_f;

    let p = n - l - 1;
    let k = 2 * l + 1;
    let lag = associated_laguerre(p, k, rho);

    // Evaluate (n - l - 1)! / (n + l)!
    let mut denom = 2.0 * n_f;
    for k_val in (n - l)..=(n + l) {
        denom *= k_val as f64;
    }

    let prefactor = (((2.0 * z_atomic / n_f).powi(3)) / denom).sqrt();
    let rho_l = if l == 0 { 1.0 } else { rho.powi(l as i32) };
    let exp_term = (-0.5 * rho).exp();

    prefactor * exp_term * rho_l * lag
}

/// Evaluates exact 3D Hydrogenic quantum orbital amplitude, probability density, and phase sign.
///
/// Returns `(psi, probability_density_psi_sq, phase_sign)`.
/// - `psi`: Amplitude $\psi_{nlm}(r, \theta, \phi) = R_{nl}(r) \cdot Y_l^m(\theta, \phi)$.
/// - `probability_density_psi_sq`: Probability density $|\psi|^2$.
/// - `phase_sign`: `+1` (positive lobe), `-1` (negative lobe), or `0` (nodal surface).
pub fn evaluate_hydrogenic_orbital_3d(
    n: u32,
    l: u32,
    m: i32,
    z_atomic: f64,
    xyz_bohr: [f64; 3],
) -> (f64, f64, i8) {
    let x = xyz_bohr[0];
    let y = xyz_bohr[1];
    let z = xyz_bohr[2];

    let r = (x * x + y * y + z * z).sqrt();
    let (theta, phi) = if r < 1e-15 {
        (0.0, 0.0)
    } else {
        let cos_theta = (z / r).clamp(-1.0, 1.0);
        (cos_theta.acos(), y.atan2(x))
    };

    let r_nl = hydrogenic_radial_wavefunction(n, l, z_atomic, r);
    let y_lm = real_spherical_harmonic(l, m, theta, phi);
    let psi = r_nl * y_lm;
    let prob_sq = psi * psi;

    let phase_sign = if psi > 1e-12 {
        1i8
    } else if psi < -1e-12 {
        -1i8
    } else {
        0i8
    };

    (psi, prob_sq, phase_sign)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laguerre_and_legendre_polynomials() {
        // L_0^k(x) = 1
        assert_eq!(associated_laguerre(0, 1, 5.0), 1.0);
        // L_1^1(x) = 1 + 1 - x = 2 - x
        assert_eq!(associated_laguerre(1, 1, 2.0), 0.0);
        assert_eq!(associated_laguerre(1, 1, 0.0), 2.0);

        // P_0^0(x) = 1
        assert_eq!(associated_legendre(0, 0, 0.5), 1.0);
        // P_1^0(x) = x
        assert!((associated_legendre(1, 0, 0.5) - 0.5).abs() < 1e-9);
        // P_2^0(x) = 0.5 * (3x^2 - 1) -> at x = 1, P_2^0 = 1
        assert!((associated_legendre(2, 0, 1.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_hydrogen_1s_orbital() {
        // 1s state: n=1, l=0, m=0, Z=1
        let (psi_0, prob_0, sign_0) = evaluate_hydrogenic_orbital_3d(1, 0, 0, 1.0, [0.0, 0.0, 0.0]);
        // Theoretical psi(0) = 1 / sqrt(pi) ~ 0.56418958
        let expected_psi0 = 1.0 / (PI.sqrt());
        assert!((psi_0 - expected_psi0).abs() < 1e-6);
        assert!((prob_0 - expected_psi0 * expected_psi0).abs() < 1e-6);
        assert_eq!(sign_0, 1);

        // 1s has no angular nodes: always positive everywhere
        let (psi_r, _, sign_r) = evaluate_hydrogenic_orbital_3d(1, 0, 0, 1.0, [1.5, 0.0, 0.0]);
        assert!(psi_r > 0.0);
        assert_eq!(sign_r, 1);

        // Radial normalization: integral_0^inf R_{10}(r)^2 r^2 dr == 1.0
        let mut sum_norm = 0.0;
        let dr = 0.005;
        let mut r = 0.5 * dr;
        while r < 20.0 {
            let rnl = hydrogenic_radial_wavefunction(1, 0, 1.0, r);
            sum_norm += rnl * rnl * r * r * dr;
            r += dr;
        }
        assert!((sum_norm - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_hydrogen_2pz_nodal_plane() {
        // 2p_z state: n=2, l=1, m=0, Z=1
        // Nodal plane at z = 0
        let (psi_node, prob_node, sign_node) =
            evaluate_hydrogenic_orbital_3d(2, 1, 0, 1.0, [1.5, 2.0, 0.0]);
        assert!(psi_node.abs() < 1e-12);
        assert!(prob_node < 1e-20);
        assert_eq!(sign_node, 0);

        // Upper lobe: z > 0 => positive phase
        let (psi_pos, _, sign_pos) =
            evaluate_hydrogenic_orbital_3d(2, 1, 0, 1.0, [0.0, 0.0, 2.0]);
        assert!(psi_pos > 0.0);
        assert_eq!(sign_pos, 1);

        // Lower lobe: z < 0 => negative phase
        let (psi_neg, _, sign_neg) =
            evaluate_hydrogenic_orbital_3d(2, 1, 0, 1.0, [0.0, 0.0, -2.0]);
        assert!(psi_neg < 0.0);
        assert_eq!(sign_neg, -1);

        // Anti-symmetry along z-axis
        assert!((psi_pos + psi_neg).abs() < 1e-12);
    }

    #[test]
    fn test_hydrogen_3dz2_nodal_cones() {
        // 3d_z2 state: n=3, l=2, m=0, Z=1
        // Nodal surfaces where 3*cos^2(theta) - 1 = 0 => cos(theta) = 1 / sqrt(3)
        let r = 4.0;
        let cos_node = 1.0 / (3.0f64.sqrt());
        let sin_node = (2.0f64 / 3.0).sqrt();
        let z_node = r * cos_node;
        let x_node = r * sin_node;

        let (psi_cone, prob_cone, sign_cone) =
            evaluate_hydrogenic_orbital_3d(3, 2, 0, 1.0, [x_node, 0.0, z_node]);
        assert!(psi_cone.abs() < 1e-10, "psi on nodal cone: {}", psi_cone);
        assert!(prob_cone < 1e-18);
        assert_eq!(sign_cone, 0);

        // Along z-axis: cos^2(theta) = 1 => 3(1) - 1 = 2 > 0 (positive lobe)
        let (psi_z, _, sign_z) = evaluate_hydrogenic_orbital_3d(3, 2, 0, 1.0, [0.0, 0.0, r]);
        assert!(psi_z > 0.0);
        assert_eq!(sign_z, 1);

        // In xy-plane: cos^2(theta) = 0 => 3(0) - 1 = -1 < 0 (negative equatorial donut)
        let (psi_xy, _, sign_xy) = evaluate_hydrogenic_orbital_3d(3, 2, 0, 1.0, [r, 0.0, 0.0]);
        assert!(psi_xy < 0.0);
        assert_eq!(sign_xy, -1);
    }
}
