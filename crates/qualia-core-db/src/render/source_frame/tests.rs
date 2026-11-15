use super::*;

fn binding() -> SourceFrameBinding {
    SourceFrameBinding {
        model_sha256: [1; 32],
        source_sha256: [2; 32],
        source_generation: 7,
        mesh_generation: 3,
        transport_epoch: 1,
        vertex_count: 3,
        index_count: 3,
    }
}
const POSITIONS: [[f32; 3]; 3] = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
const NORMALS: [[f32; 3]; 3] = [[0., 0., 1.]; 3];
const TANGENTS: [[f32; 4]; 3] = [[1., 0., 0., 1.]; 3];
fn frame(binding: SourceFrameBinding, sequence: u64, tick: u64) -> SourceMeshFrame<'static> {
    SourceMeshFrame {
        binding,
        frame_sequence: sequence,
        timeline_tick: tick,
        positions: &POSITIONS,
        normals: &NORMALS,
        tangents: &TANGENTS,
    }
}

#[test]
fn zero_initial_epoch_refuses_without_consuming_cold_binding() {
    let mut state = SourceFrameState::default();
    assert_eq!(
        state.bind(SourceFrameBinding {
            transport_epoch: 0,
            ..binding()
        }),
        Err(SourceFrameError::Input)
    );
    assert_eq!(state.binding(), None);
    assert!(!state.externally_controlled());
    let current = SourceFrameBinding {
        transport_epoch: 23,
        ..binding()
    };
    state.bind(current).unwrap();
    assert_eq!(state.binding(), Some(current));
    assert!(state
        .prepare(&frame(current, 1, 0), current.mesh_generation)
        .is_ok());
    assert!(matches!(
        state.prepare(&frame(binding(), 99, 0), current.mesh_generation),
        Err(SourceFrameError::Stale)
    ));
}

#[test]
fn detached_publication_rejects_late_invalid_stream_and_preserves_prior_sequence() {
    let mut state = SourceFrameState::default();
    state.bind(binding()).unwrap();
    let first = frame(binding(), 1, 10);
    let (next, receipt) = state.prepare(&first, 3).unwrap();
    state = next;
    assert_eq!(receipt.bounds, [[0., 0., 0.], [1., 1., 0.]]);
    let mut bad = TANGENTS;
    bad[2][3] = 0.;
    let mut rejected = frame(binding(), 2, 20);
    rejected.tangents = &bad;
    assert!(matches!(
        state.prepare(&rejected, 3),
        Err(SourceFrameError::Attributes)
    ));
    assert!(state.prepare(&frame(binding(), 2, 20), 3).is_ok());
    assert!(matches!(
        state.prepare(&first, 3),
        Err(SourceFrameError::Sequence)
    ));
}

#[test]
fn seek_epoch_rejects_even_later_old_work_and_permits_backward_exact_tick() {
    let mut state = SourceFrameState::default();
    state.bind(binding()).unwrap();
    state = state.prepare(&frame(binding(), 10, 100), 3).unwrap().0;
    let newer = state.advance_epoch(binding(), 2).unwrap();
    assert!(matches!(
        state.prepare(&frame(binding(), 999, 999), 3),
        Err(SourceFrameError::Stale)
    ));
    let (_, receipt) = state.prepare(&frame(newer, 1, 0), 3).unwrap();
    assert_eq!(receipt.timeline_tick, 0);
    assert!(matches!(
        state.advance_epoch(binding(), 3),
        Err(SourceFrameError::Stale)
    ));
    assert!(matches!(
        state.advance_epoch(newer, 2),
        Err(SourceFrameError::Sequence)
    ));
}

#[test]
fn source_model_mesh_identity_and_clock_are_independent_fences() {
    let mut state = SourceFrameState::default();
    state.bind(binding()).unwrap();
    for changed in [
        SourceFrameBinding {
            source_generation: 8,
            ..binding()
        },
        SourceFrameBinding {
            model_sha256: [3; 32],
            ..binding()
        },
        SourceFrameBinding {
            source_sha256: [3; 32],
            ..binding()
        },
    ] {
        assert!(matches!(
            state.prepare(&frame(changed, 1, 0), 3),
            Err(SourceFrameError::Stale)
        ));
    }
    assert!(matches!(
        state.prepare(&frame(binding(), 1, 0), 4),
        Err(SourceFrameError::Stale)
    ));
    assert!(matches!(
        state.prepare(&frame(binding(), 1, i64::MAX as u64 + 1), 3),
        Err(SourceFrameError::Input)
    ));
    state.retire();
    assert!(state.externally_controlled());
    assert_eq!(state.bind(binding()), Err(SourceFrameError::Stale));
    assert!(matches!(
        state.prepare(&frame(binding(), 1, 0), 3),
        Err(SourceFrameError::Stale)
    ));
    assert!(!SourceFrameState::default().externally_controlled());
}

#[test]
fn nonfinite_nonorthogonal_and_overflowing_bounds_are_refused() {
    let mut p = POSITIONS;
    p[2][0] = f32::NAN;
    let mut f = frame(binding(), 1, 0);
    f.positions = &p;
    assert_eq!(validate_pose(&f), Err(SourceFrameError::Attributes));
    let mut huge = POSITIONS;
    huge[2][0] = f32::MAX;
    f.positions = &huge;
    assert_eq!(validate_pose(&f), Err(SourceFrameError::Attributes));
    let mut t = TANGENTS;
    t[2] = [0., 0., 1., 1.];
    f.positions = &POSITIONS;
    f.tangents = &t;
    assert_eq!(validate_pose(&f), Err(SourceFrameError::Attributes));
}

#[test]
fn canonical_model_integrity_budget_and_exact_resident_topology() {
    use crate::render::{assets::Mesh, compile_10d::compile_mesh_to_10d};
    let mesh = Mesh {
        positions: POSITIONS.to_vec(),
        triangles: vec![[0, 1, 2]],
        min: [0.; 3],
        max: [1., 1., 0.],
    };
    let bytes = compile_mesh_to_10d(&mesh).unwrap();
    let (decoded, digest) = decode_bound_model(&bytes, SourceFrameBindLimits::default()).unwrap();
    assert_ne!(digest, [0; 32]);
    validate_resident_mesh(&decoded, &decoded.positions, &[0, 1, 2]).unwrap();
    assert_eq!(
        validate_resident_mesh(&decoded, &decoded.positions, &[0, 2, 1]),
        Err(SourceFrameError::ResidentMismatch)
    );
    let mut normalized = decoded.positions.clone();
    normalized[2][0] += 0.5;
    assert_eq!(
        validate_resident_mesh(&decoded, &normalized, &[0, 1, 2]),
        Err(SourceFrameError::ResidentMismatch)
    );
    let limits = SourceFrameBindLimits {
        max_vertices: 2,
        ..Default::default()
    };
    assert!(matches!(
        decode_bound_model(&bytes, limits),
        Err(SourceFrameError::Budget)
    ));
    let limits = SourceFrameBindLimits {
        max_cold_bytes: 1,
        ..Default::default()
    };
    assert!(matches!(
        decode_bound_model(&bytes, limits),
        Err(SourceFrameError::Budget)
    ));
    let mut corrupt = bytes.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    assert!(matches!(
        decode_bound_model(&corrupt, SourceFrameBindLimits::default()),
        Err(SourceFrameError::Integrity)
    ));
}

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
#[test]
fn full_pose_preflight_and_epoch_control_are_zero_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    let mut state = SourceFrameState::default();
    state.bind(binding()).unwrap();
    assert_zero_alloc("source_pose_preflight", || {
        let (next, _) = state
            .prepare(&frame(binding(), 1, 254_016_000_000), 3)
            .unwrap();
        state = next;
        let newer = state.advance_epoch(binding(), 2).unwrap();
        assert!(state.prepare(&frame(newer, 1, 0), 3).is_ok());
    });
}
