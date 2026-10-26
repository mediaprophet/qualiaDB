//! Conformance and unit tests for VibeScript v0.02 Web, CBOR-LD, Form, UI, Input, Validation, and Layout libraries.

use crate::{eval_cell, eval_function, load_program, Env, LocalHost, Value};

#[test]
fn vibescript_web_and_ui_bindings_replace_javascript_handlers() {
    let mut host = LocalHost::default();
    let mut env = Env::default();

    // 1. Test `web.response` enforces zero-JS Content-Security-Policy
    let resp = eval_cell(
        "= web.response(200, \"text/html; charset=utf-8\", \"<main>Hello Vibe</main>\")",
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(rec) = resp else {
        panic!("expected Record");
    };
    assert_eq!(rec.get("status"), Some(&Value::I64(200)));
    assert_eq!(rec.get("zero_js"), Some(&Value::Bool(true)));

    // 2. Test `web.range_response` (HTTP 206 Partial Content for 40 KiB .q42 SuperBlock)
    let range_resp = eval_cell(
        "= web.range_response(9830400, 4096, 45055, \"0x89a1f02c\")",
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(rrec) = range_resp else {
        panic!("expected Record");
    };
    assert_eq!(rrec.get("status"), Some(&Value::I64(206)));
    assert_eq!(
        rrec.get("content_range"),
        Some(&Value::String("bytes 4096-45055/9830400".into()))
    );
    assert_eq!(rrec.get("content_length"), Some(&Value::I64(40960)));

    // 3. Test `web.route_mail` (`bob@alice.tld` relationship model)
    let mail_ok = eval_cell(
        "= web.route_mail(\"bob@alice.tld\", \"alice@bob.tld\", \"alice@bob.tld\")",
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(m_ok) = mail_ok else {
        panic!("expected Record");
    };
    assert_eq!(m_ok.get("admitted"), Some(&Value::Bool(true)));
    assert_eq!(
        m_ok.get("verdict"),
        Some(&Value::String("DeliverToRelationshipThread".into()))
    );

    let mail_spam = eval_cell(
        "= web.route_mail(\"bob@alice.tld\", \"spam@broker.com\", \"alice@bob.tld\")",
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(m_spam) = mail_spam else {
        panic!("expected Record");
    };
    assert_eq!(m_spam.get("admitted"), Some(&Value::Bool(false)));
    assert_eq!(
        m_spam.get("pinpointed_leak_source"),
        Some(&Value::String("bob".into()))
    );

    // 4. Test a full VibeScript module with `web.check_permit`, `ui.select_persona`, and `ui.patch`
    let src = r#"
module webizen_cpanel_controller;

pure fn handle_persona_switch(persona: str, page: str) budget(steps: 200) {
    let state = ui.select_persona(persona, page);
    let patch = ui.patch("persona-badge", "textContent", state.badge);
    return { state: state, patch: patch };
}
"#;
    let prog = load_program(src).unwrap();
    let out = eval_function(
        &prog,
        "handle_persona_switch",
        vec![Value::String("geek".into()), Value::String("domains".into())],
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(out_rec) = out else {
        panic!("expected Record");
    };
    assert!(out_rec.contains_key("state"));
    assert!(out_rec.contains_key("patch"));
}

#[test]
fn vibescript_v0_02_cbor_ld_provider_json_uplift_and_full_form_suite() {
    let mut host = LocalHost::default();
    let mut env = Env::default();

    // 1. Test Provider/TLD-scoped JSON ingestion into native CBOR-LD
    let uplifted = eval_cell(
        r#"= cbor_ld.ingest_json("{\"systolic\":120,\"status\":\"active\"}", "clinic.example.org.au")"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(up_rec) = uplifted else {
        panic!("expected Record");
    };
    assert_eq!(
        up_rec.get("provider_tld"),
        Some(&Value::String("au".into()))
    );
    assert_eq!(
        up_rec.get("context"),
        Some(&Value::String("https://clinic.example.org.au/ns#".into()))
    );

    // 2. Test full VibeScript v0.02 form lifecycle, masking, validation, undo/redo, CRDT, and CBOR-LD response
    let src = r#"
module webizen_clinical_v0_02_intake;

pure fn run_intake_pipeline(raw_card: str, raw_price: str) budget(steps: 500) {
    let f0 = form.create("intake-1", "https://qualia.world/ontology/clinical#", {
        patient_name: "Alice",
        systolic: 118
    });
    let f1 = form.set_value(f0, "systolic", 124);
    let dirty_after_edit = form.is_dirty(f1);
    let f_undone = form.undo(f1);
    let f_redone = form.redo(f_undone);

    let masked = input.apply_mask(raw_card, "9999 9999 9999 9999");
    let price = input.parse_currency(raw_price, "AUD", 2);
    let slider = ui.range_interval("bp-bracket", 60.0, 200.0, 90.0, 130.0, 10.0, "linear");
    let vslice = layout.virtual_slice(50000, 32.0, 640.0, 320.0, 2);

    let field_check = validate.check_field("alice@example.org", {
        required: true,
        kind: "email",
        min_length: 5
    });

    let cbor_wire = form.to_cbor_ld(f_redone, "https://qualia.world/context/clinical/v1");
    let sig = form.sign_submission(f_redone, "https://qualia.world/context/clinical/v1", "did:q42:alice");

    return {
        dirty: dirty_after_edit,
        systolic_final: form.get_value(f_redone, "systolic"),
        masked_card: masked.formatted,
        minor_units: price.minor_units,
        slider_span: slider.span,
        vslice_start: vslice.start_idx,
        email_valid: field_check.valid,
        cbor_tag: cbor_wire.tag,
        deterministic_cde: sig.deterministic_cde
    };
}
"#;
    let prog = load_program(src).unwrap();
    let res = eval_function(
        &prog,
        "run_intake_pipeline",
        vec![
            Value::String("4111222233334444".into()),
            Value::String("$1,250.75".into()),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(r) = res else {
        panic!("expected Record");
    };
    assert_eq!(r.get("dirty"), Some(&Value::Bool(true)));
    assert_eq!(r.get("systolic_final"), Some(&Value::I64(124)));
    assert_eq!(
        r.get("masked_card"),
        Some(&Value::String("4111 2222 3333 4444".into()))
    );
    assert_eq!(r.get("minor_units"), Some(&Value::I64(125075)));
    assert_eq!(r.get("slider_span"), Some(&Value::F64(40.0)));
    assert_eq!(r.get("vslice_start"), Some(&Value::I64(18)));
    assert_eq!(r.get("email_valid"), Some(&Value::Bool(true)));
    assert_eq!(r.get("cbor_tag"), Some(&Value::I64(0x0501)));
    assert_eq!(r.get("deterministic_cde"), Some(&Value::Bool(true)));
}
