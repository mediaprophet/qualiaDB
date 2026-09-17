//! Bounded adapters for POET's legal-logic workbench surfaces.

use super::super::args;
use crate::modalities::logic::deontic::{
    compile_norm_quin, DeonticStatus, DeonticVerdict, OP_OBLIGATE,
};
use crate::modalities::{
    argumentation::{Argument, ArgumentationFramework, Attack, AttackType},
    capacity::{
        authorized_after_revocation, capacity_under_pressure, delegation_attenuates,
        effective_principal, posthumous_standing, stipulation_binding, stipulation_voidable,
        CapacityStatus,
    },
    consensus, contract,
    jural::{is_jural_position, jural_correlativity_holds, jural_position, position_name},
    meta_deontic, responsibility, stit,
};
use crate::poet_host::PoetSnapshot;
use crate::q_hash;
use vibe::{Diagnostic, Span, Value};

pub fn compute(snap: &PoetSnapshot, args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "mode") {
        Some("jural") => jural(snap, args_v),
        Some("stit") => stit_eval(args_v),
        Some("causal") => causal(args_v, span),
        Some("responsibility") => responsibility_eval(args_v),
        Some("capacity") => capacity(args_v, span),
        Some("delegation") => delegation(args_v, span),
        Some("contract") => contract_eval(args_v, span),
        Some("consensus") => consensus_eval(args_v),
        Some("meta_deontic") => meta_deontic_eval(args_v, span),
        Some("argumentation") => argumentation(args_v, span),
        _ => Err(args::bad(
            span,
            "LegalLogic.compute needs a supported `mode`",
        )),
    }
}

fn jural(snap: &PoetSnapshot, args_v: &Value) -> Result<Value, Diagnostic> {
    let role = args::rec_str(args_v, "role").unwrap_or("principal");
    let holder = q_hash(role);
    snap.with_live_quins(|quins| {
        let relations = quins
            .iter()
            .copied()
            .filter(|q| q.subject == holder && is_jural_position(jural_position(q.predicate)))
            .collect::<Vec<_>>();
        let unmet = relations
            .iter()
            .filter(|relation| !jural_correlativity_holds(relation, quins))
            .count();
        let positions = relations
            .iter()
            .filter_map(|q| position_name(jural_position(q.predicate)))
            .map(|name| Value::String(name.into()))
            .collect();
        Ok(args::record([
            ("role", Value::String(role.into())),
            ("relations", Value::U64(relations.len() as u64)),
            ("unmet_correlatives", Value::U64(unmet as u64)),
            ("positions", Value::List(positions)),
        ]))
    })
}

fn stit_eval(args_v: &Value) -> Result<Value, Diagnostic> {
    let brought = args::rec_bool(args_v, "brought_about").unwrap_or(false);
    let alternative = args::rec_bool(args_v, "could_do_otherwise").unwrap_or(false);
    let members = args::rec_str_list(args_v, "members").unwrap_or_default();
    let joint_acted = args::rec_bool(args_v, "joint_acted").unwrap_or(false);
    let liable = if joint_acted { 0 } else { members.len() };
    Ok(args::record([
        ("chellas_stit", Value::Bool(stit::chellas_stit(brought))),
        (
            "deliberative_stit",
            Value::Bool(stit::deliberative_stit(brought, alternative)),
        ),
        ("joint_discharged", Value::Bool(joint_acted)),
        ("joint_liable_members", Value::U64(liable as u64)),
    ]))
}

fn causal(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let edges = args::rec_str_list(args_v, "edges")
        .ok_or_else(|| args::bad(span, "causal evaluation needs `edges`"))?;
    if edges.len() > 64 {
        return Err(args::bad(span, "causal graph exceeds 64 edges"));
    }
    let target = args::rec_str(args_v, "target")
        .ok_or_else(|| args::bad(span, "causal evaluation needs `target`"))?;
    let pairs = edges
        .iter()
        .map(|edge| {
            edge.split_once(':')
                .map(|(a, b)| (a.trim(), b.trim()))
                .ok_or_else(|| args::bad(span, "causal edges use source:target"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut roots = Vec::new();
    for (from, _) in &pairs {
        if !pairs.iter().any(|(_, to)| to == from) && !roots.contains(from) {
            roots.push(*from);
        }
    }
    let causes = roots
        .iter()
        .filter(|root| reaches(root, target, &pairs))
        .map(|root| Value::String((*root).into()))
        .collect::<Vec<_>>();
    Ok(args::record([
        ("target", Value::String(target.into())),
        ("but_for_roots", Value::List(causes.clone())),
        ("overdetermined", Value::Bool(causes.len() > 1)),
        ("independent_causes", Value::U64(causes.len() as u64)),
    ]))
}

fn reaches(start: &str, target: &str, edges: &[(&str, &str)]) -> bool {
    let mut frontier = vec![start];
    let mut seen = Vec::new();
    while let Some(node) = frontier.pop() {
        if node == target {
            return true;
        }
        if seen.contains(&node) {
            continue;
        }
        seen.push(node);
        for (_, to) in edges.iter().filter(|(from, _)| *from == node) {
            frontier.push(*to);
        }
    }
    false
}

fn responsibility_eval(args_v: &Value) -> Result<Value, Diagnostic> {
    let status = responsibility::adjudicate(
        args::rec_bool(args_v, "confirmed").unwrap_or(false),
        args::rec_bool(args_v, "dismissed").unwrap_or(false),
    );
    let vacuum = responsibility::accountability_vacuum(
        args::rec_bool(args_v, "harm_occurred").unwrap_or(false),
        args::rec_bool(args_v, "accountable_person").unwrap_or(false),
    );
    Ok(args::record([
        ("status", Value::String(format!("{status:?}"))),
        (
            "enforceable",
            Value::Bool(responsibility::is_enforceable_fact(status)),
        ),
        ("accountability_vacuum", Value::Bool(vacuum)),
    ]))
}

fn capacity(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let base = capacity_status(args::rec_str(args_v, "status").unwrap_or("intact"), span)?;
    let status = capacity_under_pressure(
        base,
        args::rec_f64(args_v, "imbalance").unwrap_or(0.0) as f32,
        args::rec_bool(args_v, "explicit_threat").unwrap_or(false),
        args::rec_f64(args_v, "duress_threshold").unwrap_or(0.7) as f32,
    );
    let actor = q_hash(args::rec_str(args_v, "agent").unwrap_or("agent"));
    let dependent = q_hash(args::rec_str(args_v, "dependent").unwrap_or("dependent"));
    Ok(args::record([
        ("status", Value::String(format!("{status:?}"))),
        ("binding", Value::Bool(stipulation_binding(status))),
        ("voidable", Value::Bool(stipulation_voidable(status))),
        (
            "effective_principal",
            Value::U64(effective_principal(
                actor,
                dependent,
                args::rec_bool(args_v, "guardianship").unwrap_or(false),
            )),
        ),
        (
            "posthumous_standing",
            Value::Bool(posthumous_standing(
                args::rec_bool(args_v, "deceased").unwrap_or(false),
                args::rec_bool(args_v, "representative").unwrap_or(false),
            )),
        ),
    ]))
}

fn delegation(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let parent = hash_list(args_v, "parent_domains", span)?;
    let child = hash_list(args_v, "child_domains", span)?;
    let revoked = hash_list(args_v, "revoked_domains", span)?;
    let requested = q_hash(
        args::rec_str(args_v, "requested_domain")
            .ok_or_else(|| args::bad(span, "delegation needs `requested_domain`"))?,
    );
    Ok(args::record([
        (
            "attenuates",
            Value::Bool(delegation_attenuates(&parent, &child)),
        ),
        (
            "authorized_after_revocation",
            Value::Bool(authorized_after_revocation(&child, &revoked, requested)),
        ),
    ]))
}

fn contract_eval(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let offeror = capacity_status(
        args::rec_str(args_v, "offeror_capacity").unwrap_or("intact"),
        span,
    )?;
    let acceptor = capacity_status(
        args::rec_str(args_v, "acceptor_capacity").unwrap_or("intact"),
        span,
    )?;
    let stipulated = args::rec_bool(args_v, "stipulated").unwrap_or(false);
    let accepted = args::rec_bool(args_v, "accepted").unwrap_or(false);
    Ok(args::record([
        (
            "formation_stage",
            Value::String(format!(
                "{:?}",
                contract::formation_stage(stipulated, accepted)
            )),
        ),
        (
            "binding",
            Value::Bool(contract::is_binding_contract(
                stipulated, accepted, offeror, acceptor,
            )),
        ),
        (
            "incorporates_by_reference",
            Value::Bool(contract::incorporates_by_reference(
                args::rec_str(args_v, "instrument").map(q_hash).unwrap_or(0),
            )),
        ),
    ]))
}

fn consensus_eval(args_v: &Value) -> Result<Value, Diagnostic> {
    let votes = args::rec_u64(args_v, "votes").unwrap_or(0) as usize;
    let parties = args::rec_u64(args_v, "parties").unwrap_or(0) as usize;
    let partitioned = args::rec_bool(args_v, "partitioned").unwrap_or(false);
    Ok(args::record([
        (
            "transaction_status",
            Value::String(format!(
                "{:?}",
                consensus::transaction_status(votes, parties)
            )),
        ),
        (
            "bft_quorum",
            Value::U64(consensus::bft_quorum(parties) as u64),
        ),
        (
            "bft_committed",
            Value::Bool(consensus::bft_committed(parties, votes)),
        ),
        (
            "can_form_joint",
            Value::Bool(consensus::can_form_joint_during_partition(partitioned)),
        ),
    ]))
}

fn meta_deontic_eval(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let actor = args::rec_str(args_v, "actor")
        .ok_or_else(|| args::bad(span, "breach record needs `actor`"))?;
    let action = args::rec_str(args_v, "action")
        .ok_or_else(|| args::bad(span, "breach record needs `action`"))?;
    let instrument = q_hash(args::rec_str(args_v, "instrument").unwrap_or("instrument"));
    let norm = compile_norm_quin(
        q_hash(actor),
        OP_OBLIGATE,
        q_hash(action),
        q_hash(action),
        instrument,
        0,
        false,
    );
    let mut verdict = DeonticVerdict::default();
    verdict.norm = norm;
    verdict.opcode = OP_OBLIGATE;
    verdict.status = DeonticStatus::Violated;
    let record = meta_deontic::build_breach_record(
        &verdict,
        instrument,
        args::rec_u64(args_v, "now").unwrap_or(0) as u32,
    )
    .ok_or_else(|| args::bad(span, "breach record was not a violation"))?;
    Ok(args::record([
        ("built", Value::Bool(true)),
        ("record_subject", Value::U64(record.subject)),
        ("record_predicate", Value::U64(record.predicate)),
        (
            "provenance",
            Value::U64(meta_deontic::breach_provenance(&record)),
        ),
        (
            "endorsement_requires_signature",
            Value::Bool(args::rec_bool(args_v, "endorse").unwrap_or(false)),
        ),
    ]))
}

fn argumentation(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let names = args::rec_str_list(args_v, "arguments")
        .ok_or_else(|| args::bad(span, "argumentation needs `arguments`"))?;
    if names.len() > 32 {
        return Err(args::bad(span, "argumentation is bounded to 32 arguments"));
    }
    let attacks = args::rec_str_list(args_v, "attacks").unwrap_or_default();
    let mut framework = ArgumentationFramework::new();
    for name in &names {
        framework.add_argument(Argument::new(
            q_hash(name),
            name.clone(),
            Vec::new(),
            crate::NQuin::default(),
        ));
    }
    for edge in &attacks {
        let (from, to) = edge
            .split_once(':')
            .ok_or_else(|| args::bad(span, "argument attacks use attacker:target"))?;
        framework.add_attack(Attack {
            attacker: q_hash(from.trim()),
            target: q_hash(to.trim()),
            attack_type: AttackType::Rebuttal,
            strength: 1.0,
        });
    }
    let semantics = args::rec_str(args_v, "semantics").unwrap_or("grounded");
    let extensions = match semantics {
        "grounded" => vec![framework.grounded_extension()],
        "preferred" => framework.preferred_extensions(),
        "stable" => framework.stable_extensions(),
        "complete" => framework.complete_extensions(),
        _ => return Err(args::bad(span, "unknown argumentation semantics")),
    };
    let rendered = extensions
        .iter()
        .map(|set| {
            Value::List(
                names
                    .iter()
                    .filter(|name| set.contains(&q_hash(name)))
                    .cloned()
                    .map(Value::String)
                    .collect(),
            )
        })
        .collect();
    Ok(args::record([
        ("semantics", Value::String(semantics.into())),
        ("extensions", Value::List(rendered)),
        ("nodes", Value::U64(names.len() as u64)),
        ("edges", Value::U64(attacks.len() as u64)),
    ]))
}

fn capacity_status(value: &str, span: Span) -> Result<CapacityStatus, Diagnostic> {
    match value.to_ascii_lowercase().as_str() {
        "intact" | "full" => Ok(CapacityStatus::Intact),
        "impaired" | "limited" => Ok(CapacityStatus::Impaired),
        "duress" | "under_duress" => Ok(CapacityStatus::UnderDuress),
        _ => Err(args::bad(
            span,
            "capacity must be intact, impaired, or duress",
        )),
    }
}

fn hash_list(args_v: &Value, key: &str, span: Span) -> Result<Vec<u64>, Diagnostic> {
    args::rec_str_list(args_v, key)
        .map(|values| values.iter().map(|value| q_hash(value)).collect())
        .ok_or_else(|| args::bad(span, format!("delegation needs `{key}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_contract_and_argumentation_use_native_cores() {
        let contract_args = args::record([
            ("mode", Value::String("contract".into())),
            ("stipulated", Value::Bool(true)),
            ("accepted", Value::Bool(true)),
            ("offeror_capacity", Value::String("intact".into())),
            ("acceptor_capacity", Value::String("intact".into())),
        ]);
        let Value::Record(result) =
            compute(&PoetSnapshot::default(), &contract_args, Span::new(0, 0)).unwrap()
        else {
            panic!("expected record")
        };
        assert_eq!(result.get("binding"), Some(&Value::Bool(true)));

        let argument_args = args::record([
            ("mode", Value::String("argumentation".into())),
            ("semantics", Value::String("grounded".into())),
            (
                "arguments",
                Value::List(
                    ["a1", "a2", "a3"]
                        .into_iter()
                        .map(|name| Value::String(name.into()))
                        .collect(),
                ),
            ),
            (
                "attacks",
                Value::List(
                    ["a2:a1", "a3:a2"]
                        .into_iter()
                        .map(|edge| Value::String(edge.into()))
                        .collect(),
                ),
            ),
        ]);
        let Value::Record(result) =
            compute(&PoetSnapshot::default(), &argument_args, Span::new(0, 0)).unwrap()
        else {
            panic!("expected record")
        };
        assert_eq!(result.get("nodes"), Some(&Value::U64(3)));
        assert_eq!(result.get("edges"), Some(&Value::U64(2)));
    }
}
