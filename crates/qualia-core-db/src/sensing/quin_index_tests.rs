use super::validate_measurement_quin_parity;
use crate::{frame_layout, NQuin};

fn quin(metadata: u64, parity: u64) -> NQuin {
    NQuin {
        subject: 11,
        predicate: 0x1234,
        object: 22,
        context: 33,
        metadata,
        parity,
    }
}

#[test]
fn zero_metadata_makes_the_two_parity_conventions_agree() {
    let mut value = quin(0, 0);
    value.recalculate_parity();
    let result = validate_measurement_quin_parity(&value);
    assert!(result.nquin_ecc_valid());
    assert!(result.frame_layout_valid());
    assert!(result.conventions_agree_valid());
}

#[test]
fn nonzero_metadata_agrees_under_canonical_five_field_fold() {
    let parity = NQuin::calculate_parity(11, 0x1234, 22, 33, 0xfeed);
    assert_eq!(parity, frame_layout::parity(11, 0x1234, 22, 33, 0xfeed));
    let result = validate_measurement_quin_parity(&quin(0xfeed, parity));
    assert!(result.nquin_ecc_valid());
    assert!(result.frame_layout_valid());
    assert!(result.conventions_agree_valid());
}

#[test]
fn stale_four_field_fold_is_rejected_for_nonzero_metadata() {
    let stale_four_field = 11u64 ^ 0x1234u64 ^ 22u64 ^ 33u64;
    let value = quin(0xfeed, stale_four_field);
    let result = validate_measurement_quin_parity(&value);
    assert!(!result.nquin_ecc_valid());
    assert!(!result.frame_layout_valid());
    assert!(!result.conventions_agree_valid());
}
