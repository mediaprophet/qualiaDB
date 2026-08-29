//! Bounded adapters for POET's P1 governance workbench surfaces.

use super::super::args;
use crate::modalities::logic::deontic::{compile_norm_quin, DeonticStatus, OP_FORBID, OP_OBLIGATE};
use crate::modalities::{
    capability_gap, deontic_compose, identity_fabric, interaction_governance, legal_compose, stit,
    value_flow,
};
use crate::q_hash;
use crate::NQuin;
use vibe::{Diagnostic, Span, Value};

const MAX_ITEMS: usize = 64;

pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "mode") {
        Some("value_flow") => value_flow_eval(args_v, span),
        Some("interaction") => interaction(args_v, span),
        Some("identity") => identity(args_v, span),
        Some("capability_gap") => gap(args_v, span),
        Some("legal_compose") => legal(args_v, span),
        Some("deontic_compose") => deontic(args_v, span),
        _ => Err(args::bad(
            span,
            "GovernanceLogic.compute needs a supported `mode`",
        )),
    }
}

fn value_flow_eval(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let operation = args::rec_str(args_v, "operation").unwrap_or("flow");
    if operation == "royalty" {
        let base = need_u64(args_v, "base", span)?;
        let multiplier = need_u64(args_v, "agent_multiplier_percent", span)?;
        let generations = args::rec_u64(args_v, "generations").unwrap_or(0) as u32;
        let share = args::rec_u64(args_v, "share_percent").unwrap_or(50);
        let amount = value_flow::royalty(base, multiplier);
        return Ok(args::record([
            ("royalty", Value::U64(amount)),
            (
                "ancestor_total",
                Value::U64(value_flow::royalty_tree_total(amount, generations, share)),
            ),
        ]));
    }
    let production = need_u64(args_v, "production_cost", span)?;
    let roi_cap = need_u64(args_v, "roi_cap_percent", span)?;
    let max_roi = need_u64(args_v, "max_roi_percent", span)?;
    let pool = args::rec_u64(args_v, "pool").unwrap_or(0);
    let cost = value_flow::commons_cost(production, roi_cap, max_roi);
    let returned = args::rec_u64(args_v, "energy_returned").unwrap_or(0);
    let invested = args::rec_u64(args_v, "energy_invested").unwrap_or(0);
    let min_ratio = args::rec_f64(args_v, "min_ratio").unwrap_or(1.0) as f32;
    Ok(args::record([
        ("commons_cost", Value::U64(cost)),
        (
            "outstanding",
            Value::U64(value_flow::outstanding(pool, cost)),
        ),
        (
            "discharged",
            Value::Bool(value_flow::is_commons_discharged(pool, cost)),
        ),
        (
            "eroi_viable",
            Value::Bool(value_flow::eroi_viable(returned, invested, min_ratio)),
        ),
        (
            "eroi",
            Value::F64(value_flow::eroi(returned, invested) as f64),
        ),
    ]))
}

fn interaction(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let status = deontic_status(args_v, span)?;
    let governance = interaction_governance::Governance {
        non_derogable: args::rec_bool(args_v, "non_derogable").unwrap_or(false),
        humanitarian: args::rec_bool(args_v, "humanitarian").unwrap_or(false),
        ambiguous: args::rec_bool(args_v, "ambiguous").unwrap_or(false),
    };
    let mode = interaction_governance::map_policy(status, governance);
    let emergency = args::rec_bool(args_v, "emergency").unwrap_or(false);
    let hard_core = args::rec_bool(args_v, "hard_core").unwrap_or(false);
    let overridden = interaction_governance::apply_emergency_override(mode, emergency, hard_core);
    Ok(args::record([
        ("policy_mode", Value::String(format!("{overridden:?}"))),
        (
            "action",
            Value::String(interaction_governance::policy_action(overridden).into()),
        ),
        (
            "permits_execution",
            Value::Bool(interaction_governance::permits_execution(overridden)),
        ),
        (
            "agent",
            Value::String(args::rec_str(args_v, "agent").unwrap_or_default().into()),
        ),
        (
            "action_label",
            Value::String(args::rec_str(args_v, "action").unwrap_or_default().into()),
        ),
    ]))
}

fn identity(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let anchors = hashed_names(args_v, "anchors", span)?;
    let lost = hashed_names(args_v, "lost", span)?;
    if anchors.len() > MAX_ITEMS || lost.len() > MAX_ITEMS {
        return Err(args::bad(span, "identity fabric exceeds 64 anchors"));
    }
    let total = args::rec_u64(args_v, "total_anchors")
        .map(|n| n as usize)
        .unwrap_or(anchors.len());
    let lost_count = args::rec_u64(args_v, "lost_anchors")
        .map(|n| n as usize)
        .unwrap_or(lost.len());
    let quorum = args::rec_u64(args_v, "quorum").unwrap_or(1) as usize;
    let mut surviving = [0u64; MAX_ITEMS];
    let n = if anchors.is_empty() {
        identity_fabric::surviving_anchors(total, lost_count)
    } else {
        identity_fabric::recompute_fabric(&anchors, &lost, &mut surviving)
    };
    Ok(args::record([
        ("surviving_anchors", Value::U64(n as u64)),
        (
            "survives",
            Value::Bool(identity_fabric::identity_survives_loss(
                total, lost_count, quorum,
            )),
        ),
        (
            "confidence",
            Value::F64(identity_fabric::enumerated_identity_confidence(n, total.max(1)) as f64),
        ),
        (
            "identifier_is_not_identity",
            Value::Bool(identity_fabric::identifier_is_not_identity()),
        ),
    ]))
}

fn gap(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let required_names = names(args_v, "required", span)?;
    let held_names = names(args_v, "held", span)?;
    if required_names.len() > MAX_ITEMS || held_names.len() > MAX_ITEMS {
        return Err(args::bad(span, "capability lists exceed 64 items"));
    }
    let required: Vec<u64> = required_names.iter().map(|name| q_hash(name)).collect();
    let held: Vec<u64> = held_names.iter().map(|name| q_hash(name)).collect();
    let equivalences = pair_hashes(args_v, "equivalences", span)?;
    let mut missing = [0u64; MAX_ITEMS];
    let n = capability_gap::capability_gap(&required, &held, &equivalences, &mut missing);
    let missing_names = required_names
        .iter()
        .filter(|name| missing[..n].contains(&q_hash(name)))
        .map(|name| Value::String(name.clone()))
        .collect();
    let mut result = vec![
        ("gap_count", Value::U64(n as u64)),
        ("missing", Value::List(missing_names)),
        (
            "requirements_met",
            Value::Bool(capability_gap::requirements_met(
                &required,
                &held,
                &equivalences,
            )),
        ),
    ];
    if let Some(goal) = args::rec_str(args_v, "goal") {
        if let Some(edges) = args::rec_str_list(args_v, "edges") {
            let mut nodes = required.clone();
            for held_id in &held {
                if !nodes.contains(held_id) {
                    nodes.push(*held_id);
                }
            }
            let parsed = parse_cost_edges(&edges, span)?;
            result.push((
                "learning_path_cost",
                match capability_gap::learning_path_cost(&nodes, &parsed, &held, q_hash(goal)) {
                    Some(cost) => Value::U64(cost as u64),
                    None => Value::String("unreachable".into()),
                },
            ));
        }
    }
    Ok(args::record(result))
}

fn legal(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let operation = args::rec_str(args_v, "operation").unwrap_or("compose");
    if operation == "zk" {
        let verified = args::rec_bool(args_v, "proof_verified").unwrap_or(false);
        return Ok(args::record([(
            "eligibility",
            Value::String(format!("{:?}", legal_compose::zk_eligibility(verified))),
        )]));
    }
    let claims = hashed_names(args_v, "all_claims", span)?;
    let reveal = hashed_names(args_v, "reveal", span)?;
    if claims.len() > MAX_ITEMS || reveal.len() > MAX_ITEMS {
        return Err(args::bad(span, "legal compose exceeds 64 claims"));
    }
    let mut disclosed = [0u64; MAX_ITEMS];
    let n = legal_compose::selective_disclosure(&claims, &reveal, &mut disclosed);
    let instrument = args::rec_str(args_v, "instrument").map(q_hash).unwrap_or(0);
    Ok(args::record([
        ("disclosed_count", Value::U64(n as u64)),
        (
            "translation",
            Value::String(format!(
                "{:?}",
                legal_compose::translation_status(
                    args::rec_bool(args_v, "machine_proposed").unwrap_or(false),
                    args::rec_bool(args_v, "human_attested").unwrap_or(false),
                    args::rec_bool(args_v, "translatable").unwrap_or(true),
                )
            )),
        ),
        (
            "anchored",
            Value::Bool(legal_compose::anchored_to_instrument(instrument)),
        ),
        (
            "composition_valid",
            Value::Bool(legal_compose::composition_valid(
                instrument,
                args::rec_bool(args_v, "proportionate").unwrap_or(false),
            )),
        ),
    ]))
}

fn deontic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let agent = q_hash(args::rec_str(args_v, "agent").unwrap_or("agent"));
    let content = q_hash(args::rec_str(args_v, "content").unwrap_or("content"));
    let opcode = match args::rec_str(args_v, "opcode").unwrap_or("forbid") {
        "obligate" | "OBLIGATE" => OP_OBLIGATE,
        "forbid" | "FORBID" => OP_FORBID,
        other => {
            return Err(args::bad(
                span,
                format!("deontic compose opcode `{other}` is not supported"),
            ))
        }
    };
    let norm = compile_norm_quin(
        agent,
        opcode,
        q_hash("act"),
        content,
        q_hash("contract"),
        0,
        false,
    );
    let mut facts = Vec::new();
    if args::rec_bool(args_v, "brought_about").unwrap_or(false) {
        facts.push(fact(agent, q_hash("q42:broughtAbout"), content));
    }
    let mut epistemic = Vec::new();
    if args::rec_bool(args_v, "knows").unwrap_or(false) {
        epistemic.push(fact(agent, OP_KNOWS as u64, content));
    }
    let mens = deontic_compose::classify_mens_rea(
        &norm,
        &facts,
        &epistemic,
        args::rec_bool(args_v, "had_duty_to_know").unwrap_or(false),
    );
    let within = pair_hashes(args_v, "within", span)?;
    let tbox: Vec<NQuin> = within
        .iter()
        .map(|&(child, parent)| fact(child, q_hash("rdfs:subClassOf"), parent))
        .collect();
    let applies = deontic_compose::obligation_applies_in(
        q_hash(args::rec_str(args_v, "norm_jurisdiction").unwrap_or("au")),
        q_hash(args::rec_str(args_v, "target_jurisdiction").unwrap_or("au")),
        &tbox,
    );
    Ok(args::record([
        ("mens_rea", Value::String(format!("{mens:?}"))),
        ("applies_in_jurisdiction", Value::Bool(applies)),
        (
            "trust_gate",
            Value::Bool(deontic_compose::trust_gate(
                args::rec_f64(args_v, "trust").unwrap_or(1.0) as f32,
                args::rec_f64(args_v, "trust_threshold").unwrap_or(0.5) as f32,
            )),
        ),
        (
            "agent_knows",
            Value::Bool(deontic_compose::agent_knows(&epistemic, agent, content)),
        ),
        (
            "brought_about",
            Value::Bool(stit::brought_about(&facts, agent, content)),
        ),
    ]))
}

fn deontic_status(args_v: &Value, span: Span) -> Result<DeonticStatus, Diagnostic> {
    if let Some(status) = args::rec_str(args_v, "status") {
        return parse_status(status, span);
    }
    match args::rec_str(args_v, "requested_mode").unwrap_or("permit") {
        "waive" => Ok(DeonticStatus::Defeated),
        "permit" | "forbid" | "obligate" => Ok(DeonticStatus::Active),
        other => Err(args::bad(
            span,
            format!("interaction governance does not recognise mode `{other}`"),
        )),
    }
}

fn parse_status(status: &str, span: Span) -> Result<DeonticStatus, Diagnostic> {
    Ok(match status.to_ascii_lowercase().as_str() {
        "active" => DeonticStatus::Active,
        "defeated" => DeonticStatus::Defeated,
        "expired" => DeonticStatus::Expired,
        "malformed" => DeonticStatus::Malformed,
        "pending" => DeonticStatus::Pending,
        "violated" => DeonticStatus::Violated,
        "discharged" => DeonticStatus::Discharged,
        other => return Err(args::bad(span, format!("unknown deontic status `{other}`"))),
    })
}

fn names(args_v: &Value, key: &str, span: Span) -> Result<Vec<String>, Diagnostic> {
    args::rec_str_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("governance evaluation needs `{key}`")))
}

fn hashed_names(args_v: &Value, key: &str, _span: Span) -> Result<Vec<u64>, Diagnostic> {
    match args::rec_str_list(args_v, key) {
        Some(values) => Ok(values.iter().map(|name| q_hash(name)).collect()),
        None => Ok(Vec::new()),
    }
}

fn pair_hashes(args_v: &Value, key: &str, span: Span) -> Result<Vec<(u64, u64)>, Diagnostic> {
    let Some(values) = args::rec_str_list(args_v, key) else {
        return Ok(Vec::new());
    };
    if values.len() > MAX_ITEMS {
        return Err(args::bad(span, format!("`{key}` exceeds 64 pairs")));
    }
    values
        .iter()
        .map(|edge| {
            edge.split_once(':')
                .map(|(left, right)| (q_hash(left.trim()), q_hash(right.trim())))
                .ok_or_else(|| args::bad(span, format!("`{key}` entries use left:right")))
        })
        .collect()
}

fn parse_cost_edges(edges: &[String], span: Span) -> Result<Vec<(u64, u64, u32)>, Diagnostic> {
    if edges.len() > MAX_ITEMS {
        return Err(args::bad(span, "learning-path edges exceed 64"));
    }
    edges
        .iter()
        .map(|edge| {
            let mut parts = edge.split(':');
            let from = parts
                .next()
                .ok_or_else(|| args::bad(span, "edges use from:to:cost"))?;
            let to = parts
                .next()
                .ok_or_else(|| args::bad(span, "edges use from:to:cost"))?;
            let cost = parts
                .next()
                .and_then(|value| value.parse::<u32>().ok())
                .ok_or_else(|| args::bad(span, "edge cost must be an integer"))?;
            Ok((q_hash(from), q_hash(to), cost))
        })
        .collect()
}

fn need_u64(args_v: &Value, key: &str, span: Span) -> Result<u64, Diagnostic> {
    args::rec_u64(args_v, key).ok_or_else(|| args::bad(span, format!("needs `{key}`")))
}

fn fact(subject: u64, predicate: u64, object: u64) -> NQuin {
    let mut quin = NQuin {
        subject,
        predicate,
        object,
        context: q_hash("urn:poet:governance-workbench"),
        metadata: 0,
        parity: 0,
    };
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
    quin
}

const OP_KNOWS: u8 = crate::modalities::epistemic::OP_KNOWS;

#[cfg(test)]
#[path = "governance_workbench_tests.rs"]
mod tests;
