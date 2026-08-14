//! Shared Project / Filter / Limit application for range SPARQL pages.

use super::sparql_ast::{BindingRow, ExpressionId, SparqlQueryContext, VariableId, MAX_VARIABLES};
use super::sparql_filter::{EvalResult, ExpressionEvaluator};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectWrapperState {
    pub skipped: u64,
    pub emitted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectApplyPage {
    pub returned: usize,
    pub limit_reached: bool,
}

pub fn apply_select_wrappers(
    ctx: &SparqlQueryContext,
    filters: &[ExpressionId],
    filter_count: u8,
    projection: [VariableId; MAX_VARIABLES],
    projection_count: u8,
    limit: u64,
    offset: u64,
    state: &mut SelectWrapperState,
    input: &[BindingRow],
    out: &mut [BindingRow],
) -> Result<SelectApplyPage, String> {
    let mut returned = 0usize;
    for row in input {
        let mut accepted = true;
        for filter in filters.iter().take(filter_count as usize) {
            if !matches!(
                ExpressionEvaluator::evaluate(*filter, ctx, row),
                Ok(EvalResult::Boolean(true))
            ) {
                accepted = false;
                break;
            }
        }
        if !accepted {
            continue;
        }
        if state.skipped < offset {
            state.skipped += 1;
            continue;
        }
        if state.emitted >= limit {
            return Ok(SelectApplyPage {
                returned,
                limit_reached: true,
            });
        }
        let mut projected = *row;
        if projection_count != 0 {
            let mut next = BindingRow::default();
            for variable in projection.iter().take(projection_count as usize) {
                if let Some(value) = projected.get(*variable) {
                    next.set(*variable, value);
                }
            }
            projected = next;
        }
        if returned == out.len() {
            return Ok(SelectApplyPage {
                returned,
                limit_reached: false,
            });
        }
        out[returned] = projected;
        returned += 1;
        state.emitted += 1;
    }
    Ok(SelectApplyPage {
        returned,
        limit_reached: state.emitted >= limit,
    })
}
