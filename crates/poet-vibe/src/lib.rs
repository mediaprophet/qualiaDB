//! Poet engine — VibeScript 0.1.
//!
//! Copyright © 2026 Timothy Charles Holborn. All rights reserved.
//! Language version `vibe-0.1`. No JIT. No raw Quin overlay literals.

pub const LANGUAGE_VERSION: &str = "vibe-0.1";

mod ast;
mod bind;
mod budget;
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
mod value;

pub use ast::{Expr, Program};
pub use bind::{Host, MockHost};
pub use budget::Budget;
pub use check::{check_cell, check_program, CheckResult};
pub use diagnose::{diagnose, DiagnoseReport};
pub use error::{DiagCode, Diagnostic};
pub use eval::{populate_import_aliases, Engine, Env};
pub use grammar::{DIAGNOSTIC_SCHEMA_JSON, EBNF, GBNF, SOURCE_SCHEMA_JSON};
pub use parse::{parse_cell, parse_program};
pub use span::Span;
pub use value::Value;

/// Parse, check, and evaluate a Pure cell (`= expr`).
pub fn eval_cell<H: Host>(
    src: &str,
    host: &mut H,
    env: &mut Env,
) -> Result<Value, Diagnostic> {
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
    let mut engine = Engine::new(host, Budget::default());
    engine.call_function(program, name, args, env)
}
