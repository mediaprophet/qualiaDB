//! Compute-fabric tests; process-wide lifecycle fixtures live in test_support.
use super::*;

#[test]
fn context_ring_push_pop() {
    test_support::after_closed_request("context_ring_push_pop", || {
        let ring = ContextInjectRing::new();
        let token = ContextInjectToken {
            tensor_index: 1,
            subject_hash: 42,
            distance: 0.5,
            manifold_w: 0.0,
        };
        assert!(
            !ring.try_push(token),
            "closed request must reject publication"
        );
        let request = begin_request_epoch();
        assert!(
            !ring.try_push(token),
            "unactivated request must reject publication"
        );
        assert!(request.activate(|| {}));
        assert!(ring.try_push(token));
        let t = ring.try_pop().expect("token");
        assert_eq!(t.tensor_index, 1);
        assert!(ring.is_empty());
        assert!(ring.try_push(token));
        drop(request);
        assert!(
            !ring.try_push(token),
            "canceled request must reject publication"
        );
        assert!(
            ring.try_pop().is_none(),
            "canceled envelope must not escape"
        );
        let next = begin_request_epoch();
        assert!(next.activate(|| {}));
        assert!(ring.try_push(token));
        drop(next);
        let final_request = begin_request_epoch();
        assert!(final_request.activate(|| {}));
        assert!(
            ring.try_pop().is_none(),
            "new request must not revive old envelopes"
        );
        assert!(ring.try_push(token));
        assert_eq!(ring.try_pop().unwrap().tensor_index, 1);
    });
}

#[test]
fn substrate_node_count_from_bytes() {
    let header = std::mem::size_of::<TensorBufferHeader>() as u64;
    let sub = GraphTensorSubstrate::from_tensor_bytes(header + 40 * 10);
    assert_eq!(sub.node_count, 10);
}

#[test]
fn u0_has_sentinel_primitive() {
    let prims = ComputeUniverse::LlmInference.qualia_primitives();
    assert!(prims.contains(&QualiaPrimitive::Phase8Sentinel));
}

#[test]
fn attention_mask_sets_bits() {
    let mut mask = AttentionRouteMask::default();
    mask.set_index(100);
    assert!(mask.is_set(100));
    assert!(!mask.is_set(101));
}

#[test]
fn push_tensor_search_hits_drains_to_ring() {
    // Other tests start process-wide producers that can refill this SPSC ring.
    // Exercise the public API in a fresh process with the same assertions.
    #[cfg(not(target_arch = "wasm32"))]
    {
        const CHILD: &str = "QUALIA_CONTEXT_RING_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "inference::compute_universe::tests::push_tensor_search_hits_drains_to_ring",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .expect("run isolated context-ring test");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success()
                    && stdout.contains("running 1 test")
                    && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
                "isolated context-ring test did not execute successfully: {}\n{}\n{}",
                output.status,
                stdout,
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
    }
    let pushed = push_tensor_search_hits(&[3, 7], 99, 0.5, 1.0);
    assert_eq!(pushed, 2);
    assert_eq!(pop_tensor_context().unwrap().tensor_index, 3);
    assert_eq!(pop_tensor_context().unwrap().tensor_index, 7);
}

#[test]
fn build_attention_mask_from_hits() {
    let query = Tensor10D::default();
    let mask = build_attention_route_mask(&query, 1.0, &[2, 5, 9], 3);
    assert!(mask.is_set(2));
    assert!(mask.is_set(5));
    assert_eq!(mask.active_bits, 3);
}

#[test]
fn attention_kv_mask_maps_tensor_bits() {
    let mut route = AttentionRouteMask::default();
    route.set_index(5);
    route.set_index(12);
    publish_attention_route_mask(route);
    let (words, active) = attention_kv_mask_u32(20, 1024);
    assert_eq!(active, 1);
    assert_ne!(words[0] & (1u32 << 5), 0);
    assert_ne!(words[0] & (1u32 << 12), 0);
    assert_ne!(words[0] & (1u32 << 20), 0);
}

#[test]
fn topology_draft_ring_roundtrip() {
    test_support::after_closed_request("topology_draft_ring_roundtrip", || {
        let ring = TopologyDraftRing::new();
        let batch = TopologyDraftBatch {
            draft_len: 2,
            _pad: [0; 7],
            draft_ids: [10, 20, 0, 0, 0, 0, 0, 0],
            concept_hashes: [1, 2, 0, 0, 0, 0, 0, 0],
        };
        assert!(
            !ring.try_push(batch),
            "closed request must reject publication"
        );
        let request = begin_request_epoch();
        assert!(
            !ring.try_push(batch),
            "unactivated request must reject publication"
        );
        assert!(request.activate(|| {}));
        assert!(ring.try_push(batch));
        let popped = ring.try_pop().unwrap();
        assert_eq!(popped.draft_len, 2);
        assert_eq!(popped.draft_ids[0], 10);
        assert!(ring.try_push(batch));
        drop(request);
        assert!(
            !ring.try_push(batch),
            "canceled request must reject publication"
        );
        assert!(ring.try_pop().is_none(), "canceled draft must not escape");
        let next = begin_request_epoch();
        assert!(next.activate(|| {}));
        assert!(ring.try_push(batch));
        drop(next);
        let final_request = begin_request_epoch();
        assert!(final_request.activate(|| {}));
        assert!(
            ring.try_pop().is_none(),
            "new request must not revive old drafts"
        );
        assert!(ring.try_push(batch));
        assert_eq!(ring.try_pop().unwrap().draft_ids[0], 10);
    });
}

#[test]
fn sentinel_allows_nonempty_drafts_including_former_false_positives() {
    let clean = TopologyDraftBatch {
        draft_len: 1,
        _pad: [0; 7],
        draft_ids: [42, 0, 0, 0, 0, 0, 0, 0],
        concept_hashes: [7, 0, 0, 0, 0, 0, 0, 0],
    };
    assert!(sentinel_allows_topology_draft(&clean));

    // Token id / hash low byte 0x99 is legitimate (~1/256) — no longer rejected.
    let former_false_positive = TopologyDraftBatch {
        draft_len: 1,
        _pad: [0; 7],
        draft_ids: [0x99, 0, 0, 0, 0, 0, 0, 0],
        concept_hashes: [0x99, 0, 0, 0, 0, 0, 0, 0],
    };
    assert!(sentinel_allows_topology_draft(&former_false_positive));

    let empty = TopologyDraftBatch {
        draft_len: 0,
        _pad: [0; 7],
        draft_ids: [0; 8],
        concept_hashes: [0; 8],
    };
    assert!(!sentinel_allows_topology_draft(&empty));
}

#[test]
fn producer_cycle_with_global_substrate() {
    test_support::after_closed_request("producer_cycle_with_global_substrate", || {
        let tensors = [
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        ];
        global_resident_substrate()
            .load_from_tensors(&tensors, 77)
            .unwrap();
        assert_eq!(run_tensor_search_producer_cycle(2.0, 32_000), 0);
        let request = begin_request_epoch();
        assert_eq!(run_tensor_search_producer_cycle(2.0, 32_000), 0);
        assert!(request.activate(|| publish_query_tensor(tensors[0], 77)));
        let pushed = run_tensor_search_producer_cycle(2.0, 32_000);
        assert!(pushed >= 1);
        assert!(attention_route_mask().active_bits >= 1);
        while pop_tensor_context().is_some() {}
        drop(request);
        assert_eq!(run_tensor_search_producer_cycle(2.0, 32_000), 0);
        assert_eq!(attention_route_mask().active_bits, 0);
        assert!(pop_tensor_context().is_none());
        assert!(pop_topology_draft().is_none());
    });
}

#[test]
fn verify_topology_draft_full_acceptance() {
    let draft = TopologyDraftBatch {
        draft_len: 3,
        _pad: [0; 7],
        draft_ids: [100, 200, 300, 0, 0, 0, 0, 0],
        concept_hashes: [1, 2, 3, 0, 0, 0, 0, 0],
    };
    // Target model predicted [100, 200, 300, 400]
    let target = [100, 200, 300, 400];
    let result = verify_topology_draft(&draft, &target);
    assert_eq!(result.accepted_count, 3);
    assert!(result.fully_accepted);
    assert_eq!(result.emitted_token, 400); // bonus token from target
}

#[test]
fn verify_topology_draft_partial_acceptance_with_correction() {
    let draft = TopologyDraftBatch {
        draft_len: 3,
        _pad: [0; 7],
        draft_ids: [100, 200, 300, 0, 0, 0, 0, 0],
        concept_hashes: [1, 2, 3, 0, 0, 0, 0, 0],
    };
    // Diverges at position 1 (target predicted 999 instead of 200)
    let target = [100, 999, 500];
    let result = verify_topology_draft(&draft, &target);
    assert_eq!(result.accepted_count, 1);
    assert!(!result.fully_accepted);
    assert_eq!(result.emitted_token, 999); // corrected token
}
