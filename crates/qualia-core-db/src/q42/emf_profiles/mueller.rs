//! Packet E04: Bounded spectral extensions (0x0103: 24/28-band, precision profiles,
//! bin/Jacobian integration) and full 4x4 Mueller scattering matrices (0x0104) with
//! Cloude Hermitian coherency eigendecomposition & Stokes-Mueller physical realizability.

use super::spectrum::{
    density_coordinate_jacobian, DensityCoordinate, Quantity, SampleSemantics,
};
use super::*;

pub const MUELLER_PROFILE: &str = "mueller_scattering_v1";
pub const MUELLER_MODEL_URI: &str = "urn:qualia:emf:mueller_scattering_v1";

pub const BAND_COUNT_24: usize = 24;
pub const BAND_COUNT_28: usize = 28;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpectralPrecisionProfile {
    Float64,
    Float32,
    QuantizedScaledI32 { scale: f64, offset: f64 },
}

impl SpectralPrecisionProfile {
    pub fn encode_value(self, physical: f64) -> Result<f64, Error> {
        if !physical.is_finite() {
            return Err(Error::NonFinite);
        }
        match self {
            Self::Float64 => Ok(physical),
            Self::Float32 => Ok(f64::from(physical as f32)),
            Self::QuantizedScaledI32 { scale, offset } => {
                if !scale.is_finite() || !offset.is_finite() || scale <= 0.0 {
                    return Err(Error::Value);
                }
                let raw = ((physical - offset) / scale).round();
                if !raw.is_finite() || raw < i32::MIN as f64 || raw > i32::MAX as f64 {
                    return Err(Error::Overflow);
                }
                Ok((raw as i32) as f64 * scale + offset)
            }
        }
    }
}

/// Validates strictly ascending positive spectral bin edges (e.g. 25 edges for 24 bands, 29 for 28).
pub fn validate_band_edges(edges: &[f64]) -> Result<usize, Error> {
    if edges.len() < 2 || edges.len() - 1 > 4096 {
        return Err(Error::Shape);
    }
    let mut prev = 0.0f64;
    for &v in edges {
        if !v.is_finite() {
            return Err(Error::NonFinite);
        }
        if v <= 0.0 || v <= prev {
            return Err(Error::Value);
        }
        prev = v;
    }
    Ok(edges.len() - 1)
}

/// Fills `out_edges` (length `band_count + 1`) with deterministic log-spaced positive coordinates.
pub fn build_log_band_edges(min_coord: f64, max_coord: f64, out_edges: &mut [f64]) -> Result<usize, Error> {
    if !min_coord.is_finite() || !max_coord.is_finite() {
        return Err(Error::NonFinite);
    }
    if min_coord <= 0.0 || max_coord <= min_coord || out_edges.len() < 2 {
        return Err(Error::Value);
    }
    let bands = out_edges.len() - 1;
    if bands > 4096 {
        return Err(Error::Shape);
    }
    let ratio = (max_coord / min_coord).ln();
    for (i, slot) in out_edges.iter_mut().enumerate() {
        let frac = i as f64 / bands as f64;
        *slot = if i == 0 {
            min_coord
        } else if i == bands {
            max_coord
        } else {
            min_coord * (frac * ratio).exp()
        };
    }
    Ok(bands)
}

/// Integrates a spectral sample vector along an admitted `AxisView` respecting `PointDensity`
/// (trapezoid), `BinIntegral` (direct bin sum), or `BinAverage` (width-weighted sum).
pub fn integrate_spectrum_over_axis(axis: AxisView<'_>, values: &[f64]) -> Result<f64, Error> {
    let n = usize::try_from(axis.sample_count()).map_err(|_| Error::Overflow)?;
    if values.len() != n || n == 0 {
        return Err(Error::Shape);
    }
    for &v in values {
        if !v.is_finite() {
            return Err(Error::NonFinite);
        }
        if v < 0.0 {
            return Err(Error::Value);
        }
    }
    match axis.semantics() {
        SampleSemantics::BinIntegral => Ok(values.iter().copied().sum()),
        SampleSemantics::BinAverage => {
            let mut acc = 0.0f64;
            for (i, &val) in values.iter().enumerate() {
                let lo = axis.coordinate(i as u64).map_err(Error::Spectrum)?;
                let hi = axis.coordinate(i as u64 + 1).map_err(Error::Spectrum)?;
                acc += val * (hi - lo);
            }
            Ok(acc)
        }
        SampleSemantics::PointDensity => {
            if n == 1 {
                return Err(Error::Shape);
            }
            let mut acc = 0.0f64;
            for i in 0..(n - 1) {
                let x0 = axis.coordinate(i as u64).map_err(Error::Spectrum)?;
                let x1 = axis.coordinate(i as u64 + 1).map_err(Error::Spectrum)?;
                acc += 0.5 * (values[i] + values[i + 1]) * (x1 - x0);
            }
            Ok(acc)
        }
    }
}

/// Applies explicit instrument response weights per bin to spectral samples.
pub fn apply_spectral_response(
    samples: &[f64],
    response_weights: &[f64],
    out: &mut [f64],
) -> Result<(), Error> {
    if samples.len() != response_weights.len() || out.len() < samples.len() {
        return Err(Error::Shape);
    }
    for i in 0..samples.len() {
        if !samples[i].is_finite() || !response_weights[i].is_finite() {
            return Err(Error::NonFinite);
        }
        if samples[i] < 0.0 || response_weights[i] < 0.0 {
            return Err(Error::Value);
        }
        out[i] = samples[i] * response_weights[i];
    }
    Ok(())
}

/// Converts frequency spectral density `S_f` (`per Hz`) to angular (`S_omega = S_f / 2pi`),
/// log-frequency (`S_q = f * S_f` for `q = ln(f / f0)`), or vacuum wavelength (`S_lambda`) Jacobian.
pub fn convert_density_hz(
    s_f: f64,
    f_hz: f64,
    target: DensityTarget,
) -> Result<f64, Error> {
    if !s_f.is_finite() || !f_hz.is_finite() {
        return Err(Error::NonFinite);
    }
    if f_hz <= 0.0 || s_f < 0.0 {
        return Err(Error::Value);
    }
    match target {
        DensityTarget::FrequencyHz => Ok(s_f),
        DensityTarget::AngularFrequencyRadS => Ok(s_f / (2.0 * core::f64::consts::PI)),
        DensityTarget::LogFrequencyNatural => Ok(f_hz * s_f),
        DensityTarget::VacuumWavelengthMetres => {
            let jac = density_coordinate_jacobian(
                Quantity::SpectralIrradiance,
                SampleSemantics::PointDensity,
                DensityCoordinate::Hertz,
                DensityCoordinate::VacuumMetre,
                f_hz,
            )
            .map_err(Error::Spectrum)?;
            jac.apply(s_f).map_err(Error::Spectrum)
        }
    }
}

pub fn convert_density_wavelength_to_hz(s_lambda: f64, lambda_m: f64) -> Result<f64, Error> {
    let jac = density_coordinate_jacobian(
        Quantity::SpectralIrradiance,
        SampleSemantics::PointDensity,
        DensityCoordinate::VacuumMetre,
        DensityCoordinate::Hertz,
        lambda_m,
    )
    .map_err(Error::Spectrum)?;
    jac.apply(s_lambda).map_err(Error::Spectrum)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DensityTarget {
    FrequencyHz,
    AngularFrequencyRadS,
    LogFrequencyNatural,
    VacuumWavelengthMetres,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MuellerUnits {
    DimensionlessTransfer,
    PerSteradian,
}

impl MuellerUnits {
    pub fn parse(units: &str) -> Result<Self, Error> {
        match units {
            "1" => Ok(Self::DimensionlessTransfer),
            "sr^-1" => Ok(Self::PerSteradian),
            _ => Err(Error::Descriptor),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MuellerRecord {
    pub position_m: [f64; 3],
    pub incident_direction: [f64; 3],
    pub outgoing_direction: [f64; 3],
    pub frequency_hz: f64,
    pub material_entry: u32,
    pub flags: u32,
    pub mueller_matrix_m: [[f64; 4]; 4],
}

fn check_unit_vec(v: [f64; 3]) -> Result<(), Error> {
    let norm = ((v[0] * v[0] + v[1] * v[1]) + v[2] * v[2]).sqrt();
    if !norm.is_finite() || (norm - 1.0).abs() > 1.0e-6 {
        return Err(Error::Value);
    }
    Ok(())
}

impl MuellerRecord {
    pub fn parse(
        bytes: &[u8],
        valid: bool,
        require_cloude_realizable: bool,
    ) -> Result<Option<Self>, Error> {
        if bytes.len() != 256 {
            return Err(Error::Shape);
        }
        zeros(&bytes[88..128])?;
        if !valid {
            zeros(bytes)?;
            return Ok(None);
        }
        finite(bytes, 0, 80)?;
        finite(bytes, 128, 256)?;
        let flags = u32_at(bytes, 84);
        if flags != 0 {
            return Err(Error::Flags);
        }
        let inc = triple(bytes, 24);
        let out = triple(bytes, 48);
        check_unit_vec(inc)?;
        check_unit_vec(out)?;
        let freq = f64_at(bytes, 72);
        if freq <= 0.0 {
            return Err(Error::Value);
        }
        let mut m = [[0.0f64; 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                m[r][c] = f64_at(bytes, 128 + (r * 4 + c) * 8);
            }
        }
        if m[0][0] <= 0.0 {
            return Err(Error::Value);
        }
        let col_pol = m[1][0].hypot(m[2][0]).hypot(m[3][0]);
        let row_pol = m[0][1].hypot(m[0][2]).hypot(m[0][3]);
        let pol_tol = m[0][0] * (1.0 + 1.0e-9) + 1.0e-12;
        if col_pol > pol_tol || row_pol > pol_tol {
            return Err(Error::Value);
        }
        if require_cloude_realizable {
            let _ = verify_mueller_physical_realizability(&m)?;
        }
        Ok(Some(Self {
            position_m: triple(bytes, 0),
            incident_direction: inc,
            outgoing_direction: out,
            frequency_hz: freq,
            material_entry: u32_at(bytes, 80),
            flags,
            mueller_matrix_m: m,
        }))
    }

    pub fn encode(&self, valid: bool, out: &mut [u8; 256]) -> Result<(), Error> {
        *out = [0u8; 256];
        if !valid {
            return Ok(());
        }
        if self.flags != 0 {
            return Err(Error::Flags);
        }
        if self.position_m.iter().any(|v| !v.is_finite())
            || self.incident_direction.iter().any(|v| !v.is_finite())
            || self.outgoing_direction.iter().any(|v| !v.is_finite())
            || !self.frequency_hz.is_finite()
        {
            return Err(Error::NonFinite);
        }
        check_unit_vec(self.incident_direction)?;
        check_unit_vec(self.outgoing_direction)?;
        if self.frequency_hz <= 0.0 {
            return Err(Error::Value);
        }
        for r in 0..4 {
            for c in 0..4 {
                if !self.mueller_matrix_m[r][c].is_finite() {
                    return Err(Error::NonFinite);
                }
            }
        }
        let _ = verify_mueller_physical_realizability(&self.mueller_matrix_m)?;
        for (i, v) in self
            .position_m
            .iter()
            .chain(self.incident_direction.iter())
            .chain(self.outgoing_direction.iter())
            .enumerate()
        {
            out[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
        }
        out[72..80].copy_from_slice(&self.frequency_hz.to_le_bytes());
        out[80..84].copy_from_slice(&self.material_entry.to_le_bytes());
        out[84..88].copy_from_slice(&self.flags.to_le_bytes());
        for r in 0..4 {
            for c in 0..4 {
                let off = 128 + (r * 4 + c) * 8;
                out[off..off + 8].copy_from_slice(&self.mueller_matrix_m[r][c].to_le_bytes());
            }
        }
        Ok(())
    }

    pub fn apply_stokes(&self, s_in: [f64; 4]) -> Result<[f64; 4], Error> {
        apply_mueller(&self.mueller_matrix_m, s_in)
    }
}

pub fn encode_mueller_records(
    records: &[MuellerRecord],
    valid: Option<&[bool]>,
    out: &mut [u8],
) -> Result<usize, Error> {
    if let Some(v) = valid {
        if v.len() != records.len() {
            return Err(Error::Shape);
        }
    }
    let needed = records.len().checked_mul(256).ok_or(Error::Overflow)?;
    if out.len() < needed {
        return Err(Error::Shape);
    }
    for (idx, rec) in records.iter().enumerate() {
        let is_valid = valid.is_none_or(|v| v[idx]);
        let dst: &mut [u8; 256] = (&mut out[idx * 256..idx * 256 + 256])
            .try_into()
            .map_err(|_| Error::Shape)?;
        rec.encode(is_valid, dst)?;
    }
    Ok(needed)
}

pub fn apply_mueller(m: &[[f64; 4]; 4], s_in: [f64; 4]) -> Result<[f64; 4], Error> {
    if s_in.iter().any(|v| !v.is_finite()) {
        return Err(Error::NonFinite);
    }
    if s_in[0] < 0.0 || s_in[1].hypot(s_in[2]).hypot(s_in[3]) > s_in[0] * (1.0 + 1.0e-9) + 1.0e-12
    {
        return Err(Error::Value);
    }
    let mut out = [0.0f64; 4];
    for r in 0..4 {
        out[r] = m[r][0] * s_in[0] + m[r][1] * s_in[1] + m[r][2] * s_in[2] + m[r][3] * s_in[3];
    }
    Ok(out)
}

/// Converts a complex 2x2 Jones matrix `J = re + i*im` to its Mueller matrix `M`.
pub fn jones_to_mueller(j_re: [[f64; 2]; 2], j_im: [[f64; 2]; 2]) -> Result<[[f64; 4]; 4], Error> {
    for r in 0..2 {
        for c in 0..2 {
            if !j_re[r][c].is_finite() || !j_im[r][c].is_finite() {
                return Err(Error::NonFinite);
            }
        }
    }
    let sig_re: [[[f64; 2]; 2]; 4] = [
        [[1.0, 0.0], [0.0, 1.0]],
        [[1.0, 0.0], [0.0, -1.0]],
        [[0.0, 1.0], [1.0, 0.0]],
        [[0.0, 0.0], [0.0, 0.0]],
    ];
    let sig_im: [[[f64; 2]; 2]; 4] = [
        [[0.0, 0.0], [0.0, 0.0]],
        [[0.0, 0.0], [0.0, 0.0]],
        [[0.0, 0.0], [0.0, 0.0]],
        [[0.0, -1.0], [1.0, 0.0]],
    ];

    // M_{kl} = 0.5 * Tr(sigma_k * J * sigma_l * J^dagger)
    let mut m = [[0.0f64; 4]; 4];
    for k in 0..4 {
        for l in 0..4 {
            // Compute T = J * sigma_l * J^dagger, then 0.5 * Re(Tr(sigma_k * T)).
            let mut js_re = [[0.0f64; 2]; 2];
            let mut js_im = [[0.0f64; 2]; 2];
            for r in 0..2 {
                for c in 0..2 {
                    for p in 0..2 {
                        js_re[r][c] +=
                            j_re[r][p] * sig_re[l][p][c] - j_im[r][p] * sig_im[l][p][c];
                        js_im[r][c] +=
                            j_re[r][p] * sig_im[l][p][c] + j_im[r][p] * sig_re[l][p][c];
                    }
                }
            }
            let mut t_re = [[0.0f64; 2]; 2];
            let mut t_im = [[0.0f64; 2]; 2];
            for r in 0..2 {
                for c in 0..2 {
                    for p in 0..2 {
                        // (J^dagger)_{p, c} = conj(J_{c, p})
                        let jr = j_re[c][p];
                        let ji = -j_im[c][p];
                        t_re[r][c] += js_re[r][p] * jr - js_im[r][p] * ji;
                        t_im[r][c] += js_re[r][p] * ji + js_im[r][p] * jr;
                    }
                }
            }
            let mut tr = 0.0f64;
            for r in 0..2 {
                for p in 0..2 {
                    tr += sig_re[k][r][p] * t_re[p][r] - sig_im[k][r][p] * t_im[p][r];
                }
            }
            m[k][l] = 0.5 * tr;
        }
    }
    Ok(m)
}

fn stokes_rotator(theta: f64) -> [[f64; 4]; 4] {
    let (s, c) = (2.0 * theta).sin_cos();
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, c, s, 0.0],
        [0.0, -s, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn matmul4(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut out = [[0.0f64; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = a[r][0] * b[0][c]
                + a[r][1] * b[1][c]
                + a[r][2] * b[2][c]
                + a[r][3] * b[3][c];
        }
    }
    out
}

/// Passive basis rotation: `R(theta_out) * M * R(-theta_in)`.
pub fn rotate_mueller_passive(
    m: [[f64; 4]; 4],
    theta_in_rad: f64,
    theta_out_rad: f64,
) -> Result<[[f64; 4]; 4], Error> {
    if !theta_in_rad.is_finite() || !theta_out_rad.is_finite() {
        return Err(Error::NonFinite);
    }
    let r_out = stokes_rotator(theta_out_rad);
    let r_in_inv = stokes_rotator(-theta_in_rad);
    Ok(matmul4(&matmul4(&r_out, &m), &r_in_inv))
}

/// Builds Cloude 4x4 Hermitian coherency matrix `H(M) = (1/4) sum_{k,l} M_{kl} (sigma_k otimes sigma_l^*)`,
/// returning `(real_4x4, imag_4x4)` with trace equal to `M[0][0]`.
pub fn cloude_coherency_matrix(m: &[[f64; 4]; 4]) -> ([[f64; 4]; 4], [[f64; 4]; 4]) {
    let sig_re: [[[f64; 2]; 2]; 4] = [
        [[1.0, 0.0], [0.0, 1.0]],
        [[1.0, 0.0], [0.0, -1.0]],
        [[0.0, 1.0], [1.0, 0.0]],
        [[0.0, 0.0], [0.0, 0.0]],
    ];
    let sig_im: [[[f64; 2]; 2]; 4] = [
        [[0.0, 0.0], [0.0, 0.0]],
        [[0.0, 0.0], [0.0, 0.0]],
        [[0.0, 0.0], [0.0, 0.0]],
        [[0.0, -1.0], [1.0, 0.0]],
    ];
    let mut re = [[0.0f64; 4]; 4];
    let mut im = [[0.0f64; 4]; 4];
    for d in 0..2 {
        for b in 0..2 {
            let row = 2 * d + b;
            for a in 0..2 {
                for c in 0..2 {
                    let col = 2 * a + c;
                    let mut acc_r = 0.0f64;
                    let mut acc_i = 0.0f64;
                    for k in 0..4 {
                        let kr = sig_re[k][d][a];
                        let ki = sig_im[k][d][a];
                        if kr == 0.0 && ki == 0.0 {
                            continue;
                        }
                        for l in 0..4 {
                            let lr = sig_re[l][c][b];
                            let li = sig_im[l][c][b];
                            if lr == 0.0 && li == 0.0 {
                                continue;
                            }
                            let w = m[k][l];
                            acc_r += w * (kr * lr - ki * li);
                            acc_i += w * (kr * li + ki * lr);
                        }
                    }
                    re[row][col] = 0.25 * acc_r;
                    im[row][col] = 0.25 * acc_i;
                }
            }
        }
    }
    (re, im)
}

/// Computes the 4 real eigenvalues of `H(M)` (sorted descending) via exact 8x8 real
/// symmetric Jacobi iteration over `[[A, -B], [B, A]]`.
pub fn cloude_eigenvalues(m: &[[f64; 4]; 4]) -> Result<[f64; 4], Error> {
    let (re, im) = cloude_coherency_matrix(m);
    let mut k = [[0.0f64; 8]; 8];
    for r in 0..4 {
        for c in 0..4 {
            k[r][c] = re[r][c];
            k[r + 4][c + 4] = re[r][c];
            k[r][c + 4] = -im[r][c];
            k[r + 4][c] = im[r][c];
        }
    }
    // Symmetric Jacobi sweeps (0 heap allocation).
    for _ in 0..24 {
        let mut max_off = 0.0f64;
        for p in 0..8 {
            for q in (p + 1)..8 {
                max_off = max_off.max(k[p][q].abs());
            }
        }
        if max_off <= 1.0e-14 * m[0][0].abs().max(1.0) {
            break;
        }
        for p in 0..8 {
            for q in (p + 1)..8 {
                let apq = k[p][q];
                if apq.abs() <= 1.0e-15 * m[0][0].abs().max(1.0) {
                    continue;
                }
                let diff = k[q][q] - k[p][p];
                let t = if diff == 0.0 {
                    if apq >= 0.0 { 1.0 } else { -1.0 }
                } else {
                    let tau = diff / (2.0 * apq);
                    if tau >= 0.0 {
                        1.0 / (tau + (1.0 + tau * tau).sqrt())
                    } else {
                        -1.0 / (-tau + (1.0 + tau * tau).sqrt())
                    }
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;
                let app = k[p][p];
                let aqq = k[q][q];
                k[p][p] = app - t * apq;
                k[q][q] = aqq + t * apq;
                k[p][q] = 0.0;
                k[q][p] = 0.0;
                for r in 0..8 {
                    if r == p || r == q {
                        continue;
                    }
                    let arp = k[r][p];
                    let arq = k[r][q];
                    k[r][p] = c * arp - s * arq;
                    k[p][r] = k[r][p];
                    k[r][q] = s * arp + c * arq;
                    k[q][r] = k[r][q];
                }
            }
        }
    }
    let mut vals8 = [
        k[0][0], k[1][1], k[2][2], k[3][3], k[4][4], k[5][5], k[6][6], k[7][7],
    ];
    vals8.sort_by(|a, b| b.total_cmp(a));
    Ok([vals8[0], vals8[2], vals8[4], vals8[6]])
}

/// Verifies `M00 > 0`, Stokes-cone physicality across pure basis inputs, and Cloude PSD (`H >= 0`).
pub fn verify_mueller_physical_realizability(m: &[[f64; 4]; 4]) -> Result<[f64; 4], Error> {
    for r in 0..4 {
        for c in 0..4 {
            if !m[r][c].is_finite() {
                return Err(Error::NonFinite);
            }
        }
    }
    let m00 = m[0][0];
    if m00 <= 0.0 {
        return Err(Error::Value);
    }
    let basis_stokes = [
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 1.0, 0.0, 0.0],
        [1.0, -1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0, 0.0],
        [1.0, 0.0, -1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, -1.0],
    ];
    for s_in in basis_stokes {
        let s_out = apply_mueller(m, s_in)?;
        let pol = s_out[1].hypot(s_out[2]).hypot(s_out[3]);
        if s_out[0] < -1.0e-12 * m00 || pol > s_out[0].max(0.0) * (1.0 + 1.0e-9) + 1.0e-12 * m00 {
            return Err(Error::Value);
        }
    }
    let evals = cloude_eigenvalues(m)?;
    let tol = -1.0e-9 * m00;
    if evals.iter().any(|&lam| lam < tol) {
        return Err(Error::Value);
    }
    Ok(evals)
}
