//! SPARQL 1.1 + SPARQL-star (quoted triples) over the live snapshot.

use super::super::args;
use crate::poet_host::PoetSnapshot;
use crate::sparql_ast::SparqlQuery;
use crate::sparql_executor::QueryExecutor;
use crate::sparql_parser::parse_sparql_full;
use crate::sparql_planner::QueryPlanner;
use poet_vibe::{Diagnostic, Span, Value};

pub fn query(snap: &PoetSnapshot, args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "query"))
        .ok_or_else(|| args::bad(span, "GraphDatabase.sparql needs a query string"))?;
    let star = q.contains("<<") || q.contains(">>");
    snap.with_live_quins(|quins| run(q, star, quins, span))
}

fn run(q: &str, star: bool, quins: &[crate::NQuin], span: Span) -> Result<Value, Diagnostic> {
    let (parsed, ctx, _lits) =
        parse_sparql_full(q).map_err(|e| args::bad(span, format!("SPARQL parse: {e}")))?;
    let plan = QueryPlanner::plan(&parsed, &ctx)
        .map_err(|e| args::bad(span, format!("SPARQL plan: {e}")))?;
    let exec = QueryExecutor::new(quins);
    let form = match parsed {
        SparqlQuery::Select(_) => "SELECT",
        SparqlQuery::Ask(_) => "ASK",
        SparqlQuery::Construct(_) => "CONSTRUCT",
        SparqlQuery::Describe(_) => "DESCRIBE",
    };
    match parsed {
        SparqlQuery::Ask(_) => {
            let ok = exec
                .execute_ask(&plan, &ctx)
                .map_err(|e| args::bad(span, format!("SPARQL ask: {e}")))?;
            Ok(args::record([
                ("form", Value::String(form.into())),
                ("star", Value::Bool(star)),
                ("ask", Value::Bool(ok)),
                ("count", Value::U64(u64::from(ok))),
            ]))
        }
        _ => {
            let rows = exec
                .execute(&plan, &ctx)
                .map_err(|e| args::bad(span, format!("SPARQL exec: {e}")))?;
            let listed: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Value> = row
                        .slots
                        .iter()
                        .enumerate()
                        .filter_map(|(i, slot)| {
                            slot.map(|v| {
                                args::record([
                                    ("slot", Value::U64(i as u64)),
                                    ("value", Value::U64(v)),
                                ])
                            })
                        })
                        .collect();
                    Value::List(cells)
                })
                .collect();
            Ok(args::record([
                ("form", Value::String(form.into())),
                ("star", Value::Bool(star)),
                ("count", Value::U64(listed.len() as u64)),
                ("rows", Value::List(listed)),
            ]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NQuin;

    #[test]
    fn ask_true_on_seed() {
        let q = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: NQuin::calculate_parity(1, 2, 3, 0, 0),
        };
        let snap = PoetSnapshot::with_seed(vec![q]);
        let src = Value::String("ASK WHERE { ?s ?p ?o }".into());
        match query(&snap, &src, Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("ask"), Some(&Value::Bool(true))),
            other => panic!("{other:?}"),
        }
    }
}
