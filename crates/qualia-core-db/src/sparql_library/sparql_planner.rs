//! SPARQL Logical Query Planner
//!
//! Transforms parsed AST into an execution plan using zero-allocation patterns.

use crate::sparql_ast::*;

/// Physical operator types for the execution plan
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalOperatorType {
    /// Scan all quins matching a subject
    SubjectScan { subject: u64 },
    /// Scan all quins matching a predicate
    PredicateScan { predicate: u64 },
    /// Scan all quins matching an object
    ObjectScan { object: u64 },
    /// Triple pattern scan with all three components
    TripleScan {
        subject: u64,
        predicate: u64,
        object: u64,
    },
    /// Hash join between two operators
    HashJoin {
        left: OperatorId,
        right: OperatorId,
        join_var: VariableId,
    },
    /// Nested loop join for small datasets
    NestedLoopJoin {
        left: OperatorId,
        right: OperatorId,
        join_var: VariableId,
    },
    /// Filter operator
    Filter {
        input: OperatorId,
        expression: ExpressionId,
    },
    /// Bind operator — extends each input row with `var` = eval(`expression`)
    /// (SPARQL 1.1 Extend). Never drops rows; an expression error leaves the
    /// variable unbound.
    Bind {
        input: OperatorId,
        var: VariableId,
        expression: ExpressionId,
    },
    /// Projection operator
    Project {
        input: OperatorId,
        vars: [VariableId; MAX_VARIABLES],
        var_count: u8,
    },
    /// Limit operator
    Limit {
        input: OperatorId,
        limit: u64,
        offset: u64,
    },
    /// Sort operator
    Sort {
        input: OperatorId,
        order_by: [ExpressionId; MAX_ORDER_CONDITIONS],
        order_count: u8,
        ascending: [bool; MAX_ORDER_CONDITIONS],
    },
    /// Union operator
    Union { left: OperatorId, right: OperatorId },
    /// Optional operator (SPARQL 1.1 left-join)
    Optional { left: OperatorId, right: OperatorId },
    /// Anti-join operator (SPARQL 1.1 MINUS): keep a left solution unless a
    /// right solution is compatible with it AND shares a bound variable.
    AntiJoin { left: OperatorId, right: OperatorId },
    /// Distinct operator
    Distinct { input: OperatorId },
    /// Sub-SELECT: evaluate stored subquery `query_id` independently and join
    /// its projected solutions with the enclosing bindings.
    SubSelect { query_id: u16 },
    /// GroupBy operator with Aggregates
    GroupBy {
        input: OperatorId,
        group_vars: [VariableId; MAX_VARIABLES],
        group_var_count: u8,
        aggregates: [AggregateSpec; 16],
        aggregate_count: u8,
    },
    /// Having operator
    Having {
        input: OperatorId,
        expression: ExpressionId,
    },
    /// Property path operator
    PropertyPath {
        subject: u64,
        path_id: PathId,
        object: u64,
    },
    /// Graph operator
    Graph {
        graph_var_or_id: u64,
        inner: OperatorId,
    },
    /// Service operator (Federated Query with DID)
    Service {
        endpoint_did_id: u64,
        inner_pattern: OperatorId,
    },
    /// AS OF / AT TIME temporal snapshot operator (Phase 4).
    AsOf {
        input: OperatorId,
        timestamp_ms: u64,
        mode: TemporalMode,
    },
    /// RDF-Star annotation triple scan.
    StarTripleScan {
        inner_subject: u64,
        inner_predicate: u64,
        inner_object: u64,
        outer_predicate: u64,
        outer_object: u64,
    },
}

pub type OperatorId = u16;

/// Aggregate specification
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateSpec {
    pub func: u8, // 0=COUNT, 1=SUM, 2=AVG, 3=MIN, 4=MAX
    pub input_var: VariableId,
    pub output_var: VariableId,
}

/// Execution plan operator
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlanOperator {
    pub operator_type: PhysicalOperatorType,
    pub estimated_cardinality: u64,
}

/// Execution plan
#[repr(C)]
pub struct ExecutionPlan {
    pub operators: [PlanOperator; 64], // Max 64 operators in a plan
    pub operator_count: u8,
    pub root_operator: OperatorId,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            operators: [PlanOperator {
                operator_type: PhysicalOperatorType::SubjectScan { subject: 0 },
                estimated_cardinality: 0,
            }; 64],
            operator_count: 0,
            root_operator: 0,
        }
    }

    pub fn add_operator(
        &mut self,
        op: PhysicalOperatorType,
        cardinality: u64,
    ) -> Result<OperatorId, String> {
        if self.operator_count >= 64 {
            return Err("Operator overflow".to_string());
        }
        let id = self.operator_count as OperatorId;
        self.operators[self.operator_count as usize] = PlanOperator {
            operator_type: op,
            estimated_cardinality: cardinality,
        };
        self.operator_count += 1;
        Ok(id)
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Query planner
pub struct QueryPlanner;

impl QueryPlanner {
    /// Plan a SPARQL query into an execution plan
    pub fn plan(query: &SparqlQuery, ctx: &SparqlQueryContext) -> Result<ExecutionPlan, String> {
        match query {
            SparqlQuery::Select(select) => Self::plan_select(select, ctx),
            SparqlQuery::Ask(ask) => Self::plan_ask(ask, ctx),
            SparqlQuery::Construct(construct) => Self::plan_construct(construct, ctx),
            SparqlQuery::Describe(describe) => Self::plan_describe(describe, ctx),
        }
    }

    fn plan_select(
        select: &SelectQuery,
        ctx: &SparqlQueryContext,
    ) -> Result<ExecutionPlan, String> {
        let mut plan = ExecutionPlan::new();

        // Plan the WHERE clause
        let root_op = Self::plan_pattern(select.root_pattern, ctx, &mut plan)?;

        // Apply GroupBy/Aggregates
        let group_op = if select.group_by_count > 0 || select.aggregate_count > 0 {
            let mut group_vars = [0u8; MAX_VARIABLES];
            if select.group_by_count > 0 {
                group_vars[..select.group_by_count as usize]
                    .copy_from_slice(&select.group_by[..select.group_by_count as usize]);
            }
            let mut aggregates = [crate::sparql_planner::AggregateSpec {
                func: 0,
                input_var: 0,
                output_var: 0,
            }; 16];
            if select.aggregate_count > 0 {
                aggregates[..select.aggregate_count as usize]
                    .copy_from_slice(&select.aggregates[..select.aggregate_count as usize]);
            }
            plan.add_operator(
                PhysicalOperatorType::GroupBy {
                    input: root_op,
                    group_vars,
                    group_var_count: select.group_by_count,
                    aggregates,
                    aggregate_count: select.aggregate_count,
                },
                0,
            )?
        } else {
            root_op
        };

        // Apply projection
        let project_op = if select.var_count > 0 {
            let mut vars = [0u8; MAX_VARIABLES];
            vars[..select.var_count as usize]
                .copy_from_slice(&select.variables[..select.var_count as usize]);
            plan.add_operator(
                PhysicalOperatorType::Project {
                    input: group_op,
                    vars,
                    var_count: select.var_count,
                },
                0, // Cardinality unknown
            )?
        } else {
            root_op
        };

        // Apply DISTINCT / REDUCED — dedup the projected solutions. (Previously
        // the `distinct` flag was parsed but never planned, so `SELECT DISTINCT`
        // silently returned duplicates.) Eliminating all duplicates is also a
        // conformant realisation of REDUCED.
        let distinct_op = if select.distinct || select.reduced {
            plan.add_operator(
                PhysicalOperatorType::Distinct { input: project_op },
                0,
            )?
        } else {
            project_op
        };

        // Apply sorting
        let sort_op = if select.order_by_count > 0 {
            let mut order_by = [0u16; MAX_ORDER_CONDITIONS];
            let mut ascending = [true; MAX_ORDER_CONDITIONS];
            for i in 0..select.order_by_count as usize {
                order_by[i] = select.order_by[i].expr;
                ascending[i] = select.order_by[i].ascending;
            }
            plan.add_operator(
                PhysicalOperatorType::Sort {
                    input: distinct_op,
                    order_by,
                    order_count: select.order_by_count,
                    ascending,
                },
                0,
            )?
        } else {
            distinct_op
        };

        // Apply limit/offset
        let final_op = if select.limit.is_some() || select.offset > 0 {
            plan.add_operator(
                PhysicalOperatorType::Limit {
                    input: sort_op,
                    limit: select.limit.unwrap_or(u64::MAX),
                    offset: select.offset,
                },
                0,
            )?
        } else {
            sort_op
        };

        plan.root_operator = final_op;
        Ok(plan)
    }

    fn plan_ask(ask: &AskQuery, ctx: &SparqlQueryContext) -> Result<ExecutionPlan, String> {
        let mut plan = ExecutionPlan::new();
        let root_op = Self::plan_pattern(ask.root_pattern, ctx, &mut plan)?;
        plan.root_operator = root_op;
        Ok(plan)
    }

    fn plan_construct(
        construct: &ConstructQuery,
        ctx: &SparqlQueryContext,
    ) -> Result<ExecutionPlan, String> {
        let mut plan = ExecutionPlan::new();
        let root_op = Self::plan_pattern(construct.root_pattern, ctx, &mut plan)?;
        plan.root_operator = root_op;
        Ok(plan)
    }

    fn plan_describe(
        describe: &DescribeQuery,
        ctx: &SparqlQueryContext,
    ) -> Result<ExecutionPlan, String> {
        let mut plan = ExecutionPlan::new();
        if let Some(pattern_id) = describe.root_pattern {
            let root_op = Self::plan_pattern(pattern_id, ctx, &mut plan)?;
            plan.root_operator = root_op;
        }
        Ok(plan)
    }

    pub(crate) fn plan_pattern(
        pattern_id: PatternId,
        ctx: &SparqlQueryContext,
        plan: &mut ExecutionPlan,
    ) -> Result<OperatorId, String> {
        let pattern = ctx
            .patterns
            .get(pattern_id as usize)
            .ok_or("Pattern ID out of bounds")?;

        match pattern {
            Pattern::Triple {
                subject,
                predicate,
                object,
            } => plan.add_operator(
                PhysicalOperatorType::TripleScan {
                    subject: *subject,
                    predicate: *predicate,
                    object: *object,
                },
                0,
            ),
            Pattern::StarTriple {
                inner_subject,
                inner_predicate,
                inner_object,
                outer_predicate,
                outer_object,
            } => plan.add_operator(
                PhysicalOperatorType::StarTripleScan {
                    inner_subject: *inner_subject,
                    inner_predicate: *inner_predicate,
                    inner_object: *inner_object,
                    outer_predicate: *outer_predicate,
                    outer_object: *outer_object,
                },
                0,
            ),
            Pattern::Optional { inner } => {
                let inner_op = Self::plan_pattern(*inner, ctx, plan)?;
                // For now, just return the inner operator (simplified)
                Ok(inner_op)
            }
            Pattern::Union { left, right } => {
                let left_op = Self::plan_pattern(*left, ctx, plan)?;
                let right_op = Self::plan_pattern(*right, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Union {
                        left: left_op,
                        right: right_op,
                    },
                    0,
                )
            }

            Pattern::Filter {
                pattern: inner_pattern,
                expression,
            } => {
                let inner_op = Self::plan_pattern(*inner_pattern, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Filter {
                        input: inner_op,
                        expression: *expression,
                    },
                    0,
                )
            }
            Pattern::Bind {
                pattern: inner_pattern,
                var,
                expression,
            } => {
                let inner_op = Self::plan_pattern(*inner_pattern, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Bind {
                        input: inner_op,
                        var: *var,
                        expression: *expression,
                    },
                    0,
                )
            }
            Pattern::Minus { inner } => Self::plan_pattern(*inner, ctx, plan),
            Pattern::Group { start_idx, len } => {
                // Plan the group's children left-to-right. Plain children join
                // (natural join — the nested-loop operator checks compatibility
                // across all bound slots). OPTIONAL children become a real
                // left-join and MINUS children a real anti-join against the
                // accumulated result, rather than being folded into a plain join
                // (which would make OPTIONAL required and MINUS behave like an
                // intersection — both wrong).
                let mut current_op: Option<OperatorId> = None;
                for i in *start_idx..(*start_idx + *len) {
                    let child = ctx
                        .patterns
                        .get(i as usize)
                        .ok_or("Pattern ID out of bounds")?;
                    match child {
                        Pattern::Optional { inner } => {
                            let right = Self::plan_pattern(*inner, ctx, plan)?;
                            current_op = Some(match current_op {
                                Some(left) => plan.add_operator(
                                    PhysicalOperatorType::Optional { left, right },
                                    0,
                                )?,
                                // A leading OPTIONAL has no required left side —
                                // its solutions stand alone.
                                None => right,
                            });
                        }
                        Pattern::Minus { inner } => {
                            let right = Self::plan_pattern(*inner, ctx, plan)?;
                            current_op = Some(match current_op {
                                Some(left) => plan.add_operator(
                                    PhysicalOperatorType::AntiJoin { left, right },
                                    0,
                                )?,
                                // MINUS with nothing to subtract from is
                                // degenerate (malformed SPARQL); nothing is
                                // removed, so fall back to the inner solutions.
                                None => right,
                            });
                        }
                        _ => {
                            let pattern_op = Self::plan_pattern(i, ctx, plan)?;
                            current_op = Some(match current_op {
                                Some(curr) => plan.add_operator(
                                    PhysicalOperatorType::NestedLoopJoin {
                                        left: curr,
                                        right: pattern_op,
                                        join_var: 0,
                                    },
                                    0,
                                )?,
                                None => pattern_op,
                            });
                        }
                    }
                }
                current_op.ok_or("Empty group pattern".to_string())
            }
            Pattern::PropertyPath {
                subject,
                path,
                object,
            } => plan.add_operator(
                PhysicalOperatorType::PropertyPath {
                    subject: *subject,
                    path_id: *path,
                    object: *object,
                },
                0,
            ),
            Pattern::SubSelect { query_id } => plan.add_operator(
                PhysicalOperatorType::SubSelect {
                    query_id: *query_id,
                },
                0,
            ),
            Pattern::Graph {
                graph_var_or_id,
                inner,
            } => {
                let inner_op = Self::plan_pattern(*inner, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Graph {
                        graph_var_or_id: *graph_var_or_id,
                        inner: inner_op,
                    },
                    0,
                )
            }
            Pattern::Service {
                endpoint_did_id,
                inner_pattern,
            } => {
                let inner_op = Self::plan_pattern(*inner_pattern, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Service {
                        endpoint_did_id: *endpoint_did_id,
                        inner_pattern: inner_op,
                    },
                    0,
                )
            }
            Pattern::AsOf {
                inner,
                timestamp_ms,
                mode,
            } => {
                let inner_op = Self::plan_pattern(*inner, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::AsOf {
                        input: inner_op,
                        timestamp_ms: *timestamp_ms,
                        mode: *mode,
                    },
                    0,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_plan_creation() {
        let plan = ExecutionPlan::new();
        assert_eq!(plan.operator_count, 0);
    }

    #[test]
    fn test_add_operator() {
        let mut plan = ExecutionPlan::new();
        let op = PhysicalOperatorType::SubjectScan { subject: 42 };
        let id = plan.add_operator(op, 0).unwrap();
        assert_eq!(id, 0);
        assert_eq!(plan.operator_count, 1);
    }

    #[test]
    fn test_plan_triple_pattern() {
        let mut ctx = SparqlQueryContext::new();
        let pattern = Pattern::Triple {
            subject: 1,
            predicate: 2,
            object: 3,
        };
        let pattern_id = ctx.alloc_pattern(pattern).unwrap();

        let mut plan = ExecutionPlan::new();
        let op_id = QueryPlanner::plan_pattern(pattern_id, &ctx, &mut plan).unwrap();
        assert_eq!(op_id, 0);
        assert_eq!(plan.operator_count, 1);
    }
}
