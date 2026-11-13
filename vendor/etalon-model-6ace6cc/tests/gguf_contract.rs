use etalon_model::gguf::*;
use std::cell::Cell;
#[path = "support/gguf_fixture.rs"]
mod fixture;

fn limits() -> ParseLimits {
    ParseLimits {
        max_metadata_bytes: 16384,
        max_nesting_depth: 32,
        max_work: 4096,
        versions: VersionPolicy::V2V3,
    }
}

#[test]
fn all_thirteen_types_preserve_raw_values_order_names_and_full_rank() {
    let mut bytes = fixture::complete();
    let end = bytes.len();
    bytes.extend_from_slice(&[0xa5; 17]);
    let view = MetadataView::parse(&bytes, limits()).unwrap();
    assert_eq!(view.header().version, 3);
    assert_eq!(view.header().kv_count, 13);
    assert_eq!(view.header().tensor_count, 3);
    assert_eq!(view.metadata_end(), end);
    assert_eq!(view.metadata_bytes(), &bytes[..end]);
    assert_eq!(view.metadata_bytes().as_ptr(), bytes.as_ptr());
    for (item, (kind, expected)) in view.kvs().zip(fixture::payloads()) {
        let item = item.unwrap();
        assert_eq!(item.key, b"duplicate.raw\xff");
        assert_eq!(item.value_type, kind);
        assert_eq!(item.encoded_value, expected);
        assert!((bytes.as_ptr() as usize..bytes.as_ptr() as usize + end)
            .contains(&(item.encoded_value.as_ptr() as usize)));
    }
    let mut tensors = view.tensors();
    let first = tensors.next().unwrap().unwrap();
    assert_eq!(first.name, b"x");
    assert_eq!(first.rank, 5);
    assert_eq!(first.ggml_type, u32::MAX);
    assert_eq!(first.offset, u64::MAX);
    assert_eq!(first.dimensions_bytes().len(), 40);
    for i in 0..5 {
        assert_eq!(first.dimension(i), Some(i as u64 + 2));
    }
    assert_eq!(first.dimension(5), None);
    assert_eq!(first.name_hash(), 12_638_214_688_346_347_271);
    assert_eq!(fnv1a64(b"x"), 12_638_214_688_346_347_271);
    assert!(
        first.name_hash() > 0x0fff_ffff_ffff_ffff,
        "full64 hash must not become Qualia's masked q_hash"
    );
    let second = tensors.next().unwrap().unwrap();
    assert_eq!(second.name, b"x");
    assert_eq!(second.rank, 0);
    assert_eq!(second.dimension(0), None);
    let third = tensors.next().unwrap().unwrap();
    assert_eq!(third.name, b"");
    assert_eq!(third.dimension(0), Some(0));
    assert_eq!(third.ggml_type, 42);
    assert!(tensors.next().is_none());
    assert!(tensors.next().is_none());
}

#[test]
fn explicit_version_and_alignment_policies_keep_metadata_only_capability() {
    for version in [0, 1, 2, 3, 4, u32::MAX] {
        let bytes = fixture::header(version, 0, 0);
        assert_eq!(
            MetadataView::parse(&bytes, limits()).is_ok(),
            matches!(version, 2 | 3)
        );
        let mut compatible = limits();
        compatible.versions = VersionPolicy::AtLeast2;
        assert_eq!(
            MetadataView::parse(&bytes, compatible).is_ok(),
            version >= 2
        );
    }
    let mut bytes = fixture::header(3, 1, 1);
    fixture::kv(&mut bytes, b"general.alignment", 4, &64u32.to_le_bytes());
    fixture::tensor(&mut bytes, b"x", &[2], 0, 0);
    assert_eq!(bytes.len(), 90);
    let view = MetadataView::parse(&bytes, limits()).unwrap();
    assert_eq!(view.tensor_data_start(32).unwrap(), 96);
    assert_eq!(view.tensor_data_start(64).unwrap(), 128);
    assert_eq!(view.tensor_data_start(1).unwrap(), 90);
    assert_eq!(view.tensor_data_start(1u64 << 63).unwrap(), 1u64 << 63);
    for alignment in [0, 3, u64::MAX] {
        assert!(matches!(
            view.tensor_data_start(alignment).unwrap_err().kind,
            ParseErrorKind::InvalidAlignment
        ));
    }
}

#[test]
fn work_is_cumulative_and_metadata_byte_budget_includes_header() {
    let bytes = fixture::complete();
    let mut exact = limits();
    exact.max_metadata_bytes = bytes.len();
    // 13 KVs + 13 values + 2 array elements + 3 tensor records + 6 dimensions.
    exact.max_work = 37;
    assert!(MetadataView::parse(&bytes, exact).is_ok());
    exact.max_work = 36;
    assert!(matches!(
        MetadataView::parse(&bytes, exact).unwrap_err().kind,
        ParseErrorKind::WorkLimit
    ));
    exact.max_work = 37;
    exact.max_metadata_bytes -= 1;
    assert!(matches!(
        MetadataView::parse(&bytes, exact).unwrap_err().kind,
        ParseErrorKind::MetadataLimit
    ));
    let empty = fixture::header(3, 0, 0);
    let mut zero = limits();
    zero.max_work = 0;
    zero.max_nesting_depth = 0;
    zero.max_metadata_bytes = 24;
    assert!(MetadataView::parse(&empty, zero).is_ok());
    zero.max_metadata_bytes = 23;
    assert!(matches!(
        MetadataView::parse(&empty, zero).unwrap_err().kind,
        ParseErrorKind::MetadataLimit
    ));
    let nested = fixture::nested(2);
    let mut bounded = limits();
    bounded.max_work = 4;
    // KV + two array headers + one scalar = 4; each nesting level is charged.
    assert!(MetadataView::parse(&nested, bounded).is_ok());
    bounded.max_work = 3;
    assert!(matches!(
        MetadataView::parse(&nested, bounded).unwrap_err().kind,
        ParseErrorKind::WorkLimit
    ));
}

#[test]
fn depth_zero_thirty_two_and_thirty_three_are_bounded_without_recursion() {
    assert_eq!(MAX_ARRAY_DEPTH, 32);
    let mut scalar = fixture::header(3, 0, 1);
    fixture::kv(&mut scalar, b"x", 0, &[1]);
    let mut zero = limits();
    zero.max_nesting_depth = 0;
    assert!(MetadataView::parse(&scalar, zero).is_ok());
    assert!(matches!(
        MetadataView::parse(&fixture::nested(1), zero)
            .unwrap_err()
            .kind,
        ParseErrorKind::DepthLimit
    ));
    assert!(MetadataView::parse(&fixture::nested(32), limits()).is_ok());
    assert!(matches!(
        MetadataView::parse(&fixture::nested(33), limits())
            .unwrap_err()
            .kind,
        ParseErrorKind::DepthLimit
    ));
    let mut invalid = limits();
    invalid.max_nesting_depth = 33;
    assert!(matches!(
        MetadataView::parse(&scalar, invalid).unwrap_err().kind,
        ParseErrorKind::InvalidLimits
    ));
}

#[test]
fn strict_failures_are_distinct_from_legacy_recovery_and_prefix_is_sticky() {
    for (name, bytes) in fixture::strict_divergences() {
        assert!(
            MetadataView::parse(&bytes, limits()).is_err(),
            "{name} strict divergence"
        );
    }
    let mut bytes = fixture::header(3, 0, 2);
    fixture::kv(&mut bytes, b"first", 4, &7u32.to_le_bytes());
    let prefix_end = bytes.len();
    fixture::string(&mut bytes, b"second");
    bytes.extend_from_slice(&99u32.to_le_bytes());
    assert!(MetadataView::parse(&bytes, limits()).is_err());
    let mut cursor = KvCursor::new(&bytes, limits()).unwrap();
    assert_eq!(cursor.header().kv_count, 2);
    let first = cursor.next().unwrap().unwrap();
    assert_eq!(first.key, b"first");
    assert_eq!(cursor.offset(), prefix_end);
    let error = cursor.next().unwrap().unwrap_err();
    assert!(matches!(error.kind, ParseErrorKind::UnknownValueType(99)));
    assert_eq!(cursor.failure(), Some(error));
    assert!(!cursor.is_complete());
    let offset = cursor.offset();
    for _ in 0..3 {
        assert!(cursor.next().is_none());
        assert_eq!(cursor.offset(), offset);
        assert_eq!(cursor.failure(), Some(error));
    }
    let empty = fixture::header(3, 0, 0);
    let mut cursor = KvCursor::new(&empty, limits()).unwrap();
    assert!(cursor.next().is_none());
    assert!(cursor.is_complete());
    assert_eq!(cursor.failure(), None);
}

#[test]
fn every_truncated_prefix_and_extreme_declared_length_fail_closed() {
    let bytes = fixture::complete();
    for end in 0..bytes.len() {
        assert!(
            MetadataView::parse(&bytes[..end], limits()).is_err(),
            "truncation {end}"
        );
    }
    for count_offset in [8, 16] {
        let mut bytes = fixture::header(3, 0, 0);
        bytes[count_offset..count_offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(MetadataView::parse(&bytes, limits()).is_err());
    }
    let mut huge_name = fixture::header(3, 1, 0);
    huge_name.extend_from_slice(&u64::MAX.to_le_bytes());
    assert!(MetadataView::parse(&huge_name, limits()).is_err());
    let mut huge_key = fixture::header(3, 0, 1);
    huge_key.extend_from_slice(&u64::MAX.to_le_bytes());
    assert!(MetadataView::parse(&huge_key, limits()).is_err());
    let mut array = 0u32.to_le_bytes().to_vec();
    array.extend_from_slice(&u64::MAX.to_le_bytes());
    let mut huge_array = fixture::header(3, 0, 1);
    fixture::kv(&mut huge_array, b"a", 9, &array);
    assert!(MetadataView::parse(&huge_array, limits()).is_err());
    let mut rank = fixture::header(3, 1, 0);
    fixture::string(&mut rank, b"x");
    rank.extend_from_slice(&u32::MAX.to_le_bytes());
    assert!(MetadataView::parse(&rank, limits()).is_err());
}

struct Counter;
thread_local! {static ACTIVE:Cell<bool>=const{Cell::new(false)};static COUNT:Cell<usize>=const{Cell::new(0)};}
fn count() {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = COUNT.try_with(|count| count.set(count.get() + 1));
        }
    });
}
unsafe impl std::alloc::GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        count();
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: std::alloc::Layout) -> *mut u8 {
        count();
        unsafe { std::alloc::System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        count();
        unsafe { std::alloc::System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;

#[test]
fn parsing_iteration_and_all_bounded_rejections_allocate_zero_with_positive_control() {
    ACTIVE.with(|active| active.set(true));
    let control = std::hint::black_box(Box::new([1u8; 32]));
    std::hint::black_box(&control);
    drop(control);
    ACTIVE.with(|active| active.set(false));
    assert!(COUNT.with(Cell::get) > 0);
    let bytes = fixture::complete();
    let failures = fixture::strict_divergences();
    let deep = fixture::nested(33);
    COUNT.with(|count| count.set(0));
    ACTIVE.with(|active| active.set(true));
    let view = MetadataView::parse(&bytes, limits()).unwrap();
    let mut visited = 0;
    for item in view.kvs() {
        std::hint::black_box(item.unwrap());
        visited += 1;
    }
    for item in view.tensors() {
        std::hint::black_box(item.unwrap());
        visited += 1;
    }
    for end in 0..bytes.len() {
        std::hint::black_box(MetadataView::parse(&bytes[..end], limits()).is_err());
    }
    for (_, bytes) in &failures {
        std::hint::black_box(MetadataView::parse(bytes, limits()).is_err());
    }
    std::hint::black_box(MetadataView::parse(&deep, limits()).is_err());
    ACTIVE.with(|active| active.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
    assert_eq!(visited, 16);
}
