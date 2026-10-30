use super::*;

#[test]
fn nonfinite_values_never_pass() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for (expected, actual) in [(value, 1.0), (1.0, value), (value, value)] {
            let report = compare_f32(&[expected], &[actual], OracleTolerance::default());
            assert!(!report.passed());
            assert_eq!(report.mismatch_count, 1);
            assert_eq!(report.first_mismatch, Some(0));
            assert!(report.max_absolute_error.is_infinite());
        }
    }
}

#[test]
fn invalid_tolerances_fail_even_for_empty_inputs() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1] {
        for tolerance in [
            OracleTolerance {
                absolute: invalid,
                relative: 0.0,
            },
            OracleTolerance {
                absolute: 0.0,
                relative: invalid,
            },
        ] {
            let report = compare_f32(&[1.0], &[1.0], tolerance);
            assert!(!report.passed());
            assert_eq!(report.first_mismatch, Some(0));
            let empty = compare_f32(&[], &[], tolerance);
            assert!(!empty.passed());
            assert_eq!(empty.compared, 0);
            assert_eq!(empty.first_mismatch, None);
        }
    }
}

#[test]
fn finite_absolute_and_symmetric_relative_boundaries_are_preserved() {
    let exact = OracleTolerance {
        absolute: 0.0,
        relative: 0.0,
    };
    assert!(compare_f32(&[], &[], exact).passed());
    assert!(compare_f32(&[0.0, 1.0], &[0.0, 1.0], exact).passed());
    let absolute = OracleTolerance {
        absolute: 0.25,
        relative: 0.0,
    };
    assert!(compare_f32(&[0.0], &[0.25], absolute).passed());
    assert!(!compare_f32(&[0.0], &[0.5], absolute).passed());
    let relative = OracleTolerance {
        absolute: 0.0,
        relative: 0.25,
    };
    assert!(compare_f32(&[3.0], &[4.0], relative).passed());
    assert!(compare_f32(&[4.0], &[3.0], relative).passed());
    assert!(!compare_f32(&[3.0], &[5.0], relative).passed());
    assert!(!compare_f32(&[1.0], &[], exact).passed());
}
