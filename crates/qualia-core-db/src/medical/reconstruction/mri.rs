//! Complete 2D/3D complex k-space MRI reconstruction (IFFT + centered fftshift + wrapped
//! phase preservation) and multi-coil SoS & Cartesian SENSE unaliasing with g-factor map.
use super::spectral_fft::{fft_2d, fft_3d, fftshift_1d, fftshift_3d, validate_finite_complex};
use crate::solvers::transforms::fourier::Cplx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MriError {
    DimensionMismatch,
    InvalidAccelFactor,
    MissingCoilData,
    NonFiniteInput,
    SingularSenseSystem,
    Overflow,
}

impl std::fmt::Display for MriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MRI reconstruction error: {self:?}")
    }
}
impl std::error::Error for MriError {}

#[derive(Clone, Copy, Debug)]
pub struct MriKspaceConfig {
    pub slices: usize,
    pub rows: usize,
    pub cols: usize,
    pub ifftshift_kspace: bool,
    pub fftshift_image: bool,
    pub unitary_scaling: bool,
}

#[derive(Clone, Debug)]
pub struct MriComplexVolume {
    pub source_blake3: [u8; 32],
    pub derivation_blake3: [u8; 32],
    pub slices: usize,
    pub rows: usize,
    pub cols: usize,
    pub complex: Vec<Cplx>,
    pub magnitude: Vec<f64>,
    pub phase_rad: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct SenseOutput {
    pub derivation_blake3: [u8; 32],
    pub rows: usize,
    pub cols: usize,
    pub complex: Vec<Cplx>,
    pub magnitude: Vec<f64>,
    pub phase_rad: Vec<f64>,
    pub g_factor: Vec<f64>,
}

fn c_mul(a: Cplx, b: Cplx) -> Cplx {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

fn c_conj(a: Cplx) -> Cplx {
    (a.0, -a.1)
}

fn c_div(a: Cplx, b: Cplx) -> Result<Cplx, MriError> {
    let denom = b.0 * b.0 + b.1 * b.1;
    if denom <= 1e-30 || !denom.is_finite() {
        return Err(MriError::SingularSenseSystem);
    }
    Ok(((a.0 * b.0 + a.1 * b.1) / denom, (a.1 * b.0 - a.0 * b.1) / denom))
}

fn shift_2d(data: &[Cplx], rows: usize, cols: usize, inverse: bool) -> Vec<Cplx> {
    let mut tmp = vec![(0.0, 0.0); data.len()];
    let mut row_tmp = vec![(0.0, 0.0); cols];
    for r in 0..rows {
        let src = &data[r * cols..(r + 1) * cols];
        if inverse {
            crate::medical::reconstruction::spectral_fft::ifftshift_1d(src, &mut row_tmp);
        } else {
            fftshift_1d(src, &mut row_tmp);
        }
        tmp[r * cols..(r + 1) * cols].copy_from_slice(&row_tmp);
    }
    let split = if inverse { rows.div_ceil(2) } else { rows / 2 };
    let mut out = vec![(0.0, 0.0); data.len()];
    for r in 0..rows {
        let dr = (r + (rows - split)) % rows;
        out[dr * cols..(dr + 1) * cols].copy_from_slice(&tmp[r * cols..(r + 1) * cols]);
    }
    out
}

fn hash_complex_slice(source_blake3: [u8; 32], complex: &[Cplx]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&source_blake3);
    for &(re, im) in complex {
        hasher.update(&re.to_le_bytes());
        hasher.update(&im.to_le_bytes());
    }
    *hasher.finalize().as_bytes()
}

pub fn reconstruct_mri_kspace_2d(
    kspace: &[Cplx],
    rows: usize,
    cols: usize,
    source_blake3: [u8; 32],
    ifftshift_kspace: bool,
    fftshift_image: bool,
    unitary_scaling: bool,
) -> Result<MriComplexVolume, MriError> {
    reconstruct_mri_kspace_3d(
        kspace,
        MriKspaceConfig {
            slices: 1,
            rows,
            cols,
            ifftshift_kspace,
            fftshift_image,
            unitary_scaling,
        },
        source_blake3,
    )
}

pub fn reconstruct_mri_kspace_3d(
    kspace: &[Cplx],
    cfg: MriKspaceConfig,
    source_blake3: [u8; 32],
) -> Result<MriComplexVolume, MriError> {
    let n = cfg
        .slices
        .checked_mul(cfg.rows)
        .and_then(|v| v.checked_mul(cfg.cols))
        .ok_or(MriError::Overflow)?;
    if n == 0 || kspace.len() != n {
        return Err(MriError::DimensionMismatch);
    }
    validate_finite_complex(kspace).map_err(|_| MriError::NonFiniteInput)?;

    let prep = if cfg.ifftshift_kspace {
        if cfg.slices == 1 {
            shift_2d(kspace, cfg.rows, cfg.cols, true)
        } else {
            fftshift_3d(kspace, cfg.slices, cfg.rows, cfg.cols, true)
                .map_err(|_| MriError::DimensionMismatch)?
        }
    } else {
        kspace.to_vec()
    };

    let mut img = if cfg.slices == 1 {
        fft_2d(&prep, cfg.rows, cfg.cols, true).map_err(|_| MriError::NonFiniteInput)?
    } else {
        fft_3d(&prep, cfg.slices, cfg.rows, cfg.cols, true).map_err(|_| MriError::NonFiniteInput)?
    };

    if cfg.unitary_scaling {
        let u_scale = (n as f64).sqrt();
        for v in &mut img {
            v.0 *= u_scale;
            v.1 *= u_scale;
        }
    }

    if cfg.fftshift_image {
        img = if cfg.slices == 1 {
            shift_2d(&img, cfg.rows, cfg.cols, false)
        } else {
            fftshift_3d(&img, cfg.slices, cfg.rows, cfg.cols, false)
                .map_err(|_| MriError::DimensionMismatch)?
        };
    }

    let magnitude: Vec<f64> = img.iter().map(|&(re, im)| re.hypot(im)).collect();
    let phase_rad: Vec<f64> = img.iter().map(|&(re, im)| im.atan2(re)).collect();
    let derivation_blake3 = hash_complex_slice(source_blake3, &img);

    Ok(MriComplexVolume {
        source_blake3,
        derivation_blake3,
        slices: cfg.slices,
        rows: cfg.rows,
        cols: cfg.cols,
        complex: img,
        magnitude,
        phase_rad,
    })
}

/// Multi-coil Root-Sum-of-Squares magnitude combination `sqrt(sum_c |I_c / sigma_c|^2)`.
pub fn combine_coils_sos(coil_images: &[&[Cplx]], noise_std: Option<&[f64]>) -> Result<Vec<f64>, MriError> {
    if coil_images.is_empty() || coil_images[0].is_empty() {
        return Err(MriError::MissingCoilData);
    }
    let n = coil_images[0].len();
    for &c in coil_images {
        if c.len() != n {
            return Err(MriError::DimensionMismatch);
        }
        validate_finite_complex(c).map_err(|_| MriError::NonFiniteInput)?;
    }
    if let Some(sig) = noise_std {
        if sig.len() != coil_images.len() || sig.iter().any(|&s| !s.is_finite() || s <= 0.0) {
            return Err(MriError::DimensionMismatch);
        }
    }
    let mut out = vec![0.0f64; n];
    for (ci, &coil) in coil_images.iter().enumerate() {
        let inv_var = noise_std.map_or(1.0, |s| 1.0 / (s[ci] * s[ci]));
        for (i, &(re, im)) in coil.iter().enumerate() {
            out[i] += (re * re + im * im) * inv_var;
        }
    }
    for v in &mut out {
        *v = v.sqrt();
    }
    Ok(out)
}

/// Invert small `[r x r]` complex matrix via Gauss-Jordan with partial pivoting.
fn invert_complex_matrix(mat: &[Cplx], r: usize) -> Result<Vec<Cplx>, MriError> {
    let mut a = mat.to_vec();
    let mut inv = vec![(0.0, 0.0); r * r];
    for i in 0..r {
        inv[i * r + i] = (1.0, 0.0);
    }
    for col in 0..r {
        let mut piv = col;
        let mut best_mag = a[col * r + col].0.hypot(a[col * r + col].1);
        for row in (col + 1)..r {
            let m = a[row * r + col].0.hypot(a[row * r + col].1);
            if m > best_mag {
                best_mag = m;
                piv = row;
            }
        }
        if best_mag <= 1e-14 || !best_mag.is_finite() {
            return Err(MriError::SingularSenseSystem);
        }
        if piv != col {
            for k in 0..r {
                a.swap(col * r + k, piv * r + k);
                inv.swap(col * r + k, piv * r + k);
            }
        }
        let diag = a[col * r + col];
        for k in 0..r {
            a[col * r + k] = c_div(a[col * r + k], diag)?;
            inv[col * r + k] = c_div(inv[col * r + k], diag)?;
        }
        for row in 0..r {
            if row == col {
                continue;
            }
            let factor = a[row * r + col];
            if factor == (0.0, 0.0) {
                continue;
            }
            for k in 0..r {
                let sub_a = c_mul(factor, a[col * r + k]);
                let sub_i = c_mul(factor, inv[col * r + k]);
                a[row * r + k].0 -= sub_a.0;
                a[row * r + k].1 -= sub_a.1;
                inv[row * r + k].0 -= sub_i.0;
                inv[row * r + k].1 -= sub_i.1;
            }
        }
    }
    Ok(inv)
}

/// Pruessmann Cartesian SENSE coil unaliasing across phase-encode reduction `accel_r`.
/// `aliased_coil_images`: `[n_coils][(rows / accel_r) * cols]`.
/// `sensitivity_maps`: `[n_coils][rows * cols]`.
pub fn combine_coils_sense(
    aliased_coil_images: &[&[Cplx]],
    sensitivity_maps: &[&[Cplx]],
    rows: usize,
    cols: usize,
    accel_r: usize,
    noise_cov_inv_diag: Option<&[f64]>,
    reg_lambda: f64,
    source_blake3: [u8; 32],
) -> Result<SenseOutput, MriError> {
    if accel_r == 0 || rows == 0 || cols == 0 || rows % accel_r != 0 {
        return Err(MriError::InvalidAccelFactor);
    }
    let nc = aliased_coil_images.len();
    if nc == 0 || sensitivity_maps.len() != nc || nc < accel_r {
        return Err(MriError::MissingCoilData);
    }
    let red_rows = rows / accel_r;
    let red_len = red_rows * cols;
    let full_len = rows * cols;
    for c in 0..nc {
        if aliased_coil_images[c].len() != red_len || sensitivity_maps[c].len() != full_len {
            return Err(MriError::DimensionMismatch);
        }
        validate_finite_complex(aliased_coil_images[c]).map_err(|_| MriError::NonFiniteInput)?;
        validate_finite_complex(sensitivity_maps[c]).map_err(|_| MriError::NonFiniteInput)?;
    }
    if let Some(inv_var) = noise_cov_inv_diag {
        if inv_var.len() != nc || inv_var.iter().any(|&v| !v.is_finite() || v <= 0.0) {
            return Err(MriError::DimensionMismatch);
        }
    }

    let mut out_cx = vec![(0.0, 0.0); full_len];
    let mut g_factor = vec![1.0; full_len];

    for y0 in 0..red_rows {
        for x in 0..cols {
            let mut sh_psi_s = vec![(0.0, 0.0); accel_r * accel_r];
            let mut sh_psi_m = vec![(0.0, 0.0); accel_r];
            for c in 0..nc {
                let psi = noise_cov_inv_diag.map_or(1.0, |v| v[c]);
                let m_val = aliased_coil_images[c][y0 * cols + x];
                for r1 in 0..accel_r {
                    let y1 = y0 + r1 * red_rows;
                    let s1_conj = c_conj(sensitivity_maps[c][y1 * cols + x]);
                    let term_m = c_mul(s1_conj, m_val);
                    sh_psi_m[r1].0 += psi * term_m.0;
                    sh_psi_m[r1].1 += psi * term_m.1;
                    for r2 in 0..accel_r {
                        let y2 = y0 + r2 * red_rows;
                        let s2 = sensitivity_maps[c][y2 * cols + x];
                        let prod = c_mul(s1_conj, s2);
                        sh_psi_s[r1 * accel_r + r2].0 += psi * prod.0;
                        sh_psi_s[r1 * accel_r + r2].1 += psi * prod.1;
                    }
                }
            }
            // Capture unregularized Hermitian diagonal for g-factor calculation
            let unreg_diag: Vec<f64> = (0..accel_r).map(|r| sh_psi_s[r * accel_r + r].0).collect();
            for r in 0..accel_r {
                sh_psi_s[r * accel_r + r].0 += reg_lambda.max(0.0);
            }
            let a_inv = invert_complex_matrix(&sh_psi_s, accel_r)?;
            for r in 0..accel_r {
                let y = y0 + r * red_rows;
                let mut rho = (0.0, 0.0);
                for k in 0..accel_r {
                    let t = c_mul(a_inv[r * accel_r + k], sh_psi_m[k]);
                    rho.0 += t.0;
                    rho.1 += t.1;
                }
                out_cx[y * cols + x] = rho;
                let g_sq = (a_inv[r * accel_r + r].0 * unreg_diag[r]).max(1.0);
                g_factor[y * cols + x] = g_sq.sqrt();
            }
        }
    }

    let magnitude = out_cx.iter().map(|&(re, im)| re.hypot(im)).collect();
    let phase_rad = out_cx.iter().map(|&(re, im)| im.atan2(re)).collect();
    let derivation_blake3 = hash_complex_slice(source_blake3, &out_cx);
    Ok(SenseOutput {
        derivation_blake3,
        rows,
        cols,
        complex: out_cx,
        magnitude,
        phase_rad,
        g_factor,
    })
}
