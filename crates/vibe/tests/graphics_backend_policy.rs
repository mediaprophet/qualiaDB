use vibe::{eval_function, load_program, Env, LocalHost, Value};

const POLICY: &str = include_str!("../../qualia-core-db/src/render/portal/graphics_backend.vibe");

fn select(webgpu: bool, webgl2: bool, canvas2d: bool) -> &'static str {
    let program = load_program(POLICY).expect("graphics backend VibeScript validates");
    let mut host = LocalHost::default();
    let mut env = Env::default();
    let value = eval_function(
        &program,
        "select_backend",
        vec![
            Value::Bool(webgpu),
            Value::Bool(webgl2),
            Value::Bool(canvas2d),
        ],
        &mut host,
        &mut env,
    )
    .expect("graphics backend policy evaluates");
    match value {
        Value::String(ref name) if name == "webgpu" => "webgpu",
        Value::String(ref name) if name == "webgl2" => "webgl2",
        Value::String(ref name) if name == "canvas2d" => "canvas2d",
        Value::String(ref name) if name == "unavailable" => "unavailable",
        other => panic!("unexpected graphics backend: {other:?}"),
    }
}

fn select_recovery(webgpu_retry_allowed: bool, webgl2_allowed: bool) -> &'static str {
    let program = load_program(POLICY).expect("graphics backend VibeScript validates");
    let mut host = LocalHost::default();
    let mut env = Env::default();
    let value = eval_function(
        &program,
        "select_recovery_backend",
        vec![
            Value::Bool(webgpu_retry_allowed),
            Value::Bool(webgl2_allowed),
        ],
        &mut host,
        &mut env,
    )
    .expect("graphics recovery policy evaluates");
    match value {
        Value::String(ref name) if name == "webgpu" => "webgpu",
        Value::String(ref name) if name == "webgl2" => "webgl2",
        Value::String(ref name) if name == "canvas2d" => "canvas2d",
        other => panic!("unexpected recovery backend: {other:?}"),
    }
}

#[test]
fn graphics_policy_prefers_quality_then_degrades() {
    assert_eq!(select(true, true, true), "webgpu");
    assert_eq!(select(false, true, true), "webgl2");
    assert_eq!(select(false, false, true), "canvas2d");
    assert_eq!(select(false, false, false), "unavailable");
}

#[test]
fn webgpu_preference_does_not_hide_available_fallbacks() {
    assert_eq!(select(true, false, true), "webgpu");
    assert_eq!(select(false, true, false), "webgl2");
}

#[test]
fn recovery_policy_retries_webgpu_then_degrades() {
    assert_eq!(select_recovery(true, true), "webgpu");
    assert_eq!(select_recovery(false, true), "webgl2");
    assert_eq!(select_recovery(false, false), "canvas2d");
}
