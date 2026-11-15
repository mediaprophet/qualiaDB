use super::*;
use crate::container_10d::mesh_section::{decode_mesh_section, encode_mesh_section, encoded_len};
use crate::render::assets::Mesh;
use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;

fn mesh() -> Mesh {
    Mesh {
        positions: vec![[0.; 3], [1., 0., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2]],
        min: [0.; 3],
        max: [1., 1., 0.],
    }
}

#[test]
fn borrowed_geometry_is_exact_bounded_and_zero_heap() {
    let mut bytes = vec![0; encoded_len(3, 1)];
    encode_mesh_section(&mesh(), &mut bytes).unwrap();
    let guard = AllocGuard::begin("mesh-view-complete-admission", true);
    let v = MeshSectionView::parse(&bytes).unwrap();
    assert_eq!(v.triangle(0), Some([0, 1, 2]));
    assert_eq!(v.vertex(1), Some([1., 0., 0.]));
    assert!(v.vertex(usize::MAX).is_none());
    assert!(v.triangle(1).is_none());
    guard.check().unwrap();
}

#[test]
fn bad_index_refuses_before_decoding_or_encoding_output() {
    let mut bytes = vec![0; encoded_len(3, 1)];
    encode_mesh_section(&mesh(), &mut bytes).unwrap();
    let at = MESH_MINI_HEADER_SIZE + 3 * 6 + 2 * 2;
    bytes[at..at + 2].copy_from_slice(&3u16.to_le_bytes());
    let guard = AllocGuard::begin("mesh-view-bad-index", true);
    assert!(matches!(
        MeshSectionView::parse(&bytes),
        Err(E::IndexOutOfBounds { .. })
    ));
    assert!(matches!(
        decode_mesh_section(&bytes),
        Err(E::IndexOutOfBounds { .. })
    ));
    guard.check().unwrap();
    let mut input = mesh();
    input.triangles[0][2] = 65_536; // Previously truncated to u16 zero by encoder.
    bytes.fill(0xa5);
    assert!(matches!(
        encode_mesh_section(&input, &mut bytes),
        Err(E::IndexOutOfBounds { .. })
    ));
    assert!(bytes.iter().all(|b| *b == 0xa5));
}

#[test]
fn u32_indices_and_exact_extent_are_checked() {
    let h = MeshMiniHeader {
        flags: 0,
        reserved_u16: 0,
        vertex_count: 1,
        triangle_count: 1,
        min: [0.; 3],
        max: [0.; 3],
        reserved_u32: 0,
    };
    let mut bytes = vec![0; 40 + 6 + 12];
    bytes[..40].copy_from_slice(bytemuck::bytes_of(&h));
    assert_eq!(
        MeshSectionView::parse(&bytes).unwrap().triangle(0),
        Some([0; 3])
    );
    bytes[54..58].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        MeshSectionView::parse(&bytes),
        Err(E::IndexOutOfBounds { .. })
    ));
    bytes[54..58].fill(0);
    bytes.push(0);
    assert!(matches!(
        MeshSectionView::parse(&bytes),
        Err(E::TrailingBytes { .. })
    ));
}

#[test]
fn encoder_rejects_nonfinite_and_preserves_extreme_finite_endpoints() {
    let mut input = mesh();
    input.positions[0][0] = f32::NAN;
    let mut bytes = vec![0xa5; encoded_len(3, 1)];
    assert!(matches!(
        encode_mesh_section(&input, &mut bytes),
        Err(E::InvalidPosition { .. })
    ));
    assert!(bytes.iter().all(|b| *b == 0xa5));
    input.positions = vec![[-f32::MAX, 0., 0.], [0.; 3], [f32::MAX, 0., 0.]];
    encode_mesh_section(&input, &mut bytes).unwrap();
    let view = MeshSectionView::parse(&bytes).unwrap();
    assert_eq!(view.vertex(0).unwrap()[0], -f32::MAX);
    assert_eq!(view.vertex(2).unwrap()[0], f32::MAX);
    assert!(view.vertex(1).unwrap()[0].is_finite());
}
