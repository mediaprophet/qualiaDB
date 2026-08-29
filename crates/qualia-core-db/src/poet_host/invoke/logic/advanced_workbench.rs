//! Bounded adapters for POET's advanced-logic workbench panels.
//!
//! Cold invocation/result assembly delegates to fixed-size/zero-heap evaluators.
use super::super::args;
use crate::{q_hash, NQuin};
use vibe::{Diagnostic, Span, Value};

const MAX_ITEMS: usize = 128;
pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mode = args::rec_str(args_v, "mode")
        .ok_or_else(|| args::bad(span, "AdvancedLogic.compute needs mode"))?;
    match mode {
        "abductive" => abductive(args_v, span),
        "fuzzy" => fuzzy(args_v, span),
        "probabilistic" => probabilistic(args_v, span),
        "graph" => graph(args_v, span),
        "interval" => interval(args_v, span),
        "manifold_10d" => manifold_10d(args_v, span),
        "epistemic_boundaries" => epistemic_boundaries(args_v, span),
        "modal" => modal(args_v, span),
        _ => Err(args::bad(
            span,
            format!("unknown advanced-logic mode `{mode}`"),
        )),
    }
}

fn unit(v: f64, key: &str, span: Span) -> Result<f32, Diagnostic> {
    if v.is_finite() && (0.0..=1.0).contains(&v) {
        Ok(v as f32)
    } else {
        Err(args::bad(
            span,
            format!("{key} must be finite and in [0,1]"),
        ))
    }
}

fn abductive(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::abductive::{bayesian_posteriors, best_hypothesis, Hypothesis};
    let values = args::rec(args_v, "hypotheses")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "abductive mode needs hypotheses"))?;
    if values.is_empty() || values.len() > 32 {
        return Err(args::bad(span, "hypotheses must contain 1..=32 candidates"));
    }
    let mut hypotheses = [Hypothesis {
        id: 0,
        prior: 0.0,
        likelihood: 0.0,
    }; 32];
    let mut names = [""; 32];
    for (index, value) in values.iter().enumerate() {
        let name = args::rec_str(value, "id")
            .ok_or_else(|| args::bad(span, "each hypothesis needs id"))?;
        let prior = unit(
            args::rec_f64(value, "prior")
                .ok_or_else(|| args::bad(span, "each hypothesis needs prior"))?,
            "prior",
            span,
        )?;
        let likelihood = unit(
            args::rec_f64(value, "likelihood")
                .ok_or_else(|| args::bad(span, "each hypothesis needs likelihood"))?,
            "likelihood",
            span,
        )?;
        names[index] = name;
        hypotheses[index] = Hypothesis {
            id: q_hash(name),
            prior,
            likelihood,
        };
    }
    let count = values.len();
    let mut posteriors = [0.0f32; 32];
    let evidence = bayesian_posteriors(&hypotheses[..count], &mut posteriors[..count]);
    let best = best_hypothesis(&hypotheses[..count]);
    let ranked = (0..count)
        .map(|index| {
            args::record([
                ("id", Value::String(names[index].into())),
                ("hash", Value::U64(hypotheses[index].id)),
                ("posterior", Value::F64(posteriors[index] as f64)),
                ("best", Value::Bool(best == Some(hypotheses[index].id))),
            ])
        })
        .collect();
    Ok(args::record([
        ("mode", Value::String("bayesian-abduction".into())),
        ("evidence", Value::F64(evidence as f64)),
        ("hypotheses", Value::List(ranked)),
    ]))
}

fn fuzzy(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::fuzzy::{
        t_conorm_drastic, t_conorm_godel, t_conorm_lukasiewicz, t_conorm_product, t_norm_drastic,
        t_norm_godel, t_norm_lukasiewicz, t_norm_product,
    };
    let operation = args::rec_str(args_v, "operation").unwrap_or("godel_and");
    let a = unit(
        args::rec_f64(args_v, "a").ok_or_else(|| args::bad(span, "fuzzy mode needs a"))?,
        "a",
        span,
    )?;
    let b = unit(
        args::rec_f64(args_v, "b").ok_or_else(|| args::bad(span, "fuzzy mode needs b"))?,
        "b",
        span,
    )?;
    let result = match operation {
        "godel_and" => t_norm_godel(a, b),
        "godel_or" => t_conorm_godel(a, b),
        "lukasiewicz_and" => t_norm_lukasiewicz(a, b),
        "lukasiewicz_or" => t_conorm_lukasiewicz(a, b),
        "product_and" => t_norm_product(a, b),
        "product_or" => t_conorm_product(a, b),
        "drastic_and" => t_norm_drastic(a, b),
        "drastic_or" => t_conorm_drastic(a, b),
        _ => {
            return Err(args::bad(
                span,
                format!("unknown fuzzy operation `{operation}`"),
            ));
        }
    };
    Ok(args::record([
        ("operation", Value::String(operation.into())),
        ("truth_degree", Value::F64(result as f64)),
    ]))
}

fn probabilistic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::abductive::{bayesian_posteriors, Hypothesis};
    use crate::modalities::evaluate_threshold;
    let prior = unit(
        args::rec_f64(args_v, "prior")
            .ok_or_else(|| args::bad(span, "probabilistic mode needs prior"))?,
        "prior",
        span,
    )?;
    let positive = unit(
        args::rec_f64(args_v, "likelihood_true")
            .ok_or_else(|| args::bad(span, "probabilistic mode needs likelihood_true"))?,
        "likelihood_true",
        span,
    )?;
    let negative = unit(
        args::rec_f64(args_v, "likelihood_false")
            .ok_or_else(|| args::bad(span, "probabilistic mode needs likelihood_false"))?,
        "likelihood_false",
        span,
    )?;
    let threshold = unit(
        args::rec_f64(args_v, "threshold").unwrap_or(0.5),
        "threshold",
        span,
    )?;
    let hypotheses = [
        Hypothesis {
            id: 1,
            prior,
            likelihood: positive,
        },
        Hypothesis {
            id: 0,
            prior: 1.0 - prior,
            likelihood: negative,
        },
    ];
    let mut posterior = [0.0; 2];
    let evidence = bayesian_posteriors(&hypotheses, &mut posterior);
    Ok(args::record([
        ("posterior_true", Value::F64(posterior[0] as f64)),
        ("posterior_false", Value::F64(posterior[1] as f64)),
        ("evidence", Value::F64(evidence as f64)),
        (
            "meets_threshold",
            Value::Bool(evaluate_threshold(posterior[0], threshold)),
        ),
    ]))
}

fn graph(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::graph_theory::analyze_graph_topology;
    let edges = args::rec(args_v, "edges")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "graph mode needs edges"))?;
    if edges.is_empty() || edges.len() > MAX_ITEMS {
        return Err(args::bad(span, "edges must contain 1..=128 pairs"));
    }
    let mut quins = Vec::with_capacity(edges.len());
    for edge in edges {
        let pair =
            args::list(edge).ok_or_else(|| args::bad(span, "each edge must be [from,to]"))?;
        if pair.len() != 2 {
            return Err(args::bad(span, "each edge must contain exactly two nodes"));
        }
        let from =
            args::as_str(&pair[0]).ok_or_else(|| args::bad(span, "edge nodes must be names"))?;
        let to =
            args::as_str(&pair[1]).ok_or_else(|| args::bad(span, "edge nodes must be names"))?;
        let mut quin = NQuin {
            subject: q_hash(from),
            predicate: q_hash("q42:edge"),
            object: q_hash(to),
            context: q_hash("urn:poet:graph-workbench"),
            metadata: 0,
            parity: 0,
        };
        quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
        quins.push(quin);
    }
    let result = analyze_graph_topology(&quins, q_hash("urn:poet:graph-analysis"))
        .map_err(|error| args::bad(span, format!("graph analysis failed: {error:?}")))?;
    let top_nodes = result
        .top_nodes
        .into_iter()
        .map(|(id, score)| {
            args::record([
                ("node_hash", Value::U64(id)),
                ("centrality", Value::F64(score)),
            ])
        })
        .collect();
    Ok(args::record([
        ("nodes", Value::U64(result.node_count as u64)),
        ("edges", Value::U64(result.edge_count as u64)),
        ("density", Value::F64(result.density)),
        ("communities", Value::U64(result.communities.len() as u64)),
        ("motifs", Value::U64(result.motifs.len() as u64)),
        ("top_nodes", Value::List(top_nodes)),
    ]))
}

fn interval(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = interval_pair(args_v, "a", span)?;
    let b = interval_pair(args_v, "b", span)?;
    let relation = if a == b {
        "Equal"
    } else if a[1] < b[0] {
        "Before"
    } else if a[0] > b[1] {
        "After"
    } else if a[1] == b[0] {
        "Meets"
    } else if a[0] == b[1] {
        "MetBy"
    } else if a[0] == b[0] && a[1] < b[1] {
        "Starts"
    } else if a[0] == b[0] {
        "StartedBy"
    } else if a[1] == b[1] && a[0] > b[0] {
        "Ends"
    } else if a[1] == b[1] {
        "EndedBy"
    } else if a[0] > b[0] && a[1] < b[1] {
        "During"
    } else if a[0] < b[0] && a[1] > b[1] {
        "Contains"
    } else if a[0] < b[0] {
        "Overlaps"
    } else {
        "OverlappedBy"
    };
    Ok(args::record([
        ("allen_relation", Value::String(relation.into())),
        (
            "minkowski_sum",
            Value::List(vec![Value::I64(a[0] + b[0]), Value::I64(a[1] + b[1])]),
        ),
        (
            "intersection",
            Value::List(vec![Value::I64(a[0].max(b[0])), Value::I64(a[1].min(b[1]))]),
        ),
        ("intersects", Value::Bool(a[0] <= b[1] && b[0] <= a[1])),
    ]))
}

fn interval_pair(args_v: &Value, key: &str, span: Span) -> Result<[i64; 2], Diagnostic> {
    let values = args::rec(args_v, key)
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, format!("interval mode needs {key}: [start,end]")))?;
    let start = values
        .first()
        .and_then(args::as_i64)
        .ok_or_else(|| args::bad(span, format!("{key} needs integer bounds")))?;
    let end = values
        .get(1)
        .and_then(args::as_i64)
        .ok_or_else(|| args::bad(span, format!("{key} needs integer bounds")))?;
    if values.len() != 2 || start > end {
        return Err(args::bad(
            span,
            format!("{key} must be [start,end] with start <= end"),
        ));
    }
    Ok([start, end])
}

fn manifold_10d(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let values = args::rec_f64_list(args_v, "parameters")
        .ok_or_else(|| args::bad(span, "manifold_10d mode needs 10 parameters"))?;
    let parameters: [f64; 10] = values
        .try_into()
        .map_err(|_| args::bad(span, "parameters must contain exactly 10 numbers"))?;
    if !parameters.iter().all(|value| value.is_finite()) {
        return Err(args::bad(span, "parameters must be finite"));
    }
    let quaternion = crate::modalities::manifold::project_10d_to_quaternion(&parameters)
        .map_err(|_| args::bad(span, "10D quaternion projection did not converge"))?;
    Ok(args::record([
        (
            "projection",
            Value::String("smallest-eigenvector quaternion".into()),
        ),
        ("quaternion", args::f64_list_value(quaternion.data)),
    ]))
}

fn epistemic_boundaries(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::epistemic_boundaries::{
        degrade_claim_to_socratic, detect_referral_by_severity, forbids_definitive_classification,
        identify_degradation_vector, requires_physiological_quarantine,
    };
    let predicate = args::rec_str(args_v, "predicate")
        .ok_or_else(|| args::bad(span, "epistemic-boundaries mode needs predicate"))?;
    let severity = args::rec_u64(args_v, "severity").unwrap_or(0);
    if severity > 255 {
        return Err(args::bad(span, "severity must be in 0..=255"));
    }
    let mut quin = NQuin {
        subject: q_hash(args::rec_str(args_v, "subject").unwrap_or("urn:poet:claim")),
        predicate: q_hash(predicate),
        object: 0,
        context: q_hash("urn:poet:epistemic-boundary"),
        metadata: severity,
        parity: 0,
    };
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
    let vector = identify_degradation_vector(&quin);
    let referral = detect_referral_by_severity(vector, quin.metadata);
    let degradation = degrade_claim_to_socratic(vector);
    Ok(args::record([
        ("vector", Value::String(format!("{vector:?}"))),
        (
            "forbids_definitive_classification",
            Value::Bool(forbids_definitive_classification(quin.predicate)),
        ),
        (
            "requires_private_quarantine",
            Value::Bool(requires_physiological_quarantine(&quin)),
        ),
        ("overriding_referral", Value::Bool(referral.is_some())),
        (
            "prompt",
            Value::String(
                referral
                    .map(|r| r.overriding_prompt)
                    .or_else(|| degradation.map(|d| d.socratic_prompt))
                    .unwrap_or("No degradation rule matched this predicate.")
                    .into(),
            ),
        ),
        (
            "disclaimer",
            Value::String(
                referral
                    .map(|r| r.immutable_disclaimer)
                    .or_else(|| degradation.map(|d| d.immutable_disclaimer))
                    .unwrap_or("No domain-specific disclaimer applies.")
                    .into(),
            ),
        ),
    ]))
}

fn modal(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::modal::{necessary, possible, validates, ModalSystem};
    let system_name = args::rec_str(args_v, "system").unwrap_or("K");
    let operator = args::rec_str(args_v, "operator").unwrap_or("necessary");
    let world_name =
        args::rec_str(args_v, "world").ok_or_else(|| args::bad(span, "modal mode needs world"))?;
    let prop_name = args::rec_str(args_v, "proposition")
        .ok_or_else(|| args::bad(span, "modal mode needs proposition"))?;
    let world_values = args::rec(args_v, "worlds")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "modal mode needs worlds"))?;
    if world_values.is_empty() || world_values.len() > 16 {
        return Err(args::bad(span, "worlds must contain 1..=16 names"));
    }
    let mut worlds = [0u64; 16];
    for (index, value) in world_values.iter().enumerate() {
        worlds[index] =
            q_hash(args::as_str(value).ok_or_else(|| args::bad(span, "worlds must be names"))?);
    }
    let accesses = q_hash("q42:accesses");
    let holds = q_hash("q42:holds");
    let mut graph = [NQuin::default(); 64];
    let mut count = 0usize;
    for value in args::rec(args_v, "accesses")
        .and_then(args::list)
        .unwrap_or(&[])
    {
        let pair = args::list(value)
            .ok_or_else(|| args::bad(span, "accesses entries must be [from,to]"))?;
        if pair.len() != 2 || count >= graph.len() {
            return Err(args::bad(span, "accesses must contain at most 64 pairs"));
        }
        graph[count] = NQuin {
            subject: q_hash(
                args::as_str(&pair[0])
                    .ok_or_else(|| args::bad(span, "access worlds must be names"))?,
            ),
            predicate: accesses,
            object: q_hash(
                args::as_str(&pair[1])
                    .ok_or_else(|| args::bad(span, "access worlds must be names"))?,
            ),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        count += 1;
    }
    for value in args::rec(args_v, "holds_in")
        .and_then(args::list)
        .unwrap_or(&[])
    {
        if count >= graph.len() {
            return Err(args::bad(span, "modal frame exceeds 64 facts"));
        }
        graph[count] = NQuin {
            subject: q_hash(
                args::as_str(value)
                    .ok_or_else(|| args::bad(span, "holds_in must contain world names"))?,
            ),
            predicate: holds,
            object: q_hash(prop_name),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        count += 1;
    }
    let system = match system_name.to_ascii_uppercase().as_str() {
        "K" => ModalSystem::K,
        "T" => ModalSystem::T,
        "D" => ModalSystem::D,
        "B" => ModalSystem::B,
        "S4" => ModalSystem::S4,
        "S5" => ModalSystem::S5,
        _ => {
            return Err(args::bad(
                span,
                format!("unknown modal system `{system_name}`"),
            ));
        }
    };
    let truth = match operator {
        "necessary" | "box" => necessary(
            &graph[..count],
            q_hash(world_name),
            q_hash(prop_name),
            accesses,
            holds,
        ),
        "possible" | "diamond" => possible(
            &graph[..count],
            q_hash(world_name),
            q_hash(prop_name),
            accesses,
            holds,
        ),
        _ => return Err(args::bad(span, "operator must be necessary or possible")),
    };
    Ok(args::record([
        ("system", Value::String(system_name.to_ascii_uppercase())),
        (
            "frame_validates_system",
            Value::Bool(validates(
                system,
                &graph[..count],
                accesses,
                &worlds[..world_values.len()],
            )),
        ),
        ("operator", Value::String(operator.into())),
        ("truth", Value::Bool(truth)),
        ("world_count", Value::U64(world_values.len() as u64)),
    ]))
}

#[cfg(test)]
#[path = "advanced_workbench_tests.rs"]
mod tests;
