use super::*;
#[test]
fn retained_mutability_is_checked_before_execution() {
    let mut session = SourceSession::default();
    let mut host = crate::LocalHost::default();
    session
        .submit("let mut x: i64 = 1;", &mut host, Budget::default())
        .unwrap();
    session
        .submit("x = 2;", &mut host, Budget::default())
        .unwrap();
    assert_eq!(session.value("x"), Some(&Value::I64(2)));
}
