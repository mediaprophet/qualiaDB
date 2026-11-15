use super::super::field_envelope::{
    Block, BlockInput, EntryInput, EnvelopeInput, Family, FieldInput, FieldValue as V,
    Limits as StoreLimits, MetadataInput, StructuralView,
};
use super::super::field_semantics::{Error as SemError, NodeState};
use super::*;

fn fi<'a>(key: u16, critical: bool, value: V<'a>) -> FieldInput<'a> {
    FieldInput {
        key,
        flags: u8::from(critical),
        value,
    }
}

fn encode_env(blocks: &[BlockInput<'_>], entries: &[EntryInput<'_>]) -> Vec<u8> {
    let m_in = MetadataInput { blocks };
    let m_len = m_in.encoded_len(StoreLimits::default()).unwrap();
    let mut meta = vec![0u8; m_len];
    let meta = m_in.encode_into(&mut meta, StoreLimits::default()).unwrap();
    let e_in = EnvelopeInput {
        family: Family::EmfV4Candidate,
        flags: 0,
        metadata: meta,
        entries,
    };
    let e_len = e_in.encoded_len(StoreLimits::default()).unwrap();
    let mut out = vec![0u8; e_len];
    e_in.encode_into(&mut out, StoreLimits::default()).unwrap();
    out
}

fn admit_with_extra(extra_blocks: &[BlockInput<'_>]) -> Result<(), Error> {
    let mut payload = [0u8; 128];
    payload[0..8].copy_from_slice(&1.0f64.to_le_bytes());
    payload[32..40].copy_from_slice(&(1.0 / super::yee::SPEED_OF_LIGHT_M_PER_S).to_le_bytes());

    let f_clk = [
        fi(1, true, V::U32(&[0])),
        fi(2, true, V::I64(&[0])),
        fi(3, true, V::U32(&[0])),
        fi(4, true, V::U64(&[1000])),
        fi(5, true, V::U64(&[1])),
    ];
    let f_frm = [
        fi(1, true, V::U32(&[0])),
        fi(3, true, V::F64(&[0.0, 0.0, 0.0])),
        fi(4, true, V::F64(&[0.0, 0.0, 0.0, 1.0])),
        fi(8, true, V::Utf8("right-handed-X-forward-Y-left-Z-up")),
    ];
    let f_prv = [fi(1, true, V::U32(&[0]))];
    let f_ext = [
        fi(1, true, V::U32(&[0])),
        fi(2, true, V::U32(&[1])),
        fi(3, true, V::MetadataRefs(&[1])),
        fi(4, true, V::MetadataRefs(&[2])),
        fi(5, true, V::MetadataRefs(&[3])),
        fi(6, true, V::Utf8(FIELD_PROFILE)),
        fi(7, true, V::Utf8("linear")),
        fi(8, true, V::U64(&[1, 1, 1])),
        fi(9, true, V::U32(&[1, 1, 1])),
        fi(10, true, V::U32(&[0])),
        fi(11, true, V::I64(&[0])),
    ];
    let f_dsc = [
        fi(2, false, V::MetadataRefs(&[2])),
        fi(3, true, V::MetadataRefs(&[3])),
        fi(7, true, V::Utf8("electromagnetic_field_snapshot")),
        fi(8, true, V::Utf8("mixed:mode-registry")),
        fi(12, true, V::Utf8(FIELD_MODEL_URI)),
        fi(18, true, V::MetadataRefs(&[4])),
    ];
    let mut blocks = vec![
        BlockInput { id: 1, schema_type: 1, revision: 1, flags: 1, fields: &f_clk },
        BlockInput { id: 2, schema_type: 2, revision: 1, flags: 1, fields: &f_frm },
        BlockInput { id: 3, schema_type: 4, revision: 1, flags: 1, fields: &f_prv },
        BlockInput { id: 4, schema_type: 103, revision: 1, flags: 1, fields: &f_ext },
        BlockInput { id: 5, schema_type: 3, revision: 1, flags: 1, fields: &f_dsc },
    ];
    blocks.extend_from_slice(extra_blocks);
    let entry = [EntryInput {
        mode: 0,
        revision: 1,
        flags: 0,
        stream_id: 0,
        frame_id: 2,
        start_tick: 0,
        time_count: 0,
        spatial_key: 0,
        record_count: 1,
        stride: 128,
        codec: 0,
        descriptor_id: 5,
        payload: &payload,
    }];
    let bytes = encode_env(&blocks, &entry);
    let view = StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch {
        metadata: &mut md,
        entries: &mut ed,
    };
    admit_chunk(&view, 1, &mut scratch, Limits::default()).map(|_| ())
}

fn find_block<'a>(bytes: &'a [u8], id: u32) -> (StructuralView<'a>, Block<'a>) {
    let view = StructuralView::open(bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    for b in view.blocks() {
        if b.id() == id {
            return (view, b);
        }
    }
    panic!("missing block {id}");
}

#[test]
fn frame_pose_quaternion_validation_normalization_and_tensor_rotations() {
    let raw = FramePose {
        origin_m: [1.0, -2.0, 3.0],
        rotation_xyzw: [0.0, 0.0, 1.0 + 2.0e-13, 0.0],
    }
    .validated()
    .unwrap();
    let nsq: f64 = raw.rotation_xyzw.iter().map(|v| v * v).sum();
    assert!((nsq - 1.0).abs() < 1.0e-15);

    assert_eq!(
        FramePose {
            origin_m: [0.0; 3],
            rotation_xyzw: [0.0, 0.0, 1.0 + 1.0e-11, 0.0],
        }
        .validated(),
        Err(Error::Frame)
    );
    assert_eq!(
        FramePose {
            origin_m: [f64::NAN, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
        .validated(),
        Err(Error::NonFinite)
    );

    // 90 deg active rotation around +Z moves +X -> +Y and translates by origin.
    let s2 = std::f64::consts::FRAC_1_SQRT_2;
    let pose90 = FramePose {
        origin_m: [10.0, 20.0, 30.0],
        rotation_xyzw: [0.0, 0.0, s2, s2],
    }
    .validated()
    .unwrap();
    let p_out = pose90.transform_point([2.0, 0.0, -5.0]);
    assert!((p_out[0] - 10.0).abs() < 1.0e-12);
    assert!((p_out[1] - 22.0).abs() < 1.0e-12);
    assert!((p_out[2] - 25.0).abs() < 1.0e-12);

    // Non-trivial quaternion checks R^T R = I and Cauchy/Voigt6 stress trace invariance.
    let q = FramePose {
        origin_m: [0.0; 3],
        rotation_xyzw: [0.1, -0.2, 0.3, (1.0f64 - 0.14).sqrt()],
    }
    .validated()
    .unwrap();
    let r = q.rotation_matrix();
    let rt_v = q.inverse_transform_vector(q.transform_vector([1.2, -3.4, 5.6]));
    assert!((rt_v[0] - 1.2).abs() < 1.0e-12);
    assert!((rt_v[1] + 3.4).abs() < 1.0e-12);
    assert!((rt_v[2] - 5.6).abs() < 1.0e-12);
    assert!((det_3x3(r) - 1.0).abs() < 1.0e-12);

    let voigt = [5.0, -2.0, 1.5, 0.8, -1.1, 2.3];
    let voigt_rot = q.transform_voigt6(voigt);
    let trace_in = voigt[0] + voigt[1] + voigt[2];
    let trace_out = voigt_rot[0] + voigt_rot[1] + voigt_rot[2];
    assert!((trace_in - trace_out).abs() < 1.0e-12);
    let full = [
        [voigt[0], voigt[5], voigt[4]],
        [voigt[5], voigt[1], voigt[3]],
        [voigt[4], voigt[3], voigt[2]],
    ];
    let full_rot = q.transform_stress_tensor(full);
    assert!((full_rot[0][1] - voigt_rot[5]).abs() < 1.0e-12);
    assert!((full_rot[1][2] - voigt_rot[3]).abs() < 1.0e-12);
}

#[test]
fn schema_106_rigid_vs_affine_transforms_and_covector_dual_invariance() {
    let c = std::f64::consts::FRAC_PI_6.cos();
    let s = std::f64::consts::FRAC_PI_6.sin();
    let rigid_mat = [
        c, -s, 0.0, 1.5,
        s, c, 0.0, -2.0,
        0.0, 0.0, 1.0, 4.0,
        0.0, 0.0, 0.0, 1.0,
    ];
    let f_rigid = [
        fi(1, true, V::MetadataRefs(&[2])),
        fi(2, true, V::MetadataRefs(&[2])),
        fi(3, true, V::Utf8("rigid_matrix4")),
        fi(4, true, V::F64(&rigid_mat)),
    ];
    let bytes = encode_env(
        &[BlockInput { id: 10, schema_type: 106, revision: 1, flags: 1, fields: &f_rigid }],
        &[],
    );
    let (_v, blk) = find_block(&bytes, 10);
    let tf = Schema106Transform::from_block(blk).unwrap();
    let tp = tf.transform_point([1.0, 0.0, -1.0]);
    assert!((tp[0] - (c + 1.5)).abs() < 1.0e-12);
    assert!((tp[1] - (s - 2.0)).abs() < 1.0e-12);
    assert!((tp[2] - 3.0).abs() < 1.0e-12);

    // Reject improper reflection (det = -1) labeled as rigid_matrix4.
    let refl = [
        -1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];
    let f_refl = [
        fi(1, true, V::MetadataRefs(&[2])),
        fi(2, true, V::MetadataRefs(&[2])),
        fi(3, true, V::Utf8("rigid_matrix4")),
        fi(4, true, V::F64(&refl)),
    ];
    let refl_bytes = encode_env(
        &[BlockInput { id: 10, schema_type: 106, revision: 1, flags: 1, fields: &f_refl }],
        &[],
    );
    let (_v, rblk) = find_block(&refl_bytes, 10);
    assert_eq!(Schema106Transform::from_block(rblk), Err(Error::Frame));

    // Affine shear + stretch preserves dual pairing <w', M_lin v> == <w, v>.
    let affine_mat = [
        2.0, 0.5, 0.0, 1.0,
        0.0, 1.5, -0.3, 2.0,
        0.0, 0.0, 0.8, 3.0,
        0.0, 0.0, 0.0, 1.0,
    ];
    let f_aff = [
        fi(1, true, V::MetadataRefs(&[2])),
        fi(2, true, V::MetadataRefs(&[2])),
        fi(3, true, V::Utf8("affine_matrix4")),
        fi(4, true, V::F64(&affine_mat)),
    ];
    let aff_bytes = encode_env(
        &[BlockInput { id: 11, schema_type: 106, revision: 1, flags: 1, fields: &f_aff }],
        &[],
    );
    let (_v, ablk) = find_block(&aff_bytes, 11);
    let atf = Schema106Transform::from_block(ablk).unwrap();
    let v_vec = [1.3, -0.7, 2.1];
    let w_cov = [0.4, 1.8, -0.9];
    let w_prime = atf.transform_covector(w_cov).unwrap();
    let lp = atf.linear_part();
    let mv = [
        lp[0][0] * v_vec[0] + lp[0][1] * v_vec[1] + lp[0][2] * v_vec[2],
        lp[1][0] * v_vec[0] + lp[1][1] * v_vec[1] + lp[1][2] * v_vec[2],
        lp[2][0] * v_vec[0] + lp[2][1] * v_vec[1] + lp[2][2] * v_vec[2],
    ];
    let pair_in = w_cov[0] * v_vec[0] + w_cov[1] * v_vec[1] + w_cov[2] * v_vec[2];
    let pair_out = w_prime[0] * mv[0] + w_prime[1] * mv[1] + w_prime[2] * mv[2];
    assert!((pair_in - pair_out).abs() < 1.0e-12);
    let inv = invert_3x3(lp).unwrap();
    assert!((det_3x3(lp) * det_3x3(inv) - 1.0).abs() < 1.0e-12);

    // Singular affine matrix & non-affine projective row must reject.
    let bad_proj = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.1, 1.0,
    ];
    let f_proj = [
        fi(1, true, V::MetadataRefs(&[2])),
        fi(2, true, V::MetadataRefs(&[2])),
        fi(3, true, V::Utf8("affine_matrix4")),
        fi(4, true, V::F64(&bad_proj)),
    ];
    assert_eq!(
        admit_with_extra(&[BlockInput { id: 12, schema_type: 106, revision: 1, flags: 1, fields: &f_proj }]),
        Err(Error::Shared(SemError::Frame))
    );
}

#[test]
fn calibration_half_open_tick_window_and_validation() {
    let digest = [0x11u8; 32];
    let f_cal = [
        fi(1, true, V::Utf8("probe-rx-1")),
        fi(2, true, V::Bytes(&digest)),
        fi(3, true, V::Utf8("V")),
        fi(4, true, V::F64(&[2.5])),
        fi(5, true, V::F64(&[-0.25])),
        fi(6, false, V::F64(&[0.01])),
        fi(7, false, V::Utf8("k=1_standard")),
        fi(8, false, V::I64(&[100, 200])),
        fi(9, false, V::MetadataRefs(&[1])),
    ];
    let cal_block = BlockInput { id: 10, schema_type: 107, revision: 1, flags: 1, fields: &f_cal };
    admit_with_extra(&[cal_block]).unwrap();

    let raw_bytes = encode_env(&[cal_block], &[]);
    let (_v, blk) = find_block(&raw_bytes, 10);
    assert!((evaluate_calibration(blk, 4.0, Some(100)).unwrap() - 9.75).abs() < 1.0e-12);
    assert!((evaluate_calibration(blk, 4.0, Some(199)).unwrap() - 9.75).abs() < 1.0e-12);
    assert_eq!(evaluate_calibration(blk, 4.0, Some(99)), Err(Error::Time));
    assert_eq!(evaluate_calibration(blk, 4.0, Some(200)), Err(Error::Time));
    assert_eq!(evaluate_calibration(blk, 4.0, None), Err(Error::Time));
    assert_eq!(evaluate_calibration(blk, f64::NAN, Some(150)), Err(Error::NonFinite));

    // Inverted validity window and negative scale/uncertainty fail closed in registry.
    let f_bad_win = [
        fi(1, true, V::Utf8("probe-rx-1")),
        fi(2, true, V::Bytes(&digest)),
        fi(3, true, V::Utf8("V")),
        fi(4, true, V::F64(&[1.0])),
        fi(5, true, V::F64(&[0.0])),
        fi(8, false, V::I64(&[200, 200])),
        fi(9, false, V::MetadataRefs(&[1])),
    ];
    assert_eq!(
        admit_with_extra(&[BlockInput { id: 10, schema_type: 107, revision: 1, flags: 1, fields: &f_bad_win }]),
        Err(Error::Shared(SemError::Time))
    );
    let f_bad_scale = [
        fi(1, true, V::Utf8("probe-rx-1")),
        fi(2, true, V::Bytes(&digest)),
        fi(3, true, V::Utf8("V")),
        fi(4, true, V::F64(&[0.0])),
        fi(5, true, V::F64(&[0.0])),
    ];
    assert_eq!(
        admit_with_extra(&[BlockInput { id: 10, schema_type: 107, revision: 1, flags: 1, fields: &f_bad_scale }]),
        Err(Error::Shared(SemError::Value))
    );
}

#[test]
fn constitutive_models_material_binding_acquisition_and_bio_context_rules() {
    // 1. Constitutive Schema 133: vacuum strictness, Debye & Lorentz validation.
    let f_vac_ok = [
        fi(1, true, V::Utf8("vacuum")),
        fi(2, true, V::F64(&[1.0])),
        fi(3, true, V::F64(&[1.0])),
        fi(4, true, V::F64(&[0.0])),
    ];
    admit_with_extra(&[BlockInput { id: 10, schema_type: 133, revision: 1, flags: 1, fields: &f_vac_ok }]).unwrap();

    let f_vac_bad = [
        fi(1, true, V::Utf8("vacuum")),
        fi(2, true, V::F64(&[1.01])),
        fi(3, true, V::F64(&[1.0])),
        fi(4, true, V::F64(&[0.0])),
    ];
    assert_eq!(
        admit_with_extra(&[BlockInput { id: 10, schema_type: 133, revision: 1, flags: 1, fields: &f_vac_bad }]),
        Err(Error::Shared(SemError::Value))
    );

    let f_deb_bad_tau = [
        fi(1, true, V::Utf8("debye_isotropic")),
        fi(2, true, V::F64(&[2.0])),
        fi(3, true, V::F64(&[1.0])),
        fi(4, true, V::F64(&[0.0])),
        fi(5, false, V::F64(&[1.5])),
        fi(6, false, V::F64(&[0.0])),
    ];
    assert_eq!(
        admit_with_extra(&[BlockInput { id: 10, schema_type: 133, revision: 1, flags: 1, fields: &f_deb_bad_tau }]),
        Err(Error::Shared(SemError::Value))
    );

    let f_lor_ok = [
        fi(1, true, V::Utf8("lorentz_isotropic")),
        fi(2, true, V::F64(&[2.25])),
        fi(3, true, V::F64(&[1.0])),
        fi(4, true, V::F64(&[0.01])),
        fi(5, false, V::F64(&[0.4])),
        fi(7, false, V::F64(&[3.0e15])),
        fi(8, false, V::F64(&[1.0e13])),
    ];
    admit_with_extra(&[BlockInput { id: 10, schema_type: 133, revision: 1, flags: 1, fields: &f_lor_ok }]).unwrap();

    // 2. MaterialBinding Schema 105: duplicate IDs and unsupported hash fail.
    let d1 = [0xaa; 32];
    let d2 = [0xbb; 32];
    let f_mat_dup = [
        fi(1, true, V::Bytes(&d1)),
        fi(2, true, V::Utf8("BLAKE3-256")),
        fi(3, true, V::Bytes(&d2)),
        fi(4, true, V::U64(&[1])),
        fi(5, true, V::MetadataRefs(&[2])),
        fi(8, true, V::Utf8("v1")),
        fi(9, true, V::U64(&[42, 7, 42])),
    ];
    assert_eq!(
        admit_with_extra(&[BlockInput { id: 10, schema_type: 105, revision: 1, flags: 1, fields: &f_mat_dup }]),
        Err(Error::Shared(SemError::Value))
    );

    // 3. Acquisition Schema 136: complex_stft requires window, fft_size, hop, normalization.
    let f_cal = [
        fi(1, true, V::Utf8("sdr-0")),
        fi(2, true, V::Bytes(&d1)),
        fi(3, true, V::Utf8("V")),
        fi(4, true, V::F64(&[1.0])),
        fi(5, true, V::F64(&[0.0])),
    ];
    let f_acq_missing_stft = [
        fi(1, true, V::MetadataRefs(&[1])),
        fi(2, true, V::U64(&[4000])),
        fi(3, true, V::U64(&[1])),
        fi(4, true, V::Utf8("complex_stft")),
        fi(5, true, V::Utf8("V")),
        fi(6, true, V::MetadataRefs(&[10])),
        fi(7, true, V::Utf8("I_plus_iQ_exp_minus_iwt")),
    ];
    assert_eq!(
        admit_with_extra(&[
            BlockInput { id: 10, schema_type: 107, revision: 1, flags: 1, fields: &f_cal },
            BlockInput { id: 11, schema_type: 136, revision: 1, flags: 1, fields: &f_acq_missing_stft },
        ]),
        Err(Error::Shared(SemError::Value))
    );

    let f_acq_bad_conv = [
        fi(1, true, V::MetadataRefs(&[1])),
        fi(2, true, V::U64(&[4000])),
        fi(3, true, V::U64(&[1])),
        fi(4, true, V::Utf8("complex_baseband")),
        fi(5, true, V::Utf8("V")),
        fi(6, true, V::MetadataRefs(&[10])),
        fi(7, true, V::Utf8("I_minus_iQ_exp_plus_iwt")),
    ];
    assert_eq!(
        admit_with_extra(&[
            BlockInput { id: 10, schema_type: 107, revision: 1, flags: 1, fields: &f_cal },
            BlockInput { id: 11, schema_type: 136, revision: 1, flags: 1, fields: &f_acq_bad_conv },
        ]),
        Err(Error::UnsupportedProfile)
    );

    // 4. BioContext Schema 137: consent provenance schema 4 must carry non-zero 32B key 6 receipt.
    let zero_receipt = [0u8; 32];
    let f_consent_bad = [
        fi(1, true, V::U32(&[4])),
        fi(6, false, V::Bytes(&zero_receipt)),
    ];
    let f_bio = [
        fi(1, true, V::Utf8("FMA-5.0")),
        fi(2, true, V::Utf8("FMA:50801")),
        fi(3, true, V::Utf8("current_dipole")),
        fi(4, true, V::MetadataRefs(&[3])),
        fi(7, false, V::MetadataRefs(&[10])),
    ];
    assert_eq!(
        admit_with_extra(&[
            BlockInput { id: 10, schema_type: 4, revision: 1, flags: 1, fields: &f_consent_bad },
            BlockInput { id: 11, schema_type: 137, revision: 1, flags: 1, fields: &f_bio },
        ]),
        Err(Error::Shared(SemError::Value))
    );

    let good_receipt = [0x5au8; 32];
    let f_consent_ok = [
        fi(1, true, V::U32(&[4])),
        fi(6, false, V::Bytes(&good_receipt)),
    ];
    admit_with_extra(&[
        BlockInput { id: 10, schema_type: 4, revision: 1, flags: 1, fields: &f_consent_ok },
        BlockInput { id: 11, schema_type: 137, revision: 1, flags: 1, fields: &f_bio },
    ])
    .unwrap();
}
