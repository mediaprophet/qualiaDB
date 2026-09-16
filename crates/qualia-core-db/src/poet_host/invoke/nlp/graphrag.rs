//! `NLP.graphrag_query` — keyword-triple term overlap over caller-supplied
//! triples. Not GraphRAG, embeddings, or graph-neighborhood retrieval.

use crate::nlp::budget::{reject_query, reject_triples};
use crate::nlp::graphrag::GraphRagIndex;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `NLP.graphrag_query` — build an index from a list of `[s, p, o]` triples
/// and run a keyword query. Argument record: `{ query: string, k: int,
/// triples: [[s, p, o], ...] }`. Returns a list of
/// `{ subject, predicate, object, score }`.
pub fn graphrag_query(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rec = match args {
        Value::Record(r) => r,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.graphrag_query needs a { query, k, triples } record",
            ))
        }
    };
    let query = match rec.get("query") {
        Some(Value::String(s)) => s.as_str(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.graphrag_query needs a query string",
            ))
        }
    };
    reject_query(query.len()).map_err(|e| Diagnostic::new(DiagCode::E400, span, e.to_string()))?;
    let k = match rec.get("k") {
        Some(Value::U64(n)) => *n as usize,
        Some(Value::I64(n)) => (*n).max(0) as usize,
        _ => 10,
    };
    let triples: &[Value] = match rec.get("triples") {
        Some(Value::List(list)) => {
            reject_triples(list.len())
                .map_err(|e| Diagnostic::new(DiagCode::E400, span, e.to_string()))?;
            list.as_slice()
        }
        _ => &[],
    };
    let mut idx = GraphRagIndex::new();
    for t in triples {
        let parts: Vec<String> = match t {
            Value::List(list) => list
                .iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s.clone()),
                    _ => Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "NLP.graphrag_query triples must be [string, string, string]",
                    )),
                })
                .collect::<Result<_, _>>()?,
            Value::Triple(s, p, o) => {
                let to_str = |v: &Value| match v {
                    Value::String(s) => Ok(s.clone()),
                    Value::Iri(s) => Ok(s.clone()),
                    _ => Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "NLP.graphrag_query triple components must be strings",
                    )),
                };
                vec![
                    to_str(s.as_ref())?,
                    to_str(p.as_ref())?,
                    to_str(o.as_ref())?,
                ]
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "NLP.graphrag_query triples must be [s, p, o] lists",
                ))
            }
        };
        if parts.len() != 3 {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.graphrag_query triples must have exactly 3 components",
            ));
        }
        idx.add_triple(&parts[0], &parts[1], &parts[2]);
    }
    let results = idx.query(query, k);
    let list: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut rec = BTreeMap::new();
            rec.insert("subject".into(), Value::String(r.triple.0.clone()));
            rec.insert("predicate".into(), Value::String(r.triple.1.clone()));
            rec.insert("object".into(), Value::String(r.triple.2.clone()));
            rec.insert("score".into(), Value::F64(r.score));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(query: &str) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("query".into(), Value::String(query.into()));
        rec.insert("k".into(), Value::U64(10));
        rec.insert(
            "triples".into(),
            Value::List(vec![
                Value::List(vec![
                    Value::String("Paris".into()),
                    Value::String("locatedIn".into()),
                    Value::String("France".into()),
                ]),
                Value::List(vec![
                    Value::String("Socrates".into()),
                    Value::String("rdf:type".into()),
                    Value::String("philosopher".into()),
                ]),
            ]),
        );
        Value::Record(rec)
    }

    #[test]
    fn queries_index() {
        let r = graphrag_query(&args("Paris France"), Span { start: 0, end: 0 });
        assert!(r.is_ok());
        match r.unwrap() {
            Value::List(v) => assert_eq!(v.len(), 1),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn rejects_non_record() {
        let r = graphrag_query(&Value::I64(1), Span { start: 0, end: 0 });
        assert!(r.is_err());
    }

    #[test]
    fn rejects_oversize_query() {
        let q = "q".repeat(crate::nlp::budget::MAX_GRAPHRAG_QUERY_BYTES + 1);
        let err = graphrag_query(&args(&q), Span { start: 0, end: 0 }).unwrap_err();
        assert_eq!(err.code, DiagCode::E400);
    }

    #[test]
    fn rejects_oversize_triple_list() {
        let mut rec = BTreeMap::new();
        rec.insert("query".into(), Value::String("Paris".into()));
        rec.insert("k".into(), Value::U64(10));
        rec.insert(
            "triples".into(),
            Value::List(vec![Value::Null; crate::nlp::budget::MAX_TRIPLES + 1]),
        );
        let err = graphrag_query(&Value::Record(rec), Span { start: 0, end: 0 }).unwrap_err();
        assert_eq!(err.code, DiagCode::E400);
    }
}
