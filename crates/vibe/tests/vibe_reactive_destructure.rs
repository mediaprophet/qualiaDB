//! Test suite for VibeScript 0.1 features:
//! 1. Reactive Cells & ReactiveCellGraph DAG
//! 2. Pattern Destructuring (Record, List, Constructor)
//! 3. Bounded String Interpolation (`f"..."`)
//! 4. CBOR-LD Serialization & Deserialization
//! 5. Decompiler & Projectional Roundtrip

use vibe::decompiler::{decompile_program, DecompileOptions};
use vibe::projectional::{project_program, ProjectOptions};
use vibe::reactive_cell::{ReactiveCellError, ReactiveCellGraph};
use vibe::{decode, encode, parse_program, Budget, Engine, Env, LocalHost, Value};

#[test]
fn test_reactive_cell_dag_topological_execution() {
    let src = r#"
        cell a := 10;
        cell b := a * 2;
        cell c := b + 5;
    "#;
    let prog = parse_program(src).expect("parse failed");
    let mut graph = ReactiveCellGraph::from_program(&prog).expect("graph compilation failed");

    assert_eq!(graph.cells.len(), 3);
    // Topological order must evaluate `a` before `b` before `c`
    let a_idx = graph.name_to_index["a"];
    let b_idx = graph.name_to_index["b"];
    let c_idx = graph.name_to_index["c"];

    let a_pos = graph.topo_order.iter().position(|&x| x == a_idx).unwrap();
    let b_pos = graph.topo_order.iter().position(|&x| x == b_idx).unwrap();
    let c_pos = graph.topo_order.iter().position(|&x| x == c_idx).unwrap();

    assert!(a_pos < b_pos);
    assert!(b_pos < c_pos);

    let mut env = Env::default();
    let updates = graph.step(&mut env, 0.0).expect("step failed");

    assert_eq!(updates.len(), 3);
    assert_eq!(graph.get_value("a"), Some(&Value::I64(10)));
    assert_eq!(graph.get_value("b"), Some(&Value::I64(20)));
    assert_eq!(graph.get_value("c"), Some(&Value::I64(25)));

    // Update input 'a' and check that downstream cells recalculate
    graph.set_input("a", Value::I64(50), &mut env);
    assert!(graph.cells[b_idx].is_dirty);
    assert!(graph.cells[c_idx].is_dirty);

    let _updates2 = graph.step(&mut env, 0.0).expect("step 2 failed");
    assert_eq!(graph.get_value("b"), Some(&Value::I64(100)));
    assert_eq!(graph.get_value("c"), Some(&Value::I64(105)));
}

#[test]
fn test_reactive_cell_cycle_detection() {
    let src = r#"
        cell x := y + 1;
        cell y := x + 1;
    "#;
    let prog = parse_program(src).expect("parse failed");
    let result = ReactiveCellGraph::from_program(&prog);

    match result {
        Err(ReactiveCellError::CycleDetected { cycle }) => {
            assert!(!cycle.is_empty());
        }
        other => panic!("expected CycleDetected error, got {:?}", other),
    }
}

#[test]
fn test_reactive_cell_temporal_and_when_condition() {
    let src = r#"
        cell active := true;
        cell counter when active := 42;
    "#;
    let prog = parse_program(src).expect("parse failed");
    let mut graph = ReactiveCellGraph::from_program(&prog).expect("graph compilation failed");

    let mut env = Env::default();
    graph.step(&mut env, 0.1).expect("step 1 failed");
    assert_eq!(graph.get_value("counter"), Some(&Value::I64(42)));

    // When active becomes false, counter shouldn't recompute
    graph.set_input("active", Value::Bool(false), &mut env);
    graph.set_input("counter", Value::I64(999), &mut env);
    graph.step(&mut env, 0.1).expect("step 2 failed");
    assert_eq!(graph.get_value("counter"), Some(&Value::I64(999)));
}

#[test]
fn test_destructuring_let_record() {
    let src = r#"
        fn main() -> i64 {
            let pt = { x: 10, y: 25 };
            let { x, y } = pt;
            return x + y;
        }
    "#;
    let prog = parse_program(src).expect("parse failed");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();

    let res = engine
        .call_function(&prog, "main", vec![], &mut env)
        .expect("eval failed");
    assert_eq!(res, Value::I64(35));
}

#[test]
fn test_destructuring_let_list() {
    let src = r#"
        fn main() -> i64 {
            let list = [100, 200, 300];
            let [a, b, c] = list;
            return a + b + c;
        }
    "#;
    let prog = parse_program(src).expect("parse failed");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();

    let res = engine
        .call_function(&prog, "main", vec![], &mut env)
        .expect("eval failed");
    assert_eq!(res, Value::I64(600));
}

#[test]
fn test_destructuring_let_constructor() {
    let src = r#"
        fn main() -> f64 {
            let v = vec3(1.0, 2.5, 3.5);
            let vec3(x, y, z) = v;
            return x + y + z;
        }
    "#;
    let prog = parse_program(src).expect("parse failed");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();

    let res = engine
        .call_function(&prog, "main", vec![], &mut env)
        .expect("eval failed");
    assert_eq!(res, Value::F64(7.0));
}

#[test]
fn test_string_interpolation_evaluation() {
    let src = r#"
        fn main() -> String {
            let name = "Qualia";
            let version = 1;
            return f"Welcome to {name} v{version}!";
        }
    "#;
    let prog = parse_program(src).expect("parse failed");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();

    let res = engine
        .call_function(&prog, "main", vec![], &mut env)
        .expect("eval failed");
    assert_eq!(res, Value::String("Welcome to Qualia v1!".to_string()));
}

#[test]
fn test_cbor_cell() {
    let src = "cell energy := 100;";
    let prog = parse_program(src).expect("parse failed");
    let cbor = encode(&prog);
    let dec = decode(&cbor).expect("decode cell failed");
    assert_eq!(dec.items.len(), 1);
}

#[test]
fn test_cbor_destructure_record() {
    let src = "let { x, y } = pt;";
    let prog = parse_program(src).expect("parse failed");
    let cbor = encode(&prog);
    let dec = decode(&cbor).expect("decode record failed");
    assert_eq!(dec.items.len(), 1);
}

#[test]
fn test_cbor_destructure_list() {
    let src = "let [a, b] = [1, 2];";
    let prog = parse_program(src).expect("parse failed");
    let cbor = encode(&prog);
    let dec = decode(&cbor).expect("decode list failed");
    assert_eq!(dec.items.len(), 1);
}

#[test]
fn test_cbor_destructure_constructor() {
    let src = "let vec2(vx, vy) = vec2(3.0, 4.0);";
    let prog = parse_program(src).expect("parse failed");
    let cbor = encode(&prog);
    let dec = decode(&cbor).expect("decode constructor failed");
    assert_eq!(dec.items.len(), 1);
}

#[test]
fn test_cbor_interpolate() {
    let src = "return f\"Point: {x}\";";
    let prog = parse_program(src).expect("parse failed");
    let cbor = encode(&prog);
    let dec = decode(&cbor).expect("decode interpolate failed");
    assert_eq!(dec.items.len(), 1);
}

#[test]
fn test_cbor_composite_roundtrip() {
    let src = r#"
        cell energy := 100;
        fn compute(pt: Record) -> String {
            let { x, y } = pt;
            let [a, b] = [1, 2];
            let vec2(vx, vy) = vec2(3.0, 4.0);
            return f"Point: ({x}, {y}), vel: ({vx}, {vy})";
        }
    "#;
    let prog = parse_program(src).expect("parse failed");
    let cbor = encode(&prog);
    let dec = decode(&cbor).expect("decode composite failed");
    assert_eq!(dec.items.len(), 2);
}

#[test]
fn test_decompiler_and_projectional_roundtrip() {
    let src = r#"
cell position := 42;
fn render() -> String {
  let { x, y } = { x: 1, y: 2 };
  let [a, b] = [3, 4];
  return f"pos: {position}, x: {x}";
}
"#;
    let prog = parse_program(src).expect("parse failed");

    // Test Decompiler
    let decompiled = decompile_program(&prog, &DecompileOptions::default());
    assert!(decompiled.contains("cell position := 42;"));
    assert!(decompiled.contains("let { x, y }"));
    assert!(decompiled.contains("let [a, b]"));
    assert!(decompiled.contains("f\"pos: {position}, x: {x}\""));

    // Re-parse decompiled output
    let re_parsed = parse_program(&decompiled).expect("re-parse decompiled failed");
    assert_eq!(re_parsed.items.len(), prog.items.len());

    // Test Projectional authoring
    let projected = project_program(
        &prog,
        &ProjectOptions {
            indent: "  ".to_string(),
            blank_lines_between_decls: 1,
            max_line_width: 80,
        },
    );
    assert!(projected.contains("cell position := 42;"));
    assert!(projected.contains("let { x, y }"));
    assert!(projected.contains("let [a, b]"));

    let re_parsed_proj = parse_program(&projected).expect("re-parse projected failed");
    assert_eq!(re_parsed_proj.items.len(), prog.items.len());
}
