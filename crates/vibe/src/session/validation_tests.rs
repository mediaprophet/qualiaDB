use super::*;
use std::cell::Cell;

#[test]
fn rejected_output_preserves_values_definitions_sources_and_revision() {
    let mut session = SourceSession::default();
    let mut host = crate::LocalHost::default();
    session.submit("let mut x: i64 = 1;", &mut host, Budget::default()).unwrap();
    let called = Cell::new(0);
    let error = session.submit_validated("x = 2;", &mut host, Budget::default(), |_| {
        called.set(called.get() + 1);
        Err(Diagnostic::new(DiagCode::E701, Span::point(0), "native result rejected"))
    }).unwrap_err();
    assert_eq!(called.get(), 1);
    assert!(error.execution_attempted && error.host_reconciliation_required);
    assert_eq!(error.active_revision, 1);
    assert_eq!(session.revision(), 1);
    assert_eq!(session.value("x"), Some(&Value::I64(1)));
    assert_eq!(session.sources(), &["let mut x: i64 = 1;".to_string()]);
    session.submit_validated("x = 3;", &mut host, Budget::default(), |_| Ok(())).unwrap();
    assert_eq!(session.value("x"), Some(&Value::I64(3)));
}

#[test]
fn foreign_checked_submission_never_evaluates_or_calls_validator() {
    let first = SourceSession::default();
    let candidate = first.prepare("let x: i64 = 1;").unwrap();
    let mut second = SourceSession::default();
    let mut host = crate::LocalHost::default();
    let error = second.execute_validated(candidate, &mut host, Budget::default(), |_| {
        panic!("stale candidates must not reach output validation")
    }).unwrap_err();
    assert!(!error.execution_attempted);
    assert_eq!(second.revision(), 0);
    assert!(second.sources().is_empty());
}
