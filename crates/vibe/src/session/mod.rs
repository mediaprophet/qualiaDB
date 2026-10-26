//! Bounded in-memory checked source sessions for the pinned 0.1 interpreter.
//! This is not durable project recovery, live definition replacement, or a job API.
use crate::ast::*;
use crate::check::{check_submission, CheckEnv};
use crate::{Budget, DiagCode, Diagnostic, Engine, Env, Host, Span, Value};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
mod prepare;
mod source_limits;
#[cfg(test)]
mod tests;
mod values;
#[cfg(test)]
mod validation_tests;

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct SessionLimits {
    pub submission_bytes: usize,
    pub retained_source_bytes: usize,
    pub declarations: usize,
    pub bindings: usize,
    /// Ceiling on retained values' Debug representation, measured without
    /// allocating a serialization buffer. This is not an allocator hard limit.
    pub retained_value_bytes: usize,
    pub revisions: u64,
    /// Source-session admitted range 1..=8 calls, clamped to that range.
    pub call_depth: u32,
    /// Total evaluation nesting across functions, admitted range 1..=64.
    pub expression_depth: u32,
}
impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            submission_bytes: 64 * 1024,
            retained_source_bytes: 1024 * 1024,
            declarations: 1024,
            bindings: 1024,
            retained_value_bytes: crate::SENTINEL_BYTES as usize,
            revisions: 1024,
            call_depth: 8,
            expression_depth: 64,
        }
    }
}

/// A runtime error preserves the original diagnostic. Host state is outside the
/// tentative local frame: a dispatched operation may have committed or failed
/// after committing. Reconcile authoritative receipts with the actual host.
#[derive(Debug)]
pub struct SubmissionError {
    pub diagnostic: Diagnostic,
    pub execution_attempted: bool,
    pub host_reconciliation_required: bool,
    pub active_revision: u64,
}

#[derive(Debug)]
pub struct SubmissionResult {
    pub value: Value,
    pub revision: u64,
}

enum Body {
    Program(Program),
    Expression(Expr),
}

/// Checked candidates cannot be constructed or edited by the caller. They are
/// bound to their originating session and active revision, consumed once.
pub struct CheckedSubmission {
    owner: u64,
    base_revision: u64,
    source: String,
    definitions: Program,
    checked: CheckEnv,
    mutables: HashSet<String>,
    body: Body,
}

pub struct SourceSession {
    owner: u64,
    revision: u64,
    limits: SessionLimits,
    /// Actual grants are supplied by the host, never inferred from requests.
    grants: Vec<String>,
    definitions: Program,
    checked: CheckEnv,
    mutables: HashSet<String>,
    env: Env,
    sources: Vec<String>,
    source_bytes: usize,
}

impl Default for SourceSession {
    fn default() -> Self {
        Self::new(Vec::new(), SessionLimits::default())
    }
}

impl SourceSession {
    pub fn new(grants: Vec<String>, limits: SessionLimits) -> Self {
        Self {
            owner: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            limits,
            grants,
            definitions: Program {
                span: Span::point(0),
                module: None,
                imports: Vec::new(),
                prefixes: Vec::new(),
                locales: Vec::new(),
                requires: Vec::new(),
                items: Vec::new(),
            },
            checked: CheckEnv::default(),
            mutables: HashSet::new(),
            env: Env::default(),
            sources: Vec::new(),
            source_bytes: 0,
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn value(&self, name: &str) -> Option<&Value> {
        self.env.vars.get(name)
    }
    pub fn is_mutable(&self, name: &str) -> bool {
        self.mutables.contains(name)
    }
    pub fn sources(&self) -> &[String] {
        &self.sources
    }
    pub fn binding_type(&self, name: &str) -> Option<String> {
        self.checked.get(name).map(|t| format!("{t:?}"))
    }
    fn error(&self, diagnostic: Diagnostic, attempted: bool) -> SubmissionError {
        SubmissionError {
            diagnostic,
            execution_attempted: attempted,
            // No Value::Receipt/default host return is promoted to verified
            // effect evidence. Even a statically Pure binding can call Host.
            host_reconciliation_required: attempted,
            active_revision: self.revision,
        }
    }
    pub fn submit<H: Host>(
        &mut self,
        source: &str,
        host: &mut H,
        budget: Budget,
    ) -> Result<SubmissionResult, SubmissionError> {
        self.submit_validated(source, host, budget, |_| Ok(()))
    }
    /// Validate evaluated output before promoting any tentative local values,
    /// definitions or source. Host effects already attempted are not rolled back.
    pub fn submit_validated<H: Host, F: FnOnce(&Value) -> Result<(), Diagnostic>>(
        &mut self,
        source: &str,
        host: &mut H,
        budget: Budget,
        validate: F,
    ) -> Result<SubmissionResult, SubmissionError> {
        let candidate = self.prepare(source).map_err(|e| self.error(e, false))?;
        self.execute_validated(candidate, host, budget, validate)
    }
    pub fn execute<H: Host>(
        &mut self,
        candidate: CheckedSubmission,
        host: &mut H,
        budget: Budget,
    ) -> Result<SubmissionResult, SubmissionError> {
        self.execute_validated(candidate, host, budget, |_| Ok(()))
    }
    /// The validator receives actual evaluated output once, after interpreter
    /// checks and before promotion. It may recheck native authority while its
    /// owner transaction is held; no JSON receipt is treated as authority.
    pub fn execute_validated<H: Host, F: FnOnce(&Value) -> Result<(), Diagnostic>>(
        &mut self,
        candidate: CheckedSubmission,
        host: &mut H,
        budget: Budget,
        validate: F,
    ) -> Result<SubmissionResult, SubmissionError> {
        if candidate.owner != self.owner || candidate.base_revision != self.revision {
            return Err(self.error(
                Diagnostic::new(
                    DiagCode::E701,
                    Span::point(0),
                    "checked submission belongs to another session or stale revision",
                ),
                false,
            ));
        }
        let mut tentative = self.env.clone();
        crate::populate_import_aliases(&mut tentative, &candidate.definitions.imports)
            .map_err(|e| self.error(e, false))?;
        let mut budget = budget;
        budget.workspace_left = budget
            .workspace_left
            .min(self.limits.retained_value_bytes as u64);
        let mut engine = Engine::with_program(host, budget, &candidate.definitions);
        engine.set_session_stack_limits(self.limits.call_depth, self.limits.expression_depth);
        // Only this submission executes. Old top-level statements, constants,
        // materials and cell initialization are absent from `body`.
        let result = match &candidate.body {
            Body::Program(program) => engine.eval_program(program, &mut tentative),
            Body::Expression(expr) => engine.eval_expr(expr, &mut tentative),
        };
        let value = result.map_err(|e| self.error(e, true))?;
        if tentative.vars.len() > self.limits.bindings {
            return Err(self.error(
                Diagnostic::new(
                    DiagCode::E400,
                    Span::point(0),
                    "session binding limit exceeded after evaluation",
                ),
                true,
            ));
        }
        values::validate(&candidate.checked, &tentative).map_err(|e| self.error(e, true))?;
        values::check_retained_size(&tentative, self.limits.retained_value_bytes)
            .map_err(|e| self.error(e, true))?;
        validate(&value).map_err(|e| self.error(e, true))?;
        self.env = tentative;
        self.checked = candidate.checked;
        self.mutables = candidate.mutables;
        self.definitions = candidate.definitions;
        self.source_bytes += candidate.source.len();
        self.sources.push(candidate.source);
        self.revision += 1;
        Ok(SubmissionResult {
            value,
            revision: self.revision,
        })
    }
}
