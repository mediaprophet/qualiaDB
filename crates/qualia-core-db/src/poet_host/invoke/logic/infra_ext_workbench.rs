//! Bounded adapters for POET's P2 infrastructure-extension panels.

use super::super::args;
use crate::foundation::crdt::{CrdtResolver, DelegatedAccess};
use crate::identity::agency::compute_scoped_merkle_root;
use crate::modalities::control_feedback::{FeedbackController, PidParameters};
use crate::modalities::likeliness::{combine_premises, Likeliness};
use crate::modalities::logic::owl::{
    materialize_owl_rl, ChainAxiom, DisjointnessViolation, RdfTriple, OWL_DISJOINT_WITH,
    OWL_EQUIVALENT_CLASS, RDFS_SUBCLASS_OF, RDF_TYPE,
};
use crate::modalities::logic::qubo::{compile_quins_to_qubo, solve_classical, QuboMatrix};
use crate::modalities::{carrier, interaction_governance};
use crate::q_hash;
use crate::specialized_libs::linear_algebra::privacy::{
    HomomorphicKey, HomomorphicKeyManager, HomomorphicKeyType,
};
use crate::NQuin;
use vibe::{Diagnostic, Span, Value};

const MAX_ITEMS: usize = 64;

pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "mode") {
        Some("crdt") => crdt(args_v, span),
        Some("agency") => agency(args_v, span),
        Some("key_vault") => key_vault(args_v, span),
        Some("policy") => policy(args_v, span),
        Some("consent") => consent(args_v, span),
        Some("carrier") => carrier_eval(args_v, span),
        Some("control_feedback") => control(args_v, span),
        Some("likeliness") => likeliness(args_v, span),
        Some("qubo") => qubo(args_v, span),
        Some("owl") => owl(args_v, span),
        _ => Err(args::bad(
            span,
            "InfraExtLogic.compute needs a supported `mode`",
        )),
    }
}

fn crdt(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let local_clock = args::rec_u64(args_v, "local_clock").unwrap_or(0) as u32;
    let remote_clock = args::rec_u64(args_v, "remote_clock").unwrap_or(0) as u32;
    let local_object = args::rec_u64(args_v, "local_object").unwrap_or(1);
    let remote_object = args::rec_u64(args_v, "remote_object").unwrap_or(2);
    let mut local = quin(q_hash("local"), q_hash("p"), local_object);
    let mut remote = quin(q_hash("remote"), q_hash("p"), remote_object);
    local.set_lamport_clock(local_clock);
    remote.set_lamport_clock(remote_clock);
    let winner = CrdtResolver::resolve_lww(
        &local,
        &remote,
        args::rec_bool(args_v, "selfhood").unwrap_or(false),
    );
    let mut access = DelegatedAccess {
        principal_did: [0; 32],
        delegate_did: [0; 32],
        context_bound: args::rec_str(args_v, "context").map(q_hash).unwrap_or(0),
        expiration_timestamp: args::rec_u64(args_v, "expiry").unwrap_or(0),
        cryptographic_proof: [0; 64],
    };
    access.principal_did[..8].copy_from_slice(
        &args::rec_str(args_v, "principal")
            .map(q_hash)
            .unwrap_or(0)
            .to_le_bytes(),
    );
    access.delegate_did[..8].copy_from_slice(
        &args::rec_str(args_v, "delegate")
            .map(q_hash)
            .unwrap_or(0)
            .to_le_bytes(),
    );
    let now = args::rec_u64(args_v, "now").unwrap_or(0);
    Ok(args::record([
        (
            "winner_clock",
            Value::U64(winner.extract_lamport_clock() as u64),
        ),
        ("winner_object", Value::U64(winner.object)),
        (
            "delegation_valid",
            Value::Bool(CrdtResolver::verify_delegation(
                &access,
                access.context_bound,
                now,
            )),
        ),
    ]))
}

fn agency(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let author = q_hash(args::rec_str(args_v, "author").unwrap_or("author"));
    let claims = args::rec_str_list(args_v, "claims").unwrap_or_default();
    if claims.len() > MAX_ITEMS {
        return Err(args::bad(span, "agency claims exceed 64"));
    }
    let frame: Vec<NQuin> = claims
        .iter()
        .map(|claim| {
            let mut quin = quin(author, q_hash("q42:claims"), q_hash(claim));
            quin.context = author;
            quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
            quin
        })
        .collect();
    let root = compute_scoped_merkle_root(&frame, author);
    Ok(args::record([
        ("claims", Value::U64(frame.len() as u64)),
        (
            "merkle_root",
            Value::String(root.iter().map(|b| format!("{b:02x}")).collect()),
        ),
        ("identifier_is_not_identity", Value::Bool(true)),
    ]))
}

fn key_vault(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut vault = HomomorphicKeyManager::new();
    if args::rec_str(args_v, "operation").unwrap_or("list") == "register" {
        vault
            .register(HomomorphicKey {
                key_id_hash: q_hash(args::rec_str(args_v, "key_id").unwrap_or("key-0")),
                key_type: HomomorphicKeyType::Bfv,
                created_at: args::rec_u64(args_v, "created_at").unwrap_or(0),
                expires_at: args::rec_u64(args_v, "expires_at").unwrap_or(u64::MAX),
            })
            .map_err(|error| args::bad(span, format!("key vault: {error:?}")))?;
    }
    let now = args::rec_u64(args_v, "now").unwrap_or(0);
    let expired = vault.remove_expired(now);
    Ok(args::record([
        ("capacity", Value::U64(8)),
        ("occupied", Value::U64(vault.len() as u64)),
        ("expired_removed", Value::U64(expired as u64)),
        (
            "rotation_interval",
            Value::U64(vault.key_rotation_policy.rotation_interval),
        ),
        (
            "automatic_rotation",
            Value::Bool(vault.key_rotation_policy.automatic_rotation),
        ),
    ]))
}

fn policy(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let clearance = sensitivity(args::rec_str(args_v, "clearance").unwrap_or("public"), span)?;
    let resource = sensitivity(
        args::rec_str(args_v, "sensitivity").unwrap_or("public"),
        span,
    )?;
    let ambiguous = args::rec_str(args_v, "epistemic")
        .is_some_and(|value| matches!(value, "uncertain" | "skipped"));
    let permitted = clearance >= resource && !ambiguous;
    let mode = if ambiguous {
        interaction_governance::PolicyMode::Interactive
    } else if permitted {
        interaction_governance::PolicyMode::Allow
    } else {
        interaction_governance::PolicyMode::PreventiveBlock
    };
    Ok(args::record([
        ("permitted", Value::Bool(permitted)),
        (
            "action",
            Value::String(interaction_governance::policy_action(mode).into()),
        ),
        (
            "subject",
            Value::String(args::rec_str(args_v, "subject").unwrap_or_default().into()),
        ),
        (
            "resource",
            Value::String(args::rec_str(args_v, "resource").unwrap_or_default().into()),
        ),
    ]))
}

fn consent(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let operation = args::rec_str(args_v, "operation").unwrap_or("list");
    let now = args::rec_u64(args_v, "now").unwrap_or(0);
    let expiry = args::rec_u64(args_v, "expiry").unwrap_or(0);
    let revoked = operation == "revoke" || args::rec_bool(args_v, "revoked").unwrap_or(false);
    let in_force = operation != "revoke" && (expiry == 0 || now < expiry) && !revoked;
    Ok(args::record([
        ("operation", Value::String(operation.into())),
        (
            "scope",
            Value::String(args::rec_str(args_v, "scope").unwrap_or_default().into()),
        ),
        ("in_force", Value::Bool(in_force)),
        ("revoked", Value::Bool(revoked)),
    ]))
}

fn carrier_eval(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let payload = args::rec_str(args_v, "payload").unwrap_or("");
    if payload.is_empty() {
        return Err(args::bad(span, "carrier inspection needs `payload`"));
    }
    if payload.len() > 4096 {
        return Err(args::bad(span, "carrier payload exceeds 4 KiB"));
    }
    let tag = carrier::media_tag(payload.as_bytes());
    let expected = args::rec_u64(args_v, "bound_tag").unwrap_or(tag);
    Ok(args::record([
        ("media_tag", Value::U64(tag)),
        (
            "binding_valid",
            Value::Bool(carrier::verify_binding(payload.as_bytes(), expected)),
        ),
    ]))
}

fn control(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let setpoint = need_f64(args_v, "setpoint", span)?;
    let measured = need_f64(args_v, "measured", span)?;
    let mut controller = FeedbackController::new(
        "workbench".into(),
        setpoint,
        measured,
        PidParameters::conservative_power_system(),
    );
    controller
        .state
        .update(measured, args::rec_u64(args_v, "t").unwrap_or(1));
    let output = controller.compute_output();
    Ok(args::record([
        ("output", Value::F64(output)),
        ("error", Value::F64(controller.state.error)),
        ("integral", Value::F64(controller.state.integral)),
        ("derivative", Value::F64(controller.state.derivative)),
    ]))
}

fn likeliness(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let levels = args::rec_f64_list(args_v, "premises")
        .ok_or_else(|| args::bad(span, "likeliness needs `premises`"))?;
    if levels.is_empty() || levels.len() > MAX_ITEMS {
        return Err(args::bad(span, "premises must contain 1..=64 levels"));
    }
    let premises: Vec<Likeliness> = levels
        .iter()
        .map(|level| Likeliness::from_level(*level as i8))
        .collect();
    let combined = combine_premises(&premises);
    Ok(args::record([
        ("combined", Value::String(format!("{combined:?}"))),
        ("level", Value::I64(combined.level() as i64)),
    ]))
}

fn qubo(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let edges = args::rec_str_list(args_v, "edges")
        .ok_or_else(|| args::bad(span, "QUBO compile needs `edges`"))?;
    if edges.is_empty() || edges.len() > MAX_ITEMS {
        return Err(args::bad(span, "edges must contain 1..=64 pairs"));
    }
    let quins: Result<Vec<NQuin>, Diagnostic> = edges
        .iter()
        .map(|edge| {
            let (left, right) = edge
                .split_once(':')
                .ok_or_else(|| args::bad(span, "QUBO edges use left:right"))?;
            Ok(quin(q_hash(left.trim()), 0x10, q_hash(right.trim())))
        })
        .collect();
    let quins = quins?;
    let mut matrix = QuboMatrix::default();
    compile_quins_to_qubo(&quins, &mut matrix)
        .map_err(|error| args::bad(span, format!("QUBO compile failed: {error:?}")))?;
    let mut assignment = [0u8; crate::modalities::logic::qubo::MAX_QUBO_VARS];
    let energy = solve_classical(&matrix, &mut assignment);
    Ok(args::record([
        ("variables", Value::U64(matrix.num_vars as u64)),
        ("couplers", Value::U64(matrix.coupler_count as u64)),
        ("energy", Value::F64(energy as f64)),
        (
            "assignment",
            Value::List(
                assignment[..matrix.num_vars as usize]
                    .iter()
                    .map(|bit| Value::U64(*bit as u64))
                    .collect(),
            ),
        ),
    ]))
}

fn owl(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let triples_in = args::rec_str_list(args_v, "triples")
        .ok_or_else(|| args::bad(span, "OWL evaluation needs `triples`"))?;
    if triples_in.is_empty() || triples_in.len() > MAX_ITEMS {
        return Err(args::bad(span, "triples must contain 1..=64 axioms"));
    }
    let mut working = [RdfTriple::new(0, 0, 0); 128];
    let mut initial = 0usize;
    for triple in &triples_in {
        let mut parts = triple.split(':');
        let subject = parts
            .next()
            .ok_or_else(|| args::bad(span, "triples use subject:predicate:object"))?;
        let predicate = parts
            .next()
            .ok_or_else(|| args::bad(span, "triples use subject:predicate:object"))?;
        let object = parts
            .next()
            .ok_or_else(|| args::bad(span, "triples use subject:predicate:object"))?;
        let p = match predicate {
            "subClassOf" | "rdfs:subClassOf" => RDFS_SUBCLASS_OF,
            "type" | "rdf:type" | "a" => RDF_TYPE,
            "disjointWith" | "owl:disjointWith" => OWL_DISJOINT_WITH,
            "equivalentClass" | "owl:equivalentClass" => OWL_EQUIVALENT_CLASS,
            _ => {
                return Err(args::bad(
                    span,
                    format!("unsupported OWL predicate `{predicate}`"),
                ))
            }
        };
        working[initial] = RdfTriple::new(q_hash(subject), p, q_hash(object));
        initial += 1;
    }
    let mut contradictions = [DisjointnessViolation {
        individual: 0,
        class_a: 0,
        class_b: 0,
    }; 16];
    let summary = materialize_owl_rl(
        &mut working,
        initial,
        &[] as &[ChainAxiom],
        16,
        &mut contradictions,
    )
    .map_err(|error| args::bad(span, format!("OWL materialize failed: {error:?}")))?;
    Ok(args::record([
        ("triple_count", Value::U64(summary.triple_count as u64)),
        ("inferred", Value::U64(summary.inferred_count as u64)),
        (
            "contradictions",
            Value::U64(summary.contradiction_count as u64),
        ),
        ("iterations", Value::U64(summary.iterations as u64)),
        ("saturated", Value::Bool(summary.saturated)),
        ("consistent", Value::Bool(summary.contradiction_count == 0)),
        (
            "operation",
            Value::String(
                args::rec_str(args_v, "operation")
                    .unwrap_or("materialize")
                    .into(),
            ),
        ),
    ]))
}

fn sensitivity(label: &str, span: Span) -> Result<u8, Diagnostic> {
    Ok(match label {
        "public" => NQuin::SENSITIVITY_PUBLIC,
        "restricted" | "internal" | "confidential" => NQuin::SENSITIVITY_RESTRICTED,
        "classified" | "top_secret" => NQuin::SENSITIVITY_CLASSIFIED,
        other => return Err(args::bad(span, format!("unknown sensitivity `{other}`"))),
    })
}

fn need_f64(args_v: &Value, key: &str, span: Span) -> Result<f64, Diagnostic> {
    let value =
        args::rec_f64(args_v, key).ok_or_else(|| args::bad(span, format!("needs `{key}`")))?;
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| args::bad(span, format!("`{key}` must be finite")))
}

fn quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    let mut quin = NQuin {
        subject,
        predicate,
        object,
        context: q_hash("urn:poet:infra-ext-workbench"),
        metadata: 0,
        parity: 0,
    };
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
    quin
}

#[cfg(test)]
#[path = "infra_ext_workbench_tests.rs"]
mod tests;
