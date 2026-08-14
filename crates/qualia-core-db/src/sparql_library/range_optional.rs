//! Range-native SPARQL OPTIONAL (left join) over paged TripleScans.
//!
//! Each left row is emitted even when the right pattern produces no match.
//! Matching uses the existing range triple kernel so already-bound variables
//! stay compatible. Project / Filter / Limit wrappers are peeled, not
//! materialised.

use super::range_select_apply::{apply_select_wrappers, SelectWrapperState};
use super::sparql_ast::{BindingRow, ExpressionId, SparqlQueryContext, VariableId, MAX_VARIABLES};
use super::sparql_executor::{
    execute_range_triple_page_into, execute_range_volume_set_triple_page_into, Q42RangeNestedLoopJoinPage,
    Q42RangeSparqlCursor, Q42RangeTriplePattern, Q42RangeVolumeSetSparqlCursor,
};
use super::sparql_planner::{ExecutionPlan, PhysicalOperatorType};
use crate::NQuin;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeOptionalPlan {
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
pub struct Q42RangeOptionalState {
    pub left_scan: Q42RangeSparqlCursor,
    pub right_scan: Q42RangeSparqlCursor,
    pub left_count: usize,
    pub left_index: usize,
    pub left_exhausted: bool,
    pub right_active: bool,
    pub left_row_matched: bool,
    pub wrappers: SelectWrapperState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeVolumeSetOptionalState {
    pub left_scan: Q42RangeVolumeSetSparqlCursor,
    pub right_scan: Q42RangeVolumeSetSparqlCursor,
    pub left_count: usize,
    pub left_index: usize,
    pub left_exhausted: bool,
    pub right_active: bool,
    pub left_row_matched: bool,
    pub wrappers: SelectWrapperState,
}

impl Q42RangeOptionalPlan {
    pub fn from_execution_plan(plan: &ExecutionPlan) -> Result<Self, String> {
        if plan.operator_count == 0 || plan.root_operator as usize >= plan.operator_count as usize {
            return Err("range OPTIONAL requires a non-empty execution plan".into());
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
                        return Err("range OPTIONAL does not support nested projections".into());
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
                        return Err("range OPTIONAL does not support nested limits".into());
                    }
                    limit = configured;
                    offset = configured_offset;
                    operator = input;
                }
                PhysicalOperatorType::Filter { input, expression } => {
                    if filter_count == filters.len() {
                        return Err("range OPTIONAL supports at most eight stacked FILTER operators".into());
                    }
                    filters[filter_count] = expression;
                    filter_count += 1;
                    operator = input;
                }
                PhysicalOperatorType::Optional { left, right } => {
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
                        "range OPTIONAL supports Project/Filter/Limit over Optional(TripleScan, TripleScan)"
                            .into(),
                    );
                }
            }
        }
    }
}

fn triple_scan(plan: &ExecutionPlan, operator: u16) -> Result<Q42RangeTriplePattern, String> {
    let Some(entry) = plan.operators.get(operator as usize) else {
        return Err("range OPTIONAL input operator is out of bounds".into());
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
        _ => Err("range OPTIONAL currently requires TripleScan inputs".into()),
    }
}

pub fn execute_range_optional_page_into<S: crate::q42_volume::Q42RangeSource>(
    volume: &crate::q42_volume::Q42RangeVolume<S>,
    plan: Q42RangeOptionalPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeOptionalState,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    left_rows: &mut [BindingRow],
    right_rows: &mut [BindingRow],
    join_out: &mut [BindingRow],
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String> {
    if quin_scratch.is_empty()
        || left_rows.len() < quin_scratch.len()
        || right_rows.len() < quin_scratch.len()
        || join_out.len() < quin_scratch.len()
    {
        return Err(
            "range OPTIONAL requires row buffers at least as large as Quin scratch".into(),
        );
    }
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
        let scratch_len = quin_scratch.len();
        let produced = fill_optional_page(
            |pattern, input, cursor, rows| {
                execute_range_triple_page_into(
                    volume,
                    pattern.subject,
                    pattern.predicate,
                    pattern.object,
                    None,
                    ctx,
                    input,
                    cursor,
                    compressed,
                    decoded,
                    quin_scratch,
                    rows,
                )
                .map(|page| (page.returned, page.next_cursor))
            },
            plan,
            scratch_len,
            &mut state.left_scan,
            &mut state.right_scan,
            &mut state.left_count,
            &mut state.left_index,
            &mut state.left_exhausted,
            &mut state.right_active,
            &mut state.left_row_matched,
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
            &join_out[..produced.returned],
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

pub fn execute_range_volume_set_optional_page_into<S: crate::q42_volume::Q42RangeSource>(
    volumes: &crate::q42_volume::Q42RangeVolumeSet<S>,
    plan: Q42RangeOptionalPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeVolumeSetOptionalState,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    left_rows: &mut [BindingRow],
    right_rows: &mut [BindingRow],
    join_out: &mut [BindingRow],
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String> {
    if quin_scratch.is_empty()
        || left_rows.len() < quin_scratch.len()
        || right_rows.len() < quin_scratch.len()
        || join_out.len() < quin_scratch.len()
    {
        return Err(
            "range OPTIONAL requires row buffers at least as large as Quin scratch".into(),
        );
    }
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
        let scratch_len = quin_scratch.len();
        let produced = fill_optional_page(
            |pattern, input, cursor, rows| {
                execute_range_volume_set_triple_page_into(
                    volumes,
                    pattern.subject,
                    pattern.predicate,
                    pattern.object,
                    None,
                    ctx,
                    input,
                    cursor,
                    compressed,
                    decoded,
                    quin_scratch,
                    rows,
                )
                .map(|page| (page.returned, page.next_cursor))
            },
            plan,
            scratch_len,
            &mut state.left_scan,
            &mut state.right_scan,
            &mut state.left_count,
            &mut state.left_index,
            &mut state.left_exhausted,
            &mut state.right_active,
            &mut state.left_row_matched,
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
            &join_out[..produced.returned],
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

fn fill_optional_page<C, F>(
    mut page_fn: F,
    plan: Q42RangeOptionalPlan,
    scratch_len: usize,
    left_scan: &mut C,
    right_scan: &mut C,
    left_count: &mut usize,
    left_index: &mut usize,
    left_exhausted: &mut bool,
    right_active: &mut bool,
    left_row_matched: &mut bool,
    left_rows: &mut [BindingRow],
    right_rows: &mut [BindingRow],
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String>
where
    C: Copy + Default,
    F: FnMut(
        Q42RangeTriplePattern,
        &BindingRow,
        C,
        &mut [BindingRow],
    ) -> Result<(usize, Option<C>), String>,
{
    let mut returned = 0usize;
    loop {
        if returned + scratch_len > out.len() {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: false,
            });
        }
        if *left_index >= *left_count {
            if *left_exhausted {
                return Ok(Q42RangeNestedLoopJoinPage {
                    returned,
                    done: true,
                });
            }
            let (count, next) = page_fn(
                plan.left,
                &BindingRow::default(),
                *left_scan,
                left_rows,
            )?;
            *left_count = count;
            *left_index = 0;
            *left_scan = next.unwrap_or_default();
            *left_exhausted = next.is_none();
            *right_active = false;
            *left_row_matched = false;
            if *left_count == 0 {
                if *left_exhausted {
                    return Ok(Q42RangeNestedLoopJoinPage {
                        returned,
                        done: true,
                    });
                }
                continue;
            }
        }
        let input = left_rows[*left_index];
        if !*right_active {
            *left_row_matched = false;
        }
        let (count, next) = page_fn(
            plan.right,
            &input,
            if *right_active {
                *right_scan
            } else {
                C::default()
            },
            right_rows,
        )?;
        out[returned..returned + count].copy_from_slice(&right_rows[..count]);
        returned += count;
        if count > 0 {
            *left_row_matched = true;
        }
        match next {
            Some(cursor) => {
                *right_scan = cursor;
                *right_active = true;
            }
            None => {
                if !*left_row_matched {
                    if returned == out.len() {
                        return Ok(Q42RangeNestedLoopJoinPage {
                            returned,
                            done: false,
                        });
                    }
                    out[returned] = input;
                    returned += 1;
                }
                *left_index += 1;
                *right_scan = C::default();
                *right_active = false;
                *left_row_matched = false;
            }
        }
        if returned + scratch_len > out.len() {
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

    fn test_volume(quins: &[NQuin]) -> (tempfile::TempDir, Q42RangeVolume<LocalFileRangeSource>) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("optional.q42");
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
        let volume = Q42RangeVolume::open(source).unwrap();
        (dir, volume)
    }

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

    #[test]
    fn peels_project_from_optional_tree() {
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
                    subject: 1,
                    predicate: 40,
                    object: 2,
                },
                1,
            )
            .unwrap();
        let optional = plan
            .add_operator(PhysicalOperatorType::Optional { left, right }, 1)
            .unwrap();
        let mut vars = [0; MAX_VARIABLES];
        vars[0] = 0;
        plan.root_operator = plan
            .add_operator(
                PhysicalOperatorType::Project {
                    input: optional,
                    vars,
                    var_count: 1,
                },
                1,
            )
            .unwrap();
        let compiled = Q42RangeOptionalPlan::from_execution_plan(&plan).unwrap();
        assert_eq!(compiled.projection_count, 1);
        assert_eq!(compiled.left.predicate, 20);
        assert_eq!(compiled.right.predicate, 40);
    }

    #[test]
    fn unmatched_left_row_is_kept() {
        let required = quin(10, 20, 30);
        let unmatched = quin(11, 20, 31);
        let optional = quin(30, 40, 50);
        let (_dir, volume) = test_volume(&[required, unmatched, optional]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 2;
        let plan = Q42RangeOptionalPlan {
            left: Q42RangeTriplePattern {
                subject: 0,
                predicate: 20,
                object: 1,
            },
            right: Q42RangeTriplePattern {
                subject: 1,
                predicate: 40,
                object: 50,
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
        let mut left_rows = [BindingRow::default(); 4];
        let mut right_rows = [BindingRow::default(); 4];
        let mut join_out = [BindingRow::default(); 4];
        let mut out = [BindingRow::default(); 4];
        let mut state = Q42RangeOptionalState::default();
        let mut rows = Vec::new();
        loop {
            let page = execute_range_optional_page_into(
                &volume,
                plan,
                &ctx,
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
            rows.extend_from_slice(&out[..page.returned]);
            if page.done {
                break;
            }
        }
        assert_eq!(rows.len(), 2);
        let subjects: Vec<u64> = rows.iter().filter_map(|row| row.get(0)).collect();
        assert!(subjects.contains(&10));
        assert!(subjects.contains(&11));
        let matched = rows.iter().find(|row| row.get(0) == Some(10)).unwrap();
        assert_eq!(matched.get(1), Some(30));
        let kept = rows.iter().find(|row| row.get(0) == Some(11)).unwrap();
        assert_eq!(kept.get(1), Some(31));
    }

    #[test]
    fn empty_left_yields_no_rows() {
        let only_right = quin(30, 40, 50);
        let (_dir, volume) = test_volume(&[only_right]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 1;
        let plan = Q42RangeOptionalPlan {
            left: Q42RangeTriplePattern {
                subject: 0,
                predicate: 20,
                object: 99,
            },
            right: Q42RangeTriplePattern {
                subject: 0,
                predicate: 40,
                object: 50,
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
        let mut left_rows = [BindingRow::default(); 2];
        let mut right_rows = [BindingRow::default(); 2];
        let mut join_out = [BindingRow::default(); 2];
        let mut out = [BindingRow::default(); 2];
        let mut state = Q42RangeOptionalState::default();
        let page = execute_range_optional_page_into(
            &volume,
            plan,
            &ctx,
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
        assert!(page.done);
        assert_eq!(page.returned, 0);
    }
}
