//! Bounded SDK contract tests; all fixtures are synthetic bytes, not assets.
use qualia_core_db::container_10d::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;
thread_local! { static ALLOCS: Cell<Option<usize>> = const { Cell::new(None) }; }
fn count() {
    let _ = ALLOCS.try_with(|slot| {
        if let Some(value) = slot.get() {
            slot.set(Some(value + 1));
        }
    });
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

struct Measurement;
impl Drop for Measurement {
    fn drop(&mut self) {
        ALLOCS.with(|slot| slot.set(None));
    }
}
fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    ALLOCS.with(|slot| {
        assert!(slot.get().is_none());
        slot.set(Some(0));
    });
    let guard = Measurement;
    let value = f();
    let calls = ALLOCS.with(|slot| slot.get().unwrap());
    drop(guard);
    (value, calls)
}

const LIMITS: MeshFrameDecodeLimits = MeshFrameDecodeLimits {
    max_vertices: 3,
    max_payload_bytes: 52,
    max_decoded_bytes: 84,
};
fn frames(normals: bool, tangents: bool) -> Vec<u8> {
    let n = [[0.0, 0.0, 1.0]; 3];
    let t = [[1.0, 0.0, 0.0, -1.0]; 3];
    let mut bytes = vec![0; frame_encoded_len(3, normals, tangents).unwrap()];
    encode_mesh_frames(
        normals.then_some(&n[..]),
        tangents.then_some(&t[..]),
        &mut bytes,
    )
    .unwrap();
    bytes
}
fn envelope(payload: &[u8], manifest: Option<bool>) -> Vec<u8> {
    let section = SectionInput {
        section_type: SectionType::FieldSidecar,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload,
    };
    let mut out = vec![0; 8192];
    let size = if let Some(surface) = manifest {
        let identity = ManifoldIdentityV2 {
            entity_id: StableEntityId([7; 16]),
            profile_id: 1,
            domain_id: 2,
            coordinate_convention_id: 3,
            coordinates: [0.0; 10],
            external_digest: None,
        };
        let fields = [TypedFieldRecord {
            field_id: 10,
            kind: if surface {
                FIELD_KIND_SURFACE_FRAMES
            } else {
                FIELD_KIND_VECTOR
            },
            flags: 0,
            unit_id: 0,
            payload_section_id: SectionType::FieldSidecar as u32,
            // Explicitly exercise unaligned codec bytes inside the envelope.
            payload_offset: 1,
            payload_length: (payload.len() - 1) as u64,
        }];
        let mut scratch = [0; MANIFOLD_IDENTITY_V2_MAX_BYTES];
        let mut sections = [section; 2];
        encode_container_with_manifold_identity_extensions(
            &Container10dHeader::proposed(),
            &[section],
            &identity,
            &fields,
            &[],
            &mut scratch,
            &mut sections,
            &mut out,
        )
        .unwrap()
    } else {
        encode_container(&Container10dHeader::proposed(), &[section], &mut out).unwrap()
    };
    out.truncate(size);
    seal_whole_file_crc32c(&mut out);
    out
}

#[test]
fn raw_unaligned_and_optional_streams_preserve_tails_and_match_old_api() {
    for (normals, tangents) in [(true, false), (false, true), (true, true)] {
        let bytes = frames(normals, tangents);
        let old = decode_mesh_frames(&bytes).unwrap();
        let mut unaligned = vec![0; bytes.len() + 1];
        unaligned[1..].copy_from_slice(&bytes);
        let mut n = [[9.0; 3]; 4];
        let mut t = [[8.0; 4]; 4];
        let layout = decode_mesh_frames_into(&unaligned[1..], LIMITS, &mut n, &mut t).unwrap();
        assert_eq!(
            (layout.count, layout.has_normals, layout.has_tangents),
            (3, normals, tangents)
        );
        assert_eq!(
            layout.decoded_bytes,
            3 * (usize::from(normals) * 12 + usize::from(tangents) * 16)
        );
        if normals {
            assert_eq!(&n[..3], old.normals.as_deref().unwrap());
        } else {
            assert_eq!(n, [[9.0; 3]; 4]);
        }
        if tangents {
            assert_eq!(&t[..3], old.tangents.as_deref().unwrap());
        } else {
            assert_eq!(t, [[8.0; 4]; 4]);
        }
        assert_eq!(n[3], [9.0; 3]);
        assert_eq!(t[3], [8.0; 4]);
        assert_eq!(decode_mesh_frames_with_limits(&bytes, LIMITS).unwrap(), old);
    }
}

#[test]
fn both_capacities_are_checked_before_either_stream_changes() {
    let bytes = frames(true, true);
    for (n_len, t_len) in [(2, 3), (3, 2), (0, 0)] {
        let mut n = [[9.0; 3]; 3];
        let mut t = [[8.0; 4]; 3];
        assert!(matches!(
            decode_mesh_frames_into(&bytes, LIMITS, &mut n[..n_len], &mut t[..t_len]),
            Err(NormalSectionError::OutputTooSmall { .. })
        ));
        assert_eq!(n, [[9.0; 3]; 3]);
        assert_eq!(t, [[8.0; 4]; 3]);
    }
    let mut n = [[9.0; 3]; 3];
    decode_mesh_frames_into(&frames(true, false), LIMITS, &mut n, &mut []).unwrap();
    let mut t = [[8.0; 4]; 3];
    decode_mesh_frames_into(&frames(false, true), LIMITS, &mut [], &mut t).unwrap();
}

#[test]
fn late_invalid_tangent_is_atomic_and_allocates_nothing_even_in_cold_api() {
    for corrupt_direction in [false, true] {
        let mut bytes = frames(true, true);
        if corrupt_direction {
            bytes[44..50].fill(0);
        } else {
            bytes[50..52].fill(0);
        }
        let mut n = [[9.0; 3]; 3];
        let mut t = [[8.0; 4]; 3];
        let (result, calls) = measured(|| decode_mesh_frames_into(&bytes, LIMITS, &mut n, &mut t));
        assert_eq!(calls, 0);
        assert_eq!(result, Err(NormalSectionError::InvalidTangent { index: 2 }));
        assert_eq!(n, [[9.0; 3]; 3]);
        assert_eq!(t, [[8.0; 4]; 3]);
        let (result, calls) = measured(|| decode_mesh_frames_with_limits(&bytes, LIMITS));
        assert_eq!(calls, 0);
        assert_eq!(result, Err(NormalSectionError::InvalidTangent { index: 2 }));
    }
}

#[test]
fn independent_limits_and_malformed_headers_refuse_without_mutation() {
    let bytes = frames(true, true);
    for limits in [
        MeshFrameDecodeLimits {
            max_vertices: 2,
            ..LIMITS
        },
        MeshFrameDecodeLimits {
            max_payload_bytes: 51,
            ..LIMITS
        },
        MeshFrameDecodeLimits {
            max_decoded_bytes: 83,
            ..LIMITS
        },
    ] {
        let mut n = [[9.0; 3]; 3];
        let mut t = [[8.0; 4]; 3];
        let (result, calls) = measured(|| decode_mesh_frames_into(&bytes, limits, &mut n, &mut t));
        assert!(result.is_err());
        assert_eq!(calls, 0);
        assert_eq!(n, [[9.0; 3]; 3]);
        assert_eq!(t, [[8.0; 4]; 3]);
        let (result, calls) = measured(|| decode_mesh_frames_with_limits(&bytes, limits));
        assert!(result.is_err());
        assert_eq!(calls, 0);
    }
    for (offset, value) in [(0, b'X'), (4, 2), (6, 4), (6, 0), (12, 1), (8, 4)] {
        let mut malformed = bytes.clone();
        malformed[offset] = value;
        let mut n = [[9.0; 3]; 3];
        let mut t = [[8.0; 4]; 3];
        assert!(decode_mesh_frames_into(&malformed, LIMITS, &mut n, &mut t).is_err());
        assert_eq!(n, [[9.0; 3]; 3]);
        assert_eq!(t, [[8.0; 4]; 3]);
    }
    for size in [0, 4, 15, 16, 51] {
        assert!(inspect_mesh_frames(&bytes[..size], LIMITS).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(matches!(
        inspect_mesh_frames(&extra, MeshFrameDecodeLimits::COMPATIBILITY),
        Err(NormalSectionError::PayloadLength { .. })
    ));
    let mut legacy = frames(true, false);
    legacy[6] = 2;
    assert_eq!(
        inspect_mesh_frames(&legacy, LIMITS),
        Err(NormalSectionError::UnsupportedEncoding(2))
    );
}

#[test]
fn empty_active_stream_remains_present_and_hard_count_cap_remains_enforced() {
    let mut bytes = [0; 16];
    encode_mesh_frames(None, Some(&[]), &mut bytes).unwrap();
    let layout = decode_mesh_frames_into(&bytes, LIMITS, &mut [], &mut []).unwrap();
    assert_eq!(layout.count, 0);
    assert!(layout.has_tangents);
    assert!(!layout.has_normals);
    assert_eq!(decode_mesh_frames(&bytes).unwrap().tangents, Some(vec![]));
    bytes[8..12].copy_from_slice(&((MAX_NORMAL_COUNT + 1) as u32).to_le_bytes());
    assert!(matches!(
        inspect_mesh_frames(
            &bytes,
            MeshFrameDecodeLimits {
                max_vertices: usize::MAX,
                max_payload_bytes: usize::MAX,
                max_decoded_bytes: usize::MAX,
            }
        ),
        Err(NormalSectionError::CountTooLarge {
            max: MAX_NORMAL_COUNT,
            ..
        })
    ));
}

#[test]
fn frm1_normals_only_and_legacy_extreme_oct_codes_keep_accepted_semantics() {
    let mut legacy = frames(true, false);
    legacy[16..18].copy_from_slice(&i16::MIN.to_le_bytes());
    let expected = decode_mesh_frames(&legacy).unwrap();
    let mut frm = legacy.clone();
    frm[..4].copy_from_slice(b"FRM1");
    let mut n = [[9.0; 3]; 3];
    let layout = decode_mesh_frames_into(&frm, LIMITS, &mut n, &mut []).unwrap();
    assert!(layout.has_normals);
    assert!(!layout.has_tangents);
    assert_eq!(&n, expected.normals.as_deref().unwrap());
    assert_eq!(decode_mesh_frames(&frm).unwrap(), expected);
}

#[test]
fn typed_legacy_absence_mismatch_and_late_failure_share_canonical_atomic_path() {
    let bytes = frames(true, true);
    let mut prefixed = vec![0];
    prefixed.extend_from_slice(&bytes);
    for data in [envelope(&bytes, None), envelope(&prefixed, Some(true))] {
        let mut n = [[9.0; 3]; 3];
        let mut t = [[8.0; 4]; 3];
        let (result, calls) =
            measured(|| read_container_mesh_frames_into(&data, 3, LIMITS, &mut n, &mut t));
        assert_eq!(calls, 0);
        assert_eq!(result.unwrap().unwrap().count, 3);
        assert_eq!(n, [[0.0, 0.0, 1.0]; 3]);
        assert_eq!(t, [[1.0, 0.0, 0.0, -1.0]; 3]);
        let old = read_container_mesh_frames(&data, 3).unwrap();
        assert_eq!(
            read_container_mesh_frames_with_limits(&data, 3, LIMITS).unwrap(),
            old
        );
        let result = read_container_mesh_frames_into(&data, 2, LIMITS, &mut n, &mut t);
        assert_eq!(
            result,
            Err(TypedPayloadError::VertexCount {
                expected: 2,
                got: 3
            })
        );
        assert_eq!(n, [[0.0, 0.0, 1.0]; 3]);
        let (result, calls) =
            measured(|| read_container_mesh_frames_into(&data, 3, LIMITS, &mut n, &mut t[..2]));
        assert!(result.is_err());
        assert_eq!(calls, 0);
        assert_eq!(n, [[0.0, 0.0, 1.0]; 3]);
    }
    let data = envelope(&prefixed, Some(false));
    let mut n = [[9.0; 3]; 3];
    let mut t = [[8.0; 4]; 3];
    assert_eq!(
        read_container_mesh_frames_into(&data, 3, LIMITS, &mut n, &mut t).unwrap(),
        None
    );
    assert_eq!(n, [[9.0; 3]; 3]);
    assert_eq!(t, [[8.0; 4]; 3]);
    prefixed[51..53].fill(0);
    let data = envelope(&prefixed, Some(true));
    let (result, calls) =
        measured(|| read_container_mesh_frames_into(&data, 3, LIMITS, &mut n, &mut t));
    assert_eq!(
        result,
        Err(TypedPayloadError::Frames(
            NormalSectionError::InvalidTangent { index: 2 }
        ))
    );
    assert_eq!(calls, 0);
    assert_eq!(n, [[9.0; 3]; 3]);
    assert_eq!(t, [[8.0; 4]; 3]);
}
