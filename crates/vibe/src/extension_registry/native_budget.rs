// Copyright (c) 2026 Timothy Charles Holborn.
// SPDX-License-Identifier: CC-BY-NC-ND-4.0
//! Cold canonical graph result admission shared by all execution backends.
use crate::{Budget, DiagCode, Diagnostic, Host, Span, Value};

pub(super) fn dispatch<H: Host>(
    host: &mut H,
    path: &str,
    args: &[Value],
    named: &[(String, Value)],
    span: Span,
    budget: &mut Budget,
) -> Result<Value, Diagnostic> {
    if path == "graph.query" {
        // Match bind::dispatch's take interpretation. A negative or overflowing
        // admission is rejected before the host runs. Canonical query rows are
        // inline QuinRefs (all six lanes), with no string/container children.
        let take = named
            .iter()
            .find(|(k, _)| k == "take")
            .and_then(|(_, v)| v.as_i64())
            .unwrap_or(0) as u64;
        let cell = std::mem::size_of::<Value>() as u64;
        // Cover the result header and Vec growth up to twice the requested cells
        // (minimum initial growth four). This is a conservative retained-result
        // admission, separate from a concrete host's fixed workspace reservation.
        let cells = if take == 0 {
            Some(0)
        } else {
            take.max(4).checked_mul(2)
        };
        let bytes = cells
            .and_then(|n| n.checked_mul(cell))
            .and_then(|n| n.checked_add(cell))
            .ok_or_else(|| {
                Diagnostic::new(DiagCode::E400, span, "graph result workspace overflow")
            })?;
        budget.charge(bytes, span)?;
    }
    // Do not refund on an error: a host may already have allocated or performed
    // work. Budget charges remain monotonic within the execution.
    crate::bind::dispatch(host, path, args, named, span)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct QueryHost {
        calls: usize,
        fail: bool,
    }
    impl Host for QueryHost {
        fn graph_query(&mut self, _: &[Value], _: u64, span: Span) -> Result<Value, Diagnostic> {
            self.calls += 1;
            if self.fail {
                Err(Diagnostic::new(DiagCode::E300, span, "host failure"))
            } else {
                Ok(Value::List(Vec::new()))
            }
        }
        fn graph_stage(&mut self, _: &Value, _: Span) -> Result<Value, Diagnostic> {
            unreachable!()
        }
        fn graph_commit(&mut self, _: Span) -> Result<Value, Diagnostic> {
            unreachable!()
        }
        fn aura_validate(&mut self, _: &Value, _: &Value, _: Span) -> Result<Value, Diagnostic> {
            unreachable!()
        }
        fn pulse_publish(&mut self, _: &str, _: &Value, _: Span) -> Result<Value, Diagnostic> {
            unreachable!()
        }
    }
    #[test]
    fn query_results_charge_before_host_and_repeated_calls_exhaust() {
        let mut host = QueryHost::default();
        let bytes = 9 * std::mem::size_of::<Value>() as u64;
        let mut budget = Budget {
            steps_left: 10,
            workspace_left: bytes * 2,
        };
        let named = [("take".into(), Value::I64(1))];
        for _ in 0..2 {
            dispatch(
                &mut host,
                "graph.query",
                &[],
                &named,
                Span::point(0),
                &mut budget,
            )
            .unwrap();
        }
        assert_eq!(budget.workspace_left, 0);
        assert!(dispatch(
            &mut host,
            "graph.query",
            &[],
            &named,
            Span::point(0),
            &mut budget
        )
        .is_err());
        assert_eq!(host.calls, 2);
    }
    #[test]
    fn failed_query_keeps_charge_and_overflow_never_calls_host() {
        let mut host = QueryHost {
            fail: true,
            ..Default::default()
        };
        let mut budget = Budget::default();
        let initial = budget.workspace_left;
        assert!(dispatch(
            &mut host,
            "graph.query",
            &[],
            &[],
            Span::point(0),
            &mut budget
        )
        .is_err());
        assert_eq!(
            budget.workspace_left,
            initial - std::mem::size_of::<Value>() as u64
        );
        let before = budget.workspace_left;
        let named = [("take".into(), Value::I64(-1))];
        assert!(dispatch(
            &mut host,
            "graph.query",
            &[],
            &named,
            Span::point(0),
            &mut budget
        )
        .is_err());
        assert_eq!(budget.workspace_left, before);
        assert_eq!(host.calls, 1);
    }
}
