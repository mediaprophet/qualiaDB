use super::{
    compile, compile_expr,
    op::{Chunk, Op},
    Vm,
};
use crate::{
    bind::{Host, LocalHost},
    budget::Budget,
    error::Diagnostic,
    span::Span,
    Value,
};

#[derive(Default)]
struct CountingHost {
    inner: LocalHost,
    stages: usize,
}
impl Host for CountingHost {
    fn graph_query(&mut self, args: &[Value], take: u64, span: Span) -> Result<Value, Diagnostic> {
        self.inner.graph_query(args, take, span)
    }
    fn graph_stage(&mut self, term: &Value, span: Span) -> Result<Value, Diagnostic> {
        self.stages += 1;
        self.inner.graph_stage(term, span)
    }
    fn graph_commit(&mut self, span: Span) -> Result<Value, Diagnostic> {
        self.inner.graph_commit(span)
    }
    fn aura_validate(
        &mut self,
        node: &Value,
        shape: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.inner.aura_validate(node, shape, span)
    }
    fn pulse_publish(
        &mut self,
        topic: &str,
        payload: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.inner.pulse_publish(topic, payload, span)
    }
}

#[test]
fn compiler_refuses_scoped_tasks_before_any_chunk_can_execute() {
    for call in [
        "task.spawn(\"=42\", {})",
        "task.status({})",
        "task.await({})",
        "task.cancel({})",
        "task.with_timeout({}, 1)",
        "task.nursery([])",
        "code.spawn_at(\"did:q42:local_scheduler\", \"=42\", {})",
        "await pending",
    ] {
        let program =
            crate::parse_program(&format!("module admission; graph.stage(42); {call};")).unwrap();
        assert!(
            compile(&program).is_err(),
            "must decline entire compilation: {call}"
        );
    }
    let alias = crate::parse_program("import \"vibe:0.1/task\" as jobs; effect fn main() budget(steps:20) { return jobs.spawn(\"=42\", {}); }").unwrap();
    assert!(compile(&alias).is_err());
}

#[test]
fn vm_admission_refuses_later_scoped_host_call_before_earlier_stage() {
    let mut chunk = Chunk::new();
    let stage = chunk.add_string("graph.stage");
    let task = chunk.add_string("task.spawn");
    chunk.code.push(Op::PushInt as u8);
    chunk.code.extend_from_slice(&42i64.to_le_bytes());
    chunk.code.push(Op::CallHost as u8);
    chunk.code.extend_from_slice(&stage.to_le_bytes());
    chunk.code.push(1);
    chunk.code.push(Op::Pop as u8);
    chunk.code.push(Op::CallHost as u8);
    chunk.code.extend_from_slice(&task.to_le_bytes());
    chunk.code.push(0);
    chunk.code.push(Op::Return as u8);
    // The same guard applies after binary round-trip, not just our AST compiler.
    let bytes = super::encode_chunk(&chunk);
    let decoded = super::decode_chunk(&bytes).unwrap();
    let mut host = CountingHost::default();
    assert!(Vm::new(&decoded, &mut host, Budget::default())
        .run()
        .is_err());
    assert_eq!(host.stages, 0);
}

#[test]
fn literal_await_and_static_non_task_host_calls_remain_usable() {
    let mut host = LocalHost::default();
    let literal = compile_expr(&crate::parse_cell("= await 42").unwrap()).unwrap();
    assert_eq!(
        Vm::new(&literal, &mut host, Budget::default())
            .run()
            .unwrap(),
        Value::I64(42)
    );
    let math = compile_expr(&crate::parse_cell("= math.sqrt(16.0)").unwrap()).unwrap();
    assert_eq!(
        Vm::new(&math, &mut host, Budget::default()).run().unwrap(),
        Value::F64(4.0)
    );
}

#[test]
fn bytecode_cannot_jump_into_operands_to_hide_an_unadmitted_call() {
    let mut chunk = Chunk::new();
    chunk.code.push(Op::PushInt as u8);
    chunk.code.extend_from_slice(&0i64.to_le_bytes());
    chunk.code.push(Op::Jump as u8);
    chunk.code.extend_from_slice(&1u16.to_le_bytes()); // Inside PushInt's operand.
    chunk.code.push(Op::Halt as u8);
    assert!(
        Vm::new(&chunk, &mut LocalHost::default(), Budget::default())
            .run()
            .is_err()
    );
}

#[test]
fn conditional_edges_land_on_instructions_and_function_fallthrough_stays_local() {
    let program = crate::parse_program(
        r#"
fn classify(x: i64) {
    if x > 0 { if x > 10 { return 2; } else { return 1; } }
    else { if x == 0 { return 0; } else { return -1; } }
}
fn wrapper(x: i64) { return classify(x); }
fn maybe(x: i64) { if x > 0 { return 1; } }
fn unrelated() { return 99; }
"#,
    )
    .unwrap();
    let chunk = compile(&program).unwrap();
    super::task_admission::validate(&chunk).unwrap();
    let mut host = LocalHost::default();
    let mut vm = Vm::new(&chunk, &mut host, Budget::default());
    vm.run().unwrap();
    let wrapper = chunk.find_function("wrapper").unwrap();
    for (input, expected) in [(20, 2), (5, 1), (0, 0), (-5, -1)] {
        assert_eq!(
            vm.call_function(wrapper, &[Value::I64(input)]).unwrap(),
            Value::I64(expected)
        );
    }
    assert_eq!(
        vm.call_function(chunk.find_function("maybe").unwrap(), &[Value::I64(0)])
            .unwrap(),
        Value::Null
    );
}
