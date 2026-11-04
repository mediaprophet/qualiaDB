use super::*;
use crate::wasm_llm::execution_mode::configure_browser_execution_mode;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[test]
fn developer_round_trip_does_not_reauthorize_an_earlier_generation() {
    configure_browser_execution_mode("developer", true).unwrap();
    let first = developer_execution_generation();
    assert!(first.is_some());
    configure_browser_execution_mode("governed", false).unwrap();
    assert!(developer_execution_generation().is_none());
    configure_browser_execution_mode("developer", true).unwrap();
    assert_ne!(first, developer_execution_generation());
    configure_browser_execution_mode("governed", false).unwrap();
}

#[test]
fn public_agent_refuses_default_and_revoked_developer_execution_without_streaming() {
    configure_browser_execution_mode("governed", false).unwrap();
    let agent = LocalLlmAgent::new("did:q42:test", "test.p64");
    let streamed = Arc::new(AtomicUsize::new(0));
    let count = streamed.clone();
    let (text, _, tokens, semantic) = agent.infer_local_model_streaming(
        "hello",
        "test",
        Some(move |_| {
            count.fetch_add(1, Ordering::Relaxed);
        }),
    );
    assert!(text.contains("governed inference is unavailable"));
    assert_eq!(tokens, 0);
    assert!(semantic.is_none());
    assert_eq!(streamed.load(Ordering::Relaxed), 0);

    configure_browser_execution_mode("developer", true).unwrap();
    assert!(crate::wasm_llm::execution_mode::require_developer().is_ok());
    configure_browser_execution_mode("governed", false).unwrap();
    let (text, _, tokens, semantic) =
        agent.infer_local_model_streaming("hello", "test", None::<fn(String)>);
    assert!(text.contains("governed inference is unavailable"));
    assert_eq!(tokens, 0);
    assert!(semantic.is_none());
}

#[test]
fn explicit_developer_without_an_engine_returns_diagnostic_instead_of_panicking() {
    configure_browser_execution_mode("developer", true).unwrap();
    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|slot| *slot.borrow_mut() = None);
    let agent = LocalLlmAgent::new("did:q42:test", "test.p64");
    let (text, _, tokens, semantic) =
        agent.infer_local_model_streaming("hello", "test", None::<fn(String)>);
    assert!(text.contains("engine not initialized"));
    assert_eq!(tokens, 0);
    assert!(semantic.is_none());
    configure_browser_execution_mode("governed", false).unwrap();
}
