//! Full-spectrum EMF CPU/GPU display adapters, CIE 1931 / 6-band spectral ramps,
//! and Q42EMF64 record conversion helpers (Packet E02).

use super::EmfFieldCell;
use crate::q42::{
    emf_container::EmfCellRecord64,
    field_spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S,
};
use crate::render::portal_emf_state::{
    carrier_hz_to_spectral_sigma, display_optical_to_gpu_cell, e_magnitude_from_poynting,
    poynting_irradiance_w_per_m2, DisplayEmfCell, DisplayOpticalCell, LoadedEmf,
};
use etalon_render::spectral::sigma::sigma_to_linear_rgb;

/// Convert paired scalar and optical/polarimetric display cells into a GPU cell.
pub fn emf_field_cell_from_optical(cell: &DisplayEmfCell, opt: &DisplayOpticalCell) -> EmfFieldCell {
    display_optical_to_gpu_cell(cell, opt)
}

/// Convert all cells of an admitted `LoadedEmf` into GPU `EmfFieldCell` storage.
pub fn emf_cells_from_loaded(loaded: &LoadedEmf) -> Result<Vec<EmfFieldCell>, String> {
    let cells = loaded.display_cells();
    let optical = loaded.optical_cells();
    if cells.len() != optical.len() {
        return Err("emf: optical cell length mismatch".into());
    }
    let mut out = Vec::new();
    out.try_reserve_exact(cells.len())
        .map_err(|_| "emf: GPU cell buffer allocation failed".to_string())?;
    out.extend(
        cells
            .iter()
            .zip(optical.iter())
            .map(|(c, o)| display_optical_to_gpu_cell(c, o)),
    );
    Ok(out)
}

/// Convert one `Q42EMF64` scientific 64-byte cell record directly into `EmfFieldCell`.
pub fn emf_field_cell_from_record64(rec: &EmfCellRecord64) -> Result<EmfFieldCell, String> {
    let finite = rec.e_field.iter().all(|v| v.is_finite())
        && rec.b_field.iter().all(|v| v.is_finite())
        && rec.poynting_flux.is_finite()
        && rec.poynting_flux >= 0.0
        && rec.wavelength_nm.is_finite()
        && rec.wavelength_nm >= 0.0
        && rec.coherence_phase.is_finite()
        && rec.stokes_q.is_finite()
        && rec.stokes_u.is_finite()
        && rec.stokes_v.is_finite()
        && rec.spectral_sigma.is_finite()
        && rec.confidence_alpha.is_finite()
        && (0.0..=1.0).contains(&rec.confidence_alpha);
    if !finite {
        return Err("emf64: non-finite or out-of-range cell record".into());
    }
    if rec.confidence_alpha == 0.0 {
        return Ok(EmfFieldCell::default());
    }
    let e = [rec.e_field[0] as f64, rec.e_field[1] as f64, rec.e_field[2] as f64];
    let b = [rec.b_field[0] as f64, rec.b_field[1] as f64, rec.b_field[2] as f64];
    let e_mag = e[0].hypot(e[1]).hypot(e[2]);
    let flux = if rec.poynting_flux > 0.0 {
        rec.poynting_flux as f64
    } else {
        poynting_irradiance_w_per_m2(e, Some(b), None)
    };
    let carrier_hz = if rec.wavelength_nm > 0.0 {
        (VACUUM_SPEED_OF_LIGHT_M_PER_S * 1.0e9) / (rec.wavelength_nm as f64)
    } else {
        0.0
    };
    let spectral_sigma = if rec.spectral_sigma > 0.0 {
        (rec.spectral_sigma as f64).clamp(0.0, 5.999_999)
    } else {
        carrier_hz_to_spectral_sigma(carrier_hz)
    };
    let raw_q = rec.stokes_q as f64;
    let raw_u = rec.stokes_u as f64;
    let raw_v = rec.stokes_v as f64;
    let pol_norm = raw_q.hypot(raw_u).hypot(raw_v);
    let (stokes_q, stokes_u, stokes_v) = if pol_norm > flux && pol_norm <= 1.0 + 1.0e-5 && flux > 0.0 {
        (raw_q * flux, raw_u * flux, raw_v * flux)
    } else if pol_norm <= flux * (1.0 + 1.0e-5) + 1.0e-12 {
        (raw_q, raw_u, raw_v)
    } else if pol_norm == 0.0 {
        (0.0, 0.0, 0.0)
    } else {
        return Err("emf64: Stokes polarization exceeds total irradiance I".into());
    };
    let cell = DisplayEmfCell {
        magnitude_v_per_m: if e_mag > 0.0 { e_mag } else { e_magnitude_from_poynting(flux) },
        valid: true,
    };
    let opt = DisplayOpticalCell {
        stokes_i: flux,
        stokes_q,
        stokes_u,
        stokes_v,
        spectral_sigma,
        carrier_hz,
        phase_rad: rec.coherence_phase as f64,
        coherence: rec.confidence_alpha as f64,
        valid: true,
    };
    Ok(display_optical_to_gpu_cell(&cell, &opt))
}

fn mix_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Full-spectrum 6-band linear sRGB evaluator matching WGSL `sigma_to_linear_rgb`
/// (`0`=CIE 1931 visible, `1`=RF, `2`=IR, `3`=UV, `4`=X-Ray, `5+`=Gamma).
pub fn full_spectrum_sigma_to_linear_rgb(sigma: f32) -> [f32; 3] {
    let clamped = sigma.max(0.0);
    let band = clamped.floor() as i32;
    let s = clamped - clamped.floor();
    match band {
        0 => sigma_to_linear_rgb(s),
        1 => [
            mix_f32(0.35, 1.0, s),
            mix_f32(0.02, 0.55, s * s),
            mix_f32(0.05, 0.08, s),
        ],
        2 => {
            let r = (s * 1.6).clamp(0.0, 1.0);
            let g = ((s - 0.35) * 1.55).clamp(0.0, 1.0);
            let b = ((s * std::f32::consts::PI).sin() * 0.75 + ((s - 0.75) * 4.0).max(0.0))
                .clamp(0.0, 1.0);
            [r, g, b]
        }
        3 => [
            mix_f32(0.42, 0.05, s),
            mix_f32(0.05, 0.92, s),
            mix_f32(0.75, 1.0, s),
        ],
        4 => {
            let c = s.powf(0.85);
            [c * 0.75, c * 0.95, (c * 1.15).clamp(0.0, 1.0)]
        }
        _ => [
            mix_f32(0.15, 0.95, s * s),
            mix_f32(0.35, 0.98, s),
            1.0,
        ],
    }
}

#[inline]
fn linear_to_srgb_u8(c: f32) -> u8 {
    let clamped = c.clamp(0.0, 1.0);
    let gamma = if clamped <= 0.003_130_8 {
        12.92 * clamped
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    (gamma * 255.0).round().clamp(0.0, 255.0) as u8
}

/// CPU slice evaluator with parity to GPU `EmfSliceUniform::projection_mode` (`0` = full
/// volumetric spectral + Stokes, `1` = nearest-cell grayscale `|E|` derivative).
#[allow(clippy::too_many_arguments)]
pub fn evaluate_display_slice_rgba8(
    cells: &[DisplayEmfCell],
    optical: &[DisplayOpticalCell],
    nx: u32,
    ny: u32,
    nz: u32,
    nt: u32,
    slice_z: u32,
    slice_t: u32,
    amplitude_scale: f32,
    phase_offset: f32,
    manifold_gain: f32,
    projection_mode: u32,
) -> Result<Vec<u8>, String> {
    let expected = (nx as usize)
        .checked_mul(ny as usize)
        .and_then(|v| v.checked_mul(nz as usize))
        .and_then(|v| v.checked_mul(nt as usize))
        .ok_or_else(|| "emf: slice dimension overflow".to_string())?;
    if expected == 0 || cells.len() != expected || optical.len() != expected {
        return Err("emf: invalid display slice buffer dimensions".into());
    }
    let sz = slice_z.min(nz - 1) as usize;
    let st = slice_t.min(nt - 1) as usize;
    let plane = (nx as usize) * (ny as usize);
    let slice_base = (st * (nz as usize) + sz) * plane;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(plane * 4)
        .map_err(|_| "emf: slice RGBA allocation failed".to_string())?;
    for y in 0..ny as usize {
        for x in 0..nx as usize {
            let idx = slice_base + y * (nx as usize) + x;
            let gpu = display_optical_to_gpu_cell(&cells[idx], &optical[idx]);
            if projection_mode == 1 {
                let g = linear_to_srgb_u8(gpu.amplitude);
                let a = (gpu.epistemic_weight.clamp(0.0, 1.0) * 255.0).round() as u8;
                rgba.extend_from_slice(&[g, g, g, a]);
                continue;
            }
            if gpu.epistemic_weight <= 0.0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let amp = (gpu.amplitude.abs() * amplitude_scale).clamp(0.0, 4.0);
            let sigma = if gpu.recurrence_frequency > 0.0 || gpu.frequency > 0.0 {
                (gpu.recurrence_frequency + phase_offset / std::f32::consts::TAU).max(0.0)
            } else {
                0.0
            };
            let base_rgb = full_spectrum_sigma_to_linear_rgb(sigma);
            let sat = (gpu.scale * manifold_gain).clamp(0.0, 1.5);
            let lum = base_rgb[0] * 0.2126 + base_rgb[1] * 0.7152 + base_rgb[2] * 0.0722;
            let dop = gpu.degree_of_polarization();
            let pol_highlight = 1.0 + 0.35 * dop;
            let sat_rgb = [
                mix_f32(lum, base_rgb[0], sat) * amp * pol_highlight,
                mix_f32(lum, base_rgb[1], sat) * amp * pol_highlight,
                mix_f32(lum, base_rgb[2], sat) * amp * pol_highlight,
            ];
            let a = (amp * 0.5 + 0.1).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[
                linear_to_srgb_u8(sat_rgb[0]),
                linear_to_srgb_u8(sat_rgb[1]),
                linear_to_srgb_u8(sat_rgb[2]),
                (a * 255.0).round() as u8,
            ]);
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_field_has_zero_sigma_and_grayscale_saturation() {
        let cell = DisplayEmfCell {
            magnitude_v_per_m: 3.0,
            valid: true,
        };
        let opt = DisplayOpticalCell {
            stokes_i: poynting_irradiance_w_per_m2([3.0, 0.0, 0.0], None, None),
            spectral_sigma: carrier_hz_to_spectral_sigma(0.0),
            carrier_hz: 0.0,
            coherence: 1.0,
            valid: true,
            ..Default::default()
        };
        assert_eq!(opt.spectral_sigma, 0.0);
        let gpu = emf_field_cell_from_optical(&cell, &opt);
        assert_eq!(gpu.recurrence_frequency, 0.0);
        assert_eq!(gpu.frequency, 0.0);
        assert_eq!(gpu.scale, 0.0);
        assert_eq!(gpu.epistemic_weight, 1.0);
        assert!(gpu.amplitude > 0.9);
    }

    #[test]
    fn full_spectrum_rf_ir_vis_uv_xray_gamma_have_no_visible_cutoff() {
        let bands = [
            (1.0e9_f64, 1),     // RF 1 GHz
            (10.0e12_f64, 2),   // IR 10 THz
            (5.45e14_f64, 0),   // Visible ~550 nm
            (1.5e15_f64, 3),    // UV 200 nm
            (1.0e18_f64, 4),    // X-ray ~4 keV
            (1.0e21_f64, 5),    // Gamma ~4 MeV
        ];
        for (freq_hz, expected_band) in bands {
            let sigma = carrier_hz_to_spectral_sigma(freq_hz);
            assert_eq!(sigma.floor() as i32, expected_band, "freq={freq_hz}");
            let cell = DisplayEmfCell {
                magnitude_v_per_m: 1.5,
                valid: true,
            };
            let opt = DisplayOpticalCell {
                stokes_i: poynting_irradiance_w_per_m2([1.5, 0.0, 0.0], None, None),
                spectral_sigma: sigma,
                carrier_hz: freq_hz,
                coherence: 1.0,
                valid: true,
                ..Default::default()
            };
            let gpu = emf_field_cell_from_optical(&cell, &opt);
            assert!(gpu.amplitude > 0.5);
            assert_eq!(gpu.scale, 1.0);
            assert!((gpu.recurrence_frequency - sigma as f32).abs() < 1e-5);
            let rgb = full_spectrum_sigma_to_linear_rgb(gpu.recurrence_frequency);
            assert!(rgb[0] > 0.0 || rgb[1] > 0.0 || rgb[2] > 0.0, "band={expected_band}");
            let rgba = evaluate_display_slice_rgba8(&[cell], &[opt], 1, 1, 1, 1, 0, 0, 1.0, 0.0, 1.0, 0)
                .unwrap();
            assert!(rgba[0] > 0 || rgba[1] > 0 || rgba[2] > 0);
            assert!(rgba[3] > 0);
        }
    }

    #[test]
    fn stokes_polarization_and_record64_convert_with_causality_checks() {
        let mut rec = EmfCellRecord64::default();
        rec.e_field = [10.0, 0.0, 0.0];
        rec.b_field = [0.0, 10.0 / 299_792_458.0, 0.0];
        rec.poynting_flux = 0.265_441_87;
        rec.wavelength_nm = 550.0;
        rec.coherence_phase = 1.25;
        rec.stokes_q = 0.6;
        rec.stokes_u = 0.0;
        rec.stokes_v = 0.8; // normalized DoP = 1.0
        let cell = emf_field_cell_from_record64(&rec).unwrap();
        assert!((cell.degree_of_polarization() - 1.0).abs() < 1e-3);
        assert!((cell.recurrence_frequency - 0.5).abs() < 1e-4);
        assert!((cell.phase - 1.25).abs() < 1e-6);

        let mut bad = rec;
        bad.stokes_q = 2.0;
        assert!(emf_field_cell_from_record64(&bad).is_err());
        bad.e_field[0] = f32::NAN;
        assert!(emf_field_cell_from_record64(&bad).is_err());
    }

    #[test]
    fn cpu_slice_projection_modes_preserve_missing_zero_and_spectral_parity() {
        let cells = [
            DisplayEmfCell { magnitude_v_per_m: 0.0, valid: true },
            DisplayEmfCell::default(),
            DisplayEmfCell { magnitude_v_per_m: 2.0, valid: true },
        ];
        let opts = [
            DisplayOpticalCell { coherence: 1.0, valid: true, ..Default::default() },
            DisplayOpticalCell::default(),
            DisplayOpticalCell {
                stokes_i: poynting_irradiance_w_per_m2([2.0, 0.0, 0.0], None, None),
                stokes_q: poynting_irradiance_w_per_m2([2.0, 0.0, 0.0], None, None) * 0.5,
                spectral_sigma: 0.5,
                carrier_hz: 5.450_771_963_6e14,
                coherence: 1.0,
                valid: true,
                ..Default::default()
            },
        ];
        let mode1 = evaluate_display_slice_rgba8(&cells, &opts, 3, 1, 1, 1, 0, 0, 1.0, 0.0, 1.0, 1).unwrap();
        assert_eq!(&mode1[0..4], &[0, 0, 0, 255]); // valid zero opaque
        assert_eq!(&mode1[4..8], &[0, 0, 0, 0]);   // missing transparent
        assert_eq!(mode1[8], mode1[9]);            // grayscale E only
        assert_eq!(mode1[9], mode1[10]);

        let mode0 = evaluate_display_slice_rgba8(&cells, &opts, 3, 1, 1, 1, 0, 0, 1.0, 0.0, 1.0, 0).unwrap();
        assert_eq!(&mode0[4..8], &[0, 0, 0, 0]);   // missing stays transparent
        assert!(mode0[9] > mode0[8] && mode0[9] > mode0[10]); // 550 nm green spectral hue
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn c_emf_upload_emf64_and_render_volumetric_mode0() {
        use crate::q42::emf_container::{
            emf_brick_voxel_linear_index, EmfCellRecord64, EmfContainerWriter, EMF_BRICK_CELLS,
        };
        use crate::q42::m64_matter::brick3d::encode_hilbert3d_u64;

        let mut writer = EmfContainerWriter::new([0.0, 0.0, 0.0], 0.25, [1, 1, 1], 0.0);
        let mut cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
        for z in 0..8 {
            for y in 0..8 {
                for x in 0..8 {
                    let idx = emf_brick_voxel_linear_index(x, y, z);
                    cells[idx].e_field = [6.0, 8.0, 0.0];
                    cells[idx].wavelength_nm = 532.0;
                    cells[idx].stokes_q = 0.7;
                    cells[idx].stokes_u = 0.2;
                }
            }
        }
        writer
            .add_brick_chunk(encode_hilbert3d_u64(0, 0, 0, 2), &cells, 0, 1, 0)
            .unwrap();
        let bytes = writer.to_bytes().unwrap();

        let loaded = LoadedEmf::from_emf64(&bytes).unwrap();
        assert_eq!(loaded.dimensions(), [8, 8, 8, 1]);
        assert!((loaded.display_cells()[0].magnitude_v_per_m - 10.0).abs() < 1e-4);
        assert!(loaded.optical_cells()[0].stokes_i > 0.0);
        assert!(loaded.optical_cells()[0].spectral_sigma > 0.0);

        if crate::gpu_context::try_shared_gpu().is_none() {
            return;
        }
        let mut portal = crate::render::gpu::PortalGpu::new_offscreen(32, 32, 256).expect("offscreen");
        portal.emf_upload_emf64(&bytes).expect("emf64 upload");
        assert!(portal.emf_has_field());
        assert_eq!(portal.emf_grid_dims(), (8, 8, 8, 1));
        let (w, h, pixels) = portal.emf_render_slice_to_rgba8().expect("render+readback");
        assert_eq!((w, h), (32, 32));
        assert!(pixels.chunks(4).any(|px| px[0] > 0 || px[1] > 0 || px[2] > 0));
    }
}
