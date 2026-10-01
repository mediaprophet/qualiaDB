//! LocalHost modal-logic kernels (UE-060).
//!
//! Pure quin-array evaluators so Civics / vibe-wasm can exercise deontic,
//! epistemic, paraconsistent, LTL, and DL subsumption without Poet. Layout
//! matches `qualia-core-db` modality opcodes (AGENTS.md). Workshop terms
//! (`obligate { … }`) compile to a single Active norm when no graph is supplied.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

const OP_OBLIGATE: u8 = 0x10;
const OP_PERMIT: u8 = 0x11;
const OP_FORBID: u8 = 0x12;
const OP_KNOWS: u8 = 0x20;
const OP_BELIEVES: u8 = 0x21;
const OP_COMMON_KNOWLEDGE: u8 = 0x22;
const DEFEATER_BIT: u64 = 1u64 << 63;
const CERTAINTY_SHIFT: u32 = 8;
const MAX_DEFEATERS: usize = 64;
const MAX_VERDICTS: usize = 64;

#[derive(Clone, Copy, Default)]
struct Quin {
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
    parity: u64,
}

impl Quin {
    fn from_arr(a: &[u64; 6]) -> Self {
        Self {
            subject: a[0],
            predicate: a[1],
            object: a[2],
            context: a[3],
            metadata: a[4],
            parity: a[5],
        }
    }

    fn with_parity(mut self) -> Self {
        self.parity = self.subject ^ self.predicate ^ self.object ^ self.context;
        self
    }
}

/// Attempt a modal-logic LocalHost invoke. `None` when `id` is not handled here.
pub fn try_invoke(id: &str, args: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    match id {
        "DeonticLogic.evaluate" => Some(deontic_evaluate(args, span)),
        "EpistemicLogic.evaluate" => Some(epistemic_evaluate(args, span)),
        "ParaconsistentLogic.route" => Some(paraconsistent_route(args, span)),
        "TemporalAndDescriptionLogic.ltl.globally"
        | "TemporalAndDescriptionLogic.ltl.finally"
        | "TemporalAndDescriptionLogic.ltl.evaluate" => Some(ltl_evaluate(id, args, span)),
        "TemporalAndDescriptionLogic.subsumption" => Some(subsumption(args, span)),
        "CausalFuzzyAndControl.caused" => Some(causal_caused(args, span)),
        "CausalFuzzyAndControl.t_norm" => Some(fuzzy_t_norm(args, span)),
        "SHACL.validate" => Some(shacl_validate_local(args, span)),
        "SHACL.extensions" => Some(Ok(shacl_extensions())),
        "N3Logic.evaluate" | "SymbolicAndDefeasibleLogic.asp" => Some(engine_required(id, span)),
        _ => None,
    }
}

fn engine_required(id: &str, span: Span) -> Result<Value, Diagnostic> {
    Err(Diagnostic::new(
        DiagCode::E300,
        span,
        format!(
            "{id} on LocalHost requires Qualia engine host (Poet) or WASM \
             (forward_chain_wasm / enumerate_stable_models_wasm)"
        ),
    ))
}

fn deontic_evaluate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    if string_field(args, "operation") == Some("compile") {
        let q = compile_term_quin(args).ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                "deontic compile needs modality obligate|permit|forbid",
            )
        })?;
        return Ok(quin_compile_record(q));
    }

    let mut scan = parse_quins(args).unwrap_or_default();
    if let Some(q) = compile_term_quin(args) {
        scan.insert(0, q);
    }
    if scan.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "DeonticLogic.evaluate needs quins: [[u64;6],…] and/or modality term",
        ));
    }
    let now = u64_field(args, "now_unix").unwrap_or(0) as u32;
    let (n, verdicts) = evaluate_deontic(&scan, now);
    Ok(deontic_record(n, &verdicts[..n]))
}

fn epistemic_evaluate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let quins = parse_quins(args).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "EpistemicLogic.evaluate needs quins: [[u64;6],…]",
        )
    })?;
    let agent = u64_field(args, "agent").unwrap_or(0);
    let world = u64_field(args, "world").unwrap_or(0);
    let mut rows = Vec::new();
    for q in &quins {
        if world != 0 && q.context != world {
            continue;
        }
        if agent != 0 && q.subject != agent {
            continue;
        }
        let op = (q.predicate & 0xFF) as u8;
        if op != OP_KNOWS && op != OP_BELIEVES && op != OP_COMMON_KNOWLEDGE {
            continue;
        }
        let certainty = ((q.predicate >> CERTAINTY_SHIFT) & 0xFF) as u8;
        let status = if certainty >= 128 || op == OP_KNOWS || op == OP_COMMON_KNOWLEDGE {
            "Active"
        } else {
            "Uncertain"
        };
        let mut row = BTreeMap::new();
        row.insert("opcode".into(), Value::U64(op as u64));
        row.insert("certainty".into(), Value::U64(certainty as u64));
        row.insert("status".into(), Value::String(status.into()));
        rows.push(Value::Record(row));
    }
    Ok(record([
        ("id", Value::String("EpistemicLogic.evaluate".into())),
        ("honesty", Value::String("local".into())),
        ("evaluated", Value::Bool(true)),
        ("verdict_count", Value::U64(rows.len() as u64)),
        ("verdicts", Value::List(rows)),
    ]))
}

fn paraconsistent_route(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let quins = parse_quins(args).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "ParaconsistentLogic.route needs quins: [[u64;6],…]",
        )
    })?;
    let mut consistent = Vec::new();
    let mut isolated = Vec::new();
    let mut seen: Vec<(u64, u64, u64)> = Vec::new(); // subject, predicate, object
    for q in &quins {
        let key = (q.subject, q.predicate, q.object);
        let contradicted = seen
            .iter()
            .any(|(s, p, o)| *s == q.subject && *p == q.predicate && *o != q.object);
        if contradicted {
            let mut iq = *q;
            iq.context ^= 0xC0A1_1500_15A7_ED00;
            isolated.push(quin_list(&iq));
        } else {
            consistent.push(quin_list(q));
            seen.push(key);
        }
    }
    Ok(record([
        ("id", Value::String("ParaconsistentLogic.route".into())),
        ("honesty", Value::String("local".into())),
        ("evaluated", Value::Bool(true)),
        ("consistent_count", Value::U64(consistent.len() as u64)),
        ("isolated_count", Value::U64(isolated.len() as u64)),
        ("consistent", Value::List(consistent)),
        ("isolated", Value::List(isolated)),
    ]))
}

fn ltl_evaluate(id: &str, args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let quins = parse_quins(args).unwrap_or_default();
    let prop = u64_field(args, "property")
        .or_else(|| u64_field(args, "predicate"))
        .unwrap_or(0);
    let holds = if id.ends_with("globally") {
        !quins.is_empty() && quins.iter().all(|q| (q.predicate & 0xFF) as u8 == prop as u8 || q.predicate == prop)
    } else if id.ends_with("finally") {
        quins.iter().any(|q| q.predicate == prop || (q.predicate & 0xFF) as u8 == prop as u8)
    } else {
        // evaluate: G/F/X/U compact formula string or property + op
        let formula = string_field(args, "formula").unwrap_or("");
        eval_ltl_formula(formula, &quins)
    };
    Ok(record([
        ("id", Value::String(id.into())),
        ("honesty", Value::String("local".into())),
        ("evaluated", Value::Bool(true)),
        ("holds", Value::Bool(holds)),
    ]))
}

fn eval_ltl_formula(formula: &str, quins: &[Quin]) -> bool {
    let f = formula.trim();
    if f.is_empty() {
        return false;
    }
    if let Some(rest) = f.strip_prefix("G:") {
        if let Ok(p) = rest.parse::<u64>() {
            return !quins.is_empty() && quins.iter().all(|q| q.predicate == p);
        }
    }
    if let Some(rest) = f.strip_prefix("F:") {
        if let Ok(p) = rest.parse::<u64>() {
            return quins.iter().any(|q| q.predicate == p);
        }
    }
    if let Some(rest) = f.strip_prefix("X:") {
        if let Ok(p) = rest.parse::<u64>() {
            return quins.get(1).map(|q| q.predicate == p).unwrap_or(false);
        }
    }
    if let Some(rest) = f.strip_prefix("U:") {
        let mut parts = rest.splitn(2, ':');
        if let (Some(a), Some(b)) = (parts.next(), parts.next()) {
            if let (Ok(ante), Ok(cons)) = (a.parse::<u64>(), b.parse::<u64>()) {
                let mut saw_cons = false;
                for q in quins {
                    if q.predicate == cons {
                        saw_cons = true;
                        break;
                    }
                    if q.predicate != ante {
                        return false;
                    }
                }
                return saw_cons;
            }
        }
    }
    false
}

fn subsumption(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sub = u64_field(args, "sub")
        .or_else(|| u64_field(args, "subclass"))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                "subsumption needs sub: u64 (class hash)",
            )
        })?;
    let sup = u64_field(args, "super")
        .or_else(|| u64_field(args, "superclass"))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                "subsumption needs super: u64 (class hash)",
            )
        })?;
    let tbox = parse_quins(args).unwrap_or_default();
    // Walk rdfs:subClassOf edges encoded as predicate == subClassOf hash or raw links.
    let sub_class_of = fnv1a60(b"rdfs:subClassOf");
    let mut frontier = vec![sub];
    let mut seen = vec![sub];
    let mut found = sub == sup;
    while let Some(cur) = frontier.pop() {
        if cur == sup {
            found = true;
            break;
        }
        for q in &tbox {
            if q.subject == cur && (q.predicate == sub_class_of || q.predicate == 0) {
                if !seen.contains(&q.object) {
                    seen.push(q.object);
                    frontier.push(q.object);
                }
            }
        }
    }
    Ok(record([
        (
            "id",
            Value::String("TemporalAndDescriptionLogic.subsumption".into()),
        ),
        ("honesty", Value::String("local".into())),
        ("evaluated", Value::Bool(true)),
        ("subsumed", Value::Bool(found)),
    ]))
}

fn shacl_validate_local(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    // Full ShapeSpec validation lives in qualia-core-db (`validate_json` /
    // `validate_shacl_json_wasm`). LocalHost without the engine fails closed.
    let _ = args;
    Err(Diagnostic::new(
        DiagCode::E300,
        span,
        "SHACL.validate on LocalHost requires Qualia engine host (Poet) or validate_shacl_json_wasm",
    ))
}

fn shacl_extensions() -> Value {
    Value::List(
        [
            "minCount",
            "maxCount",
            "datatype",
            "deonticObligate",
            "deonticPermit",
            "deonticForbid",
            "epistemicKnowledge",
            "epistemicBelief",
            "ltlConstraint",
            "paraconsistentConstraint",
            "valuesConsentNonCoerced",
            "valuesHarmBelowCeiling",
            "fuzzyMinDegree",
        ]
        .iter()
        .map(|s| Value::String((*s).into()))
        .collect(),
    )
}

const MAX_CAUSAL_NODES: usize = 256;

fn causal_caused(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let edges = parse_quins(args).unwrap_or_default();
    let effect = u64_field(args, "effect").ok_or_else(|| {
        Diagnostic::new(DiagCode::E100, span, "CausalFuzzyAndControl.caused needs effect: u64")
    })?;
    let roots = u64_list(args, "roots").unwrap_or_default();
    if roots.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "CausalFuzzyAndControl.caused needs roots: [u64,…]",
        ));
    }
    let cause_of = fnv1a60(b"q42:causeOf");
    let holds = caused_bfs(&edges, &roots, effect, cause_of);
    Ok(record([
        ("id", Value::String("CausalFuzzyAndControl.caused".into())),
        ("honesty", Value::String("local".into())),
        ("evaluated", Value::Bool(true)),
        ("caused", Value::Bool(holds)),
    ]))
}

fn caused_bfs(edges: &[Quin], roots: &[u64], target: u64, pred: u64) -> bool {
    let mut frontier = [0u64; MAX_CAUSAL_NODES];
    let mut visited = [0u64; MAX_CAUSAL_NODES];
    let mut fl = 0usize;
    let mut vl = 0usize;
    for &r in roots {
        if r == target {
            return true;
        }
        if fl < MAX_CAUSAL_NODES {
            frontier[fl] = r;
            fl += 1;
        }
    }
    while fl > 0 {
        fl -= 1;
        let cur = frontier[fl];
        if visited[..vl].contains(&cur) {
            continue;
        }
        if vl < MAX_CAUSAL_NODES {
            visited[vl] = cur;
            vl += 1;
        } else {
            break;
        }
        for e in edges {
            if e.predicate == pred && e.subject == cur {
                if e.object == target {
                    return true;
                }
                if fl < MAX_CAUSAL_NODES && !visited[..vl].contains(&e.object) {
                    frontier[fl] = e.object;
                    fl += 1;
                }
            }
        }
    }
    false
}

fn fuzzy_t_norm(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = f64_field(args, "a").or_else(|| f64_field(args, "x")).ok_or_else(|| {
        Diagnostic::new(DiagCode::E100, span, "CausalFuzzyAndControl.t_norm needs a: f64")
    })?;
    let b = f64_field(args, "b").or_else(|| f64_field(args, "y")).ok_or_else(|| {
        Diagnostic::new(DiagCode::E100, span, "CausalFuzzyAndControl.t_norm needs b: f64")
    })?;
    let family = string_field(args, "family").unwrap_or("godel");
    let out = match family {
        "lukasiewicz" | "Łukasiewicz" => (a + b - 1.0).max(0.0),
        "product" => a * b,
        _ => a.min(b), // Gödel
    };
    Ok(record([
        ("id", Value::String("CausalFuzzyAndControl.t_norm".into())),
        ("honesty", Value::String("local".into())),
        ("evaluated", Value::Bool(true)),
        ("degree", Value::F64(out)),
        ("family", Value::String(family.into())),
    ]))
}

fn u64_list(args: &Value, key: &str) -> Option<Vec<u64>> {
    let Value::Record(map) = args else {
        return None;
    };
    let Value::List(xs) = map.get(key)? else {
        return None;
    };
    let mut out = Vec::new();
    for x in xs {
        match x {
            Value::U64(n) => out.push(*n),
            Value::I64(n) => out.push(*n as u64),
            Value::F64(n) if n.is_finite() && *n >= 0.0 => out.push(*n as u64),
            _ => return None,
        }
    }
    Some(out)
}

fn f64_field(args: &Value, key: &str) -> Option<f64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            Some(Value::U64(n)) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}

fn evaluate_deontic(quins: &[Quin], now_unix: u32) -> (usize, [DeonticRow; MAX_VERDICTS]) {
    let mut defeaters = [0u64; MAX_DEFEATERS];
    let mut dcount = 0usize;
    for q in quins {
        if q.predicate & DEFEATER_BIT == 0 {
            continue;
        }
        let parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        if q.parity != parity {
            continue;
        }
        if dcount < MAX_DEFEATERS {
            defeaters[dcount] = defeater_fp(q);
            dcount += 1;
        }
    }
    let mut out = [DeonticRow::default(); MAX_VERDICTS];
    let mut n = 0usize;
    for q in quins {
        if q.predicate & DEFEATER_BIT != 0 {
            continue;
        }
        let op = (q.predicate & 0xFF) as u8;
        if op != OP_OBLIGATE && op != OP_PERMIT && op != OP_FORBID {
            continue;
        }
        if n >= MAX_VERDICTS {
            break;
        }
        let expiry = (q.metadata & 0xFFFF_FFFF) as u32;
        let status = if expiry != 0 && now_unix > expiry {
            "Expired"
        } else if defeaters[..dcount].contains(&defeater_fp(q)) {
            "Defeated"
        } else {
            "Active"
        };
        out[n] = DeonticRow { opcode: op, status };
        n += 1;
    }
    (n, out)
}

#[derive(Clone, Copy, Default)]
struct DeonticRow {
    opcode: u8,
    status: &'static str,
}

fn defeater_fp(q: &Quin) -> u64 {
    let path_bits = q.predicate & 0x7FFF_FFFF_FFFF_FF00;
    q.subject ^ q.context ^ path_bits
}

fn compile_term_quin(args: &Value) -> Option<Quin> {
    let modality = string_field(args, "modality")?;
    let opcode = match modality {
        "obligate" => OP_OBLIGATE,
        "permit" => OP_PERMIT,
        "forbid" => OP_FORBID,
        _ => return None,
    };
    let body = string_field(args, "body").unwrap_or("vibe:modal");
    Some(
        Quin {
            subject: fnv1a60(b"vibe:party"),
            predicate: (fnv1a60(b"vibe:action") << 8) | opcode as u64,
            object: fnv1a60(body.as_bytes()),
            context: fnv1a60(b"vibe:contract"),
            metadata: 0,
            parity: 0,
        }
        .with_parity(),
    )
}

fn deontic_record(n: usize, verdicts: &[DeonticRow]) -> Value {
    let mut rows = Vec::with_capacity(n);
    for v in verdicts.iter().take(n) {
        let mut row = BTreeMap::new();
        row.insert("opcode".into(), Value::U64(v.opcode as u64));
        row.insert("status".into(), Value::String(v.status.into()));
        rows.push(Value::Record(row));
    }
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String("DeonticLogic.evaluate".into()));
    rec.insert("honesty".into(), Value::String("local".into()));
    rec.insert("evaluated".into(), Value::Bool(true));
    rec.insert("verdict_count".into(), Value::U64(n as u64));
    if let Some(first) = verdicts.first() {
        rec.insert("opcode".into(), Value::U64(first.opcode as u64));
        rec.insert("status".into(), Value::String(first.status.into()));
    }
    rec.insert("verdicts".into(), Value::List(rows));
    Value::Record(rec)
}

fn quin_compile_record(q: Quin) -> Value {
    record([
        ("operation", Value::String("compile".into())),
        ("compiled", Value::Bool(true)),
        ("honesty", Value::String("local".into())),
        ("subject", Value::U64(q.subject)),
        ("predicate", Value::U64(q.predicate)),
        ("object", Value::U64(q.object)),
        ("context", Value::U64(q.context)),
        ("metadata", Value::U64(q.metadata)),
        ("parity", Value::U64(q.parity)),
    ])
}

fn quin_list(q: &Quin) -> Value {
    Value::List(vec![
        Value::U64(q.subject),
        Value::U64(q.predicate),
        Value::U64(q.object),
        Value::U64(q.context),
        Value::U64(q.metadata),
        Value::U64(q.parity),
    ])
}

fn parse_quins(args: &Value) -> Option<Vec<Quin>> {
    let list = match args {
        Value::Record(m) => match m.get("quins")? {
            Value::List(xs) => xs,
            _ => return None,
        },
        Value::List(xs) => xs,
        _ => return None,
    };
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        match item {
            Value::List(cells) if cells.len() >= 6 => {
                let mut arr = [0u64; 6];
                for (i, c) in cells.iter().take(6).enumerate() {
                    arr[i] = match c {
                        Value::U64(n) => *n,
                        Value::I64(n) => *n as u64,
                        _ => return None,
                    };
                }
                out.push(Quin::from_arr(&arr));
            }
            _ => return None,
        }
    }
    Some(out)
}

fn fnv1a60(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

fn record<const N: usize>(pairs: [(&str, Value); N]) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(k.into(), v);
    }
    Value::Record(m)
}

fn string_field<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn u64_field(args: &Value, key: &str) -> Option<u64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::U64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as u64),
            Some(Value::F64(n)) if n.is_finite() && *n >= 0.0 => Some(*n as u64),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workshop_obligate_is_active() {
        let mut rec = BTreeMap::new();
        rec.insert("modality".into(), Value::String("obligate".into()));
        rec.insert("body".into(), Value::String("sign".into()));
        let v = deontic_evaluate(&Value::Record(rec), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(m) = v else { panic!() };
        assert_eq!(m.get("evaluated"), Some(&Value::Bool(true)));
        assert_eq!(m.get("status"), Some(&Value::String("Active".into())));
        assert_eq!(m.get("verdict_count"), Some(&Value::U64(1)));
    }

    #[test]
    fn shacl_validate_is_e300_on_localhost() {
        let err = shacl_validate_local(&Value::Null, Span { start: 0, end: 0 }).unwrap_err();
        assert_eq!(err.code, DiagCode::E300);
    }

    #[test]
    fn causal_caused_and_t_norm_localhost() {
        let cause_of = fnv1a60(b"q42:causeOf");
        let edge = Quin {
            subject: 1,
            predicate: cause_of,
            object: 2,
            ..Quin::default()
        }
        .with_parity();
        let mut rec = BTreeMap::new();
        rec.insert(
            "quins".into(),
            Value::List(vec![Value::List(vec![
                Value::U64(edge.subject),
                Value::U64(edge.predicate),
                Value::U64(edge.object),
                Value::U64(0),
                Value::U64(0),
                Value::U64(edge.parity),
            ])]),
        );
        rec.insert("roots".into(), Value::List(vec![Value::U64(1)]));
        rec.insert("effect".into(), Value::U64(2));
        let v = causal_caused(&Value::Record(rec), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(m) = v else { panic!() };
        assert_eq!(m.get("caused"), Some(&Value::Bool(true)));

        let mut tn = BTreeMap::new();
        tn.insert("a".into(), Value::F64(0.7));
        tn.insert("b".into(), Value::F64(0.4));
        let tv = fuzzy_t_norm(&Value::Record(tn), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(tm) = tv else { panic!() };
        assert_eq!(tm.get("degree"), Some(&Value::F64(0.4)));
    }
}
