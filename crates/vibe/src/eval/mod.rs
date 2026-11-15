//! AST interpreter. No JIT.

use crate::ast::*;
use crate::bind::Host;
use crate::budget::Budget;
use crate::error::{DiagCode, Diagnostic};
use crate::extension_registry::{
    is_extension_namespace, CallExecutionContext, DispatchOutcome, ExtensionRegistry,
    PreparedCallCache,
};
use crate::span::Span;
use crate::value::Value;
use std::collections::{HashMap, HashSet};
use vibescript_abi::{q_hash_str, ExecutionMode, SentinelPolicyGate};

mod budget;
use budget::budget_steps;
mod call;
mod expr;
mod ops;
pub(crate) use ops::lit;
use ops::{binop, collect_pat_idents, match_pat};
mod program;
mod stack;
mod stmt;
mod task_runtime;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct Env {
    /// Runtime task ownership. Clones share effects; Default starts an isolated scope.
    /// This opaque value is not a serializable grant or human delegation.
    pub task_scope: crate::task_scheduler::TaskScope,
    pub vars: HashMap<String, Value>,
    /// Set of variable names that were declared with `mut` (T10).
    pub mutables: HashSet<String>,
    /// Import alias -> namespace name (e.g. `g` -> `graph`).
    /// Populated from `import "vibe:0.1/graph" as g;` declarations.
    pub aliases: HashMap<String, String>,
    /// User-defined enum declarations (T9). Maps enum name -> declaration.
    pub enums: HashMap<String, EnumDecl>,
    /// Field declarations (T28). Maps field name -> record Value.
    pub fields: HashMap<String, Value>,
    /// Material declarations (T29). Maps material name -> record Value.
    pub materials: HashMap<String, Value>,
    /// Law declarations (T30). Maps law name -> record Value.
    pub laws: HashMap<String, Value>,
}

impl Default for Env {
    fn default() -> Self {
        Self::with_task_scope(crate::task_scheduler::TaskScope::default())
    }
}

impl Env {
    /// Function/hook locals share the admitted scope without allocating another owner.
    fn with_task_scope(task_scope: crate::task_scheduler::TaskScope) -> Self {
        Self {
            task_scope,
            vars: HashMap::new(),
            mutables: HashSet::new(),
            aliases: HashMap::new(),
            enums: HashMap::new(),
            fields: HashMap::new(),
            materials: HashMap::new(),
            laws: HashMap::new(),
        }
    }
}

/// The valid 0.1 namespace import paths. `vibe:0.1/{ns}`.
pub const VIBE_0_1_NAMESPACES: &[&str] = &[
    "math",
    "rdf",
    "quin",
    "graph",
    "aura",
    "pulse",
    "capability",
    "time",
    "conservation",
    "causal",
    "dag",
    "deontic",
    "hid",
    "cue",
    "crypto",
    "zk",
    "web",
    "ui",
    "form",
    "input",
    "validate",
    "selection",
    "layout",
    "cbor_ld",
    "reflect",
    "record",
    "uri",
    "error",
    "num",
    "decimal",
    "autodiff",
    "str",
    "regex",
    "list",
    "deque",
    "pqueue",
    "set",
    "simplex",
    "hypergraph",
    "sheaf",
    "topology",
    "homotopy",
    "context",
    "manifold",
    "inference",
    "ast",
    "diagnostic",
    "effect",
    "code",
    "task",
    "supervisor",
    "contract",
    "state",
    "value",
    "runtime",
    "continuation",
    "memory",
    "simd",
    "channel",
];

/// Populate `env.aliases` from a program's import declarations.
/// Each `import "vibe:0.1/graph" as g;` maps `g` -> `graph`.
/// Also supports Vibe 0.03 extension namespace IRIs and short aliases (`mech`, `gpu`, `infer`, `qgraph`, `modality`).
pub fn populate_import_aliases(env: &mut Env, imports: &[ImportDecl]) -> Result<(), Diagnostic> {
    let mut seen = HashSet::new();
    for imp in imports {
        let ns = imp
            .path
            .strip_prefix("vibe:0.1/")
            .or_else(|| imp.path.strip_prefix("vibe:0.3/"))
            .unwrap_or(&imp.path);
        if !VIBE_0_1_NAMESPACES.contains(&ns) && !is_extension_namespace(ns) {
            return Err(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!(
                    "unknown namespace '{}'; valid: {}",
                    ns,
                    VIBE_0_1_NAMESPACES.join(", ")
                ),
            ));
        }
        let default_alias = ns.rsplit('/').next().unwrap_or(ns);
        let alias = imp.alias.as_deref().unwrap_or(default_alias);
        if !seen.insert(alias) {
            return Err(Diagnostic::new(
                DiagCode::E100,
                imp.span,
                format!("duplicate import alias '{alias}'"),
            ));
        }
        env.aliases.insert(alias.to_string(), ns.to_string());
    }
    Ok(())
}

fn default_call_context() -> CallExecutionContext {
    CallExecutionContext {
        actor_did_hash: q_hash_str("did:q42:local_engine"),
        context_world_hash: 1,
        lamport_timestamp: 1,
        mode: ExecutionMode::StandaloneDeveloper,
    }
}

pub struct Engine<'a, H: Host> {
    pub(crate) active_tasks: Vec<(crate::task_scheduler::TaskScope, u64)>,
    host: &'a mut H,
    budget: Budget,
    depth: u32,
    call_depth_limit: u32,
    expression_depth: u32,
    expression_depth_limit: u32,
    /// Budget expressions may use operators/builtins, never user functions.
    evaluating_budget: bool,
    /// The program being evaluated, if any.
    program: Option<std::sync::Arc<Program>>,
    /// Optional deontic phase leaser (R2).
    phase_leaser: Option<&'a mut crate::deontic_interrupt::PhaseLeaser>,
    /// Capability ids granted by `using` / `requires` on the current program.
    granted: Vec<String>,
    /// Generational extension registry (`Stage S6`).
    pub(crate) extension_registry: ExtensionRegistry<'a>,
    /// Zero-heap prepared call handle cache (`Stage S6`).
    pub(crate) prepared_cache: PreparedCallCache,
    /// Active extension execution context (`mode`, `actor_did_hash`, `context_world_hash`, `lamport_timestamp`).
    pub(crate) call_context: CallExecutionContext,
    /// Optional Sentinel policy gate for governed/simulated execution modes (`Stage S6`).
    pub(crate) sentinel_gate: Option<&'a mut dyn SentinelPolicyGate>,
    /// Most recent extension `DispatchOutcome` (receipt + audit `NQuin48`s).
    pub(crate) last_extension_outcome: Option<DispatchOutcome>,
}

pub(super) enum Flow {
    Next(Value),
    Return(Value),
}

impl<'a, H: Host> Engine<'a, H> {
    /// Smaller stack profile for admitted source sessions.
    pub(crate) fn set_session_stack_limits(&mut self, depth: u32, expression_depth: u32) {
        self.call_depth_limit = depth.clamp(1, 8);
        self.expression_depth_limit = expression_depth.clamp(1, 64);
    }

    /// Workspace remaining.
    pub fn workspace_left(&self) -> u64 {
        self.budget.workspace_left
    }

    /// Creates a new `Engine` with all built-in `vibe-0.03` extension providers registered.
    pub fn new(host: &'a mut H, budget: Budget) -> Self {
        Self {
            active_tasks: Vec::new(),
            host,
            budget,
            depth: 0,
            call_depth_limit: 64,
            expression_depth: 0,
            expression_depth_limit: u32::MAX,
            evaluating_budget: false,
            program: None,
            phase_leaser: None,
            granted: Vec::new(),
            extension_registry: ExtensionRegistry::with_builtin_providers(),
            prepared_cache: PreparedCallCache::new(),
            call_context: default_call_context(),
            sentinel_gate: None,
            last_extension_outcome: None,
        }
    }

    /// Attach a program so `eval_call` can resolve user-defined functions.
    pub fn with_program(host: &'a mut H, budget: Budget, program: &Program) -> Self {
        let mut engine = Self::new(host, budget);
        engine.program = Some(std::sync::Arc::new(program.clone()));
        engine.granted = program.requires.iter().map(|c| c.id.clone()).collect();
        engine
    }

    /// Attach a deontic phase leaser (R2).
    pub fn with_phase_leaser(
        host: &'a mut H,
        budget: Budget,
        leaser: &'a mut crate::deontic_interrupt::PhaseLeaser,
    ) -> Self {
        let mut engine = Self::new(host, budget);
        engine.phase_leaser = Some(leaser);
        engine
    }

    /// Attach an explicit `ExtensionRegistry`, `CallExecutionContext`, and optional `SentinelPolicyGate` (`Stage S6`).
    pub fn with_extension_governance(
        host: &'a mut H,
        budget: Budget,
        registry: ExtensionRegistry<'a>,
        call_context: CallExecutionContext,
        gate: Option<&'a mut dyn SentinelPolicyGate>,
    ) -> Self {
        let mut engine = Self::new(host, budget);
        engine.extension_registry = registry;
        engine.call_context = call_context;
        engine.sentinel_gate = gate;
        engine
    }

    /// Returns a reference to the engine's `ExtensionRegistry`.
    #[inline]
    pub fn extension_registry(&self) -> &ExtensionRegistry<'a> {
        &self.extension_registry
    }

    /// Returns a mutable reference to the engine's `ExtensionRegistry`.
    #[inline]
    pub fn extension_registry_mut(&mut self) -> &mut ExtensionRegistry<'a> {
        &mut self.extension_registry
    }

    /// Configures the `CallExecutionContext` and optional `SentinelPolicyGate`.
    #[inline]
    pub fn set_extension_governance(
        &mut self,
        call_context: CallExecutionContext,
        gate: Option<&'a mut dyn SentinelPolicyGate>,
    ) {
        self.call_context = call_context;
        self.sentinel_gate = gate;
    }

    /// Returns the most recent extension `DispatchOutcome` (receipt and audit `NQuin48`s).
    #[inline]
    pub fn last_extension_outcome(&self) -> Option<&DispatchOutcome> {
        self.last_extension_outcome.as_ref()
    }

    /// Returns the active Sentinel policy generation (or `0` when in standalone developer mode).
    #[inline]
    pub fn active_policy_generation(&self) -> Result<u64, vibescript_abi::VibeTrap> {
        if self.call_context.mode.requires_sentinel_gate() {
            let gate = self
                .sentinel_gate
                .as_ref()
                .ok_or(vibescript_abi::VibeTrap::SentinelUnavailable)?;
            if !gate.is_controller_alive() {
                return Err(vibescript_abi::VibeTrap::SentinelUnavailable);
            }
            Ok(gate.current_policy_generation())
        } else {
            Ok(0)
        }
    }

    /// Granted refs.
    pub(crate) fn granted_refs(&self) -> Vec<&str> {
        self.granted.iter().map(|s| s.as_str()).collect()
    }

    /// Check whether a capability path is allowed by the active phase lease.
    pub(crate) fn check_phase_capability(&self, path: &str, span: Span) -> Result<(), Diagnostic> {
        let Some(leaser) = &self.phase_leaser else {
            return Ok(());
        };
        let cap = path.split('.').next().unwrap_or(path);
        if leaser.is_interrupted() {
            return Err(Diagnostic::new(
                DiagCode::E700,
                span,
                format!(
                    "deontic interrupt: agent is halted, capability '{}' denied",
                    cap
                ),
            ));
        }
        if !leaser.is_leased(cap) {
            return Err(Diagnostic::new(
                DiagCode::E700,
                span,
                format!(
                    "deontic phase violation: capability '{}' is not leased in the current phase",
                    cap
                ),
            ));
        }
        Ok(())
    }
}
