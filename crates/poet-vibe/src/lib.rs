//! Poet engine — VibeScript 0.1.
//!
//! Copyright © 2026 Timothy Charles Holborn. All rights reserved.
//! Language version `vibe-0.1`. No JIT. No raw Quin overlay literals.

pub const LANGUAGE_VERSION: &str = "vibe-0.1";

mod ast;
mod ast_query;
mod bind;
mod budget;
pub mod bytecode;
mod capability_schema;
mod cbor_ast;
mod check;
mod diagnose;
mod effects;
mod error;
mod eval;
mod grammar;
mod lex;
mod parse;
mod span;
mod types;
mod unicode_ident;
mod value;

pub mod animation;
pub mod ast_merge;
pub mod cosmic;
pub mod crypto;
pub mod dag;
pub mod decompiler;
pub mod deontic_interrupt;
pub mod frame_morphism;
pub mod hid;
pub mod law_package;
pub mod locale;
pub mod module_system;
pub mod observer;
pub mod physics;
pub mod presentation;
pub mod pretty;
pub mod projectional;
pub mod quantity;
pub mod reflection;
pub mod replay_clock;
pub mod sheaf;
pub mod tick_policy;
pub mod translate;
pub mod trivia;

pub use ast::{
    Arg, ArmBody, BinOp, Block, CapSpec, ConstDecl, EffectClass, EnumDecl, EnumVariant, Expr,
    ExprKind, FieldDecl, FieldRepresentation, FieldSupport, FunctionDecl, HookDecl, ImportDecl,
    Item, LawDecl, Literal, MatchArm, MaterialDecl, ModuleDecl, NamedArg, Param, Pattern,
    PrefixDecl, Program, Stmt, TypeExpr, UnOp,
};
pub use ast_query::{
    builtin_policies, check_custom_policies, check_policies, function_has_budget, hook_has_budget,
    parse_query, run_policies, Policy, PolicyViolation, QueryPattern,
};
pub use bind::{Host, MockHost};
pub use budget::Budget;
pub use cbor_ast::{decode, encode, DecodeError, TAG_VIBE_AST};
pub use check::{check_cell, check_program, CheckResult};
pub use dag::{
    ControlUnit, DagEdge, DagError, DagNode, DagPipeline, ExecutionState, JudgeClaim, JudgeFrame,
    NodeEffect, NodeStatus, RouterStrategy, MAX_DAG_EDGES, MAX_DAG_NODES, MAX_JUDGE_CLAIMS,
    MAX_NODE_IO,
};
pub use deontic_interrupt::{
    AgentSandbox, DeonticInterrupt, InterruptType, LeaseError, Phase, PhaseLeaser,
    MAX_CAPS_PER_PHASE, MAX_PHASES, MAX_SANDBOX_AGENTS,
};
pub use diagnose::{diagnose, DiagnoseReport};
pub use error::{DiagCode, Diagnostic};
pub use eval::{populate_import_aliases, Engine, Env};
pub use grammar::{DIAGNOSTIC_SCHEMA_JSON, EBNF, GBNF, SOURCE_SCHEMA_JSON};
pub use parse::{parse_cell, parse_program};
pub use projectional::{
    apply_edit, apply_edits, make_binary, make_call, make_field, make_float, make_ident, make_int,
    make_law, make_material, make_member, make_string, project_program, Edit, ProjectOptions,
};
pub use reflection::{
    ReflectionConfig, ReflectionEngine, ReflectionLoop, ReflectionResult, StageResult,
};
pub use span::Span;
pub use unicode_ident::{
    could_be_unicode_ident_start, is_bidi_control, is_xid_continue, is_xid_start,
    validate_identifier, IdentifierPolicyError, MAX_IDENT_LEN,
};
pub use value::{
    CausalRelation, ConservationQuantity, ConservationResult, DisclosureBoundary, Duration,
    EnumValue, FieldRef, Frame, Instant, MaterialRef, Miscibility, Mixture, MixturePhase, Pose,
    Quantity, QuinRef, SpeciesRef, TimeScale, Transform, Value, WorldLine,
};

/// Parse, check, and evaluate a Pure cell (`= expr`).
pub fn eval_cell<H: Host>(src: &str, host: &mut H, env: &mut Env) -> Result<Value, Diagnostic> {
    let expr = parse_cell(src)?;
    check_cell(&expr)?;
    let mut engine = Engine::new(host, Budget::default());
    engine.eval_expr(&expr, env)
}

/// Parse and check a module. Does not run hooks.
pub fn load_program(src: &str) -> Result<Program, Diagnostic> {
    let program = parse_program(src)?;
    check_program(&program)?;
    Ok(program)
}

/// Evaluate a named function from a checked program.
/// Import aliases from the program are populated into `env` before evaluation.
pub fn eval_function<H: Host>(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    host: &mut H,
    env: &mut Env,
) -> Result<Value, Diagnostic> {
    populate_import_aliases(env, &program.imports)?;
    let mut engine = Engine::with_program(host, Budget::default(), program);
    engine.call_function(program, name, args, env)
}

/// Dispatch a hook event on a checked program.
///
/// `path` is the event path (e.g. `["pulse", "message"]` for `on pulse:message(…)`
/// or `["tick"]` for `on tick(…)`). `args` are bound to the hook's parameters
/// in declaration order. Returns `Ok(Value::Null)` if no matching hook exists.
pub fn dispatch_hook<H: Host>(
    program: &Program,
    path: &[String],
    args: Vec<Value>,
    host: &mut H,
    env: &mut Env,
) -> Result<Value, Diagnostic> {
    populate_import_aliases(env, &program.imports)?;
    let mut engine = Engine::with_program(host, Budget::default(), program);
    engine.call_hook(program, path, args, env)
}
