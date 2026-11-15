use super::*;
use crate::q42::{
    field_envelope::{self as env, Family, StructuralView},
    field_semantics as sem,
};
use super::test_support::*;
const PCM: &[u8] = include_bytes!("fixtures/pcm-f32-special.le");
fn admit_limit<'a>(
    view: &StructuralView<'a>,
    selected: u32,
    limits: Limits,
) -> Result<SignalView<'a>, Error> {
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    admit(
        view,
        selected,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        sem::Limits::default(),
        limits,
    )
}
#[test]
fn sample_budget_precedes_corrupt_payload_hash_for_single_and_multiple_entries() {
    for source in [false, true] {
        let mut bytes = build(
            PCM,
            Config {
                source,
                ..Default::default()
            },
            |_| {},
        );
        let view =
            StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
        let at = view.entry(1).unwrap().payload_offset() as usize;
        let scalars = 6 + if source { ORIGINAL.len() as u64 } else { 0 };
        bytes[at] ^= 1;
        let view =
            StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
        assert!(matches!(
            admit_limit(
                &view,
                1,
                Limits {
                    samples: scalars - 1,
                    ..Default::default()
                }
            ),
            Err(Error::Limit)
        ));
        assert!(matches!(
            admit_limit(&view, 1, Limits::default()),
            Err(Error::Envelope(env::Error::PayloadDigest))
        ));
    }
}
#[test]
fn metadata_getters_are_copies_and_directional_profiles_are_not_guessed() {
    let bytes = build(PCM, Config::default(), |_| {});
    let view =
        StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
    let signal = admit_limit(&view, 1, Limits::default()).unwrap();
    let mut copy = signal.timing().unwrap();
    copy.start_tick = 999;
    assert_ne!(signal.timing().unwrap().start_tick, copy.start_tick);
    assert_eq!(signal.scalar(), Scalar::F32);
    assert_eq!(signal.channels(), 1);
    assert_eq!(signal.channel_order(), "channel-0");
    assert_eq!(signal.records(), 6);
    assert_eq!(
        signal.time_at(0).unwrap(),
        ExactTime {
            numerator: 0,
            denominator: 1
        }
    );
    let directional = build(PCM, Config::default(), |bs| {
        *field(bs, 6, 7) = strf(7, "particle_velocity");
        *field(bs, 6, 8) = strf(8, "m/s");
        *field(bs, 4, 5) = strf(5, "physical-base-units");
    });
    let view =
        StructuralView::open(&directional, Family::QacV1Candidate, env::Limits::default()).unwrap();
    assert!(admit_limit(&view, 1, Limits::default()).is_err());
}
#[test]
fn declared_lineage_is_never_applied_again_to_physical_samples() {
    let bytes = build(PCM, Config::default(), |bs| {
        *field(bs, 6, 7) = strf(7, "pressure");
        *field(bs, 6, 8) = strf(8, "Pa");
        *field(bs, 4, 5) = strf(5, "physical-base-units");
        let d = bs.iter_mut().find(|b| b.id == 6).unwrap();
        d.fields.insert(2, ref_f(6, 7));
        bs.push(block(
            7,
            202,
            vec![
                strf(1, "raw_sample_count"),
                strf(2, "Pa"),
                f(3, 5, 2f64.to_le_bytes().to_vec()),
                f(4, 5, 1f64.to_le_bytes().to_vec()),
            ],
        ));
    });
    let view =
        StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
    let signal = admit_limit(&view, 1, Limits::default()).unwrap();
    let Sample::F32(value) = signal.sample(3, 0, 0).unwrap() else {
        panic!()
    };
    assert_eq!(value.to_bits(), 1f32.to_bits());
    assert_eq!(signal.units_per_stored_sample(), 1.0);
    assert_eq!(signal.stored_offset(), 0.0);
}

#[test]
fn descriptor_field_scans_charge_the_preflight_work_budget() {
    let bytes = build(PCM, Config::default(), |bs| {
        let d = bs.iter_mut().find(|b| b.id == 6).unwrap();
        for key in 19..59 {
            d.fields.push(F {
                key,
                tag: 6,
                count: 4,
                critical: 0,
                bytes: b"kept".to_vec(),
            });
        }
    });
    let view =
        StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
    assert!(admit_limit(&view, 1, Limits::default()).is_ok());
    assert!(matches!(
        admit_limit(
            &view,
            1,
            Limits {
                work: 80,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
}
