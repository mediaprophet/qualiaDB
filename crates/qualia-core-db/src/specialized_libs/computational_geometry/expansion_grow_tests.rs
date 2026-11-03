//! Focused canonical-grow regression; exact test arithmetic is not production.
use super::grow_expansion;
use crate::specialized_libs::computational_geometry::exact_test_helper::{
    expansion_to_exact, Exact,
};

#[test]
fn grow_carries_sum_and_emits_low_error_without_changing_return_shape() {
    let tiny = 2f64.powi(-60);
    let mut h = [0.; 3];
    let n = grow_expansion(&[1.], tiny, &mut h).unwrap();
    assert_eq!(n, 2);
    assert_eq!(&h[..n], &[tiny, 1.]);
    assert!(expansion_to_exact(&h[..n]).equals(&Exact::from_f64(1.).add(Exact::from_f64(tiny))));
    let mut cancelled = [0.; 4];
    let count = grow_expansion(&h[..n], -1., &mut cancelled).unwrap();
    assert_eq!(count, 3);
    assert!(expansion_to_exact(&cancelled[..count]).equals(&Exact::from_f64(tiny)));
    let nonzero: Vec<_> = cancelled[..count]
        .iter()
        .copied()
        .filter(|&v| v != 0.)
        .collect();
    assert_eq!(nonzero, [tiny]);
    assert_eq!(grow_expansion(&[], 3., &mut h).unwrap(), 1);
    assert_eq!(h[0], 3.);
}

#[test]
fn actual_equal_third_arm_expansions_cancel_exactly_after_grow() {
    let old = [
        vec![3.344251798829065e-20, 0.00017451949501120633],
        vec![
            -2.033832372292349e-21,
            2.0328790734103208e-19,
            0.002250128288766181,
        ],
        vec![
            7.88786872104171e-23,
            1.2281977735187355e-20,
            -0.00016940267167579796,
        ],
    ];
    let new = [
        vec![6.337463676153038e-21, 0.00017451949501120636],
        vec![
            -3.397664777837484e-22,
            2.0159384144652348e-19,
            0.002250128288766181,
        ],
        vec![
            2.593912800702333e-23,
            1.2334917294390749e-20,
            -0.00016940267167579796,
        ],
    ];
    for (a, b) in old.into_iter().zip(new) {
        assert!(expansion_to_exact(&a).equals(&expansion_to_exact(&b)));
        let mut acc = [0.; 16];
        let mut next = [0.; 16];
        let mut n = 1;
        for component in a.into_iter().chain(b.into_iter().map(|v| -v)) {
            let m = grow_expansion(&acc[..n], component, &mut next).unwrap();
            n = 0;
            for &v in &next[..m] {
                if v != 0. {
                    acc[n] = v;
                    n += 1;
                }
            }
            if n == 0 {
                acc[0] = 0.;
                n = 1;
            }
        }
        assert_eq!(&acc[..n], &[0.]);
    }
}
