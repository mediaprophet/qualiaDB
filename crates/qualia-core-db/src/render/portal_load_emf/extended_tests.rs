use super::extended_test_builders::*;
use crate::q42::emf_container::{
    emf_brick_voxel_linear_index, EmfCellRecord64, EmfContainerWriter, EMF_BRICK_CELLS,
};
use crate::q42::field_spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S;
use crate::q42::m64_matter::brick3d::encode_hilbert3d_u64;
use crate::render::portal_emf_state::{
    carrier_hz_to_spectral_sigma, e_magnitude_from_poynting, poynting_irradiance_w_per_m2,
    DisplayOpticalCell, EmfBackend, EmfDisplayStatus, RetainedEmf,
};
use crate::render::portal_load_emf::*;

#[test]
fn mode0_ungridded_carriers_zero_static_and_point_cloud_sparsity() {
    let mut blocks = base_blocks(None);
    blocks.push(ext_block(
        4,
        0x0100,
        crate::q42::emf_profiles::FIELD_PROFILE,
        [1, 1, 1],
        [1, 1, 1],
        vec![],
    ));
    blocks.push(desc_block(5, "electric_field", "V/m", Some(4), None));

    let f_vis = VACUUM_SPEED_OF_LIGHT_M_PER_S / 532.0e-9;
    let f_rf = 1.0e9;
    let mut payload = Vec::new();
    payload.extend_from_slice(&rec_mode0([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0, None, None));
    payload.extend_from_slice(&rec_mode0([3.0, 4.0, 0.0], [1.0, 0.0, 0.0], 0, Some(f_vis), Some(0.5)));
    payload.extend_from_slice(&rec_mode0([0.0, 0.0, 2.0], [2.0, 0.0, 0.0], 0, Some(f_rf), None));

    let bytes = seal_blocks(blocks, vec![entry_ungridded(0x0100, 5, 3, 128, &payload)]);
    let loaded = decode_emf_scientific(&bytes).expect("ungridded 0x0100 point cloud");
    assert_eq!(loaded.display_mode(), 0x0100);
    assert!(loaded.dimensions()[0] >= 3);

    let cells = loaded.display_cells();
    let opts = loaded.optical_cells();
    // x=0 holds explicit zero static point -> valid == true, magnitude == 0.0, sigma == 0.0.
    assert!(cells[0].valid && cells[0].magnitude_v_per_m == 0.0);
    assert!(opts[0].valid && opts[0].spectral_sigma == 0.0 && opts[0].stokes_i == 0.0);
    // Unhit interior voxel(s) stay transparent with all zero bits.
    let invalid_count = cells.iter().filter(|c| !c.valid).count();
    assert!(invalid_count > 0);
    assert!(cells
        .iter()
        .zip(opts.iter())
        .filter(|(c, _)| !c.valid)
        .all(|(c, o)| c.magnitude_v_per_m.to_bits() == 0 && o.spectral_sigma.to_bits() == 0));
    // Visible + RF points carry distinct non-zero sigma bands.
    let vis = opts.iter().find(|o| o.valid && (o.carrier_hz - f_vis).abs() < 1.0).unwrap();
    let rf = opts.iter().find(|o| o.valid && (o.carrier_hz - f_rf).abs() < 1.0).unwrap();
    assert!((0.0..1.0).contains(&vis.spectral_sigma));
    assert!((1.0..2.0).contains(&rf.spectral_sigma));
}

#[test]
fn mode1_polarimetric_ray_deposits_stokes_and_phase_dda() {
    let mut blocks = base_blocks(None);
    blocks.push(pol_block(6, "W sr^-1 m^-2"));
    blocks.push(ext_block(
        4,
        0x0101,
        crate::q42::emf_profiles::RAY_PROFILE,
        [1, 1, 1],
        [1, 1, 1],
        vec![tref(16, 6)],
    ));
    blocks.push(desc_block(5, "radiance", "W sr^-1 m^-2", Some(4), None));

    let inv_s3 = 1.0 / 3.0_f64.sqrt();
    let f_vis = 5.0e14;
    let payload = rec_mode1(
        [0.0, 0.0, 0.0],
        [inv_s3, inv_s3, inv_s3],
        [10.0, 6.0, -4.0, 2.0],
        f_vis,
        2.0 * 3.0_f64.sqrt(),
        Some(1.25),
    );
    let bytes = seal_blocks(blocks, vec![entry_ungridded(0x0101, 5, 1, 128, &payload)]);
    let loaded = decode_emf_scientific(&bytes).expect("0x0101 ray DDA");
    assert_eq!(loaded.display_mode(), 0x0101);
    let valid: Vec<_> = loaded.optical_cells().iter().filter(|o| o.valid).collect();
    let invalid_n = loaded.display_cells().iter().filter(|c| !c.valid).count();
    assert!(valid.len() > 1, "expected multi-voxel diagonal DDA coverage");
    assert!(invalid_n > 0, "expected off-ray voxels to stay invalid");
    for o in valid {
        assert!(o.stokes_i > 0.0 && o.stokes_q > 0.0 && o.stokes_u < 0.0 && o.stokes_v > 0.0);
        let pol = o.stokes_q.hypot(o.stokes_u).hypot(o.stokes_v);
        assert!(pol <= o.stokes_i * (1.0 + 1e-6));
        assert!((0.0..1.0).contains(&o.spectral_sigma));
        assert!((o.phase_rad - 1.25).abs() < 1e-5);
    }
}

#[test]
fn mode2_yee_cartesian_exh_irradiance_and_zero_cell() {
    let mut blocks = base_blocks(Some(([1.0, 1.0, 1.0], [2, 2, 1])));
    blocks.push(constitutive_block(6));
    blocks.push(ext_block(
        4,
        0x0102,
        "yee_cartesian_v1",
        [2, 2, 1],
        [2, 2, 1],
        vec![tref(14, 6)],
    ));
    blocks.push(desc_block(5, "electric_field", "V/m", Some(4), None));

    let mut payload = Vec::with_capacity(512);
    payload.extend_from_slice(&rec_mode2_yee([3.0, 0.0, 0.0], [0.0, 4.0, 0.0], 2.25, 1.0));
    payload.extend_from_slice(&rec_mode2_yee([0.0; 3], [0.0; 3], 1.0, 1.0));
    payload.extend_from_slice(&rec_mode2_yee([0.0, 5.0, 0.0], [0.0, 0.0, 2.0], 1.0, 1.0));
    payload.extend_from_slice(&rec_mode2_yee([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0, 1.0));

    let mut e = entry_ungridded(0x0102, 5, 4, 128, &payload);
    e.flags = 4; // SpatialRegular
    let bytes = seal_blocks(blocks, vec![e]);
    let loaded = decode_emf_scientific(&bytes).expect("0x0102 Yee Cartesian");
    assert_eq!(loaded.dimensions(), [2, 2, 1, 1]);
    assert_eq!(loaded.display_mode(), 0x0102);

    assert_eq!(loaded.display_cells()[0].magnitude_v_per_m, 3.0);
    assert!((loaded.optical_cells()[0].stokes_i - 12.0).abs() < 1e-9);
    assert!(loaded.display_cells()[1].valid && loaded.display_cells()[1].magnitude_v_per_m == 0.0);
    assert!((loaded.optical_cells()[2].stokes_i - 10.0).abs() < 1e-9);
    let expected_z0_i = poynting_irradiance_w_per_m2([1.0, 0.0, 0.0], None, None);
    assert!((loaded.optical_cells()[3].stokes_i - expected_z0_i).abs() < 1e-12);
}

#[test]
fn mode3_full_spectrum_six_bands_no_visible_cutoff() {
    let cases = [
        (1.0e9_f64, 1u32),                              // RF
        (1.0e13_f64, 2u32),                             // IR / THz
        (VACUUM_SPEED_OF_LIGHT_M_PER_S / 550.0e-9, 0),  // Visible
        (VACUUM_SPEED_OF_LIGHT_M_PER_S / 150.0e-9, 3),  // UV
        (2.4e18_f64, 4u32),                             // X-ray
        (5.0e20_f64, 5u32),                             // Gamma
    ];
    for (freq_hz, expected_band) in cases {
        let mut blocks = base_blocks(None);
        blocks.push(spectrum_axis_block(7, &[freq_hz * 0.8, freq_hz * 1.25]));
        blocks.push(ext_block(
            4,
            0x0103,
            crate::q42::emf_profiles::SPECTRUM_PROFILE,
            [1, 1, 1],
            [1, 1, 1],
            vec![tref(12, 7), tu32(19, 1)],
        ));
        blocks.push(desc_block(5, "bin_integrated_irradiance", "W m^-2", Some(4), None));

        let payload = rec_mode3([0.0, 0.0, 0.0], 0, 7, &[2.5]);
        let bytes = seal_blocks(blocks, vec![entry_ungridded(0x0103, 5, 1, payload.len() as u32, &payload)]);
        let loaded = decode_emf_scientific(&bytes).expect("full-spectrum 0x0103");
        let cell = loaded.display_cells()[0];
        let opt = loaded.optical_cells()[0];
        assert!(cell.valid && (cell.magnitude_v_per_m - e_magnitude_from_poynting(2.5)).abs() < 1e-9);
        assert!((opt.stokes_i - 2.5).abs() < 1e-9);
        let band = opt.spectral_sigma.floor() as u32;
        assert_eq!(band, expected_band, "freq={freq_hz} sigma={}", opt.spectral_sigma);
    }
}

#[test]
fn mode4_mueller_and_coexisting_auxiliary_chunk_disambiguation() {
    let mut blocks = base_blocks(None);
    blocks.push(pol_block(6, "sr^-1"));
    blocks.push(ev_block(7, false));
    blocks.push(ext_block(
        4,
        0x0104,
        "mueller_scattering_v1",
        [1, 1, 1],
        [1, 1, 1],
        vec![tref(16, 6), tref(17, 7)],
    ));
    blocks.push(desc_block(5, "mueller_matrix", "sr^-1", Some(4), None));
    // Secondary descriptor WITHOUT Schema 103 ref -> admitted as ExtendedKind::Auxiliary.
    blocks.push(desc_block(8, "calibration_metadata", "1", None, None));

    let f_uv = VACUUM_SPEED_OF_LIGHT_M_PER_S / 200.0e-9;
    let mueller_payload = rec_mode4_mueller([0.0, 0.0, 0.0], f_uv, [10.0, 5.0, 0.0, -3.0]);
    let aux_payload = vec![0x42u8; 64];
    let bytes = seal_blocks(
        blocks,
        vec![
            entry_ungridded(0x0104, 5, 1, 256, &mueller_payload),
            entry_ungridded(0x0104, 8, 1, 64, &aux_payload),
        ],
    );
    let loaded = decode_emf_scientific(&bytes).expect("0x0104 + Auxiliary must not collide");
    let o = loaded.optical_cells()[0];
    assert!(o.valid && (o.stokes_i - 10.0).abs() < 1e-9);
    assert!((o.stokes_q - 5.0).abs() < 1e-9 && (o.stokes_v + 3.0).abs() < 1e-9);
    assert!((3.0..4.0).contains(&o.spectral_sigma));
}

#[test]
fn mode5_iq_calibrated_envelope_and_crc_tamper_rejected() {
    let make_iq_doc = |corrupt_crc: bool| {
        let mut blocks = base_blocks(None);
        blocks.push(calib_block(6, "V", 2.0, 0.5));
        blocks.push(acq_block(7, 6));
        blocks.push(ext_block(
            4,
            0x0105,
            "calibrated_iq_stft_v1",
            [1, 1, 1],
            [1, 1, 1],
            vec![tref(15, 7), tref(30, 6)],
        ));
        blocks.push(desc_block(5, "complex_baseband_voltage", "V", Some(4), None));

        let mut payload = rec_mode5_iq(0, 1, 1.0e8, 0.75, &[(3.0, 4.0), (0.0, 0.0)]);
        if corrupt_crc {
            payload[60] ^= 0xFF;
        }
        seal_blocks(blocks, vec![entry_ungridded(0x0105, 5, 1, payload.len() as u32, &payload)])
    };

    let bytes = make_iq_doc(false);
    let loaded = decode_emf_scientific(&bytes).expect("0x0105 calibrated IQ");
    assert_eq!(loaded.dimensions(), [2, 1, 1, 1]);
    assert!(!loaded.display_cells()[0].valid);
    let rms = ((25.0_f64) / 2.0).sqrt();
    let expected_e = (rms * 2.0 + 0.5).abs();
    assert!((loaded.display_cells()[1].magnitude_v_per_m - expected_e).abs() < 1e-9);
    assert!((1.0..2.0).contains(&loaded.optical_cells()[1].spectral_sigma));

    let bad = make_iq_doc(true);
    let err = decode_emf_with_selection(&bad, None, PortalEmfLimits::default())
        .err()
        .expect("corrupted mode 5 CRC must fail");
    assert_eq!(err, Error::Profile(crate::q42::emf_profiles::Error::Reserved));
}

#[test]
fn mode6_ionizing_mask_and_mode7_bioelectric() {
    // Unmasked 0x0106 ionizing photon at 60 keV -> X-ray sigma band 4.
    let mut blocks6 = base_blocks(None);
    blocks6.push(ev_block(6, true));
    blocks6.push(calib_block(7, "J", 1.0, 0.0));
    blocks6.push(ext_block(
        4,
        0x0106,
        "ionizing_radiation_v1",
        [1, 1, 1],
        [1, 1, 1],
        vec![tref(17, 6), tref(30, 7)],
    ));
    blocks6.push(desc_block(5, "spectral_fluence", "J", Some(4), None));
    let e_60kev = 60.0e3 * 1.602_176_634e-19;
    let p6 = rec_mode6_ionizing([0.0; 3], 0, e_60kev, 1.0e15);
    let b6 = seal_blocks(blocks6, vec![entry_ungridded(0x0106, 5, 1, 128, &p6)]);
    let l6 = decode_emf_scientific(&b6).expect("0x0106 ionizing");
    assert!((4.0..5.0).contains(&l6.optical_cells()[0].spectral_sigma));

    // Masked mode=6 (entry 1) + companion 0x106 mask (entry 2), plus orphan mask rejection.
    for attach_mask in [true, false] {
        let mut bm = base_blocks(None);
        bm.push(ev_block(6, true));
        bm.push(calib_block(7, "J", 1.0, 0.0));
        bm.push(ext_block(
            4,
            6,
            "ionizing_radiation_v1",
            [1, 1, 1],
            [1, 1, 1],
            vec![tref(17, 6), tref(30, 7)],
        ));
        let (mask_ext, mask_desc) = make_mask_desc_pair(10, 11);
        bm.push(mask_ext);
        bm.push(mask_desc);
        let mask_ref = attach_mask.then_some((2u32, 1u64));
        bm.push(desc_block(5, "spectral_fluence", "J", Some(4), mask_ref));

        let mut payload = Vec::with_capacity(256);
        payload.extend_from_slice(&rec_mode6_ionizing([0.0; 3], 0, e_60kev, 1.0e15));
        let mut masked_rec = [0u8; 128];
        let crc_zero = crate::container_10d::crc32c::crc32c(&masked_rec[..124]);
        wr_u32(&mut masked_rec, 124, crc_zero);
        payload.extend_from_slice(&masked_rec);
        let mask_bytes = [1u8, 0u8];
        let mut mask_entry = entry_ungridded(0x106, 11, 2, 1, &mask_bytes);
        mask_entry.frame_id = 0;
        let bytes = seal_blocks(
            bm,
            vec![entry_ungridded(6, 5, 2, 128, &payload), mask_entry],
        );
        let res = decode_emf_with_selection(&bytes, None, PortalEmfLimits::default());
        if attach_mask {
            let loaded = res.expect("masked mode 6 must decode");
            assert!(loaded.display_cells()[0].valid);
        } else {
            assert_eq!(res.err(), Some(Error::OrphanValidityMask));
        }
    }

    // Mode 0x0107 bioelectric current dipole at 40 Hz.
    let mut b7 = base_blocks(None);
    b7.push(ev_block(6, false));
    b7.push(calib_block(7, "V", 1.0, 0.0));
    b7.push(bio_block(8));
    b7.push(ext_block(
        4,
        0x0107,
        "bioelectric_dipole_v1",
        [1, 1, 1],
        [1, 1, 1],
        vec![tref(17, 6), tref(22, 8), tref(30, 7)],
    ));
    b7.push(desc_block(5, "bioelectric_potential", "V", Some(4), None));
    let p7 = rec_mode7_bio([0.0; 3], [0.0, 0.0, 1.0e-9], 2.5e-3, 0, 40.0);
    let s7 = seal_blocks(b7, vec![entry_ungridded(0x0107, 5, 1, 128, &p7)]);
    let l7 = decode_emf_scientific(&s7).expect("0x0107 bioelectric");
    assert!((l7.display_cells()[0].magnitude_v_per_m - 2.5e-3).abs() < 1e-12);
    assert!((1.0..2.0).contains(&l7.optical_cells()[0].spectral_sigma));
}

#[test]
fn emf64_container_roundtrip_and_retained_optical_validation() {
    let mut writer = EmfContainerWriter::new([0.0, 0.0, 0.0], 1.0, [1, 1, 1], 0.0);
    let mut cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
    cells[0].confidence_alpha = 0.0;
    let idx = emf_brick_voxel_linear_index(1, 2, 3);
    cells[idx].e_field = [3.0, 4.0, 0.0];
    cells[idx].b_field = [0.0, 0.0, 1.0e-6];
    cells[idx].wavelength_nm = 550.0;
    cells[idx].stokes_q = 0.5;
    cells[idx].coherence_phase = 0.8;
    cells[idx].confidence_alpha = 1.0;
    let h = encode_hilbert3d_u64(0, 0, 0, 0);
    writer.add_brick_chunk(h, &cells, 0, 1, 0).unwrap();
    let bytes = writer.to_bytes().unwrap();

    let loaded = LoadedEmf::from_emf64(&bytes).expect("Q42EMF64 portal decode");
    assert_eq!(loaded.dimensions(), [8, 8, 8, 1]);
    let flat = (3 * 8 + 2) * 8 + 1;
    assert!(loaded.display_cells()[flat].valid);
    assert!((loaded.optical_cells()[flat].spectral_sigma - 0.5).abs() < 1e-4);
    assert!(loaded.optical_cells()[flat].stokes_q > 0.0);
    assert!(!loaded.display_cells()[0].valid);
    assert_eq!(loaded.optical_cells()[0].spectral_sigma.to_bits(), 0);

    let mut retained = RetainedEmf::default();
    retained.accept(loaded, EmfBackend::WebGpu, |_| Ok(())).unwrap();
    assert_eq!(retained.status(), EmfDisplayStatus::Uploaded);
    assert_eq!(carrier_hz_to_spectral_sigma(0.0), 0.0);
    let _ = DisplayOpticalCell::default();
}

#[test]
fn negative_nan_stokes_causality_truncated_and_budget_limits() {
    // 1. Non-finite Mode 0 electric field.
    let mut b0 = base_blocks(None);
    b0.push(ext_block(4, 0x0100, crate::q42::emf_profiles::FIELD_PROFILE, [1, 1, 1], [1, 1, 1], vec![]));
    b0.push(desc_block(5, "electric_field", "V/m", Some(4), None));
    let p_nan = rec_mode0([f64::NAN, 0.0, 0.0], [0.0; 3], 0, None, None);
    let nan_bytes = seal_blocks(b0, vec![entry_ungridded(0x0100, 5, 1, 128, &p_nan)]);
    assert!(decode_emf_scientific(&nan_bytes).is_err());

    // 2. Stokes causality Q > I rejected in Mode 0x0101.
    let mut b1 = base_blocks(None);
    b1.push(pol_block(6, "W sr^-1 m^-2"));
    b1.push(ext_block(4, 0x0101, crate::q42::emf_profiles::RAY_PROFILE, [1, 1, 1], [1, 1, 1], vec![tref(16, 6)]));
    b1.push(desc_block(5, "radiance", "W sr^-1 m^-2", Some(4), None));
    let p_bad_stokes = rec_mode1([0.0; 3], [1.0, 0.0, 0.0], [10.0, 12.0, 0.0, 0.0], 5.0e14, 1.0, None);
    let bad_stokes = seal_blocks(b1, vec![entry_ungridded(0x0101, 5, 1, 128, &p_bad_stokes)]);
    assert_eq!(
        decode_emf_with_selection(&bad_stokes, None, PortalEmfLimits::default()).err(),
        Some(Error::Profile(crate::q42::emf_profiles::Error::Value))
    );

    // 3. Budget exhaustion on 2x2x1 Yee grid when display_cells limit = 1.
    let mut b2 = base_blocks(Some(([1.0, 1.0, 1.0], [2, 2, 1])));
    b2.push(constitutive_block(6));
    b2.push(ext_block(4, 0x0102, "yee_cartesian_v1", [2, 2, 1], [2, 2, 1], vec![tref(14, 6)]));
    b2.push(desc_block(5, "electric_field", "V/m", Some(4), None));
    let p2 = rec_mode2_yee([1.0, 0.0, 0.0], [0.0; 3], 1.0, 1.0).repeat(4);
    let mut e2 = entry_ungridded(0x0102, 5, 4, 128, &p2);
    e2.flags = 4;
    let yee_bytes = seal_blocks(b2, vec![e2]);
    let mut tight = PortalEmfLimits::default();
    tight.display_cells = 1;
    assert_eq!(decode_emf_with_selection(&yee_bytes, None, tight).err(), Some(Error::Limit));
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn gpu_parity_full_spectrum_slice_evaluation_and_record64() {
    use crate::render::gpu::{
        emf_cells_from_loaded, emf_field_cell_from_record64, evaluate_display_slice_rgba8,
        full_spectrum_sigma_to_linear_rgb,
    };

    let freqs = [1.0e9_f64, 1.0e13, 5.45e14, 2.0e15, 2.4e18, 5.0e20];
    for f in freqs {
        let mut blocks = base_blocks(None);
        blocks.push(spectrum_axis_block(7, &[f * 0.8, f * 1.25]));
        blocks.push(ext_block(
            4,
            0x0103,
            crate::q42::emf_profiles::SPECTRUM_PROFILE,
            [1, 1, 1],
            [1, 1, 1],
            vec![tref(12, 7), tu32(19, 1)],
        ));
        blocks.push(desc_block(5, "bin_integrated_irradiance", "W m^-2", Some(4), None));
        let payload = rec_mode3([0.0; 3], 0, 7, &[1.5]);
        let bytes = seal_blocks(blocks, vec![entry_ungridded(0x0103, 5, 1, payload.len() as u32, &payload)]);
        let loaded = decode_emf_scientific(&bytes).unwrap();
        let gpu_cells = emf_cells_from_loaded(&loaded).unwrap();
        assert!(gpu_cells[0].amplitude > 0.0 && gpu_cells[0].epistemic_weight == 1.0);
        let rgb = full_spectrum_sigma_to_linear_rgb(gpu_cells[0].recurrence_frequency);
        assert!(rgb[0].max(rgb[1]).max(rgb[2]) > 0.05, "band f={f} blacked out");

        let [nx, ny, nz, nt] = loaded.dimensions();
        let rgba_chroma = evaluate_display_slice_rgba8(
            loaded.display_cells(),
            loaded.optical_cells(),
            nx, ny, nz, nt, 0, 0, 1.0, 0.0, 1.0, 0,
        )
        .unwrap();
        let rgba_gray = evaluate_display_slice_rgba8(
            loaded.display_cells(),
            loaded.optical_cells(),
            nx, ny, nz, nt, 0, 0, 1.0, 0.0, 1.0, 1,
        )
        .unwrap();
        assert!(rgba_chroma[3] > 0);
        assert_eq!(rgba_gray[0], rgba_gray[1]);
        assert_eq!(rgba_gray[1], rgba_gray[2]);
    }

    let mut rec = EmfCellRecord64::default();
    rec.e_field = [2.0, 0.0, 0.0];
    rec.wavelength_nm = 200.0; // UV (outside 400..700nm)
    rec.stokes_q = 0.6;
    rec.confidence_alpha = 1.0;
    let g = emf_field_cell_from_record64(&rec).unwrap();
    assert!((3.0..4.0).contains(&g.recurrence_frequency));
    assert!(g.stokes_q > 0.0);
}
