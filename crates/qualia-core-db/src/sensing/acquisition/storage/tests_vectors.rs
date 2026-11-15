use super::tests::*;
use super::*;
use crate::{
    q42::{field_envelope as env, field_semantics as sem},
    sensing::acquisition as a,
};

const TENSOR_QAC: &[u8] = include_bytes!("fixtures/tensor.qac");
const TENSOR_EMF: &[u8] = include_bytes!("fixtures/tensor.emf");
const EVENTS_QAC: &[u8] = include_bytes!("fixtures/events.qac");
const EVENTS_EMF: &[u8] = include_bytes!("fixtures/events.emf");
const CORR_QAC: &[u8] = include_bytes!("fixtures/correlation.qac");
const CORR_EMF: &[u8] = include_bytes!("fixtures/correlation.emf");
const LINEAGE_QAC: &[u8] = include_bytes!("fixtures/lineage.qac");
const LINEAGE_EMF: &[u8] = include_bytes!("fixtures/lineage.emf");
const UNKNOWN_CRITICAL_QAC: &[u8] = include_bytes!("fixtures/unknown-critical.qac");
const UNKNOWN_OPTIONAL_QAC: &[u8] = include_bytes!("fixtures/unknown-optional.qac");
const BAD_REF_QAC: &[u8] = include_bytes!("fixtures/bad-ref.qac");
const BAD_HASH_QAC: &[u8] = include_bytes!("fixtures/bad-hash.qac");
const OVERFLOW_STRIDE_QAC: &[u8] = include_bytes!("fixtures/overflow-stride.qac");
const AUX_EMF: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/emf-aux-0x0104-0x0106-0x0107.emf"
);
const AUX_QAC: &[u8] = include_bytes!(
    "../../../../../../docs/plans/emf-qac-evolution-2026-11-13/vectors/v01/fixtures/golden/qac-aux-0x0104-0x0106-0x0107.qac"
);

fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut msg = input.to_vec();
    let bit_len = (input.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, b) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes(b.try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, delta) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(delta);
        }
    }
    let mut out = String::with_capacity(64);
    for word in h {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{word:08x}");
    }
    out
}

fn open_and_admit(bytes: &[u8], family: Family, selected: u32) -> Result<(usize, u32, u32), Error> {
    let structural = env::StructuralView::open(bytes, family, Default::default())?;
    let mut originals = [None; 65];
    let mut m = [sem::NodeState::default(); 256];
    let mut e = [sem::NodeState::default(); 128];
    let doc = admit(
        &structural,
        selected,
        &mut originals,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        Default::default(),
        Default::default(),
    )?;
    Ok((
        doc.metadata_bytes.len(),
        doc.uninterpreted_blocks,
        doc.uninterpreted_fields,
    ))
}

#[test]
fn python_fixture_manifest_hashes_match() {
    let expected = [
        ("tensor.qac", TENSOR_QAC, "3a4a54b1a4ca668fae9b2d76348d334575b995ea3cf5c2c6b7a92f6dc4bfa2db"),
        ("tensor.emf", TENSOR_EMF, "d18b466063e0e4527b82f77f78009034035cd1c78e1f4179434f3cd654d2f96f"),
        ("events.qac", EVENTS_QAC, "7a6d32f0ab279dfcbfa5edae130904443acd3870dd264199b42c96ec6a3fb3bc"),
        ("events.emf", EVENTS_EMF, "0859a5c424def50acf55cf1652f5a55e637a213b1bf09aaf10a306a13458a8d9"),
        ("correlation.qac", CORR_QAC, "3b7cd46a71eba54b62fe8b6cd1b0f9eef702b558679abadd7f0922143aed824f"),
        ("correlation.emf", CORR_EMF, "c02056cb99b1da5d5a82fc526a7c0c229895c9e53e1514db20e72a4f2c630423"),
        ("lineage.qac", LINEAGE_QAC, "c5420a9070d803c2995de570a1b07c101ce47fe44befe0f63d6a42ecffa002f0"),
        ("lineage.emf", LINEAGE_EMF, "405bd216b184478783a0cb60534f99bae5edbaf3706366ad701288b1775b7b81"),
        ("unknown-critical.qac", UNKNOWN_CRITICAL_QAC, "8a85bbec594f0450f3ee0386e57864f60fb0a6c267937e7893400a483cd142c5"),
        ("unknown-optional.qac", UNKNOWN_OPTIONAL_QAC, "149125527d3ac707d72fa379cf7b6cf4fcdf6887700ca66c4c174af67e146982"),
        ("bad-ref.qac", BAD_REF_QAC, "ad786798f632df546355513ecb47f7684df3aa5e6375e2d73c3da3fa688c085c"),
        ("bad-hash.qac", BAD_HASH_QAC, "1896f5ef1495ad66f063bc2daed74f863d4548749d996f7a259f037742e56c1d"),
        ("overflow-stride.qac", OVERFLOW_STRIDE_QAC, "4073bab742609ed742ca006e6b9476fd36db181c7a231746a2052889088df484"),
    ];
    for (name, bytes, digest) in expected {
        assert_eq!(sha256_hex(bytes), digest, "fixture sha256 mismatch for {name}");
    }
}

#[test]
fn positive_python_vectors_admit_all_views_and_preserve_originals() {
    for (family, bytes) in [
        (Family::QacV1Candidate, TENSOR_QAC),
        (Family::EmfV4Candidate, TENSOR_EMF),
    ] {
        inspect(bytes, family, |d| {
            let View::Tensor(t) = d.view else { panic!("expected tensor") };
            assert_eq!(t.numeric([0; 4]).unwrap(), [-0.0, 0.0]);
            assert_eq!(t.numeric([1, 0, 0, 0]), Err(a::Error::Nonfinite));
            assert_eq!(d.uninterpreted_blocks, 0);
            assert_eq!(d.uninterpreted_fields, 0);
        });
    }
    for (family, bytes) in [
        (Family::QacV1Candidate, EVENTS_QAC),
        (Family::EmfV4Candidate, EVENTS_EMF),
    ] {
        inspect(bytes, family, |d| {
            let View::Events { view, axes } = d.view else { panic!("expected events") };
            assert_eq!(view.records(), 2);
            assert_eq!(view.byte_order(), a::ByteOrder::Big);
            assert_eq!(view.ordering(), a::EventOrdering::NondecreasingTick);
            assert_eq!(view.event(0).unwrap(), a::Event { tick: -5, channel: 1, count: u64::MAX });
            assert_eq!(view.event(1).unwrap(), a::Event { tick: 12, channel: 0, count: 0 });
            assert_eq!(axes.carrier, a::Carrier::Hertz(2e6));
            assert_eq!(axes.baseband_hz, Some([-1e5, 1e5]));
        });
    }
    for (family, bytes) in [
        (Family::QacV1Candidate, CORR_QAC),
        (Family::EmfV4Candidate, CORR_EMF),
    ] {
        inspect(bytes, family, |d| {
            let View::Correlation(c) = d.view else { panic!("expected correlation") };
            assert_eq!(c.lag_ticks(0).unwrap(), -1);
            assert_eq!(c.lag_ticks(1).unwrap(), 1);
            assert_eq!(c.normalization(), a::Normalization::Mean { contributing_pairs: 5 });
            assert_eq!(c.tensor().numeric([0, 0, 0, 0]).unwrap(), [1.0, -2.0]);
            assert_eq!(c.tensor().numeric([1, 0, 0, 0]).unwrap(), [3.5, 0.0]);
        });
    }
    for (family, bytes) in [
        (Family::QacV1Candidate, LINEAGE_QAC),
        (Family::EmfV4Candidate, LINEAGE_EMF),
    ] {
        inspect(bytes, family, |d| {
            let lin = d.lineage.expect("expected schema7 lineage");
            assert_eq!(lin.output_entry(), 1);
            assert_eq!(lin.inputs(), 1);
            assert_eq!(lin.input_entry(0).unwrap(), 2);
            assert_eq!(d.original(2).unwrap().bytes(), b"INPUT-original-bytes");
            assert_eq!(lin.losses(), 2);
            assert_eq!(lin.loss(0).unwrap(), a::Loss::PhaseDiscarded);
            assert!(matches!(lin.loss(1).unwrap(), a::Loss::Registered(_)));
        });
        assert_eq!(open_and_admit(bytes, family, 2), Err(Error::Unsupported));
    }
}

#[test]
fn optional_and_negative_python_vectors_match_contract() {
    let (_, blocks, fields) =
        open_and_admit(UNKNOWN_OPTIONAL_QAC, Family::QacV1Candidate, 1).unwrap();
    assert_eq!(blocks, 0);
    assert_eq!(fields, 1);

    assert_eq!(
        open_and_admit(UNKNOWN_CRITICAL_QAC, Family::QacV1Candidate, 1),
        Err(Error::Semantic(sem::Error::UnknownCriticalField))
    );
    assert!(open_and_admit(BAD_REF_QAC, Family::QacV1Candidate, 1).is_err());
    assert!(open_and_admit(BAD_HASH_QAC, Family::QacV1Candidate, 1).is_err());
    assert!(open_and_admit(OVERFLOW_STRIDE_QAC, Family::QacV1Candidate, 1).is_err());
}

#[test]
fn v01_auxiliary_shared_vectors_structural_and_profile_gate() {
    assert_eq!(
        sha256_hex(AUX_EMF),
        "841a909c2c1e22427cb1c7c6e5d031f4c765f9c5fa65757aa605259d1e664d33"
    );
    assert_eq!(
        sha256_hex(AUX_QAC),
        "04bf1b3e5c0b636d8e8680bf8f5c5313e4d39448cd7557533f61f32a8b016784"
    );
    for (bytes, family) in [
        (AUX_EMF, Family::EmfV4Candidate),
        (AUX_QAC, Family::QacV1Candidate),
    ] {
        let structural = env::StructuralView::open(bytes, family, Default::default()).unwrap();
        assert_eq!(structural.entry_count(), 3);
        for i in 1..=3 {
            structural.verify_chunk(i).unwrap();
        }
        // Acquisition storage reader strictly rejects entries with flags!=0 or mode!=ORIGINAL_MODE
        assert_eq!(open_and_admit(bytes, family, 1), Err(Error::Unsupported));
    }
}
