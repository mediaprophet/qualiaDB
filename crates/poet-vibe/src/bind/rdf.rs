//! RDF 1.2 constructors.

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub fn call_rdf(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "rdf.triple" => {
            if args.len() < 3 {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "rdf.triple(s, p, o) needs 3 arguments",
                ));
            }
            Ok(Some(Value::Triple(
                Box::new(args[0].clone()),
                Box::new(args[1].clone()),
                Box::new(args[2].clone()),
            )))
        }
        "rdf.reify" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "rdf.reify(term, reifier) needs 2 arguments",
                ));
            }
            match &args[0] {
                Value::Triple(s, p, o) => Ok(Some(Value::Reified {
                    s: s.clone(),
                    p: p.clone(),
                    o: o.clone(),
                    r: Box::new(args[1].clone()),
                })),
                _ => Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "rdf.reify first argument must be a triple term",
                )),
            }
        }
        _ => Ok(None),
    }
}
