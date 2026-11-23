#[path = "projection_accum.rs"]
mod projection_accum;
#[path = "projection_modes.rs"]
mod projection_modes;

use super::admission::AdmittedEmf;
use super::{
    charge, profiles, vector, DisplayEmfCell, DisplaySelection, Error, LoadedEmf, PortalEmfLimits,
    StructuralView,
};
use crate::render::portal_emf_state::{
    carrier_hz_to_spectral_sigma, poynting_irradiance_w_per_m2, DisplayOpticalCell,
    MAX_DISPLAY_CELLS,
};
use profiles::ProfileChunk;
use projection_accum::{grid_bounds, selected_frame_pose};
use std::sync::Arc;

fn key(view: &StructuralView<'_>, c: ProfileChunk<'_>) -> Result<DisplaySelection, Error> {
    Ok(DisplaySelection {
        descriptor_id: c.descriptor_id(),
        frame_id: c.frame_id(),
        clock_id: c.clock_id(),
        stream_id: view.entry(c.entry_index())?.stream_id(),
    })
}

fn tick(c: ProfileChunk<'_>, index: u64) -> Result<i64, Error> {
    match (c.start_tick(), c.temporal_step_ticks()) {
        (Some(start), Some(step)) => i64::try_from(
            i128::from(start)
                .checked_add(
                    i128::from(index)
                        .checked_mul(step.into())
                        .ok_or(Error::Overflow)?,
                )
                .ok_or(Error::Overflow)?,
        )
        .map_err(|_| Error::Overflow),
        (None, None) if index == 0 => Ok(0),
        _ => Err(Error::UnsupportedProjection(
            "mixed static/timed projection",
        )),
    }
}

pub(super) fn project(
    bytes: &[u8],
    view: &StructuralView<'_>,
    admitted: &AdmittedEmf<'_>,
    selection: Option<DisplaySelection>,
    limits: PortalEmfLimits,
    work: &mut u64,
) -> Result<LoadedEmf, Error> {
    let chunks = match admitted {
        AdmittedEmf::Profile(chunks) => chunks.as_slice(),
        AdmittedEmf::Extended(ext) => {
            return projection_modes::project_extended(bytes, view, ext, selection, limits, work);
        }
    };
    let has_mode0_grid = chunks
        .iter()
        .any(|c| c.mode() == 0 && c.brick_layout().is_some());
    if !has_mode0_grid {
        if chunks.is_empty() {
            return Err(Error::UnsupportedProjection("mode-0 spatial grid required"));
        }
        return projection_modes::project_profile_non_grid(
            bytes, view, chunks, selection, limits, work,
        );
    }

    let mut chosen = selection;
    let mut grid = None;
    let mut temporal = None;
    let mut candidates = 0usize;
    for &c in chunks {
        charge(work, 1)?;
        if c.mode() != 0 || c.brick_layout().is_none() {
            continue;
        }
        let group = key(view, c)?;
        if selection.is_some_and(|s| s != group) {
            continue;
        }
        if let Some(s) = chosen {
            if s != group {
                return Err(Error::AmbiguousSelection);
            }
        } else {
            chosen = Some(group)
        }
        let current = c.brick_layout().unwrap().grid_layout();
        if grid.is_some_and(|g| g != current) {
            return Err(Error::AmbiguousSelection);
        }
        grid = Some(current);
        let timed = c.start_tick().is_some();
        if temporal.is_some_and(|old| old != timed) {
            return Err(Error::UnsupportedProjection(
                "mixed static/timed projection",
            ));
        }
        temporal = Some(timed);
        let count = if timed {
            view.entry(c.entry_index())?.time_count()
        } else {
            1
        };
        candidates = candidates
            .checked_add(usize::try_from(count).map_err(|_| Error::Overflow)?)
            .ok_or(Error::Overflow)?;
    }
    let selected = chosen.ok_or(Error::UnsupportedProjection("mode-0 spatial grid required"))?;
    let grid = grid.ok_or(Error::SelectionMissing)?;
    if candidates == 0 {
        return Err(Error::SelectionMissing);
    }
    let bounds = grid_bounds(grid)?;
    let sort_work = (candidates as u64)
        .checked_mul(u64::from(usize::BITS - candidates.leading_zeros()) + 1)
        .ok_or(Error::Overflow)?;
    charge(work, sort_work)?;
    let early = (view.entry_count() as usize)
        .checked_mul(std::mem::size_of::<ProfileChunk<'_>>())
        .and_then(|n| candidates.checked_mul(8).and_then(|t| n.checked_add(t)))
        .ok_or(Error::Overflow)?;
    if early > limits.peak_owned_bytes {
        return Err(Error::Limit);
    }
    let mut ticks = vector(candidates, limits.peak_owned_bytes)?;
    for &c in chunks {
        charge(work, 1)?;
        if c.mode() != 0 || c.brick_layout().is_none() || key(view, c)? != selected {
            continue;
        }
        let n = if c.start_tick().is_some() {
            view.entry(c.entry_index())?.time_count()
        } else {
            1
        };
        for i in 0..n {
            charge(work, 1)?;
            ticks.push(tick(c, i)?);
        }
    }
    ticks.sort_unstable();
    ticks.dedup();
    if ticks.is_empty() || ticks.len() > limits.time_slices {
        return Err(Error::Limit);
    }
    let dims = grid.spec().dimensions;
    let spatial = dims
        .into_iter()
        .try_fold(1usize, |n, d| n.checked_mul(d as usize))
        .ok_or(Error::Overflow)?;
    let count = spatial.checked_mul(ticks.len()).ok_or(Error::Overflow)?;
    if count == 0 || count > limits.display_cells || count > MAX_DISPLAY_CELLS {
        return Err(Error::Limit);
    }
    let scratch = (view.entry_count() as usize)
        .checked_mul(std::mem::size_of::<ProfileChunk<'_>>())
        .and_then(|n| ticks.capacity().checked_mul(8).and_then(|t| n.checked_add(t)))
        .ok_or(Error::Overflow)?;
    let per_cell =
        std::mem::size_of::<DisplayEmfCell>() + std::mem::size_of::<DisplayOpticalCell>() + 1;
    let memory = bytes
        .len()
        .checked_mul(2)
        .and_then(|n| count.checked_mul(per_cell).and_then(|c| n.checked_add(c)))
        .and_then(|n| n.checked_add(scratch))
        .ok_or(Error::Overflow)?;
    if memory > limits.peak_owned_bytes {
        return Err(Error::Limit);
    }
    charge(work, count as u64)?;
    let mut cells = vector(count, limits.peak_owned_bytes)?;
    cells.resize(count, DisplayEmfCell::default());
    let mut optical_cells = vector(count, limits.peak_owned_bytes)?;
    optical_cells.resize(count, DisplayOpticalCell::default());
    let mut occupied = vector(count, limits.peak_owned_bytes)?;
    occupied.resize(count, 0u8);
    let lookup = u64::from(usize::BITS - ticks.len().leading_zeros()) + 2;
    for &c in chunks {
        charge(work, 1)?;
        if c.mode() != 0 || c.brick_layout().is_none() || key(view, c)? != selected {
            continue;
        }
        let brick = c.brick_layout().unwrap();
        charge(
            work,
            c.record_count()
                .checked_mul(lookup)
                .ok_or(Error::Overflow)?,
        )?;
        for record_index in 0..c.record_count() {
            let local = record_index % brick.cell_count();
            let t = tick(c, record_index / brick.cell_count())?;
            let ti = ticks.binary_search(&t).map_err(|_| Error::SelectionMissing)?;
            let [x, y, z] = brick.global_cell(local)?;
            let cell = ((z as usize * dims[1] as usize + y as usize) * dims[0] as usize
                + x as usize)
                + ti * spatial;
            if occupied[cell] != 0 {
                return Err(Error::Overlap);
            }
            occupied[cell] = 1;
            if let Some(record) = c.field(record_index)? {
                let e = record.electric_v_per_m();
                let b = record.magnetic_tesla();
                let stored_p = record.stored_poynting_w_per_m2();
                let hz = record.carrier_hz().unwrap_or(0.0);
                cells[cell] = DisplayEmfCell {
                    magnitude_v_per_m: e[0].hypot(e[1]).hypot(e[2]),
                    valid: true,
                };
                optical_cells[cell] = DisplayOpticalCell {
                    stokes_i: poynting_irradiance_w_per_m2(e, Some(b), stored_p),
                    stokes_q: 0.0,
                    stokes_u: 0.0,
                    stokes_v: 0.0,
                    spectral_sigma: carrier_hz_to_spectral_sigma(hz),
                    carrier_hz: hz,
                    phase_rad: 0.0,
                    coherence: record.confidence().map_or(1.0, f64::from),
                    valid: true,
                };
            }
        }
    }
    let mut source = vector(bytes.len(), limits.peak_owned_bytes)?;
    source.extend_from_slice(bytes);
    let source: Arc<[u8]> = Arc::from(source.into_boxed_slice());
    let frame_pose = selected_frame_pose(view, selected.frame_id);
    Ok(LoadedEmf {
        nx: dims[0],
        ny: dims[1],
        nz: dims[2],
        nt: u32::try_from(ticks.len()).map_err(|_| Error::Overflow)?,
        cell_count: u32::try_from(count).map_err(|_| Error::Overflow)?,
        bounds,
        frame_pose,
        cells,
        optical_cells,
        display_mode: 0,
        source,
        selection: selected,
        ticks,
        temporal: temporal.unwrap(),
    })
}
