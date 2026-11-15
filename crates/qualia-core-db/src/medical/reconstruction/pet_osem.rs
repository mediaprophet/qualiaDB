//! Full 2D PET Ordered-Subsets Expectation Maximization (OSEM) using exact
//! Siddon / Amanatides-Woo ray-driven length-of-intersection projection,
//! multiplicative attenuation & normalization sensitivity, and count conservation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsemError {
    DimensionMismatch,
    InvalidConfig(&'static str),
    NonFiniteInput,
    NegativeMeasurements,
    Overflow,
}

impl std::fmt::Display for OsemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PET OSEM reconstruction error: {self:?}")
    }
}
impl std::error::Error for OsemError {}

#[derive(Clone, Copy, Debug)]
pub struct OsemConfig {
    pub image_width: usize,
    pub image_height: usize,
    pub pixel_size_mm: f64,
    pub subsets: usize,
    pub iterations: usize,
    pub min_sensitivity: f64,
    pub initial_activity: f64,
}

#[derive(Clone, Debug)]
pub struct PetSinogram<'a> {
    pub source_blake3: [u8; 32],
    pub n_angles: usize,
    pub n_bins: usize,
    pub bin_spacing_mm: f64,
    pub angles_rad: &'a [f64],
    pub prompts: &'a [f64],
    pub attenuation_factors: Option<&'a [f64]>,
    pub normalization_factors: Option<&'a [f64]>,
}

#[derive(Clone, Debug)]
pub struct OsemResult {
    pub source_blake3: [u8; 32],
    pub derivation_blake3: [u8; 32],
    pub sensitivity_image: Vec<f64>,
    pub per_iteration_l1_residual: Vec<f64>,
    pub total_measured_counts: f64,
    pub total_forward_counts: f64,
    pub min_voxel: f64,
}

/// Exact parametric Siddon ray-grid chord intersection through an `H x W` pixel grid
/// centered at the isocenter `(0, 0)` along projection line `x cos(theta) + y sin(theta) = u`.
pub fn trace_ray_into(
    angle_rad: f64,
    radial_u_mm: f64,
    width: usize,
    height: usize,
    pixel_size_mm: f64,
    out_sparse: &mut Vec<(usize, f64)>,
) {
    out_sparse.clear();
    let (sin_t, cos_t) = angle_rad.sin_cos();
    let ox = radial_u_mm * cos_t;
    let oy = radial_u_mm * sin_t;
    let dx = -sin_t;
    let dy = cos_t;

    let half_w = 0.5 * width as f64 * pixel_size_mm;
    let half_h = 0.5 * height as f64 * pixel_size_mm;

    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;
    if dx.abs() > 1e-12 {
        let tx0 = (-half_w - ox) / dx;
        let tx1 = (half_w - ox) / dx;
        t_min = t_min.max(tx0.min(tx1));
        t_max = t_max.min(tx0.max(tx1));
    } else if ox < -half_w || ox > half_w {
        return;
    }
    if dy.abs() > 1e-12 {
        let ty0 = (-half_h - oy) / dy;
        let ty1 = (half_h - oy) / dy;
        t_min = t_min.max(ty0.min(ty1));
        t_max = t_max.min(ty0.max(ty1));
    } else if oy < -half_h || oy > half_h {
        return;
    }
    if t_max <= t_min + 1e-9 {
        return;
    }

    // Collect sorted parametric grid plane intersections inside [t_min, t_max]
    let mut ts = Vec::with_capacity(width + height + 2);
    ts.push(t_min);
    ts.push(t_max);
    if dx.abs() > 1e-12 {
        for ix in 1..width {
            let x_plane = -half_w + ix as f64 * pixel_size_mm;
            let t = (x_plane - ox) / dx;
            if t > t_min + 1e-10 && t < t_max - 1e-10 {
                ts.push(t);
            }
        }
    }
    if dy.abs() > 1e-12 {
        for iy in 1..height {
            let y_plane = -half_h + iy as f64 * pixel_size_mm;
            let t = (y_plane - oy) / dy;
            if t > t_min + 1e-10 && t < t_max - 1e-10 {
                ts.push(t);
            }
        }
    }
    ts.sort_by(|a, b| a.total_cmp(b));

    for seg in ts.windows(2) {
        let len = seg[1] - seg[0];
        if len <= 1e-11 {
            continue;
        }
        let tm = 0.5 * (seg[0] + seg[1]);
        let mx = ox + tm * dx;
        let my = oy + tm * dy;
        let px = ((mx + half_w) / pixel_size_mm).floor() as isize;
        let py = ((my + half_h) / pixel_size_mm).floor() as isize;
        if px >= 0 && px < width as isize && py >= 0 && py < height as isize {
            let idx = (py as usize) * width + (px as usize);
            if let Some(last) = out_sparse.last_mut() {
                if last.0 == idx {
                    last.1 += len;
                    continue;
                }
            }
            out_sparse.push((idx, len));
        }
    }
}

/// Forward-project an activity map `x` across a PET sinogram with attenuation & normalization.
pub fn forward_project_pet(
    image: &[f64],
    sino: &PetSinogram<'_>,
    pixel_size_mm: f64,
    width: usize,
    height: usize,
) -> Result<Vec<f64>, OsemError> {
    if image.len() != width * height || sino.angles_rad.len() != sino.n_angles {
        return Err(OsemError::DimensionMismatch);
    }
    let total_rays = sino.n_angles * sino.n_bins;
    let mut out = vec![0.0; total_rays];
    let u_mid = 0.5 * (sino.n_bins - 1) as f64;
    let mut ray = Vec::new();
    for (ai, &theta) in sino.angles_rad.iter().enumerate() {
        for bi in 0..sino.n_bins {
            let u = (bi as f64 - u_mid) * sino.bin_spacing_mm;
            trace_ray_into(theta, u, width, height, pixel_size_mm, &mut ray);
            let idx = ai * sino.n_bins + bi;
            let acf = sino.attenuation_factors.map_or(1.0, |a| a[idx]);
            let norm = sino.normalization_factors.map_or(1.0, |n| n[idx]);
            let w_ray = acf * norm;
            let mut sum = 0.0;
            for &(pix, chord) in &ray {
                sum += chord * image[pix];
            }
            out[idx] = sum * w_ray;
        }
    }
    Ok(out)
}

pub fn reconstruct_pet_osem_into(
    sino: &PetSinogram<'_>,
    cfg: &OsemConfig,
    output: &mut [f64],
) -> Result<OsemResult, OsemError> {
    let n_pix = cfg.image_width.checked_mul(cfg.image_height).ok_or(OsemError::Overflow)?;
    let n_rays = sino.n_angles.checked_mul(sino.n_bins).ok_or(OsemError::Overflow)?;
    if n_pix == 0 || n_rays == 0 || sino.angles_rad.len() != sino.n_angles || sino.prompts.len() != n_rays {
        return Err(OsemError::DimensionMismatch);
    }
    if output.len() < n_pix {
        return Err(OsemError::DimensionMismatch);
    }
    if cfg.subsets == 0 || cfg.subsets > sino.n_angles || cfg.iterations == 0 || cfg.pixel_size_mm <= 0.0 {
        return Err(OsemError::InvalidConfig("invalid OSEM subsets, iterations, or pixel size"));
    }
    if cfg.initial_activity <= 0.0 || !cfg.initial_activity.is_finite() || sino.bin_spacing_mm <= 0.0 {
        return Err(OsemError::InvalidConfig("initial_activity and bin_spacing must be positive finite"));
    }
    if sino.prompts.iter().any(|&v| !v.is_finite()) || sino.angles_rad.iter().any(|&v| !v.is_finite()) {
        return Err(OsemError::NonFiniteInput);
    }
    if sino.prompts.iter().any(|&v| v < 0.0) {
        return Err(OsemError::NegativeMeasurements);
    }
    for opt in [sino.attenuation_factors, sino.normalization_factors].into_iter().flatten() {
        if opt.len() != n_rays {
            return Err(OsemError::DimensionMismatch);
        }
        if opt.iter().any(|&v| !v.is_finite() || v < 0.0) {
            return Err(OsemError::NonFiniteInput);
        }
    }

    // Precompute subset ray sparse system matrices `W_ij` and subset sensitivity images `S_j^{(m)}`.
    let u_mid = 0.5 * (sino.n_bins - 1) as f64;
    let mut subset_rays: Vec<Vec<(usize, Vec<(usize, f64)>)>> = vec![Vec::new(); cfg.subsets];
    let mut subset_sens: Vec<Vec<f64>> = vec![vec![0.0; n_pix]; cfg.subsets];
    let mut global_sens = vec![0.0; n_pix];
    let mut scratch = Vec::new();

    for (ai, &theta) in sino.angles_rad.iter().enumerate() {
        let sub = ai % cfg.subsets;
        for bi in 0..sino.n_bins {
            let ray_idx = ai * sino.n_bins + bi;
            let u = (bi as f64 - u_mid) * sino.bin_spacing_mm;
            trace_ray_into(theta, u, cfg.image_width, cfg.image_height, cfg.pixel_size_mm, &mut scratch);
            if scratch.is_empty() {
                continue;
            }
            let acf = sino.attenuation_factors.map_or(1.0, |a| a[ray_idx]);
            let norm = sino.normalization_factors.map_or(1.0, |n| n[ray_idx]);
            let eff = acf * norm;
            if eff <= 0.0 {
                continue;
            }
            let mut entries = Vec::with_capacity(scratch.len());
            for &(pix, chord) in &scratch {
                let w = chord * eff;
                subset_sens[sub][pix] += w;
                global_sens[pix] += w;
                entries.push((pix, w));
            }
            subset_rays[sub].push((ray_idx, entries));
        }
    }

    let mut estimate = vec![cfg.initial_activity; n_pix];
    for j in 0..n_pix {
        if global_sens[j] < cfg.min_sensitivity {
            estimate[j] = 0.0;
        }
    }

    let mut residuals = Vec::with_capacity(cfg.iterations);
    let mut back_buf = vec![0.0; n_pix];

    for _iter in 0..cfg.iterations {
        for sub in 0..cfg.subsets {
            back_buf.fill(0.0);
            for (ray_idx, entries) in &subset_rays[sub] {
                let mut fp = 0.0;
                for &(pix, w) in entries {
                    fp += w * estimate[pix];
                }
                if fp <= 1e-18 {
                    continue;
                }
                let ratio = sino.prompts[*ray_idx] / fp;
                for &(pix, w) in entries {
                    back_buf[pix] += w * ratio;
                }
            }
            for j in 0..n_pix {
                let s = subset_sens[sub][j];
                if s >= cfg.min_sensitivity {
                    estimate[j] = (estimate[j] * (back_buf[j] / s)).max(0.0);
                }
            }
        }
        let fwd = forward_project_pet(&estimate, sino, cfg.pixel_size_mm, cfg.image_width, cfg.image_height)?;
        let l1: f64 = fwd.iter().zip(sino.prompts.iter()).map(|(f, y)| (f - y).abs()).sum();
        residuals.push(l1);
    }

    let final_fwd = forward_project_pet(&estimate, sino, cfg.pixel_size_mm, cfg.image_width, cfg.image_height)?;
    let total_measured_counts = sino.prompts.iter().sum();
    let total_forward_counts = final_fwd.iter().sum();
    let min_voxel = estimate.iter().copied().fold(f64::INFINITY, f64::min);

    let mut hasher = blake3::Hasher::new();
    hasher.update(&sino.source_blake3);
    for v in &estimate {
        hasher.update(&v.to_le_bytes());
    }
    output[..n_pix].copy_from_slice(&estimate);

    Ok(OsemResult {
        source_blake3: sino.source_blake3,
        derivation_blake3: *hasher.finalize().as_bytes(),
        sensitivity_image: global_sens,
        per_iteration_l1_residual: residuals,
        total_measured_counts,
        total_forward_counts,
        min_voxel,
    })
}
