//! Range-native nested-loop joins wrapped by Project / Filter / Limit.
//!
//! The existing join kernel only accepted a root `NestedLoopJoin`. Real SELECT
//! trees are `Project(Filter(Limit(Join(Scan, Scan))))`. This module peels those
//! wrappers without materialising a resident graph.

use super::range_select_apply::{apply_select_wrappers, SelectWrapperState};
use super::sparql_ast::{BindingRow, SparqlQueryContext, VariableId, MAX_VARIABLES};
use super::sparql_executor::{
    execute_range_nested_loop_join_page_into, execute_range_volume_set_nested_loop_join_page_into,
    Q42RangeNestedLoopJoinPage, Q42RangeNestedLoopJoinPlan, Q42RangeNestedLoopJoinState,
    Q42RangeVolumeSetNestedLoopJoinState,
};
use super::sparql_planner::{ExecutionPlan, PhysicalOperatorType};
use crate::NQuin;

/// A join plus the SELECT wrappers that sit above it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeJoinSelectPlan {
    pub join: Q42RangeNestedLoopJoinPlan,
    pub projection: [VariableId; MAX_VARIABLES],
    pub projection_count: u8,
    pub filters: [super::sparql_ast::ExpressionId; 8],
    pub filter_count: u8,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeJoinSelectState {
    pub join: Q42RangeNestedLoopJoinState,
    pub wrappers: SelectWrapperState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeVolumeSetJoinSelectState {
    pub join: Q42RangeVolumeSetNestedLoopJoinState,
    pub wrappers: SelectWrapperState,
}

impl Q42RangeJoinSelectPlan {
    pub fn from_execution_plan(plan: &ExecutionPlan) -> Result<Self, String> {
        if plan.operator_count == 0 || plan.root_operator as usize >= plan.operator_count as usize {
            return Err("range join-select requires a non-empty execution plan".to_string());
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
                        return Err("range join-select does not support nested projections".into());
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
                        return Err("range join-select does not support nested limits".into());
                    }
                    limit = configured;
                    offset = configured_offset;
                    operator = input;
                }
                PhysicalOperatorType::Filter { input, expression } => {
                    if filter_count == filters.len() {
                        return Err(
                            "range join-select supports at most eight stacked FILTER operators"
                                .into(),
                        );
                    }
                    filters[filter_count] = expression;
                    filter_count += 1;
                    operator = input;
                }
                PhysicalOperatorType::NestedLoopJoin { .. } => {
                    return Ok(Self {
                        join: Q42RangeNestedLoopJoinPlan::from_join_root(plan, operator)?,
                        projection,
                        projection_count,
                        filters,
                        filter_count: filter_count as u8,
                        limit,
                        offset,
                    });
                }
                PhysicalOperatorType::HashJoin { .. } => {
                    return Err(
                        "range join-select does not yet execute HashJoin; planner must emit NestedLoopJoin"
                            .into(),
                    );
                }
                _ => {
                    return Err(
                        "range join-select supports Project/Filter/Limit over NestedLoopJoin(Scan, Scan)"
                            .into(),
                    );
                }
            }
        }
    }
}

pub fn execute_range_join_select_page_into<S: crate::q42_volume::Q42RangeSource>(
    volume: &crate::q42_volume::Q42RangeVolume<S>,
    plan: Q42RangeJoinSelectPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeJoinSelectState,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    left_rows: &mut [BindingRow],
    right_rows: &mut [BindingRow],
    join_out: &mut [BindingRow],
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
        if state.wrappers.emitted >= plan.limit {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        let page = execute_range_nested_loop_join_page_into(
            volume,
            plan.join,
            ctx,
            &mut state.join,
            compressed,
            decoded,
            quin_scratch,
            left_rows,
            right_rows,
            join_out,
        )?;
        let applied = apply_select_wrappers(
            ctx,
            &plan.filters,
            plan.filter_count,
            plan.projection,
            plan.projection_count,
            plan.limit,
            plan.offset,
            &mut state.wrappers,
            &join_out[..page.returned],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if page.done {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        if applied.limit_reached {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        if applied.returned == 0 && !page.done {
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

pub fn execute_range_volume_set_join_select_page_into<S: crate::q42_volume::Q42RangeSource>(
    volumes: &crate::q42_volume::Q42RangeVolumeSet<S>,
    plan: Q42RangeJoinSelectPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeVolumeSetJoinSelectState,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    left_rows: &mut [BindingRow],
    right_rows: &mut [BindingRow],
    join_out: &mut [BindingRow],
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
        if state.wrappers.emitted >= plan.limit {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        let page = execute_range_volume_set_nested_loop_join_page_into(
            volumes,
            plan.join,
            ctx,
            &mut state.join,
            compressed,
            decoded,
            quin_scratch,
            left_rows,
            right_rows,
            join_out,
        )?;
        let applied = apply_select_wrappers(
            ctx,
            &plan.filters,
            plan.filter_count,
            plan.projection,
            plan.projection_count,
            plan.limit,
            plan.offset,
            &mut state.wrappers,
            &join_out[..page.returned],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if page.done || applied.limit_reached {
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
    use crate::q42_volume::{LocalFileRangeSource, Q42RangeVolume};
    use crate::sparql_planner::{ExecutionPlan, PhysicalOperatorType};

    #[test]
    fn peels_project_filter_limit_from_a_join_tree() {
        let mut plan = ExecutionPlan::new();
        let left = plan
            .add_operator(
                PhysicalOperatorType::TripleScan {
                    subject: 10,
                    predicate: 20,
                    object: 0,
                },
                1,
            )
            .unwrap();
        let right = plan
            .add_operator(
                PhysicalOperatorType::TripleScan {
                    subject: 0,
                    predicate: 40,
                    object: 50,
                },
                1,
            )
            .unwrap();
        let join = plan
            .add_operator(
                PhysicalOperatorType::NestedLoopJoin {
                    left,
                    right,
                    join_var: 0,
                },
                1,
            )
            .unwrap();
        let limited = plan
            .add_operator(
                PhysicalOperatorType::Limit {
                    input: join,
                    limit: 1,
                    offset: 0,
                },
                1,
            )
            .unwrap();
        let projected = plan
            .add_operator(
                PhysicalOperatorType::Project {
                    input: limited,
                    vars: {
                        let mut vars = [0; MAX_VARIABLES];
                        vars[0] = 0;
                        vars
                    },
                    var_count: 1,
                },
                1,
            )
            .unwrap();
        plan.root_operator = projected;
        let compiled = Q42RangeJoinSelectPlan::from_execution_plan(&plan).unwrap();
        assert_eq!(compiled.limit, 1);
        assert_eq!(compiled.projection_count, 1);
        assert_eq!(compiled.join.left.subject, 10);
        assert_eq!(compiled.join.right.object, 50);
    }

    #[test]
    fn limit_stops_after_one_join_row() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("join-select.q42");
        let left = NQuin {
            subject: 10,
            predicate: 20,
            object: 30,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let right = NQuin {
            subject: 30,
            predicate: 40,
            object: 50,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        crate::q42_volume::write_unified_volume(
            &path,
            &std::collections::HashMap::new(),
            &[(left.object, right.object)],
            &[vec![left, right]],
        )
        .unwrap();
        let source = LocalFileRangeSource::open(&path).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        let mut context = SparqlQueryContext::new();
        context.variable_count = 1;
        let plan = Q42RangeJoinSelectPlan {
            join: Q42RangeNestedLoopJoinPlan {
                left: super::super::sparql_executor::Q42RangeTriplePattern {
                    subject: left.subject,
                    predicate: left.predicate,
                    object: 0,
                },
                right: super::super::sparql_executor::Q42RangeTriplePattern {
                    subject: 0,
                    predicate: right.predicate,
                    object: right.object,
                },
            },
            projection: {
                let mut vars = [0; MAX_VARIABLES];
                vars[0] = 0;
                vars
            },
            projection_count: 1,
            filters: [0; 8],
            filter_count: 0,
            limit: 1,
            offset: 0,
        };
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 1];
        let mut left_rows = [BindingRow::default(); 1];
        let mut right_rows = [BindingRow::default(); 1];
        let mut join_out = [BindingRow::default(); 1];
        let mut out = [BindingRow::default(); 1];
        let mut state = Q42RangeJoinSelectState::default();
        let page = execute_range_join_select_page_into(
            &volume,
            plan,
            &context,
            &mut state,
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut left_rows,
            &mut right_rows,
            &mut join_out,
            &mut out,
        )
        .unwrap();
        assert_eq!(page.returned, 1);
        assert!(page.done);
        assert_eq!(out[0].get(0), Some(right.subject));
    }
}
