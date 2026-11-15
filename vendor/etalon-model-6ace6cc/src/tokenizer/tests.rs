use super::*;
use crate::gguf::{MetadataView, ParseLimits, VersionPolicy};
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

fn string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}
fn kv(out: &mut Vec<u8>, key: &str, kind: u32, payload: &[u8]) {
    string(out, key);
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(payload);
}
fn strings(values: &[String]) -> Vec<u8> {
    let mut out = 8u32.to_le_bytes().to_vec();
    out.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for s in values {
        string(&mut out, s);
    }
    out
}
fn fixture(extra: &[&str], merges: &[&str], pre: &str, add_bos: bool) -> Vec<u8> {
    let mut vocab: Vec<String> = alphabet::ALPHABET.iter().map(|c| c.to_string()).collect();
    vocab.extend(extra.iter().map(|s| s.to_string()));
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend_from_slice(&7u64.to_le_bytes());
    let mut model = Vec::new();
    string(&mut model, "gpt2");
    kv(&mut out, "tokenizer.ggml.model", 8, &model);
    let mut profile = Vec::new();
    string(&mut profile, pre);
    kv(&mut out, "tokenizer.ggml.pre", 8, &profile);
    kv(&mut out, "tokenizer.ggml.tokens", 9, &strings(&vocab));
    kv(
        &mut out,
        "tokenizer.ggml.merges",
        9,
        &strings(&merges.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
    );
    kv(
        &mut out,
        "tokenizer.ggml.bos_token_id",
        4,
        &1u32.to_le_bytes(),
    );
    kv(
        &mut out,
        "tokenizer.ggml.eos_token_id",
        4,
        &2u32.to_le_bytes(),
    );
    kv(
        &mut out,
        "tokenizer.ggml.add_bos_token",
        7,
        &[u8::from(add_bos)],
    );
    out
}
fn parse(bytes: &[u8]) -> MetadataView<'_> {
    MetadataView::parse(
        bytes,
        ParseLimits {
            max_metadata_bytes: bytes.len(),
            max_nesting_depth: 4,
            max_work: 1_000_000,
            versions: VersionPolicy::V2V3,
        },
    )
    .unwrap()
}
fn budget() -> PrepareBudget {
    PrepareBudget {
        capacity_bytes: 1_000_000,
        max_work: 10_000_000,
    }
}
fn exec() -> ExecutionBudget {
    ExecutionBudget {
        capacity_bytes: 1_000_000,
        max_work: 10_000_000,
    }
}
fn with_plan(bytes: &[u8], f: impl FnOnce(&TokenizerPlan<'_, '_>)) {
    let view = parse(bytes);
    let r = requirements(&view, budget()).unwrap();
    let mut v = vec![VocabEntry::default(); r.vocab];
    let mut m = vec![MergeEntry::default(); r.merges];
    let mut s = vec![SpecialEntry::default(); r.specials];
    let plan = prepare(
        &view,
        TokenizerTables {
            vocab: &mut v,
            merges: &mut m,
            specials: &mut s,
        },
        budget(),
    )
    .unwrap();
    f(&plan);
}
fn encode(plan: &TokenizerPlan<'_, '_>, text: &str, prompt: bool) -> Vec<u32> {
    let mut spans = vec![PretokenSpan::default(); text.len() + 1];
    let mut encoded = vec![0; text.len() * 2 + 1];
    let mut symbols = vec![SymbolSpan::default(); text.len() + 1];
    let mut out = vec![u32::MAX; text.len() + 2];
    let workspace = ExecutionWorkspace {
        spans: &mut spans,
        encoded: &mut encoded,
        symbols: &mut symbols,
    };
    let n = if prompt {
        plan.encode_prompt_into(text, workspace, &mut out, exec())
    } else {
        plan.encode_raw_into(text, workspace, &mut out, exec())
    }
    .unwrap();
    assert!(out[n..].iter().all(|&v| v == u32::MAX));
    out.truncate(n);
    out
}

#[test]
fn duplicate_vocab_last_special_first_and_raw_shortcut() {
    with_plan(
        &fixture(
            &["hello", "hello", "<x>", "<x>", "<xy>", ""],
            &[],
            "smollm",
            false,
        ),
        |p| {
            assert_eq!(encode(p, "hello", false), [257]);
            assert_eq!(encode(p, "<x><xy>", false), [258, 260]);
            assert_eq!(encode(p, "é", false), [233]); // Legacy raw shortcut precedes UTF-8 byte encoding.
            let mut out = [0xa5; 8];
            assert_eq!(p.decode_token_bytes_into(261, &mut out, exec()), Ok(0));
            assert_eq!(out, [0xa5; 8]);
            assert_eq!(p.profile(), TokenizerProfile::Smollm);
            assert_eq!(p.model(), "gpt2");
        },
    );
}
#[test]
fn merge_rank_first_and_missing_merged_symbol_byte_fallback() {
    with_plan(
        &fixture(&["ab", "bc"], &["a b", "b c", "a b"], "gpt2", false),
        |p| assert_eq!(encode(p, "abc", false), [256, 99]),
    );
    with_plan(&fixture(&[], &["a b", "ab c"], "gpt2", false), |p| {
        assert_eq!(encode(p, "abc", false), [97, 98, 99])
    });
    with_plan(&fixture(&["ab"], &["a b"], "gpt2", false), |p| {
        assert_eq!(encode(p, "abab", false), [256, 256])
    });
}
#[test]
fn encoded_word_lookup_bos_and_exact_raw_decode() {
    with_plan(
        &fixture(&["Ġhello", "<0xFF>", "<0xZZ>"], &[], "gpt2", true),
        |p| {
            assert_eq!(encode(p, " hello", false), [256]);
            assert_eq!(encode(p, " hello", true), [1, 256]);
            assert_eq!(encode(p, "", true), [1]);
            assert_eq!(encode(p, "", false), []);
            let mut out = [0xa5; 8];
            assert_eq!(p.decode_token_bytes_into(257, &mut out, exec()), Ok(1));
            assert_eq!(out[0], 255);
            assert_eq!(out[1..], [0xa5; 7]);
            assert_eq!(
                p.decode_token_bytes_into(257, &mut [0; 1], exec()),
                Err(TokenizerError::Capacity)
            );
            assert_eq!(
                p.decode_token_bytes_into(258, &mut out, exec()),
                Err(TokenizerError::InvalidByteToken)
            );
            assert_eq!(
                p.decode_token_bytes_into(u32::MAX, &mut out, exec()),
                Err(TokenizerError::InvalidTokenId(u32::MAX))
            );
            assert_eq!(p.decode_token_bytes_into(195, &mut out, exec()), Ok(1));
            assert_eq!(out[0], 195);
        },
    );
}
#[test]
fn scalar_scanner_preserves_contractions_classes_and_whitespace() {
    let text = "I'm we're  A\n１２é!";
    let mut spans = [PretokenSpan::default(); 32];
    let mut work = budget::Work(10000);
    let n = pretoken::scan(text, &mut spans, &mut work).unwrap();
    let pieces: Vec<_> = spans[..n].iter().map(|s| &text[s.start..s.end]).collect();
    assert_eq!(
        pieces,
        ["I", "'m", " we", "'re", "  ", "A", "\n", "１２", "é", "!"]
    );
}
#[test]
fn strict_profile_merge_and_missing_alphabet_refusals() {
    let b = fixture(&[], &[], "qwen35", false);
    assert_eq!(
        requirements(&parse(&b), budget()),
        Err(TokenizerError::UnsupportedProfile)
    );
    let b = fixture(&[], &["bad"], "gpt2", false);
    assert_eq!(
        requirements(&parse(&b), budget()),
        Err(TokenizerError::MalformedMerge)
    );
    let mut b = fixture(&["\0!"], &[], "gpt2", false);
    let pos = b
        .windows(10)
        .position(|v| v == [2, 0, 0, 0, 0, 0, 0, 0, 0xc4, 0x80])
        .unwrap();
    b[pos + 9] = 0x81; // Replace byte0's alphabet symbol with duplicate byte1.
    let view = parse(&b);
    let r = requirements(&view, budget()).unwrap();
    let mut v = vec![VocabEntry::default(); r.vocab];
    let p = prepare(
        &view,
        TokenizerTables {
            vocab: &mut v,
            merges: &mut [],
            specials: &mut [],
        },
        budget(),
    )
    .unwrap();
    assert_eq!(encode(&p, "\0!", false), [256]);
    let mut spans = [PretokenSpan::default(); 2];
    let mut encoded = [0; 4];
    let mut symbols = [SymbolSpan::default(); 2];
    let mut out = [1234; 2];
    assert_eq!(
        p.encode_raw_into(
            "\0",
            ExecutionWorkspace {
                spans: &mut spans,
                encoded: &mut encoded,
                symbols: &mut symbols
            },
            &mut out,
            exec()
        ),
        Err(TokenizerError::MissingByteSymbol(0))
    );
}
#[test]
fn all_supplied_capacities_charged_and_workspace_reusable() {
    let b = fixture(&[], &[], "gpt2", false);
    let view = parse(&b);
    let mut v = vec![VocabEntry::default(); 257];
    let small = PrepareBudget {
        capacity_bytes: 256 * core::mem::size_of::<VocabEntry<'_>>(),
        ..budget()
    };
    assert!(matches!(
        prepare(
            &view,
            TokenizerTables {
                vocab: &mut v,
                merges: &mut [],
                specials: &mut []
            },
            small
        ),
        Err(TokenizerError::CapacityBudget)
    ));
    assert!(v.iter().all(|e| e.text.is_empty()));
    let p = prepare(
        &view,
        TokenizerTables {
            vocab: &mut v,
            merges: &mut [],
            specials: &mut [],
        },
        budget(),
    )
    .unwrap();
    let mut spans = [PretokenSpan::default(); 8];
    let mut encoded = [0; 16];
    let mut symbols = [SymbolSpan::default(); 8];
    let mut out = [1234; 8];
    for _ in 0..2 {
        let n = p
            .encode_raw_into(
                "abc",
                ExecutionWorkspace {
                    spans: &mut spans,
                    encoded: &mut encoded,
                    symbols: &mut symbols,
                },
                &mut out,
                exec(),
            )
            .unwrap();
        assert_eq!(&out[..n], &[97, 98, 99]);
    }
    out.fill(1234);
    assert_eq!(
        p.encode_raw_into(
            "abc",
            ExecutionWorkspace {
                spans: &mut spans,
                encoded: &mut encoded,
                symbols: &mut symbols
            },
            &mut out,
            ExecutionBudget {
                capacity_bytes: 0,
                ..exec()
            }
        ),
        Err(TokenizerError::CapacityBudget)
    );
    assert_eq!(out, [1234; 8]);
    assert_eq!(
        p.decode_token_bytes_into(
            97,
            &mut encoded,
            ExecutionBudget {
                max_work: 0,
                ..exec()
            }
        ),
        Err(TokenizerError::WorkBudget)
    );
    assert_eq!(v.len(), 257); // Immutable plan no longer borrows mutable table storage.
}
