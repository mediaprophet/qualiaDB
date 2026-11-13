use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{verify_leaf_span, SpanLimits},
    selection::select_entry,
};
use crate::container_10d::{
    header::Container10dHeader,
    mesh_section::{decode_mesh_section, encode_mesh_section, encoded_len},
    section::{encode_container, AlignmentTier, SectionDescriptor, SectionInput, SectionType},
};
use crate::net::qdnf::types::StrongDigest;
const EMPTY: SectionDescriptor = SectionDescriptor {
    section_type: 0,
    alignment_tier: 0,
    reserved16: 0,
    byte_offset: 0,
    byte_length: 0,
    stride: 0,
    element_count: 0,
    crc32c: 0,
};
fn limits() -> FormatLimits {
    FormatLimits {
        max_entry_bytes: 16384,
        max_rows: 8,
        max_metadata_bytes: 512,
        max_chunk_bytes: 4096,
    }
}
fn container() -> Vec<u8> {
    let mesh = crate::render::assets::Mesh {
        positions: vec![[0.; 3], [1., 0., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2]],
        min: [0.; 3],
        max: [1., 1., 0.],
    };
    let mut payload = vec![0; encoded_len(3, 1)];
    encode_mesh_section(&mesh, &mut payload).unwrap();
    let input = SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &payload,
    };
    let mut out = vec![0; 512];
    let n = encode_container(&Container10dHeader::proposed(), &[input], &mut out).unwrap();
    out.truncate(n);
    out
}
fn authenticated(
    payload: &[u8],
    run: impl FnOnce(AuthenticatedRange<'_>, AuthenticatedRange<'_>, AuthenticatedRange<'_>),
) {
    let f = Fixture::new_with_payload(payload);
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let closure = f.bound(&pins, pin, publisher);
    let h = admit_header(
        &f.source()[..64],
        f.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut index_scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &h,
        &f.source()[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut index_scratch,
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
    let mut siblings = [StrongDigest::ZERO; 32];
    let span = verify_leaf_span(
        &spec,
        selected.root(),
        0,
        payload,
        &[&[0]],
        &mut siblings,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: 16384,
        },
    )
    .unwrap();
    let header = AuthenticatedRange::new(selected, &span, 0, 64).unwrap();
    let table = AuthenticatedRange::new(selected, &span, 64, 24).unwrap();
    let section = AuthenticatedRange::new(selected, &span, 88, payload.len() - 88).unwrap();
    run(header, table, section);
}
#[test]
fn actual_mesh_decoder_consumes_selected_verified_bytes() {
    let bytes = container();
    authenticated(&bytes, |h, t, p| {
        use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
        let mut rows = [EMPTY; 1];
        let mut scratch = [PayloadInterval::default(); 1];
        let guard = AllocGuard::begin("10d-selective-metadata-section", true);
        let view = TenDDirectoryView::admit(h, t, limits(), &mut rows, &mut scratch).unwrap();
        let selected = view
            .selected_section(SectionType::QuantizedMesh, p)
            .unwrap();
        assert_eq!(selected.bytes().as_ptr(), bytes[88..].as_ptr());
        assert_eq!(selected.mesh_header().unwrap().triangle_count, 1);
        assert!(view.required_section(SectionType::Topology).is_err());
        guard.check().unwrap();
        // Existing complete typed payload decoder is a bounded cold allocation;
        // it receives the selected original payload, never a stitched container.
        let mesh = decode_mesh_section(selected.bytes()).unwrap();
        assert_eq!(mesh.triangles, vec![[0, 1, 2]]);
        assert_eq!(mesh.positions.len(), 3);
    });
}
#[test]
fn signed_descriptor_corruption_and_inner_crc_refuse() {
    let good = container();
    for byte in [66, 67, 65, 68, 72, 76] {
        let mut bad = good.clone();
        bad[byte] ^= 0xff;
        authenticated(&bad, |h, t, _| {
            assert!(TenDDirectoryView::admit(
                h,
                t,
                limits(),
                &mut [EMPTY; 1],
                &mut [PayloadInterval::default(); 1]
            )
            .is_err());
        });
    }
    let mut bad = good.clone();
    let end = bad.len() - 1;
    bad[end] ^= 1;
    authenticated(&bad, |h, t, p| {
        let mut rows = [EMPTY; 1];
        let view = TenDDirectoryView::admit(
            h,
            t,
            limits(),
            &mut rows,
            &mut [PayloadInterval::default(); 1],
        )
        .unwrap();
        assert!(matches!(
            view.selected_section(SectionType::QuantizedMesh, p),
            Err(FormatError::Integrity)
        ));
    });
}
#[test]
fn selective_metadata_and_payload_limits_refuse() {
    authenticated(&container(), |h, t, p| {
        assert!(matches!(
            TenDDirectoryView::admit(
                h,
                t,
                limits(),
                &mut [],
                &mut [PayloadInterval::default(); 1]
            ),
            Err(FormatError::Capacity)
        ));
        assert!(matches!(
            TenDDirectoryView::admit(
                h,
                t,
                FormatLimits {
                    max_metadata_bytes: 87,
                    ..limits()
                },
                &mut [EMPTY; 1],
                &mut [PayloadInterval::default(); 1]
            ),
            Err(FormatError::Limit)
        ));
        let mut rows = [EMPTY; 1];
        let view = TenDDirectoryView::admit(
            h,
            t,
            FormatLimits {
                max_chunk_bytes: 1,
                ..limits()
            },
            &mut rows,
            &mut [PayloadInterval::default(); 1],
        )
        .unwrap();
        assert!(matches!(
            view.selected_section(SectionType::QuantizedMesh, p),
            Err(FormatError::Limit)
        ));
    });
}

#[test]
fn variable_stride_count_is_metadata_not_a_fixed_stride_extent() {
    let mut bytes = container();
    // Canonical descriptor validation uses element_count only when stride > 0.
    bytes[80..84].copy_from_slice(&255u32.to_le_bytes());
    authenticated(&bytes, |h, t, p| {
        let mut rows = [EMPTY; 1];
        let directory = TenDDirectoryView::admit(
            h,
            t,
            limits(),
            &mut rows,
            &mut [PayloadInterval::default(); 1],
        )
        .unwrap();
        let geometry = directory
            .selected_section(SectionType::QuantizedMesh, p)
            .unwrap();
        assert_eq!(geometry.geometry().unwrap().triangle(0), Some([0, 1, 2]));
    });
}
