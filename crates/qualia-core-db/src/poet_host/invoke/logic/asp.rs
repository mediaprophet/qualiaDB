//! Answer-set evaluation for a bounded normal program, with legacy live-graph worlds.

use super::super::args;
use crate::modalities::asp::{
    brave_consequences, cautious_consequences, compute_answer_sets, enumerate_stable_models,
    optimal_answer_set, AspRule, WeakConstraint, ASP_MAX_ATOMS, MAX_STABLE_MODELS,
};
use crate::poet_host::PoetSnapshot;
use crate::q_hash;
use vibe::{Diagnostic, Span, Value};

pub fn evaluate(snap: &PoetSnapshot, args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let Some(source) = args::rec_str(args_v, "source") else {
        return enumerate_live(snap);
    };
    let operation = args::rec_str(args_v, "operation").unwrap_or("enumerate");
    let program = parse_program(source).map_err(|message| args::bad(span, message))?;
    let hashes = program
        .atoms
        .iter()
        .map(|atom| q_hash(atom))
        .collect::<Vec<_>>();
    let mut model_buf = [0u64; MAX_STABLE_MODELS];
    if operation == "optimal" {
        let best = optimal_answer_set(&hashes, &program.rules, &program.weak, &mut model_buf);
        return Ok(match best {
            Some((model, penalty)) => args::record([
                ("operation", Value::String(operation.into())),
                ("stable", Value::Bool(true)),
                ("penalty", Value::I64(penalty)),
                ("model", model_value(&program.atoms, model)),
            ]),
            None => args::record([
                ("operation", Value::String(operation.into())),
                ("stable", Value::Bool(false)),
                ("penalty", Value::Null),
                ("model", Value::Null),
            ]),
        });
    }
    if operation != "enumerate" {
        return Err(args::bad(
            span,
            "ASP operation must be `enumerate` or `optimal`",
        ));
    }
    let count = compute_answer_sets(&hashes, &program.rules, &mut model_buf);
    let models = model_buf[..count]
        .iter()
        .map(|model| model_value(&program.atoms, *model))
        .collect();
    Ok(args::record([
        ("operation", Value::String(operation.into())),
        ("stable_models", Value::U64(count as u64)),
        ("models", Value::List(models)),
        (
            "cautious",
            model_value(&program.atoms, cautious_consequences(&model_buf[..count])),
        ),
        (
            "brave",
            model_value(&program.atoms, brave_consequences(&model_buf[..count])),
        ),
    ]))
}

fn enumerate_live(snap: &PoetSnapshot) -> Result<Value, Diagnostic> {
    snap.with_live_quins(|quins| {
        let Some(base) = quins.first() else {
            return Ok(Value::List(Vec::new()));
        };
        let mut worlds = [0u64; MAX_STABLE_MODELS];
        let n = enumerate_stable_models(base, quins, &mut worlds);
        Ok(Value::List(
            worlds[..n].iter().copied().map(Value::U64).collect(),
        ))
    })
}

struct ParsedProgram {
    atoms: Vec<String>,
    rules: Vec<AspRule>,
    weak: Vec<WeakConstraint>,
}

fn parse_program(source: &str) -> Result<ParsedProgram, String> {
    let atoms_line = source
        .lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix("atoms=[")
                .and_then(|v| v.strip_suffix(']'))
        })
        .ok_or_else(|| "ASP input needs `atoms=[atom|atom]`".to_string())?;
    let atoms = atoms_line
        .split('|')
        .map(str::trim)
        .filter(|atom| !atom.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if atoms.is_empty() || atoms.len() > ASP_MAX_ATOMS {
        return Err(format!("ASP atoms must contain 1..={ASP_MAX_ATOMS} names"));
    }
    let atom_hash = |name: &str| {
        atoms
            .iter()
            .any(|atom| atom == name)
            .then(|| q_hash(name))
            .ok_or_else(|| format!("ASP atom `{name}` is not declared in atoms=[...]"))
    };
    let mut rules = Vec::new();
    let mut weak = Vec::new();
    for raw in source.lines().map(str::trim) {
        if raw.is_empty() || raw.starts_with('#') || raw.starts_with("atoms=") {
            continue;
        }
        if let Some(spec) = raw.strip_prefix("rule=") {
            let (head_text, body_text) = spec
                .split_once("<-")
                .ok_or_else(|| "ASP rule syntax is `rule=head <- body`".to_string())?;
            let head = if head_text.trim().is_empty() {
                0
            } else {
                atom_hash(head_text.trim())?
            };
            let (positive, negative) = parse_body(body_text, &atom_hash)?;
            rules.push(AspRule::new(head, &positive, &negative));
        } else if let Some(spec) = raw.strip_prefix("fact=") {
            rules.push(AspRule::fact(atom_hash(spec.trim())?));
        } else if let Some(spec) = raw.strip_prefix("weak=") {
            let (body, weight) = spec
                .rsplit_once(':')
                .ok_or_else(|| "ASP weak syntax is `weak=body : weight`".to_string())?;
            let weight = weight
                .trim()
                .parse::<i64>()
                .map_err(|_| "ASP weak-constraint weight must be an integer".to_string())?;
            let (positive, negative) = parse_body(body, &atom_hash)?;
            weak.push(WeakConstraint::new(&positive, &negative, weight));
        } else {
            return Err(format!("unsupported ASP input line `{raw}`"));
        }
    }
    if rules.is_empty() {
        return Err("ASP input needs at least one `fact=` or `rule=` line".into());
    }
    Ok(ParsedProgram { atoms, rules, weak })
}

fn parse_body(
    body: &str,
    atom_hash: &impl Fn(&str) -> Result<u64, String>,
) -> Result<(Vec<u64>, Vec<u64>), String> {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for term in body.split(',').map(str::trim).filter(|v| !v.is_empty()) {
        if let Some(atom) = term.strip_prefix("not ") {
            negative.push(atom_hash(atom.trim())?);
        } else {
            positive.push(atom_hash(term)?);
        }
    }
    Ok((positive, negative))
}

fn model_value(atoms: &[String], model: u64) -> Value {
    Value::List(
        atoms
            .iter()
            .enumerate()
            .filter(|(index, _)| model & (1u64 << index) != 0)
            .map(|(_, atom)| Value::String(atom.clone()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_even_loop_and_selects_weak_constraint_optimum() {
        let source = "atoms=[permit|forbid]\nrule=permit <- not forbid\nrule=forbid <- not permit\nweak=forbid : 1";
        let program = parse_program(source).unwrap();
        let hashes = program.atoms.iter().map(|a| q_hash(a)).collect::<Vec<_>>();
        let mut models = [0u64; MAX_STABLE_MODELS];
        assert_eq!(compute_answer_sets(&hashes, &program.rules, &mut models), 2);
        let (best, penalty) =
            optimal_answer_set(&hashes, &program.rules, &program.weak, &mut models).unwrap();
        assert_eq!(
            model_value(&program.atoms, best),
            Value::List(vec![Value::String("permit".into())])
        );
        assert_eq!(penalty, 0);
    }
}
