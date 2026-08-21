//! Live Vibe coverage map.
//!
//! rustdoc (`all.html` / JSON) is the *discovery* index of types. Binding every
//! public item is the wrong job. The three catalogs that drive Vibe are:
//!
//! 1. [`crate::CAPABILITY_DESCRIPTORS`] — engine families / MCP
//! 2. [`WASM_ENGINE`] — `wasm_bridge/engine` (compute-engine.html, wasm-scientific)
//! 3. [`ids::ALL_BOUND`] — what `capability.invoke` actually reaches
//!
//! Scripts may run in a WASM UI; on desktop they call the **native** host
//! (`poet_eval`). The WASM *subset* is whatever this crate compiles under
//! `wasm-ontology` / `wasm-scientific`. Nightly rustdoc JSON is an optional
//! extra pass (`scripts/vibe-coverage.ps1`) — it never replaces these tables.

use super::ids;
use crate::CAPABILITY_DESCRIPTORS;
use poet_vibe::Value;
use std::collections::BTreeMap;

/// Where a candidate lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Descriptor,
    WasmEngine,
    Invoke,
}

/// Where it can execute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Native,
    WasmScientific,
    WasmOntology,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bind {
    Bound,
    Unbound,
}

#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub source: Source,
    pub id: &'static str,
    pub seam: &'static str,
    pub profile: Profile,
    pub bind: Bind,
}

/// Browser compute-engine exports (`wasm_bridge/engine/*`). Same math as native
/// solvers; GPU forge is native-only. Keep this list in lockstep with that folder.
pub const WASM_ENGINE: &[(&str, &str)] = &[
    ("cas_differentiate_wasm", "math"),
    ("cas_simplify_wasm", "math"),
    ("cas_expand_wasm", "math"),
    ("cas_evaluate_wasm", "math"),
    ("cas_factor_wasm", "math"),
    ("cas_solve_quadratic_wasm", "math"),
    ("la_matmul_wasm", "math"),
    ("la_transpose_wasm", "math"),
    ("la_determinant_wasm", "math"),
    ("la_solve_wasm", "math"),
    ("la_eigen_symmetric_wasm", "math"),
    ("la_eigenvalues_wasm", "math"),
    ("la_svd_wasm", "math"),
    ("la_polynomial_roots_wasm", "math"),
    ("stats_describe_wasm", "stats"),
    ("stats_correlation_wasm", "stats"),
    ("stats_linear_regression_wasm", "stats"),
    ("num_bessel_j_wasm", "math"),
    ("num_gcd_lcm_wasm", "math"),
    ("num_is_prime_wasm", "math"),
    ("crypto_sha256", "crypto"),
    ("crypto_sha512", "crypto"),
    ("crypto_blake3", "crypto"),
    ("units_convert", "math"),
    ("xform_dft", "math"),
    ("graph_shortest_path", "graph"),
    ("graph_spreading_activation", "graph"),
];

fn profile_name(p: Profile) -> &'static str {
    match p {
        Profile::Native => "native",
        Profile::WasmScientific => "wasm-scientific",
        Profile::WasmOntology => "wasm-ontology",
        Profile::Both => "native+wasm",
    }
}

fn source_name(s: Source) -> &'static str {
    match s {
        Source::Descriptor => "descriptor",
        Source::WasmEngine => "wasm-engine",
        Source::Invoke => "invoke",
    }
}

fn bind_name(b: Bind) -> &'static str {
    match b {
        Bind::Bound => "bound",
        Bind::Unbound => "unbound",
    }
}

fn wasm_suggested_invoke(export: &str) -> Option<&'static str> {
    match export {
        "la_matmul_wasm" => Some(ids::LINALG_MATMUL),
        "la_transpose_wasm" => Some(ids::LA_TRANSPOSE),
        "la_determinant_wasm" => Some(ids::LA_DET),
        "la_solve_wasm" => Some(ids::LA_SOLVE),
        "la_eigen_symmetric_wasm" => Some(ids::LA_EIGEN_SYM),
        "la_eigenvalues_wasm" => Some(ids::LA_EIGENVALUES),
        "la_svd_wasm" => Some(ids::LA_SVD),
        "la_polynomial_roots_wasm" => Some(ids::LA_POLY_ROOTS),
        "cas_differentiate_wasm" => Some(ids::CAS_DIFFERENTIATE),
        "cas_simplify_wasm" => Some(ids::CAS_SIMPLIFY),
        "cas_expand_wasm" => Some(ids::CAS_EXPAND),
        "cas_evaluate_wasm" => Some(ids::SYMBOLIC_EVAL),
        "cas_factor_wasm" => Some(ids::CAS_FACTOR),
        "cas_solve_quadratic_wasm" => Some(ids::CAS_SOLVE_QUADRATIC),
        "num_gcd_lcm_wasm" => Some(ids::NT_GCD),
        "num_is_prime_wasm" => Some(ids::NT_PRIME),
        "num_bessel_j_wasm" => Some(ids::SPEC_BESSEL),
        "stats_describe_wasm" => Some(ids::STAT_MEAN),
        "stats_correlation_wasm" => Some(ids::STAT_PEARSON),
        "stats_linear_regression_wasm" => Some(ids::STAT_LINEAR_REGRESSION),
        "crypto_sha256" => Some(ids::CRYPTO_SHA256),
        "crypto_sha512" => Some(ids::CRYPTO_SHA512),
        "crypto_blake3" => Some(ids::CRYPTO_BLAKE3),
        "units_convert" => Some(ids::UNITS_CONVERT),
        "xform_dft" => Some(ids::XFORM_DFT),
        "graph_shortest_path" => Some(ids::GRAPH_SHORTEST_PATH),
        "graph_spreading_activation" => Some(ids::GRAPH_SPREADING_ACTIVATION),
        _ => None,
    }
}

/// Full coverage rows. Cold path — `Vec` is allowed.
pub fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    for d in CAPABILITY_DESCRIPTORS {
        let bound = ids::family_bound(d.name);
        out.push(Row {
            source: Source::Descriptor,
            id: d.name,
            seam: d.domain,
            profile: if d.surfaces.iter().any(|s| s.contains("wasm")) {
                Profile::Both
            } else {
                Profile::Native
            },
            bind: if bound { Bind::Bound } else { Bind::Unbound },
        });
    }
    for (export, seam) in WASM_ENGINE {
        let bound = wasm_suggested_invoke(export).is_some();
        out.push(Row {
            source: Source::WasmEngine,
            id: export,
            seam,
            profile: Profile::WasmScientific,
            bind: if bound { Bind::Bound } else { Bind::Unbound },
        });
    }
    for id in ids::ALL_BOUND {
        out.push(Row {
            source: Source::Invoke,
            id,
            seam: ids::seam_for(id),
            profile: Profile::Both,
            bind: Bind::Bound,
        });
    }
    out
}

pub fn as_value() -> Value {
    let rs = rows();
    let bound = rs.iter().filter(|r| r.bind == Bind::Bound).count() as u64;
    let unbound = rs.iter().filter(|r| r.bind == Bind::Unbound).count() as u64;
    let list: Vec<Value> = rs
        .iter()
        .map(|r| {
            let mut m = BTreeMap::new();
            m.insert("source".into(), Value::String(source_name(r.source).into()));
            m.insert("id".into(), Value::String(r.id.into()));
            m.insert("seam".into(), Value::String(r.seam.into()));
            m.insert(
                "profile".into(),
                Value::String(profile_name(r.profile).into()),
            );
            m.insert("bind".into(), Value::String(bind_name(r.bind).into()));
            Value::Record(m)
        })
        .collect();
    let mut out = BTreeMap::new();
    out.insert("bound".into(), Value::U64(bound));
    out.insert("unbound".into(), Value::U64(unbound));
    out.insert("rows".into(), Value::List(list));
    out.insert(
        "note".into(),
        Value::String(
            "rustdoc lists types; bind operations. Desktop Vibe calls native. WASM subset = wasm-ontology/wasm-scientific cfgs."
                .into(),
        ),
    );
    Value::Record(out)
}

pub fn markdown() -> String {
    let rs = rows();
    let mut s = String::from(
        "# Vibe coverage matrix\n\nGenerated from `CAPABILITY_DESCRIPTORS` + `wasm_bridge/engine` + `ids::ALL_BOUND`.\n\n| source | id | seam | profile | bind |\n|---|---|---|---|---|\n",
    );
    for r in &rs {
        s.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            source_name(r.source),
            r.id,
            r.seam,
            profile_name(r.profile),
            bind_name(r.bind)
        ));
    }
    s
}

/// Machine-readable schema for a capability.invoke ID (T51).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineSchema {
    pub id: &'static str,
    pub family: &'static str,
    pub honesty: &'static str,
    pub effect_class: &'static str,
    pub arg_shape: &'static str,
    pub return_shape: &'static str,
}

pub const SCHEMAS: &[MachineSchema] = &[
    MachineSchema {
        id: ids::DAG_EXECUTE,
        family: "agent",
        honesty: "certified",
        effect_class: "Cold",
        arg_shape: r#"{"type":"list","items":{"type":"record"}}"#,
        return_shape: r#"{"type":"string"}"#,
    },
    MachineSchema {
        id: ids::DAG_VALIDATE,
        family: "agent",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"list","items":{"type":"record"}}"#,
        return_shape: r#"{"type":"bool"}"#,
    },
    MachineSchema {
        id: ids::DAG_STATUS,
        family: "agent",
        honesty: "stub",
        effect_class: "Pure",
        arg_shape: r#"{"type":"null"}"#,
        return_shape: r#"{"type":"null"}"#,
    },
    MachineSchema {
        id: ids::DISCOVERY_LIST,
        family: "runtime",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"null"}"#,
        return_shape: r#"{"type":"list","items":{"type":"string"}}"#,
    },
    MachineSchema {
        id: ids::SHACL_VALIDATE,
        family: "graph",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::GRAPH_STATS,
        family: "graph",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"null"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::GRAPH_SPARQL,
        family: "graph",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"string"}"#,
        return_shape: r#"{"type":"list"}"#,
    },
    MachineSchema {
        id: ids::GRAPH_SHORTEST_PATH,
        family: "graph",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"list"}"#,
    },
    MachineSchema {
        id: ids::LINALG_MATMUL,
        family: "math",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record","properties":{"a":{"type":"list"},"b":{"type":"list"}}}"#,
        return_shape: r#"{"type":"list"}"#,
    },
    MachineSchema {
        id: ids::NT_GCD,
        family: "math",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record","properties":{"a":{"type":"integer"},"b":{"type":"integer"}}}"#,
        return_shape: r#"{"type":"integer"}"#,
    },
    MachineSchema {
        id: ids::NT_PRIME,
        family: "math",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"integer"}"#,
        return_shape: r#"{"type":"bool"}"#,
    },
    MachineSchema {
        id: ids::CALC_SIMPSON,
        family: "math",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"number"}"#,
    },
    MachineSchema {
        id: ids::DEONTIC_EVAL,
        family: "logic",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"null"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::EPISTEMIC_EVAL,
        family: "logic",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"null"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::PHYS_PROJECTILE,
        family: "physics",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::PHYS_WAVE_1D,
        family: "physics",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"list"}"#,
    },
    MachineSchema {
        id: ids::PHYS_HARMONIC_OSCILLATOR,
        family: "physics",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::SPECTRAL_EMF_TO_RGB,
        family: "spectral",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::RENDER_SCENE,
        family: "render",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::RENDER_SVG_PATH,
        family: "render",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"string"}"#,
    },
    MachineSchema {
        id: ids::RENDER_CSS_COLOR,
        family: "render",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"string"}"#,
    },
    MachineSchema {
        id: ids::MANIFOLD_PROJECT,
        family: "manifold",
        honesty: "stub",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::NLP_GAZETTEER_RUN,
        family: "nlp",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"string"}"#,
        return_shape: r#"{"type":"list","items":{"type":"record"}}"#,
    },
    MachineSchema {
        id: ids::INFERENCE_EMBED,
        family: "inference",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"string"}"#,
        return_shape: r#"{"type":"list","items":{"type":"number"}}"#,
    },
    MachineSchema {
        id: ids::INFERENCE_GROUNDING,
        family: "inference",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record","properties":{"prompt":{"type":"string"},"text":{"type":"string"}}}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::FINANCE_CONVERT_CURRENCY,
        family: "econ",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record","properties":{"amount":{"type":"integer"},"rate_micros":{"type":"integer"}}}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::CAPABILITY_GRANT,
        family: "governance",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::SENTINEL_INSPECT,
        family: "governance",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record","properties":{"agent_did":{"type":"string"}}}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::AUDIO_SPECTRUM,
        family: "audio",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record"}"#,
        return_shape: r#"{"type":"record"}"#,
    },
    MachineSchema {
        id: ids::SCENE_CREATE,
        family: "render",
        honesty: "certified",
        effect_class: "Pure",
        arg_shape: r#"{"type":"record","properties":{"name":{"type":"string"}}}"#,
        return_shape: r#"{"type":"record"}"#,
    },
];

pub fn schema_for(id: &str) -> Option<&'static MachineSchema> {
    SCHEMAS.iter().find(|s| s.id == id)
}

pub fn all_schemas() -> &'static [MachineSchema] {
    SCHEMAS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_are_all_bound() {
        for r in rows() {
            if r.source == Source::Descriptor {
                assert_eq!(r.bind, Bind::Bound, "{}", r.id);
            }
        }
    }

    #[test]
    fn wasm_engine_fully_bound() {
        // Every wasm_bridge/engine export now has a capability.invoke wrapper.
        // If you add a new export to WASM_ENGINE, also add it to wasm_suggested_invoke.
        let unbound = rows()
            .into_iter()
            .filter(|r| r.source == Source::WasmEngine && r.bind == Bind::Unbound)
            .count();
        assert_eq!(unbound, 0, "every WASM_ENGINE export must have a capability.invoke id — add it to wasm_suggested_invoke");
    }

    #[test]
    fn coverage_value_has_counts() {
        match as_value() {
            Value::Record(m) => match m.get("bound") {
                Some(Value::U64(n)) => assert!(*n > 20),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn schema_for_known_id() {
        let s = schema_for(ids::DAG_EXECUTE).expect("DAG_EXECUTE must have a schema");
        assert_eq!(s.family, "agent");
        assert_eq!(s.effect_class, "Cold");
    }

    #[test]
    fn schema_for_unknown_id() {
        assert!(schema_for("nonexistent.foo").is_none());
    }

    #[test]
    fn all_schemas_nonempty() {
        assert!(!all_schemas().is_empty());
    }

    #[test]
    fn every_schema_has_family() {
        for s in all_schemas() {
            assert!(!s.family.is_empty(), "schema {} has empty family", s.id);
            assert!(
                !s.arg_shape.is_empty(),
                "schema {} has empty arg_shape",
                s.id
            );
            assert!(
                !s.return_shape.is_empty(),
                "schema {} has empty return_shape",
                s.id
            );
        }
    }
}
