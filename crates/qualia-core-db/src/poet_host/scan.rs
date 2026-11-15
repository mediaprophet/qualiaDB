//! Query scan helpers. No daemon lock lives here.

use super::values::hash_val;
use crate::NQuin;
use vibe::Value;

/// Collects matches.
pub fn collect_matches(
    quins: &[NQuin],
    s: Option<&Value>,
    p: Option<&Value>,
    o: Option<&Value>,
    take: u64,
    out: &mut Vec<Value>,
) {
    if out.len() as u64 >= take {
        return;
    }
    // Hash/allocate query syntax once on the cold entry path, never per row.
    let [s, p, o] = [pattern(s), pattern(p), pattern(o)];
    for q in quins {
        if !s.matches(q.subject) {
            continue;
        }
        if !p.matches(q.predicate) {
            continue;
        }
        if !o.matches(q.object) {
            continue;
        }
        out.push(Value::QuinRef(vibe::QuinRef::from_raw(
            q.subject,
            q.predicate,
            q.object,
            q.context,
            q.metadata,
            q.parity,
        )));
        if out.len() as u64 >= take {
            break;
        }
    }
}

/// `true` when `subject` is present.
pub fn subject_present(quins: &[NQuin], id: u64, predicate: Option<u64>) -> bool {
    quins
        .iter()
        .any(|q| q.subject == id && predicate.map(|p| q.predicate == p).unwrap_or(true))
}

#[derive(Clone, Copy)]
enum Pattern {
    Any,
    Exact(u64),
    Never,
}
impl Pattern {
    fn matches(self, field: u64) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(value) => value == field,
            Self::Never => false,
        }
    }
}
fn pattern(filter: Option<&Value>) -> Pattern {
    match filter {
        None | Some(Value::Var(_)) | Some(Value::Null) => Pattern::Any,
        Some(value) => hash_val(value).map_or(Pattern::Never, Pattern::Exact),
    }
}
#[cfg(test)]
mod scoped_scan_tests {
    use super::*;
    #[test]
    fn bounded_scan_preserves_full_quin_and_zero_take() {
        let mut quin = NQuin {
            subject: 11,
            predicate: 22,
            object: 33,
            context: 44,
            metadata: u64::MAX,
            parity: 0,
        };
        quin.recalculate_parity();
        let mut output = Vec::new();
        collect_matches(&[quin], None, None, None, 0, &mut output);
        assert!(output.is_empty());
        collect_matches(&[quin], None, None, None, 1, &mut output);
        let Value::QuinRef(found) = output[0] else {
            panic!("expected canonical quin")
        };
        assert_eq!(
            found.raw_fields(),
            [
                quin.subject,
                quin.predicate,
                quin.object,
                quin.context,
                quin.metadata,
                quin.parity
            ]
        );
    }
}
