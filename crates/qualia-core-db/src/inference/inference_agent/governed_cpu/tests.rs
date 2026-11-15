use super::*;
use crate::inference::inference_agent::governed::ConstraintInstruction as I;
use capabilities::check_projection_program;

#[test]
fn token_event_projection_refuses_metadata_and_float_placeholders() {
    assert!(check_projection_program(&[
        I::MatchSubject { value: 1 },
        I::MatchPredicate {
            value: observation::TOKEN_EVENT
        },
        I::Bind {
            vector: 3,
            register: 15
        },
        I::MatchRegister {
            vector: 0,
            register: 0
        },
        I::HaltIfFalse,
    ])
    .is_ok());
    for instruction in [
        I::Confidence { minimum: 0.0 },
        I::MetadataMask { mask: 0 },
        I::LessThan {
            vector: 2,
            value: 1.0,
        },
        I::GreaterThan {
            vector: 0,
            value: 1.0,
        },
        I::Bind {
            vector: 4,
            register: 0,
        },
        I::MatchRegister {
            vector: 0,
            register: 16,
        },
    ] {
        assert_eq!(
            check_projection_program(&[instruction]),
            Err(CpuAdmissionError::UnsupportedObservation)
        );
    }
}

#[test]
fn explicit_raw_request_binds_stop_order_bos_and_prompt() {
    use request::*;
    fn digest(text: &[u8], ids: &[u32], stops: &[u32], add_bos: bool) -> [u8; 32] {
        bind_encoded(
            RawRequest {
                text,
                encoded: ids,
                stops,
                max_new_tokens: 2,
                context: 8,
                add_bos,
                capabilities: Default::default(),
            },
            [1; 32],
            [2; 32],
            [3; 32],
            10,
            RequestBudget {
                max_text_bytes: 128,
                max_prompt_tokens: 8,
                max_stop_ids: 8,
                capacity_bytes: 256,
            },
        )
        .unwrap()
        .digest
    }
    let baseline = digest(b"a", &[1, 2], &[3, 4], true);
    for changed in [
        digest(b"b", &[1, 2], &[3, 4], true),
        digest(b"a", &[2, 1], &[3, 4], true),
        digest(b"a", &[1, 2], &[4, 3], true),
        digest(b"a", &[1, 2], &[3, 4], false),
        digest(b"a", &[1, 2], &[], true),
    ] {
        assert_ne!(baseline, changed);
    }
}

#[test]
fn host_preflight_refusal_and_drop_preserve_state_and_slots() {
    use state::*;
    let mut state = HostState {
        position: 3,
        current_token: 7,
        generated: 1,
        stopped: false,
        terminal: false,
    };
    let mut token = 99;
    let mut lineage = None;
    assert!(StagedHost::prepare(&mut state, &mut token, &mut lineage, 8, false, 1, 4).is_err());
    assert_eq!(
        (state.position, state.current_token, state.generated, token),
        (3, 7, 1, 99)
    );
    let pending = StagedHost::prepare(&mut state, &mut token, &mut lineage, 8, true, 2, 4).unwrap();
    drop(pending);
    assert_eq!(
        (state.position, state.current_token, state.generated, token),
        (3, 7, 1, 99)
    );
    assert!(!state.stopped);
    assert!(lineage.is_none());
}

#[test]
fn actual_event_encodes_integer_token_and_preserves_raw_logit_bits() {
    use crate::inference::inference_agent::governed::evidence::{
        BoundProposal, Payload, ScoreMeaning,
    };
    use etalon_inference_contract::SessionIdentity;
    fn context() -> observation::EventContext {
        observation::EventContext {
            identity: SessionIdentity {
                session_id: 1,
                session_generation: 2,
                request_id: 3,
                request_generation: 4,
                model_id: [5; 32],
                model_generation: 6,
                device_id: 7,
                device_generation: 8,
                policy_id: [9; 32],
                policy_generation: 10,
            },
            request: [11; 32],
            actor: 12,
            graph: 13,
            sequence: 1,
            position: 14,
            provenance: [15; 32],
        }
    }
    let p = observation::token_proposal(context(), 42, -0.0, &[0xC3]).unwrap();
    assert_eq!(p.observation.subject, 12);
    assert_eq!(p.observation.predicate, observation::TOKEN_EVENT);
    assert_eq!(
        p.observation.object,
        crate::frame_layout::INLINE_TAG_INTEGER | 42
    );
    assert_eq!(p.observation.context, 13);
    assert_eq!(
        p.observation.parity,
        12 ^ observation::TOKEN_EVENT ^ (crate::frame_layout::INLINE_TAG_INTEGER | 42) ^ 13
    );
    let score = p.numerical.unwrap();
    assert_eq!(score.meaning, ScoreMeaning::Logit);
    assert_eq!(score.value.to_bits(), (-0.0f32).to_bits());
    assert!(matches!(
        p.payload,
        Payload::Token {
            token: 42,
            position: 14,
            piece: [0xC3]
        }
    ));
    let minus = BoundProposal::bind(p, 1).unwrap().digest();
    let plus = observation::token_proposal(context(), 42, 0.0, &[0xC3]).unwrap();
    assert_ne!(minus, BoundProposal::bind(plus, 1).unwrap().digest());
    let control = observation::token_proposal(context(), 0, 5.0, &[]).unwrap();
    assert!(BoundProposal::bind(control, 1).is_ok());
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(observation::token_proposal(context(), 42, invalid, b"x").is_err());
    }
}

#[test]
fn request_refuses_capacity_and_unsupported_controls_before_binding() {
    use request::*;
    let make = |caps| RawRequest {
        text: b"a",
        encoded: &[1],
        stops: &[2],
        max_new_tokens: 2,
        context: 2,
        add_bos: false,
        capabilities: caps,
    };
    let budget = RequestBudget {
        max_text_bytes: 1,
        max_prompt_tokens: 1,
        max_stop_ids: 1,
        capacity_bytes: 9,
    };
    assert!(bind_encoded(
        make(Default::default()),
        [1; 32],
        [2; 32],
        [3; 32],
        3,
        budget
    )
    .is_ok());
    assert!(matches!(
        bind_encoded(
            make(Default::default()),
            [1; 32],
            [2; 32],
            [3; 32],
            3,
            RequestBudget {
                capacity_bytes: 8,
                ..budget
            }
        ),
        Err(CpuAdmissionError::Budget)
    ));
    let unsupported = capabilities::RequestedCapabilities {
        sampling: true,
        ..Default::default()
    };
    assert!(matches!(
        bind_encoded(make(unsupported), [1; 32], [2; 32], [3; 32], 3, budget),
        Err(CpuAdmissionError::UnsupportedCapability)
    ));
    assert!(matches!(
        bind_encoded(
            make(Default::default()),
            [1; 32],
            [2; 32],
            [3; 32],
            2,
            budget
        ),
        Err(CpuAdmissionError::InvalidRequest)
    ));
}
