use super::*;

#[test]
fn tentative_bindings_are_visible_before_promotion_and_rejection_preserves_session() {
    let mut session = SourceSession::default();
    let mut host = crate::LocalHost::default();
    session.submit("let mut x: i64 = 1;", &mut host, Budget::default()).unwrap();
    let error = session.submit_validated_bindings("x = 2;", &mut host, Budget::default(), |_, bindings, _| {
        assert_eq!(bindings.value("x"), Some(&Value::I64(2)));
        Err(Diagnostic::new(DiagCode::E701, Span::point(0), "binding refused"))
    }).unwrap_err();
    assert!(error.execution_attempted && error.host_reconciliation_required);
    assert_eq!(session.value("x"), Some(&Value::I64(1)));
    assert_eq!(session.revision(), 1);
    assert_eq!(session.sources().len(), 1);
}

#[test]
fn accepted_bindings_callback_receives_original_host_once() {
    let mut session = SourceSession::default();
    let mut host = crate::LocalHost::default();
    let expected_host = &mut host as *mut crate::LocalHost;
    let mut calls = 0;
    session.submit_validated_bindings("let x: i64 = 7;", &mut host, Budget::default(), |_, bindings, actual_host| {
        assert!(std::ptr::eq(actual_host as *mut _, expected_host));
        assert_eq!(bindings.value("x"), Some(&Value::I64(7)));
        calls += 1;
        Ok(())
    }).unwrap();
    assert_eq!(calls, 1);
    assert_eq!(session.value("x"), Some(&Value::I64(7)));
    assert_eq!(session.revision(), 1);
}

#[test]
fn foreign_checked_binding_candidate_never_calls_validator() {
    let first = SourceSession::default();
    let candidate = first.prepare("let x: i64 = 1;").unwrap();
    let mut second = SourceSession::default();
    let mut host = crate::LocalHost::default();
    let error = second.execute_validated_bindings(candidate, &mut host, Budget::default(), |_, _, _| {
        panic!("foreign candidate must not execute callback")
    }).unwrap_err();
    assert!(!error.execution_attempted);
    assert_eq!(second.revision(), 0);
    assert!(second.sources().is_empty());
}