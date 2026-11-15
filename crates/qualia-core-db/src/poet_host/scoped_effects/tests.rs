//! Correct pinned-0.1 effect contracts over the real native state owner.
use super::super::PoetSnapshot;
use super::ScopedEffects;
use vibe::{DiagCode, Diagnostic, Env, Host, Value};

fn function(host: &mut impl Host, source: &str, name: &str) -> Result<Value, Diagnostic> {
    let program = vibe::load_program(source)?;
    vibe::eval_function(&program, name, vec![], host, &mut Env::default())
}
fn hook(
    host: &mut impl Host,
    source: &str,
    path: &[&str],
    args: Vec<Value>,
) -> Result<Value, Diagnostic> {
    let program = vibe::load_program(source)?;
    vibe::dispatch_hook(
        &program,
        &path.iter().map(|part| (*part).into()).collect::<Vec<_>>(),
        args,
        host,
        &mut Env::default(),
    )
}
const POLL: &str = r#"
import "hid" as input;
requires [capability("hid.poll")];
effect fn poll_now() { return input.poll(); }
"#;
const EMIT: &str = r#"
requires [capability("pulse.publish")];
effect fn emit() { effect pulse.publish("poet/tick",42); return null; }
on pulse:message(topic:string) { return emit(); }
"#;

#[test]
fn dispatch_tick_hook_fires() {
    let mut snapshot = PoetSnapshot::default();
    let mut host = ScopedEffects::new(&mut snapshot, &[]).unwrap();
    let result = hook(
        &mut host,
        "hot fn step(x:i64) { return x + 1; } on tick() { return step(41); }",
        &["tick"],
        vec![],
    )
    .unwrap();
    assert_eq!(result, Value::I64(42));
    assert!(
        snapshot.published.is_empty(),
        "Hot tick must not manufacture external events"
    );
}

#[test]
fn g_tick_hook_with_pulse_publish_through_snapshot() {
    // This legacy case previously depended on an indirect-effect bypass. Hot
    // ticks cannot publish. Keep the denial and prove external event publishing
    // separately below instead of weakening the canonical transitive checker.
    let mut snapshot = PoetSnapshot::default();
    let mut host = ScopedEffects::new(&mut snapshot, &["pulse.publish"]).unwrap();
    let source = EMIT.replace("on pulse:message(topic:string)", "on tick()");
    assert_eq!(
        hook(&mut host, &source, &["tick"], vec![])
            .unwrap_err()
            .code,
        DiagCode::E200
    );
    assert!(snapshot.published.is_empty());
}

#[test]
fn external_hook_publishes_with_declared_and_actual_native_permission() {
    let mut snapshot = PoetSnapshot::default();
    let mut host = ScopedEffects::new(&mut snapshot, &["pulse.publish"]).unwrap();
    assert_eq!(
        hook(
            &mut host,
            EMIT,
            &["pulse", "message"],
            vec![Value::String("poet/input".into())]
        )
        .unwrap(),
        Value::Null
    );
    assert_eq!(snapshot.published.len(), 1);
    assert_eq!(snapshot.published[0].topic, "poet/tick");
    assert_eq!(snapshot.published[0].payload_summary, "42");
}

#[test]
fn external_hook_requires_source_request_host_grant_and_allowed_topic() {
    for (source, grants, code) in [
        (EMIT.to_string(), vec![], DiagCode::E300),
        (
            EMIT.replace("requires [capability(\"pulse.publish\")];", ""),
            vec!["pulse.publish"],
            DiagCode::E300,
        ),
        (
            EMIT.replace("poet/tick", "outside/topic"),
            vec!["pulse.publish"],
            DiagCode::E500,
        ),
    ] {
        let mut snapshot = PoetSnapshot::default();
        let mut host = ScopedEffects::new(&mut snapshot, &grants).unwrap();
        assert_eq!(
            hook(
                &mut host,
                &source,
                &["pulse", "message"],
                vec![Value::String("poet/input".into())]
            )
            .unwrap_err()
            .code,
            code
        );
        assert!(snapshot.published.is_empty());
    }
}

#[test]
fn t42_hid_vibescript_poll() {
    let mut snapshot = PoetSnapshot::default();
    snapshot
        .enqueue_hid_event(&vibe::hid::InboundEvent::pointer_move(
            100, "mouse:0", 5.0, 10.0,
        ))
        .unwrap();
    let mut host = ScopedEffects::new(&mut snapshot, &["hid.poll"]).unwrap();
    let Value::Record(record) = function(&mut host, POLL, "poll_now").unwrap() else {
        panic!("actual HID record")
    };
    assert_eq!(record.get("timestamp_ns"), Some(&Value::U64(100)));
    assert_eq!(snapshot.hid_event_count(), 0);
}

#[test]
fn t42_hid_vibescript_poll_empty() {
    let mut snapshot = PoetSnapshot::default();
    let mut host = ScopedEffects::new(&mut snapshot, &["hid.poll"]).unwrap();
    assert_eq!(function(&mut host, POLL, "poll_now").unwrap(), Value::Null);
    assert_eq!(snapshot.hid_event_count(), 0);
}

#[test]
fn pure_cell_and_ungranted_hid_do_not_consume_queued_input() {
    let mut snapshot = PoetSnapshot::default();
    snapshot
        .enqueue_hid_event(&vibe::hid::InboundEvent::pointer_move(
            101, "mouse:0", 1.0, 2.0,
        ))
        .unwrap();
    {
        let mut host = ScopedEffects::new(&mut snapshot, &["hid.poll"]).unwrap();
        assert_eq!(
            vibe::eval_cell("= hid.poll()", &mut host, &mut Env::default())
                .unwrap_err()
                .code,
            DiagCode::E200
        );
    }
    assert_eq!(snapshot.hid_event_count(), 1);
    {
        let mut host = ScopedEffects::new(&mut snapshot, &[]).unwrap();
        assert_eq!(
            function(&mut host, POLL, "poll_now").unwrap_err().code,
            DiagCode::E300
        );
    }
    assert_eq!(snapshot.hid_event_count(), 1);
}

#[test]
fn source_requires_cannot_replace_session_or_host_grants() {
    use vibe::session::{SessionLimits, SourceSession};
    let mut snapshot = PoetSnapshot::default();
    snapshot
        .enqueue_hid_event(&vibe::hid::InboundEvent::pointer_move(
            102, "mouse:0", 1.0, 2.0,
        ))
        .unwrap();
    let source = format!("{POLL}\npoll_now();");
    let mut no_session_grant = SourceSession::new(vec![], SessionLimits::default());
    let mut host = ScopedEffects::new(&mut snapshot, &["hid.poll"]).unwrap();
    assert_eq!(
        no_session_grant
            .submit(&source, &mut host, vibe::Budget::default())
            .unwrap_err()
            .diagnostic
            .code,
        DiagCode::E300
    );
    assert_eq!(snapshot.hid_event_count(), 1);
    let mut session = SourceSession::new(vec!["hid.poll".into()], SessionLimits::default());
    {
        let mut denied_host = ScopedEffects::new(&mut snapshot, &[]).unwrap();
        assert_eq!(
            session
                .submit(&source, &mut denied_host, vibe::Budget::default())
                .unwrap_err()
                .diagnostic
                .code,
            DiagCode::E300
        );
    }
    assert_eq!(snapshot.hid_event_count(), 1);
    let mut host = ScopedEffects::new(&mut snapshot, &["hid.poll"]).unwrap();
    let result = session
        .submit(&source, &mut host, vibe::Budget::default())
        .unwrap();
    assert!(matches!(result.value, Value::Record(_)));
    assert_eq!(snapshot.hid_event_count(), 0);
}

#[test]
fn crypto_sign_fail_closed() {
    let mut snapshot = PoetSnapshot::default();
    let mut host = ScopedEffects::new(&mut snapshot, &["crypto.sign"]).unwrap();
    let source = r#"import "crypto" as crypto; requires [capability("crypto.sign")]; effect fn sign_it(){return crypto.sign("key:ed25519:0","data");}"#;
    assert_eq!(
        function(&mut host, source, "sign_it").unwrap_err().code,
        DiagCode::E702,
        "the real native missing-key denial must be reached, not an earlier Pure-effect failure"
    );
}

#[test]
fn test_crypto_ed25519_generate_sign_verify_roundtrip() {
    let mut snapshot = PoetSnapshot::default();
    let generate = r#"import "crypto" as crypto; requires [capability("crypto.generate_key")]; effect fn make_key(){return crypto.generate_key("Ed25519");}"#;
    {
        let mut denied = ScopedEffects::new(&mut snapshot, &[]).unwrap();
        assert_eq!(
            function(&mut denied, generate, "make_key")
                .unwrap_err()
                .code,
            DiagCode::E300
        );
    }
    let mut host =
        ScopedEffects::new(&mut snapshot, &["crypto.generate_key", "crypto.sign"]).unwrap();
    assert_eq!(
        function(&mut host, &generate.replace("effect fn", "fn"), "make_key")
            .unwrap_err()
            .code,
        DiagCode::E200
    );
    let Value::Record(record) = function(&mut host, generate, "make_key").unwrap() else {
        panic!("actual generated key record")
    };
    let Some(Value::String(key_id)) = record.get("key_id") else {
        panic!("actual opaque key handle")
    };
    assert_eq!(
        key_id, "key:ed25519:0",
        "denied attempts must not generate a native key"
    );
    let sign = format!(
        r#"import "crypto" as crypto; requires [capability("crypto.sign")]; effect fn sign_doc(){{return crypto.sign("{key_id}","authentic contract payload");}}"#
    );
    let Value::String(signature) = function(&mut host, &sign, "sign_doc").unwrap() else {
        panic!("actual signature")
    };
    let verify = format!(
        r#"import "crypto" as crypto; fn verify_doc(){{return crypto.verify("{key_id}","authentic contract payload","{signature}");}}"#
    );
    assert_eq!(
        function(&mut host, &verify, "verify_doc").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        function(
            &mut host,
            &verify.replace("authentic contract payload", "tampered payload"),
            "verify_doc"
        )
        .unwrap(),
        Value::Bool(false)
    );
    drop(host);
    let mut revoked = ScopedEffects::new(&mut snapshot, &[]).unwrap();
    assert_eq!(
        function(&mut revoked, &sign, "sign_doc").unwrap_err().code,
        DiagCode::E300
    );
}

#[test]
fn native_grants_are_bounded_explicit_and_do_not_admit_other_host_domains() {
    let mut snapshot = PoetSnapshot::default();
    assert!(ScopedEffects::new(&mut snapshot, &["*"]).is_err());
    assert!(ScopedEffects::new(&mut snapshot, &["hid.wait"]).is_err());
    assert!(ScopedEffects::new(&mut snapshot, &["hid.poll", "hid.poll"]).is_err());
    assert!(ScopedEffects::new(&mut snapshot, &["hid.poll"; 33]).is_err());
    let mut host = ScopedEffects::new(&mut snapshot, &["hid.poll"]).unwrap();
    assert_eq!(
        host.graph_query(&[], 1, vibe::Span::point(0))
            .unwrap_err()
            .code,
        DiagCode::E300
    );
}
