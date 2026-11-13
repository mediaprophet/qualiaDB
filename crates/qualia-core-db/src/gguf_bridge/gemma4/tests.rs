use super::*;
#[path = "fixtures.rs"]
mod fixtures;

#[test]
fn gemma_per_layer_kv_and_shared_producers_do_not_guess_by_dimension() {
    let (p, index, source) = fixtures::fixture();
    let plan = plan::Plan::new(index, p, &source, 4).unwrap();
    assert_eq!(
        plan.layers.iter().map(|l| l.kv_heads).collect::<Vec<_>>(),
        [8, 2, 8, 2, 8, 2]
    );
    assert_eq!(
        plan.layers.iter().map(|l| l.source).collect::<Vec<_>>(),
        [0, 1, 2, 3, 2, 3]
    );
    assert_eq!(
        plan.layers.iter().map(|l| l.slots).collect::<Vec<_>>(),
        [2, 4, 2, 4, 2, 4]
    );
    assert_eq!(plan.kv_floats, 192);
    assert_eq!(plan.layers[4].offset, plan.layers[2].offset);
    assert_eq!(plan.layers[5].offset, plan.layers[3].offset);
}
#[test]
fn gemma_unscaled_attention_matches_independent_softmax_and_sliding_boundary() {
    let q = [1., 2.];
    let cache = [2., 1., 1., 3., -1., 2., 4., 6.];
    let mut out = [0.; 2];
    attention::causal_gqa(&q, &cache, 2, 1, 1, 2, 0, 1, &mut out).unwrap();
    let a = 4f64.exp();
    let b = 3f64.exp();
    for (i, expected) in [(a + 4. * b) / (a + b), (3. * a + 6. * b) / (a + b)]
        .into_iter()
        .enumerate()
    {
        assert!((out[i] as f64 - expected).abs() < 1e-6);
    }
    attention::causal_gqa(&q, &cache, 2, 1, 1, 2, 1, 1, &mut out).unwrap();
    assert_eq!(out, [4., 6.]);
    // Token2 replaced slot0. Only tokens1 and2 are in this sliding view.
    let ring = [0., 0., 10., 20., -1., 2., 4., 6.];
    attention::causal_gqa(&q, &ring, 2, 1, 1, 2, 1, 2, &mut out).unwrap();
    let expected = [(4. * b + 10.) / (b + 1.), (6. * b + 20.) / (b + 1.)];
    for i in 0..2 {
        assert!((out[i] as f64 - expected[i]).abs() < 2e-6);
    }
    assert!(attention::causal_gqa(&q, &ring, 2, 1, 1, 2, 0, 2, &mut out).is_none());
}
#[test]
fn gemma_ple_merge_matches_f64_oracle_for_distinct_token_rows() {
    let weights = [1.2, 0.8];
    let projection = [3., 4., -1., 2.];
    for token in [[0.5, -0.2, 0.7, 0.9], [-0.8, 0.6, 0.1, -0.4]] {
        let mut actual = token;
        let mut scratch = projection;
        ple::combine(2, 2, 4, 1e-6, &weights, &mut actual, &mut scratch).unwrap();
        for row in 0..2 {
            let x = [
                projection[row * 2] as f64 / 2.,
                projection[row * 2 + 1] as f64 / 2.,
            ];
            let norm = ((x[0] * x[0] + x[1] * x[1]) / 2. + 1e-6).sqrt();
            for j in 0..2 {
                let expected = (token[row * 2 + j] as f64 * 2f64.sqrt()
                    + x[j] / norm * weights[j] as f64)
                    / 2f64.sqrt();
                assert!((actual[row * 2 + j] as f64 - expected).abs() < 1e-6);
            }
        }
    }
}
#[test]
fn gemma_two_sessions_nonzero_tokens_reset_and_failure_are_isolated() {
    let (mut a, index) = fixtures::engine();
    let (mut b, other) = fixtures::engine();
    let (mut oracle, reference) = fixtures::engine();
    let mut h = [0.; 4];
    let mut scratch = [0.; 16];
    let mut spare = [0.; 16];
    assert_eq!(
        a.dispatch_gemma4_token(&index, 1, 0, &mut h, &mut scratch, &mut spare),
        6
    );
    let first = h;
    assert_eq!(
        b.dispatch_gemma4_token(&other, 2, 0, &mut h, &mut scratch, &mut spare),
        6
    );
    assert_ne!(first, h);
    assert_eq!(
        a.dispatch_gemma4_token(&index, 2, 1, &mut h, &mut scratch, &mut spare),
        6
    );
    let interleaved = h;
    assert_eq!(
        oracle.dispatch_gemma4_token(&reference, 1, 0, &mut h, &mut scratch, &mut spare),
        6
    );
    assert_eq!(
        oracle.dispatch_gemma4_token(&reference, 2, 1, &mut h, &mut scratch, &mut spare),
        6
    );
    assert_eq!(interleaved, h);
    a.reset_kv_cache();
    assert_eq!(
        a.dispatch_gemma4_token(&index, 1, 0, &mut h, &mut scratch, &mut spare),
        6
    );
    assert_eq!(first, h);
    assert_eq!(
        a.dispatch_gemma4_token(&index, 3, 1, &mut h, &mut scratch, &mut spare),
        0
    );
    assert!(!a.gemma4_ready(&index));
    assert!(!a.gemma4_output_norm(&index, &mut h, 4));
}
#[test]
fn gemma_bad_shared_geometry_unknown_roles_and_context_budget_refuse() {
    let (p, index, source) = fixtures::fixture();
    let mut bad = p.clone();
    bad.kv_heads[4] = 2;
    assert!(plan::Plan::new(index.clone(), bad, &source, 4).is_err());
    assert!(plan::Plan::new(index.clone(), p.clone(), &source, 9).is_err());
    let mut bad = index.clone();
    bad.entries.push(bad.entries[0]);
    assert!(plan::Plan::new(bad, p.clone(), &source, 4).is_err());
    assert!(session::Session::new(source, index, p, 4, 0).is_err());
}

#[test]
fn gemma_shared_cache_requires_current_producer_and_context_never_wraps_global() {
    let (mut e, index) = fixtures::engine();
    let session = e.gemma4.take().unwrap();
    let mut cache = vec![0.; session.plan.kv_floats];
    let mut written = vec![None; 6];
    let mut h = [0.5; 4];
    let mut out = [0.; 16];
    assert!(
        attention::execute(
            &e,
            &session.plan,
            4,
            0,
            &mut h,
            &mut cache,
            &mut written,
            &mut out
        )
        .is_none()
    );
    written[2] = Some(1);
    assert!(
        attention::execute(
            &e,
            &session.plan,
            4,
            0,
            &mut h,
            &mut cache,
            &mut written,
            &mut out
        )
        .is_none()
    );
    e.gemma4 = Some(session);
    let mut spare = [0.; 16];
    for position in 0..4 {
        assert_eq!(
            e.dispatch_gemma4_token(&index, 1, position, &mut h, &mut out, &mut spare),
            6
        );
    }
    assert_eq!(
        e.dispatch_gemma4_token(&index, 1, 4, &mut h, &mut out, &mut spare),
        0
    );
    assert!(!e.gemma4_ready(&index));
}

#[test]
fn gemma_admission_rejects_unknown_roles_ple_dialect_and_cached_aliases() {
    let (p, index, source) = fixtures::fixture();
    let mut bad = index.clone();
    let mut row = bad.entries[0];
    row.0 = etalon_model::gguf::fnv1a64(b"unknown.weight");
    bad.entries.push(row);
    assert!(plan::Plan::new(bad, p.clone(), &source, 4).is_err());
    let mut bad = index.clone();
    bad.ple_config = Some(Default::default());
    assert!(plan::Plan::new(bad, p.clone(), &source, 4).is_err());
    let mut bad = index.clone();
    bad.output_norm.as_mut().unwrap().byte_offset += 4;
    assert!(plan::Plan::new(bad, p.clone(), &source, 4).is_err());
    let mut bad = index.clone();
    bad.ple_ngram_embedding.as_mut().unwrap().byte_offset += 4;
    assert!(plan::Plan::new(bad, p.clone(), &source, 4).is_err());
    assert!(plan::Plan::new(index, p, &source[..source.len() - 1], 4).is_err());
}
#[test]
fn gemma_current_source_and_directory_must_match_retained_session() {
    let (mut e, index) = fixtures::engine();
    let mut h = [0.; 4];
    let mut a = [0.; 16];
    let mut b = [0.; 16];
    let mut bad = index.clone();
    bad.entries[0].1.byte_offset += 4;
    assert_eq!(
        e.dispatch_gemma4_token(&bad, 1, 0, &mut h, &mut a, &mut b),
        0
    );
    assert_eq!(
        e.dispatch_gemma4_token(&index, 1, 0, &mut h, &mut a, &mut b),
        6
    );
    let (_, _, foreign) = fixtures::fixture();
    e.prepared_native_bytes = Some(foreign);
    assert!(!e.gemma4_ready(&index));
    assert_eq!(
        e.dispatch_gemma4_token(&index, 1, 1, &mut h, &mut a, &mut b),
        0
    );
    assert!(!e.gemma4_output_norm(&index, &mut h, 4));
}

#[test]
fn gemma_legacy_hidden_only_and_speculative_paths_cannot_consume_live_state() {
    let (mut e, index) = fixtures::engine();
    let mut h = [0.; 4];
    let mut a = [0.; 16];
    let mut b = [0.; 16];
    assert_eq!(
        e.dispatch_gemma4_token(&index, 1, 0, &mut h, &mut a, &mut b),
        6
    );
    let ready = h;
    assert_eq!(
        e.dispatch_transformer_forward(&index, &mut h, 4, &mut a, &mut b, 1, 0),
        0
    );
    assert!(!e.dispatch_transformer_layer(&index, 0, 1, &mut h, 4, &mut a, &mut b));
    assert!(
        e.dispatch_prefill_chunk(&index, &mut h, 4, 1, 1, &mut a, &mut b, 0)
            .is_err()
    );
    assert!(e.snapshot_recurrent_state_into(&mut a).is_none());
    assert!(!e.restore_recurrent_state_from_slice(&a));
    let mut draft = crate::compute_universe::TopologyDraftBatch::empty();
    draft.draft_len = 1;
    let mut context = vec![1];
    assert_eq!(
        e.verify_topology_draft_batch(
            &index,
            &mut context,
            &draft,
            4,
            &mut h,
            &mut a,
            &mut b,
            0,
            0
        ),
        0
    );
    assert_eq!(context, [1]);
    assert_eq!(h, ready);
    assert_eq!(e.gemma4.as_ref().unwrap().position, 1);
    assert!(e.gemma4_ready(&index));
}
