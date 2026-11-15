use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{
        encode_proof, sibling_ranges, verify_leaf_span, OutboardNode, ProofSiblingRange,
        SpanLimits, TreeBuilder, MAX_SIBLINGS,
    },
    selection::select_entry,
};
use crate::net::qdnf::types::StrongDigest as D;
use crate::q42::emf_container::*;
fn payload() -> Vec<u8> {
    let field = EmfFieldInterpretation {
        colour: EmfColourInterpretation::SpectralOnly,
        timebase: EmfTimebase {
            epoch: EmfEpoch {
                scale: EmfEpochScale::Tai,
                seconds: 10,
                nanoseconds: 0,
            },
            ticks_per_second_num: 48000,
            ticks_per_second_den: 1,
            slice_start_tick: 0,
            slice_step_ticks: 480,
        },
        hilbert_order: 1,
        spectral_semantics: EmfSpectralSemantics::AuthoredWavelengthLegacySigma,
    };
    let mut writer = EmfContainerWriter::new(
        [0.; 3],
        1.,
        [2, 1, 1],
        field.timebase.epoch.header_seconds(),
    );
    let mut cells = [EmfCellRecord64::default(); 512];
    cells[511].wavelength_nm = 1000.;
    writer.add_brick_chunk(0, &cells, 0, 1, 0).unwrap();
    let second_brick = crate::q42::m64_matter::brick3d::encode_hilbert3d_u64(1, 0, 0, 1);
    writer.add_brick_chunk(second_brick, &cells, 0, 1, 0).unwrap();
    let spec = EmfAudioTrackSpec {
        id: "pressure".into(),
        purpose: EmfAudioPurpose::Recorded,
        channels: 1,
        channel_layout: EmfChannelLayout::Mono,
        sample_rate_num: 48000,
        sample_rate_den: 1,
        start_tick: 0,
        units: EmfSignalUnits::Pascal,
        units_per_sample: 1.,
    };
    writer
        .to_bytes_v3(
            &field,
            &[EmfAudioTrack {
                spec,
                samples: &[0., -0., 2.5, -3.75],
            }],
            EmfExtensionLimits::conservative(),
        )
        .unwrap()
}
fn authenticated(
    bytes: &[u8],
    run: impl FnOnce(
        AuthenticatedRange<'_>,
        AuthenticatedRange<'_>,
        AuthenticatedRange<'_>,
        AuthenticatedRange<'_>,
    ),
) {
    let f = Fixture::new_with_payload(bytes);
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let closure = f.bound(&pins, pin, publisher);
    let h = admit_header(
        &f.source()[..64],
        f.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &h,
        &f.source()[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut scratch,
    )
    .unwrap();
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let selected = select_entry(
        &closure,
        f.body(),
        &f.source()[..64],
        &index,
        f.entry_page(),
        0,
        &mut rows,
        &mut framing,
    )
    .unwrap();
    let spec = selected.tree_spec().unwrap();
    assert_eq!(spec.leaf_count(), 5);
    let mut tree = TreeBuilder::new(spec);
    let mut nodes: Vec<OutboardNode> = Vec::new();
    for leaf in bytes.chunks(16384) {
        tree.append(leaf, |n| {
            nodes.push(n);
            Ok(())
        })
        .unwrap();
    }
    assert_eq!(
        tree.finish(|n| {
            nodes.push(n);
            Ok(())
        })
        .unwrap(),
        selected.root()
    );
    let mut encoded = [[0; 1537]; 4];
    let mut lens = [0; 4];
    for (slot, leaf) in [0, 1, 2, 4].into_iter().enumerate() {
        let mut path = [ProofSiblingRange::EMPTY; MAX_SIBLINGS];
        let n = sibling_ranges(5, leaf, &mut path).unwrap();
        let mut siblings = [D::ZERO; 32];
        for (dst, r) in siblings.iter_mut().zip(&path[..n]) {
            *dst = nodes
                .iter()
                .find(|v| v.start_leaf == r.start_leaf && v.leaf_count == r.leaf_count)
                .unwrap()
                .digest;
        }
        lens[slot] = encode_proof(&siblings[..n], &mut encoded[slot]).unwrap();
    }
    let proofs = [
        &encoded[0][..lens[0]],
        &encoded[1][..lens[1]],
        &encoded[2][..lens[2]],
    ];
    let mut siblings = [D::ZERO; 32];
    let prefix = verify_leaf_span(
        &spec,
        selected.root(),
        0,
        &bytes[..49152],
        &proofs,
        &mut siblings,
        SpanLimits {
            max_leaves: 3,
            max_encoded_bytes: 49152,
        },
    )
    .unwrap();
    let tail_span = verify_leaf_span(
        &spec,
        selected.root(),
        4,
        &bytes[65536..],
        &[&encoded[3][..lens[3]]],
        &mut siblings,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: 16384,
        },
    )
    .unwrap();
    let h = AuthenticatedRange::new(selected, &prefix, 0, 64).unwrap();
    let d = AuthenticatedRange::new(selected, &prefix, 64, 128).unwrap();
    let p = AuthenticatedRange::new(selected, &prefix, 192, 32832).unwrap();
    let start = u64::from_le_bytes(
        bytes[bytes.len() - 56..bytes.len() - 48]
            .try_into()
            .unwrap(),
    );
    assert_eq!(start, 65856);
    let t =
        AuthenticatedRange::new(selected, &tail_span, start, bytes.len() - start as usize).unwrap();
    assert!(AuthenticatedRange::new(selected, &prefix, 33024, 32832).is_err());
    assert!(AuthenticatedRange::new(selected, &tail_span, 33024, 32832).is_err());
    run(h, d, t, p);
}
fn limits(bytes: usize) -> FormatLimits {
    FormatLimits {
        max_entry_bytes: bytes as u64,
        max_rows: 2,
        max_metadata_bytes: 2 * 1024 * 1024,
        max_chunk_bytes: 32832,
    }
}
#[test]
fn v3_complete_interpretation_pcm_and_one_brick_need_no_middle_leaf() {
    let mut bytes = payload();
    let full = EmfContainerReader::parse(&bytes).unwrap();
    assert_eq!(
        full.extension()
            .unwrap()
            .audio_sample(0, 1, 0)
            .unwrap()
            .to_bits(),
        (-0f32).to_bits()
    );
    drop(full);
    // Deliberately corrupt only the UNFETCHED middle of the other brick, then
    // re-sign the actual outer entry. Full-file verification refuses; a partial
    // view must still accurately expose only the independently verified brick.
    bytes[50000] ^= 1;
    assert!(EmfContainerReader::parse(&bytes).is_err());
    authenticated(&bytes, |h, d, t, p| {
        let view = EmfV3DirectoryView::admit(
            h,
            d,
            t,
            limits(bytes.len()),
            EmfExtensionLimits::conservative(),
            &mut [PayloadInterval::default(); 2],
        )
        .unwrap();
        assert_eq!(
            view.extension().audio_sample(0, 1, 0).unwrap().to_bits(),
            (-0f32).to_bits()
        );
        assert_eq!(view.extension().audio_sample(0, 3, 0), Some(-3.75));
        use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
        let guard = AllocGuard::begin("emf-v3-selected-brick", true);
        let brick = view.selected_chunk(0, p).unwrap();
        assert_eq!(brick.cell(511).unwrap().wavelength_nm, 1000.);
        assert_eq!(brick.records().as_ptr(), bytes[192..].as_ptr());
        guard.check().unwrap();
    });
}
#[test]
fn v3_extension_and_selected_full_trailer_mutations_refuse_under_fresh_outer_roots() {
    let good = payload();
    for at in [good.len() - 128, good.len() - 96, good.len() - 65] {
        let mut bad = good.clone();
        bad[at] ^= 1;
        authenticated(&bad, |h, d, t, _| {
            assert!(EmfV3DirectoryView::admit(
                h,
                d,
                t,
                limits(bad.len()),
                EmfExtensionLimits::conservative(),
                &mut [PayloadInterval::default(); 2]
            )
            .is_err())
        });
    }
    for at in [32991, 33023] {
        let mut bad = good.clone();
        bad[at] ^= 1;
        authenticated(&bad, |h, d, t, p| {
            let view = EmfV3DirectoryView::admit(
                h,
                d,
                t,
                limits(bad.len()),
                EmfExtensionLimits::conservative(),
                &mut [PayloadInterval::default(); 2],
            )
            .unwrap();
            assert!(matches!(
                view.selected_chunk(0, p),
                Err(FormatError::Integrity)
            ));
        });
    }
    authenticated(&good, |h, d, t, _| {
        let mut ext = EmfExtensionLimits::conservative();
        ext.max_audio_bytes = 1;
        assert!(EmfV3DirectoryView::admit(
            h,
            d,
            t,
            limits(good.len()),
            ext,
            &mut [PayloadInterval::default(); 2]
        )
        .is_err());
        assert!(EmfV3DirectoryView::admit(
            h,
            d,
            d,
            limits(good.len()),
            EmfExtensionLimits::conservative(),
            &mut [PayloadInterval::default(); 2]
        )
        .is_err());
    });
}
