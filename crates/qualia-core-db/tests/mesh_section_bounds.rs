//! Public codec regression: decoded positions remain in the declared finite frame.
use qualia_core_db::{
    container_10d::mesh_section::{
        decode_mesh_section, encode_mesh_section, encoded_len, MeshSectionError,
        MAX_VERTEX_COUNT, MESH_MINI_HEADER_SIZE,
    },
    render::assets::Mesh,
};

fn payload(min: [f32; 3], max: [f32; 3], quantized: &[u16]) -> Vec<u8> {
    assert!(quantized.len() >= 2);
    let mut positions = vec![min; quantized.len()];
    *positions.last_mut().unwrap() = max;
    let mesh = Mesh { positions, triangles: Vec::new(), min, max };
    let mut bytes = vec![0; encoded_len(mesh.positions.len(), 0)];
    let written = encode_mesh_section(&mesh, &mut bytes).unwrap();
    assert_eq!(written, bytes.len());
    // Preserve the requested stored endpoint bits, including -0/+0. The public
    // encoder recomputes a numeric bbox and need not retain signed-zero extrema.
    for axis in 0..3 {
        bytes[12 + axis * 4..16 + axis * 4].copy_from_slice(&min[axis].to_le_bytes());
        bytes[24 + axis * 4..28 + axis * 4].copy_from_slice(&max[axis].to_le_bytes());
    }
    // Supply exact quantized samples so these tests isolate decoding, including
    // large finite frames whose f32 encoder extent would itself overflow.
    for (vertex, q) in quantized.iter().enumerate() {
        for axis in 0..3 {
            let offset = MESH_MINI_HEADER_SIZE + vertex * 6 + axis * 2;
            bytes[offset..offset + 2].copy_from_slice(&q.to_le_bytes());
        }
    }
    bytes
}

fn assert_frame(min: [f32; 3], max: [f32; 3], samples: &[u16]) {
    let bytes = payload(min, max, samples);
    let decoded = decode_mesh_section(&bytes).unwrap();
    assert_eq!(decoded.positions.len(), samples.len());
    assert_eq!(decoded.min, min);
    assert_eq!(decoded.max, max);
    for axis in 0..3 {
        assert_eq!(decoded.positions.first().unwrap()[axis].to_bits(), min[axis].to_bits());
        assert_eq!(decoded.positions.last().unwrap()[axis].to_bits(), max[axis].to_bits());
        let mut previous = min[axis];
        for position in &decoded.positions {
            assert!(position[axis].is_finite());
            assert!(position[axis] >= min[axis] && position[axis] <= max[axis],
                "axis {axis}: {} outside [{}, {}]", position[axis], min[axis], max[axis]);
            assert!(position[axis] >= previous);
            previous = position[axis];
        }
    }
    // The decoder never patches the source payload or its declared frame.
    assert_eq!(bytes, payload(min, max, samples));
}

#[test]
fn measured_growing_station_frame_contains_every_quantized_coordinate() {
    let samples = (0..=u16::MAX).collect::<Vec<_>>();
    assert_frame(
        [-1.2999999523162842_f32, 0.014999999664723873_f32, -0.8690000176429749_f32],
        [1.2999999523162842_f32, 1.7200000286102295_f32, 0.8109999895095825_f32],
        &samples,
    );
}

#[test]
fn asymmetric_negative_and_zero_span_frames_preserve_exact_intervals() {
    let samples = [0, 1, 2, 16383, 32767, 32768, 49152, 65533, 65534, 65535];
    assert_frame([-31.7, -9.25, -0.000001], [-0.13, -1.5, -0.0000001], &samples);
    assert_frame([0.3, -12.0, -0.0], [0.7, 91.0, 0.0], &samples);
    assert_frame([3.25, -7.0, 0.0], [3.25, -7.0, 0.0], &samples);
}

#[test]
fn large_finite_frames_do_not_overflow_through_an_f32_extent() {
    let samples = [0, 1, 16383, 32767, 32768, 49152, 65534, 65535];
    assert!((f32::MAX - (-f32::MAX)).is_infinite());
    assert_frame([-f32::MAX, -f32::MAX, f32::MAX / 2.0],
        [f32::MAX, -f32::MAX / 2.0, f32::MAX], &samples);
}

#[test]
fn malformed_frames_and_existing_header_errors_still_fail_closed() {
    let valid = payload([0.0; 3], [1.0; 3], &[0, 65535]);
    for (offset, value) in [(12, f32::NAN), (16, f32::NEG_INFINITY),
        (24, f32::INFINITY), (28, -1.0)] {
        let mut bytes = valid.clone();
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(matches!(decode_mesh_section(&bytes), Err(MeshSectionError::InvalidBounds { .. })));
    }
    let mut reserved = valid.clone();
    reserved[2] = 1;
    assert!(matches!(decode_mesh_section(&reserved), Err(MeshSectionError::NonZeroReserved { .. })));
    let mut flags = valid.clone();
    flags[0] |= 2;
    assert!(matches!(decode_mesh_section(&flags), Err(MeshSectionError::UnknownFlags { .. })));
    let mut count = valid.clone();
    count[4..8].copy_from_slice(&((MAX_VERTEX_COUNT + 1) as u32).to_le_bytes());
    assert!(matches!(decode_mesh_section(&count), Err(MeshSectionError::VertexCountTooLarge { .. })));
    assert!(matches!(decode_mesh_section(&valid[..valid.len() - 1]), Err(MeshSectionError::PayloadTruncated { .. })));
    assert!(matches!(decode_mesh_section(&valid[..39]), Err(MeshSectionError::PayloadTooShort { .. })));
}