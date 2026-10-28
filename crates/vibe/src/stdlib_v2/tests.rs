//! Selected standard-library and scoped runtime integration regressions.
//! Historical 0.02 library planning is not a released-dialect or full-conformance claim.

use crate::{eval_cell, eval_function, load_program, Env, LocalHost, Value};

/// Explicit trusted synthetic Host/Env setup through the low-level Engine API.
/// This bypasses Pure-cell admission for runtime tests, not native human authority.
fn eval_admitted(
    source: &str,
    host: &mut LocalHost,
    env: &mut Env,
) -> Result<Value, crate::Diagnostic> {
    let expr = crate::parse_cell(source)?;
    let mut engine = crate::Engine::new(host, crate::Budget::default());
    engine.eval_expr(&expr, env)
}

#[test]
fn track1_reflection_records_uri_and_errors() {
    let mut host = LocalHost::default();
    let mut env = Env::default();

    // 1. `reflect.type_of(null)` returns `"Option::None"` (never `"object"`)
    let t_null = eval_cell("= reflect.type_of(null)", &mut host, &mut env).unwrap();
    let Value::Record(r_null) = t_null else {
        panic!("expected Record");
    };
    assert_eq!(
        r_null.get("kind"),
        Some(&Value::String("Option::None".into()))
    );
    assert_eq!(r_null.get("is_null"), Some(&Value::Bool(true)));

    // 2. Null-prototype `record.from_entries` strips `__proto__` and `constructor`
    let rec = eval_cell(
        r#"= record.from_entries([["__proto__", "bad"], ["constructor", "bad"], ["host", "qualia.world"]])"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(r_map) = rec else {
        panic!("expected Record");
    };
    assert!(!r_map.contains_key("__proto__"));
    assert!(!r_map.contains_key("constructor"));
    assert_eq!(
        r_map.get("host"),
        Some(&Value::String("qualia.world".into()))
    );

    // 3. `uri.parse` + `uri.join` + `error.create`
    let uri = eval_cell(
        r#"= uri.parse("https://api.qualia.org.au:8443/v2/manifold?mode=strict#s1")"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(u_map) = uri else {
        panic!("expected Record");
    };
    assert_eq!(u_map.get("valid"), Some(&Value::Bool(true)));
    assert_eq!(u_map.get("tld"), Some(&Value::String("au".into())));
    assert_eq!(u_map.get("port"), Some(&Value::I64(8443)));
}

#[test]
fn track2_strict_num_exact_decimal_euclidean_math_and_autodiff() {
    let mut host = LocalHost::default();
    let mut env = Env::default();

    // 1. Strict parser rejects trailing garbage ("123abc") unlike JS parseInt
    let bad_int = eval_cell(r#"= num.parse_i64("123abc", 10)"#, &mut host, &mut env).unwrap();
    let Value::Record(bi) = bad_int else {
        panic!("expected Record");
    };
    assert_eq!(bi.get("ok"), Some(&Value::Bool(false)));

    let good_int = eval_cell(r#"= num.parse_i64("123", 10)"#, &mut host, &mut env).unwrap();
    let Value::Record(gi) = good_int else {
        panic!("expected Record");
    };
    assert_eq!(gi.get("ok"), Some(&Value::Bool(true)));
    assert_eq!(gi.get("value"), Some(&Value::I64(123)));

    // 2. Exact Decimal arithmetic: 0.1 + 0.2 == "0.3" and round(1.005, 2) == "1.01"
    let dec_sum = eval_cell(r#"= decimal.add("0.1", "0.2")"#, &mut host, &mut env).unwrap();
    let Value::Record(ds) = dec_sum else {
        panic!("expected Record");
    };
    assert_eq!(ds.get("text"), Some(&Value::String("0.3".into())));

    let fixed = eval_cell(r#"= decimal.to_fixed("1.005", 2)"#, &mut host, &mut env).unwrap();
    assert_eq!(fixed, Value::String("1.01".into()));

    // 3. True Euclidean modulo (`math.imod(-1, 5) == 4`) & floor division (`math.idiv(-7, 2) == -4`)
    assert_eq!(
        eval_cell("= math.imod(-1, 5)", &mut host, &mut env).unwrap(),
        Value::I64(4)
    );
    assert_eq!(
        eval_cell("= math.idiv(-7, 2)", &mut host, &mut env).unwrap(),
        Value::I64(-4)
    );

    // 4. Compensated summation (`math.sum_precise`)
    assert_eq!(
        eval_cell(
            "= math.sum_precise([10000000000000000.0, 1.0, -10000000000000000.0])",
            &mut host,
            &mut env
        )
        .unwrap(),
        Value::F64(1.0)
    );

    // 5. First-class forward-mode Automatic Differentiation: f(x) = 2 + 3x + 4x^2 at x = 5 => val = 117, grad = 43
    let vg = eval_cell(
        "= autodiff.value_and_grad([2.0, 3.0, 4.0], 5.0)",
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(vg_map) = vg else {
        panic!("expected Record");
    };
    assert_eq!(vg_map.get("value"), Some(&Value::F64(117.0)));
    assert_eq!(vg_map.get("grad"), Some(&Value::F64(43.0)));
}

#[test]
fn track3_grapheme_strings_linear_regex_collections_and_negative_indexing() {
    let mut host = LocalHost::default();
    let mut env = Env::default();

    // 1. UAX #29 Grapheme cluster length & negative indexing on composite emojis
    assert_eq!(
        eval_cell(r#"= str.grapheme_len("a🚀👨‍👩‍👧‍👦🇦🇺z")"#, &mut host, &mut env).unwrap(),
        Value::I64(5)
    );
    assert_eq!(
        eval_cell(r#"= "a🚀👨‍👩‍👧‍👦🇦🇺z"[-2]"#, &mut host, &mut env).unwrap(),
        Value::String("🇦🇺".into())
    );
    assert_eq!(
        eval_cell("= [10, 20, 30, 40][-1]", &mut host, &mut env).unwrap(),
        Value::I64(40)
    );

    // 2. Linear-time Thompson NFA regex (stateless & ReDoS-immune)
    assert_eq!(
        eval_cell(
            r#"= regex.is_match("^(a+)+b$", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaac")"#,
            &mut host,
            &mut env
        )
        .unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        eval_cell(
            r#"= regex.is_match("^did:q42:[a-z0-9_]+$", "did:q42:agent_01")"#,
            &mut host,
            &mut env
        )
        .unwrap(),
        Value::Bool(true)
    );

    // 3. Natural numeric sort (`[10, 2, 1]` -> `[1, 2, 10]`, never UTF-16 `[1, 10, 2]`)
    assert_eq!(
        eval_cell("= list.sort_natural([10, 2, 1])", &mut host, &mut env).unwrap(),
        Value::List(vec![Value::I64(1), Value::I64(2), Value::I64(10)])
    );
}

#[test]
fn track4_topology_sheaf_homology_ontological_manifold_and_systems() {
    let mut host = LocalHost::default();
    let mut env = Env::default();

    let src = r#"
module vibe_v0_02_full_verification;

effect async fn verify_v0_02_pipeline() budget(steps: 600) {
    // 1. Simplicial complex & Betti numbers (hollow triangle has beta_0 = 1, beta_1 = 1)
    let hollow_triangle = {
        vertices: ["v0", "v1", "v2"],
        edges: [["v0", "v1"], ["v1", "v2"], ["v0", "v2"]],
        faces: []
    };
    let betti = topology.betti_numbers(hollow_triangle);

    // 2. Sheaf stalk projection & gluing
    let global_state = { schema: "v2", policy: "strict", ui: "dark" };
    let stalk_a = sheaf.project_stalk(global_state, ["schema", "policy"], "did:q42:agent_a");
    let stalk_b = sheaf.project_stalk(global_state, ["policy", "ui"], "did:q42:agent_b");
    let glued = sheaf.apply_glue([stalk_a, stalk_b], {});

    // 3. Context/mount descriptors; these fields do not prove dataset residency.
    let fork = context.fork_by_ontology(global_state, "https://qualia.world/ontology/policy#", "audit");
    let mounted = manifold.mount("https://qualia.world/datasets/genome_10tb", { read: true });

    // 4. Content-addressed code, Perceus reuse analysis, and 64B SoA layout
    let code_hash = code.hash_ast("= 40 + 2;");
    let perceus = memory.perceus_plan({ x: 1, y: 2 }, 1);
    let soa = memory.layout_soa({ pos_x: "f64", pos_y: "f64", vel_x: "f64" }, { count: 16 });

    return {
        beta_0: betti.beta_0,
        beta_1: betti.beta_1,
        has_voids: betti.has_voids,
        glued_ok: glued.glued,
        mounted_cost: mounted.token_cost,
        ast_tag: code_hash.ast_cbor_tag,
        in_place_reuse: perceus.in_place_reuse,
        soa_align: soa.alignment_bytes
    };
}
"#;
    let prog = load_program(src).unwrap();
    let out = eval_function(&prog, "verify_v0_02_pipeline", vec![], &mut host, &mut env).unwrap();
    let Value::Record(r) = out else {
        panic!("expected Record");
    };
    assert_eq!(r.get("beta_0"), Some(&Value::I64(1)));
    assert_eq!(r.get("beta_1"), Some(&Value::I64(1)));
    assert_eq!(r.get("has_voids"), Some(&Value::Bool(true)));
    assert_eq!(r.get("glued_ok"), Some(&Value::Bool(true)));
    assert_eq!(r.get("mounted_cost"), Some(&Value::I64(1)));
    assert_eq!(r.get("ast_tag"), Some(&Value::I64(4200)));
    assert_eq!(r.get("in_place_reuse"), Some(&Value::Bool(true)));
    assert_eq!(r.get("soa_align"), Some(&Value::I64(64)));
}

#[test]
fn scoped_local_tasks_and_content_integrity_preserve_boundaries() {
    let mut host = LocalHost::default();
    let mut env = Env::default();

    let pure_task_refusal = eval_cell("= task.spawn(\"= 1 + 1\", {})", &mut host, &mut env)
        .expect_err("pure cell must reject a task-registry mutation");
    assert_eq!(pure_task_refusal.code, crate::DiagCode::E200);

    // 1. Register a pending local task and join it through the AST interpreter.
    let src_join = r#"
module test_scheduler_join;

effect async fn spawn_and_await() budget(steps: 200) {
    let job = task.spawn("= 40 + 2", {});
    let before = task.status(job);
    let joined = await job;
    let after = task.status(job);
    return {
        before_status: before.status,
        joined_status: joined.status,
        joined_result: joined.result,
        live_joined: joined.live_job_joined,
        qualification: joined.execution_qualification,
        after_status: after.status
    };
}

"#;
    let prog_join = load_program(src_join).unwrap();
    let out_join =
        eval_function(&prog_join, "spawn_and_await", vec![], &mut host, &mut env).unwrap();
    let Value::Record(rj) = out_join else {
        panic!("expected Record");
    };
    assert_eq!(
        rj.get("before_status"),
        Some(&Value::String("pending".into()))
    );
    assert_eq!(
        rj.get("joined_status"),
        Some(&Value::String("completed".into()))
    );
    assert_eq!(rj.get("joined_result"), Some(&Value::I64(42)));
    assert_eq!(rj.get("live_joined"), Some(&Value::Bool(true)));
    assert_eq!(
        rj.get("qualification"),
        Some(&Value::String("local_deferred_synchronous_ast_task".into()))
    );
    assert_eq!(
        rj.get("after_status"),
        Some(&Value::String("completed".into()))
    );

    // 2. Cancel a registered task through the task registry,
    //    and verify that `await` on the original unmutated `job` binding fails closed!
    let src_cancel = r#"
module test_scheduler_cancel;

effect async fn cancel_live_job() budget(steps: 200) {
    let job = code.spawn_at("did:q42:local_scheduler", "= 100 + 23", []);
    let cancelled_rec = task.cancel(job);
    let live_state = task.status(job);
    return {
        job_handle: job,
        cancelled_rec: cancelled_rec,
        live_state: live_state
    };
}

effect async fn await_cancelled_original_handle() budget(steps: 200) {
    let job = task.spawn("= 10 + 20", {});
    let _c = task.cancel(job);
    // Awaiting the original `job` record (whose in-memory map still had status="pending")
    // MUST fail closed because `await` contacts the live TaskRegistry!
    return await job;
}
"#;
    let prog_cancel = load_program(src_cancel).unwrap();
    let out_cancel =
        eval_function(&prog_cancel, "cancel_live_job", vec![], &mut host, &mut env).unwrap();
    let Value::Record(rc) = out_cancel else {
        panic!("expected Record");
    };
    let Some(Value::Record(cancelled_rec)) = rc.get("cancelled_rec") else {
        panic!("expected cancelled_rec");
    };
    assert_eq!(
        cancelled_rec.get("scheduler_contacted"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        cancelled_rec.get("live_job_cancelled"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        cancelled_rec.get("execution_qualification"),
        Some(&Value::String("local_deferred_synchronous_ast_task".into()))
    );
    let Some(Value::Record(live_state)) = rc.get("live_state") else {
        panic!("expected live_state");
    };
    assert_eq!(
        live_state.get("status"),
        Some(&Value::String("cancelled".into()))
    );
    assert_eq!(live_state.get("cancelled"), Some(&Value::Bool(true)));

    let err = eval_function(
        &prog_cancel,
        "await_cancelled_original_handle",
        vec![],
        &mut host,
        &mut env,
    )
    .expect_err("awaiting a task cancelled in live scheduler must fail closed");
    assert_eq!(err.code, crate::DiagCode::E400);

    // 3. Detached records are not task handles and cannot mutate scheduler state.
    let detached_err = eval_admitted(
        r#"= task.cancel({ status: "running" })"#,
        &mut host,
        &mut env,
    )
    .unwrap_err();
    assert_eq!(detached_err.code, crate::DiagCode::E400);
    assert!(detached_err.message.contains("task record has no task_id"));

    let strict_err = eval_admitted(
        r#"= task.cancel({ status: "running", require_live_job: true })"#,
        &mut host,
        &mut env,
    )
    .expect_err("require_live_job on detached record must fail closed");
    assert_eq!(strict_err.code, crate::DiagCode::E400);

    // 4. A genuine cancelled child causes the nursery to cancel a pending sibling.
    let src_nursery = r#"
module test_scheduler_nursery;

effect async fn nursery_cancels_live_sibling() budget(steps: 200) {
    let sibling = task.spawn("= 7 * 6", {});
    let trigger = task.spawn("= 8 * 8", {});
    let trigger_cancel = task.cancel(trigger);
    let res = task.nursery([trigger, sibling]);
    let sibling_after = task.status(sibling);
    let trigger_after = task.status(trigger);
    return {
        trigger_cancelled: trigger_cancel.status,
        trigger_status: trigger_after.status,
        all_succeeded: res.all_succeeded,
        cancelled_siblings: res.cancelled_siblings,
        cancellation_requested: res.cancellation_requested,
        live_siblings_cancelled: res.live_siblings_cancelled,
        cancellation_requested_count: res.cancellation_requested_count,
        sibling_status: sibling_after.status,
        sibling_cancelled: sibling_after.cancelled
    };
}
"#;
    let prog_nursery = load_program(src_nursery).unwrap();
    let out_nursery = eval_function(
        &prog_nursery,
        "nursery_cancels_live_sibling",
        vec![],
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(rn) = out_nursery else {
        panic!("expected Record");
    };
    assert_eq!(rn.get("live_siblings_cancelled"), Some(&Value::U64(1)));
    assert_eq!(rn.get("all_succeeded"), Some(&Value::Bool(false)));
    assert_eq!(rn.get("cancelled_siblings"), Some(&Value::Bool(true)));
    assert_eq!(rn.get("cancellation_requested"), Some(&Value::Bool(true)));
    assert_eq!(rn.get("cancellation_requested_count"), Some(&Value::U64(1)));
    assert_eq!(
        rn.get("trigger_cancelled"),
        Some(&Value::String("cancelled".into()))
    );
    assert_eq!(
        rn.get("trigger_status"),
        Some(&Value::String("cancelled".into()))
    );
    assert_eq!(
        rn.get("sibling_status"),
        Some(&Value::String("cancelled".into()))
    );
    assert_eq!(rn.get("sibling_cancelled"), Some(&Value::Bool(true)));

    // 5. VT-01: Truthful SHA-256 code addressing & resolution (distinct from 64-bit FNV-1a fast index)
    assert_eq!(
        crate::crypto::sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        crate::crypto::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let hashed = eval_admitted(r#"= code.hash_ast("= 21 * 2;")"#, &mut host, &mut env).unwrap();
    let Value::Record(hr) = hashed else {
        panic!("expected Record");
    };
    assert_eq!(hr.get("algorithm"), Some(&Value::String("SHA-256".into())));
    assert_eq!(
        hr.get("fast_index_algorithm"),
        Some(&Value::String("FNV-1a-64".into()))
    );
    let Some(Value::String(sha256_hex)) = hr.get("sha256_hex") else {
        panic!("expected sha256_hex");
    };
    assert_eq!(
        sha256_hex.len(),
        64,
        "SHA-256 hex digest must be 64 hex chars (256 bits)"
    );
    let Some(Value::String(hash_addr)) = hr.get("hash_hex") else {
        panic!("expected hash_hex");
    };
    assert_eq!(hash_addr, &format!("sha256:vibe:{sha256_hex}"));

    // Arbitrary nonempty string MUST NOT resolve
    let bad_resolve = eval_admitted(
        r#"= code.resolve("nonempty-random-string")"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(br) = bad_resolve else {
        panic!("expected Record");
    };
    assert_eq!(br.get("resolved"), Some(&Value::Bool(false)));
    assert_eq!(br.get("verified"), Some(&Value::Bool(false)));

    // Unavailable 64-hex SHA-256 MUST NOT resolve
    let missing_resolve = eval_admitted(
        r#"= code.resolve("sha256:vibe:0000000000000000000000000000000000000000000000000000000000000000")"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(mr) = missing_resolve else {
        panic!("expected Record");
    };
    assert_eq!(mr.get("resolved"), Some(&Value::Bool(false)));
    assert_eq!(
        mr.get("reason"),
        Some(&Value::String("content_not_found".into()))
    );

    // Changed/tampered candidate source MUST fail verification
    let tampered_resolve = eval_admitted(
        &format!(r#"= code.resolve("{hash_addr}", "= 21 * 3;")"#),
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(tr) = tampered_resolve else {
        panic!("expected Record");
    };
    assert_eq!(tr.get("resolved"), Some(&Value::Bool(false)));
    assert_eq!(tr.get("verified"), Some(&Value::Bool(false)));
    assert_eq!(
        tr.get("reason"),
        Some(&Value::String("digest_mismatch".into()))
    );

    // Registered content-addressed AST resolves and verifies bytes
    let good_resolve = eval_admitted(
        &format!(r#"= code.resolve("{hash_addr}", "= 21 * 2;")"#),
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(gr) = good_resolve else {
        panic!("expected Record");
    };
    assert_eq!(gr.get("resolved"), Some(&Value::Bool(true)));
    assert_eq!(gr.get("verified"), Some(&Value::Bool(true)));

    // Remote launch is unsupported; unavailable content also refuses on the local route.
    let remote_error = eval_admitted(
        &format!(r#"= code.spawn_at("did:q42:node_alpha", "{hash_addr}", [])"#),
        &mut host,
        &mut env,
    )
    .expect_err("this local profile must refuse remote launch");
    assert_eq!(remote_error.code, crate::DiagCode::E400);
    assert!(remote_error.message.contains("unsupported"));
    let missing_code = eval_admitted(
        r#"= code.spawn_at("did:q42:local_scheduler", "sha256:vibe:0000000000000000000000000000000000000000000000000000000000000000", [])"#,
        &mut host,
        &mut env,
    )
    .expect_err("the admitted local route must refuse unavailable content");
    assert_eq!(missing_code.code, crate::DiagCode::E400);
    assert!(missing_code.message.contains("unavailable"));

    // 6. Unregistered records refuse; authored IDs do not provide runtime ownership.
    let forged_cancel = eval_admitted(
        r#"= task.cancel({ task_id: 18446744073709551615u64, status: "running" })"#,
        &mut host,
        &mut env,
    )
    .unwrap_err();
    assert_eq!(forged_cancel.code, crate::DiagCode::E400);
    assert!(forged_cancel
        .message
        .contains("unknown or its runtime scope was disposed"));
    let forged_await = eval_admitted(
        r#"= await { task_id: 18446744073709551615u64, status: "running" }"#,
        &mut host,
        &mut env,
    )
    .unwrap_err();
    assert_eq!(forged_await.code, crate::DiagCode::E400);
    assert!(forged_await
        .message
        .contains("unknown or its runtime scope was disposed"));

    // 7. Terminal state immutability: cancelling an already-completed task preserves its committed result
    let src_immut = r#"
module test_terminal_immutability;

effect async fn completed_task_cannot_be_overwritten_by_cancel() budget(steps: 200) {
    let job = task.spawn("= 19 + 23", {});
    let joined = await job;
    let after_cancel = task.cancel(job);
    return {
        joined_result: joined.result,
        cancel_status: after_cancel.status,
        cancel_result: after_cancel.result,
        already_committed: after_cancel.already_committed,
        cancel_requested: after_cancel.cancel_requested,
        cancel_acknowledged: after_cancel.cancel_acknowledged,
        live_job_cancelled: after_cancel.live_job_cancelled
    };
}
"#;
    let prog_immut = load_program(src_immut).unwrap();
    let out_immut = eval_function(
        &prog_immut,
        "completed_task_cannot_be_overwritten_by_cancel",
        vec![],
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(ri) = out_immut else {
        panic!("expected Record");
    };
    assert_eq!(ri.get("joined_result"), Some(&Value::I64(42)));
    assert_eq!(
        ri.get("cancel_status"),
        Some(&Value::String("completed".into()))
    );
    assert_eq!(ri.get("cancel_result"), Some(&Value::I64(42)));
    assert_eq!(ri.get("already_committed"), Some(&Value::Bool(true)));
    assert_eq!(ri.get("cancel_requested"), Some(&Value::Bool(true)));
    assert_eq!(ri.get("cancel_acknowledged"), Some(&Value::Bool(false)));
    assert_eq!(ri.get("live_job_cancelled"), Some(&Value::Bool(false)));

    // 8. Provenance local integrity and ZK proof availability remain distinct.
    let forged_prov = eval_admitted(
        r#"= value.verify_provenance({ value: 42, provenance: { author: "alice" }, signature_u64: 12345u64 })"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(fp) = forged_prov else {
        panic!("expected Record");
    };
    assert_eq!(fp.get("authentic"), Some(&Value::Bool(false)));

    let valid_prov = eval_admitted(
        r#"= value.verify_provenance(value.attach_provenance(42, { author: "alice" }))"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(vp) = valid_prov else {
        panic!("expected Record");
    };
    assert_eq!(vp.get("authentic"), Some(&Value::Bool(true)));
    assert_eq!(vp.get("integrity_verified"), Some(&Value::Bool(true)));
    assert_eq!(
        vp.get("issuer_signature_verified"),
        Some(&Value::Bool(false))
    );

    let forged_zk = eval_admitted(
        r#"= crypto.verify_zk_predicate({ predicate: "age >= 18", commitment: 0u64, verified: true }, 18)"#,
        &mut host,
        &mut env,
    )
    .unwrap_err();
    assert_eq!(forged_zk.code, crate::DiagCode::E600);
    assert!(forged_zk
        .message
        .contains("no native zero-knowledge proof provider"));

    let unsupported_zk = eval_admitted(
        r#"= crypto.verify_zk_predicate(crypto.prove_zk_predicate("age >= 18", 18, { secret_dob: 2000 }), 18)"#,
        &mut host,
        &mut env,
    )
    .unwrap_err();
    assert_eq!(unsupported_zk.code, crate::DiagCode::E600);
    assert!(unsupported_zk
        .message
        .contains("no native zero-knowledge proof provider"));

    // A local commitment proves only that these exact public strings/values
    // still match. It does not evaluate the predicate or establish ZK proof.
    let commitment = eval_admitted(
        r#"= crypto.commit_predicate("1 == 0", 0)"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(commitment_record) = commitment else {
        panic!("expected Record")
    };
    assert_eq!(
        commitment_record.get("predicate_truth_verified"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        commitment_record.get("issuer_authenticity_verified"),
        Some(&Value::Bool(false))
    );
    let integrity = eval_admitted(
        r#"= crypto.verify_commitment(crypto.commit_predicate("1 == 0", 0), 0)"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(integrity_record) = integrity else {
        panic!("expected Record")
    };
    assert_eq!(
        integrity_record.get("integrity_verified"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        integrity_record.get("predicate_truth_verified"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        integrity_record.get("issuer_authenticity_verified"),
        Some(&Value::Bool(false))
    );
    let tampered = eval_admitted(
        r#"= crypto.verify_commitment(crypto.commit_predicate("1 == 0", 0), 1)"#,
        &mut host,
        &mut env,
    )
    .unwrap();
    let Value::Record(tampered_record) = tampered else {
        panic!("expected Record")
    };
    assert_eq!(
        tampered_record.get("integrity_verified"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        tampered_record.get("predicate_truth_verified"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        tampered_record.get("issuer_authenticity_verified"),
        Some(&Value::Bool(false))
    );
}

#[test]
fn content_addressed_cell_executes_through_admitted_local_scheduler() {
    let mut host = LocalHost::default();
    let mut env = Env::default();
    let source = r#"
module test_hashed_cell_execution;

effect async fn main() budget(steps: 200) {
    let hashed = code.hash_ast("= 21 + 21");
    let job = code.spawn_at("did:q42:local_scheduler", hashed.hash_hex, []);
    return await job;
}


"#;
    let program = load_program(source).expect("effect declaration should parse and check");
    let result = eval_function(&program, "main", vec![], &mut host, &mut env)
        .expect("checked local source cell should run through the admitted scheduler");
    let Value::Record(result) = result else {
        panic!("expected joined task record")
    };
    assert_eq!(
        result.get("status"),
        Some(&Value::String("completed".into()))
    );
    assert_eq!(result.get("result"), Some(&Value::I64(42)));
    assert_eq!(
        result.get("execution_qualification"),
        Some(&Value::String("local_deferred_synchronous_ast_task".into()))
    );
    assert_eq!(result.get("live_job_joined"), Some(&Value::Bool(true)));
}

#[test]
fn nursery_prevalidates_all_scoped_children_before_cancelling_any() {
    let mut host = LocalHost::default();
    let mut env = Env::default();
    let pending_sibling = eval_admitted(r#"= task.spawn("= 7 * 6", {})"#, &mut host, &mut env)
        .expect("a synthetic runtime-scoped task should register as pending");
    env.vars.insert("pending_sibling".into(), pending_sibling);

    let forged_group = eval_admitted(
        r#"= task.nursery([pending_sibling, { task_id: 18446744073709551615u64, status: "failed" }])"#,
        &mut host,
        &mut env,
    )
    .expect_err("nursery must validate every child handle before cancelling siblings");
    assert_eq!(forged_group.code, crate::DiagCode::E400);
    assert!(forged_group
        .message
        .contains("unknown or its runtime scope was disposed"));

    let state = eval_admitted("= task.status(pending_sibling)", &mut host, &mut env)
        .expect("valid sibling remains queryable after refusal");
    let Value::Record(state) = state else {
        panic!("expected status record");
    };
    assert_eq!(state.get("status"), Some(&Value::String("pending".into())));
    assert_eq!(state.get("cancel_requested"), Some(&Value::Bool(false)));
}
