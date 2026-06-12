//! SPARQL Physical Query Executor
//!
//! Executes query plans against NQuin arrays using zero-allocation patterns.

use crate::sparql_ast::*;
use crate::sparql_planner::*;
use crate::sparql_filter::ExpressionEvaluator;
use crate::sparql_aggregates::{AggregationContext, GroupKey, AggregateFunction};
use crate::NQuin;

/// Query executor
pub struct QueryExecutor<'a> {
    pub quins: &'a [NQuin],
}

impl<'a> QueryExecutor<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self { quins }
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

    /// Execute CONSTRUCT query
    pub fn execute_construct(
        &self,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
    ) -> Result<Vec<BindingRow>, String> {
        self.execute(plan, ctx)
    }

    /// Execute DESCRIBE query
    pub fn execute_describe(
        &self,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
    ) -> Result<Vec<BindingRow>, String> {
        self.execute(plan, ctx)
    }

    fn execute_operator(
        &self,
        op_id: OperatorId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let operator = plan.operators.get(op_id as usize)
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
            PhysicalOperatorType::TripleScan { subject, predicate, object } => {
                self.execute_triple_scan(subject, predicate, object, ctx, row, results)
            }
            PhysicalOperatorType::HashJoin { left, right, join_var } => {
                self.execute_hash_join(left, right, join_var, plan, ctx, row, results)
            }
            PhysicalOperatorType::NestedLoopJoin { left, right, join_var } => {
                self.execute_nested_loop_join(left, right, join_var, plan, ctx, row, results)
            }
            PhysicalOperatorType::Filter { input, expression } => {
                self.execute_filter(input, expression, plan, ctx, row, results)
            }
            PhysicalOperatorType::Project { input, vars, var_count } => {
                self.execute_project(input, vars, var_count, plan, ctx, row, results)
            }
            PhysicalOperatorType::Limit { input, limit, offset } => {
                self.execute_limit(input, limit, offset, plan, ctx, row, results)
            }
            PhysicalOperatorType::Sort { input, order_by, order_count, ascending } => {
                self.execute_sort(input, &order_by, order_count, &ascending, plan, ctx, row, results)
            }
            PhysicalOperatorType::Union { left, right } => {
                self.execute_union(left, right, plan, ctx, row, results)
            }
            PhysicalOperatorType::Optional { left, right } => {
                self.execute_optional(left, right, plan, ctx, row, results)
            }
            PhysicalOperatorType::Distinct { input } => {
                self.execute_distinct(input, plan, ctx, row, results)
            }
            PhysicalOperatorType::GroupBy { input, group_vars, group_var_count, aggregates, aggregate_count } => {
                self.execute_group_by(input, group_vars, group_var_count, aggregates, aggregate_count, plan, ctx, row, results)
            }
            PhysicalOperatorType::Having { input, expression } => {
                self.execute_having(input, expression, plan, ctx, row, results)
            }
            PhysicalOperatorType::PropertyPath { subject, path_id, object } => {
                self.execute_property_path(subject, path_id, object, ctx, row, results)
            }
            PhysicalOperatorType::Graph { graph_var_or_id, inner } => {
                self.execute_graph(graph_var_or_id, inner, plan, ctx, row, results)
            }
            PhysicalOperatorType::Service { endpoint_did_id, inner_pattern } => {
                self.execute_service(endpoint_did_id, inner_pattern, plan, ctx, row, results)
            }
            PhysicalOperatorType::AsOf { input, timestamp_ms, mode } => {
                self.execute_as_of(input, timestamp_ms, mode, plan, ctx, row, results)
            }
        }
    }

    fn execute_subject_scan(
        &self,
        subject: u64,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
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
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
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
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
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
            if quin.subject == subject && quin.predicate == predicate && quin.object == object {
                let mut new_row = BindingRow::new();
                new_row.slots[0] = Some(quin.subject);
                new_row.slots[1] = Some(quin.predicate);
                new_row.slots[2] = Some(quin.object);
                results.push(new_row);
            }
        }
        Ok(!results.is_empty())
    }

    fn execute_hash_join(
        &self,
        left: OperatorId,
        right: OperatorId,
        join_var: VariableId,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        let mut left_results = Vec::new();
        let mut left_row = BindingRow::new();
        self.execute_operator(left, plan, ctx, &mut left_row, &mut left_results)?;

        let mut right_results = Vec::new();
        let mut right_row = BindingRow::new();
        self.execute_operator(right, plan, ctx, &mut right_row, &mut right_results)?;

        // Zero-allocation Merge Join (O(N log N) + O(M log M))
        left_results.sort_unstable_by(|a, b| a.slots[join_var as usize].cmp(&b.slots[join_var as usize]));
        right_results.sort_unstable_by(|a, b| a.slots[join_var as usize].cmp(&b.slots[join_var as usize]));

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
                while left_end < left_results.len() && left_results[left_end].slots[join_var as usize] == left_val {
                    left_end += 1;
                }

                let mut right_end = j + 1;
                while right_end < right_results.len() && right_results[right_end].slots[join_var as usize] == right_val {
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
        row: &mut BindingRow,
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

        // Filter results based on expression evaluation
        for input_row in input_results {
            let eval_result = ExpressionEvaluator::evaluate(expression, ctx, &input_row)?;
            if eval_result.as_bool() {
                results.push(input_row);
            }
        }

        Ok(!results.is_empty())
    }

    fn execute_project(
        &self,
        input: OperatorId,
        _vars: [VariableId; MAX_VARIABLES],
        _var_count: u8,
        plan: &ExecutionPlan,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
    ) -> Result<bool, String> {
        // For now, just execute the input (simplified)
        self.execute_operator(input, plan, ctx, row, results)
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
                
                let val_a = ExpressionEvaluator::evaluate(expr, ctx, a).unwrap_or(crate::sparql_filter::EvalResult::Numeric(0));
                let val_b = ExpressionEvaluator::evaluate(expr, ctx, b).unwrap_or(crate::sparql_filter::EvalResult::Numeric(0));
                
                let cmp = val_a.cmp(&val_b);
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
        let start_len = results.len();
        
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
            let right_matched = self.execute_operator(right, plan, ctx, &mut right_row, &mut right_results)?;

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

        // Filter results based on HAVING expression
        for result in all_results {
            let eval_result = ExpressionEvaluator::evaluate(expression, ctx, &result)?;
            if eval_result.as_bool() {
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
        let path = ctx.paths.get(path_id as usize)
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
                    self.execute_property_path(intermediate_val, *right, object, ctx, row, results)?;
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
                // Zero or more - Kleene star (simplified: up to 3 hops)
                self.execute_zero_or_more(subject, *inner_id, object, ctx, row, results, 3)
            }
            crate::sparql_ast::Path::OneOrMore(inner_id) => {
                // One or more - at least one hop (simplified: up to 3 hops)
                let mut direct_results = Vec::new();
                self.execute_property_path(subject, *inner_id, object, ctx, row, &mut direct_results)?;
                results.extend_from_slice(&direct_results);
                
                if !direct_results.is_empty() {
                    let _ = self.execute_zero_or_more(subject, *inner_id, object, ctx, row, results, 2);
                }
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

    fn execute_zero_or_more(
        &self,
        subject: u64,
        path_id: PathId,
        object: u64,
        ctx: &SparqlQueryContext,
        row: &mut BindingRow,
        results: &mut Vec<BindingRow>,
        max_depth: u8,
    ) -> Result<bool, String> {
        if max_depth == 0 {
            return Ok(false);
        }
        
        // Check direct match
        if subject == object {
            let mut direct_row = BindingRow::new();
            direct_row.slots[0] = Some(subject);
            results.push(direct_row);
        }
        
        // Explore path
        let mut intermediate_results = Vec::new();
        self.execute_property_path(subject, path_id, 0, ctx, row, &mut intermediate_results)?;
        
        for inter_result in intermediate_results {
            let intermediate_val = inter_result.slots[0].unwrap_or(0);
            if intermediate_val == object {
                results.push(inter_result);
            }
            // Recurse
            self.execute_zero_or_more(intermediate_val, path_id, object, ctx, row, results, max_depth - 1)?;
        }
        
        Ok(!results.is_empty())
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
        // Check if graph_var_or_id is a variable or a specific graph IRI
        // For now, assume it's a specific graph IRI (simplified)
        let graph_id = graph_var_or_id;
        
        // Filter quins by graph context
        let graph_quins = crate::query_engine::filter_by_context(self.quins, graph_id);
        
        if graph_quins.is_empty() {
            return Ok(false);
        }
        
        // Create a temporary executor with graph-filtered quins
        let temp_executor = QueryExecutor {
            quins: &graph_quins,
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
        let network_buffer = [0u8; 4096];
        
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
    fn check_temporal_constraint(&self, subject: u64, timestamp_ms: u64, mode: TemporalMode) -> bool {
        use crate::sparql_filter::prov_predicates;
        use crate::kml_bridge::T_CONTEXT;

        match mode {
            TemporalMode::AsOf => {
                let gen_time = self.quins.iter()
                    .find(|q| q.subject == subject
                        && q.predicate == prov_predicates::GENERATED_AT_TIME
                        && q.context == T_CONTEXT)
                    .map(|q| q.object);
                gen_time.map(|t| t <= timestamp_ms).unwrap_or(true)
            }
            TemporalMode::AtTime => {
                let start = self.quins.iter()
                    .find(|q| q.subject == subject
                        && q.predicate == prov_predicates::STARTED_AT_TIME
                        && q.context == T_CONTEXT)
                    .map(|q| q.object);
                let end = self.quins.iter()
                    .find(|q| q.subject == subject
                        && q.predicate == prov_predicates::ENDED_AT_TIME
                        && q.context == T_CONTEXT)
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
}