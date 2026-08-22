//! Value ↔ Quin conversions. Host seals parity; scripts never write it.

use crate::lexicon::generate_60bit_token;
use crate::NQuin;
use vibe::Value;

const RDF_REIFIES: &[u8] = b"http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

pub(crate) fn hash_val(v: &Value) -> Option<u64> {
    match v {
        Value::U64(n) => Some(*n),
        Value::I64(n) => Some(*n as u64),
        Value::Iri(s) | Value::String(s) => Some(generate_60bit_token(s.as_bytes())),
        Value::Prefixed(p, l) => Some(generate_60bit_token(format!("{p}:{l}").as_bytes())),
        Value::QuinRef(qr) => Some(qr.raw_fields()[0]),
        _ => None,
    }
}

pub(crate) fn value_to_quin(term: &Value, context: u64) -> Option<NQuin> {
    match term {
        Value::QuinRef(qr) => {
            let [subject, predicate, object, ctx, metadata, _parity] = qr.raw_fields();
            let parity = NQuin::calculate_parity(subject, predicate, object, ctx, metadata);
            Some(NQuin {
                subject,
                predicate,
                object,
                context: ctx,
                metadata,
                parity,
            })
        }
        Value::Triple(s, p, o) => {
            let subject = hash_val(s)?;
            let predicate = hash_val(p)?;
            let object = hash_val(o)?;
            let metadata = 0;
            let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
            Some(NQuin {
                subject,
                predicate,
                object,
                context,
                metadata,
                parity,
            })
        }
        Value::Reified { s, p, o, .. } => {
            value_to_quin(&Value::Triple(s.clone(), p.clone(), o.clone()), context)
        }
        _ => None,
    }
}

pub(crate) fn reifier_quin(term: &Value, context: u64) -> Option<NQuin> {
    let Value::Reified { s, p, o, r } = term else {
        return None;
    };
    let subject = hash_val(r)?;
    let predicate = generate_60bit_token(RDF_REIFIES);
    let object = hash_val(s)? ^ hash_val(p)? ^ hash_val(o)?;
    let metadata = 0;
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    Some(NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    })
}

pub(crate) fn shape_local_name(shape: &Value) -> String {
    match shape {
        Value::Prefixed(_, local) => local.clone(),
        Value::Iri(s) | Value::String(s) => {
            s.rsplit(['#', '/', ':']).next().unwrap_or(s).to_string()
        }
        _ => String::new(),
    }
}

pub fn format_value(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::I64(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::F64(n) => n.to_string(),
        Value::String(s) => format!("\"{s}\""),
        Value::Iri(s) => format!("<{s}>"),
        Value::Ok(inner) => format!("Ok({})", format_value(inner)),
        Value::Err(inner) => format!("Err({})", format_value(inner)),
        Value::Receipt => "Receipt".into(),
        Value::List(xs) => {
            let parts: Vec<String> = xs.iter().map(format_value).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Record(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{k}: {}", format_value(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::QuinRef(qr) => {
            let f = qr.raw_fields();
            format!(
                "Quin(s={:#x},p={:#x},o={:#x},c={:#x})",
                f[0], f[1], f[2], f[3]
            )
        }
        Value::Triple(s, p, o) => format!(
            "<<( {} {} {} )>>",
            format_value(s),
            format_value(p),
            format_value(o)
        ),
        other => format!("{other:?}"),
    }
}
