//! Query scan helpers. No daemon lock lives here.

use crate::lexicon::generate_60bit_token;
use crate::NQuin;
use vibe::Value;

pub fn collect_matches(
    quins: &[NQuin],
    s: Option<&Value>,
    p: Option<&Value>,
    o: Option<&Value>,
    take: u64,
    out: &mut Vec<Value>,
) {
    for q in quins {
        if !term_match(s, q.subject) {
            continue;
        }
        if !term_match(p, q.predicate) {
            continue;
        }
        if !term_match(o, q.object) {
            continue;
        }
        out.push(Value::QuinRef(vibe::QuinRef::from_quin(
            q.subject,
            q.predicate,
            q.object,
            q.context,
        )));
        if out.len() as u64 >= take {
            break;
        }
    }
}

pub fn subject_present(quins: &[NQuin], id: u64, predicate: Option<u64>) -> bool {
    quins
        .iter()
        .any(|q| q.subject == id && predicate.map(|p| q.predicate == p).unwrap_or(true))
}

fn term_match(filter: Option<&Value>, field: u64) -> bool {
    match filter {
        None | Some(Value::Var(_)) | Some(Value::Null) => true,
        Some(Value::U64(n)) => *n == field,
        Some(Value::I64(n)) => *n as u64 == field,
        Some(Value::Iri(s) | Value::String(s)) => generate_60bit_token(s.as_bytes()) == field,
        Some(Value::Prefixed(p, l)) => generate_60bit_token(format!("{p}:{l}").as_bytes()) == field,
        Some(Value::QuinRef(qr)) => qr.raw_fields()[0] == field,
        _ => false,
    }
}
