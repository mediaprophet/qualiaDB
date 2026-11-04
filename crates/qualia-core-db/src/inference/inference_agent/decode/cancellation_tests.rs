//! Native publication regression harness: real installed signed Allow, authority,
//! gate, scalar publication and finalization; no model/numerical qualification.
use super::super::native_request::{cancellation_fixture as fixture, PublicationGate};
use super::*;

fn publish(gate: &mut PublicationGate<'_, '_>, closed: bool) -> (bool, Vec<u32>, Vec<String>) {
    let tok = crate::gguf_sharder::GgufTokenizer::default();
    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    let rx = if closed {
        drop(rx);
        None
    } else {
        Some(rx)
    };
    let mut ids = Vec::new();
    let mut ctx = vec![0];
    let mut sieve = None;
    let mut trace = StageTrace::new();
    let mut thinking = ThinkingTracker::new();
    let result = publication::run(publication::Publication {
        gate,
        top_v: 2.,
        top_i: 65,
        vlen: 256,
        tok: &tok,
        sieve: &mut sieve,
        out_ids: &mut ids,
        ctx: &mut ctx,
        prov_hash: 71,
        semantic_quin: None,
        sieve_failed: false,
        stage_telemetry: false,
        ssm_layer_count: 0,
        moe_layer_count: 0,
        stage_trace: &mut trace,
        track_thinking: false,
        think_piece: &mut [0; 64],
        think_tracker: &mut thinking,
        thinking_budget: None,
        stream_tx_thread: &Some(tx),
        streamed_len: 0,
        gen_budget: 4,
    });
    let deltas = rx.map(|rx| rx.try_iter().collect()).unwrap_or_default();
    (result.0, ids, deltas)
}

fn finish(gate: &mut PublicationGate<'_, '_>) -> WorkerResult {
    finish_with_closed_ring(gate, false)
}
fn finish_with_closed_ring(gate: &mut PublicationGate<'_, '_>, closed: bool) -> WorkerResult {
    let (mut lp, _consumer) = rtrb::RingBuffer::new(8);
    let _consumer = if closed {
        drop(_consumer);
        None
    } else {
        Some(_consumer)
    };
    let tok = crate::gguf_sharder::GgufTokenizer::default();
    render::run(render::Finalization {
        gate,
        t_decode: std::time::Instant::now(),
        out_ids: &vec![65],
        stage_telemetry: false,
        stage_trace: StageTrace::new(),
        lp: &mut lp,
        semantic_quin: Some(NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 71,
            metadata: 0,
            parity: 1 ^ 2 ^ 3 ^ 71,
        }),
        sieve_failed: false,
        tok: &tok,
        prompt_owned: "a",
    })
}

#[test]
#[serial_test::serial]
fn native_cancellation_after_signed_allow_preparation_blocks_scalar_publication() {
    let control = DecodeControl::default();
    fixture::with_gate(&control, true, |gate| {
        assert!(!control.is_cancelled()); // outer numerical-loop check passed
        let (stop, ids, deltas) = publish(gate, false);
        assert!(control.is_cancelled()); // cancelled after actual policy preparation
        assert!(stop);
        assert!(ids.is_empty());
        assert!(deltas.is_empty());
        assert_eq!(fixture::counts(gate), (0, 1, 1));
        assert!(gate.is_terminal());
        assert!(!gate.reusable());
        let result = finish(gate);
        assert!(result.0.is_empty());
        assert_eq!(result.1, 0);
        assert!(result.2.is_none());
        assert_eq!(fixture::counts(gate), (0, 1, 1));
    });
}

#[test]
#[serial_test::serial]
fn native_signed_allow_positive_control_publishes_and_finalizes_artifact() {
    fixture::with_gate(&DecodeControl::default(), false, |gate| {
        let (stop, ids, deltas) = publish(gate, false);
        assert!(!stop);
        assert_eq!(ids, vec![65]);
        assert_eq!(deltas, vec![String::from("A")]);
        assert_eq!(fixture::counts(gate), (1, 2, 1));
        let result = finish(gate);
        assert!(result.2.is_some());
        assert_eq!(fixture::counts(gate), (1, 2, 2));
        assert!(!gate.is_terminal());
        assert!(gate.reusable());
    });
}

#[test]
#[serial_test::serial]
fn native_cancellation_after_tokens_refuses_final_semantic_artifact() {
    let control = DecodeControl::default();
    fixture::with_gate(&control, false, |gate| {
        assert_eq!(publish(gate, false).1, vec![65]);
        control.cancel();
        let result = finish(gate);
        assert!(result.0.is_empty());
        assert_eq!(result.1, 0);
        assert!(result.2.is_none());
        assert_eq!(fixture::counts(gate), (1, 2, 1));
        assert!(gate.is_terminal());
        assert!(!gate.reusable());
    });
}

#[test]
#[serial_test::serial]
fn native_closed_stream_is_terminal_and_refuses_final_artifact() {
    fixture::with_gate(&DecodeControl::default(), false, |gate| {
        assert!(publish(gate, true).0);
        assert!(gate.is_terminal());
        assert!(!gate.reusable());
        let result = finish(gate);
        assert!(result.0.is_empty());
        assert!(result.2.is_none());
        assert_eq!(fixture::counts(gate), (1, 2, 1));
    });
}

#[test]
#[serial_test::serial]
fn native_developer_cancellation_also_refuses_publication_and_finalization() {
    let control = DecodeControl::default();
    let mut gate = PublicationGate::Developer {
        control: None,
        terminal: false,
    };
    gate.bind_control(Some(control.clone()));
    control.cancel();
    let (stop, ids, deltas) = publish(&mut gate, false);
    assert!(stop);
    assert!(ids.is_empty());
    assert!(deltas.is_empty());
    assert!(!gate.reusable());
    assert!(finish(&mut gate).0.is_empty());
}

#[test]
#[serial_test::serial]
fn native_closed_ring_refuses_final_artifact() {
    fixture::with_gate(&DecodeControl::default(), false, |gate| {
        let result = finish_with_closed_ring(gate, true);
        assert!(result.0.is_empty());
        assert!(result.2.is_none());
        assert_eq!(fixture::counts(gate), (0, 1, 1));
        assert!(gate.is_terminal());
        assert!(!gate.reusable());
    });
}

#[test]
#[serial_test::serial]
fn native_exact_output_record_and_all_token_sources_refuse_cancelled_acceptance() {
    use super::super::governed::evidence::CandidateSource;
    let tok = crate::gguf_sharder::GgufTokenizer::default();
    fixture::with_gate(&DecodeControl::default(), false, |gate| {
        let mut output = None;
        gate.token(65, 0, CandidateSource::ModelToken, None, &tok, &mut output)
            .unwrap();
        assert_eq!(output.take().unwrap().token_id(), 65);
        assert_eq!(fixture::counts(gate), (1, 2, 1));
    });
    for source in [
        CandidateSource::ModelToken,
        CandidateSource::GraphForcedToken,
        CandidateSource::SpeculativeToken,
        CandidateSource::TopologyToken,
    ] {
        fixture::with_gate(&DecodeControl::default(), true, |gate| {
            let mut output = None;
            assert!(gate.token(65, 0, source, None, &tok, &mut output).is_err());
            assert!(output.is_none());
            assert_eq!(fixture::counts(gate), (0, 1, 1));
            assert!(gate.is_terminal());
            assert!(!gate.reusable());
        });
    }
}

#[test]
#[serial_test::serial]
fn native_cancellation_during_artifact_preparation_refuses_transfer() {
    use super::super::governed::evidence::{ArtifactMedia, ArtifactOrigin, CandidateSource};
    fixture::with_gate(&DecodeControl::default(), true, |gate| {
        let mut bytes = *b"artifact";
        assert!(gate
            .artifact_in_place(
                CandidateSource::TextReplacement,
                ArtifactOrigin::GroundedReplacement,
                ArtifactMedia::Utf8Text,
                &mut bytes
            )
            .is_err());
        assert_eq!(&bytes, b"artifact");
        assert_eq!(fixture::counts(gate), (0, 1, 1));
        assert!(gate.is_terminal());
    });
}
