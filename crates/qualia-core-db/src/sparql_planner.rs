//! SPARQL Logical Query Planner
//!
//! Transforms parsed AST into an execution plan using zero-allocation patterns.

use crate::sparql_ast::*;

/// Physical operator types for the execution plan
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalOperatorType {
    /// Scan all quins matching a subject
    SubjectScan {
        subject: u64,
    },
    /// Scan all quins matching a predicate
    PredicateScan {
        predicate: u64,
    },
    /// Scan all quins matching an object
    ObjectScan {
        object: u64,
    },
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
    Union {
        left: OperatorId,
        right: OperatorId,
    },
    /// Optional operator
    Optional {
        left: OperatorId,
        right: OperatorId,
    },
    /// Distinct operator
    Distinct {
        input: OperatorId,
    },
    /// GroupBy operator
    GroupBy {
        input: OperatorId,
        group_vars: [VariableId; MAX_VARIABLES],
        group_var_count: u8,
    },
    /// Aggregate operator
    Aggregate {
        input: OperatorId,
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
}

pub type OperatorId = u16;

/// Aggregate specification
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateSpec {
    pub func: u8, // 0=COUNT, 1=SUM, 2=AVG, 3=MIN, 4=MAX
    pub var: VariableId,
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

    pub fn add_operator(&mut self, op: PhysicalOperatorType, cardinality: u64) -> Result<OperatorId, String> {
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

    fn plan_select(select: &SelectQuery, ctx: &SparqlQueryContext) -> Result<ExecutionPlan, String> {
        let mut plan = ExecutionPlan::new();
        
        // Plan the WHERE clause
        let root_op = Self::plan_pattern(select.root_pattern, ctx, &mut plan)?;
        
        // Apply projection
        let project_op = if select.var_count > 0 {
            let mut vars = [0u8; MAX_VARIABLES];
            vars[..select.var_count as usize].copy_from_slice(&select.variables[..select.var_count as usize]);
            plan.add_operator(
                PhysicalOperatorType::Project {
                    input: root_op,
                    vars,
                    var_count: select.var_count,
                },
                0, // Cardinality unknown
            )?
        } else {
            root_op
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
                    input: project_op,
                    order_by,
                    order_count: select.order_by_count,
                    ascending,
                },
                0,
            )?
        } else {
            project_op
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

    fn plan_construct(construct: &ConstructQuery, ctx: &SparqlQueryContext) -> Result<ExecutionPlan, String> {
        let mut plan = ExecutionPlan::new();
        let root_op = Self::plan_pattern(construct.root_pattern, ctx, &mut plan)?;
        plan.root_operator = root_op;
        Ok(plan)
    }

    fn plan_describe(describe: &DescribeQuery, ctx: &SparqlQueryContext) -> Result<ExecutionPlan, String> {
        let mut plan = ExecutionPlan::new();
        if let Some(pattern_id) = describe.root_pattern {
            let root_op = Self::plan_pattern(pattern_id, ctx, &mut plan)?;
            plan.root_operator = root_op;
        }
        Ok(plan)
    }

    fn plan_pattern(pattern_id: PatternId, ctx: &SparqlQueryContext, plan: &mut ExecutionPlan) -> Result<OperatorId, String> {
        let pattern = ctx.patterns.get(pattern_id as usize)
            .ok_or("Pattern ID out of bounds")?;
        
        match pattern {
            Pattern::Triple { subject, predicate, object } => {
                // Determine which scan to use based on what's bound
                // For now, use simple heuristic: if subject is a constant, use subject scan
                // If subject is variable, use triple scan
                let op = if *subject < 0x8000_0000_0000_0000 { // Likely a hash (not a did:q42 pointer)
                    PhysicalOperatorType::SubjectScan { subject: *subject }
                } else {
                    PhysicalOperatorType::TripleScan {
                        subject: *subject,
                        predicate: *predicate,
                        object: *object,
                    }
                };
                plan.add_operator(op, 0)
            }
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
            Pattern::Graph { graph_var_or_id, inner } => {
                // For now, just plan the inner pattern
                Self::plan_pattern(*inner, ctx, plan)
            }
            Pattern::Filter { pattern: inner_pattern, expression } => {
                let inner_op = Self::plan_pattern(*inner_pattern, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Filter {
                        input: inner_op,
                        expression: *expression,
                    },
                    0,
                )
            }
            Pattern::Minus { inner } => {
                Self::plan_pattern(*inner, ctx, plan)
            }
            Pattern::Group { start_idx, len } => {
                // Plan all patterns in the group and join them
                let mut current_op: Option<OperatorId> = None;
                for i in *start_idx..(*start_idx + *len) {
                    let pattern_op = Self::plan_pattern(i, ctx, plan)?;
                    if let Some(curr) = current_op {
                        // Join with current
                        current_op = Some(plan.add_operator(
                            PhysicalOperatorType::NestedLoopJoin {
                                left: curr,
                                right: pattern_op,
                                join_var: 0, // TODO: Determine join variable
                            },
                            0,
                        )?);
                    } else {
                        current_op = Some(pattern_op);
                    }
                }
                current_op.ok_or("Empty group pattern".to_string())
            }
            Pattern::PropertyPath { subject, path, object } => {
                plan.add_operator(
                    PhysicalOperatorType::PropertyPath {
                        subject: *subject,
                        path_id: *path,
                        object: *object,
                    },
                    0,
                )
            }
            Pattern::Graph { graph_var_or_id, inner } => {
                let inner_op = Self::plan_pattern(*inner, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Graph {
                        graph_var_or_id: *graph_var_or_id,
                        inner: inner_op,
                    },
                    0,
                )
            }
            Pattern::Service { endpoint_did_id, inner_pattern } => {
                let inner_op = Self::plan_pattern(*inner_pattern, ctx, plan)?;
                plan.add_operator(
                    PhysicalOperatorType::Service {
                        endpoint_did_id: *endpoint_did_id,
                        inner_pattern: inner_op,
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
        assert_eq!(plan.operator_count, 1);
    }
}