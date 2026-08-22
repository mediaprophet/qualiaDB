use std::collections::{HashMap, HashSet};

use crate::ast::{CellDecl, Expr, ExprKind, Item, Program};
use crate::bind::{Host, LocalHost};
use crate::error::Diagnostic;
use crate::eval::{Engine, Env};
use crate::value::{Instant, Value};

/// An error that can occur in the reactive cell graph engine.
#[derive(Debug, Clone, PartialEq)]
pub enum ReactiveCellError {
    /// A cyclic dependency was detected among reactive cells.
    CycleDetected {
        cycle: Vec<String>,
    },
    /// A cell with the given name was not found.
    CellNotFound(String),
    /// Evaluation error during cell computation.
    EvaluationError(String),
}

impl std::fmt::Display for ReactiveCellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CycleDetected { cycle } => {
                write!(f, "cycle detected in reactive cell graph: {}", cycle.join(" -> "))
            }
            Self::CellNotFound(name) => write!(f, "reactive cell '{name}' not found"),
            Self::EvaluationError(msg) => write!(f, "evaluation error in cell: {msg}"),
        }
    }
}

impl std::error::Error for ReactiveCellError {}

impl From<Diagnostic> for ReactiveCellError {
    fn from(diag: Diagnostic) -> Self {
        Self::EvaluationError(diag.message)
    }
}

/// A compiled reactive cell node.
#[derive(Debug, Clone)]
pub struct ReactiveCell {
    /// Unique identifier / index in graph.
    pub id: usize,
    /// Cell identifier name.
    pub name: String,
    /// The original declaration AST.
    pub decl: CellDecl,
    /// Names of variables and cells this cell depends on.
    pub dependencies: Vec<String>,
    /// Indices of cells that depend on this cell (downstream).
    pub dependents: Vec<usize>,
    /// Current evaluated cached value.
    pub value: Value,
    /// True if the cell needs re-evaluation.
    pub is_dirty: bool,
    /// True if this cell takes time/clock input parameters (e.g. `t: Instant`).
    pub is_temporal: bool,
}

/// Compiled reactive cell dependency graph.
#[derive(Debug, Clone)]
pub struct ReactiveCellGraph {
    /// List of cell nodes.
    pub cells: Vec<ReactiveCell>,
    /// Map from cell name to its index in `cells`.
    pub name_to_index: HashMap<String, usize>,
    /// Topologically sorted order of cell indices (dependencies before dependents).
    pub topo_order: Vec<usize>,
    /// Dirty bitset words (64 cells per word; grows past 64 cells).
    pub dirty_words: Vec<u64>,
    /// Current logical time clock (in seconds since epoch / start).
    pub clock_seconds: f64,
}

impl ReactiveCellGraph {
    /// Construct a new empty reactive cell graph.
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            name_to_index: HashMap::new(),
            topo_order: Vec::new(),
            dirty_words: Vec::new(),
            clock_seconds: 0.0,
        }
    }

    /// Build a reactive cell graph from an AST Program.
    pub fn from_program(program: &Program) -> Result<Self, ReactiveCellError> {
        let mut graph = Self::new();
        for item in &program.items {
            if let Item::Cell(cd) = item {
                graph.add_cell(cd.clone());
            }
        }
        graph.compile()?;
        Ok(graph)
    }

    /// Add a cell declaration to the graph.
    pub fn add_cell(&mut self, decl: CellDecl) -> usize {
        let id = self.cells.len();
        let name = decl.name.clone();
        let is_temporal = !decl.params.is_empty();
        let mut dependencies = extract_expr_dependencies(&decl.expr);
        if let Some(ref when_expr) = decl.when {
            dependencies.extend(extract_expr_dependencies(when_expr));
        }

        let cell = ReactiveCell {
            id,
            name: name.clone(),
            decl,
            dependencies,
            dependents: Vec::new(),
            value: Value::Null,
            is_dirty: true,
            is_temporal,
        };

        self.name_to_index.insert(name, id);
        self.cells.push(cell);
        self.set_dirty_bit(id);
        id
    }

    fn dirty_word_bit(idx: usize) -> (usize, u64) {
        (idx / 64, 1u64 << (idx % 64))
    }

    fn ensure_dirty_word(&mut self, word: usize) {
        if self.dirty_words.len() <= word {
            self.dirty_words.resize(word + 1, 0);
        }
    }

    fn set_dirty_bit(&mut self, idx: usize) {
        let (word, bit) = Self::dirty_word_bit(idx);
        self.ensure_dirty_word(word);
        self.dirty_words[word] |= bit;
    }

    fn clear_dirty_bit(&mut self, idx: usize) {
        let (word, bit) = Self::dirty_word_bit(idx);
        self.ensure_dirty_word(word);
        self.dirty_words[word] &= !bit;
    }

    /// True if cell `idx` is marked dirty. Works past 64 cells.
    pub fn is_dirty_bit(&self, idx: usize) -> bool {
        let (word, bit) = Self::dirty_word_bit(idx);
        self.dirty_words
            .get(word)
            .map(|w| w & bit != 0)
            .unwrap_or(false)
    }

    /// First 64 dirty bits (debug / HUD). Prefer [`Self::is_dirty_bit`].
    pub fn dirty_mask(&self) -> u64 {
        self.dirty_words.first().copied().unwrap_or(0)
    }

    /// Compile dependencies, wire dependents, check cycles, and compute topological order.
    pub fn compile(&mut self) -> Result<(), ReactiveCellError> {
        let n = self.cells.len();
        for i in 0..n {
            self.cells[i].dependents.clear();
        }

        // Build adjacency: dependency -> dependent
        for i in 0..n {
            let deps = self.cells[i].dependencies.clone();
            for dep in deps {
                if let Some(&dep_idx) = self.name_to_index.get(&dep) {
                    if !self.cells[dep_idx].dependents.contains(&i) {
                        self.cells[dep_idx].dependents.push(i);
                    }
                }
            }
        }

        // Kahn's algorithm for topological sorting and cycle detection
        let mut in_degree = vec![0usize; n];
        for i in 0..n {
            for &dep in &self.cells[i].dependents {
                in_degree[dep] += 1;
            }
        }

        let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut order = Vec::with_capacity(n);

        while let Some(u) = queue.pop() {
            order.push(u);
            for &v in &self.cells[u].dependents {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push(v);
                }
            }
        }

        if order.len() < n {
            // Cycle detected — collect remaining nodes with in_degree > 0
            let cycle_nodes: Vec<String> = (0..n)
                .filter(|&i| in_degree[i] > 0)
                .map(|i| self.cells[i].name.clone())
                .collect();
            return Err(ReactiveCellError::CycleDetected { cycle: cycle_nodes });
        }

        self.topo_order = order;
        self.mark_all_dirty();
        Ok(())
    }

    /// Mark a single cell and all its transitive downstream dependents as dirty.
    pub fn mark_dirty(&mut self, name: &str) {
        if let Some(&idx) = self.name_to_index.get(name) {
            self.mark_dirty_index(idx);
        }
    }

    fn mark_dirty_index(&mut self, idx: usize) {
        self.cells[idx].is_dirty = true;
        self.set_dirty_bit(idx);
        for &dep in &self.cells[idx].dependents.clone() {
            self.mark_dirty_index(dep);
        }
    }

    /// Mark all cells in the graph as dirty.
    pub fn mark_all_dirty(&mut self) {
        for cell in &mut self.cells {
            cell.is_dirty = true;
        }
        let words = if self.cells.is_empty() {
            0
        } else {
            (self.cells.len() + 63) / 64
        };
        self.dirty_words = vec![0; words];
        for i in 0..self.cells.len() {
            self.set_dirty_bit(i);
        }
    }

    /// Set an external input signal / variable and mark downstream cells dirty.
    pub fn set_input(&mut self, name: &str, value: Value, env: &mut Env) {
        env.vars.insert(name.to_string(), value.clone());
        if let Some(&idx) = self.name_to_index.get(name) {
            self.cells[idx].value = value;
            self.cells[idx].is_dirty = false;
            self.clear_dirty_bit(idx);
            for &dep in &self.cells[idx].dependents.clone() {
                self.mark_dirty_index(dep);
            }
        } else {
            self.mark_dirty(name);
        }
    }

    /// Advance time and evaluate dirty cells on a **local** in-process host.
    ///
    /// Prefer [`Self::step_with`] when a live `Host` (Poet, tests with
    /// query rows, GPU) must dispatch capability.invoke.
    pub fn step(
        &mut self,
        env: &mut Env,
        dt_seconds: f64,
    ) -> Result<Vec<(String, Value)>, ReactiveCellError> {
        let mut local_host = LocalHost::default();
        let mut engine = Engine::new(&mut local_host, crate::budget::Budget::default());
        self.step_with(&mut engine, env, dt_seconds)
    }

    /// Homogeneous ready-cluster size for P16.2. Scalar path still
    /// evaluates one cell at a time; the batch is the set of dirty cells
    /// whose dependencies are already clean, capped at 512.
    pub const CELL_BATCH_512: usize = 512;

    /// One ready cluster of up to 512 dirty cells, then stop.
    /// Callers loop until the dirty set is empty.
    pub fn evaluate_cell_batch_512<H: Host>(
        &mut self,
        engine: &mut Engine<'_, H>,
        env: &mut Env,
        dt_seconds: f64,
    ) -> Result<Vec<(String, Value)>, ReactiveCellError> {
        self.clock_seconds += dt_seconds;
        let secs = self.clock_seconds.floor() as i64;
        let nanos = ((self.clock_seconds - self.clock_seconds.floor()) * 1e9) as u32;
        let clock_instant = Instant::unix(secs, nanos);

        for i in 0..self.cells.len() {
            if self.cells[i].is_temporal {
                self.mark_dirty_index(i);
            }
        }

        let mut ready = Vec::new();
        for &idx in &self.topo_order {
            if ready.len() >= crate::accel::cell_batch_cap() {
                break;
            }
            if !self.cells[idx].is_dirty {
                continue;
            }
            let deps_ready = self.cells[idx]
                .dependencies
                .iter()
                .all(|name| match self.name_to_index.get(name) {
                    Some(&dep) => !self.cells[dep].is_dirty,
                    None => true,
                });
            if deps_ready {
                ready.push(idx);
            }
        }

        let mut updated = Vec::new();
        for idx in ready {
            if self.cells[idx].is_temporal {
                if let Some(first_param) = self.cells[idx].decl.params.first() {
                    env.vars.insert(
                        first_param.name.clone(),
                        Value::Instant(clock_instant.clone()),
                    );
                }
            }
            let should_run = if let Some(ref cond) = self.cells[idx].decl.when {
                match engine.eval_expr(cond, env) {
                    Ok(v) => v.is_truthy(),
                    Err(e) => return Err(ReactiveCellError::EvaluationError(e.message)),
                }
            } else {
                true
            };
            if should_run {
                match engine.eval_expr(&self.cells[idx].decl.expr, env) {
                    Ok(val) => {
                        self.cells[idx].value = val.clone();
                        self.cells[idx].is_dirty = false;
                        self.clear_dirty_bit(idx);
                        env.vars.insert(self.cells[idx].name.clone(), val.clone());
                        updated.push((self.cells[idx].name.clone(), val));
                    }
                    Err(e) => return Err(ReactiveCellError::EvaluationError(e.message)),
                }
            } else {
                self.cells[idx].is_dirty = false;
                self.clear_dirty_bit(idx);
            }
        }
        Ok(updated)
    }

    /// Advance time and evaluate dirty cells through the caller's engine/host.
    pub fn step_with<H: Host>(
        &mut self,
        engine: &mut Engine<'_, H>,
        env: &mut Env,
        dt_seconds: f64,
    ) -> Result<Vec<(String, Value)>, ReactiveCellError> {
        self.clock_seconds += dt_seconds;
        let secs = self.clock_seconds.floor() as i64;
        let nanos = ((self.clock_seconds - self.clock_seconds.floor()) * 1e9) as u32;
        let clock_instant = Instant::unix(secs, nanos);

        for i in 0..self.cells.len() {
            if self.cells[i].is_temporal {
                self.mark_dirty_index(i);
            }
        }

        let mut updated = Vec::new();
        let topo = self.topo_order.clone();

        for &idx in &topo {
            if self.cells[idx].is_dirty {
                if self.cells[idx].is_temporal {
                    if let Some(first_param) = self.cells[idx].decl.params.first() {
                        env.vars.insert(
                            first_param.name.clone(),
                            Value::Instant(clock_instant.clone()),
                        );
                    }
                }

                let should_run = if let Some(ref cond) = self.cells[idx].decl.when {
                    match engine.eval_expr(cond, env) {
                        Ok(v) => v.is_truthy(),
                        Err(e) => return Err(ReactiveCellError::EvaluationError(e.message)),
                    }
                } else {
                    true
                };

                if should_run {
                    match engine.eval_expr(&self.cells[idx].decl.expr, env) {
                        Ok(val) => {
                            self.cells[idx].value = val.clone();
                            self.cells[idx].is_dirty = false;
                            self.clear_dirty_bit(idx);
                            env.vars.insert(self.cells[idx].name.clone(), val.clone());
                            updated.push((self.cells[idx].name.clone(), val));
                        }
                        Err(e) => return Err(ReactiveCellError::EvaluationError(e.message)),
                    }
                } else {
                    self.cells[idx].is_dirty = false;
                    self.clear_dirty_bit(idx);
                }
            }
        }

        Ok(updated)
    }

    /// Retrieve the current cached value of a cell by name.
    pub fn get_value(&self, name: &str) -> Option<&Value> {
        self.name_to_index.get(name).map(|&idx| &self.cells[idx].value)
    }
}

impl Default for ReactiveCellGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively collect all identifier variable names referenced in an expression.
fn extract_expr_dependencies(expr: &Expr) -> Vec<String> {
    let mut deps = HashSet::new();
    walk_expr_deps(expr, &mut deps);
    deps.into_iter().collect()
}

fn walk_expr_deps(expr: &Expr, deps: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::Ident(name) => {
            deps.insert(name.clone());
        }
        ExprKind::Binary { left, right, .. } => {
            walk_expr_deps(left, deps);
            walk_expr_deps(right, deps);
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Await(expr)
        | ExprKind::Try(expr) => {
            walk_expr_deps(expr, deps);
        }
        ExprKind::Member { recv, .. } => {
            walk_expr_deps(recv, deps);
        }
        ExprKind::Index { recv, index } => {
            walk_expr_deps(recv, deps);
            walk_expr_deps(index, deps);
        }
        ExprKind::Call { callee, args } => {
            walk_expr_deps(callee, deps);
            for arg in args {
                match arg {
                    crate::ast::Arg::Pos(e) => walk_expr_deps(e, deps),
                    crate::ast::Arg::Named(na) => walk_expr_deps(&na.value, deps),
                }
            }
        }
        ExprKind::List(items) => {
            for item in items {
                walk_expr_deps(item, deps);
            }
        }
        ExprKind::Record(fields) => {
            for f in fields {
                walk_expr_deps(&f.value, deps);
            }
        }
        ExprKind::Interpolate(parts) => {
            for p in parts {
                walk_expr_deps(p, deps);
            }
        }
        ExprKind::Pipe { left, right } => {
            walk_expr_deps(left, deps);
            walk_expr_deps(right, deps);
        }
        ExprKind::Triple { subject, predicate, object } => {
            walk_expr_deps(subject, deps);
            walk_expr_deps(predicate, deps);
            walk_expr_deps(object, deps);
        }
        ExprKind::Reified { subject, predicate, object, reifier } => {
            walk_expr_deps(subject, deps);
            walk_expr_deps(predicate, deps);
            walk_expr_deps(object, deps);
            walk_expr_deps(reifier, deps);
        }
        ExprKind::Lambda { body, .. } => walk_expr_deps(body, deps),
        ExprKind::Tween {
            from, to, over, ..
        } => {
            walk_expr_deps(from, deps);
            walk_expr_deps(to, deps);
            walk_expr_deps(over, deps);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Env;
    use crate::parse::parse_program;

    #[test]
    fn dirty_bits_cover_more_than_64_cells() {
        let mut src = String::from("cell c0 := 0;\n");
        for i in 1..70 {
            src.push_str(&format!("cell c{i} := c{} + 1;\n", i - 1));
        }
        let prog = parse_program(&src).expect("parse");
        let mut graph = ReactiveCellGraph::from_program(&prog).expect("graph");
        assert!(graph.cells.len() >= 70);
        assert!(graph.is_dirty_bit(0));
        assert!(graph.is_dirty_bit(69));
        let mut env = Env::default();
        graph.step(&mut env, 0.0).expect("step");
        assert!(!graph.is_dirty_bit(69), "step should clear dirty bits past 64");
        graph.mark_dirty("c0");
        assert!(graph.is_dirty_bit(0));
        assert!(
            graph.is_dirty_bit(69),
            "downstream cell 69 must dirty when c0 changes"
        );
    }

    #[test]
    fn cell_batch_512_evaluates_ready_cluster() {
        let src = "cell a := 1;\ncell b := 2;\ncell c := a + b;\n";
        let prog = parse_program(src).expect("parse");
        let mut graph = ReactiveCellGraph::from_program(&prog).expect("graph");
        let mut env = Env::default();
        let mut host = crate::bind::LocalHost::default();
        let mut engine = Engine::new(&mut host, crate::budget::Budget::default());
        let updated = graph
            .evaluate_cell_batch_512(&mut engine, &mut env, 0.0)
            .expect("batch");
        assert!(!updated.is_empty());
        assert!(updated.len() <= ReactiveCellGraph::CELL_BATCH_512);
        assert_eq!(graph.get_value("a"), Some(&Value::I64(1)));
        assert_eq!(graph.get_value("b"), Some(&Value::I64(2)));
        // c depends on a and b; first ready cluster is a and b.
        if graph.is_dirty_bit(graph.name_to_index["c"]) {
            graph
                .evaluate_cell_batch_512(&mut engine, &mut env, 0.0)
                .expect("second batch");
        }
        assert_eq!(graph.get_value("c"), Some(&Value::I64(3)));
    }
}
