//! Compile-time capability declarations for the supported WebAssembly profiles.
//!
//! This is the source of truth used by browser metadata and packaging docs. A
//! capability belongs here only when its profile is covered by a wasm32 build.

pub const ONTOLOGY_KERNEL: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "n3-parser",
    "shacl-property-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "yaml-ld-q42",
    "hcf-ingest",
];

pub const WEBCIVICS: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
    "yaml-ld-q42",
    "hcf-ingest",
    "cml-graph-model",
    "solid-protocol-records",
];

pub const PORTAL: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "json-ingest",
    "json-ld-ingest",
    "json-ld-serialize",
    "json-ld-context-digest",
    "rdfc-1.0-status",
    "vendor-nquin-cbor",
    "tensor-10d",
    "spatial-encoding",
    "webgpu-viewport",
    "acoustic-plane",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "shacl-graph-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
    "yaml-ld-q42",
    "hcf-ingest",
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "economics",
    "computational-geometry",
    "q42-kernel-sparql-extensions",
    "symbolic-logic",
    "numerical-solvers",
    "control-theory",
    "geometric-algebra",
    "quantum-dft",
];

pub const LOGIC: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
    "yaml-ld-q42",
    "hcf-ingest",
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "computational-geometry",
    "economics",
    "q42-kernel-sparql-extensions",
    "symbolic-logic",
    "numerical-solvers",
    "control-theory",
    "geometric-algebra",
    "quantum-dft",
];

pub const SCIENTIFIC: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
    "yaml-ld-q42",
    "hcf-ingest",
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "economics",
    "computational-geometry",
    "q42-kernel-sparql-extensions",
    "symbolic-logic",
    "numerical-solvers",
    "control-theory",
    "geometric-algebra",
    "quantum-dft",
];

pub const LLM: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
    "yaml-ld-q42",
    "hcf-ingest",
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "economics",
    "symbolic-logic",
    "numerical-solvers",
    "control-theory",
    "geometric-algebra",
    "quantum-dft",
    "gguf-parser",
    "q42-model-container",
    "webgpu-inference",
    "streaming-decode",
];

pub const PLAYGROUND: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "json-ld-ingest",
    "json-ld-serialize",
    "json-ld-context-digest",
    "rdfc-1.0-status",
    "vendor-nquin-cbor",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "shacl-graph-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
    "yaml-ld-q42",
    "hcf-ingest",
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "economics",
    "computational-geometry",
    "q42-kernel-sparql-extensions",
    "symbolic-logic",
    "numerical-solvers",
    "control-theory",
    "geometric-algebra",
    "quantum-dft",
    "wasm-playground-api",
];

pub const FULL: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "json-ingest",
    "json-ld-ingest",
    "json-ld-serialize",
    "json-ld-context-digest",
    "rdfc-1.0-status",
    "vendor-nquin-cbor",
    "tensor-10d",
    "spatial-encoding",
    "webgpu-viewport",
    "acoustic-plane",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "shacl-graph-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
    "yaml-ld-q42",
    "hcf-ingest",
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "economics",
    "computational-geometry",
    "q42-kernel-sparql-extensions",
    "symbolic-logic",
    "numerical-solvers",
    "control-theory",
    "geometric-algebra",
    "quantum-dft",
    "gguf-parser",
    "q42-model-container",
    "webgpu-inference",
    "streaming-decode",
    "wasm-playground-api",
];

/// WebCivics decision-evidence package (JSON-LD / SHACL / modal / SPARQL / Civics stats).
///
/// Deliberately omits WebGPU viewport, acoustic plane, tensor-10d demos, GGUF/LLM,
/// and heavy science (DFT/GA/full bio). Prefer this profile for Civics.au Node/browser
/// admission over `wasm-logic` (which currently pulls scientific → gpu-runtime).
pub const WEBCIVICS: &[&str] = &[
    "nquin-48-byte-abi",
    "q-hash",
    "json-ingest",
    "json-ld-ingest",
    "json-ld-serialize",
    "json-ld-context-digest",
    "rdfc-1.0-status",
    "vendor-nquin-cbor",
    "package-exposure-manifest",
    "n3-parser",
    "turtle-parser",
    "rdf-serialization",
    "solid-rdf-media-types",
    "solid-leave-migrate",
    "solid-qualia-return",
    "json-ld-compact-serialize",
    "ntriples-query",
    "query-compiler",
    "shacl-property-validation",
    "shacl-graph-validation",
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "values-personhood-guard",
    "lww-crdt",
    "economics",
    "q42-kernel-sparql-extensions",
    "symbolic-logic",
    "numerical-solvers",
    "device-storage-policy",
    "opfs-primary-vault",
    "backup-folder-link",
    "regression-verification",
    "multiple-ols",
    "jarque-bera",
    "breusch-pagan",
    "durbin-watson",
    "vif",
    "influence-diagnostics",
    "ramsey-reset",
    "chow-test",
    "logit-lda",
];

/// Capabilities that must **not** be advertised as browser-available (UE-035/044/045).
///
/// Listed so Civics / packaging never confuse absence with a silent stub success.
/// BFV HE stays native: unaudited upstream `fhe`, large binary, and local-first
/// key material — see `docs/manuals/privacy-engine.md` § Browser policy.
pub const NATIVE_ONLY: &[&str] = &[
    "solvers-qpu",
    "specialized-libs-qpu-bridge",
    "privacy-he-bfv",
    "privacy-dp-calibrated-releases", // available native; browser omits until policy revisit
    "cryptographic-library-full",
    "machine-learning-heavy",
    "engineering-cfd",
    "quantum-biology",
    "daemon-filesystem-volumes",
    "nvme-zns-csd",
    "ble-mesh",
    "ebpf-wfp-network-filter",
    "directml-vulkan-native-inference",
];

/// Stable profile label for diagnostics and package manifests.
pub const fn compiled_profile() -> &'static str {
    if cfg!(feature = "wasm-full") {
        "full"
    } else if cfg!(feature = "wasm-webcivics") {
        "webcivics"
    } else if cfg!(all(
        feature = "wasm-ontology",
        not(any(
            feature = "portal",
            feature = "wasm-webcivics",
            feature = "wasm-logic",
            feature = "wasm-scientific",
            feature = "wasm-llm",
            feature = "wasm-full",
            feature = "wasm-webcivics"
        ))
    )) {
        "ontology-mcp-kernel"
    } else if cfg!(feature = "portal") {
        "portal"
    } else if cfg!(feature = "wasm-llm") {
        "llm"
    } else if cfg!(feature = "wasm-playground") {
        "playground"
    } else if cfg!(feature = "wasm-webcivics") {
        "webcivics"
    } else if cfg!(feature = "wasm-logic") {
        "logic"
    } else if cfg!(feature = "wasm-scientific") {
        "scientific"
    } else {
        "core"
    }
}

/// Capabilities contributed by the selected top-level profile.
pub const fn compiled_capabilities() -> &'static [&'static str] {
    if cfg!(feature = "wasm-full") {
        FULL
    } else if cfg!(feature = "wasm-webcivics") {
        WEBCIVICS
    } else if cfg!(all(
        feature = "wasm-ontology",
        not(any(
            feature = "portal",
            feature = "wasm-webcivics",
            feature = "wasm-logic",
            feature = "wasm-scientific",
            feature = "wasm-llm",
            feature = "wasm-full",
            feature = "wasm-webcivics"
        ))
    )) {
        ONTOLOGY_KERNEL
    } else if cfg!(feature = "portal") {
        PORTAL
    } else if cfg!(feature = "wasm-llm") {
        LLM
    } else if cfg!(feature = "wasm-playground") {
        PLAYGROUND
    } else if cfg!(feature = "wasm-webcivics") {
        WEBCIVICS
    } else if cfg!(feature = "wasm-logic") {
        LOGIC
    } else if cfg!(feature = "wasm-scientific") {
        SCIENTIFIC
    } else {
        &[]
    }
}

/// Native-only register (never advertised as present in a WASM profile).
pub const fn native_only_capabilities() -> &'static [&'static str] {
    NATIVE_ONLY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_only_includes_qpu_and_bfv() {
        assert!(NATIVE_ONLY.contains(&"solvers-qpu"));
        assert!(NATIVE_ONLY.contains(&"privacy-he-bfv"));
        assert!(!compiled_capabilities().contains(&"privacy-he-bfv"));
    }

    #[test]
    fn webcivics_excludes_gpu_and_llm_capabilities() {
        for forbidden in [
            "webgpu-viewport",
            "webgpu-inference",
            "gguf-parser",
            "streaming-decode",
            "acoustic-plane",
            "tensor-10d",
            "quantum-dft",
        ] {
            assert!(
                !WEBCIVICS.contains(&forbidden),
                "{forbidden} must not appear in WEBCIVICS"
            );
        }
        assert!(WEBCIVICS.contains(&"json-ld-ingest"));
        assert!(WEBCIVICS.contains(&"shacl-graph-validation"));
        assert!(WEBCIVICS.contains(&"deontic-logic"));
        assert!(WEBCIVICS.contains(&"package-exposure-manifest"));
        assert!(WEBCIVICS.contains(&"solid-rdf-media-types"));
        assert!(WEBCIVICS.contains(&"solid-leave-migrate"));
        assert!(WEBCIVICS.contains(&"solid-qualia-return"));
        assert!(WEBCIVICS.contains(&"json-ld-compact-serialize"));
        assert!(WEBCIVICS.contains(&"rdf-serialization"));
        assert!(WEBCIVICS.contains(&"device-storage-policy"));
        assert!(WEBCIVICS.contains(&"opfs-primary-vault"));
        assert!(WEBCIVICS.contains(&"backup-folder-link"));
        assert!(WEBCIVICS.contains(&"regression-verification"));
        assert!(WEBCIVICS.contains(&"multiple-ols"));
        assert!(WEBCIVICS.contains(&"jarque-bera"));
        assert!(WEBCIVICS.contains(&"vif"));
    }

    #[test]
    fn native_only_disjoint_from_full_advertised() {
        for name in NATIVE_ONLY {
            assert!(
                !FULL.contains(name),
                "{name} must not appear in FULL advertised list"
            );
        }
    }
}
