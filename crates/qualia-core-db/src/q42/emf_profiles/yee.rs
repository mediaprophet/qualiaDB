//! Packet E03: Instantaneous/phasor EM fields (0x0100), polarimetric rays (0x0101),
//! and staggered Cartesian FDTD Yee lattice records & operators (0x0102 / yee_cartesian_v1).

use super::schemas::{i64_at, ConstitutiveKind};
use super::*;
use core::f64::consts::{FRAC_1_SQRT_2, PI, SQRT_2};

pub const YEE_PROFILE: &str = "yee_cartesian_v1";
pub const YEE_MODEL_URI: &str = "urn:qualia:emf:yee_cartesian_v1";

pub const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;
pub const VACUUM_PERMEABILITY_H_PER_M: f64 = 4.0 * PI * 1.0e-7;
pub const VACUUM_PERMITTIVITY_F_PER_M: f64 =
    1.0 / (VACUUM_PERMEABILITY_H_PER_M * SPEED_OF_LIGHT_M_PER_S * SPEED_OF_LIGHT_M_PER_S);
pub const VACUUM_IMPEDANCE_OHM: f64 = VACUUM_PERMEABILITY_H_PER_M * SPEED_OF_LIGHT_M_PER_S;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhasorAmplitudeKind {
    Peak,
    Rms,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarmonicSign {
    MinusIwt,
    PlusIwt,
}

impl HarmonicSign {
    pub const fn sign_factor(self) -> f64 {
        match self {
            Self::MinusIwt => -1.0,
            Self::PlusIwt => 1.0,
        }
    }
}

#[inline]
pub fn rms_to_peak(rms: f64) -> f64 {
    rms * SQRT_2
}

#[inline]
pub fn peak_to_rms(peak: f64) -> f64 {
    peak * FRAC_1_SQRT_2
}

/// Evaluates `Re(F_tilde * exp(i * s * omega * t))` for a 3-vector complex phasor.
pub fn evaluate_instantaneous_phasor(
    phasor_re: [f64; 3],
    phasor_im: [f64; 3],
    omega_rad_s: f64,
    t_s: f64,
    amplitude: PhasorAmplitudeKind,
    sign: HarmonicSign,
) -> Result<[f64; 3], Error> {
    if !omega_rad_s.is_finite()
        || !t_s.is_finite()
        || phasor_re.iter().any(|v| !v.is_finite())
        || phasor_im.iter().any(|v| !v.is_finite())
    {
        return Err(Error::NonFinite);
    }
    if omega_rad_s < 0.0 {
        return Err(Error::Value);
    }
    let scale = match amplitude {
        PhasorAmplitudeKind::Peak => 1.0,
        PhasorAmplitudeKind::Rms => SQRT_2,
    };
    let theta = sign.sign_factor() * omega_rad_s * t_s;
    let (s_th, c_th) = theta.sin_cos();
    Ok([
        scale * (phasor_re[0] * c_th - phasor_im[0] * s_th),
        scale * (phasor_re[1] * c_th - phasor_im[1] * s_th),
        scale * (phasor_re[2] * c_th - phasor_im[2] * s_th),
    ])
}

#[inline]
pub fn poynting_e_cross_h(e: [f64; 3], h: [f64; 3]) -> [f64; 3] {
    [
        e[1] * h[2] - e[2] * h[1],
        e[2] * h[0] - e[0] * h[2],
        e[0] * h[1] - e[1] * h[0],
    ]
}

#[inline]
pub fn poynting_from_eb_vacuum(e: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let inv_mu0 = 1.0 / VACUUM_PERMEABILITY_H_PER_M;
    poynting_e_cross_h(e, [b[0] * inv_mu0, b[1] * inv_mu0, b[2] * inv_mu0])
}

pub fn energy_density_isotropic(
    e: [f64; 3],
    h: [f64; 3],
    eps_r: f64,
    mu_r: f64,
) -> Result<f64, Error> {
    if e.iter().any(|v| !v.is_finite())
        || h.iter().any(|v| !v.is_finite())
        || !eps_r.is_finite()
        || !mu_r.is_finite()
    {
        return Err(Error::NonFinite);
    }
    if eps_r <= 0.0 || mu_r <= 0.0 {
        return Err(Error::Value);
    }
    let e2 = e[0] * e[0] + e[1] * e[1] + e[2] * e[2];
    let h2 = h[0] * h[0] + h[1] * h[1] + h[2] * h[2];
    Ok(0.5 * (VACUUM_PERMITTIVITY_F_PER_M * eps_r * e2 + VACUUM_PERMEABILITY_H_PER_M * mu_r * h2))
}

#[inline]
pub fn energy_density_vacuum_eb(e: [f64; 3], b: [f64; 3]) -> Result<f64, Error> {
    if e.iter().any(|v| !v.is_finite()) || b.iter().any(|v| !v.is_finite()) {
        return Err(Error::NonFinite);
    }
    let e2 = e[0] * e[0] + e[1] * e[1] + e[2] * e[2];
    let b2 = b[0] * b[0] + b[1] * b[1] + b[2] * b[2];
    Ok(0.5 * (VACUUM_PERMITTIVITY_F_PER_M * e2 + b2 / VACUUM_PERMEABILITY_H_PER_M))
}

/// Complex transverse Jones vector `(Ex, Ey)` in horizontal/vertical basis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JonesVector {
    pub ex_re: f64,
    pub ex_im: f64,
    pub ey_re: f64,
    pub ey_im: f64,
}

impl JonesVector {
    /// Converts to Stokes `[I, Q, U, V]` (`Q^2 + U^2 + V^2 == I^2`) with scale factor.
    pub fn to_stokes(self, scale_irradiance: f64) -> Result<[f64; 4], Error> {
        if !self.ex_re.is_finite()
            || !self.ex_im.is_finite()
            || !self.ey_re.is_finite()
            || !self.ey_im.is_finite()
            || !scale_irradiance.is_finite()
        {
            return Err(Error::NonFinite);
        }
        if scale_irradiance < 0.0 {
            return Err(Error::Value);
        }
        let ax2 = self.ex_re * self.ex_re + self.ex_im * self.ex_im;
        let ay2 = self.ey_re * self.ey_re + self.ey_im * self.ey_im;
        let i = scale_irradiance * (ax2 + ay2);
        let q = scale_irradiance * (ax2 - ay2);
        let u = scale_irradiance * 2.0 * (self.ex_re * self.ey_re + self.ex_im * self.ey_im);
        let v = scale_irradiance * 2.0 * (self.ex_re * self.ey_im - self.ex_im * self.ey_re);
        Ok([i, q, u, v])
    }
}

/// Passive linear basis rotation of Stokes `[I, Q, U, V]` by angle `theta_rad`.
pub fn rotate_stokes_passive(stokes: [f64; 4], theta_rad: f64) -> Result<[f64; 4], Error> {
    if stokes.iter().any(|v| !v.is_finite()) || !theta_rad.is_finite() {
        return Err(Error::NonFinite);
    }
    let (s, c) = (2.0 * theta_rad).sin_cos();
    Ok([
        stokes[0],
        stokes[1] * c + stokes[2] * s,
        -stokes[1] * s + stokes[2] * c,
        stokes[3],
    ])
}

pub fn optical_path_delay_seconds(
    geometric_path_m: f64,
    group_refractive_index: f64,
) -> Result<f64, Error> {
    if !geometric_path_m.is_finite() || !group_refractive_index.is_finite() {
        return Err(Error::NonFinite);
    }
    if geometric_path_m < 0.0 || group_refractive_index <= 0.0 {
        return Err(Error::Value);
    }
    Ok(geometric_path_m * group_refractive_index / SPEED_OF_LIGHT_M_PER_S)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct YeeRecord {
    pub electric_field_v_per_m: [f64; 3],
    pub magnetic_field_h_a_per_m: [f64; 3],
    pub impressed_current_j_a_per_m2: [f64; 3],
    pub scalar_potential_v: f64,
    pub charge_density_c_per_m3: f64,
    pub epsilon_relative: f64,
    pub mu_relative: f64,
    pub conductivity_s_per_m: f64,
    pub material_entry: u32,
    pub constitutive_metadata_id: u32,
    pub state_record_index: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct YeeParsePolicy {
    pub require_no_phi_placeholder: bool,
    pub require_charge_free: bool,
    pub expected_constitutive_id: Option<u32>,
    pub has_auxiliary_state_streams: bool,
}

impl Default for YeeParsePolicy {
    fn default() -> Self {
        Self {
            require_no_phi_placeholder: true,
            require_charge_free: false,
            expected_constitutive_id: None,
            has_auxiliary_state_streams: false,
        }
    }
}

impl YeeRecord {
    pub fn vacuum(
        e: [f64; 3],
        h: [f64; 3],
        constitutive_metadata_id: u32,
    ) -> Self {
        Self {
            electric_field_v_per_m: e,
            magnetic_field_h_a_per_m: h,
            impressed_current_j_a_per_m2: [0.0; 3],
            scalar_potential_v: 0.0,
            charge_density_c_per_m3: 0.0,
            epsilon_relative: 1.0,
            mu_relative: 1.0,
            conductivity_s_per_m: 0.0,
            material_entry: 0,
            constitutive_metadata_id,
            state_record_index: u64::MAX,
        }
    }

    pub fn parse(
        bytes: &[u8],
        valid: bool,
        policy: YeeParsePolicy,
    ) -> Result<Option<Self>, Error> {
        if bytes.len() != 128 {
            return Err(Error::Shape);
        }
        if !valid {
            zeros(bytes)?;
            return Ok(None);
        }
        finite(bytes, 0, 112)?;
        let phi = f64_at(bytes, 72);
        let rho = f64_at(bytes, 80);
        let eps_r = f64_at(bytes, 88);
        let mu_r = f64_at(bytes, 96);
        let sigma = f64_at(bytes, 104);
        if eps_r <= 0.0 || mu_r <= 0.0 || sigma < 0.0 {
            return Err(Error::Value);
        }
        if policy.require_no_phi_placeholder && phi.to_bits() != 0 {
            return Err(Error::Reserved);
        }
        if policy.require_charge_free && rho != 0.0 {
            return Err(Error::Value);
        }
        let material_entry = u32_at(bytes, 112);
        let constitutive_metadata_id = u32_at(bytes, 116);
        let state_record_index = u64_at(bytes, 120);
        if let Some(expected_id) = policy.expected_constitutive_id {
            if constitutive_metadata_id != expected_id {
                return Err(Error::Descriptor);
            }
        }
        if !policy.has_auxiliary_state_streams && state_record_index != u64::MAX {
            return Err(Error::Index);
        }
        Ok(Some(Self {
            electric_field_v_per_m: triple(bytes, 0),
            magnetic_field_h_a_per_m: triple(bytes, 24),
            impressed_current_j_a_per_m2: triple(bytes, 48),
            scalar_potential_v: phi,
            charge_density_c_per_m3: rho,
            epsilon_relative: eps_r,
            mu_relative: mu_r,
            conductivity_s_per_m: sigma,
            material_entry,
            constitutive_metadata_id,
            state_record_index,
        }))
    }

    pub fn encode(&self, valid: bool, out: &mut [u8; 128]) -> Result<(), Error> {
        *out = [0u8; 128];
        if !valid {
            return Ok(());
        }
        let all_f64 = [
            self.electric_field_v_per_m[0],
            self.electric_field_v_per_m[1],
            self.electric_field_v_per_m[2],
            self.magnetic_field_h_a_per_m[0],
            self.magnetic_field_h_a_per_m[1],
            self.magnetic_field_h_a_per_m[2],
            self.impressed_current_j_a_per_m2[0],
            self.impressed_current_j_a_per_m2[1],
            self.impressed_current_j_a_per_m2[2],
            self.scalar_potential_v,
            self.charge_density_c_per_m3,
            self.epsilon_relative,
            self.mu_relative,
            self.conductivity_s_per_m,
        ];
        if all_f64.iter().any(|v| !v.is_finite()) {
            return Err(Error::NonFinite);
        }
        if self.epsilon_relative <= 0.0 || self.mu_relative <= 0.0 || self.conductivity_s_per_m < 0.0
        {
            return Err(Error::Value);
        }
        for (i, v) in all_f64.iter().enumerate() {
            out[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
        }
        out[112..116].copy_from_slice(&self.material_entry.to_le_bytes());
        out[116..120].copy_from_slice(&self.constitutive_metadata_id.to_le_bytes());
        out[120..128].copy_from_slice(&self.state_record_index.to_le_bytes());
        Ok(())
    }
}

pub fn encode_yee_records(
    records: &[YeeRecord],
    valid: Option<&[bool]>,
    out: &mut [u8],
) -> Result<usize, Error> {
    if let Some(v) = valid {
        if v.len() != records.len() {
            return Err(Error::Shape);
        }
    }
    let needed = records.len().checked_mul(128).ok_or(Error::Overflow)?;
    if out.len() < needed {
        return Err(Error::Shape);
    }
    for (idx, rec) in records.iter().enumerate() {
        let is_valid = valid.is_none_or(|v| v[idx]);
        let dst: &mut [u8; 128] = (&mut out[idx * 128..idx * 128 + 128])
            .try_into()
            .map_err(|_| Error::Shape)?;
        rec.encode(is_valid, dst)?;
    }
    Ok(needed)
}

/// Validates Yee extension (`schema 103`, mode `2 | 0x0102`), constitutive model completeness,
/// auxiliary/boundary restart requirements, and staggered time offset arrays.
pub fn validate_yee_extension(ext: Block<'_>, frame: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    if [5, 6].iter().any(|&k| field(frame, k).is_none()) {
        return Err(Error::Frame);
    }
    let _ = spatial::from_metadata(frame, ext)?;
    let f_const = required(ext, 14)?;
    if f_const.flags() & 1 == 0 {
        return Err(Error::Descriptor);
    }
    let const_block = ctx
        .require_schema(u32_at(f_const.value(), 0), 133)
        .map_err(Error::Shared)?;
    let kind = ConstitutiveKind::parse(text(required(const_block, 1)?)?)?;

    // Dispersive or anisotropic restarts fail closed unless auxiliary and boundary state exist.
    let aux_entries = field(ext, 26);
    let bnd_entries = field(ext, 29);
    if let Some(aux) = aux_entries {
        if aux.value().is_empty() || !aux.value().len().is_multiple_of(4) {
            return Err(Error::Descriptor);
        }
        for chunk in aux.value().chunks_exact(4) {
            let _ = ctx.entry(u32_at(chunk, 0)).map_err(Error::Shared)?;
        }
    }
    if let Some(bnd) = bnd_entries {
        if bnd.value().is_empty() || !bnd.value().len().is_multiple_of(4) {
            return Err(Error::Descriptor);
        }
        for chunk in bnd.value().chunks_exact(4) {
            let _ = ctx.entry(u32_at(chunk, 0)).map_err(Error::Shared)?;
        }
    }
    match kind {
        ConstitutiveKind::Vacuum | ConstitutiveKind::IsotropicNondispersive => {}
        ConstitutiveKind::DebyeIsotropic
        | ConstitutiveKind::LorentzIsotropic
        | ConstitutiveKind::AnisotropicTabulated => {
            if aux_entries.is_none() || bnd_entries.is_none() {
                return Err(Error::UnsupportedProfile);
            }
        }
    }

    // Staggered temporal offsets (component_time_offset_num / den in clock ticks).
    let nums = field(ext, 27);
    let dens = field(ext, 28);
    match (nums, dens) {
        (None, None) => {}
        (Some(n_f), Some(d_f)) => {
            if n_f.element_count() != d_f.element_count() || n_f.element_count() < 6 {
                return Err(Error::Shape);
            }
            for i in 0..n_f.element_count() as usize {
                let num = i64_at(n_f.value(), i * 8);
                let den = u64_at(d_f.value(), i * 8);
                if den == 0 || num < 0 {
                    return Err(Error::Time);
                }
                if i < 3 && num != 0 {
                    return Err(Error::Time);
                }
                if (3..6).contains(&i) && (num == 0 || num as u64 >= den) {
                    return Err(Error::Time);
                }
            }
        }
        _ => return Err(Error::Time),
    }
    Ok(())
}

/// Exact 3D Courant-Friedrichs-Lewy maximum stable time step for homogeneous isotropic cell.
pub fn yee_cfl_max_dt_seconds(spacing_m: [f64; 3], eps_r: f64, mu_r: f64) -> Result<f64, Error> {
    if spacing_m.iter().any(|v| !v.is_finite()) || !eps_r.is_finite() || !mu_r.is_finite() {
        return Err(Error::NonFinite);
    }
    if spacing_m.iter().any(|&v| v <= 0.0) || eps_r <= 0.0 || mu_r <= 0.0 {
        return Err(Error::Value);
    }
    let inv_h2 = 1.0 / (spacing_m[0] * spacing_m[0])
        + 1.0 / (spacing_m[1] * spacing_m[1])
        + 1.0 / (spacing_m[2] * spacing_m[2]);
    let c_mat = SPEED_OF_LIGHT_M_PER_S / (eps_r * mu_r).sqrt();
    Ok(1.0 / (c_mat * inv_h2.sqrt()))
}

/// Checks `dt_s <= dt_cfl` and returns the Courant number `S in (0, 1]`.
pub fn yee_check_cfl(
    dt_s: f64,
    spacing_m: [f64; 3],
    eps_r: f64,
    mu_r: f64,
) -> Result<f64, Error> {
    if !dt_s.is_finite() {
        return Err(Error::NonFinite);
    }
    if dt_s <= 0.0 {
        return Err(Error::Time);
    }
    let dt_max = yee_cfl_max_dt_seconds(spacing_m, eps_r, mu_r)?;
    let s = dt_s / dt_max;
    if s > 1.0 + 1.0e-12 {
        return Err(Error::Time);
    }
    Ok(s.min(1.0))
}

/// Solves discrete 3D Yee plane-wave dispersion for angular frequency `omega (rad/s)`:
/// `sin^2(omega * dt / 2) = (c_mat * dt)^2 * sum_i sin^2(k_i * dx_i / 2) / dx_i^2`.
pub fn yee_discrete_dispersion_omega(
    k_rad_per_m: [f64; 3],
    spacing_m: [f64; 3],
    dt_s: f64,
    eps_r: f64,
    mu_r: f64,
) -> Result<f64, Error> {
    if k_rad_per_m.iter().any(|v| !v.is_finite()) {
        return Err(Error::NonFinite);
    }
    let _ = yee_check_cfl(dt_s, spacing_m, eps_r, mu_r)?;
    let c_mat = SPEED_OF_LIGHT_M_PER_S / (eps_r * mu_r).sqrt();
    let mut sum = 0.0f64;
    for i in 0..3 {
        let s = (0.5 * k_rad_per_m[i] * spacing_m[i]).sin();
        sum += (s * s) / (spacing_m[i] * spacing_m[i]);
    }
    let rhs = (c_mat * dt_s) * (c_mat * dt_s) * sum;
    if rhs > 1.0 + 1.0e-12 {
        return Err(Error::Value);
    }
    Ok((2.0 / dt_s) * rhs.clamp(0.0, 1.0).sqrt().asin())
}

#[inline]
fn cell_idx(x: usize, y: usize, z: usize, dims: [usize; 3]) -> usize {
    x + dims[0] * (y + dims[1] * z)
}

/// Advances a 3D staggered Yee lattice (`x + nx*(y + ny*z)`) one full leapfrog step:
/// `H^{n+1/2} = H^{n-1/2} - (dt / mu) * curl_forward(E^n)` followed by
/// `E^{n+1} = alpha * E^n + beta * (curl_backward(H^{n+1/2}) - J)` with periodic wraps.
pub fn yee_step_periodic(
    cells: &mut [YeeRecord],
    dims: [u32; 3],
    spacing_m: [f64; 3],
    dt_s: f64,
) -> Result<(), Error> {
    let nx = usize::try_from(dims[0]).map_err(|_| Error::Overflow)?;
    let ny = usize::try_from(dims[1]).map_err(|_| Error::Overflow)?;
    let nz = usize::try_from(dims[2]).map_err(|_| Error::Overflow)?;
    if nx == 0 || ny == 0 || nz == 0 {
        return Err(Error::Shape);
    }
    let expected = nx
        .checked_mul(ny)
        .and_then(|n| n.checked_mul(nz))
        .ok_or(Error::Overflow)?;
    if cells.len() != expected {
        return Err(Error::Shape);
    }
    let d3 = [nx, ny, nz];
    for c in cells.iter() {
        let _ = yee_check_cfl(dt_s, spacing_m, c.epsilon_relative, c.mu_relative)?;
    }

    // 1) Update staggered H^{n+1/2} using forward differences of E^n.
    for z in 0..nz {
        let zp = (z + 1) % nz;
        for y in 0..ny {
            let yp = (y + 1) % ny;
            for x in 0..nx {
                let xp = (x + 1) % nx;
                let idx = cell_idx(x, y, z, d3);
                let e = cells[idx].electric_field_v_per_m;
                let ex_p = cells[cell_idx(xp, y, z, d3)].electric_field_v_per_m;
                let ey_p = cells[cell_idx(x, yp, z, d3)].electric_field_v_per_m;
                let ez_p = cells[cell_idx(x, y, zp, d3)].electric_field_v_per_m;
                let curl_x = (ey_p[2] - e[2]) / spacing_m[1] - (ez_p[1] - e[1]) / spacing_m[2];
                let curl_y = (ez_p[0] - e[0]) / spacing_m[2] - (ex_p[2] - e[2]) / spacing_m[0];
                let curl_z = (ex_p[1] - e[1]) / spacing_m[0] - (ey_p[0] - e[0]) / spacing_m[1];
                let coeff = dt_s / (VACUUM_PERMEABILITY_H_PER_M * cells[idx].mu_relative);
                cells[idx].magnetic_field_h_a_per_m[0] -= coeff * curl_x;
                cells[idx].magnetic_field_h_a_per_m[1] -= coeff * curl_y;
                cells[idx].magnetic_field_h_a_per_m[2] -= coeff * curl_z;
            }
        }
    }

    // 2) Update E^{n+1} using backward differences of H^{n+1/2}.
    for z in 0..nz {
        let zm = (z + nz - 1) % nz;
        for y in 0..ny {
            let ym = (y + ny - 1) % ny;
            for x in 0..nx {
                let xm = (x + nx - 1) % nx;
                let idx = cell_idx(x, y, z, d3);
                let h = cells[idx].magnetic_field_h_a_per_m;
                let hx_m = cells[cell_idx(xm, y, z, d3)].magnetic_field_h_a_per_m;
                let hy_m = cells[cell_idx(x, ym, z, d3)].magnetic_field_h_a_per_m;
                let hz_m = cells[cell_idx(x, y, zm, d3)].magnetic_field_h_a_per_m;
                let curl_x = (h[2] - hy_m[2]) / spacing_m[1] - (h[1] - hz_m[1]) / spacing_m[2];
                let curl_y = (h[0] - hz_m[0]) / spacing_m[2] - (h[2] - hx_m[2]) / spacing_m[0];
                let curl_z = (h[1] - hx_m[1]) / spacing_m[0] - (h[0] - hy_m[0]) / spacing_m[1];
                let eps = VACUUM_PERMITTIVITY_F_PER_M * cells[idx].epsilon_relative;
                let sigma = cells[idx].conductivity_s_per_m;
                let j = cells[idx].impressed_current_j_a_per_m2;
                let loss = sigma * dt_s / (2.0 * eps);
                let ca = (1.0 - loss) / (1.0 + loss);
                let cb = (dt_s / eps) / (1.0 + loss);
                let e_prev = cells[idx].electric_field_v_per_m;
                cells[idx].electric_field_v_per_m = [
                    ca * e_prev[0] + cb * (curl_x - j[0]),
                    ca * e_prev[1] + cb * (curl_y - j[1]),
                    ca * e_prev[2] + cb * (curl_z - j[2]),
                ];
            }
        }
    }
    Ok(())
}

/// Computes total electromagnetic energy across Yee cells using isotropic permittivity/permeability.
pub fn yee_total_energy_j(cells: &[YeeRecord], spacing_m: [f64; 3]) -> Result<f64, Error> {
    if spacing_m.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err(Error::Value);
    }
    let vol = spacing_m[0] * spacing_m[1] * spacing_m[2];
    let mut sum = 0.0f64;
    for c in cells {
        sum += vol
            * energy_density_isotropic(
                c.electric_field_v_per_m,
                c.magnetic_field_h_a_per_m,
                c.epsilon_relative,
                c.mu_relative,
            )?;
    }
    Ok(sum)
}
