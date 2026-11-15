//! Unit tests for medical reconstruction modules: FFT/Hilbert, CT FBP (parallel & fan),
//! MRI 2D/3D k-space & SENSE, PET OSEM, Ultrasound DAS, quantitative inference, and anatomy reg.
use super::anatomy_reg::*;
use super::fbp::*;
use super::inference::*;
use super::mri::*;
use super::pet_osem::*;
use super::spectral_fft::*;
use super::ultrasound_das::*;
use crate::medical::dicom_dataset::{ParseLimits, Part10Dataset};
use crate::medical::dicom_frames::{FrameLimits, FrameSet};
use crate::medical::dicom_geometry::{FrameGeometryInput, PatientImagePlane};
use crate::solvers::transforms::fourier::{dft, idft, Cplx};
use core::f64::consts::PI;

#[test]
fn spectral_fft_matches_solvers_oracle_for_pow2_and_non_pow2_and_hilbert() {
    for n in [15usize, 16, 37] {
        let sig: Vec<Cplx> = (0..n)
            .map(|k| ((k as f64 * 0.3).cos(), (k as f64 * 0.7).sin() * 0.5))
            .collect();
        let fast = fft_1d(&sig).unwrap();
        let oracle = dft(&sig);
        for (a, b) in fast.iter().zip(oracle.iter()) {
            assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        }
        let inv_fast = ifft_1d(&fast).unwrap();
        let inv_oracle = idft(&oracle);
        for ((a, b), o) in inv_fast.iter().zip(inv_oracle.iter()).zip(sig.iter()) {
            assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
            assert!((a.0 - o.0).abs() < 1e-10 && (a.1 - o.1).abs() < 1e-10);
        }
        let mut shifted = vec![(0.0, 0.0); n];
        let mut back = vec![(0.0, 0.0); n];
        fftshift_1d(&sig, &mut shifted);
        ifftshift_1d(&shifted, &mut back);
        assert_eq!(back, sig);
    }

    // Analytic Hilbert envelope of pure carrier `cos(w0 * n)` away from boundary ~ 1.0
    let n = 128usize;
    let carrier: Vec<f64> = (0..n).map(|i| (2.0 * PI * 8.0 * i as f64 / n as f64).cos()).collect();
    let env = analytic_hilbert_envelope_1d(&carrier).unwrap();
    for &e in &env[16..n - 16] {
        assert!((e - 1.0).abs() < 1e-10, "envelope {e} != 1.0");
    }
}

#[test]
fn ct_fbp_recovers_circular_disk_across_parallel_and_fan_geometries() {
    let radius = 8.0f64;
    let n_det = 65usize;
    let det_sp = 0.5f64;
    let det_mid = 0.5 * (n_det - 1) as f64;
    let img_dim = 33usize;
    let pix_sp = 0.75f64;
    let src_hash = [0x11u8; 32];

    // 1. Parallel beam [0, pi)
    let n_ang_p = 90usize;
    let angles_p: Vec<f64> = (0..n_ang_p).map(|i| PI * i as f64 / n_ang_p as f64).collect();
    let mut sino_p = vec![0.0; n_ang_p * n_det];
    for ai in 0..n_ang_p {
        for di in 0..n_det {
            let t = (di as f64 - det_mid) * det_sp;
            if t.abs() < radius {
                sino_p[ai * n_det + di] = 2.0 * (radius * radius - t * t).sqrt();
            }
        }
    }
    let mut out_p = vec![0.0; img_dim * img_dim];
    let prov = reconstruct_ct_fbp_into(
        &SinogramView {
            source_blake3: src_hash,
            n_angles: n_ang_p,
            n_detectors: n_det,
            detector_spacing_mm: det_sp,
            center_detector_offset: 0.0,
            angles_rad: &angles_p,
            projections: &sino_p,
            units: crate::medical::dicom_frames::ValueUnits::RescaleType("1/mm"),
        },
        &FbpConfig {
            image_width: img_dim,
            image_height: img_dim,
            pixel_spacing_mm: [pix_sp, pix_sp],
            filter: FbpFilter::RamLak,
            cutoff_ratio: 1.0,
            geometry: ProjectionGeometry::Parallel,
            apply_parker_short_scan: false,
        },
        &mut out_p,
    )
    .unwrap();
    let center = out_p[(img_dim / 2) * img_dim + (img_dim / 2)];
    assert!((center - 1.0).abs() < 0.08, "Parallel disk center {center}");
    assert_ne!(prov.derivation_blake3, src_hash);

    // 2. FanEquiangular full-scan [0, 2*pi) on centered disk: ray distance t = D * sin(gamma)
    let n_ang_f = 180usize;
    let d_iso = 80.0f64;
    let d_gamma = 0.005f64;
    let angles_f: Vec<f64> = (0..n_ang_f).map(|i| 2.0 * PI * i as f64 / n_ang_f as f64).collect();
    let mut sino_f = vec![0.0; n_ang_f * n_det];
    for ai in 0..n_ang_f {
        for di in 0..n_det {
            let gamma = (di as f64 - det_mid) * d_gamma;
            let t = d_iso * gamma.sin();
            if t.abs() < radius {
                sino_f[ai * n_det + di] = 2.0 * (radius * radius - t * t).sqrt();
            }
        }
    }
    let mut out_f = vec![0.0; img_dim * img_dim];
    reconstruct_ct_fbp_into(
        &SinogramView {
            source_blake3: src_hash,
            n_angles: n_ang_f,
            n_detectors: n_det,
            detector_spacing_mm: det_sp,
            center_detector_offset: 0.0,
            angles_rad: &angles_f,
            projections: &sino_f,
            units: crate::medical::dicom_frames::ValueUnits::RescaleType("1/mm"),
        },
        &FbpConfig {
            image_width: img_dim,
            image_height: img_dim,
            pixel_spacing_mm: [pix_sp, pix_sp],
            filter: FbpFilter::SheppLogan,
            cutoff_ratio: 0.95,
            geometry: ProjectionGeometry::FanEquiangular {
                source_to_isocenter_mm: d_iso,
                detector_arc_spacing_rad: d_gamma,
            },
            apply_parker_short_scan: false,
        },
        &mut out_f,
    )
    .unwrap();
    let center_f = out_f[(img_dim / 2) * img_dim + (img_dim / 2)];
    assert!((center_f - 1.0).abs() < 0.12, "FanEquiangular center {center_f}");

    // Fail-closed sentinel check on invalid geometry
    let mut sentinel = vec![-99.0; img_dim * img_dim];
    assert!(reconstruct_ct_fbp_into(
        &SinogramView {
            source_blake3: src_hash, n_angles: n_ang_p, n_detectors: n_det, detector_spacing_mm: 0.0,
            center_detector_offset: 0.0, angles_rad: &angles_p, projections: &sino_p,
            units: crate::medical::dicom_frames::ValueUnits::RescaleType("1/mm"),
        },
        &FbpConfig {
            image_width: img_dim, image_height: img_dim, pixel_spacing_mm: [pix_sp, pix_sp],
            filter: FbpFilter::Hann, cutoff_ratio: 0.8, geometry: ProjectionGeometry::Parallel, apply_parker_short_scan: false,
        },
        &mut sentinel,
    ).is_err());
    assert!(sentinel.iter().all(|&v| v == -99.0));
}

#[test]
fn mri_2d_3d_complex_phase_and_cartesian_sense_unfolding() {
    // 3D non-pow2 phase preservation roundtrip (slices=3, rows=6, cols=5)
    let (s, r, c) = (3usize, 6usize, 5usize);
    let orig_vol: Vec<Cplx> = (0..s * r * c)
        .map(|i| {
            let amp = 1.0 + (i % 7) as f64;
            let ph = -1.2 + 0.1 * (i % 20) as f64;
            (amp * ph.cos(), amp * ph.sin())
        })
        .collect();
    let kspace = fft_3d(&orig_vol, s, r, c, false).unwrap();
    let recon = reconstruct_mri_kspace_3d(
        &kspace,
        MriKspaceConfig { slices: s, rows: r, cols: c, ifftshift_kspace: false, fftshift_image: false, unitary_scaling: false },
        [0x22; 32],
    )
    .unwrap();
    for (o, rc) in orig_vol.iter().zip(recon.complex.iter()) {
        assert!((o.0 - rc.0).abs() < 1e-9 && (o.1 - rc.1).abs() < 1e-9);
    }
    let exp_phase0 = orig_vol[5].1.atan2(orig_vol[5].0);
    assert!((recon.phase_rad[5] - exp_phase0).abs() < 1e-10);

    // Cartesian R=2 SENSE unaliasing across 4 coils on 8x4 object
    let (rows, cols, accel_r) = (8usize, 4usize, 2usize);
    let obj: Vec<Cplx> = (0..rows * cols)
        .map(|i| ((i / cols) as f64 + 1.0, ((i % cols) as f64) * 0.25))
        .collect();
    let mut sens_vecs = Vec::new();
    for ci in 0..4usize {
        let mut s_map = vec![(0.0, 0.0); rows * cols];
        for y in 0..rows {
            for x in 0..cols {
                let ph = 0.4 * (ci as f64) * (y as f64) - 0.1 * (x as f64);
                let mag = 0.6 + 0.15 * ((y + ci) % 4) as f64;
                s_map[y * cols + x] = (mag * ph.cos(), mag * ph.sin());
            }
        }
        sens_vecs.push(s_map);
    }
    let red_rows = rows / accel_r;
    let mut aliased = vec![vec![(0.0, 0.0); red_rows * cols]; 4];
    for ci in 0..4 {
        for y0 in 0..red_rows {
            for x in 0..cols {
                for r_idx in 0..accel_r {
                    let y = y0 + r_idx * red_rows;
                    let sv = sens_vecs[ci][y * cols + x];
                    let ov = obj[y * cols + x];
                    aliased[ci][y0 * cols + x].0 += sv.0 * ov.0 - sv.1 * ov.1;
                    aliased[ci][y0 * cols + x].1 += sv.0 * ov.1 + sv.1 * ov.0;
                }
            }
        }
    }
    let a_refs: Vec<&[Cplx]> = aliased.iter().map(|v| v.as_slice()).collect();
    let s_refs: Vec<&[Cplx]> = sens_vecs.iter().map(|v| v.as_slice()).collect();
    let sense = combine_coils_sense(&a_refs, &s_refs, rows, cols, accel_r, None, 0.0, [0x33; 32]).unwrap();
    for i in 0..rows * cols {
        assert!((sense.complex[i].0 - obj[i].0).abs() < 1e-8);
        assert!((sense.complex[i].1 - obj[i].1).abs() < 1e-8);
        assert!(sense.g_factor[i] >= 1.0);
    }
}

#[test]
fn pet_osem_recovers_hot_spots_with_acf_and_count_conservation() {
    let w = 15usize;
    let h = 15usize;
    let mut truth = vec![0.2f64; w * h];
    truth[7 * w + 7] = 10.0;
    truth[4 * w + 10] = 6.0;
    let n_ang = 24usize;
    let n_bins = 21usize;
    let angles: Vec<f64> = (0..n_ang).map(|i| PI * i as f64 / n_ang as f64).collect();
    let acf: Vec<f64> = (0..n_ang * n_bins).map(|i| 0.7 + 0.1 * ((i % 3) as f64)).collect();
    let norm = vec![1.1f64; n_ang * n_bins];
    let tmp_sino = PetSinogram {
        source_blake3: [0x44; 32], n_angles: n_ang, n_bins, bin_spacing_mm: 2.0,
        angles_rad: &angles, prompts: &norm, attenuation_factors: Some(&acf), normalization_factors: Some(&norm),
    };
    let prompts = forward_project_pet(&truth, &tmp_sino, 2.0, w, h).unwrap();
    let sino = PetSinogram { prompts: &prompts, ..tmp_sino };
    let mut recon = vec![0.0; w * h];
    let res = reconstruct_pet_osem_into(
        &sino,
        &OsemConfig {
            image_width: w, image_height: h, pixel_size_mm: 2.0,
            subsets: 4, iterations: 12, min_sensitivity: 1e-3, initial_activity: 1.0,
        },
        &mut recon,
    )
    .unwrap();
    assert!(res.min_voxel >= 0.0);
    assert!(recon[7 * w + 7] > recon[7 * w + 6] * 2.5);
    assert!(recon[4 * w + 10] > recon[4 * w + 8] * 2.0);
    let rel_count_err = (res.total_forward_counts - res.total_measured_counts).abs() / res.total_measured_counts;
    assert!(rel_count_err < 0.02, "count conservation residual {rel_count_err}");
}

#[test]
fn ultrasound_das_focuses_point_scatterer_envelope() {
    let n_elem = 32usize;
    let elem_x: Vec<f64> = (0..n_elem).map(|i| (i as f64 - 15.5) * 0.5).collect();
    let elem_z = vec![0.0; n_elem];
    let c_mm_s = 1_540_000.0;
    let fs = 20_000_000.0;
    let n_samp = 1024usize;
    let target = [2.0f64, 20.0f64];
    let mut rf = vec![0.0f64; n_elem * n_samp];
    for e in 0..n_elem {
        let t_tot = target[1] / c_mm_s + (target[0] - elem_x[e]).hypot(target[1]) / c_mm_s;
        for s in 0..n_samp {
            let dt = (s as f64 / fs) - t_tot;
            if dt.abs() < 4e-7 {
                rf[e * n_samp + s] = (-dt * dt / (2.0 * 1e-14)).exp() * (2.0 * PI * 5e6 * dt).cos();
            }
        }
    }
    let px_grid = [-4.0, 0.0, 2.0, 4.0];
    let pz_grid: Vec<f64> = (0..64).map(|k| 15.0 + k as f64 * 0.2).collect(); // idx 25 is z=20.0
    let mut db_out = vec![0.0; px_grid.len() * pz_grid.len()];
    let res = beamform_ultrasound_das_into(
        &rf, n_samp,
        &UltrasoundArray { elem_x_mm: &elem_x, elem_z_mm: &elem_z, tx_mode: TxFocusMode::PlaneWave { angle_rad: 0.0 } },
        &DasConfig {
            sound_speed_mps: 1540.0, sample_rate_hz: fs, f_number: 1.5, min_aperture_half_mm: 3.0,
            apodization: WindowKind::Hanning, dynamic_range_db: 60.0, pixel_x_mm: &px_grid, pixel_z_mm: &pz_grid,
        },
        [0x55; 32],
        &mut db_out,
    ).unwrap();
    let peak_val = db_out[25 * 4 + 2]; // (x=2.0, z=20.0)
    let off_val = db_out[25 * 4 + 0];  // (x=-4.0, z=20.0)
    assert!(peak_val > -3.0 && off_val < peak_val - 10.0, "peak={peak_val}, off={off_val}");
    assert!(res.peak_envelope > 0.5);
}

#[test]
fn inference_and_anatomy_registration_enforce_units_identity_and_for() {
    let ds = Part10Dataset::parse(include_bytes!("../dicom_dataset/fixtures/enhanced-defined.dcm"), ParseLimits::default()).unwrap();
    let frames = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    let hu_tf = frames.frames[0].quantitative().modality.unwrap();

    // Calibrated waveform (250 Hz, 10 Hz dominant sine wave)
    let n = 100usize;
    let wave: Vec<f64> = (0..n).map(|i| (2.0 * PI * 10.0 * i as f64 / 250.0).sin() * 100.0).collect();
    let mut cal_out = vec![0.0; n];
    let mut unc_out = vec![0.0; n];
    let prov = calibrate_and_infer_into(
        &wave, &hu_tf, ds.source_blake3(), 1, NonVisualDomain::Waveform { sample_rate_hz: 250.0 },
        None, UncertaintyBudget { stored_measurement_std: 0.5, ..Default::default() }, "hu_ecg_v1", &mut cal_out, &mut unc_out,
    ).unwrap();
    assert!((prov.dominant_frequency_hz.unwrap() - 10.0).abs() < 0.25);

    // Unspecified modality units rejected; RWVM UCUM:mm accepted inside range and rejected outside
    let rw_ds = Part10Dataset::parse(include_bytes!("../dicom_dataset/fixtures/native-u8-rwvm.dcm"), ParseLimits::default()).unwrap();
    let rw_frames = FrameSet::extract(&rw_ds, FrameLimits::default()).unwrap();
    let q = rw_frames.frames[0].quantitative();
    let mut c2 = [0.0; 2];
    let mut u2 = [0.0; 2];
    assert_eq!(
        calibrate_and_infer_into(&[10.0, 20.0], &q.modality.unwrap(), rw_ds.source_blake3(), 1, NonVisualDomain::ScalarSeries, None, UncertaintyBudget::default(), "t", &mut c2, &mut u2).unwrap_err(),
        InferenceError::UnspecifiedUnits
    );
    assert_eq!(
        calibrate_and_infer_into(&[256.0], &q.real_world.unwrap(), rw_ds.source_blake3(), 1, NonVisualDomain::ScalarSeries, None, UncertaintyBudget::default(), "t", &mut c2, &mut u2).unwrap_err(),
        InferenceError::OutOfStoredRange
    );

    // Anatomy registration SO(3) 90-deg Z rotation + FoR guard + reflection rejection
    let plane = PatientImagePlane::from_frame_geometry(FrameGeometryInput {
        image_position_patient: Some([10.0, 20.0, 30.0]),
        image_orientation_patient: Some([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        pixel_spacing: Some([1.0, 1.0]),
        frame_of_reference_uid: Some("1.2.840.999.1"),
        anatomical_orientation_type: Some("BIPED"),
        invalid_source_fields: &[],
    }).unwrap();
    let rot90 = AnatomyTransform::Rigid {
        rotation_3x3: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        translation_mm: [1.0, 2.0, 3.0],
    };
    let reg = register_patient_to_anatomy(&plane, ds.source_blake3(), 2, 1, "1.2.840.999.1", &rot90, None, 1e-6).unwrap();
    assert!(reg.roundtrip_residual_mm < 1e-10);
    assert_ne!(reg.derivation_blake3, ds.source_blake3());
    assert_eq!(
        register_patient_to_anatomy(&plane, ds.source_blake3(), 0, 0, "1.2.840.999.2", &rot90, None, 1e-6).unwrap_err(),
        AnatomyRegError::FrameOfReferenceMismatch
    );
    let refl = AnatomyTransform::Rigid {
        rotation_3x3: [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation_mm: [0.0; 3],
    };
    assert_eq!(
        register_patient_to_anatomy(&plane, ds.source_blake3(), 0, 0, "1.2.840.999.1", &refl, None, 1e-6).unwrap_err(),
        AnatomyRegError::ReflectionRejected
    );
}
