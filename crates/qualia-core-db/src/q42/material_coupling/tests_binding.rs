//! Unit tests for `q42::material_coupling::binding` and M64 strata join contracts (Packet T01).

use crate::q42::m64_matter::{
    BankedConstituentLut, M64ChunkDirectoryEntry, M64Header, M64PayloadInterval, M64ReadLimits,
    M64StrataColumnRecord, M64_MAGIC, M64_MODE_STRATA,
};

use super::binding::{
    compute_blake3_256, reject_spatial_key_only_join, CoordinateTransform4x4,
    CoordinateTransformKind, HalfOpenValidityInterval, MaterialBinding, MaterialBindingError,
    MaterialEntryIdentity, RationalClock, RationalInstant,
};
use super::transduction::IqCarrierConvention;

fn build_valid_m64_strata_container() -> Vec<u8> {
    let mut rec = M64StrataColumnRecord::default();
    rec.update_crc();
    let mut header = M64Header::default();
    header.magic = M64_MAGIC;
    header.total_file_size = 192;
    header.chunk_count = 1;
    header.chunk_table_offset = 64;
    header.update_crc();

    let mut entry = M64ChunkDirectoryEntry {
        chunk_uuid: [1; 16],
        spatial_hilbert_index: 0x1234_5678,
        byte_offset: 128,
        byte_length: 64,
        record_count: 1,
        srid_geodetic_code: 0,
        lut_bank_epoch_id: 7,
        chunk_record_mode: M64_MODE_STRATA,
        compression_flags: 0,
        bao_root_hash_prefix: [0; 12],
        entry_crc32c: 0,
    };
    entry.update_crc();

    let mut buf = vec![0u8; 192];
    buf[..64].copy_from_slice(bytemuck::bytes_of(&header));
    buf[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
    buf[128..192].copy_from_slice(bytemuck::bytes_of(&rec));
    buf
}

#[test]
fn cryptographic_material_binding_epoch_frame_and_rational_clock_verification() {
    // 1. C02-C03 exact rational sample offset oracles:
    // 48 kHz sample 1 = 1/48000 s, sample 48000 = 1/1 s
    let clk_48k = RationalClock {
        clock_id: 10,
        scale: 0,
        epoch_seconds: 1_700_000_000,
        epoch_nanoseconds: 0,
        ticks_per_second_num: 96_000,
        ticks_per_second_den: 1,
    };
    let start_zero = RationalInstant::from_integral_tick(10, 0).unwrap();
    assert_eq!(
        start_zero
            .sample_offset_from_clock_epoch_seconds(&clk_48k, 1, 48_000, 1)
            .unwrap(),
        (1, 48_000)
    );
    assert_eq!(
        start_zero
            .sample_offset_from_clock_epoch_seconds(&clk_48k, 48_000, 48_000, 1)
            .unwrap(),
        (1, 1)
    );
    // Negative start tick (-2 ticks on 96 kHz clock = -1/48000 s) + sample 1 at 48 kHz = 0 s
    let start_neg = RationalInstant::from_integral_tick(10, -2).unwrap();
    assert_eq!(
        start_neg
            .sample_offset_from_clock_epoch_seconds(&clk_48k, 1, 48_000, 1)
            .unwrap(),
        (0, 1)
    );
    // 48000000/1001 Hz rate: sample 1001 starting at 1/48000000 s (1 tick at 48 MHz) = 1002001/48000000 s
    let clk_48m = RationalClock {
        clock_id: 11,
        scale: 0,
        epoch_seconds: 0,
        epoch_nanoseconds: 0,
        ticks_per_second_num: 48_000_000,
        ticks_per_second_den: 1,
    };
    let start_1tick = RationalInstant::from_integral_tick(11, 1).unwrap();
    assert_eq!(
        start_1tick
            .sample_offset_from_clock_epoch_seconds(&clk_48m, 1001, 48_000_000, 1001)
            .unwrap(),
        (1_002_001, 48_000_000)
    );

    // 2. Build valid .m64 container and BankedConstituentLut at epoch 7
    let m64_bytes = build_valid_m64_strata_container();
    let mut lut = BankedConstituentLut::new_with_terrestrial_defaults();
    lut.active_bank_1_epoch_id = 7;
    // Set explicit provenance DID hash on slot 3 (Quartz Sand) and update CRC
    lut.entry_mut(3).provenance_did_hash = 0xCAFE_BABE_1234_5678;
    lut.entry_mut(3).update_crc();

    let validity = HalfOpenValidityInterval::new(
        clk_48k,
        RationalInstant::from_integral_tick(10, 100).unwrap(),
        RationalInstant::from_integral_tick(10, 200).unwrap(),
    )
    .unwrap();

    // Proper rigid +90 deg Z rotation matrix in 4x4 homogeneous form
    let rigid_transform = CoordinateTransform4x4 {
        source_frame_id: 1,
        target_frame_id: 2,
        kind: CoordinateTransformKind::ProperRigid,
        matrix_row_major: [
            [0.0, -1.0, 0.0, 10.0],
            [1.0, 0.0, 0.0, -5.0],
            [0.0, 0.0, 1.0, 2.5],
            [0.0, 0.0, 0.0, 1.0],
        ],
        validity,
    };

    let binding = MaterialBinding {
        container_blake3: compute_blake3_256(&m64_bytes),
        material_table_blake3: compute_blake3_256(lut.as_gpu_bytes()),
        table_schema_revision: 1,
        table_epoch_id: 7,
        material_frame_id: 1,
        field_frame_id: 2,
        transform: rigid_transform,
        entries: vec![MaterialEntryIdentity {
            lut_slot_index: 3,
            canonical_q42_material_hash: lut.entry(3).canonical_q42_material_hash,
            provenance_did_hash: 0xCAFE_BABE_1234_5678,
            spatial_hilbert_key: 0x1234_5678,
        }],
    };

    let limits = M64ReadLimits {
        max_file_bytes: 1024,
        max_chunks: 4,
        max_records: 4,
    };
    let mut scratch = [M64PayloadInterval::default(); 4];

    // Valid admission at tick 100 (exact start boundary) and tick 199 (just before expiry)
    for valid_tick in [100, 150, 199] {
        let verified = binding
            .verify_and_resolve_entry(
                &m64_bytes,
                limits,
                &mut scratch,
                &lut,
                1,
                1,
                &clk_48k,
                &RationalInstant::from_integral_tick(10, valid_tick).unwrap(),
            )
            .expect("valid material binding");
        assert_eq!(verified.identity.lut_slot_index, 3);
        assert_eq!(verified.lut_entry.bulk_density_kg_m3, 2050);
    }

    // 3. Reject just before `from` (tick 99), exactly at `until` (tick 200), and after `until` (tick 201)
    for invalid_tick in [99, 200, 201] {
        assert_eq!(
            binding
                .verify_and_resolve_entry(
                    &m64_bytes,
                    limits,
                    &mut scratch,
                    &lut,
                    1,
                    1,
                    &clk_48k,
                    &RationalInstant::from_integral_tick(10, invalid_tick).unwrap(),
                )
                .err(),
            Some(MaterialBindingError::ValidityIntervalExpiredOrNotYetActive)
        );
    }

    // 4. Reject cross-origin clock even with identical tick and rate
    let mut other_origin_clock = clk_48k;
    other_origin_clock.epoch_seconds += 1;
    assert!(matches!(
        binding.verify_and_resolve_entry(
            &m64_bytes,
            limits,
            &mut scratch,
            &lut,
            1,
            1,
            &other_origin_clock,
            &RationalInstant::from_integral_tick(10, 150).unwrap(),
        ),
        Err(MaterialBindingError::ClockOriginMismatch { .. })
    ));

    // 5. Reject mismatched container BLAKE3 digest
    let mut bad_container_binding = binding.clone();
    bad_container_binding.container_blake3[0] ^= 0xFF;
    assert!(matches!(
        bad_container_binding.verify_and_resolve_entry(
            &m64_bytes,
            limits,
            &mut scratch,
            &lut,
            1,
            1,
            &clk_48k,
            &RationalInstant::from_integral_tick(10, 150).unwrap(),
        ),
        Err(MaterialBindingError::ContainerDigestMismatch { .. })
    ));

    // 6. Reject mismatched material table BLAKE3 digest
    let mut bad_table_binding = binding.clone();
    bad_table_binding.material_table_blake3[0] ^= 0xFF;
    assert!(matches!(
        bad_table_binding.verify_and_resolve_entry(
            &m64_bytes,
            limits,
            &mut scratch,
            &lut,
            1,
            1,
            &clk_48k,
            &RationalInstant::from_integral_tick(10, 150).unwrap(),
        ),
        Err(MaterialBindingError::MaterialTableDigestMismatch { .. })
    ));

    // 7. Reject mismatched Bank 1 epoch
    let mut bad_epoch_binding = binding.clone();
    bad_epoch_binding.table_epoch_id = 8;
    assert_eq!(
        bad_epoch_binding
            .verify_and_resolve_entry(
                &m64_bytes,
                limits,
                &mut scratch,
                &lut,
                1,
                1,
                &clk_48k,
                &RationalInstant::from_integral_tick(10, 150).unwrap(),
            )
            .err(),
        Some(MaterialBindingError::TableEpochMismatch {
            expected_epoch: 8,
            actual_epoch: 7,
        })
    );

    // 8. Reject improper reflection (det = -1), sheared matrix, and GeneralAffine/MovingFrame
    let mut reflection_binding = binding.clone();
    reflection_binding.transform.matrix_row_major[2][2] = -1.0;
    assert!(matches!(
        reflection_binding.verify_and_resolve_entry(
            &m64_bytes,
            limits,
            &mut scratch,
            &lut,
            1,
            1,
            &clk_48k,
            &RationalInstant::from_integral_tick(10, 150).unwrap(),
        ),
        Err(MaterialBindingError::ImproperReflectionRejected { .. })
    ));

    let mut sheared_binding = binding.clone();
    sheared_binding.transform.matrix_row_major[0][2] = 0.25;
    assert!(matches!(
        sheared_binding.verify_and_resolve_entry(
            &m64_bytes,
            limits,
            &mut scratch,
            &lut,
            1,
            1,
            &clk_48k,
            &RationalInstant::from_integral_tick(10, 150).unwrap(),
        ),
        Err(MaterialBindingError::NonOrthogonalOrShearedTransformRejected { .. })
    ));

    let mut affine_binding = binding.clone();
    affine_binding.transform.kind = CoordinateTransformKind::GeneralAffine;
    assert_eq!(
        affine_binding
            .verify_and_resolve_entry(
                &m64_bytes,
                limits,
                &mut scratch,
                &lut,
                1,
                1,
                &clk_48k,
                &RationalInstant::from_integral_tick(10, 150).unwrap(),
            )
            .err(),
        Some(MaterialBindingError::UnsupportedGeneralAffineInInitialSolver)
    );

    // 9. Reject mismatched canonical material hash even when spatial key and LUT row match
    let mut bad_hash_binding = binding.clone();
    bad_hash_binding.entries[0].canonical_q42_material_hash ^= 1;
    assert!(matches!(
        bad_hash_binding.verify_and_resolve_entry(
            &m64_bytes,
            limits,
            &mut scratch,
            &lut,
            1,
            1,
            &clk_48k,
            &RationalInstant::from_integral_tick(10, 150).unwrap(),
        ),
        Err(MaterialBindingError::CanonicalMaterialHashMismatch { .. })
    ));

    // 10. Reject spatial-key-only join
    assert_eq!(
        reject_spatial_key_only_join(0x1234_5678),
        Err(MaterialBindingError::SpatialKeyAloneInsufficient)
    );
}
