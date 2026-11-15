//! Constitutive electromagnetic models, multi-pole dispersion (Debye + Lorentz + Drude/Ohmic),
//! and C02-C04 active Hamilton quaternion frame & $3\times 3$ tensor transformations.
//!
//! Governs Schema 133 (`ConstitutiveModel`) and couples `.m64` (`Q42ConstituentLutEntry` /
//! `M64StrataColumnRecord`), EMF v4 (`Q42EMF64`), and QAC v1 (`qac\0\0\0\0\0`).

use crate::q42::m64_matter::{M64StrataColumnRecord, Q42ConstituentLutEntry};

/// Vacuum electric permittivity $\varepsilon_0$ in $\text{F}\cdot\text{m}^{-1}$ (SI 2019 value).
pub const EPSILON_0_F_PER_M: f64 = 8.854_187_812_8e-12;

/// Vacuum magnetic permeability $\mu_0$ in $\text{H}\cdot\text{m}^{-1}$ (SI 2019 value).
pub const MU_0_H_PER_M: f64 = 1.256_637_062_12e-6;

/// Speed of light in vacuum $c_0$ in $\text{m}\cdot\text{s}^{-1}$ (exact SI definition).
pub const C_0_M_PER_S: f64 = 299_792_458.0;

/// Free-space wave impedance $Z_0 = \sqrt{\mu_0 / \varepsilon_0}$ in $\Omega$.
pub const Z_0_OHMS: f64 = 376.730_313_668;

/// Maximum admitted deviation of raw quaternion squared norm from $1.0$ per C02-C04.
pub const QUATERNION_NORM_SQ_TOLERANCE: f64 = 1e-12;

/// Maximum orthogonality residual for derived $3\times 3$ rotation matrices ($64 \times 2^{-52}$).
pub const ROTATION_ORTHOGONALITY_BUDGET: f64 = 64.0 * f64::EPSILON;

/// Errors arising during constitutive model admission, evaluation, or frame rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstitutiveError {
    NonFiniteParameter,
    InvalidHighFrequencyLimit { value: f64 },
    InvalidConductivity { sigma_s_per_m: f64 },
    InvalidDebyePole { index: usize, delta: f64, tau_s: f64 },
    InvalidLorentzPole {
        index: usize,
        delta: f64,
        omega_0_rad_s: f64,
        gamma_rad_s: f64,
    },
    InvalidFrequency { omega_rad_s: f64 },
    QuaternionNormOutOfBounds { norm_sq: f64, deviation: f64 },
    NonSymmetricTensor { max_asymmetry: f64 },
    NonPositiveDefiniteTensor { min_diagonal_or_minor: f64 },
}

/// Complex scalar $z = \text{re} + i\,\text{im}$ in `f64` precision under the $\exp(-i\omega t)$ harmonic convention.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    #[inline]
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    #[inline]
    pub const fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    #[inline]
    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    #[inline]
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    #[inline]
    pub fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }

    #[inline]
    pub fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }

    #[inline]
    pub fn scale(self, s: f64) -> Self {
        Self {
            re: self.re * s,
            im: self.im * s,
        }
    }

    #[inline]
    pub fn div(self, rhs: Self) -> Option<Self> {
        let d = rhs.norm_sq();
        if !d.is_finite() || d <= 0.0 {
            return None;
        }
        Some(Self {
            re: (self.re * rhs.re + self.im * rhs.im) / d,
            im: (self.im * rhs.re - self.re * rhs.im) / d,
        })
    }

    /// Principal square root with $\operatorname{Re}(\sqrt{z}) \ge 0$.
    #[inline]
    pub fn sqrt(self) -> Self {
        let r = self.abs();
        if r == 0.0 {
            return Self::new(0.0, 0.0);
        }
        let re = (0.5 * (r + self.re).max(0.0)).sqrt();
        let im_mag = (0.5 * (r - self.re).max(0.0)).sqrt();
        let im = if self.im < 0.0 { -im_mag } else { im_mag };
        Self::new(re, im)
    }
}

/// Single Debye dipolar relaxation pole:
/// $\chi_{\text{Debye},p}(\omega) = \frac{\Delta\chi_p}{1 - i\omega\tau_p}$.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebyePole {
    /// Dimensionless static susceptibility increment $\Delta\chi_p \ge 0$.
    pub delta_chi: f64,
    /// Relaxation time constant $\tau_p > 0$ in seconds.
    pub tau_s: f64,
}

impl DebyePole {
    pub fn validate(&self, index: usize) -> Result<(), ConstitutiveError> {
        if !self.delta_chi.is_finite() || !self.tau_s.is_finite() {
            return Err(ConstitutiveError::NonFiniteParameter);
        }
        if self.delta_chi < 0.0 || self.tau_s <= 0.0 {
            return Err(ConstitutiveError::InvalidDebyePole {
                index,
                delta: self.delta_chi,
                tau_s: self.tau_s,
            });
        }
        Ok(())
    }

    /// Evaluates $\frac{\Delta\chi_p}{1 - i\omega\tau_p} = \Delta\chi_p \frac{1 + i\omega\tau_p}{1 + (\omega\tau_p)^2}$.
    #[inline]
    pub fn evaluate(&self, omega_rad_s: f64) -> Complex64 {
        let wt = omega_rad_s * self.tau_s;
        let denom = 1.0 + wt * wt;
        Complex64 {
            re: self.delta_chi / denom,
            im: (self.delta_chi * wt) / denom,
        }
    }

    /// Analytical derivative $\frac{\partial \operatorname{Re}\chi_p(\omega)}{\partial \omega}$.
    #[inline]
    pub fn d_re_d_omega(&self, omega_rad_s: f64) -> f64 {
        let wt = omega_rad_s * self.tau_s;
        let denom = 1.0 + wt * wt;
        (-2.0 * self.delta_chi * self.tau_s * wt) / (denom * denom)
    }
}

/// Single Lorentz bound-oscillator resonance pole:
/// $\chi_{\text{Lorentz},k}(\omega) = \frac{\Delta\chi_k \omega_{0,k}^2}{\omega_{0,k}^2 - \omega^2 - i\gamma_k\omega}$.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LorentzPole {
    /// Dimensionless oscillator strength $\Delta\chi_k \ge 0$.
    pub delta_chi: f64,
    /// Undamped resonance angular frequency $\omega_{0,k} > 0$ in $\text{rad}\cdot\text{s}^{-1}$.
    pub omega_0_rad_s: f64,
    /// Viscous damping rate $\gamma_k > 0$ in $\text{rad}\cdot\text{s}^{-1}$.
    pub gamma_rad_s: f64,
}

impl LorentzPole {
    pub fn validate(&self, index: usize) -> Result<(), ConstitutiveError> {
        if !self.delta_chi.is_finite()
            || !self.omega_0_rad_s.is_finite()
            || !self.gamma_rad_s.is_finite()
        {
            return Err(ConstitutiveError::NonFiniteParameter);
        }
        if self.delta_chi < 0.0 || self.omega_0_rad_s <= 0.0 || self.gamma_rad_s <= 0.0 {
            return Err(ConstitutiveError::InvalidLorentzPole {
                index,
                delta: self.delta_chi,
                omega_0_rad_s: self.omega_0_rad_s,
                gamma_rad_s: self.gamma_rad_s,
            });
        }
        Ok(())
    }

    /// Evaluates $\frac{\Delta\chi_k \omega_{0,k}^2}{\omega_{0,k}^2 - \omega^2 - i\gamma_k\omega}$.
    #[inline]
    pub fn evaluate(&self, omega_rad_s: f64) -> Complex64 {
        let w0_sq = self.omega_0_rad_s * self.omega_0_rad_s;
        let w_sq = omega_rad_s * omega_rad_s;
        let diff = w0_sq - w_sq;
        let damp = self.gamma_rad_s * omega_rad_s;
        let denom = diff * diff + damp * damp;
        let num = self.delta_chi * w0_sq;
        Complex64 {
            re: (num * diff) / denom,
            im: (num * damp) / denom,
        }
    }

    /// Analytical derivative $\frac{\partial \operatorname{Re}\chi_k(\omega)}{\partial \omega}$.
    #[inline]
    pub fn d_re_d_omega(&self, omega_rad_s: f64) -> f64 {
        let w0_sq = self.omega_0_rad_s * self.omega_0_rad_s;
        let w_sq = omega_rad_s * omega_rad_s;
        let diff = w0_sq - w_sq;
        let g = self.gamma_rad_s;
        let damp = g * omega_rad_s;
        let denom = diff * diff + damp * damp;
        let num = self.delta_chi * w0_sq;
        // d/dw [ diff / (diff^2 + g^2 w^2) ]
        let d_num = -2.0 * omega_rad_s * denom - diff * (-4.0 * omega_rad_s * diff + 2.0 * g * g * omega_rad_s);
        num * d_num / (denom * denom)
    }
}

/// Schema 133 (`ConstitutiveModel`) multi-pole isotropic electromagnetic constitutive model.
///
/// Under the $\exp(-i\omega t)$ harmonic convention:
/// $$\varepsilon(\omega) = \varepsilon_0 \left(\varepsilon_\infty + \sum_p \frac{\Delta\varepsilon_p}{1 - i\omega\tau_p} + \sum_k \frac{\Delta\varepsilon_k \omega_{0,k}^2}{\omega_{0,k}^2 - \omega^2 - i\gamma_k\omega} + i\frac{\sigma}{\omega\varepsilon_0}\right)$$
/// $$\mu(\omega) = \mu_0 \left(\mu_\infty + \sum_p \frac{\Delta\mu_p}{1 - i\omega\tau_{m,p}} + \sum_k \frac{\Delta\mu_k \omega_{m,0,k}^2}{\omega_{m,0,k}^2 - \omega^2 - i\gamma_{m,k}\omega}\right)$$
#[derive(Clone, Debug, PartialEq)]
pub struct ConstitutiveModel {
    /// Optical / high-frequency relative permittivity $\varepsilon_\infty \ge 1.0$.
    pub epsilon_infinity_relative: f64,
    /// High-frequency relative magnetic permeability $\mu_\infty \ge 1.0$.
    pub mu_infinity_relative: f64,
    /// Static ohmic electrical conductivity $\sigma \ge 0$ in $\text{S}\cdot\text{m}^{-1}$.
    pub conductivity_s_per_m: f64,
    /// Dielectric Debye relaxation poles.
    pub electric_debye_poles: Vec<DebyePole>,
    /// Dielectric Lorentz resonance poles.
    pub electric_lorentz_poles: Vec<LorentzPole>,
    /// Magnetic Debye relaxation poles.
    pub magnetic_debye_poles: Vec<DebyePole>,
    /// Magnetic Lorentz resonance poles.
    pub magnetic_lorentz_poles: Vec<LorentzPole>,
}

impl ConstitutiveModel {
    /// Construct a vacuum constitutive model ($\varepsilon_r = 1, \mu_r = 1, \sigma = 0$).
    pub fn vacuum() -> Self {
        Self {
            epsilon_infinity_relative: 1.0,
            mu_infinity_relative: 1.0,
            conductivity_s_per_m: 0.0,
            electric_debye_poles: Vec::new(),
            electric_lorentz_poles: Vec::new(),
            magnetic_debye_poles: Vec::new(),
            magnetic_lorentz_poles: Vec::new(),
        }
    }

    /// Construct a constitutive model from a validated `.m64` `Q42ConstituentLutEntry`.
    pub fn from_m64_lut_entry(entry: &Q42ConstituentLutEntry) -> Result<Self, ConstitutiveError> {
        let eps_r = (entry.dielectric_permittivity_x100 as f64) * 0.01;
        let sigma = (entry.electrical_conductivity_ms_m as f64) * 1e-3;
        let model = Self {
            epsilon_infinity_relative: eps_r,
            mu_infinity_relative: 1.0,
            conductivity_s_per_m: sigma,
            electric_debye_poles: Vec::new(),
            electric_lorentz_poles: Vec::new(),
            magnetic_debye_poles: Vec::new(),
            magnetic_lorentz_poles: Vec::new(),
        };
        model.validate()?;
        Ok(model)
    }

    /// Construct a homogenized subsurface dielectric model from an `.m64` `M64StrataColumnRecord`.
    pub fn from_m64_strata_summary(
        strata: &M64StrataColumnRecord,
        conductivity_s_per_m: f64,
    ) -> Result<Self, ConstitutiveError> {
        let eps_r = (strata.gpr_dielectric_perm_x100 as f64) * 0.01;
        let model = Self {
            epsilon_infinity_relative: eps_r,
            mu_infinity_relative: 1.0,
            conductivity_s_per_m,
            electric_debye_poles: Vec::new(),
            electric_lorentz_poles: Vec::new(),
            magnetic_debye_poles: Vec::new(),
            magnetic_lorentz_poles: Vec::new(),
        };
        model.validate()?;
        Ok(model)
    }

    /// Validate all physical bounds and finiteness invariants.
    pub fn validate(&self) -> Result<(), ConstitutiveError> {
        if !self.epsilon_infinity_relative.is_finite()
            || !self.mu_infinity_relative.is_finite()
            || !self.conductivity_s_per_m.is_finite()
        {
            return Err(ConstitutiveError::NonFiniteParameter);
        }
        if self.epsilon_infinity_relative < 1.0 {
            return Err(ConstitutiveError::InvalidHighFrequencyLimit {
                value: self.epsilon_infinity_relative,
            });
        }
        if self.mu_infinity_relative <= 0.0 {
            return Err(ConstitutiveError::InvalidHighFrequencyLimit {
                value: self.mu_infinity_relative,
            });
        }
        if self.conductivity_s_per_m < 0.0 {
            return Err(ConstitutiveError::InvalidConductivity {
                sigma_s_per_m: self.conductivity_s_per_m,
            });
        }
        for (i, p) in self.electric_debye_poles.iter().enumerate() {
            p.validate(i)?;
        }
        for (i, p) in self.electric_lorentz_poles.iter().enumerate() {
            p.validate(i)?;
        }
        for (i, p) in self.magnetic_debye_poles.iter().enumerate() {
            p.validate(i)?;
        }
        for (i, p) in self.magnetic_lorentz_poles.iter().enumerate() {
            p.validate(i)?;
        }
        Ok(())
    }

    /// Evaluate the bound electric susceptibility $\chi_e(\omega) = \varepsilon_r^{\text{bound}}(\omega) - \varepsilon_\infty$
    /// (excluding the $i\sigma / (\omega\varepsilon_0)$ conduction pole at $\omega = 0$).
    pub fn bound_electric_susceptibility(&self, omega_rad_s: f64) -> Result<Complex64, ConstitutiveError> {
        if !omega_rad_s.is_finite() {
            return Err(ConstitutiveError::InvalidFrequency { omega_rad_s });
        }
        let mut chi = Complex64::new(0.0, 0.0);
        for p in &self.electric_debye_poles {
            chi = chi.add(p.evaluate(omega_rad_s));
        }
        for p in &self.electric_lorentz_poles {
            chi = chi.add(p.evaluate(omega_rad_s));
        }
        Ok(chi)
    }

    /// Evaluate relative complex permittivity $\varepsilon_r(\omega)$ at angular frequency $\omega > 0$.
    pub fn relative_permittivity(&self, omega_rad_s: f64) -> Result<Complex64, ConstitutiveError> {
        if !omega_rad_s.is_finite() || omega_rad_s <= 0.0 {
            return Err(ConstitutiveError::InvalidFrequency { omega_rad_s });
        }
        let chi = self.bound_electric_susceptibility(omega_rad_s)?;
        let cond_im = self.conductivity_s_per_m / (omega_rad_s * EPSILON_0_F_PER_M);
        Ok(Complex64 {
            re: self.epsilon_infinity_relative + chi.re,
            im: chi.im + cond_im,
        })
    }

    /// Evaluate absolute SI complex permittivity $\varepsilon(\omega) = \varepsilon_0 \varepsilon_r(\omega)$ in $\text{F}\cdot\text{m}^{-1}$.
    pub fn permittivity_f_per_m(&self, omega_rad_s: f64) -> Result<Complex64, ConstitutiveError> {
        Ok(self.relative_permittivity(omega_rad_s)?.scale(EPSILON_0_F_PER_M))
    }

    /// Evaluate bound magnetic susceptibility $\chi_m(\omega) = \mu_r(\omega) - \mu_\infty$.
    pub fn bound_magnetic_susceptibility(&self, omega_rad_s: f64) -> Result<Complex64, ConstitutiveError> {
        if !omega_rad_s.is_finite() {
            return Err(ConstitutiveError::InvalidFrequency { omega_rad_s });
        }
        let mut chi = Complex64::new(0.0, 0.0);
        for p in &self.magnetic_debye_poles {
            chi = chi.add(p.evaluate(omega_rad_s));
        }
        for p in &self.magnetic_lorentz_poles {
            chi = chi.add(p.evaluate(omega_rad_s));
        }
        Ok(chi)
    }

    /// Evaluate relative complex permeability $\mu_r(\omega)$ at angular frequency $\omega > 0$.
    pub fn relative_permeability(&self, omega_rad_s: f64) -> Result<Complex64, ConstitutiveError> {
        if !omega_rad_s.is_finite() || omega_rad_s <= 0.0 {
            return Err(ConstitutiveError::InvalidFrequency { omega_rad_s });
        }
        let chi = self.bound_magnetic_susceptibility(omega_rad_s)?;
        Ok(Complex64 {
            re: self.mu_infinity_relative + chi.re,
            im: chi.im,
        })
    }

    /// Evaluate absolute SI complex permeability $\mu(\omega) = \mu_0 \mu_r(\omega)$ in $\text{H}\cdot\text{m}^{-1}$.
    pub fn permeability_h_per_m(&self, omega_rad_s: f64) -> Result<Complex64, ConstitutiveError> {
        Ok(self.relative_permeability(omega_rad_s)?.scale(MU_0_H_PER_M))
    }

    /// Analytical derivative $\frac{\partial (\omega \operatorname{Re}\varepsilon(\omega))}{\partial \omega}$ in $\text{F}\cdot\text{m}^{-1}$
    /// required for Brillouin dispersive electromagnetic energy density.
    pub fn d_omega_re_epsilon_d_omega(&self, omega_rad_s: f64) -> Result<f64, ConstitutiveError> {
        if !omega_rad_s.is_finite() || omega_rad_s <= 0.0 {
            return Err(ConstitutiveError::InvalidFrequency { omega_rad_s });
        }
        let eps_r = self.relative_permittivity(omega_rad_s)?.re;
        let mut d_re_eps_r = 0.0;
        for p in &self.electric_debye_poles {
            d_re_eps_r += p.d_re_d_omega(omega_rad_s);
        }
        for p in &self.electric_lorentz_poles {
            d_re_eps_r += p.d_re_d_omega(omega_rad_s);
        }
        Ok(EPSILON_0_F_PER_M * (eps_r + omega_rad_s * d_re_eps_r))
    }

    /// Analytical derivative $\frac{\partial (\omega \operatorname{Re}\mu(\omega))}{\partial \omega}$ in $\text{H}\cdot\text{m}^{-1}$.
    pub fn d_omega_re_mu_d_omega(&self, omega_rad_s: f64) -> Result<f64, ConstitutiveError> {
        if !omega_rad_s.is_finite() || omega_rad_s <= 0.0 {
            return Err(ConstitutiveError::InvalidFrequency { omega_rad_s });
        }
        let mu_r = self.relative_permeability(omega_rad_s)?.re;
        let mut d_re_mu_r = 0.0;
        for p in &self.magnetic_debye_poles {
            d_re_mu_r += p.d_re_d_omega(omega_rad_s);
        }
        for p in &self.magnetic_lorentz_poles {
            d_re_mu_r += p.d_re_d_omega(omega_rad_s);
        }
        Ok(MU_0_H_PER_M * (mu_r + omega_rad_s * d_re_mu_r))
    }
}

/// Active local-to-parent Hamilton quaternion rotation (`[x, y, z, w]`) per C02-C04.
///
/// Retains the exact authored `raw_xyzw` bytes for provenance while computing
/// the derived normalized quaternion `derived_xyzw` and orthogonal rotation matrix $R(q)$
/// using the exact non-fused ordered sum `(((x*x + y*y) + z*z) + w*w)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveHamiltonRotation {
    pub raw_xyzw: [f64; 4],
    pub raw_norm_sq: f64,
    pub derived_xyzw: [f64; 4],
    pub matrix: [[f64; 3]; 3],
}

impl ActiveHamiltonRotation {
    /// Admit an authored `[x, y, z, w]` quaternion under C02-C04 (`|norm_sq - 1| <= 1e-12`).
    /// Never mutates `raw_xyzw`; computes derived `q_i / sqrt(n2)` without fused multiply-add.
    pub fn admit(raw_xyzw: [f64; 4]) -> Result<Self, ConstitutiveError> {
        let [x, y, z, w] = raw_xyzw;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() || !w.is_finite() {
            return Err(ConstitutiveError::NonFiniteParameter);
        }
        // C02-C04 ordered sum: (((x*x + y*y) + z*z) + w*w)
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let ww = w * w;
        let raw_norm_sq = ((xx + yy) + zz) + ww;
        let deviation = (raw_norm_sq - 1.0).abs();
        if deviation > QUATERNION_NORM_SQ_TOLERANCE {
            return Err(ConstitutiveError::QuaternionNormOutOfBounds {
                norm_sq: raw_norm_sq,
                deviation,
            });
        }
        let n = raw_norm_sq.sqrt();
        let dx = x / n;
        let dy = y / n;
        let dz = z / n;
        let dw = w / n;
        let derived_xyzw = [dx, dy, dz, dw];

        let tx = 2.0 * dx;
        let ty = 2.0 * dy;
        let tz = 2.0 * dz;
        let twx = tx * dw;
        let twy = ty * dw;
        let twz = tz * dw;
        let txx = tx * dx;
        let txy = ty * dx;
        let txz = tz * dx;
        let tyy = ty * dy;
        let tyz = tz * dy;
        let tzz = tz * dz;

        let matrix = [
            [1.0 - (tyy + tzz), txy - twz, txz + twy],
            [txy + twz, 1.0 - (txx + tzz), tyz - twx],
            [txz - twy, tyz + twx, 1.0 - (txx + tyy)],
        ];

        Ok(Self {
            raw_xyzw,
            raw_norm_sq,
            derived_xyzw,
            matrix,
        })
    }

    /// Compose `self` (parent) with `local` (`q_parent * q_local` = apply `local` then `parent`).
    pub fn compose(&self, local: &Self) -> Result<Self, ConstitutiveError> {
        let [x1, y1, z1, w1] = self.derived_xyzw;
        let [x2, y2, z2, w2] = local.derived_xyzw;
        let prod = [
            w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
            w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
            w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
            w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
        ];
        Self::admit(prod)
    }

    /// Active vector transformation $v_{\text{parent}} = R(q)\,v_{\text{local}}$.
    #[inline]
    pub fn rotate_vector(&self, v_local: [f64; 3]) -> [f64; 3] {
        let r = &self.matrix;
        [
            r[0][0] * v_local[0] + r[0][1] * v_local[1] + r[0][2] * v_local[2],
            r[1][0] * v_local[0] + r[1][1] * v_local[1] + r[1][2] * v_local[2],
            r[2][0] * v_local[0] + r[2][1] * v_local[1] + r[2][2] * v_local[2],
        ]
    }

    /// Active rank-2 real tensor transformation $T_{\text{parent}} = R(q)\,T_{\text{local}}\,R(q)^T$.
    pub fn rotate_tensor_real(&self, t_local: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let r = &self.matrix;
        let mut rt = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                rt[i][j] = r[i][0] * t_local[0][j] + r[i][1] * t_local[1][j] + r[i][2] * t_local[2][j];
            }
        }
        let mut out = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = rt[i][0] * r[j][0] + rt[i][1] * r[j][1] + rt[i][2] * r[j][2];
            }
        }
        out
    }

    /// Active rank-2 complex tensor transformation $T_{\text{parent}} = R(q)\,T_{\text{local}}\,R(q)^T$.
    pub fn rotate_tensor_complex(&self, t_local: &[[Complex64; 3]; 3]) -> [[Complex64; 3]; 3] {
        let r = &self.matrix;
        let mut rt = [[Complex64::default(); 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                rt[i][j] = t_local[0][j]
                    .scale(r[i][0])
                    .add(t_local[1][j].scale(r[i][1]))
                    .add(t_local[2][j].scale(r[i][2]));
            }
        }
        let mut out = [[Complex64::default(); 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = rt[i][0]
                    .scale(r[j][0])
                    .add(rt[i][1].scale(r[j][1]))
                    .add(rt[i][2].scale(r[j][2]));
            }
        }
        out
    }

    /// Maximum absolute residual $\|R R^T - I\|_{\max}$.
    pub fn orthogonality_residual(&self) -> f64 {
        let r = &self.matrix;
        let mut max_err = 0.0f64;
        for i in 0..3 {
            for j in 0..3 {
                let dot = r[i][0] * r[j][0] + r[i][1] * r[j][1] + r[i][2] * r[j][2];
                let target = if i == j { 1.0 } else { 0.0 };
                max_err = max_err.max((dot - target).abs());
            }
        }
        max_err
    }

    /// Determinant $\det(R)$ (proper rotation has $+1.0$).
    pub fn determinant(&self) -> f64 {
        let r = &self.matrix;
        r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
            - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
            + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0])
    }
}

/// Full $3\times 3$ complex anisotropic constitutive tensor (reciprocal symmetric $T_{ij} = T_{ji}$).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnisotropicConstitutiveTensor3x3 {
    pub components: [[Complex64; 3]; 3],
}

impl AnisotropicConstitutiveTensor3x3 {
    /// Construct a diagonal uniaxial/biaxial constitutive tensor in its principal frame.
    pub fn from_principal_axes(principal: [Complex64; 3]) -> Result<Self, ConstitutiveError> {
        let z = Complex64::new(0.0, 0.0);
        let t = Self {
            components: [
                [principal[0], z, z],
                [z, principal[1], z],
                [z, z, principal[2]],
            ],
        };
        t.validate_passivity_and_symmetry(1e-12)?;
        Ok(t)
    }

    /// Verify symmetric reciprocity ($T_{ij} = T_{ji}$), positive-definite real part,
    /// and positive-semidefinite imaginary part (passivity under $\exp(-i\omega t)$).
    pub fn validate_passivity_and_symmetry(&self, sym_tol: f64) -> Result<(), ConstitutiveError> {
        for i in 0..3 {
            for j in 0..3 {
                if !self.components[i][j].is_finite() {
                    return Err(ConstitutiveError::NonFiniteParameter);
                }
            }
        }
        let mut max_asym = 0.0f64;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let d_re = (self.components[i][j].re - self.components[j][i].re).abs();
                let d_im = (self.components[i][j].im - self.components[j][i].im).abs();
                max_asym = max_asym.max(d_re).max(d_im);
            }
        }
        if max_asym > sym_tol {
            return Err(ConstitutiveError::NonSymmetricTensor {
                max_asymmetry: max_asym,
            });
        }
        // Sylvester's criterion on Re(T) > 0 and Im(T) >= 0
        let re = self.real_part();
        let m1 = re[0][0];
        let m2 = re[0][0] * re[1][1] - re[0][1] * re[1][0];
        let m3 = re[0][0] * (re[1][1] * re[2][2] - re[1][2] * re[2][1])
            - re[0][1] * (re[1][0] * re[2][2] - re[1][2] * re[2][0])
            + re[0][2] * (re[1][0] * re[2][1] - re[1][1] * re[2][0]);
        let min_minor = m1.min(m2).min(m3);
        if min_minor <= 0.0 {
            return Err(ConstitutiveError::NonPositiveDefiniteTensor {
                min_diagonal_or_minor: min_minor,
            });
        }
        let im = self.imag_part();
        for i in 0..3 {
            if im[i][i] < -sym_tol {
                return Err(ConstitutiveError::NonPositiveDefiniteTensor {
                    min_diagonal_or_minor: im[i][i],
                });
            }
        }
        Ok(())
    }

    #[inline]
    pub fn real_part(&self) -> [[f64; 3]; 3] {
        let mut out = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = self.components[i][j].re;
            }
        }
        out
    }

    #[inline]
    pub fn imag_part(&self) -> [[f64; 3]; 3] {
        let mut out = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = self.components[i][j].im;
            }
        }
        out
    }

    /// Transform tensor from local material crystal axes to parent frame via $R(q)\,T\,R(q)^T$.
    pub fn transform_to_parent(&self, rot: &ActiveHamiltonRotation) -> Self {
        Self {
            components: rot.rotate_tensor_complex(&self.components),
        }
    }

    /// Trace $\operatorname{tr}(T) = T_{00} + T_{11} + T_{22}$ (rotational invariant).
    pub fn trace(&self) -> Complex64 {
        self.components[0][0]
            .add(self.components[1][1])
            .add(self.components[2][2])
    }
}
