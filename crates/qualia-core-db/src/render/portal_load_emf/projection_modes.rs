//! Bounded local-frame display projections for non-grid Profile modes (1, 3)
//! and frozen EMF v4 wire modes (0x0100..=0x0107 / 0..=7).

use super::super::admission::extended_records::*;
use super::super::admission::{ExtendedChunk, ExtendedKind};
use super::projection_accum::*;
use crate::q42::{
    emf_profiles as profiles,
    field_envelope::StructuralView,
    field_spectrum::photon_energy_j_to_frequency_hz,
};
use crate::render::portal_emf_state::{
    carrier_hz_to_spectral_sigma, e_magnitude_from_poynting, poynting_irradiance_w_per_m2,
    DisplayOpticalCell, MAX_DISPLAY_CELLS, VACUUM_IMPEDANCE_OHM,
};
use crate::render::portal_load_emf::{
    charge, vector, DisplayEmfCell, DisplaySelection, Error, LoadedEmf, PortalEmfLimits,
};
use profiles::ProfileChunk;
use sha2::{Digest, Sha256};
use std::sync::Arc;

fn ext_key(c: &ExtendedChunk<'_>) -> DisplaySelection {
    DisplaySelection {
        descriptor_id: c.descriptor_id,
        frame_id: c.frame_id,
        clock_id: c.clock_id,
        stream_id: c.stream_id,
    }
}

fn ext_tick(c: &ExtendedChunk<'_>, index: u64) -> Result<i64, Error> {
    match (c.start_tick, c.temporal_step_ticks) {
        (Some(start), Some(step)) => i64::try_from(
            i128::from(start)
                .checked_add(i128::from(index).checked_mul(step.into()).ok_or(Error::Overflow)?)
                .ok_or(Error::Overflow)?,
        )
        .map_err(|_| Error::Overflow),
        (None, None) if index == 0 => Ok(0),
        _ => Err(Error::UnsupportedProjection("mixed static/timed projection")),
    }
}

fn record_time_index(c: &ExtendedChunk<'_>, ri: u64) -> u64 {
    if c.start_tick.is_some() && c.time_count > 1 && c.record_count >= c.time_count {
        let per = (c.record_count / c.time_count).max(1);
        (ri / per).min(c.time_count - 1)
    } else {
        0
    }
}

pub(super) fn project_profile_non_grid(
    bytes: &[u8],
    view: &StructuralView<'_>,
    chunks: &[ProfileChunk<'_>],
    selection: Option<DisplaySelection>,
    limits: PortalEmfLimits,
    work: &mut u64,
) -> Result<LoadedEmf, Error> {
    let mut chosen = selection;
    let mut temporal = None;
    let mut extent = Extent::default();
    let mut ticks = Vec::new();
    for &c in chunks {
        charge(work, 1)?;
        let group = DisplaySelection {
            descriptor_id: c.descriptor_id(),
            frame_id: c.frame_id(),
            clock_id: c.clock_id(),
            stream_id: view.entry(c.entry_index())?.stream_id(),
        };
        if selection.is_some_and(|s| s != group) {
            continue;
        }
        if chosen.is_some_and(|s| s != group) {
            return Err(Error::AmbiguousSelection);
        }
        chosen = Some(group);
        let timed = c.start_tick().is_some();
        if temporal.is_some_and(|old| old != timed) {
            return Err(Error::UnsupportedProjection("mixed static/timed projection"));
        }
        temporal = Some(timed);
        match c.mode() {
            1 => {
                ticks.push(c.start_tick().unwrap_or(0));
                for ri in 0..c.record_count() {
                    if let Some(r) = c.ray(ri)? {
                        extent.push(r.position_m());
                        let len = r.geometric_path_m().min(1.0e4);
                        if len > 0.0 {
                            let (p, d) = (r.position_m(), r.direction());
                            extent.push([p[0] + d[0] * len, p[1] + d[1] * len, p[2] + d[2] * len]);
                        }
                    }
                }
            }
            3 => {
                for ri in 0..c.record_count() {
                    if let Some(r) = c.spectrum(ri)? {
                        extent.push(r.position_m());
                        ticks.push(r.tick());
                    }
                }
            }
            _ => return Err(Error::UnsupportedProjection("unsupported-profile-mode")),
        }
    }
    let selected = chosen.ok_or(Error::SelectionMissing)?;
    if ticks.is_empty() {
        ticks.push(0);
    }
    ticks.sort_unstable();
    ticks.dedup();
    if ticks.len() > limits.time_slices {
        return Err(Error::Limit);
    }
    let max_cells = limits.display_cells.min(MAX_DISPLAY_CELLS);
    let (bounds, dims) = extent.bounds_and_dims(ticks.len(), false, max_cells)?;
    let spatial = dims[0] as usize * dims[1] as usize * dims[2] as usize;
    let count = spatial.checked_mul(ticks.len()).ok_or(Error::Overflow)?;
    if count == 0 || count > max_cells {
        return Err(Error::Limit);
    }
    check_peak_memory(bytes.len(), count, std::mem::size_of::<VoxelAccum>(), limits.peak_owned_bytes)?;
    let mut acc = vector::<VoxelAccum>(count, limits.peak_owned_bytes)?;
    acc.resize(count, VoxelAccum::default());
    let mut mode_out = 1u16;
    for &c in chunks {
        let group = DisplaySelection {
            descriptor_id: c.descriptor_id(),
            frame_id: c.frame_id(),
            clock_id: c.clock_id(),
            stream_id: view.entry(c.entry_index())?.stream_id(),
        };
        if group != selected {
            continue;
        }
        mode_out = c.mode();
        match c.mode() {
            1 => {
                let ti = ticks.binary_search(&c.start_tick().unwrap_or(0)).map_err(|_| Error::SelectionMissing)?;
                for ri in 0..c.record_count() {
                    if let Some(r) = c.ray(ri)? {
                        deposit_ray(&mut acc, dims, bounds, ti, r.position_m(), r.direction(), r.geometric_path_m(), r.stokes(), r.carrier_hz(), 0.0, work)?;
                    }
                }
            }
            3 => {
                let axis = c.spectral_axis().ok_or(Error::Profile(profiles::Error::Descriptor))?;
                for ri in 0..c.record_count() {
                    if let Some(r) = c.spectrum(ri)? {
                        let ti = ticks.binary_search(&r.tick()).map_err(|_| Error::SelectionMissing)?;
                        let (total, hz, sigma) = integrate_spectrum(axis, r.values())?;
                        let v = voxel_of(r.position_m(), bounds, dims);
                        let idx = (v[2] * dims[1] as usize + v[1]) * dims[0] as usize + v[0] + ti * spatial;
                        acc[idx].deposit(e_magnitude_from_poynting(total), [total, 0.0, 0.0, 0.0], hz, sigma, 0.0, 1.0);
                    }
                }
            }
            _ => return Err(Error::UnsupportedProjection("unsupported-profile-mode")),
        }
    }
    let frame_pose = selected_frame_pose(view, selected.frame_id);
    finalize_loaded(bytes, dims, bounds, frame_pose, ticks, temporal.unwrap_or(false), selected, mode_out, acc, limits)
}

pub(super) fn project_extended(
    bytes: &[u8],
    view: &StructuralView<'_>,
    chunks: &[ExtendedChunk<'_>],
    selection: Option<DisplaySelection>,
    limits: PortalEmfLimits,
    work: &mut u64,
) -> Result<LoadedEmf, Error> {
    let mut chosen = selection;
    let mut mode_sel = None;
    let mut grid = None;
    let mut temporal = None;
    let mut extent = Extent::default();
    let mut ticks = Vec::new();
    for c in chunks {
        charge(work, 1)?;
        if matches!(c.kind, ExtendedKind::Auxiliary) {
            continue;
        }
        let group = ext_key(c);
        if selection.is_some_and(|s| s != group) {
            continue;
        }
        if chosen.is_some_and(|s| s != group) || mode_sel.is_some_and(|m| m != c.canonical_mode) {
            return Err(Error::AmbiguousSelection);
        }
        chosen = Some(group);
        mode_sel = Some(c.canonical_mode);
        let timed = c.start_tick.is_some();
        if temporal.is_some_and(|old| old != timed) {
            return Err(Error::UnsupportedProjection("mixed static/timed projection"));
        }
        temporal = Some(timed);
        match &c.kind {
            ExtendedKind::Field0(Some(b)) | ExtendedKind::Yee2(Some(b)) => {
                let cur = b.grid_layout();
                if grid.is_some_and(|g| g != cur) {
                    return Err(Error::AmbiguousSelection);
                }
                grid = Some(cur);
                let n = if timed { c.time_count } else { 1 };
                for i in 0..n {
                    ticks.push(ext_tick(c, i)?);
                }
            }
            ExtendedKind::Yee2(None) => return Err(Error::Profile(profiles::Error::Frame)),
            _ => scan_ungridded_chunk(c, &mut extent, &mut ticks, work)?,
        }
    }
    let selected = chosen.ok_or(Error::SelectionMissing)?;
    let canonical_mode = mode_sel.ok_or(Error::SelectionMissing)?;
    if ticks.is_empty() {
        ticks.push(0);
    }
    ticks.sort_unstable();
    ticks.dedup();
    if ticks.len() > limits.time_slices {
        return Err(Error::Limit);
    }
    let frame_pose = selected_frame_pose(view, selected.frame_id);
    if let Some(gl) = grid {
        return project_extended_grid(bytes, chunks, selected, canonical_mode, gl, frame_pose, ticks, temporal.unwrap_or(false), limits, work);
    }
    let max_cells = limits.display_cells.min(MAX_DISPLAY_CELLS);
    let (bounds, dims) = extent.bounds_and_dims(ticks.len(), canonical_mode == 0x0105, max_cells)?;
    let spatial = dims[0] as usize * dims[1] as usize * dims[2] as usize;
    let count = spatial.checked_mul(ticks.len()).ok_or(Error::Overflow)?;
    if count == 0 || count > max_cells {
        return Err(Error::Limit);
    }
    check_peak_memory(bytes.len(), count, std::mem::size_of::<VoxelAccum>(), limits.peak_owned_bytes)?;
    let mut acc = vector::<VoxelAccum>(count, limits.peak_owned_bytes)?;
    acc.resize(count, VoxelAccum::default());
    for c in chunks {
        if matches!(c.kind, ExtendedKind::Auxiliary) || ext_key(c) != selected {
            continue;
        }
        deposit_ungridded_chunk(c, dims, bounds, &ticks, &mut acc, work)?;
    }
    finalize_loaded(bytes, dims, bounds, frame_pose, ticks, temporal.unwrap_or(false), selected, canonical_mode, acc, limits)
}

fn scan_ungridded_chunk(c: &ExtendedChunk<'_>, extent: &mut Extent, ticks: &mut Vec<i64>, work: &mut u64) -> Result<(), Error> {
    for ri in 0..c.record_count {
        charge(work, 1)?;
        let valid = c.is_valid(ri);
        let slice = c.record_slice(ri)?;
        match &c.kind {
            ExtendedKind::Field0(None) => if let Some(r) = parse_mode0_field(slice, valid)? {
                extent.push(r.position_m);
                ticks.push(r.tick);
            },
            ExtendedKind::Ray1 => {
                ticks.push(ext_tick(c, record_time_index(c, ri))?);
                if let Some(r) = parse_mode1_ray(slice, valid)? {
                    extent.push(r.position_m);
                    let len = r.path_m.min(1.0e4);
                    if len > 0.0 {
                        extent.push([r.position_m[0] + r.direction[0] * len, r.position_m[1] + r.direction[1] * len, r.position_m[2] + r.direction[2] * len]);
                    }
                }
            }
            ExtendedKind::Spectrum3(_, bins, aid) => if let Some(r) = parse_mode3_spectrum(slice, valid, *bins, *aid)? {
                extent.push(r.position_m);
                ticks.push(r.tick);
            },
            ExtendedKind::Mueller4 => {
                ticks.push(ext_tick(c, record_time_index(c, ri))?);
                if let Some(r) = parse_mode4_mueller(slice, valid)? {
                    extent.push(r.position_m);
                }
            }
            ExtendedKind::Iq5 { .. } => if let Some(r) = parse_mode5_iq(slice, valid)? {
                extent.max_x_slot = extent.max_x_slot.max(r.channel_index);
                ticks.push(r.first_tick);
            },
            ExtendedKind::Ionizing6 { .. } => if let Some(r) = parse_mode6_ionizing(slice, valid)? {
                extent.push(r.position_m);
                ticks.push(r.tick);
            },
            ExtendedKind::Bio7 { .. } => if let Some(r) = parse_mode7_bio(slice, valid)? {
                extent.push(r.position_m);
                ticks.push(r.tick);
            },
            _ => {}
        }
    }
    Ok(())
}

fn deposit_ungridded_chunk(
    c: &ExtendedChunk<'_>,
    dims: [u32; 3],
    bounds: [f32; 6],
    ticks: &[i64],
    acc: &mut [VoxelAccum],
    work: &mut u64,
) -> Result<(), Error> {
    let spatial = dims[0] as usize * dims[1] as usize * dims[2] as usize;
    for ri in 0..c.record_count {
        charge(work, 1)?;
        let valid = c.is_valid(ri);
        let slice = c.record_slice(ri)?;
        match &c.kind {
            ExtendedKind::Field0(None) => if let Some(r) = parse_mode0_field(slice, valid)? {
                let ti = ticks.binary_search(&r.tick).map_err(|_| Error::SelectionMissing)?;
                let v = voxel_of(r.position_m, bounds, dims);
                let idx = (v[2] * dims[1] as usize + v[1]) * dims[0] as usize + v[0] + ti * spatial;
                let e_mag = r.electric_v_per_m[0].hypot(r.electric_v_per_m[1]).hypot(r.electric_v_per_m[2]);
                let i = poynting_irradiance_w_per_m2(r.electric_v_per_m, Some(r.magnetic_tesla), r.stored_poynting);
                let hz = r.carrier_hz.unwrap_or(0.0);
                acc[idx].deposit(e_mag, [i, 0.0, 0.0, 0.0], hz, carrier_hz_to_spectral_sigma(hz), r.phase_rad.unwrap_or(0.0), r.confidence.map_or(1.0, f64::from));
            },
            ExtendedKind::Ray1 => if let Some(r) = parse_mode1_ray(slice, valid)? {
                let tick_v = ext_tick(c, record_time_index(c, ri))?;
                let ti = ticks.binary_search(&tick_v).map_err(|_| Error::SelectionMissing)?;
                deposit_ray(acc, dims, bounds, ti, r.position_m, r.direction, r.path_m, r.stokes, r.carrier_hz, r.phase_rad.unwrap_or(0.0), work)?;
            },
            ExtendedKind::Spectrum3(axis, bins, aid) => if let Some(r) = parse_mode3_spectrum(slice, valid, *bins, *aid)? {
                let ti = ticks.binary_search(&r.tick).map_err(|_| Error::SelectionMissing)?;
                let vals = r.values.chunks_exact(8).map(|ch| f64::from_le_bytes(ch.try_into().unwrap()));
                let (total, hz, sigma) = integrate_spectrum(*axis, vals)?;
                let v = voxel_of(r.position_m, bounds, dims);
                let idx = (v[2] * dims[1] as usize + v[1]) * dims[0] as usize + v[0] + ti * spatial;
                acc[idx].deposit(e_magnitude_from_poynting(total), [total, 0.0, 0.0, 0.0], hz, sigma, 0.0, 1.0);
            },
            ExtendedKind::Mueller4 => if let Some(r) = parse_mode4_mueller(slice, valid)? {
                let tick_v = ext_tick(c, record_time_index(c, ri))?;
                let ti = ticks.binary_search(&tick_v).map_err(|_| Error::SelectionMissing)?;
                let v = voxel_of(r.position_m, bounds, dims);
                let idx = (v[2] * dims[1] as usize + v[1]) * dims[0] as usize + v[0] + ti * spatial;
                acc[idx].deposit(e_magnitude_from_poynting(r.m00), [r.m00, r.m10, r.m20, r.m30], r.frequency_hz, carrier_hz_to_spectral_sigma(r.frequency_hz), 0.0, 1.0);
            },
            ExtendedKind::Iq5 { calib_scale, calib_offset, fallback_carrier_hz } => if let Some(r) = parse_mode5_iq(slice, valid)? {
                let ti = ticks.binary_search(&r.first_tick).map_err(|_| Error::SelectionMissing)?;
                let e_mag = (r.rms_v * calib_scale + calib_offset).abs();
                let hz = if r.centre_hz > 0.0 { r.centre_hz } else { *fallback_carrier_hz };
                let i = (e_mag / VACUUM_IMPEDANCE_OHM.sqrt()).powi(2);
                let x = (r.channel_index as usize).min(dims[0] as usize - 1);
                acc[x + ti * spatial].deposit(e_mag, [i, 0.0, 0.0, 0.0], hz, carrier_hz_to_spectral_sigma(hz), r.mean_phase_rad, 1.0);
            },
            ExtendedKind::Ionizing6 { calib_scale } => if let Some(r) = parse_mode6_ionizing(slice, valid)? {
                let ti = ticks.binary_search(&r.tick).map_err(|_| Error::SelectionMissing)?;
                let v = voxel_of(r.position_m, bounds, dims);
                let idx = (v[2] * dims[1] as usize + v[1]) * dims[0] as usize + v[0] + ti * spatial;
                let hz = if r.photon_energy_j > 0.0 {
                    photon_energy_j_to_frequency_hz(r.photon_energy_j)
                        .map_err(|e| Error::Profile(profiles::Error::Spectrum(e)))?
                } else {
                    0.0
                };
                let i = (if r.photon_fluence_per_m2 > 0.0 && r.photon_energy_j > 0.0 { r.photon_fluence_per_m2 * r.photon_energy_j } else { r.absorbed_dose_gy }) * calib_scale;
                acc[idx].deposit(e_magnitude_from_poynting(i), [i, 0.0, 0.0, 0.0], hz, carrier_hz_to_spectral_sigma(hz), 0.0, 1.0);
            },
            ExtendedKind::Bio7 { calib_scale } => if let Some(r) = parse_mode7_bio(slice, valid)? {
                let ti = ticks.binary_search(&r.tick).map_err(|_| Error::SelectionMissing)?;
                let v = voxel_of(r.position_m, bounds, dims);
                let idx = (v[2] * dims[1] as usize + v[1]) * dims[0] as usize + v[0] + ti * spatial;
                let d_mag = r.dipole_a_m[0].hypot(r.dipole_a_m[1]).hypot(r.dipole_a_m[2]);
                let e_mag = (if r.potential_v != 0.0 { r.potential_v.abs() } else { d_mag }) * calib_scale;
                let i = poynting_irradiance_w_per_m2([e_mag, 0.0, 0.0], None, None);
                acc[idx].deposit(e_mag, [i, 0.0, 0.0, 0.0], r.carrier_hz, carrier_hz_to_spectral_sigma(r.carrier_hz), 0.0, 1.0);
            },
            _ => {}
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn project_extended_grid(
    bytes: &[u8],
    chunks: &[ExtendedChunk<'_>],
    selected: DisplaySelection,
    canonical_mode: u16,
    grid: profiles::GridLayout,
    frame_pose: profiles::FramePose,
    ticks: Vec<i64>,
    temporal: bool,
    limits: PortalEmfLimits,
    work: &mut u64,
) -> Result<LoadedEmf, Error> {
    let bounds = grid_bounds(grid)?;
    let dims = grid.spec().dimensions;
    let spatial = dims.into_iter().try_fold(1usize, |n, d| n.checked_mul(d as usize)).ok_or(Error::Overflow)?;
    let count = spatial.checked_mul(ticks.len()).ok_or(Error::Overflow)?;
    if count == 0 || count > limits.display_cells || count > MAX_DISPLAY_CELLS {
        return Err(Error::Limit);
    }
    check_peak_memory(bytes.len(), count, 1, limits.peak_owned_bytes)?;
    let mut cells = vector(count, limits.peak_owned_bytes)?;
    cells.resize(count, DisplayEmfCell::default());
    let mut optical_cells = vector(count, limits.peak_owned_bytes)?;
    optical_cells.resize(count, DisplayOpticalCell::default());
    let mut occupied = vector(count, limits.peak_owned_bytes)?;
    occupied.resize(count, 0u8);
    for c in chunks {
        if matches!(c.kind, ExtendedKind::Auxiliary) || ext_key(c) != selected {
            continue;
        }
        let brick = match &c.kind {
            ExtendedKind::Field0(Some(b)) | ExtendedKind::Yee2(Some(b)) => *b,
            _ => continue,
        };
        charge(work, c.record_count)?;
        for ri in 0..c.record_count {
            let local = ri % brick.cell_count();
            let ti = ticks.binary_search(&ext_tick(c, ri / brick.cell_count())?).map_err(|_| Error::SelectionMissing)?;
            let [x, y, z] = brick.global_cell(local)?;
            let cell = (z as usize * dims[1] as usize + y as usize) * dims[0] as usize + x as usize + ti * spatial;
            if occupied[cell] != 0 {
                return Err(Error::Overlap);
            }
            occupied[cell] = 1;
            let valid = c.is_valid(ri);
            let slice = c.record_slice(ri)?;
            match &c.kind {
                ExtendedKind::Field0(_) => if let Some(r) = parse_mode0_field(slice, valid)? {
                    let e_mag = r.electric_v_per_m[0].hypot(r.electric_v_per_m[1]).hypot(r.electric_v_per_m[2]);
                    let i = poynting_irradiance_w_per_m2(r.electric_v_per_m, Some(r.magnetic_tesla), r.stored_poynting);
                    let hz = r.carrier_hz.unwrap_or(0.0);
                    cells[cell] = DisplayEmfCell { magnitude_v_per_m: e_mag, valid: true };
                    optical_cells[cell] = DisplayOpticalCell {
                        stokes_i: i, stokes_q: 0.0, stokes_u: 0.0, stokes_v: 0.0,
                        spectral_sigma: carrier_hz_to_spectral_sigma(hz), carrier_hz: hz,
                        phase_rad: r.phase_rad.unwrap_or(0.0),
                        coherence: r.confidence.map_or(1.0, f64::from), valid: true,
                    };
                },
                ExtendedKind::Yee2(_) => if let Some(y) = parse_mode2_yee(slice, valid)? {
                    let (e, h) = (y.electric_v_per_m, y.magnetic_h_a_per_m);
                    let e_mag = e[0].hypot(e[1]).hypot(e[2]);
                    let s = [e[1] * h[2] - e[2] * h[1], e[2] * h[0] - e[0] * h[2], e[0] * h[1] - e[1] * h[0]].into_iter().fold(0.0_f64, |a, v| a.hypot(v));
                    let i = if s > 0.0 { s } else { let z = VACUUM_IMPEDANCE_OHM * (y.mu_r / y.eps_r).sqrt(); (e_mag / z.sqrt()).powi(2) };
                    cells[cell] = DisplayEmfCell { magnitude_v_per_m: e_mag, valid: true };
                    optical_cells[cell] = DisplayOpticalCell { stokes_i: i, coherence: 1.0, valid: true, ..Default::default() };
                },
                _ => {}
            }
        }
    }
    let mut source = vector(bytes.len(), limits.peak_owned_bytes)?;
    source.extend_from_slice(bytes);
    let source_sha256: [u8; 32] = Sha256::digest(bytes).into();
    Ok(LoadedEmf {
        nx: dims[0], ny: dims[1], nz: dims[2], nt: u32::try_from(ticks.len()).map_err(|_| Error::Overflow)?,
        cell_count: u32::try_from(count).map_err(|_| Error::Overflow)?, bounds, frame_pose, source_sha256,
        cells, optical_cells,
        display_mode: canonical_mode, source: Arc::from(source.into_boxed_slice()),
        selection: selected, ticks, temporal,
    })
}
