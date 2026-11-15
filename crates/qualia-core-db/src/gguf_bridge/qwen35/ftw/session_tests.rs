use super::*;
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
#[path = "session_fixture.rs"]
mod fixture;

#[test]
fn complete_token_advances_once_and_reset_replays_identically() {
    let fixture = fixture::complete_fixture();
    let mut session = Session::new(fixture.plan.clone(), 1024 * 1024).unwrap();
    // One full-attention layer: two positions, one KV head, sixteen channels,
    // separate F32 key and value arrays. This measures allocated capacity.
    assert_eq!(session.kv_cache_bytes(), 2 * 1 * 16 * 2 * 4);
    let mut hidden = [0f32; 16];
    session.embed(0, &mut hidden).unwrap();
    let mut first = None;
    assert_zero_alloc("qwen35_ftw_complete_token", || {
        first = Some(session.step(&mut hidden, 0).unwrap());
    });
    let first = first.unwrap();
    assert!(first.1.is_finite());
    assert_eq!(session.position(), Ok(1));
    assert_eq!(session.step(&mut hidden, 0), Err(SessionError::Position));
    session.embed(1, &mut hidden).unwrap();
    session
        .prefill_controlled(&mut hidden, 1, &|| false)
        .unwrap();
    assert_eq!(session.position(), Ok(2));
    assert_eq!(session.step(&mut hidden, 2), Err(SessionError::ContextFull));
    let mut replay = None;
    assert_zero_alloc("qwen35_ftw_reset_embed_and_replay", || {
        session.reset();
        session.embed(0, &mut hidden).unwrap();
        replay = Some(session.step(&mut hidden, 0).unwrap());
    });
    let replay = replay.unwrap();
    assert_eq!(first, replay);
    assert_eq!(session.position(), Ok(1));
}

#[test]
fn engine_bridge_rejects_replaced_source_and_mismatched_tensor_directory() {
    let fixture = fixture::complete_fixture();
    let mut engine = crate::gguf_bridge::QTensorEngine::new();
    let index = fixture.plan.pkg.tensor_index.clone();
    engine.tensor_index_cache = Some(index.clone());
    engine.hyperparams = index.hyperparams;
    engine.tensor_data_offset = index.tensor_data_start;
    engine.ftw_package = Some(fixture.plan.pkg.clone());
    engine.qwen35_ftw = Some(Session::new(fixture.plan.clone(), 1024 * 1024).unwrap());
    assert_eq!(engine.kv_cache_bytes(), 256);
    assert!(engine.qwen35_plan(&index).is_some());
    let error = engine
        .adopt_ftw_package_with_budget(fixture.path(), 2, 1)
        .unwrap_err();
    assert!(
        error.contains("budget"),
        "tiny caller budget must fail before session construction: {error}"
    );
    assert!(
        engine.qwen35_plan(&index).is_some(),
        "failed budget admission must preserve previous owner"
    );
    engine.ftw_package = Some(Arc::new((*fixture.plan.pkg).clone()));
    assert!(
        engine.qwen35_plan(&index).is_none(),
        "equal bytes cannot replace retained source identity"
    );
    assert!(engine.take_qwen35_session(&index).is_none());
    engine.ftw_package = Some(fixture.plan.pkg.clone());
    let mut changed = index.clone();
    changed.tensor_data_start = changed.tensor_data_start.wrapping_add(1);
    assert!(engine.qwen35_plan(&changed).is_none());
    assert!(engine.take_qwen35_session(&changed).is_none());
    engine.tensor_data_offset = engine.tensor_data_offset.wrapping_add(1);
    assert!(engine.qwen35_plan(&index).is_none());
    engine.tensor_data_offset = index.tensor_data_start;
    engine.tensor_index_cache = None;
    assert!(engine.qwen35_plan(&index).is_none());
    engine.tensor_index_cache = Some(index.clone());
    assert!(engine.qwen35_plan(&index).is_some());
    assert!(engine.take_qwen35_session(&index).is_some());
    assert!(engine.qwen35_plan(&index).is_none());
}

#[test]
fn partial_layer_error_and_cancellation_poison_real_session_until_reset() {
    let fixture = fixture::fixture();
    assert!(matches!(
        Session::new(fixture.plan.clone(), 0),
        Err(SessionError::Budget)
    ));
    let mut session = Session::new(fixture.plan.clone(), 1024 * 1024).unwrap();
    let mut hidden = [0f32; 16];
    session.embed(0, &mut hidden).unwrap();
    assert_eq!(session.step(&mut hidden, 0), Err(SessionError::Provider));
    assert!(
        session.buffers.layers[0].keys.iter().any(|x| *x != 0.0),
        "attention committed before missing router failed"
    );
    assert_eq!(session.position(), Err(SessionError::Invalidated));
    assert_eq!(
        session.attention(0, &mut hidden),
        Err(SessionError::Invalidated)
    );
    session.reset();
    assert_eq!(session.position(), Ok(0));
    assert!(session.buffers.layers[0].keys.iter().all(|x| *x == 0.0));
    assert!(session.buffers.layers[0].values.iter().all(|x| *x == 0.0));
    let checks = std::cell::Cell::new(0);
    session.embed(0, &mut hidden).unwrap();
    assert_eq!(
        session.step_controlled(&mut hidden, 0, &|| {
            checks.set(checks.get() + 1);
            checks.get() == 2
        }),
        Err(SessionError::Canceled)
    );
    assert!(session.buffers.layers[0].keys.iter().any(|x| *x != 0.0));
    assert!(!session.is_valid());
    session.reset();
    session.embed(0, &mut hidden).unwrap();
    session.attention(0, &mut hidden).unwrap();
    assert!(hidden.iter().all(|x| x.is_finite()));
    assert_eq!(
        session.position(),
        Ok(0),
        "standalone lab layer cannot commit a full token"
    );
}

#[test]
fn incomplete_token_poison_and_commit_are_distinct() {
    let mut sequence = Sequence {
        cursor: 0,
        valid: true,
    };
    {
        let _token = Token {
            sequence: &mut sequence,
            committed: false,
        };
        // A partial numerical write is followed by an error before commit.
    }
    assert!(!sequence.valid);
    assert_eq!(sequence.cursor, 0);
    sequence = Sequence {
        cursor: 0,
        valid: true,
    };
    Token {
        sequence: &mut sequence,
        committed: false,
    }
    .commit();
    assert!(sequence.valid);
    assert_eq!(sequence.cursor, 1);
    Token {
        sequence: &mut sequence,
        committed: false,
    }
    .commit_partial();
    assert!(sequence.valid);
    assert_eq!(
        sequence.cursor, 1,
        "lab execution cannot commit a full token"
    );
}

#[test]
fn unwind_poison_prevents_reusing_partially_advanced_states() {
    let mut sequence = Sequence {
        cursor: 3,
        valid: true,
    };
    let mut recurrent = [0f32; 4];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _token = Token {
            sequence: &mut sequence,
            committed: false,
        };
        recurrent[0] = 9.0;
        panic!("injected partial layer failure");
    }));
    assert!(result.is_err());
    assert_eq!(recurrent[0], 9.0);
    assert!(!sequence.valid);
    assert_eq!(sequence.cursor, 3);
}

#[test]
fn state_budget_arithmetic_fails_before_allocation() {
    assert_eq!(add(usize::MAX, 1), Err(SessionError::Budget));
    assert_eq!(mul(usize::MAX, 4), Err(SessionError::Budget));
}
