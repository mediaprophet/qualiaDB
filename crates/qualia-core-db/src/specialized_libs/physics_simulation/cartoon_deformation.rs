//! Parameterized 4-Regime Cartoon Kinetics & Nonlinear Deformation.
//!
//! Implements:
//! 1. Singular-value wafer floor & parameterized deformation Jacobian:
//!    $\tilde{\lambda} = \max(\lambda, \varepsilon_{\text{wafer}}), \quad \det(\mathbf{F}) = \tilde{\lambda}^{1 - 2\nu_{\text{cartoon}}}$
//! 2. Anharmonic cubic potential well (anticipation squash & recoil snap):
//!    $\ddot{\mathbf{q}} + \gamma \dot{\mathbf{q}} + k \mathbf{q} + a \mathbf{q}^2 + b \mathbf{q}^3 = \frac{\mathbf{F}_{\text{ext}}}{m}$
//! 3. Comedic aerodynamic hang-time impulse at trajectory apexes:
//!    $\mathbf{F}_{\text{aero}} = -\frac{1}{2}\rho C_d A \|\mathbf{v}\|\mathbf{v} + C_{\text{cartoon}}\exp\left(-\frac{\|\mathbf{v}\|^2}{2 v_0^2}\right)\hat{\mathbf{z}}$

/// Configuration parameters for non-photorealistic cartoon kinetics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartoonKineticsConfig {
    /// Minimum wafer thickness floor $\varepsilon_{\text{wafer}}$ (e.g. 0.005 m = 5mm) to prevent numerical inversion.
    pub wafer_floor_epsilon: f64,
    /// Cartoon Poisson exponent $\nu_{\text{cartoon}}$ ($\nu = 0.5$ preserves volume; $\nu < 0.5$ exaggerates compression).
    pub cartoon_poisson_nu: f64,
    /// Linear harmonic stiffness $k$ in N/m.
    pub linear_stiffness_k: f64,
    /// Quadratic asymmetry coefficient $a$ in N/m² (anticipation squash).
    pub quadratic_asymmetry_a: f64,
    /// Cubic hardening coefficient $b$ in N/m³ (snap limit).
    pub cubic_hardening_b: f64,
    /// Velocity damping ratio $\gamma$ in 1/s.
    pub damping_gamma: f64,
    /// Comedic apex upward force magnitude $C_{\text{cartoon}}$ in Newtons.
    pub comedic_apex_hang_force_n: f64,
    /// Velocity threshold $v_0$ in m/s under which apex hang-time triggers.
    pub apex_velocity_threshold_v0: f64,
}

impl Default for CartoonKineticsConfig {
    fn default() -> Self {
        Self {
            wafer_floor_epsilon: 0.005, // 5 mm minimum thickness
            cartoon_poisson_nu: 0.35,   // Exaggerated pancake squash
            linear_stiffness_k: 250.0,
            quadratic_asymmetry_a: 50.0,
            cubic_hardening_b: 15.0,
            damping_gamma: 4.5,
            comedic_apex_hang_force_n: 9.81 * 1.5, // 1.5g counter-gravity lift
            apex_velocity_threshold_v0: 0.25,      // < 0.25 m/s at apex
        }
    }
}

/// Evaluates the cartoon deformation gradient singular value and determinant.
///
/// $\tilde{\lambda} = \max(\lambda, \varepsilon_{\text{wafer}})$
/// $\det(\mathbf{F}) = \tilde{\lambda}^{1 - 2\nu_{\text{cartoon}}}$
#[inline]
pub fn compute_cartoon_jacobian_det(
    compression_lambda: f64,
    config: &CartoonKineticsConfig,
) -> (f64, f64) {
    let lambda_tilde = compression_lambda.max(config.wafer_floor_epsilon);
    let exponent = 1.0 - 2.0 * config.cartoon_poisson_nu;
    let det_f = lambda_tilde.powf(exponent);
    (lambda_tilde, det_f)
}

/// Computes the volume deficit $\Delta V = V_0(1 - \det(\mathbf{F}))$ in m³.
#[inline]
pub fn compute_wafer_volume_deficit_m3(
    initial_volume_m3: f64,
    compression_lambda: f64,
    config: &CartoonKineticsConfig,
) -> f64 {
    let (_, det_f) = compute_cartoon_jacobian_det(compression_lambda, config);
    (initial_volume_m3 * (1.0 - det_f)).max(0.0)
}

/// Evaluates restorative restoring force in an anharmonic potential well:
///
/// $F_{\text{restore}} = -(k q + a q^2 + b q^3)$
#[inline]
pub fn anharmonic_restoring_force(q: f64, config: &CartoonKineticsConfig) -> f64 {
    -(config.linear_stiffness_k * q
        + config.quadratic_asymmetry_a * q * q.abs()
        + config.cubic_hardening_b * q * q * q)
}

/// Symplectic step for anharmonic anticipation/snap oscillator.
///
/// Returns updated `(q, v)` displacement and velocity.
#[inline]
pub fn step_anharmonic_oscillator(
    q: f64,
    v: f64,
    mass_kg: f64,
    f_ext: f64,
    dt_seconds: f64,
    config: &CartoonKineticsConfig,
) -> (f64, f64) {
    if mass_kg <= 0.0 || dt_seconds <= 0.0 {
        return (q, v);
    }

    let f_restore = anharmonic_restoring_force(q, config);
    let f_damping = -config.damping_gamma * v * mass_kg;
    let total_f = f_restore + f_damping + f_ext;
    let accel = total_f / mass_kg;

    // Semi-implicit Euler
    let new_v = v + accel * dt_seconds;
    let new_q = q + new_v * dt_seconds;

    (new_q, new_v)
}

/// Evaluates aerodynamic drag + comedic apex hang-time impulse in 3D:
///
/// $\mathbf{F}_{\text{aero}} = -\frac{1}{2}\rho C_d A \|\mathbf{v}\|\mathbf{v} + C_{\text{cartoon}}\exp\left(-\frac{\|\mathbf{v}\|^2}{2 v_0^2}\right)\hat{\mathbf{z}}$
pub fn compute_comedic_aerodynamics(
    velocity: [f64; 3],
    fluid_density_rho: f64,
    drag_coeff_cd: f64,
    area_m2: f64,
    config: &CartoonKineticsConfig,
) -> [f64; 3] {
    let speed_sq = velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2];
    let speed = speed_sq.sqrt();

    // Standard quadratic aerodynamic drag
    let drag_magnitude = 0.5 * fluid_density_rho * drag_coeff_cd * area_m2 * speed;
    let mut f_total = [
        -drag_magnitude * velocity[0],
        -drag_magnitude * velocity[1],
        -drag_magnitude * velocity[2],
    ];

    // Comedic apex hang-time impulse (counteracting gravity along +Z)
    let v0_sq = config.apex_velocity_threshold_v0 * config.apex_velocity_threshold_v0;
    if v0_sq > 0.0 {
        let gaussian_apex = (-speed_sq / (2.0 * v0_sq)).exp();
        f_total[2] += config.comedic_apex_hang_force_n * gaussian_apex;
    }

    f_total
}
