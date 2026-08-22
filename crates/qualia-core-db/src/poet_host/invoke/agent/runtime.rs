//! Agent runtime invoke seams — planner, corpus, evaluator.
//!
//! Exposes the `agent_runtime` module's planner, corpus loader, and evaluator
//! through VibeScript invoke IDs.

use super::super::args;
use crate::agent_runtime::{corpus, evaluator, planner};
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `Agent.plan` — plan an agent task given a task description and capabilities.
///
/// Takes `task` (string) and `capabilities` (list of strings). Returns a
/// record with `steps` (list of step records), `total_budget`, and `task`.
pub fn agent_plan(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let task =
        args::rec_str(args, "task").ok_or_else(|| args::bad(span, "Agent.plan needs task"))?;
    let capabilities: Vec<String> = match args {
        Value::Record(map) => match map.get("capabilities") {
            Some(Value::List(caps)) => caps
                .iter()
                .filter_map(|c| match c {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        },
        _ => vec![],
    };

    let plan = planner::plan_task(task, &capabilities);

    let steps: Vec<Value> = plan
        .steps
        .iter()
        .map(|step| {
            let mut rec = std::collections::BTreeMap::new();
            rec.insert("id".into(), Value::U64(step.id as u64));
            rec.insert("name".into(), Value::String(step.name.clone()));
            rec.insert("capability".into(), Value::String(step.capability.clone()));
            rec.insert("effect".into(), Value::String(step.effect.into()));
            rec.insert("budget".into(), Value::U64(step.budget as u64));
            rec.insert(
                "inputs".into(),
                Value::List(
                    step.inputs
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
            rec.insert(
                "outputs".into(),
                Value::List(
                    step.outputs
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                ),
            );
            rec.insert(
                "depends_on".into(),
                Value::List(
                    step.depends_on
                        .iter()
                        .map(|d| Value::U64(*d as u64))
                        .collect(),
                ),
            );
            Value::Record(rec)
        })
        .collect();

    Ok(args::record([
        ("task", Value::String(plan.task.clone())),
        ("steps", Value::List(steps)),
        ("total_budget", Value::U64(plan.total_budget as u64)),
        ("step_count", Value::U64(plan.steps.len() as u64)),
    ]))
}

/// `Agent.execute` — execute a planned agent task.
///
/// Currently this builds the plan and validates it (the actual DAG execution
/// is handled by `agent.dag.execute`). Takes `task` and `capabilities`,
/// returns a plan record with an `execution_ready` flag.
pub fn agent_execute(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let task =
        args::rec_str(args, "task").ok_or_else(|| args::bad(span, "Agent.execute needs task"))?;
    let capabilities: Vec<String> = match args {
        Value::Record(map) => match map.get("capabilities") {
            Some(Value::List(caps)) => caps
                .iter()
                .filter_map(|c| match c {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        },
        _ => vec![],
    };

    let plan = planner::plan_task(task, &capabilities);
    let step_count = plan.steps.len();
    let execution_ready = step_count > 0 && plan.total_budget > 0;

    Ok(args::record([
        ("task", Value::String(plan.task.clone())),
        ("step_count", Value::U64(step_count as u64)),
        ("total_budget", Value::U64(plan.total_budget as u64)),
        ("execution_ready", Value::Bool(execution_ready)),
        ("status", Value::String("planned".into())),
    ]))
}

/// `Corpus.load` — load a golden corpus from a file path (native only).
///
/// Takes `path` (string). Returns a record with `name`, `case_count`, and
/// `tags`. On WASM without filesystem access, returns E300.
#[cfg(not(target_arch = "wasm32"))]
pub fn corpus_load(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let path =
        args::rec_str(args, "path").ok_or_else(|| args::bad(span, "Corpus.load needs path"))?;
    match corpus::load_corpus_from_file(path) {
        Ok(c) => {
            let case_count = c.len();
            let tags: Vec<Value> = c.all_tags().into_iter().map(Value::String).collect();
            Ok(args::record([
                ("name", Value::String(c.name)),
                ("case_count", Value::U64(case_count as u64)),
                ("tags", Value::List(tags)),
                ("loaded", Value::Bool(true)),
            ]))
        }
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Corpus.load: {e}"),
        )),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn corpus_load(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Corpus.load"))
}

/// `Corpus.parse` — parse a golden corpus from an inline text string.
///
/// Takes `name` (string) and `text` (string). Returns a record with `name`,
/// `case_count`, and `tags`.
pub fn corpus_parse(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name =
        args::rec_str(args, "name").ok_or_else(|| args::bad(span, "Corpus.parse needs name"))?;
    let text =
        args::rec_str(args, "text").ok_or_else(|| args::bad(span, "Corpus.parse needs text"))?;
    let c = corpus::parse_corpus(name, text);
    let case_count = c.len();
    let tags: Vec<Value> = c.all_tags().into_iter().map(Value::String).collect();
    Ok(args::record([
        ("name", Value::String(c.name)),
        ("case_count", Value::U64(case_count as u64)),
        ("tags", Value::List(tags)),
    ]))
}

/// `Agent.evaluate` — evaluate agent outputs against a golden corpus.
///
/// Takes `expected` (list of strings), `outputs` (list of strings), and
/// optional `method` (string: "exact", "substring", "token_overlap").
/// Returns a record with `total`, `passed`, `accuracy`, `mean_score`,
/// `min_score`, `max_score`, and `results` (list of per-case records).
pub fn agent_evaluate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expected = args::rec_str_list(args, "expected")
        .ok_or_else(|| args::bad(span, "Agent.evaluate needs expected"))?;
    let outputs = args::rec_str_list(args, "outputs")
        .ok_or_else(|| args::bad(span, "Agent.evaluate needs outputs"))?;
    let method_str = args::rec_str(args, "method").unwrap_or("exact");
    let method = match method_str {
        "exact" => evaluator::MatchMethod::Exact,
        "substring" => evaluator::MatchMethod::Substring,
        "token_overlap" => evaluator::MatchMethod::TokenOverlap,
        _ => evaluator::MatchMethod::Exact,
    };

    if expected.len() != outputs.len() {
        return Err(args::bad(
            span,
            "Agent.evaluate: expected and outputs must have the same length",
        ));
    }

    // Build a temporary corpus from expected strings.
    let cases: Vec<corpus::GoldenCase> = expected
        .iter()
        .enumerate()
        .map(|(i, exp)| corpus::GoldenCase {
            name: format!("case_{i}"),
            input: String::new(),
            expected: exp.clone(),
            tags: std::collections::BTreeSet::new(),
        })
        .collect();
    let corp = corpus::GoldenCorpus {
        name: "eval".into(),
        cases,
    };

    let results = evaluator::evaluate_corpus(&corp, &outputs, method);
    let metrics = evaluator::compute_metrics(&results);

    let result_values: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut rec = std::collections::BTreeMap::new();
            rec.insert("name".into(), Value::String(r.name.clone()));
            rec.insert("passed".into(), Value::Bool(r.passed));
            rec.insert("score".into(), Value::F64(r.score));
            Value::Record(rec)
        })
        .collect();

    Ok(args::record([
        ("total", Value::U64(metrics.total as u64)),
        ("passed", Value::U64(metrics.passed as u64)),
        ("accuracy", Value::F64(metrics.accuracy)),
        ("mean_score", Value::F64(metrics.mean_score)),
        ("min_score", Value::F64(metrics.min_score)),
        ("max_score", Value::F64(metrics.max_score)),
        ("results", Value::List(result_values)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn agent_plan_basic() {
        let mut m = BTreeMap::new();
        m.insert("task".into(), Value::String("research the topic".into()));
        m.insert(
            "capabilities".into(),
            Value::List(vec![
                Value::String("NLP.substrate_extract".into()),
                Value::String("NLP.analyze".into()),
                Value::String("Asset.create".into()),
            ]),
        );
        let result = agent_plan(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("steps"));
                assert!(rec.contains_key("total_budget"));
                match rec.get("steps") {
                    Some(Value::List(steps)) => assert!(!steps.is_empty()),
                    _ => panic!("expected steps list"),
                }
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn agent_plan_missing_task() {
        let m = BTreeMap::new();
        let result = agent_plan(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn agent_execute_basic() {
        let mut m = BTreeMap::new();
        m.insert("task".into(), Value::String("analyse the data".into()));
        m.insert(
            "capabilities".into(),
            Value::List(vec![Value::String("NLP.analyze".into())]),
        );
        let result = agent_execute(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("execution_ready"), Some(&Value::Bool(true)));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn corpus_parse_inline() {
        let mut m = BTreeMap::new();
        m.insert("name".into(), Value::String("test".into()));
        m.insert(
            "text".into(),
            Value::String("# case: a\ninput: x\nexpected: y\n".into()),
        );
        let result = corpus_parse(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("case_count"), Some(&Value::U64(1)));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn agent_evaluate_exact() {
        let mut m = BTreeMap::new();
        m.insert(
            "expected".into(),
            Value::List(vec![
                Value::String("hello".into()),
                Value::String("world".into()),
            ]),
        );
        m.insert(
            "outputs".into(),
            Value::List(vec![
                Value::String("hello".into()),
                Value::String("world".into()),
            ]),
        );
        m.insert("method".into(), Value::String("exact".into()));
        let result = agent_evaluate(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("total"), Some(&Value::U64(2)));
                assert_eq!(rec.get("passed"), Some(&Value::U64(2)));
                match rec.get("accuracy") {
                    Some(Value::F64(a)) => assert!((a - 1.0).abs() < 1e-9),
                    _ => panic!("expected f64 accuracy"),
                }
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn agent_evaluate_mismatch_length() {
        let mut m = BTreeMap::new();
        m.insert(
            "expected".into(),
            Value::List(vec![Value::String("hello".into())]),
        );
        m.insert(
            "outputs".into(),
            Value::List(vec![
                Value::String("hello".into()),
                Value::String("world".into()),
            ]),
        );
        let result = agent_evaluate(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn agent_evaluate_token_overlap() {
        let mut m = BTreeMap::new();
        m.insert(
            "expected".into(),
            Value::List(vec![Value::String("hello world".into())]),
        );
        m.insert(
            "outputs".into(),
            Value::List(vec![Value::String("hello world foo".into())]),
        );
        m.insert("method".into(), Value::String("token_overlap".into()));
        let result = agent_evaluate(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("mean_score") {
                Some(Value::F64(s)) => assert!(*s > 0.0 && *s < 1.0),
                _ => panic!("expected f64 mean_score"),
            },
            _ => panic!("expected record"),
        }
    }
}
