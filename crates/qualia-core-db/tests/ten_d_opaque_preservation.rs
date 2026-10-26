//! Canonical public preservation API. All payloads here are synthetic byte
//! fixtures, not typed semantic admission or new production assets.
use qualia_core_db::container_10d::header::Container10dHeader;
use qualia_core_db::container_10d::integrity::{check_whole_file_crc32c, seal_whole_file_crc32c};
use qualia_core_db::container_10d::section::*;
mod opaque_preservation_support;
use opaque_preservation_support::*;

#[test]
fn unknown_sections_copy_exactly_and_strict_reader_still_refuses() {
    for version in [1, 2] {
        let mut data = fixture(version);
        descriptor_edit(&mut data, 2, |d| d.section_type = 230);
        let header = Container10dHeader::parse(&data).unwrap();
        assert!(matches!(
            parse_section_table(&data, &header),
            Err(SectionTableError::UndefinedSectionType { index: 2, got: 230 })
        ));
        // Retain flags, reserved metadata and tail exactly, without authority.
        header_edit(&mut data, |h| h.flags |= 0x8000);
        descriptor_edit(&mut data, 2, |d| d.reserved16 = 0xFACE);
        data.extend_from_slice(b"opaque-tail");
        seal_whole_file_crc32c(&mut data);
        let mut output = [0xA5; 1024];
        let n = copy_opaque_container_into(&data, budget(), &mut output).unwrap();
        assert_eq!(&output[..n], data);
        assert!(output[n..].iter().all(|b| *b == 0xA5));
        let fresh = output[..n].to_vec();
        let reopened = inspect_opaque_container(&fresh, budget()).unwrap();
        assert_eq!(reopened.header().version, version);
        assert_eq!(reopened.header().flags, 0x8001);
        assert_eq!(reopened.sections()[2].section_type, 230);
        assert_eq!(reopened.sections()[2].reserved16, 0xFACE);
        assert_eq!(reopened.payload(2), Some(b"mat00001".as_slice()));
        assert_eq!(reopened.payload(3), None);
        assert_eq!(reopened.bytes(), data);
    }
}

#[test]
fn known_replacement_retains_unrelated_bytes_offsets_versions_and_reopens() {
    for version in [1, 2] {
        let data = fixture(version);
        let before = inspect_opaque_container(&data, budget()).unwrap();
        let target = before.sections()[2];
        let mut output = [0xA5; 1024];
        let n = replace_known_section_bytes_preserving_layout(
            &data,
            SectionType::Materials,
            b"mat00002",
            budget(),
            &mut output,
        )
        .unwrap();
        let reopened = inspect_opaque_container(&output[..n], budget()).unwrap();
        assert_eq!(reopened.header().version, version);
        assert_eq!(reopened.payload(2), Some(b"mat00002".as_slice()));
        assert_eq!(reopened.sections()[..2], before.sections()[..2]);
        assert_eq!(reopened.payload(0), before.payload(0));
        assert_eq!(reopened.payload(1), before.payload(1));
        assert_eq!(reopened.sections()[2].byte_offset, target.byte_offset);
        check_whole_file_crc32c(&output[..n]).unwrap();
        parse_section_table(&output[..n], reopened.header()).unwrap();
        let crc_offset = 64 + 2 * 24 + 20;
        for i in 0..n {
            if !(52..56).contains(&i)
                && !(crc_offset..crc_offset + 4).contains(&i)
                && !(target.byte_offset as usize..target.byte_offset as usize + 8).contains(&i)
            {
                assert_eq!(output[i], data[i], "unrelated byte {i}");
            }
        }
        assert!(output[n..].iter().all(|b| *b == 0xA5));
    }
}

#[test]
fn unknown_dependency_refuses_change_but_permits_byte_identical_noop() {
    let data = unknown_fixture();
    let mut output = [0xA5; 1024];
    assert_eq!(
        replace_known_section_bytes_preserving_layout(
            &data,
            SectionType::QuantizedMesh,
            b"mesh0002",
            budget(),
            &mut output
        ),
        Err(OpaquePreservationError::UnresolvedDependencies { section_type: 230 })
    );
    assert_eq!(output, [0xA5; 1024]);
    let n = replace_known_section_bytes_preserving_layout(
        &data,
        SectionType::QuantizedMesh,
        b"mesh0001",
        budget(),
        &mut output,
    )
    .unwrap();
    assert_eq!(&output[..n], data);
}

#[test]
fn reserved_tags_flags_and_descriptor_metadata_are_not_edit_authority() {
    for kind in 0..3 {
        let mut data = fixture(2);
        let expected = match kind {
            0 => {
                header_edit(&mut data, |h| h.flags |= 0x8000);
                OpaquePreservationError::UnknownHeaderFlags { flags: 0x8000 }
            }
            1 => {
                descriptor_edit(&mut data, 1, |d| d.reserved16 = 1);
                OpaquePreservationError::ReservedDescriptorMetadata {
                    index: 1,
                    reserved16: 1,
                }
            }
            _ => {
                descriptor_edit(&mut data, 1, |d| d.section_type = 4);
                OpaquePreservationError::UnresolvedDependencies { section_type: 4 }
            }
        };
        let mut output = [0xA5; 1024];
        assert_eq!(
            replace_known_section_bytes_preserving_layout(
                &data,
                SectionType::Materials,
                b"mat00002",
                budget(),
                &mut output
            ),
            Err(expected)
        );
        assert_eq!(output, [0xA5; 1024]);
        copy_opaque_container_into(&data, budget(), &mut output).unwrap();
        assert_eq!(&output[..data.len()], data);
    }
}

#[test]
fn whole_crc_and_late_payload_crc_failure_leave_output_untouched() {
    let mut data = fixture(2);
    let offset = inspect_opaque_container(&data, budget())
        .unwrap()
        .sections()[2]
        .byte_offset as usize;
    data[offset] ^= 1;
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Integrity(_)
    ));
    seal_whole_file_crc32c(&mut data);
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::CrcMismatch { index: 2, .. })
    ));
    let mut output = [0xA5; 1024];
    assert!(replace_known_section_bytes_preserving_layout(
        &data,
        SectionType::QuantizedMesh,
        b"mesh0002",
        budget(),
        &mut output
    )
    .is_err());
    assert_eq!(output, [0xA5; 1024]);
}

#[test]
fn later_invalid_bounds_and_stride_product_refuse_before_output() {
    let mut data = unknown_fixture();
    descriptor_edit(&mut data, 2, |d| d.byte_offset = u32::MAX - 63);
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::OutOfBounds { index: 2, .. })
    ));
    let mut data = unknown_fixture();
    descriptor_edit(&mut data, 2, |d| {
        d.stride = u32::MAX;
        d.element_count = u32::MAX;
    });
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::StrideInconsistentDescriptor {
            index: 2,
            ..
        })
    ));
}

#[test]
fn padding_duplicates_zero_tags_alignment_and_overlaps_refuse() {
    let mut data = fixture(2);
    data[160] = 1; // Gap after the field and before the page-aligned material.
    seal_whole_file_crc32c(&mut data);
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::NonZeroPadding { .. })
    ));
    for kind in 0..5 {
        let mut data = fixture(2);
        descriptor_edit(&mut data, 2, |d| match kind {
            0 => d.section_type = 1,
            1 => d.section_type = 0,
            2 => d.alignment_tier = 255,
            3 => d.byte_offset += 1,
            _ => d.byte_offset = 64,
        });
        let error = copy_error_keeps_output(&data);
        assert!(match (kind, error) {
            (
                0,
                OpaquePreservationError::Section(SectionTableError::DuplicateSectionType {
                    ..
                }),
            ) => true,
            (
                1,
                OpaquePreservationError::Section(SectionTableError::UndefinedSectionType {
                    ..
                }),
            ) => true,
            (
                2,
                OpaquePreservationError::Section(SectionTableError::UndefinedAlignmentTier {
                    ..
                }),
            ) => true,
            (3, OpaquePreservationError::Section(SectionTableError::MisalignedSection { .. })) =>
                true,
            (
                4,
                OpaquePreservationError::Section(SectionTableError::SectionOverlapsMetadata {
                    ..
                }),
            ) => true,
            _ => false,
        });
    }
    let mut data = fixture(2);
    // Make a second descriptor share the first payload with a valid CRC.
    let first = inspect_opaque_container(&data, budget())
        .unwrap()
        .sections()[0];
    descriptor_edit(&mut data, 1, |d| {
        d.byte_offset = first.byte_offset;
        d.alignment_tier = first.alignment_tier;
        d.crc32c = first.crc32c;
    });
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::OverlappingSections { .. })
    ));
}

#[test]
fn unaligned_table_and_short_table_return_errors_without_panics() {
    let mut data = fixture(2);
    // Shift table one byte; payloads need not be consulted to refuse the borrow.
    data.copy_within(64..136, 65);
    header_edit(&mut data, |h| h.section_table_offset = 65);
    assert_eq!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::UnalignedSectionTable)
    );
    let mut data = fixture(2);
    data.truncate(100);
    seal_whole_file_crc32c(&mut data);
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::InputTooShort { .. })
    ));
}

#[test]
fn all_later_table_layout_bytes_and_empty_section_offsets_are_retained() {
    let original = unknown_fixture();
    let mut data = vec![0; 400];
    data[..original.len()].copy_from_slice(&original);
    data.copy_within(64..136, 256);
    data[64..136].fill(0);
    header_edit(&mut data, |h| h.section_table_offset = 256);
    let mut output = [0xA5; 1024];
    let n = copy_opaque_container_into(&data, budget(), &mut output).unwrap();
    assert_eq!(&output[..n], data);
    let mut data = fixture(2);
    // Empty payload occupies no bytes and may share metadata offset.
    descriptor_edit(&mut data, 2, |d| {
        d.byte_offset = 64;
        d.byte_length = 0;
        d.crc32c = 0;
    });
    // Old material payload is now opaque tail. Interior gap remains zero.
    copy_opaque_container_into(&data, budget(), &mut output).unwrap();
    assert_eq!(&output[..data.len()], data);
}

#[test]
fn unknown_outer_version_and_v1_mid2_refuse() {
    let mut data = fixture(2);
    header_edit(&mut data, |h| h.version = 3);
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Header(_)
    ));
    let mut data = fixture(1);
    descriptor_edit(&mut data, 2, |d| {
        d.section_type = SectionType::ManifoldIdentityV2 as u8
    });
    assert!(matches!(
        copy_error_keeps_output(&data),
        OpaquePreservationError::Section(SectionTableError::SectionRequiresVersion { .. })
    ));
}

#[test]
fn explicit_limits_and_output_capacity_and_length_refuse_transactionally() {
    let data = fixture(2);
    let mut output = [0xA5; 1024];
    assert!(matches!(
        copy_opaque_container_into(
            &data,
            OpaquePreservationBudget {
                max_bytes: data.len() - 1,
                ..budget()
            },
            &mut output
        ),
        Err(OpaquePreservationError::ByteBudget { .. })
    ));
    assert!(matches!(
        copy_opaque_container_into(
            &data,
            OpaquePreservationBudget {
                max_sections: 2,
                ..budget()
            },
            &mut output
        ),
        Err(OpaquePreservationError::SectionBudget { .. })
    ));
    assert_eq!(output, [0xA5; 1024]);
    let n = data.len() - 1;
    assert_eq!(
        copy_opaque_container_into(&data, budget(), &mut output[..n]),
        Err(OpaquePreservationError::OutputTooSmall {
            needed: data.len(),
            have: n
        })
    );
    assert!(matches!(
        replace_known_section_bytes_preserving_layout(
            &data,
            SectionType::Materials,
            b"mat00002",
            budget(),
            &mut output[..n]
        ),
        Err(OpaquePreservationError::OutputTooSmall { .. })
    ));
    assert!(matches!(
        replace_known_section_bytes_preserving_layout(
            &data,
            SectionType::Materials,
            b"longer material",
            budget(),
            &mut output
        ),
        Err(OpaquePreservationError::LengthChangeUnsupported { .. })
    ));
    assert!(matches!(
        replace_known_section_bytes_preserving_layout(
            &data,
            SectionType::Topology,
            b"",
            budget(),
            &mut output
        ),
        Err(OpaquePreservationError::MissingTarget { .. })
    ));
    assert!(matches!(
        replace_known_section_bytes_preserving_layout(
            &data,
            SectionType::SpecReservedGovernance,
            b"",
            budget(),
            &mut output
        ),
        Err(OpaquePreservationError::UnsupportedTarget { .. })
    ));
    assert_eq!(output, [0xA5; 1024]);
    let mut empty = [0; 64];
    Container10dHeader::proposed().encode(&mut empty);
    seal_whole_file_crc32c(&mut empty);
    assert_eq!(
        copy_opaque_container_into(
            &empty,
            OpaquePreservationBudget {
                max_bytes: 64,
                max_sections: 0
            },
            &mut output
        ),
        Ok(64)
    );
}

#[test]
fn measured_success_and_policy_failure_fixtures_allocate_zero() {
    let known = fixture(2);
    let unknown = unknown_fixture();
    let mut output = [0; 1024];
    ALLOCS.with(|c| c.set(Some(0)));
    let inspected = inspect_opaque_container(&unknown, budget());
    let copied = copy_opaque_container_into(&unknown, budget(), &mut output);
    let replaced = replace_known_section_bytes_preserving_layout(
        &known,
        SectionType::Materials,
        b"mat00002",
        budget(),
        &mut output,
    );
    let refused = replace_known_section_bytes_preserving_layout(
        &unknown,
        SectionType::QuantizedMesh,
        b"mesh0002",
        budget(),
        &mut output,
    );
    let count = ALLOCS.with(|c| c.replace(None).unwrap());
    assert!(inspected.is_ok() && copied.is_ok() && replaced.is_ok() && refused.is_err());
    assert_eq!(
        count, 0,
        "measured fixtures only; no universal failure-path allocation claim"
    );
}

#[test]
fn uninterpreted_tail_retains_exactly_but_refuses_changed_replacement() {
    let mut data = fixture(2);
    let at = data.len();
    data.extend_from_slice(b"future dependency");
    seal_whole_file_crc32c(&mut data);
    let mut output = [0xA5; 1024];
    assert_eq!(
        replace_known_section_bytes_preserving_layout(
            &data,
            SectionType::Materials,
            b"mat00002",
            budget(),
            &mut output
        ),
        Err(OpaquePreservationError::OpaqueTailBytes { at })
    );
    assert_eq!(output, [0xA5; 1024]);
    let n = replace_known_section_bytes_preserving_layout(
        &data,
        SectionType::Materials,
        b"mat00001",
        budget(),
        &mut output,
    )
    .unwrap();
    assert_eq!(&output[..n], data);
    data[at..].fill(0);
    seal_whole_file_crc32c(&mut data);
    let n = replace_known_section_bytes_preserving_layout(
        &data,
        SectionType::Materials,
        b"mat00002",
        budget(),
        &mut output,
    )
    .unwrap();
    assert_eq!(&output[at..n], &data[at..]);
}
