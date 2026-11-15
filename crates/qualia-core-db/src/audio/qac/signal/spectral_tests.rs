use super::*;
use crate::q42::{
    field_envelope::{self as env, Family, StructuralView},
    field_semantics as sem,
};
use super::test_support::*;
#[test]
fn imaginary_phase_bin_order_and_negative_nyquist_are_explicit() {
    let original = include_bytes!("fixtures/stft-n4-imaginary.le");
    let bytes = build(
        original,
        Config {
            spectral: true,
            scalar: 6,
            ..Default::default()
        },
        |_| {},
    );
    let view =
        StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
    let mut m = [sem::NodeState::default(); 16];
    let mut e = m;
    let signal = admit(
        &view,
        1,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        sem::Limits::default(),
        Limits::default(),
    )
    .unwrap();
    for (bin, imaginary) in [0f32, -2., 0., 2.].into_iter().enumerate() {
        let Sample::ComplexF32 { real, imag } = signal.sample(0, 0, bin as u32).unwrap() else {
            panic!()
        };
        assert_eq!(real.to_bits(), 0);
        assert_eq!(imag.to_bits(), imaginary.to_bits());
    }
    let transform = signal.transform().unwrap();
    for (bin, hz) in [0i128, 1, -2, -1].into_iter().enumerate() {
        let f = transform.frequency_offset(bin as u32).unwrap();
        assert_eq!((f.numerator, f.denominator), (hz, 1));
    }
    assert!(transform.frequency_offset(4).is_err());
    assert_eq!(signal.as_bytes(), original);
}
#[test]
fn nonfinite_complex_lane_and_negative_magnitude_reject() {
    let mut complex = include_bytes!("fixtures/stft-n4-imaginary.le").to_vec();
    complex[4..8].copy_from_slice(&f32::INFINITY.to_le_bytes());
    let magnitude = [0f32, -2., 0., 2.]
        .iter()
        .flat_map(|x| x.to_le_bytes())
        .collect::<Vec<_>>();
    for (payload, scalar) in [(&complex[..], 6), (&magnitude[..], 1)] {
        let bytes = build(
            payload,
            Config {
                spectral: true,
                scalar,
                ..Default::default()
            },
            |_| {},
        );
        let view =
            StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
        let mut m = [sem::NodeState::default(); 16];
        let mut e = m;
        assert!(admit(
            &view,
            1,
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e
            },
            sem::Limits::default(),
            Limits::default()
        )
        .is_err());
    }
}
