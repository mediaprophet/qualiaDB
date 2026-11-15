//! D06 DICOM Conformance Corpus — source-bound CT/MRI/PET/US reconstruction,
//! calibrated quantitative / spectroscopy / waveform inference, non-image identity,
//! and PatientImagePlane -> anatomy registration checks.
use crate::medical::dicom_dataset::{ParseLimits, Part10Dataset};
use crate::medical::dicom_frames::{FrameLimits, FrameSet, ValueUnits};
use crate::medical::dicom_geometry::{FrameGeometryInput, PatientImagePlane};
use crate::medical::reconstruction::*;
use crate::solvers::transforms::fourier::Cplx;
use core::f64::consts::PI;

#[test]
fn d06_corpus_quantitative_calibration_and_non_image_refusal_gates() {
    // Verified calibrated CT HU frame preserves source BLAKE3 and propagates uncertainty
    let ct_bytes = include_bytes!("../dicom_dataset/fixtures/enhanced-defined.dcm");
    let ct_ds = Part10Dataset::parse(ct_bytes, ParseLimits::default()).unwrap();
    let ct_frames = FrameSet::extract(&ct_ds, FrameLimits::default()).unwrap();
    let ct_tf = ct_frames.frames[0].quantitative().modality.unwrap();
    let mut cal = [0.0; 3];
    let mut unc = [0.0; 3];
    let prov = calibrate_and_infer_into(
        &[-1000.0, 0.0, 1000.0],
        &ct_tf,
        ct_ds.source_blake3(),
        1,
        NonVisualDomain::Spectroscopy { axis_values: &[1.0, 2.0, 3.0], axis_units: "ppm" },
        None,
        UncertaintyBudget { stored_measurement_std: 1.0, epistemic_std: 0.5, ..Default::default() },
        "d06_spec_hu",
        &mut cal,
        &mut unc,
    )
    .unwrap();
    assert_eq!(prov.units_description, "HU");
    assert_ne!(prov.derivation_blake3, ct_ds.source_blake3());
    assert!(prov.integral_trapezoid.unwrap().is_finite());

    // Swapped source BLAKE3 digest fails closed
    assert_eq!(
        calibrate_and_infer_into(
            &[0.0], &ct_tf, [0xFF; 32], 1, NonVisualDomain::ScalarSeries, None,
            UncertaintyBudget::default(), "swapped", &mut cal, &mut unc
        ).unwrap_err(),
        InferenceError::SourceIdentityMismatch
    );

    // Unspecified modality rescale units rejected; calibrated RWVM UCUM:mm accepted
    let rw_bytes = include_bytes!("../dicom_dataset/fixtures/native-u8-rwvm.dcm");
    let rw_ds = Part10Dataset::parse(rw_bytes, ParseLimits::default()).unwrap();
    let rw_frames = FrameSet::extract(&rw_ds, FrameLimits::default()).unwrap();
    let q = rw_frames.frames[0].quantitative();
    assert_eq!(
        calibrate_and_infer_into(
            &[128.0], &q.modality.unwrap(), rw_ds.source_blake3(), 1,
            NonVisualDomain::ScalarSeries, None, UncertaintyBudget::default(), "rw", &mut cal, &mut unc
        ).unwrap_err(),
        InferenceError::UnspecifiedUnits
    );
    let rw_tf = q.real_world.unwrap();
    assert_eq!(
        rw_tf.units(),
        ValueUnits::Code { scheme: "UCUM", value: "mm", meaning: Some("millimeter") }
    );
    calibrate_and_infer_into(
        &[128.0], &rw_tf, rw_ds.source_blake3(), 1,
        NonVisualDomain::ScalarSeries, None, UncertaintyBudget::default(), "rw_ok", &mut cal, &mut unc
    ).unwrap();
    assert!((cal[0] - 33.5).abs() < 1e-12);

    // Non-image DICOM carries zero image frames without inventing pixels
    let non_img = Part10Dataset::parse(include_bytes!("../dicom_dataset/fixtures/non-image.dcm"), ParseLimits::default()).unwrap();
    let non_f = FrameSet::extract(&non_img, FrameLimits::default()).unwrap();
    assert!(non_f.frames.is_empty());
    assert_eq!(non_f.identity.source_blake3, non_img.source_blake3());
}

#[test]
fn d06_corpus_multimodality_reconstruction_and_cross_frame_registration() {
    // MRI complex k-space + SoS coil combine parity
    let img: Vec<Cplx> = (0..16).map(|i| ((i as f64 + 1.0) * 0.5, -(i as f64) * 0.1)).collect();
    let ksp = {
        let mut out = vec![(0.0, 0.0); 16];
        // Forward 4x4 via fft_2d(false)
        let fwd = crate::medical::reconstruction::fft_2d(&img, 4, 4, false).unwrap();
        out.copy_from_slice(&fwd);
        out
    };
    let rec = reconstruct_mri_kspace_2d(&ksp, 4, 4, [0x77; 32], false, false, false).unwrap();
    let sos = combine_coils_sos(&[&rec.complex, &rec.complex], None).unwrap();
    for (i, &(re, im)) in img.iter().enumerate() {
        assert!((rec.complex[i].0 - re).abs() < 1e-10);
        assert!((rec.complex[i].1 - im).abs() < 1e-10);
        assert!((sos[i] - (2.0f64).sqrt() * re.hypot(im)).abs() < 1e-10);
    }

    // Cross-frame anatomical registration enforces certificate when FoR differs
    let plane = PatientImagePlane::from_frame_geometry(FrameGeometryInput {
        image_position_patient: Some([5.0, 10.0, 15.0]),
        image_orientation_patient: Some([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        pixel_spacing: Some([0.5, 0.5]),
        frame_of_reference_uid: Some("1.2.840.100.1"),
        anatomical_orientation_type: Some("BIPED"),
        invalid_source_fields: &[],
    }).unwrap();
    let affine = AnatomyTransform::Affine4x4 {
        matrix: [
            [1.0, 0.1, 0.0, -2.0],
            [0.0, 1.0, 0.0, 4.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
    assert_eq!(
        register_patient_to_anatomy(&plane, [0x88; 32], 1, 1, "1.2.840.100.2", &affine, None, 1e-6).unwrap_err(),
        AnatomyRegError::FrameOfReferenceMismatch
    );
    let cert = CrossFrameRegistrationCertificate {
        source_frame_of_reference_uid: "1.2.840.100.1",
        target_frame_of_reference_uid: "1.2.840.100.2",
        certificate_blake3: [0x99; 32],
    };
    let reg = register_patient_to_anatomy(&plane, [0x88; 32], 2, 4, "1.2.840.100.2", &affine, Some(&cert), 1e-6).unwrap();
    assert!(reg.roundtrip_residual_mm < 1e-12);
    assert_ne!(reg.derivation_blake3, [0x88; 32]);

    // Check CT parallel FBP invariance on empty/zero projection yields finite zero image
    let angles: Vec<f64> = (0..12).map(|k| PI * k as f64 / 12.0).collect();
    let sino = vec![0.0; 12 * 16];
    let mut fbp_out = vec![1.0; 64];
    reconstruct_ct_fbp_into(
        &SinogramView {
            source_blake3: [0x12; 32], n_angles: 12, n_detectors: 16, detector_spacing_mm: 1.0,
            center_detector_offset: 0.0, angles_rad: &angles, projections: &sino, units: ValueUnits::RescaleType("HU"),
        },
        &FbpConfig {
            image_width: 8, image_height: 8, pixel_spacing_mm: [1.0, 1.0],
            filter: FbpFilter::Hann, cutoff_ratio: 1.0, geometry: ProjectionGeometry::Parallel, apply_parker_short_scan: false,
        },
        &mut fbp_out,
    ).unwrap();
    assert!(fbp_out.iter().all(|&v| v == 0.0));
}
