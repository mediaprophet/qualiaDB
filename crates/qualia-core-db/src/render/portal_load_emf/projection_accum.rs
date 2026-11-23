//! Shared voxel accumulator, full-spectrum quadrature, and bounding helpers
//! for local-frame EMF display projection.

use crate::q42::emf_profiles::{self as profiles, FramePose};
use crate::q42::field_envelope::StructuralView;
use crate::q42::field_spectrum::{
    AxisKind, AxisView, SampleSemantics, VACUUM_SPEED_OF_LIGHT_M_PER_S,
};
use crate::render::portal_emf_state::{
    carrier_hz_to_spectral_sigma, e_magnitude_from_poynting, DisplayOpticalCell, MAX_DISPLAY_CELLS,
};
use crate::render::portal_load_emf::{
    charge, vector, DisplayEmfCell, DisplaySelection, Error, LoadedEmf, PortalEmfLimits,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) fn selected_frame_pose(view: &StructuralView<'_>, frame_id: u32) -> FramePose {
    view.blocks()
        .find(|b| b.id() == frame_id && b.schema_type() == 2)
        .and_then(|b| FramePose::from_block(b).ok())
        .unwrap_or_else(FramePose::identity)
}

pub(super) fn grid_bounds(grid: profiles::GridLayout) -> Result<[f32; 6], Error> {
    let spec = grid.spec();
    let mut bounds = [0f32; 6];
    for i in 0..3 {
        let low = -0.5 * spec.spacing_m[i];
        let high = (f64::from(spec.dimensions[i]) - 0.5) * spec.spacing_m[i];
        let (a, b) = (low as f32, high as f32);
        if !a.is_finite() || !b.is_finite() || a >= b {
            return Err(Error::UnsupportedProjection("unsupported-display-range"));
        }
        bounds[i * 2] = a;
        bounds[i * 2 + 1] = b;
    }
    Ok(bounds)
}

fn coord_hz(axis: AxisView<'_>, idx: u64) -> Result<f64, Error> {
    let c = axis
        .coordinate(idx)
        .map_err(|e| Error::Profile(profiles::Error::Spectrum(e)))?;
    if !c.is_finite() || c <= 0.0 {
        return Err(Error::Profile(profiles::Error::Value));
    }
    Ok(match axis.kind() {
        AxisKind::FrequencyHz => c,
        AxisKind::VacuumWavelengthMetres => VACUUM_SPEED_OF_LIGHT_M_PER_S / c,
    })
}

/// Integrate physical spectrum across RF/THz/IR/Visible/UV/X-ray/Gamma with no visible cutoff.
pub(super) fn integrate_spectrum(
    axis: AxisView<'_>,
    mut vals: impl Iterator<Item = f64>,
) -> Result<(f64, f64, f64), Error> {
    let n = axis.sample_count();
    let mut total = 0.0_f64;
    let mut log_f_w = 0.0_f64;
    let mut first_hz = 0.0_f64;
    match axis.semantics() {
        SampleSemantics::BinIntegral | SampleSemantics::BinAverage => {
            for i in 0..n {
                let v = vals.next().ok_or(Error::Profile(profiles::Error::Shape))?;
                if !v.is_finite() || v < 0.0 {
                    return Err(Error::Profile(profiles::Error::Value));
                }
                let x0 = axis
                    .coordinate(i)
                    .map_err(|e| Error::Profile(profiles::Error::Spectrum(e)))?;
                let x1 = axis
                    .coordinate(i + 1)
                    .map_err(|e| Error::Profile(profiles::Error::Spectrum(e)))?;
                let f_mid = (coord_hz(axis, i)? * coord_hz(axis, i + 1)?).sqrt();
                if i == 0 {
                    first_hz = f_mid;
                }
                let e = if axis.semantics() == SampleSemantics::BinAverage {
                    v * (x1 - x0).abs()
                } else {
                    v
                };
                total += e;
                log_f_w += e * f_mid.ln();
            }
        }
        SampleSemantics::PointDensity => {
            if n == 1 {
                let v = vals.next().ok_or(Error::Profile(profiles::Error::Shape))?;
                if !v.is_finite() || v < 0.0 {
                    return Err(Error::Profile(profiles::Error::Value));
                }
                first_hz = coord_hz(axis, 0)?;
                total = v;
                log_f_w = v * first_hz.ln();
            } else {
                let mut prev_v = vals.next().ok_or(Error::Profile(profiles::Error::Shape))?;
                let mut prev_x = axis
                    .coordinate(0)
                    .map_err(|e| Error::Profile(profiles::Error::Spectrum(e)))?;
                let mut prev_f = coord_hz(axis, 0)?;
                first_hz = prev_f;
                if !prev_v.is_finite() || prev_v < 0.0 {
                    return Err(Error::Profile(profiles::Error::Value));
                }
                for i in 1..n {
                    let v = vals.next().ok_or(Error::Profile(profiles::Error::Shape))?;
                    if !v.is_finite() || v < 0.0 {
                        return Err(Error::Profile(profiles::Error::Value));
                    }
                    let x = axis
                        .coordinate(i)
                        .map_err(|e| Error::Profile(profiles::Error::Spectrum(e)))?;
                    let f = coord_hz(axis, i)?;
                    let e = 0.5 * (prev_v + v) * (x - prev_x).abs();
                    total += e;
                    log_f_w += e * (prev_f * f).sqrt().ln();
                    prev_v = v;
                    prev_x = x;
                    prev_f = f;
                }
            }
        }
    }
    let carrier_hz = if total > 0.0 && total.is_finite() {
        (log_f_w / total).exp()
    } else {
        first_hz
    };
    Ok((total, carrier_hz, carrier_hz_to_spectral_sigma(carrier_hz)))
}

#[derive(Clone, Copy, Default)]
pub(super) struct VoxelAccum {
    pub count: u32,
    has_carrier: bool,
    e_sq_sum: f64,
    i_sum: f64,
    q_sum: f64,
    u_sum: f64,
    v_sum: f64,
    sigma_w: f64,
    carrier_ln_w: f64,
    carrier_w: f64,
    carrier_count: u32,
    single_carrier_hz: f64,
    single_sigma: f64,
    cos_w: f64,
    sin_w: f64,
    coh_sum: f64,
}

impl VoxelAccum {
    pub fn deposit(
        &mut self,
        e_mag: f64,
        stokes: [f64; 4],
        carrier_hz: f64,
        sigma: f64,
        phase_rad: f64,
        coherence: f64,
    ) {
        self.count = self.count.saturating_add(1);
        self.e_sq_sum += e_mag * e_mag;
        self.i_sum += stokes[0];
        self.q_sum += stokes[1];
        self.u_sum += stokes[2];
        self.v_sum += stokes[3];
        let w = if stokes[0].is_finite() && stokes[0] > 0.0 {
            stokes[0]
        } else {
            1.0
        };
        if sigma > 0.0 || carrier_hz > 0.0 {
            self.has_carrier = true;
            self.sigma_w += sigma * w;
            self.carrier_w += w;
            self.single_sigma = sigma;
            if carrier_hz > 0.0 {
                self.carrier_count = self.carrier_count.saturating_add(1);
                self.single_carrier_hz = carrier_hz;
                self.carrier_ln_w += carrier_hz.ln() * w;
            }
        }
        self.cos_w += phase_rad.cos() * w;
        self.sin_w += phase_rad.sin() * w;
        self.coh_sum += coherence;
    }

    pub fn finish(self) -> (DisplayEmfCell, DisplayOpticalCell) {
        if self.count == 0 {
            return (DisplayEmfCell::default(), DisplayOpticalCell::default());
        }
        let n = f64::from(self.count);
        let e_mag = (self.e_sq_sum / n).sqrt();
        let i = self.i_sum / n;
        let (mut q, mut u, mut v) = (self.q_sum / n, self.u_sum / n, self.v_sum / n);
        let pol = q.hypot(u).hypot(v);
        if i.is_finite() && pol > i && pol > 0.0 {
            let s = i / pol;
            q *= s;
            u *= s;
            v *= s;
        } else if !i.is_finite() {
            q = 0.0;
            u = 0.0;
            v = 0.0;
        }
        let sigma = if self.has_carrier && self.carrier_w > 0.0 {
            if self.carrier_count <= 1 {
                self.single_sigma.clamp(0.0, 5.999_999)
            } else {
                (self.sigma_w / self.carrier_w).clamp(0.0, 5.999_999)
            }
        } else {
            0.0
        };
        let carrier_hz = if self.has_carrier && self.carrier_w > 0.0 {
            if self.carrier_count == 1 {
                self.single_carrier_hz
            } else {
                (self.carrier_ln_w / self.carrier_w).exp()
            }
        } else {
            0.0
        };
        let phase_rad = if self.cos_w == 0.0 && self.sin_w == 0.0 {
            0.0
        } else {
            self.sin_w.atan2(self.cos_w)
        };
        (
            DisplayEmfCell { magnitude_v_per_m: e_mag, valid: true },
            DisplayOpticalCell {
                stokes_i: i,
                stokes_q: q,
                stokes_u: u,
                stokes_v: v,
                spectral_sigma: sigma,
                carrier_hz,
                phase_rad,
                coherence: (self.coh_sum / n).clamp(0.0, 1.0),
                valid: true,
            },
        )
    }
}

pub(super) struct Extent {
    min: [f64; 3],
    max: [f64; 3],
    any: bool,
    pub max_x_slot: u32,
}

impl Default for Extent {
    fn default() -> Self {
        Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
            any: false,
            max_x_slot: 0,
        }
    }
}

impl Extent {
    pub fn push(&mut self, p: [f64; 3]) {
        self.any = true;
        for i in 0..3 {
            self.min[i] = self.min[i].min(p[i]);
            self.max[i] = self.max[i].max(p[i]);
        }
    }

    pub fn bounds_and_dims(
        &self,
        nt: usize,
        is_iq: bool,
        max_cells: usize,
    ) -> Result<([f32; 6], [u32; 3]), Error> {
        if is_iq {
            let nx = self.max_x_slot.saturating_add(1).clamp(1, 256);
            if (nx as usize).checked_mul(nt).is_none_or(|c| c > max_cells) {
                return Err(Error::Limit);
            }
            return Ok(([0.0, nx as f32, -0.5, 0.5, -0.5, 0.5], [nx, 1, 1]));
        }
        if !self.any {
            return Ok(([-0.5, 0.5, -0.5, 0.5, -0.5, 0.5], [1, 1, 1]));
        }
        let mut bounds = [0.0f32; 6];
        let mut active = [false; 3];
        for i in 0..3 {
            let (mut lo, mut hi) = (self.min[i], self.max[i]);
            if (hi - lo).abs() <= 1.0e-12 {
                lo -= 0.5;
                hi += 0.5;
            } else {
                active[i] = true;
            }
            let (a, b) = (lo as f32, hi as f32);
            if !a.is_finite() || !b.is_finite() || a >= b {
                return Err(Error::UnsupportedProjection("unsupported-display-range"));
            }
            bounds[i * 2] = a;
            bounds[i * 2 + 1] = b;
        }
        let budget = max_cells / nt.max(1);
        if budget == 0 {
            return Err(Error::Limit);
        }
        let n_active = active.iter().filter(|&&a| a).count().max(1);
        let side = match n_active {
            1 => budget.min(64) as u32,
            2 => ((budget as f64).sqrt().floor() as u32).clamp(2, 32),
            _ => ((budget as f64).cbrt().floor() as u32).clamp(2, 16),
        };
        Ok((
            bounds,
            [
                if active[0] { side } else { 1 },
                if active[1] { side } else { 1 },
                if active[2] { side } else { 1 },
            ],
        ))
    }
}

pub(super) fn voxel_of(p: [f64; 3], bounds: [f32; 6], dims: [u32; 3]) -> [usize; 3] {
    let mut out = [0usize; 3];
    for i in 0..3 {
        let lo = f64::from(bounds[i * 2]);
        let hi = f64::from(bounds[i * 2 + 1]);
        let t = ((p[i] - lo) / (hi - lo)).clamp(0.0, 0.999_999_9);
        out[i] = ((t * f64::from(dims[i])).floor() as usize).min(dims[i] as usize - 1);
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub(super) fn deposit_ray(
    acc: &mut [VoxelAccum],
    dims: [u32; 3],
    bounds: [f32; 6],
    ti: usize,
    pos: [f64; 3],
    dir: [f64; 3],
    path_m: f64,
    stokes: [f64; 4],
    carrier_hz: f64,
    phase_rad: f64,
    work: &mut u64,
) -> Result<(), Error> {
    let spatial = dims[0] as usize * dims[1] as usize * dims[2] as usize;
    let len = path_m.min(1.0e4);
    let steps = if len <= 0.0 {
        1
    } else {
        (dims[0] + dims[1] + dims[2] + 2).min(128)
    };
    let e_mag = e_magnitude_from_poynting(stokes[0]);
    let sigma = carrier_hz_to_spectral_sigma(carrier_hz);
    let mut prev = [usize::MAX; 3];
    for s in 0..steps {
        charge(work, 1)?;
        let frac = if steps == 1 { 0.0 } else { f64::from(s) / f64::from(steps - 1) };
        let p = [
            pos[0] + dir[0] * len * frac,
            pos[1] + dir[1] * len * frac,
            pos[2] + dir[2] * len * frac,
        ];
        let v = voxel_of(p, bounds, dims);
        if v != prev {
            prev = v;
            let idx = (v[2] * dims[1] as usize + v[1]) * dims[0] as usize + v[0] + ti * spatial;
            acc[idx].deposit(e_mag, stokes, carrier_hz, sigma, phase_rad, 1.0);
        }
    }
    Ok(())
}

pub(super) fn check_peak_memory(
    source_len: usize,
    count: usize,
    per_cell_extra: usize,
    limit: usize,
) -> Result<(), Error> {
    let per_cell = std::mem::size_of::<DisplayEmfCell>()
        + std::mem::size_of::<DisplayOpticalCell>()
        + per_cell_extra;
    let total = source_len
        .checked_mul(2)
        .and_then(|s| count.checked_mul(per_cell).and_then(|c| s.checked_add(c)))
        .ok_or(Error::Overflow)?;
    if total > limit {
        return Err(Error::Limit);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finalize_loaded(
    bytes: &[u8],
    dims: [u32; 3],
    bounds: [f32; 6],
    frame_pose: FramePose,
    ticks: Vec<i64>,
    temporal: bool,
    selected: DisplaySelection,
    display_mode: u16,
    acc: Vec<VoxelAccum>,
    limits: PortalEmfLimits,
) -> Result<LoadedEmf, Error> {
    let count = acc.len();
    if count == 0 || count > limits.display_cells.min(MAX_DISPLAY_CELLS) {
        return Err(Error::Limit);
    }
    let mut cells = vector(count, limits.peak_owned_bytes)?;
    let mut optical_cells = vector(count, limits.peak_owned_bytes)?;
    for a in acc {
        let (c, o) = a.finish();
        cells.push(c);
        optical_cells.push(o);
    }
    let mut source = vector(bytes.len(), limits.peak_owned_bytes)?;
    source.extend_from_slice(bytes);
    let source_sha256: [u8; 32] = Sha256::digest(bytes).into();
    Ok(LoadedEmf {
        nx: dims[0],
        ny: dims[1],
        nz: dims[2],
        nt: u32::try_from(ticks.len()).map_err(|_| Error::Overflow)?,
        cell_count: u32::try_from(count).map_err(|_| Error::Overflow)?,
        bounds,
        frame_pose,
        source_sha256,
        cells,
        optical_cells,
        display_mode,
        source: Arc::from(source.into_boxed_slice()),
        selection: selected,
        ticks,
        temporal,
    })
}
