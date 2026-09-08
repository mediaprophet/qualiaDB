//! Lock Pages playground/showcase samples to the current engine.
//! Mirrors `vibe-wasm::eval_program_src`: preamble then `main`.

use vibe::{check_program, eval_function, parse_program, Budget, Engine, Env, LocalHost, Value};

fn eval_pages_src(src: &str) -> Result<Value, String> {
    let prog = parse_program(src).map_err(|e| e.message)?;
    check_program(&prog).map_err(|e| e.message)?;
    let mut host = LocalHost::default();
    let mut env = Env::default();
    {
        let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
        engine.eval_program(&prog, &mut env).map_err(|e| e.message)?;
    }
    eval_function(&prog, "main", Vec::new(), &mut host, &mut env).map_err(|e| e.message)
}

#[test]
fn fib_let_mut_runs() {
    let src = r#"
fn fib_iter(n: i64) budget(steps: 100000) -> i64 {
  let mut a = 0;
  let mut b = 1;
  let mut i = 0;
  while i < n {
    let t = a + b;
    a = b;
    b = t;
    i = i + 1;
  }
  return a;
}

fn main() -> i64 {
  return fib_iter(10);
}
"#;
    match eval_pages_src(src).expect("fib") {
        Value::I64(55) => {}
        other => panic!("expected 55, got {other}"),
    }
}

#[test]
fn for_loop_let_mut_runs() {
    let src = r#"
fn sum_list() budget(steps: 10000) -> i64 {
  let mut s = 0;
  for x in [10, 20, 30, 40, 50] {
    s = s + x;
  }
  return s;
}

fn main() -> i64 {
  return sum_list();
}
"#;
    match eval_pages_src(src).expect("for") {
        Value::I64(150) => {}
        other => panic!("expected 150, got {other}"),
    }
}

#[test]
fn while_let_mut_runs() {
    let src = r#"
fn sum_to(n: i64) budget(steps: 10000) -> i64 {
  let mut s = 0;
  let mut i = 0;
  while i < n {
    s = s + i;
    i = i + 1;
  }
  return s;
}

fn main() -> i64 {
  return sum_to(100);
}
"#;
    match eval_pages_src(src).expect("while") {
        Value::I64(4950) => {}
        other => panic!("expected 4950, got {other}"),
    }
}

#[test]
fn lamp_color_and_pose_record() {
    let src = r#"
using Animation;

fn main() -> Record {
  let glow = oklch(0.72, 0.12, 230.0);
  let pose = Animation.orbit_spin(1.0);
  return { color: glow, pose: pose };
}
"#;
    match eval_pages_src(src).expect("lamp") {
        Value::Record(map) => {
            assert!(map.contains_key("color"), "keys: {map:?}");
            assert!(map.contains_key("pose"), "keys: {map:?}");
        }
        other => panic!("expected lamp record, got {other}"),
    }
}

#[test]
fn zh_locale_main_returns_one() {
    let src = "locale zh;\n函数 main() -> i64 {\n  返回 1;\n}\n";
    match eval_pages_src(src).expect("zh") {
        Value::I64(1) => {}
        other => panic!("expected 1, got {other}"),
    }
}

#[test]
fn cosmic_geodetic_to_ecef_runs() {
    let src = r#"
using Cosmic;

effect fn main() -> Record {
  return Cosmic.geodetic_to_ecef({
    lat_deg: -37.8,
    lon_deg: 144.9,
    alt_m: 0.0
  });
}
"#;
    match eval_pages_src(src).expect("cosmic") {
        Value::Record(map) => {
            assert!(map.contains_key("x") || map.contains_key("x_m"), "ecef keys: {map:?}");
        }
        other => panic!("expected ECEF record, got {other}"),
    }
}
