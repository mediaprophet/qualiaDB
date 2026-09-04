//! Thin `vibe-host-0.1` facade for Poet (G-A).
//!
//! Re-exports the frozen four-op surface + versions. No AST / Host trait
//! bleed into Poet UI — capability reach is invoke-only and is resolved
//! through Vibe's in-process catalog kernels.

pub use vibe::{
    check_cell, check_program, diagnose, parse_cell, parse_program, DiagCode, DiagnoseReport,
    Diagnostic, Span, Value, LANGUAGE_VERSION,
};

/// Host ABI stamp for the frozen four-op surface.
pub const HOST_VERSION: &str = "vibe-host-0.1";

/// Crate / branch stamp aligned with `0.0.36-dev`.
pub const CRATE_STAMP: &str = "0.0.36-dev";

/// Pin: invoke by id only. Does not expose the wide [`vibe::Host`] trait.
///
/// Catalog capabilities are executed by Vibe's bounded in-process kernels, so
/// Poet remains usable without a Webizen Studio or daemon dependency. Unknown
/// ids still fail closed with the catalog diagnostic.
pub fn capability_invoke(id: &str, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    vibe::catalog::invoke_local(id, args, span)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn catalog_capability_invocation_is_live_without_studio() {
        let mut args = BTreeMap::new();
        args.insert("family".into(), Value::String("spatial_kinematics".into()));
        args.insert("preset".into(), Value::String("orbit_spin".into()));
        args.insert("t".into(), Value::F64(0.5));
        let value = capability_invoke(
            "Animation.evaluate_preset",
            &Value::Record(args),
            Span::point(0),
        )
        .expect("catalog kernel should execute");
        assert!(matches!(value, Value::Record(_)));
    }

    #[test]
    fn unknown_capability_remains_an_explicit_error() {
        let error = capability_invoke("Unknown.capability", &Value::Null, Span::point(0))
            .expect_err("unknown ids must not silently succeed");
        assert_eq!(error.code, DiagCode::E100);
    }
}
