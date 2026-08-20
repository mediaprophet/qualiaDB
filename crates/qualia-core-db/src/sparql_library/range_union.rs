//! Range-native SPARQL UNION over two paged TripleScans.
//!
//! SPARQL UNION is a bag union: left pages are emitted first, then right
//! pages. No deduplication. Project / Filter / Limit wrappers sit above the
//! union and are applied per page.

use super::range_select_apply::{apply_select_wrappers, SelectWrapperState};
use super::sparql_ast::{BindingRow, ExpressionId, SparqlQueryContext, VariableId, MAX_VARIABLES};
use super::sparql_executor::{
    execute_range_triple_page_into, execute_range_volume_set_triple_page_into,
    Q42RangeNestedLoopJoinPage, Q42RangeSparqlCursor, Q42RangeTriplePattern,
    Q42RangeVolumeSetSparqlCursor,
};
use super::sparql_planner::{ExecutionPlan, PhysicalOperatorType};
use crate::NQuin;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeUnionPlan {
    pub left: Q42RangeTriplePattern,
    pub right: Q42RangeTriplePattern,
    pub projection: [VariableId; MAX_VARIABLES],
    pub projection_count: u8,
    pub filters: [ExpressionId; 8],
    pub filter_count: u8,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeUnionState {
    pub left_scan: Q42RangeSparqlCursor,
    pub right_scan: Q42RangeSparqlCursor,
    pub left_done: bool,
    pub right_done: bool,
    pub wrappers: SelectWrapperState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeVolumeSetUnionState {
    pub left_scan: Q42RangeVolumeSetSparqlCursor,
    pub right_scan: Q42RangeVolumeSetSparqlCursor,
    pub left_done: bool,
    pub right_done: bool,
    pub wrappers: SelectWrapperState,
}

impl Q42RangeUnionPlan {
    pub fn from_execution_plan(plan: &ExecutionPlan) -> Result<Self, String> {
        if plan.operator_count == 0 || plan.root_operator as usize >= plan.operator_count as usize {
            return Err("range UNION requires a non-empty execution plan".into());
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
                        return Err("range UNION does not support nested projections".into());
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
                        return Err("range UNION does not support nested limits".into());
                    }
                    limit = configured;
                    offset = configured_offset;
                    operator = input;
                }
                PhysicalOperatorType::Filter { input, expression } => {
                    if filter_count == filters.len() {
                        return Err(
                            "range UNION supports at most eight stacked FILTER operators".into(),
                        );
                    }
                    filters[filter_count] = expression;
                    filter_count += 1;
                    operator = input;
                }
                PhysicalOperatorType::Union { left, right } => {
                    return Ok(Self {
                        left: triple_scan(plan, left)?,
                        right: triple_scan(plan, right)?,
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
                        "range UNION supports Project/Filter/Limit over Union(TripleScan, TripleScan)"
                            .into(),
                    );
                }
            }
        }
    }
}

fn triple_scan(plan: &ExecutionPlan, operator: u16) -> Result<Q42RangeTriplePattern, String> {
    let Some(entry) = plan.operators.get(operator as usize) else {
        return Err("range UNION input operator is out of bounds".into());
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
        _ => Err("range UNION currently requires TripleScan inputs".into()),
    }
}

pub fn execute_range_union_page_into<S: crate::q42_volume::Q42RangeSource>(
    volume: &crate::q42_volume::Q42RangeVolume<S>,
    plan: Q42RangeUnionPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeUnionState,
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
        if state.wrappers.emitted >= plan.limit {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        let produced = fill_union_page(
            |pattern, cursor, rows| {
                execute_range_triple_page_into(
                    volume,
                    pattern.subject,
                    pattern.predicate,
                    pattern.object,
                    None,
                    ctx,
                    &BindingRow::default(),
                    cursor,
                    compressed,
                    decoded,
                    quin_scratch,
                    rows,
                )
                .map(|page| (page.returned, page.next_cursor))
            },
            plan,
            &mut state.left_scan,
            &mut state.right_scan,
            &mut state.left_done,
            &mut state.right_done,
            raw,
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
            &raw[..produced.returned],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if produced.done || applied.limit_reached {
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

pub fn execute_range_volume_set_union_page_into<S: crate::q42_volume::Q42RangeSource>(
    volumes: &crate::q42_volume::Q42RangeVolumeSet<S>,
    plan: Q42RangeUnionPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeVolumeSetUnionState,
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
        if state.wrappers.emitted >= plan.limit {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        let produced = fill_union_page(
            |pattern, cursor, rows| {
                execute_range_volume_set_triple_page_into(
                    volumes,
                    pattern.subject,
                    pattern.predicate,
                    pattern.object,
                    None,
                    ctx,
                    &BindingRow::default(),
                    cursor,
                    compressed,
                    decoded,
                    quin_scratch,
                    rows,
                )
                .map(|page| (page.returned, page.next_cursor))
            },
            plan,
            &mut state.left_scan,
            &mut state.right_scan,
            &mut state.left_done,
            &mut state.right_done,
            raw,
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
            &raw[..produced.returned],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if produced.done || applied.limit_reached {
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

fn fill_union_page<C, F>(
    mut page_fn: F,
    plan: Q42RangeUnionPlan,
    left_scan: &mut C,
    right_scan: &mut C,
    left_done: &mut bool,
    right_done: &mut bool,
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String>
where
    C: Copy + Default,
    F: FnMut(Q42RangeTriplePattern, C, &mut [BindingRow]) -> Result<(usize, Option<C>), String>,
{
    if !*left_done {
        let (count, next) = page_fn(plan.left, *left_scan, out)?;
        *left_scan = next.unwrap_or_default();
        *left_done = next.is_none();
        if count > 0 || !*left_done {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned: count,
                done: false,
            });
        }
    }
    if !*right_done {
        let (count, next) = page_fn(plan.right, *right_scan, out)?;
        *right_scan = next.unwrap_or_default();
        *right_done = next.is_none();
        return Ok(Q42RangeNestedLoopJoinPage {
            returned: count,
            done: *right_done && count == 0,
        });
    }
    Ok(Q42RangeNestedLoopJoinPage {
        returned: 0,
        done: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{write_unified_volume, LocalFileRangeSource, Q42RangeVolume};

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
        let path = dir.path().join("union.q42");
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
    fn peels_project_from_union_tree() {
        let mut plan = ExecutionPlan::new();
        let left = plan
            .add_operator(
                PhysicalOperatorType::TripleScan {
                    subject: 0,
                    predicate: 20,
                    object: 1,
                },
                1,
            )
            .unwrap();
        let right = plan
            .add_operator(
                PhysicalOperatorType::TripleScan {
                    subject: 0,
                    predicate: 40,
                    object: 1,
                },
                1,
            )
            .unwrap();
        let union = plan
            .add_operator(PhysicalOperatorType::Union { left, right }, 2)
            .unwrap();
        let mut vars = [0; MAX_VARIABLES];
        vars[0] = 0;
        plan.root_operator = plan
            .add_operator(
                PhysicalOperatorType::Project {
                    input: union,
                    vars,
                    var_count: 1,
                },
                2,
            )
            .unwrap();
        let compiled = Q42RangeUnionPlan::from_execution_plan(&plan).unwrap();
        assert_eq!(compiled.left.predicate, 20);
        assert_eq!(compiled.right.predicate, 40);
        assert_eq!(compiled.projection_count, 1);
    }

    #[test]
    fn concatenates_left_then_right_without_dedup() {
        let left = quin(10, 20, 30);
        let right = quin(11, 40, 31);
        let (_dir, volume) = test_volume(&[left, right]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 2;
        let plan = Q42RangeUnionPlan {
            left: Q42RangeTriplePattern {
                subject: 0,
                predicate: 20,
                object: 1,
            },
            right: Q42RangeTriplePattern {
                subject: 0,
                predicate: 40,
                object: 1,
            },
            projection: [0; MAX_VARIABLES],
            projection_count: 0,
            filters: [0; 8],
            filter_count: 0,
            limit: u64::MAX,
            offset: 0,
        };
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 4];
        let mut raw = [BindingRow::default(); 4];
        let mut out = [BindingRow::default(); 4];
        let mut state = Q42RangeUnionState::default();
        let mut rows = Vec::new();
        loop {
            let page = execute_range_union_page_into(
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
            rows.extend_from_slice(&out[..page.returned]);
            if page.done {
                break;
            }
        }
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get(0), Some(10));
        assert_eq!(rows[1].get(0), Some(11));
    }

    #[test]
    fn empty_both_sides_is_done() {
        let only = quin(10, 99, 30);
        let (_dir, volume) = test_volume(&[only]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 1;
        let plan = Q42RangeUnionPlan {
            left: Q42RangeTriplePattern {
                subject: 0,
                predicate: 20,
                object: 1,
            },
            right: Q42RangeTriplePattern {
                subject: 0,
                predicate: 40,
                object: 1,
            },
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
        let mut state = Q42RangeUnionState::default();
        let page = execute_range_union_page_into(
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
        assert_eq!(page.returned, 0);
    }
}
