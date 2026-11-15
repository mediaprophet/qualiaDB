//! Assignment admission must precede RHS host effects, including unchecked AST entry.
use vibe::{Budget, DiagCode, Diagnostic, Engine, Env, Host, Span, Value};
#[derive(Default)]
struct Effects {
    dispatches: usize,
}
impl Host for Effects {
    fn graph_query(&mut self, _: &[Value], _: u64, _: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_stage(&mut self, _: &Value, _: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_commit(&mut self, _: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn aura_validate(&mut self, _: &Value, _: &Value, _: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn pulse_publish(&mut self, _: &str, _: &Value, _: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn capability_invoke(&mut self, _: &str, _: &Value, _: Span) -> Result<Value, Diagnostic> {
        self.dispatches += 1;
        Ok(Value::I64(9))
    }
}
fn source(binding: &str, target: &str) -> String {
    format!(
        r#"requires [capability("capability.invoke")]; {binding} {target} = capability.invoke("Fixture.assignment", {{}});"#
    )
}
#[test]
fn member_and_index_assignment_are_rejected_before_rhs_and_preserve_session() {
    for (binding, target) in [
        ("let mut x = {value: 1};", "x.value"),
        ("let mut x = [1];", "x[0]"),
    ] {
        let text = source(binding, target);
        let program = vibe::parse_program(&text).unwrap();
        assert_eq!(
            vibe::check_program(&program).unwrap_err().code,
            DiagCode::E701
        );
        let mut host = Effects::default();
        let mut env = Env::default();
        let error = Engine::new(&mut host, Budget::default())
            .eval_program(&program, &mut env)
            .unwrap_err();
        assert_eq!(error.code, DiagCode::E701);
        assert_eq!(host.dispatches, 0);
        let mut session =
            vibe::session::SourceSession::new(vec!["capability.invoke".into()], Default::default());
        let error = session
            .submit(&text, &mut host, Budget::default())
            .unwrap_err();
        assert_eq!(error.diagnostic.code, DiagCode::E701);
        assert!(!error.execution_attempted);
        assert_eq!(host.dispatches, 0);
        assert_eq!(session.revision(), 0);
    }
}
#[test]
fn immutable_identifier_refuses_before_rhs_even_without_checker() {
    let program = vibe::parse_program(&source("let x = 1;", "x")).unwrap();
    let mut host = Effects::default();
    let mut env = Env::default();
    let error = Engine::new(&mut host, Budget::default())
        .eval_program(&program, &mut env)
        .unwrap_err();
    assert_eq!(error.code, DiagCode::E701);
    assert_eq!(host.dispatches, 0);
    assert_eq!(env.vars.get("x"), Some(&Value::I64(1)));
}
#[test]
fn supported_mutable_whole_binding_still_evaluates_rhs_once() {
    let program = vibe::parse_program(&source("let mut x = 1;", "x")).unwrap();
    vibe::check_program(&program).unwrap();
    let mut host = Effects::default();
    let mut env = Env::default();
    let value = Engine::new(&mut host, Budget::default())
        .eval_program(&program, &mut env)
        .unwrap();
    assert_eq!(value, Value::I64(9));
    assert_eq!(env.vars.get("x"), Some(&Value::I64(9)));
    assert_eq!(host.dispatches, 1);
}
