//! Range-native SPARQL BIND (Extend) over a paged TripleScan.
//!
//! Every input row is kept. A successful expression writes `var`; an
//! evaluation error leaves the variable unbound. Wrappers above BIND are
//! applied with the shared Project/Filter/Limit kernel.

use super::range_select_apply::{apply_select_wrappers, SelectWrapperState};
use super::sparql_ast::{BindingRow, ExpressionId, SparqlQueryContext, VariableId, MAX_VARIABLES};
use super::sparql_executor::{
    execute_range_triple_page_into, execute_range_volume_set_triple_page_into,
    Q42RangeNestedLoopJoinPage, Q42RangeSparqlCursor, Q42RangeTriplePattern,
    Q42RangeVolumeSetSparqlCursor,
};
use super::sparql_filter::{EvalResult, ExpressionEvaluator};
use super::sparql_planner::{ExecutionPlan, PhysicalOperatorType};
use crate::NQuin;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeBindPlan {
    pub input: Q42RangeTriplePattern,
    pub var: VariableId,
    pub expression: ExpressionId,
    pub projection: [VariableId; MAX_VARIABLES],
    pub projection_count: u8,
    pub filters: [ExpressionId; 8],
    pub filter_count: u8,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeBindState {
    pub scan: Q42RangeSparqlCursor,
    pub exhausted: bool,
    pub wrappers: SelectWrapperState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeVolumeSetBindState {
    pub scan: Q42RangeVolumeSetSparqlCursor,
    pub exhausted: bool,
    pub wrappers: SelectWrapperState,
}

impl Q42RangeBindPlan {
    pub fn from_execution_plan(plan: &ExecutionPlan) -> Result<Self, String> {
        if plan.operator_count == 0 || plan.root_operator as usize >= plan.operator_count as usize {
            return Err("range BIND requires a non-empty execution plan".into());
        }
        let mut operator = plan.root_operator;
        let mut projection = [0; MAX_VARIABLES];
        let mut projection_count = 0;
        let mut filters = [0; 8];
        let mut filter_count = 0usize;
        let mut limit = u64::MAX;
        let mut offset = 0;
        loop {
            match plan.operators[operator as usize].operator_type {
                PhysicalOperatorType::Project {
                    input,
                    vars,
                    var_count,
                } => {
                    if projection_count != 0 {
                        return Err("range BIND does not support nested projections".into());
                    }
                    projection = vars;
                    projection_count = var_count;
                    operator = input;
                }
                PhysicalOperatorType::Limit {
                    input,
                    limit: configured,
                    offset: configured_offset,
                } => {
                    if limit != u64::MAX || offset != 0 {
                        return Err("range BIND does not support nested limits".into());
                    }
                    limit = configured;
                    offset = configured_offset;
                    operator = input;
                }
                PhysicalOperatorType::Filter { input, expression } => {
                    if filter_count == filters.len() {
                        return Err(
                            "range BIND supports at most eight stacked FILTER operators".into()
                        );
                    }
                    filters[filter_count] = expression;
                    filter_count += 1;
                    operator = input;
                }
                PhysicalOperatorType::Bind {
                    input,
                    var,
                    expression,
                } => {
                    return Ok(Self {
                        input: triple_scan(plan, input)?,
                        var,
                        expression,
                        projection,
                        projection_count,
                        filters,
                        filter_count: filter_count as u8,
                        limit,
                        offset,
                    });
                }
                _ => {
                    return Err(
                        "range BIND supports Project/Filter/Limit over Bind(TripleScan)".into(),
                    );
                }
            }
        }
    }
}

fn triple_scan(plan: &ExecutionPlan, operator: u16) -> Result<Q42RangeTriplePattern, String> {
    let Some(entry) = plan.operators.get(operator as usize) else {
        return Err("range BIND input operator is out of bounds".into());
    };
    match entry.operator_type {
        PhysicalOperatorType::TripleScan {
            subject,
            predicate,
            object,
        } => Ok(Q42RangeTriplePattern {
            subject,
            predicate,
            object,
        }),
        _ => Err("range BIND currently requires a TripleScan input".into()),
    }
}

pub fn apply_bind(
    ctx: &SparqlQueryContext,
    var: VariableId,
    expression: ExpressionId,
    row: &mut BindingRow,
) {
    match ExpressionEvaluator::evaluate(expression, ctx, row) {
        Ok(EvalResult::Numeric(n)) | Ok(EvalResult::Iri(n)) | Ok(EvalResult::String(n)) => {
            row.set(var, n);
        }
        Ok(EvalResult::Boolean(b)) => row.set(var, b as u64),
        Ok(EvalResult::Float(f)) => row.set(var, f.to_bits()),
        Err(_) => {}
    }
}

pub fn execute_range_bind_page_into<S: crate::q42_volume::Q42RangeSource>(
    volume: &crate::q42_volume::Q42RangeVolume<S>,
    plan: Q42RangeBindPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeBindState,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    raw: &mut [BindingRow],
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String> {
    let mut returned = 0usize;
    loop {
        if returned == out.len() {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: false,
            });
        }
        if state.wrappers.emitted >= plan.limit || state.exhausted {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        let page = execute_range_triple_page_into(
            volume,
            plan.input.subject,
            plan.input.predicate,
            plan.input.object,
            None,
            ctx,
            &BindingRow::default(),
            state.scan,
            compressed,
            decoded,
            quin_scratch,
            raw,
        )?;
        state.scan = page.next_cursor.unwrap_or_default();
        state.exhausted = page.next_cursor.is_none();
        for row in &mut raw[..page.returned] {
            apply_bind(ctx, plan.var, plan.expression, row);
        }
        let applied = apply_select_wrappers(
            ctx,
            &plan.filters,
            plan.filter_count,
            plan.projection,
            plan.projection_count,
            plan.limit,
            plan.offset,
            &mut state.wrappers,
            &raw[..page.returned],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if state.exhausted || applied.limit_reached {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        if applied.returned == 0 {
            continue;
        }
        if returned > 0 {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: false,
            });
        }
    }
}

pub fn execute_range_volume_set_bind_page_into<S: crate::q42_volume::Q42RangeSource>(
    volumes: &crate::q42_volume::Q42RangeVolumeSet<S>,
    plan: Q42RangeBindPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeVolumeSetBindState,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    raw: &mut [BindingRow],
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String> {
    let mut returned = 0usize;
    loop {
        if returned == out.len() {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: false,
            });
        }
        if state.wrappers.emitted >= plan.limit || state.exhausted {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        let page = execute_range_volume_set_triple_page_into(
            volumes,
            plan.input.subject,
            plan.input.predicate,
            plan.input.object,
            None,
            ctx,
            &BindingRow::default(),
            state.scan,
            compressed,
            decoded,
            quin_scratch,
            raw,
        )?;
        state.scan = page.next_cursor.unwrap_or_default();
        state.exhausted = page.next_cursor.is_none();
        for row in &mut raw[..page.returned] {
            apply_bind(ctx, plan.var, plan.expression, row);
        }
        let applied = apply_select_wrappers(
            ctx,
            &plan.filters,
            plan.filter_count,
            plan.projection,
            plan.projection_count,
            plan.limit,
            plan.offset,
            &mut state.wrappers,
            &raw[..page.returned],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if state.exhausted || applied.limit_reached {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        if applied.returned == 0 {
            continue;
        }
        if returned > 0 {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{write_unified_volume, LocalFileRangeSource, Q42RangeVolume};
    use crate::sparql_ast::Expression;

    fn quin(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    fn test_volume(quins: &[NQuin]) -> (tempfile::TempDir, Q42RangeVolume<LocalFileRangeSource>) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bind.q42");
        let lo = quins.iter().map(|q| q.object).min().unwrap_or(0);
        let hi = quins.iter().map(|q| q.object).max().unwrap_or(0);
        write_unified_volume(
            &path,
            &std::collections::HashMap::new(),
            &[(lo, hi)],
            &[quins.to_vec()],
        )
        .unwrap();
        let source = LocalFileRangeSource::open(&path).unwrap();
        (dir, Q42RangeVolume::open(source).unwrap())
    }

    #[test]
    fn peels_project_from_bind_tree() {
        let mut plan = ExecutionPlan::new();
        let scan = plan
            .add_operator(
                PhysicalOperatorType::TripleScan {
                    subject: 0,
                    predicate: 20,
                    object: 1,
                },
                1,
            )
            .unwrap();
        let bind = plan
            .add_operator(
                PhysicalOperatorType::Bind {
                    input: scan,
                    var: 2,
                    expression: 0,
                },
                1,
            )
            .unwrap();
        let mut vars = [0; MAX_VARIABLES];
        vars[0] = 2;
        plan.root_operator = plan
            .add_operator(
                PhysicalOperatorType::Project {
                    input: bind,
                    vars,
                    var_count: 1,
                },
                1,
            )
            .unwrap();
        let compiled = Q42RangeBindPlan::from_execution_plan(&plan).unwrap();
        assert_eq!(compiled.var, 2);
        assert_eq!(compiled.input.predicate, 20);
        assert_eq!(compiled.projection_count, 1);
    }

    #[test]
    fn bind_literal_extends_every_row() {
        let row = quin(10, 20, 30);
        let (_dir, volume) = test_volume(&[row]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 3;
        let expr = ctx.alloc_expression(Expression::Literal(42)).unwrap();
        let plan = Q42RangeBindPlan {
            input: Q42RangeTriplePattern {
                subject: 0,
                predicate: 20,
                object: 1,
            },
            var: 2,
            expression: expr,
            projection: [0; MAX_VARIABLES],
            projection_count: 0,
            filters: [0; 8],
            filter_count: 0,
            limit: u64::MAX,
            offset: 0,
        };
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 2];
        let mut raw = [BindingRow::default(); 2];
        let mut out = [BindingRow::default(); 2];
        let mut state = Q42RangeBindState::default();
        let page = execute_range_bind_page_into(
            &volume,
            plan,
            &ctx,
            &mut state,
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut raw,
            &mut out,
        )
        .unwrap();
        assert!(page.done);
        assert_eq!(page.returned, 1);
        assert_eq!(out[0].get(0), Some(10));
        assert_eq!(out[0].get(1), Some(30));
        assert_eq!(out[0].get(2), Some(42));
    }

    #[test]
    fn bind_error_leaves_variable_unbound() {
        let row = quin(10, 20, 30);
        let (_dir, volume) = test_volume(&[row]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 3;
        let plan = Q42RangeBindPlan {
            input: Q42RangeTriplePattern {
                subject: 0,
                predicate: 20,
                object: 1,
            },
            var: 2,
            expression: 200,
            projection: [0; MAX_VARIABLES],
            projection_count: 0,
            filters: [0; 8],
            filter_count: 0,
            limit: u64::MAX,
            offset: 0,
        };
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 2];
        let mut raw = [BindingRow::default(); 2];
        let mut out = [BindingRow::default(); 2];
        let mut state = Q42RangeBindState::default();
        let page = execute_range_bind_page_into(
            &volume,
            plan,
            &ctx,
            &mut state,
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut raw,
            &mut out,
        )
        .unwrap();
        assert_eq!(page.returned, 1);
        assert_eq!(out[0].get(0), Some(10));
        assert_eq!(out[0].get(2), None);
    }
}
