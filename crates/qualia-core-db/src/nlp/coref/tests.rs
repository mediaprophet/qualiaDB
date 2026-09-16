//! NLP-001 acceptance tests. Linguistic detector redesign belongs to NLP-500/501.

use super::*;
use crate::nlp::span::DocSpan;

fn mention(text: &str, start: u32, kind: MentionKind) -> CorefMention {
    CorefMention {
        span: DocSpan::new(start, start + text.len() as u32),
        text: text.into(),
        kind,
    }
}

fn at(source: &str, text: &str, kind: MentionKind, which: usize) -> CorefMention {
    let mut from = 0usize;
    let mut seen = 0usize;
    loop {
        let idx = source[from..]
            .find(text)
            .unwrap_or_else(|| panic!("missing {text:?} in {source:?}"));
        let start = from + idx;
        if seen == which {
            return mention(text, start as u32, kind);
        }
        seen += 1;
        from = start + text.len();
    }
}

fn resolve(source: &str, mentions: Vec<CorefMention>) -> Vec<CorefChain> {
    resolve_coreferences(source, &mentions).expect("expected valid coref input")
}

#[test]
fn exact_string_match_merges() {
    let source = "John ran. John fell.";
    let mentions = vec![
        at(source, "John", MentionKind::Proper, 0),
        at(source, "John", MentionKind::Proper, 1),
    ];
    let chains = resolve(source, mentions);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].mentions.len(), 2);
}

#[test]
fn exact_match_is_transitive_across_pronoun_kind() {
    // Legacy pairwise grouping merged equal-key non-pronouns even when a
    // pronoun-kind mention with the same folded key sat between them.
    let source = "It it It";
    let mentions = vec![
        at(source, "It", MentionKind::Proper, 0),
        at(source, "it", MentionKind::Pronoun, 0),
        at(source, "It", MentionKind::Proper, 1),
    ];
    let chains = resolve(source, mentions);
    let proper_chain = chains
        .iter()
        .find(|c| {
            c.mentions
                .iter()
                .filter(|m| m.kind == MentionKind::Proper)
                .count()
                == 2
        })
        .expect("two proper It mentions must share a chain");
    assert_eq!(proper_chain.mentions.len(), 2);
    assert_eq!(chains.len(), 2);
}

#[test]
fn pronoun_resolves_to_preceding_proper() {
    let source = "John ran. He fell.";
    let mentions = vec![
        at(source, "John", MentionKind::Proper, 0),
        at(source, "He", MentionKind::Pronoun, 0),
    ];
    let chains = resolve(source, mentions);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].mentions.len(), 2);
}

#[test]
fn pronoun_gender_mismatch_keeps_separate() {
    let source = "Mary ran. He fell.";
    let mentions = vec![
        at(source, "Mary", MentionKind::Proper, 0),
        at(source, "He", MentionKind::Pronoun, 0),
    ];
    let chains = resolve(source, mentions);
    assert_eq!(chains.len(), 2);
}

#[test]
fn she_resolves_to_female_name() {
    let source = "Mary left. She smiled.";
    let mentions = vec![
        at(source, "Mary", MentionKind::Proper, 0),
        at(source, "She", MentionKind::Pronoun, 0),
    ];
    let chains = resolve(source, mentions);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].mentions.len(), 2);
}

#[test]
fn empty_mentions_returns_empty() {
    let chains = resolve("", Vec::new());
    assert!(chains.is_empty());
}

#[test]
fn empty_mentions_on_nonempty_source_is_no_detection() {
    let chains = resolve("John ran.", Vec::new());
    assert!(chains.is_empty());
}

#[test]
fn it_resolves_neutral() {
    let source = "John ran. It rained.";
    let mentions = vec![
        at(source, "John", MentionKind::Proper, 0),
        at(source, "It", MentionKind::Pronoun, 0),
    ];
    let chains = resolve(source, mentions);
    assert_eq!(chains.len(), 2);
}

#[test]
fn oracle_repeated_and_distinct_names() {
    let source = "John saw John. Mary left.";
    let mentions = vec![
        at(source, "John", MentionKind::Proper, 0),
        at(source, "John", MentionKind::Proper, 1),
        at(source, "Mary", MentionKind::Proper, 0),
    ];
    let chains = resolve(source, mentions);
    assert_eq!(chains.len(), 2);
    assert_eq!(chains[0].mentions.len(), 2);
    assert_eq!(chains[0].mentions[0].text, "John");
    assert_eq!(chains[1].mentions[0].text, "Mary");
}

#[test]
fn rejects_reversed_and_out_of_range_spans() {
    let source = "John";
    let reversed = CorefMention {
        span: DocSpan::new(4, 0),
        text: "John".into(),
        kind: MentionKind::Proper,
    };
    assert_eq!(
        resolve_coreferences(source, &[reversed]).unwrap_err(),
        CorefError::SpanEmpty
    );
    let oob = mention("John", 1, MentionKind::Proper);
    assert_eq!(
        resolve_coreferences(source, &[oob]).unwrap_err(),
        CorefError::SpanOutOfRange
    );
}

#[test]
fn rejects_text_mismatch_and_utf8_interior() {
    let source = "John";
    let mismatch = CorefMention {
        span: DocSpan::new(0, 4),
        text: "Jane".into(),
        kind: MentionKind::Proper,
    };
    assert_eq!(
        resolve_coreferences(source, &[mismatch]).unwrap_err(),
        CorefError::TextMismatch
    );
    let cafe = "café";
    assert_eq!(cafe.len(), 5);
    let interior = CorefMention {
        span: DocSpan::new(3, 4),
        text: "x".into(),
        kind: MentionKind::Common,
    };
    assert_eq!(
        resolve_coreferences(cafe, &[interior]).unwrap_err(),
        CorefError::SpanNotUtf8Boundary
    );
}

#[test]
fn rejects_unsorted_mentions() {
    let source = "John Mary";
    let mentions = vec![
        at(source, "Mary", MentionKind::Proper, 0),
        at(source, "John", MentionKind::Proper, 0),
    ];
    assert_eq!(
        resolve_coreferences(source, &mentions).unwrap_err(),
        CorefError::UnsortedMentions
    );
}

#[test]
fn overlapping_same_span_is_not_unsorted() {
    let source = "John";
    let mentions = vec![
        mention("John", 0, MentionKind::Proper),
        mention("John", 0, MentionKind::Proper),
    ];
    let chains = resolve(source, mentions);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].mentions.len(), 2);
}

fn counted(
    source: &str,
    mentions: &[CorefMention],
    limits: &CorefLimits,
) -> (Vec<CorefChain>, CorefWorkStats) {
    resolve_coreferences_counted(source, mentions, limits, &CancellationToken::new()).unwrap()
}

#[test]
fn source_limit_boundary() {
    let mut limits = CorefLimits::DEFAULT;
    limits.max_source_bytes = 4;
    let ok = mention("John", 0, MentionKind::Proper);
    counted("John", &[ok.clone()], &limits);
    assert!(matches!(
        resolve_coreferences_counted("John!", &[ok], &limits, &CancellationToken::new()),
        Err(CorefError::SourceTooLarge { bytes: 5, max: 4 })
    ));
}

#[test]
fn mention_count_limit_boundary() {
    let mut limits = CorefLimits::DEFAULT;
    limits.max_mentions = 2;
    let source = "John";
    let two = vec![
        mention("John", 0, MentionKind::Proper),
        mention("John", 0, MentionKind::Proper),
    ];
    counted(source, &two, &limits);
    let three = vec![
        mention("John", 0, MentionKind::Proper),
        mention("John", 0, MentionKind::Proper),
        mention("John", 0, MentionKind::Proper),
    ];
    assert!(matches!(
        resolve_coreferences_counted(source, &three, &limits, &CancellationToken::new()),
        Err(CorefError::TooManyMentions { count: 3, max: 2 })
    ));
}

#[test]
fn mention_text_byte_limit_boundary() {
    let mut limits = CorefLimits::DEFAULT;
    limits.max_mention_text_bytes = 4;
    let source = "John";
    counted(source, &[mention("John", 0, MentionKind::Proper)], &limits);
    let two = vec![
        mention("John", 0, MentionKind::Proper),
        mention("John", 0, MentionKind::Proper),
    ];
    assert!(matches!(
        resolve_coreferences_counted(source, &two, &limits, &CancellationToken::new()),
        Err(CorefError::MentionTextTooLarge { .. })
    ));
}

#[test]
fn chain_limit_boundary() {
    let mut limits = CorefLimits::DEFAULT;
    limits.max_chains = 1;
    let source = "John Mary";
    let one = vec![at(source, "John", MentionKind::Proper, 0)];
    counted(source, &one, &limits);
    let two = vec![
        at(source, "John", MentionKind::Proper, 0),
        at(source, "Mary", MentionKind::Proper, 0),
    ];
    assert!(matches!(
        resolve_coreferences_counted(source, &two, &limits, &CancellationToken::new()),
        Err(CorefError::TooManyChains { count: 2, max: 1 })
    ));
}

#[test]
fn default_mention_cap_is_4096() {
    let source = "x";
    let mut mentions = Vec::with_capacity(4096);
    for _ in 0..4096 {
        mentions.push(mention("x", 0, MentionKind::Common));
    }
    let chains = resolve_coreferences(source, &mentions).unwrap();
    assert_eq!(chains.len(), 1);
    mentions.push(mention("x", 0, MentionKind::Common));
    assert!(matches!(
        resolve_coreferences(source, &mentions),
        Err(CorefError::TooManyMentions {
            count: 4097,
            max: 4096
        })
    ));
}

#[test]
fn output_buffer_exhaustion_is_not_consumable() {
    let source = "John";
    let mentions = [ValidatedMention {
        start: 0,
        end: 4,
        kind: MentionKind::Proper,
    }];
    let mut work = [MentionWork::default(); 1];
    let mut order = [0u32; 1];
    let mut gather = [0u32; 1];
    let mut norm = [0u8; 8];
    let mut scratch = CorefScratch {
        work: &mut work,
        order: &mut order,
        gather: &mut gather,
        norm: &mut norm,
    };
    let mut chains: [CorefChainHead; 0] = [];
    let mut ids = [0u32; 1];
    let mut stats = CorefWorkStats::default();
    let err = resolve_coreferences_into(
        source,
        &mentions,
        &CorefLimits::DEFAULT,
        &mut scratch,
        &mut chains,
        &mut ids,
        &CancellationToken::new(),
        &mut stats,
    )
    .unwrap_err();
    assert_eq!(err, CorefError::OutputBufferFull);
}

#[test]
fn cancellation_before_and_during_work() {
    let source = "John";
    let mentions = [mention("John", 0, MentionKind::Proper)];
    let done = CancellationToken::new();
    done.cancel();
    assert_eq!(
        resolve_coreferences_counted(source, &mentions, &CorefLimits::DEFAULT, &done).unwrap_err(),
        CorefError::Cancelled
    );
    let mid = CancellationToken::cancel_after_polls(1);
    assert_eq!(
        resolve_coreferences_counted(source, &mentions, &CorefLimits::DEFAULT, &mid).unwrap_err(),
        CorefError::Cancelled
    );
}

#[test]
fn antecedent_window_unresolved_vs_budget_error() {
    let source = "John Mary Mary He";
    let mentions = vec![
        at(source, "John", MentionKind::Proper, 0),
        at(source, "Mary", MentionKind::Proper, 0),
        at(source, "Mary", MentionKind::Proper, 1),
        at(source, "He", MentionKind::Pronoun, 0),
    ];
    let mut window = CorefLimits::DEFAULT;
    window.max_antecedents_per_pronoun = 1;
    let (chains, stats) = counted(source, &mentions, &window);
    assert_eq!(stats.antecedent_candidates, 1);
    assert_eq!(
        chains.len(),
        3,
        "window miss stays unresolved, not an error"
    );

    let budget_src = "John Mary He";
    let budget_mentions = vec![
        at(budget_src, "John", MentionKind::Proper, 0),
        at(budget_src, "Mary", MentionKind::Proper, 0),
        at(budget_src, "He", MentionKind::Pronoun, 0),
    ];
    let mut budget = CorefLimits::DEFAULT;
    budget.max_antecedent_checks = 1;
    assert!(matches!(
        resolve_coreferences_counted(
            budget_src,
            &budget_mentions,
            &budget,
            &CancellationToken::new()
        ),
        Err(CorefError::AntecedentBudget { .. })
    ));
}

#[test]
fn forced_hash_collision_does_not_merge_distinct_keys() {
    let source = "John Mary";
    let mentions = vec![
        at(source, "John", MentionKind::Proper, 0),
        at(source, "Mary", MentionKind::Proper, 0),
    ];
    let mut limits = CorefLimits::DEFAULT;
    limits.key_hash = |_| 1;
    let (chains, stats) = counted(source, &mentions, &limits);
    assert_eq!(chains.len(), 2);
    assert!(stats.comparison_bytes > 0);
}

#[test]
fn long_shared_prefix_counts_comparison_bytes() {
    let left = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaX";
    let right = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaY";
    let source = format!("{left} {right}");
    let mentions = vec![
        at(&source, left, MentionKind::Proper, 0),
        at(&source, right, MentionKind::Proper, 0),
    ];
    let mut limits = CorefLimits::DEFAULT;
    limits.key_hash = |_| 7;
    let (_, stats) = counted(&source, &mentions, &limits);
    assert!(stats.comparison_bytes >= 30);
}

#[test]
fn exact_match_work_is_not_quadratic() {
    fn load(n: usize) -> (u64, u64) {
        let source = "John";
        let mentions: Vec<CorefMention> = (0..n)
            .map(|_| mention("John", 0, MentionKind::Proper))
            .collect();
        let (_, stats) = counted(source, &mentions, &CorefLimits::DEFAULT);
        (stats.key_comparisons, stats.comparison_bytes)
    }
    let (c32, _) = load(32);
    let (c64, _) = load(64);
    let (c128, _) = load(128);
    let pairwise_128 = 128 * 127 / 2;
    assert!(
        c128 < pairwise_128,
        "128-way exact match must not inspect every pair ({c128} vs {pairwise_128})"
    );
    assert!(
        c128 < c32.saturating_mul(12),
        "comparisons at 4n must stay far below quadratic growth: 32={c32} 64={c64} 128={c128}"
    );
    assert!(c64 >= c32);
    assert!(c128 >= c64);
}

#[test]
fn normalization_budget_boundary() {
    let source = "John";
    let mentions = vec![mention("John", 0, MentionKind::Proper)];
    let mut under = CorefLimits::DEFAULT;
    under.max_normalization_bytes = 4;
    counted(source, &mentions, &under);

    let mut over = CorefLimits::DEFAULT;
    over.max_normalization_bytes = 3;
    assert!(matches!(
        resolve_coreferences_counted(source, &mentions, &over, &CancellationToken::new()),
        Err(CorefError::NormalizationBudget { used: 4, max: 3 })
    ));
}

#[test]
fn comparison_budget_boundary() {
    let left = "aaaaaaaaaa";
    let right = "aaaaaaaaab";
    let source = format!("{left} {right}");
    let mentions = vec![
        at(&source, left, MentionKind::Proper, 0),
        at(&source, right, MentionKind::Proper, 0),
    ];
    let mut under = CorefLimits::DEFAULT;
    under.key_hash = |_| 1;
    under.max_comparison_bytes = 10_000;
    counted(&source, &mentions, &under);

    let mut over = CorefLimits::DEFAULT;
    over.key_hash = |_| 1;
    over.max_comparison_bytes = 1;
    assert!(matches!(
        resolve_coreferences_counted(&source, &mentions, &over, &CancellationToken::new()),
        Err(CorefError::ComparisonBudget { .. })
    ));
}

#[test]
fn default_limits_succeed_at_two_thousand_mentions() {
    let mut source = String::new();
    let mut mentions = Vec::with_capacity(2000);
    for i in 0..2000 {
        let start = source.len() as u32;
        if i % 20 == 19 {
            source.push_str("He ");
            mentions.push(mention("He", start, MentionKind::Pronoun));
        } else {
            source.push_str("John ");
            mentions.push(mention("John", start, MentionKind::Proper));
        }
    }
    let (chains, stats) = counted(&source, &mentions, &CorefLimits::DEFAULT);
    assert!(!chains.is_empty());
    assert!(
        stats.antecedent_checks <= DEFAULT_MAX_ANTECEDENT_CHECKS,
        "Proper-only charging must succeed at defaults: checks={}",
        stats.antecedent_checks
    );
}

#[test]
fn offset_conversions_reject_negative_and_overflow() {
    assert_eq!(offset_from_i64(-1).unwrap_err(), CorefError::NegativeOffset);
    assert_eq!(
        offset_from_u64(u64::from(u32::MAX) + 1).unwrap_err(),
        CorefError::OffsetOverflow
    );
    assert_eq!(offset_from_i64(4).unwrap(), 4);
}

#[test]
fn unknown_kind_label_is_not_coerced() {
    assert_eq!(
        mention_kind_from_label("entity").unwrap_err(),
        CorefError::InvalidKind
    );
    assert_eq!(
        mention_kind_from_label("common").unwrap(),
        MentionKind::Common
    );
}

#[test]
fn default_workspace_fits_sentinel() {
    let required = workspace_bytes_required(DEFAULT_MAX_MENTIONS, DEFAULT_MAX_MENTION_TEXT_BYTES);
    assert!(
        required <= DEFAULT_MAX_WORKSPACE_BYTES,
        "hot workspace {required} must fit the 2 MiB default"
    );
    assert!(DEFAULT_MAX_SOURCE_BYTES + DEFAULT_MAX_WORKSPACE_BYTES <= SENTINEL_BYTES);
    let n = DEFAULT_MAX_MENTIONS;
    let cold_extra = n
        .saturating_mul(core::mem::size_of::<ValidatedMention>())
        .saturating_add(n.saturating_mul(core::mem::size_of::<CorefChainHead>()))
        .saturating_add(n.saturating_mul(core::mem::size_of::<u32>()));
    assert!(
        required + cold_extra < SENTINEL_BYTES,
        "cold adapter buffers plus hot workspace must stay under the Sentinel"
    );
}

#[test]
fn hot_path_is_zero_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    let source = "John ran. John fell.";
    let mentions = [
        ValidatedMention {
            start: 0,
            end: 4,
            kind: MentionKind::Proper,
        },
        ValidatedMention {
            start: 10,
            end: 14,
            kind: MentionKind::Proper,
        },
    ];
    let mut work = [MentionWork::default(); 2];
    let mut order = [0u32; 2];
    let mut gather = [0u32; 2];
    let mut norm = [0u8; 16];
    let mut scratch = CorefScratch {
        work: &mut work,
        order: &mut order,
        gather: &mut gather,
        norm: &mut norm,
    };
    let mut chains = [CorefChainHead::default(); 2];
    let mut ids = [0u32; 2];
    let mut stats = CorefWorkStats::default();
    let cancel = CancellationToken::new();
    let limits = CorefLimits::DEFAULT;
    let _ = resolve_coreferences_into(
        source,
        &mentions,
        &limits,
        &mut scratch,
        &mut chains,
        &mut ids,
        &cancel,
        &mut stats,
    );
    let mut work = [MentionWork::default(); 2];
    let mut order = [0u32; 2];
    let mut gather = [0u32; 2];
    let mut norm = [0u8; 16];
    let mut scratch = CorefScratch {
        work: &mut work,
        order: &mut order,
        gather: &mut gather,
        norm: &mut norm,
    };
    let mut chains = [CorefChainHead::default(); 2];
    let mut ids = [0u32; 2];
    let mut stats = CorefWorkStats::default();
    assert_zero_alloc("coref_resolve_into", || {
        let n = resolve_coreferences_into(
            source,
            &mentions,
            &limits,
            &mut scratch,
            &mut chains,
            &mut ids,
            &cancel,
            &mut stats,
        )
        .expect("hot resolve");
        assert_eq!(n, 1);
    });
}

#[test]
fn hot_path_pronoun_sieve_is_zero_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    let source = "John ran. He fell.";
    let mentions = [
        ValidatedMention {
            start: 0,
            end: 4,
            kind: MentionKind::Proper,
        },
        ValidatedMention {
            start: 10,
            end: 12,
            kind: MentionKind::Pronoun,
        },
    ];
    let mut work = [MentionWork::default(); 2];
    let mut order = [0u32; 2];
    let mut gather = [0u32; 2];
    let mut norm = [0u8; 16];
    let mut scratch = CorefScratch {
        work: &mut work,
        order: &mut order,
        gather: &mut gather,
        norm: &mut norm,
    };
    let mut chains = [CorefChainHead::default(); 2];
    let mut ids = [0u32; 2];
    let mut stats = CorefWorkStats::default();
    let cancel = CancellationToken::new();
    let limits = CorefLimits::DEFAULT;
    assert_zero_alloc("coref_resolve_into_pronoun", || {
        let n = resolve_coreferences_into(
            source,
            &mentions,
            &limits,
            &mut scratch,
            &mut chains,
            &mut ids,
            &cancel,
            &mut stats,
        )
        .expect("hot pronoun resolve");
        assert_eq!(n, 1);
    });
}

#[test]
fn cold_adapter_cleans_up_on_error_and_unwind() {
    let source = "John";
    let bad = CorefMention {
        span: DocSpan::new(0, 4),
        text: "nope".into(),
        kind: MentionKind::Proper,
    };
    assert!(resolve_coreferences(source, &[bad]).is_err());
    let mentions = vec![mention("John", 0, MentionKind::Proper)];
    let panicked = std::panic::catch_unwind(|| {
        let _ = resolve_coreferences(source, &mentions);
        panic!("intentional unwind after success");
    });
    assert!(panicked.is_err());
}

#[test]
fn invalid_kind_enum_is_only_via_label() {
    // Rust callers cannot construct an unknown MentionKind; host labels are the
    // only coercion risk, covered by mention_kind_from_label.
    assert!(mention_kind_from_label("pronoun").is_ok());
}
