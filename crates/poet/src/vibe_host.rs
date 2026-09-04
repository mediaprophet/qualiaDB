//! Thin `vibe-host-0.1` facade for Poet (G-A).
//!
//! Re-exports the frozen four-op surface + versions. No AST / Host trait
//! bleed into Poet UI — capability reach is invoke-only (default E300).

pub use vibe::{
    check_cell, check_program, diagnose, parse_cell, parse_program, DiagnoseReport,
    DiagCode, Diagnostic, LANGUAGE_VERSION, Span, Value,
};

/// Host ABI stamp for the frozen four-op surface.
pub const HOST_VERSION: &str = "vibe-host-0.1";

/// Crate / branch stamp aligned with `0.0.36-dev`.
pub const CRATE_STAMP: &str = "0.0.36-dev";

/// Pin: invoke by id only. Does not expose the wide [`vibe::Host`] trait.
/// Default fail-closed E300 — Poet daemon / qualia-core-db overrides in-process.
pub fn capability_invoke(
    id: &str,
    args: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let _ = args;
    Err(Diagnostic::new(
        DiagCode::E300,
        span,
        format!("capability.invoke not bound on this host: {id}"),
    ))
}

/// Diagnose JSON string with native shape (`valid`, `kind`, `error_code`, …).
pub fn diagnose_json(src: &str) -> String {
    diagnose(src).to_json()
}

pub fn host_version() -> &'static str {
    HOST_VERSION
}

pub fn language_version() -> &'static str {
    LANGUAGE_VERSION
}
