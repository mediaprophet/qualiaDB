use crate::q42::{
    emf_profiles::{FieldClock, GridIndexing},
    field_envelope::{Family, Limits as StoreLimits, StructuralView},
};
use crate::render::{
    portal_emf_state::{EmfBackend, EmfDisplayStatus, RetainedEmf},
    portal_load_emf::*,
};
#[path = "fixtures.rs"]
mod fixtures;
use fixtures::*;
fn selection(descriptor_id: u32, frame_id: u32, clock_id: u32, stream_id: u32) -> DisplaySelection {
    DisplaySelection {
        descriptor_id,
        frame_id,
        clock_id,
        stream_id,
    }
}
#[test]
fn anisotropic_bounds_are_interleaved_and_local_frame_source_stays_exact() {
    let mut grid = spec();
    grid.dimensions = [2, 3, 4];
    grid.brick_dimensions = grid.dimensions;
    grid.spacing_m = [0.5, 2.0, 3.0];
    let bytes = encode(
        grid,
        &[Part {
            key: 0,
            start: 0,
            times: 0,
            stream: 0,
            electric: [3.0, 4.0, 0.0],
        }],
        None,
        clock(0),
        [0.0, 0.0, 1.0, 0.0],
    );
    let loaded = decode_emf_with_selection(&bytes, None, PortalEmfLimits::default()).unwrap();
    assert_eq!(loaded.dimensions(), [2, 3, 4, 1]);
    assert_eq!(loaded.cell_count(), 24);
    assert_eq!(loaded.local_bounds(), [-0.25, 0.75, -1.0, 5.0, -1.5, 10.5]);
    assert_eq!(loaded.source_bytes(), bytes);
    assert_eq!(loaded.selection(), selection(5, 2, 1, 0));
    assert_eq!(loaded.time_ticks(), &[0]);
    assert!(!loaded.is_temporal());
    assert!(loaded
        .display_cells()
        .iter()
        .all(|c| c.valid && c.magnitude_v_per_m == 5.0));
}
#[test]
fn sparse_linear_morton_and_hilbert_missing_cells_are_transparent() {
    for indexing in [
        GridIndexing::Linear,
        GridIndexing::Morton { order: 2 },
        GridIndexing::Hilbert { order: 2 },
    ] {
        let mut grid = spec();
        grid.dimensions = [5, 3, 2];
        grid.brick_dimensions = [2, 2, 2];
        grid.indexing = indexing;
        let layout = grid.validate().unwrap();
        let parts = [
            Part {
                key: layout.encode_key([0, 0, 0]).unwrap(),
                start: 0,
                times: 0,
                stream: 0,
                electric: [3.0, 4.0, 0.0],
            },
            Part {
                key: layout.encode_key([2, 1, 0]).unwrap(),
                start: 0,
                times: 0,
                stream: 0,
                electric: [3.0, 4.0, 0.0],
            },
        ];
        let bytes = encode(grid, &parts, None, clock(0), [0.0, 0.0, 0.0, 1.0]);
        let loaded = decode_emf_with_selection(&bytes, None, PortalEmfLimits::default()).unwrap();
        assert_eq!(loaded.dimensions(), [5, 3, 2, 1]);
        assert_eq!(
            loaded.display_cells().iter().filter(|c| c.valid).count(),
            10
        );
        assert!(loaded
            .display_cells()
            .iter()
            .filter(|c| !c.valid)
            .all(|c| c.magnitude_v_per_m.to_bits() == 0));
        assert!(loaded.display_cells()[0].valid);
        assert!(!loaded.display_cells()[2].valid);
        assert!(loaded.display_cells()[14].valid);
    }
}
#[test]
fn valid_zero_is_opaque_and_masked_invalid_is_transparent_with_orphans_rejected() {
    let original = simple([0.0; 3]);
    let loaded = decode_emf_scientific(&original).unwrap();
    assert!(loaded
        .display_cells()
        .iter()
        .all(|c| c.valid && c.magnitude_v_per_m == 0.0));
    let bytes = masked(&original, &[1, 0], false);
    let loaded = decode_emf_scientific(&bytes).unwrap();
    assert!(loaded.display_cells()[0].valid);
    assert!(!loaded.display_cells()[1].valid);
    assert_eq!(loaded.source_bytes(), bytes);
    assert!(matches!(
        decode_emf_with_selection(
            &masked(&original, &[1, 1], true),
            None,
            PortalEmfLimits::default()
        ),
        Err(Error::OrphanValidityMask)
    ));
}
#[test]
fn exact_distinct_ticks_have_no_invented_uniform_time_axis_and_overlaps_reject() {
    let parts = [
        Part {
            key: 0,
            start: 100,
            times: 2,
            stream: 1,
            electric: [3.0, 4.0, 0.0],
        },
        Part {
            key: 0,
            start: 200,
            times: 1,
            stream: 1,
            electric: [0.0; 3],
        },
    ];
    let bytes = encode(spec(), &parts, Some(5), clock(2), [0.0, 0.0, 0.0, 1.0]);
    let loaded = decode_emf_scientific(&bytes).unwrap();
    assert_eq!(loaded.time_ticks(), &[100, 105, 200]);
    assert_eq!(loaded.dimensions(), [2, 1, 1, 3]);
    assert!(loaded.is_temporal());
    assert_eq!(loaded.selection(), selection(5, 2, 1, 1));
    assert_eq!(loaded.display_cells()[4].magnitude_v_per_m, 0.0);
    assert!(loaded.display_cells()[4].valid);
    let overlap = [
        parts[0],
        Part {
            start: 105,
            times: 2,
            ..parts[1]
        },
    ];
    let bytes = encode(spec(), &overlap, Some(5), clock(2), [0.0, 0.0, 0.0, 1.0]);
    assert!(matches!(
        decode_emf_with_selection(&bytes, None, PortalEmfLimits::default()),
        Err(Error::Overlap)
    ));
}
#[test]
fn separate_descriptor_frame_and_clock_groups_require_selection_and_all_chunks_verify() {
    let a = encode(
        spec(),
        &[Part {
            key: 0,
            start: 100,
            times: 1,
            stream: 1,
            electric: [3.0, 4.0, 0.0],
        }],
        Some(1),
        clock(0),
        [0.0, 0.0, 0.0, 1.0],
    );
    let b = encode(
        spec(),
        &[Part {
            key: 0,
            start: 200,
            times: 1,
            stream: 2,
            electric: [0.0; 3],
        }],
        Some(1),
        FieldClock {
            epoch_seconds: 9,
            ..clock(2)
        },
        [0.0, 0.0, 0.0, 1.0],
    );
    let mut bytes = merge(&a, &b);
    assert!(matches!(
        decode_emf_with_selection(&bytes, None, PortalEmfLimits::default()),
        Err(Error::AmbiguousSelection)
    ));
    let loaded = decode_emf_with_selection(
        &bytes,
        Some(selection(5, 2, 1, 1)),
        PortalEmfLimits::default(),
    )
    .unwrap();
    assert_eq!(loaded.time_ticks(), &[100]);
    assert!(loaded
        .display_cells()
        .iter()
        .all(|c| c.magnitude_v_per_m == 5.0));
    let other = decode_emf_with_selection(
        &bytes,
        Some(selection(10, 7, 6, 2)),
        PortalEmfLimits::default(),
    )
    .unwrap();
    assert_eq!(other.time_ticks(), &[200]);
    assert!(other
        .display_cells()
        .iter()
        .all(|c| c.valid && c.magnitude_v_per_m == 0.0));
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let at = view.entry(2).unwrap().payload_offset() as usize;
    bytes[at] ^= 1;
    assert!(decode_emf_with_selection(
        &bytes,
        Some(selection(5, 2, 1, 1)),
        PortalEmfLimits::default()
    )
    .is_err());
}
#[test]
fn finite_vector_hypot_overflow_is_display_only_and_source_is_not_rejected() {
    let bytes = simple([f64::MAX, f64::MAX, 0.0]);
    let loaded = decode_emf_scientific(&bytes).unwrap();
    assert!(loaded.display_cells()[0].magnitude_v_per_m.is_infinite());
    assert!(loaded.display_cells()[0].valid);
    assert_eq!(loaded.source_bytes(), bytes);
    let mut state = RetainedEmf::default();
    state
        .accept(loaded, EmfBackend::WebGpu, |field| {
            assert!(field.display_cells()[0].magnitude_v_per_m.is_infinite());
            Ok(())
        })
        .unwrap();
    assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
}
#[test]
fn bounds_out_of_display_range_and_explicit_budgets_reject_without_source_changes() {
    let mut grid = spec();
    grid.spacing_m = [1e300, 2.0, 3.0];
    let bytes = encode(
        grid,
        &[Part {
            key: 0,
            start: 0,
            times: 0,
            stream: 0,
            electric: [3., 4., 0.],
        }],
        None,
        clock(0),
        [0., 0., 0., 1.],
    );
    let before = bytes.clone();
    assert!(matches!(
        decode_emf_with_selection(&bytes, None, PortalEmfLimits::default()),
        Err(Error::UnsupportedProjection("unsupported-display-range"))
    ));
    assert_eq!(bytes, before);
    let bytes = simple([3., 4., 0.]);
    for limits in [
        PortalEmfLimits {
            source_bytes: 1,
            ..Default::default()
        },
        PortalEmfLimits {
            peak_owned_bytes: 1,
            ..Default::default()
        },
        PortalEmfLimits {
            display_cells: 1,
            ..Default::default()
        },
        PortalEmfLimits {
            projection_work: 0,
            ..Default::default()
        },
        PortalEmfLimits {
            total_payload_work: 0,
            ..Default::default()
        },
    ] {
        assert!(decode_emf_with_selection(&bytes, None, limits).is_err());
    }
    assert!(decode_emf_with_selection(
        &bytes,
        Some(selection(99, 2, 1, 0)),
        PortalEmfLimits::default()
    )
    .is_err());
}
#[test]
fn retained_source_replays_after_device_loss_and_failed_replacement_is_atomic() {
    let bytes = simple([3., 4., 0.]);
    let loaded = decode_emf_scientific(&bytes).unwrap();
    let mut state = RetainedEmf::default();
    state
        .accept(loaded, EmfBackend::Pending, |_| panic!("no device"))
        .unwrap();
    assert_eq!(state.status(), EmfDisplayStatus::Pending);
    for _ in 0..2 {
        state
            .replay(|field| {
                assert_eq!(field.source_bytes(), bytes);
                assert_eq!(field.time_ticks(), &[0]);
                assert_eq!(field.selection(), selection(5, 2, 1, 0));
                Ok(())
            })
            .unwrap();
        assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
        state.invalidate(true);
    }
    state.replay(|_| Ok(())).unwrap();
    let next = decode_emf_scientific(&simple([0.; 3])).unwrap();
    assert!(state
        .accept(next, EmfBackend::WebGpu, |_| Err("budget".into()))
        .is_err());
    assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
    assert_eq!(state.accepted().unwrap().source_bytes(), bytes);
    let mut invalid = decode_emf_scientific(&bytes).unwrap();
    invalid.nx = u32::MAX;
    assert!(state
        .accept(invalid, EmfBackend::WebGpu, |_| panic!("invalid upload"))
        .is_err());
    assert_eq!(state.accepted().unwrap().source_bytes(), bytes);
}
#[test]
fn unavailable_and_failed_replay_keep_exact_accepted_scientific_source() {
    let bytes = simple([3., 4., 0.]);
    let mut state = RetainedEmf::default();
    state
        .accept(
            decode_emf_scientific(&bytes).unwrap(),
            EmfBackend::Unavailable,
            |_| panic!("unavailable backend"),
        )
        .unwrap();
    assert_eq!(state.status(), EmfDisplayStatus::Unavailable);
    assert!(state.replay(|_| Err("device limits".into())).is_err());
    assert_eq!(state.accepted().unwrap().source_bytes(), bytes);
    state.replay(|_| Ok(())).unwrap();
    assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
    state.invalidate(false);
    assert_eq!(state.status(), EmfDisplayStatus::Unavailable);
    let mut corrupt = bytes.clone();
    corrupt[0] ^= 1;
    assert!(decode_emf_scientific(&corrupt).is_err());
    assert_eq!(state.accepted().unwrap().source_bytes(), bytes);
}

#[path = "extended_test_builders.rs"]
mod extended_test_builders;
#[path = "extended_tests.rs"]
mod extended_tests;
