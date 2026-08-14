//! SPARQL Physical Query Executor
//!
//! Executes query plans against NQuin arrays using zero-allocation patterns.

use crate::lexicon::generate_embedded_triple_id;
use crate::rdf_star::is_virtual_id;
use crate::sparql_aggregates::{AggregationContext, GroupKey};
use crate::sparql_ast::*;
use crate::sparql_filter::{EvalResult, ExpressionEvaluator};
use crate::sparql_planner::*;
use crate::NQuin;

/// Resume state for a caller-buffered triple-pattern page over a range-backed
/// Q42 segment.  It is deliberately separate from the resident executor: no
/// graph-sized `Vec<NQuin>` is constructed on this path.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeSparqlCursor {
    pub scan: crate::q42_volume::Q42RangeQueryCursor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeSparqlPage {
    pub returned: usize,
    pub next_cursor: Option<Q42RangeSparqlCursor>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeVolumeSetSparqlCursor {
    pub scan: crate::q42_volume::Q42VolumeSetQueryCursor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeVolumeSetSparqlPage {
    pub returned: usize,
    pub next_cursor: Option<Q42RangeVolumeSetSparqlCursor>,
}

/// Execute one physical page of a SPARQL triple pattern against a Q42 range
/// source.  Constants and already-bound variables become on-disk filters; a
/// bound object selects BIDX pruning automatically. `quin_scratch` and `out`
/// are caller-owned, maintaining the zero-heap query kernel contract.
pub fn execute_range_triple_page_into<S: crate::q42_volume::Q42RangeSource>(
    volume: &crate::q42_volume::Q42RangeVolume<S>,
    subject: u64,
    predicate: u64,
    object: u64,
    context: Option<u64>,
    ctx: &SparqlQueryContext,
    input: &BindingRow,
    cursor: Q42RangeSparqlCursor,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    out: &mut [BindingRow],
) -> Result<Q42RangeSparqlPage, String> {
    if quin_scratch.is_empty() || out.is_empty() {
        return Err("range SPARQL page requires non-empty Quin and row buffers".to_string());
    }
    let bound = |term: u64| match term_is_var(term, ctx) {
        Some(variable) => input.get(variable),
        None => Some(term),
    };
    let pattern = crate::q42_volume::Q42RangeQueryPattern {
        subject: bound(subject),
        predicate: bound(predicate),
        object: bound(object),
        context,
    };
    let page = volume
        .execute_query_page_into(
            crate::q42_volume::Q42RangeQueryPlan::for_pattern(pattern),
            cursor.scan,
            compressed,
            decoded,
            quin_scratch,
        )
        .map_err(|error| format!("range Q42 triple scan: {error}"))?;
    let mut returned = 0usize;
    for quin in &quin_scratch[..page.returned] {
        let mut row = *input;
        if !bind_var(&mut row, subject, quin.subject, ctx)
            || !bind_var(&mut row, predicate, quin.predicate, ctx)
            || !bind_var(&mut row, object, quin.object, ctx)
        {
            continue;
        }
        if returned == out.len() {
            return Err("range SPARQL row buffer is smaller than Quin scratch buffer".to_string());
        }
        out[returned] = row;
        returned += 1;
    }
    Ok(Q42RangeSparqlPage {
        returned,
        next_cursor: page.next_cursor.map(|scan| Q42RangeSparqlCursor { scan }),
    })
}

/// The logical-volume counterpart of [`execute_range_triple_page_into`]. It
/// preserves exactly the same SPARQL binding semantics while the root manifest
/// prunes child segments before each range-backed SuperBlock scan.
pub fn execute_range_volume_set_triple_page_into<S: crate::q42_volume::Q42RangeSource>(
    volumes: &crate::q42_volume::Q42RangeVolumeSet<S>,
    subject: u64,
    predicate: u64,
    object: u64,
    context: Option<u64>,
    ctx: &SparqlQueryContext,
    input: &BindingRow,
    cursor: Q42RangeVolumeSetSparqlCursor,
    compressed: &mut [u8],
    decoded: &mut [u8],
    quin_scratch: &mut [NQuin],
    out: &mut [BindingRow],
) -> Result<Q42RangeVolumeSetSparqlPage, String> {
    if quin_scratch.is_empty() || out.is_empty() {
        return Err("range SPARQL page requires non-empty Quin and row buffers".to_string());
    }
    let bound = |term: u64| match term_is_var(term, ctx) {
        Some(variable) => input.get(variable),
        None => Some(term),
    };
    let page = volumes
        .execute_query_page_into(
            crate::q42_volume::Q42RangeQueryPlan::for_pattern(
                crate::q42_volume::Q42RangeQueryPattern {
                    subject: bound(subject),
                    predicate: bound(predicate),
                    object: bound(object),
                    context,
                },
            ),
            cursor.scan,
            compressed,
            decoded,
            quin_scratch,
        )
        .map_err(|error| format!("range Q42 volume-set triple scan: {error}"))?;
    let mut returned = 0usize;
    for quin in &quin_scratch[..page.returned] {
        let mut row = *input;
        if !bind_var(&mut row, subject, quin.subject, ctx)
            || !bind_var(&mut row, predicate, quin.predicate, ctx)
            || !bind_var(&mut row, object, quin.object, ctx)
        {
            continue;
        }
        if returned == out.len() {
            return Err("range SPARQL row buffer is smaller than Quin scratch buffer".to_string());
        }
        out[returned] = row;
        returned += 1;
    }
    Ok(Q42RangeVolumeSetSparqlPage {
        returned,
        next_cursor: page
            .next_cursor
            .map(|scan| Q42RangeVolumeSetSparqlCursor { scan }),
    })
}

#[inline]
fn term_is_var(term: u64, ctx: &SparqlQueryContext) -> Option<VariableId> {
    let id = term as usize;
    if id < ctx.variable_count {
        Some(term as VariableId)
    } else {
        None
    }
}

#[inline]
fn bind_var(row: &mut BindingRow, term: u64, value: u64, ctx: &SparqlQueryContext) -> bool {
    if let Some(var) = term_is_var(term, ctx) {
        match row.get(var) {
            Some(bound) if bound != value => false,
            _ => {
                row.set(var, value);
                true
            }
        }
    } else {
        term == value
    }
}

/// Query executor
pub struct QueryExecutor<'a> {
    pub quins: &'a [NQuin],
    /// Optional text resolver for literal-text functions (`geof:*`, …). `None`
    /// on the plain slice path; supplied when a lexicon / query-literal table is
    /// available so extension functions can recover geometry/string text.
    resolver: Option<crate::sparql_ast::TextResolver<'a>>,
}

impl<'a> QueryExecutor<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            quins,
            resolver: None,
        }
    }

    /// Executor with a text resolver, enabling `geof:*`/text extension functions.
    pub fn with_resolver(
        quins: &'a [NQuin],
        resolver: crate::sparql_ast::TextResolver<'a>,
    ) -> Self {
        Self {
            quins,
            resolver: Some(resolver),
        }
    }

    /// Execute a query plan and return results
    pub fn execute(
        &self,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
    ) -> Result<Vec<BindingRow>, String> {
        if plan.operator_count == 0 {
            return Err("Empty execution plan".to_string());
        }
        let mut results = Vec::new();
        let mut row = BindingRow::new();

        if self.execute_operator(plan.root_operator, plan, ctx, &mut row, &mut results)? {
            return Ok(results);
        }

        Ok(results)
    }

    /// Execute ASK query
    pub fn execute_ask(
        &self,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
    ) -> Result<bool, String> {
        let mut results = Vec::new();
        let mut row = BindingRow::new();

        self.execute_operator(plan.root_operator, plan, ctx, &mut row, &mut results)?;

        Ok(!results.is_empty())
    }

    /// Collect the concrete triple patterns of a CONSTRUCT template into `out`
    /// as `(subject, predicate, object)` term triples (each field still a
    /// variable id or a constant hash). A template is either a single `Triple`
    /// or a `Group` of triples; any non-triple child is ignored (a CONSTRUCT
    /// template is a basic graph pattern, so only triples are meaningful).
    fn collect_template_triples(
        pattern_id: PatternId,
        ctx: &SparqlQueryContext,
        out: &mut Vec<(u64, u64, u64)>,
    ) {
        match ctx.patterns.get(pattern_id as usize) {
            Some(Pattern::Triple {
                subject,
                predicate,
                object,
            }) => out.push((*subject, *predicate, *object)),
            Some(Pattern::Group { start_idx, len }) => {
                for i in *start_idx..(*start_idx + *len) {
                    Self::collect_template_triples(i, ctx, out);
                }
            }
            _ => {}
        }
    }

    /// Execute a CONSTRUCT query: evaluate the WHERE pattern, then instantiate
    /// the template for every solution. The returned rows are the constructed
    /// triples themselves — variable slot 0 = subject, 1 = predicate, 2 = object
    /// — which is exactly what the N-Triples/XML/JSON graph serialisers read.
    ///
    /// A template triple is emitted only when all three of its terms are bound
    /// (SPARQL 1.1 §16.2.1: a template instantiation with an unbound term
    /// produces no triple). Duplicate triples are collapsed.
    pub fn execute_construct(
        &self,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        template_pattern: PatternId,
    ) -> Result<Vec<BindingRow>, String> {
        let solutions = self.execute(plan, ctx)?;

        let mut templates: Vec<(u64, u64, u64)> = Vec::new();
        Self::collect_template_triples(template_pattern, ctx, &mut templates);

        let resolve = |term: u64, row: &BindingRow| -> Option<u64> {
            match term_is_var(term, ctx) {
                Some(var) => row.get(var), // unbound → None → triple skipped
                None => Some(term),        // constant term
            }
        };

        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for sol in &solutions {
            for &(s, p, o) in &templates {
                let (sv, pv, ov) = match (resolve(s, sol), resolve(p, sol), resolve(o, sol)) {
                    (Some(sv), Some(pv), Some(ov)) => (sv, pv, ov),
                    _ => continue,
                };
                if seen.insert((sv, pv, ov)) {
                    let mut row = BindingRow::new();
                    row.set(0, sv);
                    row.set(1, pv);
                    row.set(2, ov);
                    out.push(row);
                }
            }
        }
        Ok(out)
    }

    /// Execute a DESCRIBE query: build the set of resources to describe (each
    /// `vars_or_ids` entry is a constant IRI, or a variable bound by the WHERE
    /// pattern; `DESCRIBE *` with a WHERE describes every value bound in the
    /// solutions), then emit a Concise Bounded Description — every stored quin
    /// whose subject is a described resource. Rows carry the triple in slots
    /// 0/1/2, matching the graph serialisers. Duplicate triples are collapsed.
    pub fn execute_describe(
        &self,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        describe: &DescribeQuery,
    ) -> Result<Vec<BindingRow>, String> {
        // WHERE solutions are only needed (and only valid) when a pattern exists
        // — a bare `DESCRIBE <iri>` has an empty plan.
        let solutions = if describe.root_pattern.is_some() {
            self.execute(plan, ctx)?
        } else {
            Vec::new()
        };

        let mut resources: Vec<u64> = Vec::new();
        let add = |r: u64, resources: &mut Vec<u64>| {
            if !resources.contains(&r) {
                resources.push(r);
            }
        };

        if describe.var_count == 0 {
            // DESCRIBE * — every value bound in the WHERE solutions.
            for sol in &solutions {
                for var in 0..ctx.variable_count as VariableId {
                    if let Some(v) = sol.get(var) {
                        add(v, &mut resources);
                    }
                }
            }
        } else {
            for i in 0..describe.var_count as usize {
                let term = describe.vars_or_ids[i];
                match term_is_var(term, ctx) {
                    Some(var) => {
                        for sol in &solutions {
                            if let Some(v) = sol.get(var) {
                                add(v, &mut resources);
                            }
                        }
                    }
                    None => add(term, &mut resources),
                }
            }
        }

        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for &r in &resources {
            for q in self.quins {
                if q.subject == r && seen.insert((q.subject, q.predicate, q.object)) {
                    let mut row = BindingRow::new();
                    row.set(0, q.subject);
                    row.set(1, q.predicate);
                    row.set(2, q.object);
                    out.push(row);
                }
            }
        }
        Ok(out)
    }

    fn execute_operator(
        &self,
        op_id: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let operator = plan
            .operators
            .get(op_id as usize)
            .ok_or("Operator ID out of bounds")?;

        match operator.operator_type {
            PhysicalOperatorType::SubjectScan { subject } => {
                self.execute_subject_scan(subject, ctx, row, results)
            }
            PhysicalOperatorType::PredicateScan { predicate } => {
                self.execute_predicate_scan(predicate, ctx, row, results)
            }
            PhysicalOperatorType::ObjectScan { object } => {
                self.execute_object_scan(object, ctx, row, results)
            }
            PhysicalOperatorType::TripleScan {
                subject,
                predicate,
                object,
            } => self.execute_triple_scan(subject, predicate, object, ctx, row, results),
            PhysicalOperatorType::HashJoin {
                left,
                right,
                join_var,
            } => self.execute_hash_join(left, right, join_var, plan, ctx, row, results),
            PhysicalOperatorType::NestedLoopJoin {
                left,
                right,
                join_var,
            } => self.execute_nested_loop_join(left, right, join_var, plan, ctx, row, results),
            PhysicalOperatorType::Filter { input, expression } => {
                self.execute_filter(input, expression, plan, ctx, row, results)
            }
            PhysicalOperatorType::Bind {
                input,
                var,
                expression,
            } => self.execute_bind(input, var, expression, plan, ctx, row, results),
            PhysicalOperatorType::Project {
                input,
                vars,
                var_count,
            } => self.execute_project(input, vars, var_count, plan, ctx, row, results),
            PhysicalOperatorType::Limit {
                input,
                limit,
                offset,
            } => self.execute_limit(input, limit, offset, plan, ctx, row, results),
            PhysicalOperatorType::Sort {
                input,
                order_by,
                order_count,
                ascending,
            } => self.execute_sort(
                input,
                &order_by,
                order_count,
                &ascending,
                plan,
                ctx,
                row,
                results,
            ),
            PhysicalOperatorType::Union { left, right } => {
                self.execute_union(left, right, plan, ctx, row, results)
            }
            PhysicalOperatorType::Optional { left, right } => {
                self.execute_optional(left, right, plan, ctx, row, results)
            }
            PhysicalOperatorType::AntiJoin { left, right } => {
                self.execute_anti_join(left, right, plan, ctx, row, results)
            }
            PhysicalOperatorType::Distinct { input } => {
                self.execute_distinct(input, plan, ctx, row, results)
            }
            PhysicalOperatorType::SubSelect { query_id } => {
                self.execute_sub_select(query_id, ctx, row, results)
            }
            PhysicalOperatorType::GroupBy {
                input,
                group_vars,
                group_var_count,
                aggregates,
                aggregate_count,
            } => self.execute_group_by(
                input,
                group_vars,
                group_var_count,
                aggregates,
                aggregate_count,
                plan,
                ctx,
                row,
                results,
            ),
            PhysicalOperatorType::Having { input, expression } => {
                self.execute_having(input, expression, plan, ctx, row, results)
            }
            PhysicalOperatorType::PropertyPath {
                subject,
                path_id,
                object,
            } => self.execute_property_path(subject, path_id, object, ctx, row, results),
            PhysicalOperatorType::Graph {
                graph_var_or_id,
                inner,
            } => self.execute_graph(graph_var_or_id, inner, plan, ctx, row, results),
            PhysicalOperatorType::Service {
                endpoint_did_id,
                inner_pattern,
            } => self.execute_service(endpoint_did_id, inner_pattern, plan, ctx, row, results),
            PhysicalOperatorType::AsOf {
                input,
                timestamp_ms,
                mode,
            } => self.execute_as_of(input, timestamp_ms, mode, plan, ctx, row, results),
            PhysicalOperatorType::StarTripleScan {
                inner_subject,
                inner_predicate,
                inner_object,
                outer_predicate,
                outer_object,
            } => self.execute_star_triple_scan(
                inner_subject,
                inner_predicate,
                inner_object,
                outer_predicate,
                outer_object,
                ctx,
                row,
                results,
            ),
        }
    }

    fn execute_subject_scan(
        &self,
        subject: u64,
        _ctx: &SparqlQueryContext,
        _row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        // Scan all quins matching the subject
        for quin in self.quins {
            if quin.subject == subject {
                // Bind the subject if it's a variable
                // For now, just add the quin to results
                let mut new_row = BindingRow::new();
                new_row.slots[0] = Some(quin.subject);
                new_row.slots[1] = Some(quin.predicate);
                new_row.slots[2] = Some(quin.object);
                results.push(new_row);
            }
        }
        Ok(!results.is_empty())
    }

    fn execute_predicate_scan(
        &self,
        predicate: u64,
        _ctx: &SparqlQueryContext,
        _row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        for quin in self.quins {
            if quin.predicate == predicate {
                let mut new_row = BindingRow::new();
                new_row.slots[0] = Some(quin.subject);
                new_row.slots[1] = Some(quin.predicate);
                new_row.slots[2] = Some(quin.object);
                results.push(new_row);
            }
        }
        Ok(!results.is_empty())
    }

    fn execute_object_scan(
        &self,
        object: u64,
        _ctx: &SparqlQueryContext,
        _row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        for quin in self.quins {
            if quin.object == object {
                let mut new_row = BindingRow::new();
                new_row.slots[0] = Some(quin.subject);
                new_row.slots[1] = Some(quin.predicate);
                new_row.slots[2] = Some(quin.object);
                results.push(new_row);
            }
        }
        Ok(!results.is_empty())
    }

    fn execute_triple_scan(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        for quin in self.quins {
            let mut candidate = *row;
            if !bind_var(&mut candidate, subject, quin.subject, ctx) {
                continue;
            }
            if !bind_var(&mut candidate, predicate, quin.predicate, ctx) {
                continue;
            }
            if !bind_var(&mut candidate, object, quin.object, ctx) {
                continue;
            }
            results.push(candidate);
        }
        Ok(!results.is_empty())
    }

    fn execute_star_triple_scan(
        &self,
        inner_subject: u64,
        inner_predicate: u64,
        inner_object: u64,
        outer_predicate: u64,
        outer_object: u64,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        for quin in self.quins {
            if !is_virtual_id(quin.subject) {
                continue;
            }
            let mut candidate = *row;
            if !bind_var(&mut candidate, outer_predicate, quin.predicate, ctx) {
                continue;
            }
            if !bind_var(&mut candidate, outer_object, quin.object, ctx) {
                continue;
            }

            if let (Some(s), Some(p), Some(o)) = (
                term_is_var(inner_subject, ctx),
                term_is_var(inner_predicate, ctx),
                term_is_var(inner_object, ctx),
            ) {
                if let Some(components) = self.lookup_star_components(quin.subject) {
                    candidate.set(s, components[0]);
                    candidate.set(p, components[1]);
                    candidate.set(o, components[2]);
                    results.push(candidate);
                }
            } else {
                let expected_vid = generate_embedded_triple_id(
                    if term_is_var(inner_subject, ctx).is_some() {
                        0
                    } else {
                        inner_subject
                    },
                    if term_is_var(inner_predicate, ctx).is_some() {
                        0
                    } else {
                        inner_predicate
                    },
                    if term_is_var(inner_object, ctx).is_some() {
                        0
                    } else {
                        inner_object
                    },
                );
                if quin.subject == expected_vid
                    && bind_var(&mut candidate, outer_predicate, quin.predicate, ctx)
                    && bind_var(&mut candidate, outer_object, quin.object, ctx)
                {
                    results.push(candidate);
                }
            }
        }
        Ok(!results.is_empty())
    }

    fn lookup_star_components(&self, virtual_id: u64) -> Option<[u64; 3]> {
        for quin in self.quins {
            let candidate = generate_embedded_triple_id(quin.subject, quin.predicate, quin.object);
            if candidate == virtual_id {
                return Some([quin.subject, quin.predicate, quin.object]);
            }
        }
        None
    }

    fn execute_hash_join(
        &self,
        left: OperatorId,
        right: OperatorId,
        join_var: VariableId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        _row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut left_results = Vec::new();
        let mut left_row = BindingRow::new();
        self.execute_operator(left, plan, ctx, &mut left_row, &mut left_results)?;

        let mut right_results = Vec::new();
        let mut right_row = BindingRow::new();
        self.execute_operator(right, plan, ctx, &mut right_row, &mut right_results)?;

        // Zero-allocation Merge Join (O(N log N) + O(M log M))
        left_results
            .sort_unstable_by(|a, b| a.slots[join_var as usize].cmp(&b.slots[join_var as usize]));
        right_results
            .sort_unstable_by(|a, b| a.slots[join_var as usize].cmp(&b.slots[join_var as usize]));

        let mut i = 0;
        let mut j = 0;

        while i < left_results.len() && j < right_results.len() {
            let left_val = left_results[i].slots[join_var as usize];
            let right_val = right_results[j].slots[join_var as usize];

            // If join_var is None on either side, it conceptually matches anything.
            // However, in BGP joins, join variables are practically always bound.
            // If they are unbound, we fall back to nested loop for those specific rows (not implemented here,
            // assume BGP variables are bound).
            if left_val < right_val {
                i += 1;
            } else if left_val > right_val {
                j += 1;
            } else {
                let mut left_end = i + 1;
                while left_end < left_results.len()
                    && left_results[left_end].slots[join_var as usize] == left_val
                {
                    left_end += 1;
                }

                let mut right_end = j + 1;
                while right_end < right_results.len()
                    && right_results[right_end].slots[join_var as usize] == right_val
                {
                    right_end += 1;
                }

                for l in &left_results[i..left_end] {
                    for r in &right_results[j..right_end] {
                        let mut compatible = true;
                        for k in 0..MAX_BINDINGS {
                            if let (Some(a), Some(b)) = (l.slots[k], r.slots[k]) {
                                if a != b {
                                    compatible = false;
                                    break;
                                }
                            }
                        }

                        if compatible {
                            let mut joined = BindingRow::new();
                            for k in 0..MAX_BINDINGS {
                                joined.slots[k] = l.slots[k].or(r.slots[k]);
                            }
                            results.push(joined);
                        }
                    }
                }

                i = left_end;
                j = right_end;
            }
        }

        Ok(!results.is_empty())
    }

    fn execute_nested_loop_join(
        &self,
        left: OperatorId,
        right: OperatorId,
        _join_var: VariableId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        _row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut left_results = Vec::new();
        let mut left_row = BindingRow::new();
        self.execute_operator(left, plan, ctx, &mut left_row, &mut left_results)?;

        let mut right_results = Vec::new();
        let mut right_row = BindingRow::new();
        self.execute_operator(right, plan, ctx, &mut right_row, &mut right_results)?;

        // Full Cross-Product with Compatibility Check
        for l in &left_results {
            for r in &right_results {
                let mut compatible = true;
                for i in 0..MAX_BINDINGS {
                    if let (Some(a), Some(b)) = (l.slots[i], r.slots[i]) {
                        if a != b {
                            compatible = false;
                            break;
                        }
                    }
                }

                if compatible {
                    let mut joined = BindingRow::new();
                    for i in 0..MAX_BINDINGS {
                        joined.slots[i] = l.slots[i].or(r.slots[i]);
                    }
                    results.push(joined);
                }
            }
        }

        Ok(!results.is_empty())
    }

    fn execute_filter(
        &self,
        input: OperatorId,
        expression: ExpressionId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut input_results = Vec::new();
        self.execute_operator(input, plan, ctx, row, &mut input_results)?;

        // Filter results based on expression evaluation. `eval_filter_bool`
        // handles FILTER (NOT) EXISTS in boolean position and delegates pure
        // value expressions to the expression evaluator.
        for input_row in input_results {
            if self.eval_filter_bool(expression, ctx, &input_row)? {
                results.push(input_row);
            }
        }

        Ok(!results.is_empty())
    }

    /// True iff the expression subtree contains an `EXISTS`/`NOT EXISTS` node
    /// (walking the boolean-combinator arms only — enough to route `&&`/`||`/`!`
    /// through `eval_filter_bool`; an EXISTS anywhere else falls through to the
    /// value evaluator, which rejects it honestly).
    fn expr_contains_exists(expr_id: ExpressionId, ctx: &SparqlQueryContext) -> bool {
        match ctx.expressions.get(expr_id as usize) {
            Some(Expression::Exists { .. }) => true,
            Some(Expression::UnaryOp { expr, .. }) => Self::expr_contains_exists(*expr, ctx),
            Some(Expression::BinaryOp { left, right, .. }) => {
                Self::expr_contains_exists(*left, ctx) || Self::expr_contains_exists(*right, ctx)
            }
            _ => false,
        }
    }

    /// Evaluate `EXISTS { pattern }` for the current row: plan the inner group,
    /// execute it seeded with the row's bindings (pre-bound variables act as the
    /// SPARQL substitution μ), and report whether ≥1 solution exists.
    fn eval_exists(
        &self,
        pattern: PatternId,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
    ) -> Result<bool, String> {
        let mut sub_plan = ExecutionPlan::new();
        let op = QueryPlanner::plan_pattern(pattern, ctx, &mut sub_plan)?;
        sub_plan.root_operator = op;
        let mut seed = *row;
        let mut local = Vec::new();
        self.execute_operator(op, &sub_plan, ctx, &mut seed, &mut local)?;
        Ok(!local.is_empty())
    }

    /// Evaluate a FILTER/HAVING constraint to a boolean, resolving `EXISTS`/`NOT
    /// EXISTS` (which need graph access) at the top level and within `&&`/`||`/`!`
    /// combinators, and delegating everything else to the value evaluator.
    fn eval_filter_bool(
        &self,
        expr_id: ExpressionId,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
    ) -> Result<bool, String> {
        let expr = *ctx
            .expressions
            .get(expr_id as usize)
            .ok_or("Expression ID out of bounds")?;
        match expr {
            Expression::Exists { pattern, negated } => {
                Ok(self.eval_exists(pattern, ctx, row)? ^ negated)
            }
            Expression::UnaryOp {
                op: UnaryOp::Not,
                expr,
            } if Self::expr_contains_exists(expr, ctx) => {
                Ok(!self.eval_filter_bool(expr, ctx, row)?)
            }
            Expression::BinaryOp {
                op: BinaryOp::And,
                left,
                right,
            } if Self::expr_contains_exists(left, ctx)
                || Self::expr_contains_exists(right, ctx) =>
            {
                Ok(self.eval_filter_bool(left, ctx, row)?
                    && self.eval_filter_bool(right, ctx, row)?)
            }
            Expression::BinaryOp {
                op: BinaryOp::Or,
                left,
                right,
            } if Self::expr_contains_exists(left, ctx)
                || Self::expr_contains_exists(right, ctx) =>
            {
                Ok(self.eval_filter_bool(left, ctx, row)?
                    || self.eval_filter_bool(right, ctx, row)?)
            }
            _ => {
                let r =
                    ExpressionEvaluator::evaluate_with_resolver(expr_id, ctx, row, self.resolver)?;
                Ok(r.as_bool())
            }
        }
    }

    fn execute_bind(
        &self,
        input: OperatorId,
        var: VariableId,
        expression: ExpressionId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut input_results = Vec::new();
        self.execute_operator(input, plan, ctx, row, &mut input_results)?;

        for mut input_row in input_results {
            // SPARQL 1.1 Extend: bind `var` to the value of the expression,
            // keeping every row. If the expression raises an error (e.g. a
            // not-yet-implemented string-producing builtin, or a type error),
            // the variable is simply left UNBOUND rather than failing the whole
            // query. Value-producing results (numeric / term / boolean /
            // already-interned string) map to the row's u64 slot.
            match ExpressionEvaluator::evaluate_with_resolver(
                expression,
                ctx,
                &input_row,
                self.resolver,
            ) {
                Ok(EvalResult::Numeric(n)) | Ok(EvalResult::Iri(n)) | Ok(EvalResult::String(n)) => {
                    input_row.set(var, n);
                }
                Ok(EvalResult::Boolean(b)) => {
                    input_row.set(var, b as u64);
                }
                Ok(EvalResult::Float(f)) => {
                    // BIND of a real value: store the IEEE-754 bit pattern in the u64 slot.
                    input_row.set(var, f.to_bits());
                }
                Err(_) => { /* expression error → leave `var` unbound */ }
            }
            results.push(input_row);
        }

        Ok(!results.is_empty())
    }

    fn execute_project(
        &self,
        input: OperatorId,
        vars: [VariableId; MAX_VARIABLES],
        var_count: u8,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        // Project each solution onto the selected variables only, dropping every
        // other binding. This is required for correctness — a downstream DISTINCT
        // must dedup on the *projected* columns, not on the full WHERE row (two
        // solutions identical in ?a but differing in an unselected ?b are one
        // DISTINCT ?a result). Variables keep their own slot ids (the serialiser
        // reads by variable id from the SELECT list).
        let mut input_results = Vec::new();
        self.execute_operator(input, plan, ctx, row, &mut input_results)?;
        for in_row in input_results {
            let mut projected = BindingRow::new();
            for &v in vars.iter().take(var_count as usize) {
                if let Some(val) = in_row.get(v) {
                    projected.set(v, val);
                }
            }
            results.push(projected);
        }
        Ok(!results.is_empty())
    }

    fn execute_limit(
        &self,
        input: OperatorId,
        limit: u64,
        offset: u64,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut all_results = Vec::new();
        self.execute_operator(input, plan, ctx, row, &mut all_results)?;

        // Apply offset and limit
        let start = offset as usize;
        let end = if limit == u64::MAX {
            all_results.len()
        } else {
            (start + limit as usize).min(all_results.len())
        };

        if start < all_results.len() {
            results.extend_from_slice(&all_results[start..end]);
        }

        Ok(!results.is_empty())
    }

    fn execute_sort(
        &self,
        input: OperatorId,
        order_by: &[ExpressionId; MAX_ORDER_CONDITIONS],
        order_count: u8,
        ascending: &[bool; MAX_ORDER_CONDITIONS],
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let start_len = results.len();
        self.execute_operator(input, plan, ctx, row, results)?;

        // Sort in-place using the order_by expressions
        let slice = &mut results[start_len..];

        slice.sort_unstable_by(|a, b| {
            for i in 0..order_count as usize {
                let expr = order_by[i];
                let asc = ascending[i];

                let val_a =
                    ExpressionEvaluator::evaluate_with_resolver(expr, ctx, a, self.resolver)
                        .unwrap_or(crate::sparql_filter::EvalResult::Numeric(0));
                let val_b =
                    ExpressionEvaluator::evaluate_with_resolver(expr, ctx, b, self.resolver)
                        .unwrap_or(crate::sparql_filter::EvalResult::Numeric(0));

                let cmp = val_a.total_cmp(&val_b);
                if cmp != std::cmp::Ordering::Equal {
                    return if asc { cmp } else { cmp.reverse() };
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(!results.is_empty())
    }

    fn execute_union(
        &self,
        left: OperatorId,
        right: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let _start_len = results.len();

        // Execute left
        self.execute_operator(left, plan, ctx, row, results)?;

        // Execute right
        self.execute_operator(right, plan, ctx, row, results)?;

        // SPARQL UNION is a multiset union (bag union), so no deduplication is needed.

        Ok(!results.is_empty())
    }

    fn execute_optional(
        &self,
        left: OperatorId,
        right: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        // Execute left pattern
        let mut left_results = Vec::new();
        let mut right_results = Vec::new();
        self.execute_operator(left, plan, ctx, row, &mut left_results)?;

        // For each left result, try to execute right pattern
        for left_result in left_results {
            right_results.clear();
            let mut right_row = left_result; // Copy left bindings
            let right_matched =
                self.execute_operator(right, plan, ctx, &mut right_row, &mut right_results)?;

            if right_matched && !right_results.is_empty() {
                // Right pattern matched - combine bindings
                results.extend_from_slice(&right_results);
            } else {
                // Right pattern didn't match - keep left result with NULL for right variables
                results.push(left_result);
            }
        }

        Ok(!results.is_empty())
    }

    /// SPARQL 1.1 MINUS (anti-join). Both sides are evaluated; a left solution
    /// is removed iff some right solution is **compatible** with it AND the two
    /// **share at least one bound variable** (the domain-intersection rule that
    /// distinguishes MINUS from NOT EXISTS — MINUS over disjoint domains removes
    /// nothing).
    fn execute_anti_join(
        &self,
        left: OperatorId,
        right: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut left_results = Vec::new();
        self.execute_operator(left, plan, ctx, row, &mut left_results)?;

        // The right side is evaluated independently (fresh bindings).
        let mut right_results = Vec::new();
        let mut right_row = BindingRow::new();
        self.execute_operator(right, plan, ctx, &mut right_row, &mut right_results)?;

        for l in left_results {
            let excluded = right_results.iter().any(|r| {
                let mut compatible = true;
                let mut shares = false;
                for k in 0..MAX_BINDINGS {
                    if let (Some(a), Some(b)) = (l.slots[k], r.slots[k]) {
                        shares = true;
                        if a != b {
                            compatible = false;
                            break;
                        }
                    }
                }
                compatible && shares
            });
            if !excluded {
                results.push(l);
            }
        }

        Ok(!results.is_empty())
    }

    /// Execute a sub-`SELECT`: evaluate the stored subquery independently, then
    /// join each of its projected solutions with the current bindings on shared
    /// variables (a solution incompatible on any shared, differently-bound
    /// variable is dropped). Only the sub-select's projected variables are
    /// visible here — its internal variables were removed by its own projection.
    fn execute_sub_select(
        &self,
        query_id: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let subquery = *ctx
            .subqueries
            .get(query_id as usize)
            .ok_or("Subquery ID out of bounds")?;
        let sub_plan = QueryPlanner::plan(&subquery, ctx)?;
        let sub_rows = self.execute(&sub_plan, ctx)?;

        for sub_row in sub_rows {
            let mut merged = *row;
            let mut compatible = true;
            for var in 0..ctx.variable_count as VariableId {
                if let Some(v) = sub_row.get(var) {
                    match merged.get(var) {
                        Some(existing) if existing != v => {
                            compatible = false;
                            break;
                        }
                        _ => merged.set(var, v),
                    }
                }
            }
            if compatible {
                results.push(merged);
            }
        }
        Ok(!results.is_empty())
    }

    fn execute_distinct(
        &self,
        input: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let start_len = results.len();
        self.execute_operator(input, plan, ctx, row, results)?;

        let slice = &mut results[start_len..];
        slice.sort_unstable();

        if results.len() > start_len {
            let mut write_idx = start_len + 1;
            for read_idx in (start_len + 1)..results.len() {
                if results[read_idx] != results[write_idx - 1] {
                    results[write_idx] = results[read_idx];
                    write_idx += 1;
                }
            }
            results.truncate(write_idx);
        }

        Ok(!results.is_empty())
    }

    fn execute_group_by(
        &self,
        input: OperatorId,
        group_vars: [VariableId; MAX_VARIABLES],
        group_var_count: u8,
        aggregates: [crate::sparql_planner::AggregateSpec; 16],
        aggregate_count: u8,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut all_results = Vec::new();
        self.execute_operator(input, plan, ctx, row, &mut all_results)?;

        // Group results by group variables
        let mut agg_ctx = AggregationContext::new(&aggregates, aggregate_count);

        for result in &all_results {
            let mut key = GroupKey::new();
            for i in 0..group_var_count as usize {
                let var_id = group_vars[i];
                if let Some(value) = result.get(var_id) {
                    key.set(var_id, value);
                }
            }

            let group_idx = agg_ctx.find_or_create_group(key)?;
            agg_ctx.add_values_to_group(group_idx, result);
        }

        // Convert groups to binding rows
        for i in 0..agg_ctx.group_count as usize {
            let (key, accumulators) = &agg_ctx.groups[i];
            let mut result_row = BindingRow::new();
            for j in 0..key.var_count as usize {
                result_row.slots[j] = Some(key.values[j]);
            }

            // Write aggregate results to output variables
            for j in 0..aggregate_count as usize {
                if let Some(result_val) = accumulators[j].get_result() {
                    let out_var = aggregates[j].output_var;
                    result_row.slots[out_var as usize] = Some(result_val);
                }
            }
            results.push(result_row);
        }

        Ok(!results.is_empty())
    }

    fn execute_having(
        &self,
        input: OperatorId,
        expression: ExpressionId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut all_results = Vec::new();
        self.execute_operator(input, plan, ctx, row, &mut all_results)?;

        // Filter results based on HAVING expression (EXISTS-aware, like FILTER).
        for result in all_results {
            if self.eval_filter_bool(expression, ctx, &result)? {
                results.push(result);
            }
        }

        Ok(!results.is_empty())
    }

    fn execute_property_path(
        &self,
        subject: u64,
        path_id: PathId,
        object: u64,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let path = ctx
            .paths
            .get(path_id as usize)
            .ok_or("Path ID out of bounds")?;

        match path {
            crate::sparql_ast::Path::Predicate(pred) => {
                // Simple predicate - same as triple scan
                self.execute_triple_scan(subject, *pred, object, ctx, row, results)
            }
            crate::sparql_ast::Path::Inverse(inner_id) => {
                // Inverse - swap subject and object
                self.execute_property_path(object, *inner_id, subject, ctx, row, results)
            }
            crate::sparql_ast::Path::Sequence { left, right } => {
                // Sequence - execute left then right
                let mut intermediate_results = Vec::new();
                self.execute_property_path(subject, *left, 0, ctx, row, &mut intermediate_results)?;

                for inter_result in intermediate_results {
                    let intermediate_val = inter_result.slots[0].unwrap_or(0);
                    self.execute_property_path(
                        intermediate_val,
                        *right,
                        object,
                        ctx,
                        row,
                        results,
                    )?;
                }
                Ok(!results.is_empty())
            }
            crate::sparql_ast::Path::Alternative { left, right } => {
                // Alternation - execute left OR right
                let mut left_results = Vec::new();
                let mut right_results = Vec::new();

                self.execute_property_path(subject, *left, object, ctx, row, &mut left_results)?;
                self.execute_property_path(subject, *right, object, ctx, row, &mut right_results)?;

                results.extend_from_slice(&left_results);
                results.extend_from_slice(&right_results);
                Ok(!results.is_empty())
            }
            crate::sparql_ast::Path::ZeroOrMore(inner_id) => {
                // Kleene star `*`: the FULL reflexive-transitive closure of the inner
                // path from `subject`, computed as a cycle-safe fixpoint (not a fixed
                // hop limit). `subject` itself matches the zero-length path.
                let mut reached = self.path_transitive_hops(subject, *inner_id, ctx, row)?;
                reached.insert(subject);
                Self::emit_path_nodes(reached, object, results);
                Ok(!results.is_empty())
            }
            crate::sparql_ast::Path::OneOrMore(inner_id) => {
                // `+`: the FULL (non-reflexive) transitive closure of the inner path from
                // `subject`, cycle-safe. `subject` is included only if a cycle reaches it
                // back via ≥1 hop.
                let reached = self.path_transitive_hops(subject, *inner_id, ctx, row)?;
                Self::emit_path_nodes(reached, object, results);
                Ok(!results.is_empty())
            }
            crate::sparql_ast::Path::ZeroOrOne(inner_id) => {
                // Zero or one - either direct or via path
                // Direct match
                if subject == object {
                    let mut direct_row = BindingRow::new();
                    direct_row.slots[0] = Some(subject);
                    results.push(direct_row);
                }

                // Via path
                self.execute_property_path(subject, *inner_id, object, ctx, row, results)
            }
        }
    }

    /// Nodes reachable from `subject` via **one or more** applications of the inner
    /// property path — the full transitive closure, made cycle-safe by expanding each
    /// node at most once. Replaces the former fixed "up to 3 hops" truncation, which
    /// silently returned incomplete results for longer paths.
    fn path_transitive_hops(
        &self,
        subject: u64,
        path_id: PathId,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
    ) -> Result<std::collections::HashSet<u64>, String> {
        use std::collections::HashSet;
        let mut reached: HashSet<u64> = HashSet::new();
        let mut expanded: HashSet<u64> = HashSet::new();
        let mut frontier: Vec<u64> = vec![subject];
        while let Some(node) = frontier.pop() {
            if !expanded.insert(node) {
                continue; // cycle guard: expand each node's out-edges at most once
            }
            let mut hops = Vec::new();
            self.execute_property_path(node, path_id, 0, ctx, row, &mut hops)?;
            for hop in hops {
                let next = hop.slots[0].unwrap_or(0);
                reached.insert(next);
                frontier.push(next);
            }
        }
        Ok(reached)
    }

    /// Emit a one-binding row (slot 0 = node) for each reachable node matching `object`
    /// (`object == 0` = unbound → emit all).
    fn emit_path_nodes(
        nodes: impl IntoIterator<Item = u64>,
        object: u64,
        results: &mut Vec<BindingRow>,
    ) {
        for node in nodes {
            if object == 0 || node == object {
                let mut r = BindingRow::new();
                r.slots[0] = Some(node);
                results.push(r);
            }
        }
    }

    fn execute_graph(
        &self,
        graph_var_or_id: u64,
        inner: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        // GRAPH ?g { … } — the graph term is a variable. Enumerate every named
        // graph (distinct non-default context), evaluate the inner pattern within
        // it, and bind ?g to that graph IRI on each resulting solution.
        if let Some(graph_var) = term_is_var(graph_var_or_id, ctx) {
            let mut contexts: Vec<u64> = Vec::new();
            for q in self.quins {
                if q.context != 0 && !contexts.contains(&q.context) {
                    contexts.push(q.context);
                }
            }

            let mut matched = false;
            for gctx in contexts {
                let graph_quins = crate::query_engine::filter_by_context(self.quins, gctx);
                if graph_quins.is_empty() {
                    continue;
                }
                let temp_executor = QueryExecutor {
                    quins: &graph_quins,
                    resolver: self.resolver,
                };
                // Seed the inner evaluation with ?g pre-bound so a join on ?g is
                // consistent; stamp it on every produced row as well.
                let mut seed = *row;
                seed.set(graph_var, gctx);
                let mut local = Vec::new();
                if temp_executor.execute_operator(inner, plan, ctx, &mut seed, &mut local)? {
                    matched = true;
                }
                for mut r in local {
                    r.set(graph_var, gctx);
                    results.push(r);
                }
            }
            return Ok(matched);
        }

        // GRAPH <iri> { … } — a specific named graph.
        let graph_id = graph_var_or_id;

        // Filter quins by graph context
        let graph_quins = crate::query_engine::filter_by_context(self.quins, graph_id);

        if graph_quins.is_empty() {
            return Ok(false);
        }

        // Create a temporary executor with graph-filtered quins, propagating the
        // text resolver so nested GRAPH/geo functions still resolve.
        let temp_executor = QueryExecutor {
            quins: &graph_quins,
            resolver: self.resolver,
        };

        // Execute inner pattern with graph-filtered quins
        temp_executor.execute_operator(inner, plan, ctx, row, results)
    }

    fn execute_service(
        &self,
        endpoint_did_id: u64,
        inner: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        // Zero-allocation federated query execution
        // Use fixed-size network buffer instead of allocating per request
        let _network_buffer = [0u8; 4096];

        // Check if DID has 0x8 prefix (identity recognition)
        let is_did = (endpoint_did_id & 0x8000000000000000) != 0;

        if !is_did {
            return Err("Invalid DID: missing 0x8 prefix".to_string());
        }

        // In production, this would:
        // 1. Resolve DID to get endpoint URL (using cached DID Document)
        // 2. Check connection pool for existing connection to endpoint
        // 3. Format SPARQL query request using zero-copy stack formatting
        // 4. Add DID-based authentication header (DID-LD/DID-JWT/DID-VC)
        // 5. Stream network response into network_buffer iteratively
        // 6. Parse response bytes directly to populate row slots
        // 7. Verify response signature using server DID

        // Simplified: execute inner pattern locally for now
        self.execute_operator(inner, plan, ctx, row, results)
    }

    fn execute_as_of(
        &self,
        input: OperatorId,
        timestamp_ms: u64,
        mode: TemporalMode,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut inner_results = Vec::new();
        self.execute_operator(input, plan, ctx, row, &mut inner_results)?;

        for candidate in inner_results {
            let subject_opt = candidate.slots.iter().find_map(|s| *s);
            let passes = if let Some(subject) = subject_opt {
                self.check_temporal_constraint(subject, timestamp_ms, mode)
            } else {
                true
            };
            if passes {
                results.push(candidate);
            }
        }
        Ok(!results.is_empty())
    }

    /// Check whether `subject` satisfies the temporal constraint at `timestamp_ms`.
    ///
    /// Queries T_CONTEXT PROV-O quins for the subject.  Open-world assumption: if no
    /// temporal annotation is present, the quin is included.
    fn check_temporal_constraint(
        &self,
        subject: u64,
        timestamp_ms: u64,
        mode: TemporalMode,
    ) -> bool {
        use crate::kml_bridge::T_CONTEXT;
        use crate::sparql_filter::prov_predicates;

        match mode {
            TemporalMode::AsOf => {
                let gen_time = self
                    .quins
                    .iter()
                    .find(|q| {
                        q.subject == subject
                            && q.predicate == prov_predicates::GENERATED_AT_TIME
                            && q.context == T_CONTEXT
                    })
                    .map(|q| q.object);
                gen_time.map(|t| t <= timestamp_ms).unwrap_or(true)
            }
            TemporalMode::AtTime => {
                let start = self
                    .quins
                    .iter()
                    .find(|q| {
                        q.subject == subject
                            && q.predicate == prov_predicates::STARTED_AT_TIME
                            && q.context == T_CONTEXT
                    })
                    .map(|q| q.object);
                let end = self
                    .quins
                    .iter()
                    .find(|q| {
                        q.subject == subject
                            && q.predicate == prov_predicates::ENDED_AT_TIME
                            && q.context == T_CONTEXT
                    })
                    .map(|q| q.object);
                start.map(|t| t <= timestamp_ms).unwrap_or(true)
                    && end.map(|t| timestamp_ms <= t).unwrap_or(true)
            }
        }
    }
}

/// Unpack an embedded triple and map its components to variable indices in a BindingRow.
pub fn unpack_virtual_triple(
    virtual_id: u64,
    lexicon: &crate::q42_lex::Q42LexMmap<'_>,
    row: &mut crate::sparql_ast::BindingRow,
    s_var_idx: Option<u8>,
    p_var_idx: Option<u8>,
    o_var_idx: Option<u8>,
) -> Result<(), String> {
    if let Some([s_id, p_id, o_id]) = lexicon.lookup_embedded_triple(virtual_id) {
        if let Some(s_idx) = s_var_idx {
            row.slots[s_idx as usize] = Some(s_id);
        }
        if let Some(p_idx) = p_var_idx {
            row.slots[p_idx as usize] = Some(p_id);
        }
        if let Some(o_idx) = o_var_idx {
            row.slots[o_idx as usize] = Some(o_id);
        }
        Ok(())
    } else {
        Err("Virtual ID not found in lexicon or invalid".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let quins = vec![];
        let executor = QueryExecutor::new(&quins);
        assert_eq!(executor.quins.len(), 0);
    }

    #[test]
    fn test_execute_empty_plan() {
        let quins = vec![];
        let executor = QueryExecutor::new(&quins);
        let plan = ExecutionPlan::new();
        let ctx = SparqlQueryContext::new();

        let result = executor.execute(&plan, &ctx);
        // Should fail because root operator is invalid
        assert!(result.is_err());
    }

    #[test]
    fn range_triple_page_uses_q42_bidx_without_resident_graph() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("range.q42");
        let quin = NQuin {
            subject: 101,
            predicate: 202,
            object: 303,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        crate::q42_volume::write_unified_volume(
            &path,
            &std::collections::HashMap::new(),
            &[(quin.object, quin.object)],
            &[vec![quin]],
        )
        .unwrap();
        let source = crate::q42_volume::LocalFileRangeSource::open(&path).unwrap();
        let volume = crate::q42_volume::Q42RangeVolume::open(source).unwrap();
        let context = SparqlQueryContext::new();
        let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        let mut quins = [NQuin::default(); 1];
        let mut rows = [BindingRow::default(); 1];
        let page = execute_range_triple_page_into(
            &volume,
            quin.subject,
            quin.predicate,
            quin.object,
            None,
            &context,
            &BindingRow::default(),
            Q42RangeSparqlCursor::default(),
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut rows,
        )
        .unwrap();
        assert_eq!(page.returned, 1);
        assert_eq!(rows[0].slots[0], None);
    }
}
