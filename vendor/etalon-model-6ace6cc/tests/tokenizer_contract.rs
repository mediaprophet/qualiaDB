use etalon_model::{gguf::*, tokenizer::*};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    mem::size_of,
};

const FIXTURE: &[u8] = include_bytes!("support/tokenizer_contract.gguf");
struct Counter;
thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; static COUNT: Cell<usize> = const { Cell::new(0) }; }
fn tick() {
    if ACTIVE.try_with(Cell::get).unwrap_or(false) {
        COUNT.with(|n| n.set(n.get() + 1));
    }
}
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        tick();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        tick();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, len: usize) -> *mut u8 {
        tick();
        unsafe { System.realloc(ptr, layout, len) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn measured(f: impl FnOnce()) -> usize {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|v| v.set(false));
        }
    }
    COUNT.with(|v| v.set(0));
    ACTIVE.with(|v| v.set(true));
    let reset = Reset;
    f();
    drop(reset);
    COUNT.with(Cell::get)
}
fn preparation() -> PrepareBudget {
    PrepareBudget {
        capacity_bytes: 1024 * 1024,
        max_work: 10_000_000,
    }
}
fn execution() -> ExecutionBudget {
    ExecutionBudget {
        capacity_bytes: 1024 * 1024,
        max_work: 10_000_000,
    }
}
fn parse(bytes: &[u8]) -> MetadataView<'_> {
    MetadataView::parse(
        bytes,
        ParseLimits {
            max_metadata_bytes: bytes.len(),
            max_nesting_depth: 32,
            max_work: 1_000_000,
            versions: VersionPolicy::V2V3,
        },
    )
    .unwrap()
}
fn plan_with(bytes: &[u8], run: impl FnOnce(&TokenizerPlan<'_, '_>)) {
    let view = parse(bytes);
    let r = requirements(&view, preparation()).unwrap();
    let mut v = vec![VocabEntry::default(); r.vocab];
    let mut m = vec![MergeEntry::default(); r.merges];
    let mut s = vec![SpecialEntry::default(); r.specials];
    let p = prepare(
        &view,
        TokenizerTables {
            vocab: &mut v,
            merges: &mut m,
            specials: &mut s,
        },
        preparation(),
    )
    .unwrap();
    run(&p);
}
fn raw(
    p: &TokenizerPlan<'_, '_>,
    text: &str,
    out: &mut [u32],
    budget: ExecutionBudget,
) -> Result<usize, TokenizerError> {
    let mut spans = [PretokenSpan::default(); 512];
    let mut encoded = [0xa5; 1024];
    let mut symbols = [SymbolSpan::default(); 512];
    p.encode_raw_into(
        text,
        ExecutionWorkspace {
            spans: &mut spans,
            encoded: &mut encoded,
            symbols: &mut symbols,
        },
        out,
        budget,
    )
}
fn replace_string(bytes: &mut Vec<u8>, old: &[u8], new: &[u8]) {
    let mut key = (old.len() as u64).to_le_bytes().to_vec();
    key.extend_from_slice(old);
    let pos = bytes
        .windows(key.len())
        .position(|w| w == key)
        .expect("exact fixture string");
    let mut value = (new.len() as u64).to_le_bytes().to_vec();
    value.extend_from_slice(new);
    bytes.splice(pos..pos + key.len(), value);
}

#[test]
fn independent_ids_raw_shortcuts_and_special_order() {
    let borrowed_fixture = FIXTURE;
    plan_with(borrowed_fixture, |p| {
        for (text, expected) in [
            ("a", &[258][..]),
            ("<x>", &[259]),
            ("<x><x-long><x>", &[259, 261, 259]),
            ("é", &[233]),
            ("éé", &[270]),
            ("hello", &[273]),
            (" hello", &[271]),
        ] {
            let mut out = [u32::MAX; 64];
            let n = raw(p, text, &mut out, execution()).unwrap();
            assert_eq!(&out[..n], expected, "{text:?}");
            assert!(out[n..].iter().all(|&id| id == u32::MAX));
        }
        assert_eq!(p.bos_token_id(), 263);
        assert_eq!(p.eos_token_id(), 264);
        assert!(p.add_bos_token());
        assert_eq!(p.vocab_count(), 275);
        assert_eq!(p.metadata_bytes().as_ptr(), borrowed_fixture.as_ptr());
    });
}

#[test]
fn all_byte_ids_decode_exactly_including_incomplete_utf8_and_empty_piece() {
    plan_with(FIXTURE, |p| {
        for id in 0..256 {
            let mut out = [0xa5; 32];
            assert_eq!(p.decode_token_bytes_into(id, &mut out, execution()), Ok(1));
            assert_eq!(out[0], id as u8);
            assert_eq!(&out[1..], &[0xa5; 31]);
        }
        let mut out = [0xa5; 32];
        assert_eq!(p.decode_token_bytes_into(256, &mut out, execution()), Ok(0));
        assert_eq!(out, [0xa5; 32]);
        assert_eq!(p.decode_token_bytes_into(262, &mut out, execution()), Ok(1));
        assert_eq!(out[0], b'A');
        assert_eq!(
            p.decode_token_bytes_into(262, &mut [0xa5; 1], execution()),
            Err(TokenizerError::Capacity)
        );
        assert_eq!(
            p.decode_token_bytes_into(u32::MAX, &mut out, execution()),
            Err(TokenizerError::InvalidTokenId(u32::MAX))
        );
    });
}

#[test]
fn malformed_metadata_and_duplicate_fields_refuse_without_default_tokenizer() {
    let mut bytes = FIXTURE.to_vec();
    replace_string(&mut bytes, b"gpt2", b"other-model");
    assert_eq!(
        requirements(&parse(&bytes), preparation()),
        Err(TokenizerError::UnsupportedProfile)
    );
    let mut bytes = FIXTURE.to_vec();
    replace_string(&mut bytes, b"a b", b"a  b");
    assert_eq!(
        requirements(&parse(&bytes), preparation()),
        Err(TokenizerError::MalformedMerge)
    );
    let mut bytes = FIXTURE.to_vec();
    replace_string(&mut bytes, b"tokenizer.ggml.pre", b"tokenizer.ggml.model");
    assert_eq!(
        requirements(&parse(&bytes), preparation()),
        Err(TokenizerError::DuplicateField)
    );
    let mut bytes = FIXTURE.to_vec();
    replace_string(&mut bytes, b"hello", &[0xff]);
    assert_eq!(
        requirements(&parse(&bytes), preparation()),
        Err(TokenizerError::Metadata)
    );
    assert!(MetadataView::parse(
        &FIXTURE[..FIXTURE.len() - 1],
        ParseLimits {
            max_metadata_bytes: FIXTURE.len(),
            max_nesting_depth: 32,
            max_work: 1_000_000,
            versions: VersionPolicy::V2V3
        }
    )
    .is_err());
}

#[test]
fn missing_actual_fallback_refuses_but_raw_vocabulary_shortcut_is_valid() {
    let mut bytes = FIXTURE.to_vec();
    // Byte97's standalone symbol is absent, but an exact whole-word token still exists.
    // Replace all three duplicate 'a' entries, preserving an unrelated full 'hello' token.
    for _ in 0..3 {
        replace_string(&mut bytes, b"a", b"z");
    }
    plan_with(&bytes, |p| {
        let mut out = [u32::MAX; 64];
        assert_eq!(
            raw(p, "a", &mut out, execution()),
            Err(TokenizerError::MissingByteSymbol(97))
        );
        let n = raw(p, "hello", &mut out, execution()).unwrap();
        assert_eq!(&out[..n], &[273]);
        // A merged/whole-word match can cover a missing constituent byte.
        let n = raw(p, "ab", &mut out, execution()).unwrap();
        assert_eq!(&out[..n], &[265]);
    });
}

#[test]
fn all_execution_capacity_is_charged_before_mutation() {
    plan_with(FIXTURE, |p| {
        let mut spans = [PretokenSpan { start: 91, end: 92 }; 5];
        let mut encoded = [0xa5; 9];
        let mut symbols = [SymbolSpan::default(); 7];
        let mut out = [u32::MAX; 13];
        let charged = spans.len() * size_of::<PretokenSpan>()
            + encoded.len()
            + symbols.len() * size_of::<SymbolSpan>()
            + out.len() * size_of::<u32>();
        let result = p.encode_raw_into(
            "a",
            ExecutionWorkspace {
                spans: &mut spans,
                encoded: &mut encoded,
                symbols: &mut symbols,
            },
            &mut out,
            ExecutionBudget {
                capacity_bytes: charged - 1,
                ..execution()
            },
        );
        assert_eq!(result, Err(TokenizerError::CapacityBudget));
        assert_eq!(spans, [PretokenSpan { start: 91, end: 92 }; 5]);
        assert_eq!(encoded, [0xa5; 9]);
        assert_eq!(out, [u32::MAX; 13]);
        let result = p.encode_raw_into(
            "a",
            ExecutionWorkspace {
                spans: &mut spans,
                encoded: &mut encoded,
                symbols: &mut symbols,
            },
            &mut out,
            ExecutionBudget {
                capacity_bytes: charged,
                ..execution()
            },
        );
        assert_eq!(result, Ok(1));
        assert_eq!(out[0], 258);
        assert_eq!(&out[1..], &[u32::MAX; 12]);
        let mut decoded = [0xa5; 64];
        assert_eq!(
            p.decode_token_bytes_into(
                97,
                &mut decoded,
                ExecutionBudget {
                    capacity_bytes: 63,
                    ..execution()
                }
            ),
            Err(TokenizerError::CapacityBudget)
        );
        assert_eq!(decoded, [0xa5; 64]);
    });
}

#[test]
fn exact_work_threshold_and_capacity_refusals_never_report_truncated_success() {
    plan_with(FIXTURE, |p| {
        let text = "a<x> hello abcabcd";
        let (mut low, mut high) = (0, 10_000_000);
        while high - low > 1 {
            let mid = low + (high - low) / 2;
            match raw(
                p,
                text,
                &mut [u32::MAX; 64],
                ExecutionBudget {
                    max_work: mid,
                    ..execution()
                },
            ) {
                Ok(_) => high = mid,
                Err(TokenizerError::WorkBudget) => low = mid,
                other => panic!("unexpected budget result {other:?}"),
            }
        }
        let mut out = [u32::MAX; 64];
        let n = raw(
            p,
            text,
            &mut out,
            ExecutionBudget {
                max_work: high,
                ..execution()
            },
        )
        .unwrap();
        assert!(n > 2);
        assert!(out[n..].iter().all(|&id| id == u32::MAX));
        assert_eq!(
            raw(
                p,
                text,
                &mut out,
                ExecutionBudget {
                    max_work: low,
                    ..execution()
                }
            ),
            Err(TokenizerError::WorkBudget)
        );
        assert_eq!(
            raw(p, text, &mut [0; 1], execution()),
            Err(TokenizerError::Capacity)
        );
        assert_eq!(
            raw(p, text, &mut [], execution()),
            Err(TokenizerError::Capacity)
        );
    });
}

#[test]
fn preparation_and_hot_success_refusal_paths_allocate_zero_with_positive_control() {
    assert!(
        measured(|| {
            drop(std::hint::black_box(Box::new([7u8; 32])));
        }) > 0
    );
    let view = parse(FIXTURE);
    let r = requirements(&view, preparation()).unwrap();
    let mut v = vec![VocabEntry::default(); r.vocab + 1];
    let mut m = vec![MergeEntry::default(); r.merges + 1];
    let mut s = vec![SpecialEntry::default(); r.specials + 1];
    assert_eq!(
        measured(|| {
            let p = prepare(
                &view,
                TokenizerTables {
                    vocab: &mut v,
                    merges: &mut m,
                    specials: &mut s,
                },
                preparation(),
            )
            .unwrap();
            let mut out = [u32::MAX; 128];
            assert!(raw(&p, "a<x> hello abcabcd", &mut out, execution()).unwrap() > 0);
            assert_eq!(
                raw(&p, "abcabcd", &mut [], execution()),
                Err(TokenizerError::Capacity)
            );
            assert_eq!(
                raw(
                    &p,
                    "abcabcd",
                    &mut out,
                    ExecutionBudget {
                        max_work: 0,
                        ..execution()
                    }
                ),
                Err(TokenizerError::WorkBudget)
            );
            let mut piece = [0xa5; 32];
            assert_eq!(
                p.decode_token_bytes_into(262, &mut piece, execution()),
                Ok(1)
            );
            assert_eq!(
                p.decode_token_bytes_into(u32::MAX, &mut piece, execution()),
                Err(TokenizerError::InvalidTokenId(u32::MAX))
            );
        }),
        0
    );
    let exact = v.len() * size_of::<VocabEntry<'_>>()
        + m.len() * size_of::<MergeEntry<'_>>()
        + s.len() * size_of::<SpecialEntry<'_>>();
    assert!(matches!(
        prepare(
            &view,
            TokenizerTables {
                vocab: &mut v,
                merges: &mut m,
                specials: &mut s
            },
            PrepareBudget {
                capacity_bytes: exact - 1,
                ..preparation()
            }
        ),
        Err(TokenizerError::CapacityBudget)
    ));
    assert_eq!(
        requirements(
            &view,
            PrepareBudget {
                max_work: 0,
                ..preparation()
            }
        ),
        Err(TokenizerError::WorkBudget)
    );
}

#[test]
fn versioned_scalar_vertical_tab_is_consistent_across_vector_boundaries() {
    assert_eq!(TOKENIZER_SEMANTICS_VERSION, 1);
    plan_with(FIXTURE, |p| {
        for prefix in [b' ', b'!'] {
            for count in [0, 1, 30, 31, 32, 33, 63, 64] {
                let mut text = String::from_utf8(vec![prefix; count]).unwrap();
                text.push_str("\u{b} ");
                let mut out = [u32::MAX; 128];
                let n = raw(p, &text, &mut out, execution()).unwrap();
                assert_eq!(n, count + 1);
                assert!(out[..count].iter().all(|&id| id == u32::from(prefix)));
                assert_eq!(out[count], 274);
                assert!(out[n..].iter().all(|&id| id == u32::MAX));
                let mut piece = [0xa5; 16];
                assert_eq!(
                    p.decode_token_bytes_into(274, &mut piece, execution()),
                    Ok(2)
                );
                assert_eq!(&piece[..2], b"\x0b ");
            }
        }
    });
    // Actual prior x86_64 runtime-selected tokenizer returned [11,32] for VT+space
    // on this same fixture. The additive version deliberately preserves the old
    // scalar result [274] consistently; this is a reviewed correction, not parity.
}
