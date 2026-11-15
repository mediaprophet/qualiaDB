//! A complete geometry payload from one leaf; the remaining node payload is absent.
use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{encode_proof, verify_leaf_span, OutboardNode, SpanLimits, TreeBuilder},
    selection::select_entry,
};
use crate::container_10d::{
    crc32c::crc32c,
    header::Container10dHeader,
    mesh_section::{encode_mesh_section, encoded_len},
    node_section::write_node_section_aos,
    section::{encode_container, AlignmentTier, SectionDescriptor, SectionInput, SectionType},
};
use crate::net::qdnf::types::StrongDigest as D;
use bytemuck::Zeroable;

fn container() -> Vec<u8> {
    let mesh = crate::render::assets::Mesh {
        positions: vec![[0.; 3], [1., 0., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2]],
        min: [0.; 3],
        max: [1., 1., 0.],
    };
    let mut geometry = vec![0; encoded_len(3, 1)];
    encode_mesh_section(&mesh, &mut geometry).unwrap();
    let nodes = [crate::tensor::Tensor10D::default(); 512];
    let mut node_bytes = vec![0; 16 + 40 * 512];
    write_node_section_aos(&nodes, &mut node_bytes).unwrap();
    let inputs = [
        SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &geometry,
        },
        SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &node_bytes,
        },
    ];
    let mut bytes = vec![0; 32768];
    let n = encode_container(&Container10dHeader::proposed(), &inputs, &mut bytes).unwrap();
    bytes.truncate(n);
    bytes
}

fn verify(bytes: &[u8], valid: bool) {
    assert!(bytes.len() > 16384 && bytes.len() < 32768);
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
    let mut idx = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &h,
        &f.source()[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut idx,
    )
    .unwrap();
    let selected = select_entry(
        &closure,
        f.body(),
        &f.source()[..64],
        &index,
        f.entry_page(),
        0,
        &mut [EntryRecord::EMPTY; 128],
        &mut vec![0; HASH_SCRATCH_BYTES],
    )
    .unwrap();
    let spec = selected.tree_spec().unwrap();
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
    let sibling = nodes
        .iter()
        .find(|n| n.start_leaf == 1 && n.leaf_count == 1)
        .unwrap()
        .digest;
    let mut proof = [0; 49];
    assert_eq!(encode_proof(&[sibling], &mut proof).unwrap(), 49);
    let received = &bytes[..16384];
    let span = verify_leaf_span(
        &spec,
        selected.root(),
        0,
        received,
        &[&proof],
        &mut [D::ZERO; 32],
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: 16384,
        },
    )
    .unwrap();
    let header = AuthenticatedRange::new(selected, &span, 0, 64).unwrap();
    let table = AuthenticatedRange::new(selected, &span, 64, 48).unwrap();
    let d: SectionDescriptor = bytemuck::pod_read_unaligned(&bytes[64..88]);
    let payload = AuthenticatedRange::new(
        selected,
        &span,
        d.byte_offset as u64,
        d.byte_length as usize,
    )
    .unwrap();
    let other: SectionDescriptor = bytemuck::pod_read_unaligned(&bytes[88..112]);
    assert!(AuthenticatedRange::new(
        selected,
        &span,
        other.byte_offset as u64,
        other.byte_length as usize
    )
    .is_err());
    let limits = FormatLimits {
        max_entry_bytes: 32768,
        max_rows: 2,
        max_metadata_bytes: 112,
        max_chunk_bytes: 1024,
    };
    let mut rows = [SectionDescriptor::zeroed(); 2];
    let mut intervals = [PayloadInterval::default(); 2];
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    let guard = AllocGuard::begin("complete-partial-geometry", true);
    let directory =
        TenDDirectoryView::admit(header, table, limits, &mut rows, &mut intervals).unwrap();
    let section = directory
        .selected_section(SectionType::QuantizedMesh, payload)
        .unwrap();
    let geometry = section.geometry();
    if valid {
        let geometry = geometry.unwrap();
        assert_eq!(geometry.triangle(0), Some([0, 1, 2]));
        assert_eq!(geometry.vertex(1), Some([1., 0., 0.]));
        assert_eq!(geometry.vertex(2), Some([0., 1., 0.]));
        assert!(geometry.triangle(1).is_none());
    } else {
        assert!(matches!(geometry, Err(FormatError::Integrity)));
    }
    guard.check().unwrap();
}

#[test]
fn complete_mesh_is_usable_without_unfetched_nodes() {
    verify(&container(), true);
}

#[test]
fn authenticated_mesh_with_valid_section_crc_but_bad_index_is_refused() {
    let mut bytes = container();
    let d: SectionDescriptor = bytemuck::pod_read_unaligned(&bytes[64..88]);
    let start = d.byte_offset as usize;
    let end = start + d.byte_length as usize;
    bytes[end - 2..end].copy_from_slice(&3u16.to_le_bytes());
    let crc = crc32c(&bytes[start..end]);
    bytes[84..88].copy_from_slice(&crc.to_le_bytes());
    // Fixture signs a new outer tree; refusal is semantic, not the old root/CRC.
    verify(&bytes, false);
}
