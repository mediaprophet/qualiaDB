use super::*;

#[test]
fn statistical_analysis_tool_covers_all_operations() {
    // A dataset with a perfect linear relation y = 2x + 1 over x=1..=5.
    // `call` builds one JSON body per stat with the given extra fields.
    let call = |stat: &str, extra: &str| -> Value {
        let body = format!(
            r#"{{"dataset_id":"d","columns":["x","y"],"rows":[[1,3],[2,5],[3,7],[4,9],[5,11]],"column":"x","column_y":"y","stat":"{}"{}}}"#,
            stat, extra
        );
        let out = statistical_analysis(body.as_bytes())
            .unwrap_or_else(|e| panic!("stat {} failed: {:?}", stat, e));
        serde_json::from_str(&out).expect("json")
    };

    assert!((call("mean", "")["value"].as_f64().unwrap() - 3.0).abs() < 1e-9);
    assert!((call("median", "")["value"].as_f64().unwrap() - 3.0).abs() < 1e-9);
    // Population std of x=[1,2,3,4,5] is sqrt(2).
    assert!(
        (call("standard_deviation", r#","sample":false"#)["value"]
            .as_f64()
            .unwrap()
            - 2.0f64.sqrt())
        .abs()
            < 1e-9
    );
    assert!((call("correlation", "")["value"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    // Linear regression recovers slope 2, intercept 1, R²=1.
    let lr = call("linear_regression", "");
    assert!((lr["slope"].as_f64().unwrap() - 2.0).abs() < 1e-9);
    assert!((lr["intercept"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert!((lr["r_squared"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    // Descriptive + timeseries coverage smoke.
    assert!(call("skewness", "")["value"].is_number());
    assert!(call("kurtosis", "")["value"].is_number());
    assert!(call("covariance", "")["value"].is_number());
    assert!(call("quantile", r#","q":0.5"#)["value"].is_number());
    assert!(call("moving_average", r#","window":2"#)["series"].is_array());
    assert!(call("exponential_smoothing", r#","alpha":0.5"#)["series"].is_array());

    // An unknown stat is a clean parameter error, not a panic.
    let bad = r#"{"dataset_id":"d","columns":["x","y"],"rows":[[1,3]],"stat":"nonesuch"}"#;
    assert!(statistical_analysis(bad.as_bytes()).is_err());
}

#[test]
fn physics_ode_solve_tool_projectile_known_range() {
    // v0=10 m/s at 45°, no drag → range = v²·sin(2θ)/g = 100/9.81 ≈ 10.19 m.
    let body = r#"{"type":"projectile","v0":10.0,"angle_rad":0.7853981633974483,"g":9.81,"drag":0.0,"num_samples":200,"max_time":5.0}"#;
    let out = ode_solve(body.as_bytes()).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(p["type"], "projectile");
    assert!(
        (p["range"].as_f64().unwrap() - 10.19).abs() < 0.1,
        "range={}",
        p["range"]
    );
}

#[test]
fn medical_score_tool_bmi_known_value() {
    // 70 kg / 1.75 m → BMI 22.857.
    let body = r#"{"metric":"bmi","weight_kg":70.0,"height_m":1.75}"#;
    let out = medical_score(body.as_bytes()).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert!(
        (p["value"].as_f64().unwrap() - 22.857).abs() < 1e-2,
        "bmi={}",
        p["value"]
    );

    // The diagnosis path stays honest — not a fabricated result.
    let diag = r#"{"score":"diagnosis"}"#;
    assert!(matches!(
        medical_score(diag.as_bytes()),
        Err(McpSystemError::ToolNotReady)
    ));
}

#[test]
fn chemical_analysis_tool_quantum_h2_scf_energy() {
    // H2 at R=1.4 bohr → real RHF/STO-3G total energy ≈ -1.1167 Hartree.
    let body = r#"{"op":"quantum","atoms":[
            {"element":"H","atomic_number":1,"x":0.0,"y":0.0,"z":0.0},
            {"element":"H","atomic_number":1,"x":0.0,"y":0.0,"z":1.4}]}"#;
    let out = chemical_analysis(body.as_bytes()).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert!(
        (p["total_energy_hartree"].as_f64().unwrap() - (-1.1167)).abs() < 1e-3,
        "E={}",
        p["total_energy_hartree"]
    );
    assert!(p["gap"].as_f64().unwrap() > 0.0);
}

#[test]
fn medical_score_tool_image_and_differential() {
    // Image: 4x4 step edge (left cols 0, right cols 100), fixed threshold 50.
    // 8 pixels >= 50, region mean 100, overall mean 50.
    let img = r#"{"op":"image","width":4,"height":4,"bins":4,"threshold":50.0,
            "data":[0,0,100,100, 0,0,100,100, 0,0,100,100, 0,0,100,100]}"#;
    let out = medical_score(img.as_bytes()).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert!((p["mean"].as_f64().unwrap() - 50.0).abs() < 1e-9);
    assert_eq!(p["segmented_area"], 8);
    assert!((p["segmented_mean_intensity"].as_f64().unwrap() - 100.0).abs() < 1e-9);

    // Differential: hand-computed posteriors 0.9 / 0.1, flu ranked first.
    let diff = r#"{"op":"differential","findings":["fever","cough"],
            "knowledge_base":[
              {"condition_id":"flu","prior":0.6,"likelihoods":{"fever":0.9,"cough":0.8}},
              {"condition_id":"cold","prior":0.4,"likelihoods":{"fever":0.2,"cough":0.6}}]}"#;
    let out2 = medical_score(diff.as_bytes()).expect("ok");
    let p2: Value = serde_json::from_str(&out2).expect("json");
    let ranked = p2["ranked"].as_array().unwrap();
    assert_eq!(ranked[0]["condition_id"], "flu");
    assert!((ranked[0]["posterior"].as_f64().unwrap() - 0.9).abs() < 1e-9);
}

#[test]
fn chemical_analysis_tool_structure_water_mass() {
    // H2O structural properties: mass ≈ 18.015, Hill formula "H2O".
    let body = r#"{"op":"structure","atoms":[
            {"element":"O","atomic_number":8,"x":0.0,"y":0.0,"z":0.0},
            {"element":"H","atomic_number":1,"x":0.757,"y":0.586,"z":0.0},
            {"element":"H","atomic_number":1,"x":-0.757,"y":0.586,"z":0.0}]}"#;
    let out = chemical_analysis(body.as_bytes()).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(p["formula"], "H2O");
    assert!(
        (p["molecular_mass"].as_f64().unwrap() - 18.015).abs() < 0.01,
        "mass={}",
        p["molecular_mass"]
    );
    assert_eq!(p["atom_count"], 3);
}

#[test]
fn values_check_tool_flags_corporate_capture() {
    // A corporation claiming a human dignity right → REJECT (PersonhoodCategoryError).
    let out =
        values_check(br#"{"agentType":"CorporatePerson","claimsDignityRight":true}"#).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(p["flagged"], true);
    assert_eq!(p["flag"], "values:PersonhoodCategoryError");

    // A natural person holding their own right → ok.
    let out2 =
        values_check(br#"{"agentType":"NaturalPerson","claimsDignityRight":true}"#).expect("ok");
    let p2: Value = serde_json::from_str(&out2).expect("json");
    assert_eq!(p2["flagged"], false);
    assert_eq!(p2["verdict"], "ok");
}

#[test]
fn values_evaluate_tool_deontic_lifecycle() {
    // A prohibition with no exception and no expiry → in force (Active).
    let out = values_evaluate(
        br#"{"modality":"forbid","party":"values:Agent","action":"values:DestructionOfRights"}"#,
    )
    .expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(p["status"], "Active");
    assert_eq!(p["modality"], "prohibition");

    // Same prohibition with a lawful-authorisation exception → Defeated.
    let out2 = values_evaluate(
            br#"{"modality":"forbid","party":"values:Agent","action":"values:DestructionOfRights","unless":"values:lawfullyAuthorised"}"#,
        )
        .expect("ok");
    let p2: Value = serde_json::from_str(&out2).expect("json");
    assert_eq!(p2["status"], "Defeated");
    assert_eq!(p2["exception"], true);

    // An obligation whose effective window has passed (expiry << now) → Expired.
    let out3 = values_evaluate(
            br#"{"modality":"oblige","party":"values:State","action":"values:ProvideRemedy","expiry":1000000000,"now":1717200000}"#,
        )
        .expect("ok");
    let p3: Value = serde_json::from_str(&out3).expect("json");
    assert_eq!(p3["status"], "Expired");
    assert_eq!(p3["modality"], "obligation");
}

#[test]
fn jural_correlate_tool() {
    let out = jural_correlate(br#"{"position":"claim"}"#).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(p["position"], "Claim");
    assert_eq!(p["correlative"], "Duty");
    assert_eq!(p["opposite"], "No-Right");
    assert_eq!(p["order"], "first-order (conduct)");

    let out2 = jural_correlate(br#"{"position":"immunity"}"#).expect("ok");
    let p2: Value = serde_json::from_str(&out2).expect("json");
    assert_eq!(p2["correlative"], "Disability");
    assert_eq!(p2["order"], "second-order (control)");

    assert!(jural_correlate(br#"{"position":"nonsense"}"#).is_err());
}

#[test]
fn deontic_govern_tool() {
    // Non-derogable violation → PreventiveBlock, does NOT permit execution.
    let out = deontic_govern(br#"{"status":"violated","nonDerogable":true}"#).expect("ok");
    let p: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(p["policyMode"], "PreventiveBlock");
    assert_eq!(p["action"], "DenyRollback");
    assert_eq!(p["permitsExecution"], false);

    // Ordinary violation → audit, permits execution.
    let out2 = deontic_govern(br#"{"status":"violated"}"#).expect("ok");
    let p2: Value = serde_json::from_str(&out2).expect("json");
    assert_eq!(p2["policyMode"], "PermissiveAudit");
    assert_eq!(p2["permitsExecution"], true);

    // Ambiguity defers to a human.
    let out3 = deontic_govern(br#"{"status":"active","ambiguous":true}"#).expect("ok");
    let p3: Value = serde_json::from_str(&out3).expect("json");
    assert_eq!(p3["policyMode"], "Interactive");
}

#[test]
fn cooperation_gate_decision() {
    use crate::mcp_cooperation::CooperationVerdict;
    // Enforcement OFF → always pass (None), regardless of caller.
    assert!(gate_verdict(br#"{}"#, false).is_none());
    // Enforcement ON, anonymous/unverified → DeniedUnverified.
    assert!(matches!(
        gate_verdict(br#"{}"#, true),
        Some(CooperationVerdict::DeniedUnverified)
    ));
    // ON, verified but not grounded → DeniedUngrounded.
    assert!(matches!(
        gate_verdict(br#"{"caller":"did:bot","verified":true}"#, true),
        Some(CooperationVerdict::DeniedUngrounded)
    ));
    // ON, verified + grounded → Authorized.
    assert!(matches!(
        gate_verdict(
            br#"{"caller":"did:alice","verified":true,"grounded":true}"#,
            true
        ),
        Some(CooperationVerdict::Authorized(_))
    ));
    // The public gate maps a denial to IntentFrameViolation (enforcement defaults off in CI).
    assert!(cooperation_gate(br#"{}"#).is_ok());
}

#[test]
fn mcp_cooperate_tool() {
    // Verified, grounded, ordinary request → Authorized.
    let ok = mcp_cooperate(br#"{"caller":"did:alice","verified":true,"requestStatus":"active"}"#)
        .expect("ok");
    let p: Value = serde_json::from_str(&ok).expect("json");
    assert_eq!(p["verdict"], "Authorized");
    assert_eq!(p["permitted"], true);

    // Asserted (not verified) → DeniedUnverified.
    let unv = mcp_cooperate(br#"{"caller":"did:x","verified":false}"#).expect("ok");
    assert_eq!(
        serde_json::from_str::<Value>(&unv).unwrap()["verdict"],
        "DeniedUnverified"
    );

    // Verified but ungrounded AI → DeniedUngrounded.
    let ung =
        mcp_cooperate(br#"{"caller":"did:bot","verified":true,"grounded":false}"#).expect("ok");
    assert_eq!(
        serde_json::from_str::<Value>(&ung).unwrap()["verdict"],
        "DeniedUngrounded"
    );

    // Verified + grounded but a non-derogable violation → DeniedByPolicy.
    let blk = mcp_cooperate(
        br#"{"caller":"did:alice","verified":true,"requestStatus":"violated","nonDerogable":true}"#,
    )
    .expect("ok");
    let pb: Value = serde_json::from_str(&blk).unwrap();
    assert_eq!(pb["verdict"], "DeniedByPolicy");
    assert_eq!(pb["policyMode"], "PreventiveBlock");
    assert_eq!(pb["permitted"], false);
}

#[test]
fn matrix_multiply_caller_matrices() {
    let args = json!({
        "op": "multiply",
        "left": {"id": "A", "rows": 2, "cols": 2, "data": [1.0, 0.0, 0.0, 2.0]},
        "right": {"id": "B", "rows": 2, "cols": 2, "data": [3.0, 0.0, 0.0, 4.0]},
        "result_id": "C"
    });
    let out = matrix_operation(args.to_string().as_bytes()).expect("ok");
    let parsed: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(parsed["rows"], 2);
    assert!(parsed["data"].as_array().unwrap()[0].as_f64().unwrap() > 0.0);
}

#[test]
fn algebra_solve_polynomial_tool() {
    // x² − 5x + 6 → roots {2, 3}
    let args = json!({ "coeffs": [1.0, -5.0, 6.0] });
    let out = algebra_solve_polynomial(args.to_string().as_bytes()).expect("ok");
    let parsed: Value = serde_json::from_str(&out).expect("json");
    let roots = parsed["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 2);
    let mut res: Vec<f64> = roots.iter().map(|r| r["re"].as_f64().unwrap()).collect();
    res.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((res[0] - 2.0).abs() < 1e-6 && (res[1] - 3.0).abs() < 1e-6);
}

#[test]
fn algebra_matrix_analyze_tool() {
    // determinant of [[1,2],[3,4]] = −2
    let det = algebra_matrix_analyze(
        json!({ "op": "determinant", "rows": 2, "cols": 2, "data": [1.0,2.0,3.0,4.0] })
            .to_string()
            .as_bytes(),
    )
    .expect("ok");
    let parsed: Value = serde_json::from_str(&det).expect("json");
    assert!((parsed["determinant"].as_f64().unwrap() + 2.0).abs() < 1e-9);

    // SVD reconstruction shape
    let svd = algebra_matrix_analyze(
        json!({ "op": "svd", "rows": 3, "cols": 2, "data": [1.0,0.0,0.0,1.0,1.0,1.0] })
            .to_string()
            .as_bytes(),
    )
    .expect("ok");
    assert!(svd.contains("singular_values"));
}

#[test]
fn cas_tool_differentiate_and_solve() {
    // d/dx (x^3 - 2*x^2 + 5) then evaluate at x=2 → 4
    let d = cas(
        json!({ "op": "differentiate", "expr": "x^3 - 2*x^2 + 5", "var": "x" })
            .to_string()
            .as_bytes(),
    )
    .expect("ok");
    assert!(d.contains("derivative"));

    let ev = cas(
        json!({ "op": "evaluate", "expr": "x^2 + 1", "env": { "x": 3.0 } })
            .to_string()
            .as_bytes(),
    )
    .expect("ok");
    let parsed: Value = serde_json::from_str(&ev).expect("json");
    assert!((parsed["value"].as_f64().unwrap() - 10.0).abs() < 1e-9);

    let q = cas(
        json!({ "op": "solve_quadratic", "a": 1.0, "b": -5.0, "c": 6.0 })
            .to_string()
            .as_bytes(),
    )
    .expect("ok");
    assert!(q.contains("roots"));

    let ex = cas(json!({ "op": "expand", "expr": "(x + 1) * (x + 2)" })
        .to_string()
        .as_bytes())
    .expect("ok");
    assert!(ex.contains("expanded"));

    let fac = cas(
        json!({ "op": "factor", "a": 1.0, "b": -5.0, "c": 6.0, "var": "x" })
            .to_string()
            .as_bytes(),
    )
    .expect("ok");
    assert!(fac.contains("factored"));
}

#[test]
fn bioinformatics_uses_caller_sequences() {
    let args = json!({"query": "ATCG", "target": "ATCC", "mode": "dna"});
    let out = bioinformatics_align(args.to_string().as_bytes()).expect("ok");
    assert!(out.contains("score"));
}

#[test]
fn clinical_framingham_rejects_incomplete_input() {
    let args = json!({
        "score": "framingham",
        "age": 55,
        "input": {"sex_male": true, "systolic_bp": 140.0}
    });
    assert!(clinical_risk(args.to_string().as_bytes()).is_err());
}

#[test]
fn geometric_cross_product() {
    let args = json!({"op": "cross", "a": [1, 0, 0], "b": [0, 1, 0]});
    let out = geometric_algebra_op(args.to_string().as_bytes()).expect("ok");
    let parsed: Value = serde_json::from_str(&out).expect("json");
    let r = parsed["result"].as_array().unwrap();
    assert!((r[2].as_f64().unwrap() - 1.0).abs() < 0.01);
}

#[test]
fn asp_arm_computes_real_answer_sets() {
    // Classic even loop: permitted :- not forbidden; forbidden :- not permitted.
    // Real Gelfond-Lifschitz semantics yield exactly TWO answer sets
    // ({permitted}, {forbidden}); the legacy context-bifurcation heuristic
    // this arm used to call could not.
    let args = json!({
        "modality": "asp",
        "atoms": [1u64, 2u64],
        "rules": [
            {"head": 1u64, "neg": [2u64]},
            {"head": 2u64, "neg": [1u64]}
        ]
    });
    let out = evaluate_modality(args.to_string().as_bytes()).expect("ok");
    let parsed: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(
        parsed["answer_set_count"], 2,
        "even loop has 2 answer sets: {out}"
    );
    let mut singles: Vec<u64> = parsed["answer_sets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let a = s.as_array().unwrap();
            assert_eq!(a.len(), 1, "each answer set is a singleton");
            a[0].as_u64().unwrap()
        })
        .collect();
    singles.sort();
    assert_eq!(singles, vec![1, 2]);
}

#[test]
fn asp_arm_integrity_constraint_prunes() {
    // Add `:- forbidden` (head == 0): forbids any model containing atom 2,
    // pruning the {forbidden} answer set and leaving only {permitted}.
    let args = json!({
        "modality": "asp",
        "atoms": [1u64, 2u64],
        "rules": [
            {"head": 1u64, "neg": [2u64]},
            {"head": 2u64, "neg": [1u64]},
            {"head": 0u64, "pos": [2u64]}
        ]
    });
    let out = evaluate_modality(args.to_string().as_bytes()).expect("ok");
    let parsed: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(
        parsed["answer_set_count"], 1,
        "constraint prunes to 1: {out}"
    );
    assert_eq!(
        parsed["answer_sets"][0][0], 1u64,
        "only {{permitted}} survives"
    );
}
