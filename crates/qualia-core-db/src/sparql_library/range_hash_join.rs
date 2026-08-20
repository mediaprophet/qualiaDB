//! Bounded range HashJoin. The smaller TripleScan is hashed into a
//! caller-capped table; the larger side probes it. If the build side would
//! exceed the table, the operator fails closed so the planner can keep
//! NestedLoopJoin. No unbounded allocation.

use super::range_join_select::Q42RangeJoinSelectPlan;
use super::range_select_apply::{apply_select_wrappers, SelectWrapperState};
use super::sparql_ast::{
    BindingRow, ExpressionId, SparqlQueryContext, VariableId, MAX_BINDINGS, MAX_VARIABLES,
};
use super::sparql_executor::{
    execute_range_triple_page_into, execute_range_volume_set_triple_page_into,
    Q42RangeNestedLoopJoinPage, Q42RangeNestedLoopJoinPlan, Q42RangeSparqlCursor,
    Q42RangeTriplePattern, Q42RangeVolumeSetSparqlCursor,
};
use super::sparql_planner::{ExecutionPlan, PhysicalOperatorType};
use crate::NQuin;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeHashJoinSlot {
    pub key: u64,
    pub row: BindingRow,
    pub occupied: bool,
}

impl Default for Q42RangeHashJoinSlot {
    fn default() -> Self {
        Self {
            key: 0,
            row: BindingRow::default(),
            occupied: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeHashJoinPlan {
    pub left: Q42RangeTriplePattern,
    pub right: Q42RangeTriplePattern,
    pub join_var: VariableId,
    pub projection: [VariableId; MAX_VARIABLES],
    pub projection_count: u8,
    pub filters: [ExpressionId; 8],
    pub filter_count: u8,
    pub limit: u64,
    pub offset: u64,
}

impl Q42RangeHashJoinPlan {
    pub fn as_join_select(&self) -> Q42RangeJoinSelectPlan {
        Q42RangeJoinSelectPlan {
            join: Q42RangeNestedLoopJoinPlan {
                left: self.left,
                right: self.right,
            },
            projection: self.projection,
            projection_count: self.projection_count,
            filters: self.filters,
            filter_count: self.filter_count,
            limit: self.limit,
            offset: self.offset,
        }
    }

    pub fn from_execution_plan(plan: &ExecutionPlan) -> Result<Self, String> {
        if plan.operator_count == 0 || plan.root_operator as usize >= plan.operator_count as usize {
            return Err("range HashJoin requires a non-empty execution plan".into());
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
                        return Err("range HashJoin does not support nested projections".into());
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
                        return Err("range HashJoin does not support nested limits".into());
                    }
                    limit = configured;
                    offset = configured_offset;
                    operator = input;
                }
                PhysicalOperatorType::Filter { input, expression } => {
                    if filter_count == filters.len() {
                        return Err(
                            "range HashJoin supports at most eight stacked FILTER operators".into(),
                        );
                    }
                    filters[filter_count] = expression;
                    filter_count += 1;
                    operator = input;
                }
                PhysicalOperatorType::HashJoin {
                    left,
                    right,
                    join_var,
                }
                | PhysicalOperatorType::NestedLoopJoin {
                    left,
                    right,
                    join_var,
                } => {
                    return Ok(Self {
                        left: triple_scan(plan, left)?,
                        right: triple_scan(plan, right)?,
                        join_var,
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
                        "range HashJoin supports Project/Filter/Limit over HashJoin or NestedLoopJoin(TripleScan, TripleScan)"
                            .into(),
                    );
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeHashJoinState {
    pub built: bool,
    pub build_is_right: bool,
    pub probe_scan: Q42RangeSparqlCursor,
    pub probe_count: usize,
    pub probe_index: usize,
    pub probe_exhausted: bool,
    pub match_offset: usize,
    pub wrappers: SelectWrapperState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeVolumeSetHashJoinState {
    pub built: bool,
    pub build_is_right: bool,
    pub probe_scan: Q42RangeVolumeSetSparqlCursor,
    pub probe_count: usize,
    pub probe_index: usize,
    pub probe_exhausted: bool,
    pub match_offset: usize,
    pub wrappers: SelectWrapperState,
}

fn triple_scan(plan: &ExecutionPlan, operator: u16) -> Result<Q42RangeTriplePattern, String> {
    let Some(entry) = plan.operators.get(operator as usize) else {
        return Err("range HashJoin input operator is out of bounds".into());
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
        _ => Err("range HashJoin currently requires TripleScan inputs".into()),
    }
}

fn cap_error(count: usize, cap: usize) -> String {
    format!(
        "range HashJoin build side ({count} rows) exceeds caller cap ({cap}); planner must keep NestedLoopJoin"
    )
}

fn term_var(term: u64, ctx: &SparqlQueryContext) -> Option<VariableId> {
    if (term as usize) < ctx.variable_count {
        Some(term as VariableId)
    } else {
        None
    }
}

fn pattern_vars(pattern: Q42RangeTriplePattern, ctx: &SparqlQueryContext) -> ([VariableId; 3], u8) {
    let mut vars = [0; 3];
    let mut count = 0u8;
    for term in [pattern.subject, pattern.predicate, pattern.object] {
        if let Some(var) = term_var(term, ctx) {
            if !vars[..count as usize].contains(&var) {
                vars[count as usize] = var;
                count += 1;
            }
        }
    }
    (vars, count)
}

fn shared_join_vars(
    left: Q42RangeTriplePattern,
    right: Q42RangeTriplePattern,
    ctx: &SparqlQueryContext,
    join_var: VariableId,
) -> ([VariableId; 3], u8) {
    let (left_vars, left_n) = pattern_vars(left, ctx);
    let (right_vars, right_n) = pattern_vars(right, ctx);
    let mut shared = [0; 3];
    let mut count = 0u8;
    let left_slice = &left_vars[..left_n as usize];
    let right_slice = &right_vars[..right_n as usize];
    if left_slice.contains(&join_var) && right_slice.contains(&join_var) {
        shared[0] = join_var;
        count = 1;
    }
    for var in left_slice {
        if right_slice.contains(var) && !shared[..count as usize].contains(var) && count < 3 {
            shared[count as usize] = *var;
            count += 1;
        }
    }
    (shared, count)
}

fn join_key(row: &BindingRow, vars: &[VariableId]) -> u64 {
    for var in vars {
        if let Some(value) = row.get(*var) {
            return value;
        }
    }
    0
}

fn rows_compatible(left: &BindingRow, right: &BindingRow) -> bool {
    for slot in 0..MAX_BINDINGS {
        if let (Some(a), Some(b)) = (left.slots[slot], right.slots[slot]) {
            if a != b {
                return false;
            }
        }
    }
    true
}

fn merge_rows(left: &BindingRow, right: &BindingRow) -> BindingRow {
    let mut joined = BindingRow::new();
    for slot in 0..MAX_BINDINGS {
        joined.slots[slot] = left.slots[slot].or(right.slots[slot]);
    }
    joined
}

fn insert_slot(
    table: &mut [Q42RangeHashJoinSlot],
    key: u64,
    row: BindingRow,
) -> Result<(), String> {
    if table.is_empty() {
        return Err(cap_error(1, 0));
    }
    let cap = table.len();
    let start = (key as usize) % cap;
    for step in 0..cap {
        let slot = &mut table[(start + step) % cap];
        if !slot.occupied {
            slot.occupied = true;
            slot.key = key;
            slot.row = row;
            return Ok(());
        }
    }
    Err(cap_error(cap + 1, cap))
}

fn count_side<C, F>(
    mut page_fn: F,
    pattern: Q42RangeTriplePattern,
    scratch: &mut [BindingRow],
) -> Result<usize, String>
where
    C: Copy + Default,
    F: FnMut(Q42RangeTriplePattern, C, &mut [BindingRow]) -> Result<(usize, Option<C>), String>,
{
    let mut cursor = C::default();
    let mut total = 0usize;
    loop {
        let (count, next) = page_fn(pattern, cursor, scratch)?;
        total = total.saturating_add(count);
        match next {
            Some(next_cursor) => cursor = next_cursor,
            None => return Ok(total),
        }
    }
}

fn build_side<C, F>(
    mut page_fn: F,
    pattern: Q42RangeTriplePattern,
    key_vars: &[VariableId],
    table: &mut [Q42RangeHashJoinSlot],
    scratch: &mut [BindingRow],
) -> Result<usize, String>
where
    C: Copy + Default,
    F: FnMut(Q42RangeTriplePattern, C, &mut [BindingRow]) -> Result<(usize, Option<C>), String>,
{
    let mut cursor = C::default();
    let mut inserted = 0usize;
    loop {
        let (count, next) = page_fn(pattern, cursor, scratch)?;
        for row in &scratch[..count] {
            insert_slot(table, join_key(row, key_vars), *row)?;
            inserted += 1;
        }
        match next {
            Some(next_cursor) => cursor = next_cursor,
            None => return Ok(inserted),
        }
    }
}

fn emit_matches(
    table: &[Q42RangeHashJoinSlot],
    probe: &BindingRow,
    key_vars: &[VariableId],
    match_offset: &mut usize,
    out: &mut [BindingRow],
) -> usize {
    if table.is_empty() {
        *match_offset = 0;
        return 0;
    }
    let key = join_key(probe, key_vars);
    let cap = table.len();
    let start = (key as usize) % cap;
    let mut returned = 0usize;
    while *match_offset < cap && returned < out.len() {
        let slot = &table[(start + *match_offset) % cap];
        *match_offset += 1;
        if !slot.occupied {
            *match_offset = cap;
            break;
        }
        if slot.key != key || !rows_compatible(probe, &slot.row) {
            continue;
        }
        out[returned] = merge_rows(probe, &slot.row);
        returned += 1;
    }
    returned
}

fn emit_matches_accel(
    table: &[Q42RangeHashJoinSlot],
    probes: &[BindingRow],
    key_vars: &[VariableId],
    out: &mut [BindingRow],
) -> usize {
    let mut build_keys = Vec::new();
    let mut build_rows = Vec::new();
    for slot in table {
        if slot.occupied {
            build_keys.push(slot.key);
            build_rows.push(slot.row);
        }
    }
    let probe_keys: Vec<u64> = probes.iter().map(|r| join_key(r, key_vars)).collect();
    let mut pairs = vec![(0u32, 0u32); out.len()];
    let joined = crate::query::graph_accel::hash_join_u64(&build_keys, &probe_keys, &mut pairs);
    let mut n = 0usize;
    for &(pi, bi) in &pairs[..joined.written] {
        if n >= out.len() {
            break;
        }
        let probe = &probes[pi as usize];
        let built = &build_rows[bi as usize];
        if !rows_compatible(probe, built) {
            continue;
        }
        out[n] = merge_rows(probe, built);
        n += 1;
    }
    n
}

pub fn execute_range_hash_join_page_into<S: crate::q42_volume::Q42RangeSource>(
    volume: &crate::q42_volume::Q42RangeVolume<S>,
    plan: Q42RangeHashJoinPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeHashJoinState,
    table: &mut [Q42RangeHashJoinSlot],
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    probe_rows: &mut [BindingRow],
    join_out: &mut [BindingRow],
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String> {
    let mut page_fn =
        |pattern: Q42RangeTriplePattern, cursor: Q42RangeSparqlCursor, rows: &mut [BindingRow]| {
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
        };
    if !state.built {
        let left_n = count_side(&mut page_fn, plan.left, probe_rows)?;
        let right_n = count_side(&mut page_fn, plan.right, probe_rows)?;
        let build_is_right = right_n <= left_n;
        let build_n = if build_is_right { right_n } else { left_n };
        if build_n > table.len() {
            return Err(cap_error(build_n, table.len()));
        }
        let build_pattern = if build_is_right {
            plan.right
        } else {
            plan.left
        };
        let (key_vars, key_n) = shared_join_vars(plan.left, plan.right, ctx, plan.join_var);
        build_side(
            &mut page_fn,
            build_pattern,
            &key_vars[..key_n as usize],
            table,
            probe_rows,
        )?;
        state.built = true;
        state.build_is_right = build_is_right;
        state.probe_scan = Q42RangeSparqlCursor::default();
        state.probe_exhausted = false;
        state.probe_count = 0;
        state.probe_index = 0;
        state.match_offset = 0;
    }
    let probe_pattern = if state.build_is_right {
        plan.left
    } else {
        plan.right
    };
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
        if state.probe_index >= state.probe_count {
            if state.probe_exhausted {
                return Ok(Q42RangeNestedLoopJoinPage {
                    returned,
                    done: true,
                });
            }
            let (count, next) = page_fn(probe_pattern, state.probe_scan, probe_rows)?;
            state.probe_count = count;
            state.probe_index = 0;
            state.match_offset = 0;
            state.probe_scan = next.unwrap_or_default();
            state.probe_exhausted = next.is_none();
            if state.probe_count == 0 {
                if state.probe_exhausted {
                    return Ok(Q42RangeNestedLoopJoinPage {
                        returned,
                        done: true,
                    });
                }
                continue;
            }
        }
        let (key_vars, key_n) = shared_join_vars(plan.left, plan.right, ctx, plan.join_var);
        if state.probe_index == 0
            && state.match_offset == 0
            && state.probe_count >= 32
            && table.iter().filter(|s| s.occupied).count() >= 32
        {
            let produced = emit_matches_accel(
                table,
                &probe_rows[..state.probe_count],
                &key_vars[..key_n as usize],
                join_out,
            );
            state.probe_index = state.probe_count;
            let applied = apply_select_wrappers(
                ctx,
                &plan.filters,
                plan.filter_count,
                plan.projection,
                plan.projection_count,
                plan.limit,
                plan.offset,
                &mut state.wrappers,
                &join_out[..produced],
                &mut out[returned..],
            )?;
            returned += applied.returned;
            if applied.limit_reached || returned == out.len() {
                return Ok(Q42RangeNestedLoopJoinPage {
                    returned,
                    done: applied.limit_reached,
                });
            }
            continue;
        }
        let probe = probe_rows[state.probe_index];
        let produced = emit_matches(
            table,
            &probe,
            &key_vars[..key_n as usize],
            &mut state.match_offset,
            join_out,
        );
        let applied = apply_select_wrappers(
            ctx,
            &plan.filters,
            plan.filter_count,
            plan.projection,
            plan.projection_count,
            plan.limit,
            plan.offset,
            &mut state.wrappers,
            &join_out[..produced],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if state.match_offset >= table.len() || table.is_empty() {
            state.probe_index += 1;
            state.match_offset = 0;
        }
        if applied.limit_reached {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        if returned == out.len() {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: false,
            });
        }
    }
}

pub fn execute_range_volume_set_hash_join_page_into<S: crate::q42_volume::Q42RangeSource>(
    volumes: &crate::q42_volume::Q42RangeVolumeSet<S>,
    plan: Q42RangeHashJoinPlan,
    ctx: &SparqlQueryContext,
    state: &mut Q42RangeVolumeSetHashJoinState,
    table: &mut [Q42RangeHashJoinSlot],
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    probe_rows: &mut [BindingRow],
    join_out: &mut [BindingRow],
    out: &mut [BindingRow],
) -> Result<Q42RangeNestedLoopJoinPage, String> {
    let mut page_fn = |pattern: Q42RangeTriplePattern,
                       cursor: Q42RangeVolumeSetSparqlCursor,
                       rows: &mut [BindingRow]| {
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
    };
    if !state.built {
        let left_n = count_side(&mut page_fn, plan.left, probe_rows)?;
        let right_n = count_side(&mut page_fn, plan.right, probe_rows)?;
        let build_is_right = right_n <= left_n;
        let build_n = if build_is_right { right_n } else { left_n };
        if build_n > table.len() {
            return Err(cap_error(build_n, table.len()));
        }
        let build_pattern = if build_is_right {
            plan.right
        } else {
            plan.left
        };
        let (key_vars, key_n) = shared_join_vars(plan.left, plan.right, ctx, plan.join_var);
        build_side(
            &mut page_fn,
            build_pattern,
            &key_vars[..key_n as usize],
            table,
            probe_rows,
        )?;
        state.built = true;
        state.build_is_right = build_is_right;
        state.probe_scan = Q42RangeVolumeSetSparqlCursor::default();
        state.probe_exhausted = false;
        state.probe_count = 0;
        state.probe_index = 0;
        state.match_offset = 0;
    }
    let probe_pattern = if state.build_is_right {
        plan.left
    } else {
        plan.right
    };
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
        if state.probe_index >= state.probe_count {
            if state.probe_exhausted {
                return Ok(Q42RangeNestedLoopJoinPage {
                    returned,
                    done: true,
                });
            }
            let (count, next) = page_fn(probe_pattern, state.probe_scan, probe_rows)?;
            state.probe_count = count;
            state.probe_index = 0;
            state.match_offset = 0;
            state.probe_scan = next.unwrap_or_default();
            state.probe_exhausted = next.is_none();
            if state.probe_count == 0 {
                if state.probe_exhausted {
                    return Ok(Q42RangeNestedLoopJoinPage {
                        returned,
                        done: true,
                    });
                }
                continue;
            }
        }
        let (key_vars, key_n) = shared_join_vars(plan.left, plan.right, ctx, plan.join_var);
        if state.probe_index == 0
            && state.match_offset == 0
            && state.probe_count >= 32
            && table.iter().filter(|s| s.occupied).count() >= 32
        {
            let produced = emit_matches_accel(
                table,
                &probe_rows[..state.probe_count],
                &key_vars[..key_n as usize],
                join_out,
            );
            state.probe_index = state.probe_count;
            let applied = apply_select_wrappers(
                ctx,
                &plan.filters,
                plan.filter_count,
                plan.projection,
                plan.projection_count,
                plan.limit,
                plan.offset,
                &mut state.wrappers,
                &join_out[..produced],
                &mut out[returned..],
            )?;
            returned += applied.returned;
            if applied.limit_reached || returned == out.len() {
                return Ok(Q42RangeNestedLoopJoinPage {
                    returned,
                    done: applied.limit_reached,
                });
            }
            continue;
        }
        let probe = probe_rows[state.probe_index];
        let produced = emit_matches(
            table,
            &probe,
            &key_vars[..key_n as usize],
            &mut state.match_offset,
            join_out,
        );
        let applied = apply_select_wrappers(
            ctx,
            &plan.filters,
            plan.filter_count,
            plan.projection,
            plan.projection_count,
            plan.limit,
            plan.offset,
            &mut state.wrappers,
            &join_out[..produced],
            &mut out[returned..],
        )?;
        returned += applied.returned;
        if state.match_offset >= table.len() || table.is_empty() {
            state.probe_index += 1;
            state.match_offset = 0;
        }
        if applied.limit_reached {
            return Ok(Q42RangeNestedLoopJoinPage {
                returned,
                done: true,
            });
        }
        if returned == out.len() {
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
        let path = dir.path().join("hash-join.q42");
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

    fn join_plan() -> Q42RangeHashJoinPlan {
        Q42RangeHashJoinPlan {
            left: Q42RangeTriplePattern {
                subject: 0,
                predicate: 20,
                object: 1,
            },
            right: Q42RangeTriplePattern {
                subject: 1,
                predicate: 40,
                object: 2,
            },
            join_var: 1,
            projection: [0; MAX_VARIABLES],
            projection_count: 0,
            filters: [0; 8],
            filter_count: 0,
            limit: u64::MAX,
            offset: 0,
        }
    }

    #[test]
    fn accepts_nested_loop_join_tree() {
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
        plan.root_operator = plan
            .add_operator(
                PhysicalOperatorType::NestedLoopJoin {
                    left,
                    right,
                    join_var: 1,
                },
                1,
            )
            .unwrap();
        let compiled = Q42RangeHashJoinPlan::from_execution_plan(&plan).unwrap();
        assert_eq!(compiled.join_var, 1);
        assert_eq!(compiled.right.predicate, 40);
    }

    #[test]
    fn hashes_smaller_side_and_joins() {
        let left = quin(10, 20, 30);
        let right = quin(30, 40, 50);
        let extra = quin(31, 40, 51);
        let (_dir, volume) = test_volume(&[left, right, extra]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 3;
        let mut table = [Q42RangeHashJoinSlot::default(); 8];
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 4];
        let mut probe_rows = [BindingRow::default(); 4];
        let mut join_out = [BindingRow::default(); 4];
        let mut out = [BindingRow::default(); 4];
        let mut state = Q42RangeHashJoinState::default();
        let page = execute_range_hash_join_page_into(
            &volume,
            join_plan(),
            &ctx,
            &mut state,
            &mut table,
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut probe_rows,
            &mut join_out,
            &mut out,
        )
        .unwrap();
        assert!(page.done);
        assert_eq!(page.returned, 1);
        assert_eq!(out[0].get(0), Some(10));
        assert_eq!(out[0].get(1), Some(30));
        assert_eq!(out[0].get(2), Some(50));
        assert!(!state.build_is_right);
    }

    #[test]
    fn cap_exceeded_tells_planner_to_keep_nested_loop() {
        let left_a = quin(10, 20, 30);
        let left_b = quin(11, 20, 32);
        let right_a = quin(30, 40, 50);
        let right_b = quin(32, 40, 51);
        let (_dir, volume) = test_volume(&[left_a, left_b, right_a, right_b]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 3;
        let mut table = [Q42RangeHashJoinSlot::default(); 1];
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 4];
        let mut probe_rows = [BindingRow::default(); 4];
        let mut join_out = [BindingRow::default(); 4];
        let mut out = [BindingRow::default(); 4];
        let mut state = Q42RangeHashJoinState::default();
        let err = execute_range_hash_join_page_into(
            &volume,
            join_plan(),
            &ctx,
            &mut state,
            &mut table,
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut probe_rows,
            &mut join_out,
            &mut out,
        )
        .unwrap_err();
        assert!(
            err.contains("planner must keep NestedLoopJoin"),
            "unexpected error: {err}"
        );
        assert!(!state.built);
    }

    #[test]
    fn empty_table_is_a_cap_error() {
        let left = quin(10, 20, 30);
        let right = quin(30, 40, 50);
        let (_dir, volume) = test_volume(&[left, right]);
        let mut ctx = SparqlQueryContext::new();
        ctx.variable_count = 3;
        let mut table = [];
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 2];
        let mut probe_rows = [BindingRow::default(); 2];
        let mut join_out = [BindingRow::default(); 2];
        let mut out = [BindingRow::default(); 2];
        let mut state = Q42RangeHashJoinState::default();
        let err = execute_range_hash_join_page_into(
            &volume,
            join_plan(),
            &ctx,
            &mut state,
            &mut table,
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut probe_rows,
            &mut join_out,
            &mut out,
        )
        .unwrap_err();
        assert!(err.contains("NestedLoopJoin"));
    }
}
