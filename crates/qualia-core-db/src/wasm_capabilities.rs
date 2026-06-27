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
];

pub const PORTAL: &[&str] = &[
    "nquin-48-byte-abi",
    "json-ingest",
    "cbor-ld-ingest",
    "tensor-10d",
    "spatial-encoding",
    "webgpu-viewport",
    "acoustic-plane",
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
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "economics",
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
    "bioinformatics",
    "clinical-risk",
    "organic-chemistry",
    "economics",
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
    "cbor-ld-ingest",
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
    "deontic-logic",
    "epistemic-logic",
    "paraconsistent-routing",
    "temporal-ltl",
    "description-logic",
    "answer-set-programming",
    "linear-logic",
    "interaction-governance",
    "lww-crdt",
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
    "wasm-playground-api",
];

/// Stable profile label for diagnostics and package manifests.
pub const fn compiled_profile() -> &'static str {
    if cfg!(feature = "wasm-full") {
        "full"
    } else if cfg!(feature = "wasm-playground") {
        "playground"
    } else if cfg!(feature = "wasm-ontology") {
        "ontology-mcp-kernel"
    } else if cfg!(feature = "wasm-llm") {
        "llm"
    } else if cfg!(feature = "wasm-logic") {
        "logic"
    } else if cfg!(feature = "wasm-scientific") {
        "scientific"
    } else if cfg!(feature = "portal") {
        "portal"
    } else {
        "core"
    }
}

/// Capabilities contributed by the selected top-level profile.
pub const fn compiled_capabilities() -> &'static [&'static str] {
    if cfg!(feature = "wasm-full") {
        FULL
    } else if cfg!(feature = "wasm-playground") {
        PLAYGROUND
    } else if cfg!(feature = "wasm-ontology") {
        ONTOLOGY_KERNEL
    } else if cfg!(feature = "wasm-llm") {
        LLM
    } else if cfg!(feature = "wasm-logic") {
        LOGIC
    } else if cfg!(feature = "wasm-scientific") {
        SCIENTIFIC
    } else if cfg!(feature = "portal") {
        PORTAL
    } else {
        &[]
    }
}
