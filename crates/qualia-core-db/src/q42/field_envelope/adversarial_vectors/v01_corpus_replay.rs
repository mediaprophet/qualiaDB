use crate::q42::field_envelope::{Error, Family, Limits, StructuralView};

const EMF_0100: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/emf-v4-mode-0x0100.emf"
);
const EMF_0107: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/emf-v4-mode-0x0107.emf"
);
const EMF_EDGE: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/emf-v4-edge-neg-zero-subnormal-dag32-sort.emf"
);
const QAC_0100: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/qac-v1-mode-0x0100.qac"
);
const QAC_CONT0: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/qac-v1-continuum-0x0000.qac"
);
const QAC_NEG_ZERO: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/qac-v1-pcm-neg-zero-preserved.qac"
);
const MAL_IDX_NO_DOM: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/malformed/emf-index-digest-missing-domain.emf"
);
const MAL_SNAN: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/malformed/qac-num-f64-snan.qac"
);
const MAL_T01_SWAP: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/malformed/t01-cross-entry-chunk-swap.emf"
);
const MAL_T01_SPLICE: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/malformed/t01-cross-file-chunk-splice.emf"
);

#[test]
fn replays_python_generated_v01_fixtures_against_structural_view() {
    for (bytes, family) in [
        (EMF_0100, Family::EmfV4Candidate),
        (EMF_0107, Family::EmfV4Candidate),
        (EMF_EDGE, Family::EmfV4Candidate),
        (QAC_0100, Family::QacV1Candidate),
        (QAC_CONT0, Family::QacV1Candidate),
        (QAC_NEG_ZERO, Family::QacV1Candidate),
    ] {
        let view = StructuralView::open(bytes, family, Limits::default()).unwrap();
        for idx in 1..=view.entry_count() {
            view.verify_chunk(idx).unwrap();
        }
    }

    assert_eq!(
        StructuralView::open(MAL_IDX_NO_DOM, Family::EmfV4Candidate, Limits::default())
            .unwrap_err(),
        Error::IndexDigest
    );
    assert_eq!(
        StructuralView::open(MAL_SNAN, Family::QacV1Candidate, Limits::default()).unwrap_err(),
        Error::Numeric
    );

    let swap_view =
        StructuralView::open(MAL_T01_SWAP, Family::EmfV4Candidate, Limits::default()).unwrap();
    assert_eq!(swap_view.verify_chunk(1).unwrap_err(), Error::PayloadDigest);
    assert_eq!(swap_view.verify_chunk(2).unwrap_err(), Error::PayloadDigest);

    let splice_view =
        StructuralView::open(MAL_T01_SPLICE, Family::EmfV4Candidate, Limits::default()).unwrap();
    assert_eq!(
        splice_view.verify_chunk(1).unwrap_err(),
        Error::PayloadDigest
    );
}
