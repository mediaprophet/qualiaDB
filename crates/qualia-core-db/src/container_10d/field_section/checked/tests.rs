use super::*;
use crate::container_10d::field_section::{
    encode_field_section_with_rasters, ContinuousRasterField,
};
fn fixture() -> Vec<u8> {
    let mut out = vec![0; 256];
    let n = encode_field_section_with_rasters(
        &[ContinuousRasterField {
            name_hash: 17,
            unit_hash: 23,
            width: 2,
            height: 2,
            samples: &[1.0, -3.0, 2.5, 9.0],
        }],
        &mut out,
    )
    .unwrap();
    out.truncate(n);
    out
}
fn limits() -> FieldInspectionLimits {
    FieldInspectionLimits {
        max_payload_bytes: 256,
        max_fields: 8,
        max_samples: 16,
    }
}
#[test]
fn checked_fiel_grid_unaligned_and_declarations_are_borrowed() {
    let bytes = fixture();
    assert_eq!(
        inspect_field_section(&bytes, limits()).unwrap(),
        FieldInspection {
            fields: 1,
            grid_fields: 1,
            samples: 4,
            type_mask: 1
        }
    );
    let mut unaligned = vec![0];
    unaligned.extend_from_slice(&bytes);
    assert_eq!(
        inspect_field_section(&unaligned[1..], limits()),
        inspect_field_section(&bytes, limits())
    );
    let mut declaration = bytes[..44].to_vec();
    declaration[36..44].fill(0);
    assert_eq!(
        inspect_field_section(&declaration, limits())
            .unwrap()
            .grid_fields,
        0
    );
}
#[test]
fn checked_fiel_refuses_reserved_shape_offsets_nonfinite_and_tail() {
    let bytes = fixture();
    for (offset, value) in [(8, 1), (31, 1), (32, 1), (36, 43), (44, 3), (56, 1)] {
        let mut corrupt = bytes.clone();
        corrupt[offset] = value;
        assert!(
            inspect_field_section(&corrupt, limits()).is_err(),
            "offset {offset}"
        );
    }
    let mut corrupt = bytes.clone();
    corrupt[60..64].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(matches!(
        inspect_field_section(&corrupt, limits()),
        Err(FieldInspectionError::NonFinite { .. })
    ));
    let mut tail = bytes.clone();
    tail.push(0);
    assert_eq!(
        inspect_field_section(&tail, limits()),
        Err(FieldInspectionError::UnclaimedBytes)
    );
    for n in 0..bytes.len() {
        assert!(inspect_field_section(&bytes[..n], limits()).is_err());
    }
}
#[test]
fn checked_fiel_budgets_precede_large_or_late_payload_work() {
    let bytes = fixture();
    for l in [
        FieldInspectionLimits {
            max_payload_bytes: bytes.len() - 1,
            ..limits()
        },
        FieldInspectionLimits {
            max_fields: 0,
            ..limits()
        },
        FieldInspectionLimits {
            max_samples: 3,
            ..limits()
        },
    ] {
        assert_eq!(
            inspect_field_section(&bytes, l),
            Err(FieldInspectionError::Budget)
        );
    }
    let mut huge = bytes;
    huge[52..56].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(inspect_field_section(&huge, limits()).is_err());
}
