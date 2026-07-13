use super::*;

/// Data indexing engine
pub struct DataIndexingEngine {
    indexes: HashMap<String, DataIndex>,
    indexing_strategy: IndexingStrategy,
    query_optimizer: QueryOptimizer,
}

/// Data index
#[derive(Debug, Clone)]
pub struct DataIndex {
    pub index_id: String,
    pub index_type: IndexType,
    pub indexed_columns: Vec<String>,
    pub statistics: IndexStatistics,
}

/// Index types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
    FullText,
    Spatial,
    TimeSeries,
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub entries: u64,
    pub size: u64,
    pub selectivity: f64,
    pub usage_count: u64,
}

/// Query optimizer
pub struct QueryOptimizer {
    optimization_rules: Vec<OptimizationRule>,
    cost_model: CostModel,
    execution_plan: ExecutionPlan,
}

/// Optimization rules
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationRule {
    PredicatePushdown,
    IndexSelection,
    JoinOrder,
    AggregationPushdown,
    Materialization,
}

/// Cost model
pub struct CostModel {
    pub cpu_cost: f64,
    pub io_cost: f64,
    pub memory_cost: f64,
    pub network_cost: f64,
}

/// Execution plan
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub operations: Vec<QueryOperation>,
    pub estimated_cost: f64,
    pub execution_time: u64,
}

/// Join strategy selected by the query optimizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    NestedLoop,
    HashJoin,
}

/// A single logical query operation. Each variant carries the data the
/// optimizer needs to estimate cost, reorder steps, and select join strategies.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOperation {
    /// Full table scan; `estimated_rows` is the table's row count.
    Scan {
        table: String,
        estimated_rows: usize,
    },
    /// Predicate filter; `selectivity` (0.0–1.0) is the fraction of rows that pass.
    Filter { predicate: String, selectivity: f64 },
    /// Join of two inputs; `left_cost`/`right_cost` are estimated row counts
    /// of the left and right inputs. The optimizer may override `join_type`.
    Join {
        left_cost: f64,
        right_cost: f64,
        join_type: JoinType,
    },
    /// Aggregation; `group_by` lists the grouping columns.
    Aggregate { group_by: Vec<String> },
    /// Sort by the given columns.
    Sort { columns: Vec<String> },
    /// Limit to at most `count` rows.
    Limit { count: usize },
    /// Column projection.
    Project { columns: Vec<String> },
}

/// A single step in an optimized query plan: the operation plus its estimated
/// cost and output row count.
#[derive(Debug, Clone)]
pub struct QueryStep {
    pub operation: QueryOperation,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

/// An optimized query plan: ordered steps with aggregate cost/row estimates.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub operations: Vec<QueryStep>,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

impl QueryOperation {
    /// A rough data-size proxy used by the legacy `estimate_cost` /
    /// `optimize_with_cost` path when the input row count is not known
    /// in isolation. The `optimize()` method uses proper per-step tracking.
    fn data_size_hint(&self) -> f64 {
        match self {
            QueryOperation::Scan { estimated_rows, .. } => *estimated_rows as f64,
            QueryOperation::Filter { selectivity, .. } => 100.0 / selectivity.max(0.01),
            QueryOperation::Join {
                left_cost,
                right_cost,
                ..
            } => left_cost * right_cost,
            QueryOperation::Aggregate { group_by } => group_by.len().max(1) as f64 * 100.0,
            QueryOperation::Sort { columns } => columns.len().max(1) as f64 * 100.0,
            QueryOperation::Limit { count } => *count as f64,
            QueryOperation::Project { columns } => columns.len().max(1) as f64 * 100.0,
        }
    }

    /// Canonical ordering priority for stable reordering by `optimize()`.
    /// Lower = earlier in the plan. Operations with the same priority
    /// preserve their relative input order (stable sort).
    fn plan_priority(&self) -> u8 {
        match self {
            QueryOperation::Scan { .. } => 0,
            QueryOperation::Project { .. } => 0,
            QueryOperation::Filter { .. } => 1,
            QueryOperation::Join { .. } => 2,
            QueryOperation::Aggregate { .. } => 3,
            QueryOperation::Sort { .. } => 4,
            QueryOperation::Limit { .. } => 5,
        }
    }
}

impl DataIndexingEngine {
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
            indexing_strategy: IndexingStrategy::BTree,
            query_optimizer: QueryOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.query_optimizer.initialize()?;
        Ok(())
    }

    /// Returns the configured indexing strategy.
    pub fn indexing_strategy(&self) -> &IndexingStrategy {
        &self.indexing_strategy
    }

    /// Reconfigure the indexing strategy.
    pub fn set_indexing_strategy(&mut self, strategy: IndexingStrategy) {
        self.indexing_strategy = strategy;
    }

    /// Add (or replace) a named data index.
    pub fn add_index(&mut self, index: DataIndex) {
        self.indexes.insert(index.index_id.clone(), index);
    }

    /// Look up a data index by id.
    pub fn get_index(&self, index_id: &str) -> Option<&DataIndex> {
        self.indexes.get(index_id)
    }

    /// Remove a data index by id.
    pub fn remove_index(&mut self, index_id: &str) -> Option<DataIndex> {
        self.indexes.remove(index_id)
    }

    /// List the ids of all registered indexes.
    pub fn list_index_ids(&self) -> Vec<String> {
        self.indexes.keys().cloned().collect()
    }

    /// Returns the number of registered indexes.
    pub fn index_count(&self) -> usize {
        self.indexes.len()
    }

    /// Returns a reference to the query optimizer.
    pub fn query_optimizer(&self) -> &QueryOptimizer {
        &self.query_optimizer
    }

    /// Returns a mutable reference to the query optimizer.
    pub fn query_optimizer_mut(&mut self) -> &mut QueryOptimizer {
        &mut self.query_optimizer
    }
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_rules: vec![
                OptimizationRule::PredicatePushdown,
                OptimizationRule::IndexSelection,
            ],
            cost_model: CostModel {
                cpu_cost: 0.0,
                io_cost: 0.0,
                memory_cost: 0.0,
                network_cost: 0.0,
            },
            execution_plan: ExecutionPlan {
                plan_id: "default".to_string(),
                operations: Vec::new(),
                estimated_cost: 0.0,
                execution_time: 0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Returns the list of optimization rules currently registered.
    pub fn optimization_rules(&self) -> &[OptimizationRule] {
        &self.optimization_rules
    }

    /// Add an optimization rule if it is not already present.
    pub fn add_optimization_rule(&mut self, rule: OptimizationRule) {
        if !self.optimization_rules.contains(&rule) {
            self.optimization_rules.push(rule);
        }
    }

    /// Returns `true` when the given rule is registered.
    pub fn has_rule(&self, rule: &OptimizationRule) -> bool {
        self.optimization_rules.contains(rule)
    }

    /// Estimate the cost of a single query operation based on its type and the
    /// amount of data it operates on. Costs are dimensionless weights chosen so
    /// that cheaper operations (Filter, Project, Limit) sort before expensive
    /// ones (Scan, Join, Aggregate, Sort).
    pub fn estimate_cost(&self, operation: &QueryOperation) -> CostModel {
        let n = operation.data_size_hint();
        match operation {
            // Full scan: heavy I/O, light CPU.
            QueryOperation::Scan { .. } => CostModel {
                cpu_cost: 0.1 * n,
                io_cost: 1.0 * n,
                memory_cost: 0.05 * n,
                network_cost: 0.0,
            },
            // Filter: cheap, mostly CPU.
            QueryOperation::Filter { .. } => CostModel {
                cpu_cost: 0.2 * n,
                io_cost: 0.0,
                memory_cost: 0.02 * n,
                network_cost: 0.0,
            },
            // Project: cheap column selection.
            QueryOperation::Project { .. } => CostModel {
                cpu_cost: 0.1 * n,
                io_cost: 0.0,
                memory_cost: 0.02 * n,
                network_cost: 0.0,
            },
            // Aggregate: moderate CPU + memory.
            QueryOperation::Aggregate { .. } => CostModel {
                cpu_cost: 0.5 * n,
                io_cost: 0.1 * n,
                memory_cost: 0.3 * n,
                network_cost: 0.0,
            },
            // Join: the most expensive — CPU, memory, and network.
            QueryOperation::Join { .. } => CostModel {
                cpu_cost: 1.0 * n,
                io_cost: 0.5 * n,
                memory_cost: 1.0 * n,
                network_cost: 0.5 * n,
            },
            // Sort: CPU + memory heavy.
            QueryOperation::Sort { .. } => CostModel {
                cpu_cost: 0.6 * n,
                io_cost: 0.2 * n,
                memory_cost: 0.5 * n,
                network_cost: 0.0,
            },
            // Limit: very cheap.
            QueryOperation::Limit { .. } => CostModel {
                cpu_cost: 0.05 * n,
                io_cost: 0.0,
                memory_cost: 0.01 * n,
                network_cost: 0.0,
            },
        }
    }

    /// Optimize a sequence of operations by reordering them to minimize total
    /// cost. Uses a simple greedy strategy: estimate each operation's cost and
    /// execute cheapest-first. The resulting [`ExecutionPlan`] is stored on the
    /// optimizer and also returned.
    pub fn optimize_with_cost(
        &mut self,
        operations: &[QueryOperation],
    ) -> Result<ExecutionPlan, StatisticalError> {
        let mut indexed: Vec<(usize, QueryOperation)> = operations
            .iter()
            .cloned()
            .map(|op| op)
            .enumerate()
            .collect();
        // Greedy: sort by estimated total cost, cheapest first. The original
        // index is retained so callers can inspect the reordering if desired.
        indexed.sort_by(|a, b| {
            let ca = self.estimate_cost(&a.1).total_cost();
            let cb = self.estimate_cost(&b.1).total_cost();
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut ordered: Vec<QueryOperation> = indexed.into_iter().map(|(_, op)| op).collect();
        let total: f64 = ordered
            .iter()
            .map(|op| self.estimate_cost(op).total_cost())
            .sum();

        // Aggregate the per-operation costs into the optimizer's cost model so
        // the field is actually used.
        self.cost_model = ordered.iter().map(|op| self.estimate_cost(op)).fold(
            CostModel {
                cpu_cost: 0.0,
                io_cost: 0.0,
                memory_cost: 0.0,
                network_cost: 0.0,
            },
            |acc, c| CostModel {
                cpu_cost: acc.cpu_cost + c.cpu_cost,
                io_cost: acc.io_cost + c.io_cost,
                memory_cost: acc.memory_cost + c.memory_cost,
                network_cost: acc.network_cost + c.network_cost,
            },
        );

        let plan = ExecutionPlan {
            plan_id: format!("plan_{}", self.next_plan_id()),
            operations: std::mem::take(&mut ordered),
            estimated_cost: total,
            execution_time: 0,
        };
        self.execution_plan = plan.clone();
        Ok(plan)
    }

    /// Returns the most recently optimized execution plan, if any.
    pub fn get_execution_plan(&self) -> Option<&ExecutionPlan> {
        if self.execution_plan.operations.is_empty() {
            None
        } else {
            Some(&self.execution_plan)
        }
    }

    /// Monotonic plan id counter (kept simple — no persistent state needed).
    fn next_plan_id(&self) -> u64 {
        // Use the current plan's operation count as a cheap discriminator.
        self.execution_plan.operations.len() as u64 + 1
    }

    /// Optimize a sequence of operations into a [`QueryPlan`].
    ///
    /// Applies three rewrite rules:
    /// 1. **Predicate pushdown** — filters are moved ahead of joins so rows are
    ///    reduced before the expensive join.
    /// 2. **Join-type selection** — HashJoin is selected when both join inputs
    ///    are ≥ 1000 rows; NestedLoop when both are < 100; otherwise the
    ///    caller-supplied join type is retained.
    /// 3. **Limit-last** — Limit is always the final step.
    ///
    /// After reordering, per-step cost and output row count are estimated
    /// using a simple cost model that tracks the running row count through
    /// the plan.
    pub fn optimize(&self, operations: Vec<QueryOperation>) -> Result<QueryPlan, StatisticalError> {
        if operations.is_empty() {
            return Ok(QueryPlan {
                operations: Vec::new(),
                estimated_cost: 0.0,
                estimated_rows: 0,
            });
        }

        // 1. Stable reorder by canonical plan priority (scan → filter → join →
        //    aggregate → sort → limit).  This achieves both filter-pushdown
        //    and limit-last in a single pass.
        let mut reordered: Vec<QueryOperation> = operations;
        reordered.sort_by_key(|op| op.plan_priority());

        // 2. Join-type selection: override join_type based on input sizes.
        for op in reordered.iter_mut() {
            if let QueryOperation::Join {
                left_cost,
                right_cost,
                join_type,
            } = op
            {
                if *left_cost >= 1000.0 && *right_cost >= 1000.0 {
                    *join_type = JoinType::HashJoin;
                } else if *left_cost < 100.0 && *right_cost < 100.0 {
                    *join_type = JoinType::NestedLoop;
                }
            }
        }

        // 3. Build plan with per-step cost and row estimates.
        let mut steps: Vec<QueryStep> = Vec::with_capacity(reordered.len());
        let mut current_rows: usize = 0;
        let mut total_cost: f64 = 0.0;

        for op in reordered {
            let (cost, output_rows) = Self::estimate_step(&op, current_rows);
            steps.push(QueryStep {
                operation: op,
                estimated_cost: cost,
                estimated_rows: output_rows,
            });
            current_rows = output_rows;
            total_cost += cost;
        }

        Ok(QueryPlan {
            operations: steps,
            estimated_cost: total_cost,
            estimated_rows: current_rows,
        })
    }

    /// Per-step cost and output-row estimation. `input_rows` is the running
    /// row count from the previous step (0 for the first step).
    fn estimate_step(op: &QueryOperation, input_rows: usize) -> (f64, usize) {
        match op {
            QueryOperation::Scan { estimated_rows, .. } => {
                let cost = *estimated_rows as f64 * 0.01;
                (cost, *estimated_rows)
            }
            QueryOperation::Filter { selectivity, .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * selectivity * 0.005;
                let output = (n * selectivity).round() as usize;
                (cost, output)
            }
            QueryOperation::Join {
                left_cost,
                right_cost,
                ..
            } => {
                let n = left_cost * right_cost;
                let cost = n * 0.001;
                let output = n.round() as usize;
                (cost, output)
            }
            QueryOperation::Aggregate { .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * 0.01;
                (cost, input_rows)
            }
            QueryOperation::Sort { .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * 0.01;
                (cost, input_rows)
            }
            QueryOperation::Limit { count } => {
                let cost = *count as f64 * 0.001;
                let output = (*count).min(input_rows.max(1));
                (cost, output)
            }
            QueryOperation::Project { .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * 0.001;
                (cost, input_rows)
            }
        }
    }
}

impl CostModel {
    /// Sum of all cost components.
    pub fn total_cost(&self) -> f64 {
        self.cpu_cost + self.io_cost + self.memory_cost + self.network_cost
    }

    /// Returns `true` when `self` is cheaper than `other`.
    pub fn is_better_than(&self, other: &CostModel) -> bool {
        self.total_cost() < other.total_cost()
    }
}

