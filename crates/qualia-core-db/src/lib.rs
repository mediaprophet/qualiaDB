use futures_core::Stream;
use std::fs::File;
use std::pin::Pin;
use std::task::{Context, Poll};
use zeroize::Zeroize;

#[cfg(all(
    target_arch = "wasm32",
    feature = "wasm-ontology",
    any(
        feature = "portal",
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-llm",
        feature = "wasm-playground",
        feature = "wasm-full"
    )
))]
compile_error!(
    "wasm-ontology is an exclusive lite profile; build heavier WASM products separately"
);

// --- services/ category (reorg) ---
pub mod services;
#[cfg(not(target_arch = "wasm32"))]
pub use services::chat_relay_daemon;
#[cfg(not(target_arch = "wasm32"))]
pub use services::daemon;
#[cfg(not(target_arch = "wasm32"))]
pub use services::daemon_graph;
#[cfg(not(target_arch = "wasm32"))]
pub use services::daemon_query;
#[cfg(not(target_arch = "wasm32"))]
pub use services::daemon_swarm;
#[cfg(not(target_arch = "wasm32"))]
pub use services::daemon_tensor;
#[cfg(not(target_arch = "wasm32"))]
pub use services::ilp_dispatcher;
#[cfg(not(target_arch = "wasm32"))]
pub use services::pulse_transport;
#[cfg(not(target_arch = "wasm32"))]
pub use services::rpc;
pub use services::solid_ldp;
#[cfg(not(target_arch = "wasm32"))]
pub use services::webizen_server;
#[cfg(not(target_arch = "wasm32"))]
pub use services::webtorrent_routes;
#[cfg(not(target_arch = "wasm32"))]
pub use services::webtorrent_seeder;
// --- medical/ category (reorg) ---
pub mod medical;
#[cfg(not(target_arch = "wasm32"))]
pub use medical::comorbidity_eval;
#[cfg(not(target_arch = "wasm32"))]
pub use medical::dicom;
#[cfg(not(target_arch = "wasm32"))]
pub use medical::dicom_ingest;
// --- query/ category (reorg) ---
pub mod query;
pub use query::cbor_compiler;
pub use query::graph_accel;
pub use query::graph_index;
#[cfg(not(target_arch = "wasm32"))]
pub use query::graph_proof;
pub use query::indexing;
#[cfg(not(target_arch = "wasm32"))]
pub use query::ingest;
#[cfg(not(target_arch = "wasm32"))]
pub use query::ingest_job;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use query::ingestion;
pub use query::lexicon;
pub use query::mini_parser;
#[cfg(not(target_arch = "wasm32"))]
pub use query::ontology_loader;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use query::query_compiler;
pub use query::query_engine;
pub use query::rdf_star;
pub use query::resolve;
pub use query::resolver;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-ontology",
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use query::shacl_compiler;
pub use query::spawn_decay;
pub use query::temporal_graph;
pub use query::temporal_scrub;
// --- platform/ category (reorg) ---
pub mod platform;
#[cfg(not(target_arch = "wasm32"))]
pub use platform::device_benchmark;
pub use platform::git_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub use platform::hardware_passport;
#[cfg(target_os = "android")]
pub use platform::jni_bridge;
pub use platform::kml_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub use platform::local_scheduler;
#[cfg(not(target_arch = "wasm32"))]
pub use platform::npu_ffi;
#[cfg(not(target_arch = "wasm32"))]
pub use platform::platform_scheduler;
pub use platform::tee_ffi;
// --- inference/ category (reorg) ---
pub mod inference;
// Inference-runtime components (honest names); `llm_*` retained as transitional aliases.
pub use inference::agent;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::ambient_orchestration;
pub use inference::application_profile;
pub use inference::application_profile::{
    active_application_profile, apply_application_profile, bootstrap_application_profile,
    set_application_profile, ApplicationProfile,
};
pub use inference::compute_universe;
#[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
pub use inference::cuda_lane::{
    cache_dense_weight, clear_weight_cache, dense_weight_cached, device_kv_ready,
    ensure_device_kv_cache, preload_q4k_soa_weights, q4k_device_weight_count, try_cuda_batch_gemv,
    try_cuda_batch_gemv_cached, try_cuda_batch_gemv_cached_only, try_cuda_mega_pass,
    try_q4k_soa_attention_device, try_q4k_soa_ffn_block, try_q4k_soa_ffn_block_residual,
    try_q4k_soa_fused_swiglu, try_q4k_soa_gemv, try_q4k_soa_qkv, warm_cuda_context,
    weight_cache_len, weight_fingerprint, MegaPassLayerDims, MegaPassLayerWeights, MAX_DENSE_ELEMS,
};
#[cfg(target_os = "windows")]
pub use inference::directml_bridge;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::ggml_quants;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::gguf_sharder;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::inference_agent;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::inference_agent as llm_agent;
pub use inference::inference_awq;
pub use inference::inference_awq as llm_awq;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::inference_bench;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::inference_bench as llm_bench;
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm"))]
pub use inference::inference_bench_wasm as llm_bench;
pub use inference::inference_eval;
pub use inference::inference_eval as llm_eval;
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub use inference::inference_gpu_profiler;
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub use inference::inference_gpu_profiler as llm_gpu_profiler;
pub use inference::inference_kernel_parity;
pub use inference::inference_kernel_parity as llm_kernel_parity;
pub use inference::inference_modes;
pub use inference::inference_modes::{
    active_inference_mode, apply_mode_toggles, bootstrap_inference_mode, fast_verify_html_default,
    post_turn_verify_enabled, prefer_tensor_core_gemm, quant_graph_grounding_enabled,
    rights_mode_enabled, sentinel_mid_decode_enabled, set_inference_mode, InferenceMode,
};
#[cfg(not(target_arch = "wasm32"))]
pub use inference::inference_path_selector;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::inference_path_selector::{
    apply_inference_path_plan, bootstrap_optimal_inference_path, format_path_plan,
    last_inference_path_plan, path_auto_enabled, resolve_inference_path_plan, run_path_select_cli,
    ComputeLane, InferencePathPlan, QuantProfile,
};
#[cfg(not(target_arch = "wasm32"))]
pub use inference::kv_capture;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::kv_dict;
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm"))]
pub use inference::kv_dict;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::kv_dict_runtime;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::lab;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::lab::{
    append_run_csv, audit_hot_path, calibrate_device_roof, default_search_space,
    format_lockin_summary, run_ablation_matrix, run_auto_improve, run_decode_timeline,
    run_q4k_soa_microbench, AblationRow, AutoImproveConfig, DecodeTimeline, DeviceRoof,
    ExperimentRun, HotPathAudit, LabConfig, LockInPackage, MicrobenchResult, TrialResult,
    CSV_HEADER,
};
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use inference::metal_bridge;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::neuro_symbolic_sieve;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::orchestrator;
pub use inference::post_turn_verify;
pub use inference::post_turn_verify::{
    maybe_verify_turn, return_html_as_text, verify_and_heal_turn, VerifiedTurn, VerifyCheck,
};
pub use inference::prompt_lookup;
pub use inference::qualia_hybrid;
pub use inference::qualia_hybrid::{
    apply_graph_logit_bias, force_fact_tokens, graph_force_enabled, prepare_hybrid_decode,
    propose_best_draft, propose_fact_draft, publish_graph_route_from_prompt,
    publish_grounding_obligation, publish_prompt_query_tensor, GRAPH_LOGIT_BIAS,
};
pub use inference::quant_graph_grounding;
pub use inference::quant_graph_grounding::{
    export_fact_quins, fact_count, ground_generation, load_facts_from_tsv, lookup_capital_object,
    maybe_ground_generation, register_capital_fact, register_fact, reset_fact_store_to_defaults,
    seed_facts_from_bundled, GroundingFact, GroundingResult, CTX_GROUNDING, P_CAPITAL_OF,
};
#[cfg(not(target_arch = "wasm32"))]
pub use inference::residency_planner;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::resident_model;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::safetensor;
pub use inference::sampler;
pub use inference::semantic_culler;
pub use inference::spatial_sieve;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::tensor_roles;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::ternary;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::ternary_gpu;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference::topk;
// W7: GPU thermal/power telemetry + auto-cap governor (native-only). Exposes the UI-reachable mode
// switch (`set_gpu_auto_cap` / `gpu_auto_cap_enabled`) and `sample_gpu_thermal()` telemetry.
#[cfg(not(target_arch = "wasm32"))]
pub use inference::thermal_telemetry;
#[cfg(not(target_arch = "wasm32"))]
pub use inference::topk_gpu;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod lora;
// --- q42/ category (reorg) ---
pub mod q42;
pub use q42::design_encode;
pub use q42::execution_profile;
pub use q42::machine_gpu_profile;
pub use q42::model_helper;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use q42::p64_weight;
/// Backward-compatible module name retained for existing inference and
/// transcode harnesses while the on-disk magic/API is P64.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use q42::p64_weight as q42_weight;
#[cfg(not(target_arch = "wasm32"))]
pub use q42::q42_lexicon;
#[cfg(not(target_arch = "wasm32"))]
pub use q42::q42_reader;
#[cfg(not(target_arch = "wasm32"))]
pub use q42::q42_volume;
pub use q42::yaml_ld_q42;
// --- extensions/ category (reorg) ---
pub mod extensions;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use extensions::extension_bus;
pub use extensions::extension_manifest;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use extensions::resource_catalog;
#[cfg(all(
    target_arch = "wasm32",
    feature = "wasm-ontology",
    not(any(
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full"
    ))
))]
#[path = "modalities_lite/mod.rs"]
pub mod modalities;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod modalities;
// --- identity/ category (reorg) ---
pub mod identity;
pub use identity::agency;
pub use identity::identifier;
#[cfg(not(target_arch = "wasm32"))]
pub use identity::key_vault;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use identity::profiles;
pub use identity::vault_manifest;
pub use identity::webizen_identifiers;
pub mod gpu_context;
pub mod shaders;
#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
pub mod wgsl_forge;
// --- foundation/ category (reorg) ---
pub mod foundation;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use foundation::crdt;
pub use foundation::frame_layout;
pub use foundation::fuzz_testing;
pub use foundation::telemetry;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use foundation::topology_draft;
// --- net/ category (reorg) ---
pub mod net;
#[cfg(not(target_arch = "wasm32"))]
pub use net::acoustic_ble_mesh;
#[cfg(not(target_arch = "wasm32"))]
pub use net::ebpf_filter;
#[cfg(not(target_arch = "wasm32"))]
pub use net::ebpf_firewall;
#[cfg(not(target_arch = "wasm32"))]
pub use net::host_topology;
#[cfg(not(target_arch = "wasm32"))]
pub use net::nym_adapter;
pub use net::sonic_token;
pub mod audio;
/// `.hmc` — a transparent container-of-files bundle for shipping a set of
/// sealed assets (`.10d` / `.q42` / `.p64`) as one attestable unit. Available to
/// both native and WASM builds (native adds the zero-copy `BundleMmap`).
pub mod bundle;
/// `.10d` living-container v1 — normative header, axis-role taxonomy, and
/// metric-completeness descriptor for the 10-D tensor substrate. P0.1 barrier
/// task. Available to browser/WASM builds (P0.8 parity target). See
/// `docs/plans/native-computational-geometry-EXECUTION.md` P0.1.
pub mod container_10d;
pub mod tensor;
// geometric_algebra moved into solvers/ (it is a math solver, not a logic modality);
// re-exported here so `crate::geometric_algebra::*` paths keep resolving. Gated to match the
// `solvers` module below — on wasm32 it only exists under the `wasm-scientific` feature.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use crate::solvers::geometric_algebra;
// --- governance/ category (reorg) ---
pub mod governance;
pub use governance::illocution;
pub use governance::modal_kind;
pub use governance::provenance;
#[cfg(not(target_arch = "wasm32"))]
pub use governance::web_civics;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use governance::webizen;
#[cfg(any(
    not(target_arch = "wasm32"),
    any(feature = "wasm-scientific", feature = "wasm-logic")
))]
pub use governance::webizen_bytecode;
#[cfg(not(target_arch = "wasm32"))]
pub use governance::webizen_sync;
pub use governance::webizen_validator;
pub mod sparql_library;
pub use sparql_library::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod q42_lex;
#[cfg(target_arch = "wasm32")]
pub mod q42_lex {
    pub struct Q42LexMmap<'a> {
        _marker: std::marker::PhantomData<&'a ()>,
    }
    impl<'a> Q42LexMmap<'a> {
        pub fn from_bytes(_data: &'a [u8]) -> Result<Self, String> {
            Err("Not supported on WASM".to_string())
        }
        pub fn lookup_embedded_triple(&self, _id: u64) -> Option<[u64; 3]> {
            None
        }
        pub fn lookup_hash(&self, _id: u64) -> Option<&'a str> {
            None
        }
    }
    pub struct Q42LexFile {}
    impl Q42LexFile {
        pub fn open(_p: &std::path::Path) -> Result<Self, String> {
            Err("Not supported on WASM".to_string())
        }
        pub fn view(&self) -> Q42LexMmap<'_> {
            Q42LexMmap {
                _marker: std::marker::PhantomData,
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub mod clinical_engine;
#[cfg(target_arch = "wasm32")]
pub mod clinical_engine {
    pub struct GeneExpressionResult {
        pub fold_change: f64,
        pub log2_fold_change: f64,
        pub is_significant: bool,
    }
    pub fn evaluate_gene_expression(
        _gene_id: u64,
        _baseline: f64,
        _treatment: f64,
        _fc_threshold: f64,
    ) -> GeneExpressionResult {
        GeneExpressionResult {
            fold_change: 0.0,
            log2_fold_change: 0.0,
            is_significant: false,
        }
    }
}
/// Hypermedia semantic library — asset ⊕ analytics ⊕ related-assets bound as a semantic graph (not a
/// directory). See `docs/plans/hypermedia-semantic-library.md`.
pub mod agent_runtime;
/// Entity-view kernel: entity id, observer status, rights filter, attribution, packages (shared by whole desktop; not "mindware-only").
pub mod entity_view;
pub mod hypermedia;
/// N9: Hypermedia asset authoring — image, video, 3D, interactive, portals, DMX.
pub mod hypermedia_authoring;
/// Document NLP (tokenize, gazetteer, span plans). Engine capability, not Vibe.
pub mod nlp;
pub mod poet_host;
pub mod qubo_compiler;
pub mod render;
/// N8: Research / investigation / epistemics — enquiry, corpus, dark links, inference chains,
/// investigations, hypothesis graphs, epistemic assessment, sentiment analysis.
pub mod research;
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod spatial_wasm;
pub mod text_span;
#[cfg(all(
    target_arch = "wasm32",
    any(
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full",
        feature = "wasm-playground"
    )
))]
pub mod wasm_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_bridge;
#[cfg(all(
    target_arch = "wasm32",
    feature = "portal",
    not(any(
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full",
        feature = "wasm-playground"
    ))
))]
pub mod wasm_bridge_core;
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm"))]
pub mod wasm_llm;
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub use render::portal::QualiaPortal;
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub use render::portal_wasm::{create_canvas, init_panic_hook, WebEngine};
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub use spatial_wasm::{
    design_encode_wasm, export_tensor_buffer_wasm, export_tensor_slice_wasm,
    geosparql_operation_wasm, sample_browser_telemetry_wasm, spatial_encode_wasm,
};
#[cfg(all(
    target_arch = "wasm32",
    any(
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full",
        feature = "wasm-playground"
    )
))]
pub use wasm_bridge::{parse_cbor_ld_wasm, parse_json_wasm, parse_n3logic_wasm, parse_turtle_wasm};
#[cfg(all(
    target_arch = "wasm32",
    feature = "portal",
    not(any(
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full",
        feature = "wasm-playground"
    ))
))]
pub use wasm_bridge_core::{parse_cbor_ld_wasm, parse_json_wasm};
pub mod storage_driver;
#[cfg(all(
    target_arch = "wasm32",
    any(feature = "wasm-playground", feature = "wasm-full")
))]
pub mod wasm_playground;
#[cfg(not(target_arch = "wasm32"))]
pub mod zns_storage;
// --- crypto/ category (reorg) ---
pub mod crypto;
#[cfg(feature = "zk-culling")]
pub use crypto::deontic_circuit;
pub use crypto::fiduciary_crypto;
#[cfg(feature = "pq-kem")]
pub use crypto::pq_kem_shim;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "sanctuary-crypto")]
pub use crypto::sanctuary_crypto;
pub use crypto::verifiable_credential;
pub use crypto::zk_proofs;
#[cfg(not(target_arch = "wasm32"))]
pub mod csd_storage;
// pub mod clinical_engine; // Temporarily disabled
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod specialized_libs;

// pub use specialized_libs::linear_algebra;
// pub use specialized_libs::statistical_computing;
// // pub use specialized_libs::cryptographic_library;
// pub use specialized_libs::physics_simulation;
// pub use specialized_libs::machine_learning;
// pub use specialized_libs::financial_modeling;
// pub use specialized_libs::chemistry_modeling;
// pub use specialized_libs::medical_computing; // Temporarily disabled
// pub use specialized_libs::engineering_analysis;

pub mod wasm_capabilities;

/// The Global Capability Registry exposes which features are compiled into the
/// current qualia-core-db binary. This allows the CLI to dynamically self-document
/// and progressively expose features like SHACL extensions or specific logic modalities.
/// Crate semver baked in at compile time — shared by daemon `/health`, CLI, and WASM `get_engine_version()`.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Canonical description of an inference capability and the MCP tools that
/// execute it. Chat prompting and MCP registration both consume this table, so
/// STEM solvers are exposed as tools rather than reimplemented in the LLM layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub name: &'static str,
    pub domain: &'static str,
    /// Concrete operations implemented by this capability family. This is
    /// discovery metadata, not a promise that every operation has an MCP route.
    pub operations: &'static [&'static str],
    pub mcp_tools: &'static [&'static str],
    /// `stable`, `partial`, `experimental`, or `fail-closed`.
    pub maturity: &'static str,
    /// Surfaces that can currently execute the capability.
    pub surfaces: &'static [&'static str],
}

pub const CAPABILITY_DESCRIPTORS: &[CapabilityDescriptor] = &[
    CapabilityDescriptor {
        name: "CapabilityDiscovery",
        domain: "runtime",
        operations: &[
            "list capability families",
            "inspect operations",
            "inspect maturity",
            "inspect runtime surfaces",
        ],
        mcp_tools: &["list_capabilities"],
        maturity: "stable",
        surfaces: &["native", "wasm", "mcp", "cli", "chat"],
    },
    CapabilityDescriptor {
        name: "SHACL",
        domain: "ontology",
        operations: &[
            "validate shapes",
            "route decentralized shapes",
            "credential gate",
            "degrade violations",
        ],
        mcp_tools: &["validate_shacl", "shacl_route"],
        maturity: "stable",
        surfaces: &["native", "wasm-logic", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "GraphDatabase",
        domain: "ontology",
        operations: &[
            "graph query",
            "SPARQL query",
            "identifier resolution",
            "ontology ingest",
            "RDF parse and serialize",
        ],
        mcp_tools: &["query_graph", "query_sparql", "graph_resolve"],
        maturity: "stable",
        surfaces: &["native", "wasm-ontology", "mcp"],
    },
    CapabilityDescriptor {
        name: "DeonticLogic",
        domain: "logic",
        operations: &[
            "obligation",
            "permission",
            "prohibition",
            "defeaters",
            "expiry",
            "jural correlation",
            "policy governance",
        ],
        mcp_tools: &["evaluate_modality", "deontic_govern", "jural_correlate"],
        maturity: "stable",
        surfaces: &["native", "wasm-logic", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "EpistemicLogic",
        domain: "logic",
        operations: &[
            "knowledge",
            "belief",
            "common knowledge",
            "certainty",
            "possible-world filtering",
        ],
        mcp_tools: &["evaluate_modality"],
        maturity: "stable",
        surfaces: &["native", "wasm-logic", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "ParaconsistentLogic",
        domain: "logic",
        operations: &[
            "contradiction detection",
            "context isolation",
            "non-explosive merge",
        ],
        mcp_tools: &["evaluate_modality"],
        maturity: "stable",
        surfaces: &["native", "wasm-logic", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "TemporalAndDescriptionLogic",
        domain: "logic",
        operations: &[
            "LTL globally/finally/next/until/release",
            "CTL path operators",
            "description-logic subsumption",
            "interval reasoning",
        ],
        mcp_tools: &["evaluate_modality"],
        maturity: "partial",
        surfaces: &["native", "wasm-logic", "mcp"],
    },
    CapabilityDescriptor {
        name: "SymbolicAndDefeasibleLogic",
        domain: "logic",
        operations: &[
            "bounded SAT",
            "defeasible forward chaining",
            "ASP stable models",
            "argumentation semantics",
            "abductive reasoning",
        ],
        mcp_tools: &["symbolic_logic_infer", "evaluate_modality"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "SymbolicAlgebra",
        domain: "mathematics",
        operations: &[
            "parse/evaluate expressions",
            "simplify",
            "expand",
            "differentiate",
            "symbolic integration",
            "limits",
            "assumptions",
            "trigonometric simplification",
            "Taylor series",
            "polynomial roots",
            "symbolic ODE/PDE",
        ],
        mcp_tools: &["cas", "algebra_solve_polynomial"],
        maturity: "partial",
        surfaces: &["native", "mcp"],
    },
    CapabilityDescriptor {
        name: "LinearAlgebra",
        domain: "mathematics",
        operations: &[
            "matrix multiply",
            "transpose",
            "inverse",
            "linear solve",
            "determinant",
            "LU",
            "QR",
            "Cholesky",
            "SVD",
            "general eigenvalues",
            "symmetric eigensystem",
            "tensor contraction",
        ],
        mcp_tools: &["matrix_operation", "algebra_matrix_analyze"],
        maturity: "stable",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "NumericalCalculus",
        domain: "mathematics",
        operations: &[
            "RK4",
            "BDF1/BDF2",
            "Verlet/Ruth/Yoshida symplectic integration",
            "shooting BVP",
            "Simpson/trapezoid integration",
            "sensitivity analysis",
            "interpolation",
            "splines",
            "least-squares polynomial fit",
        ],
        mcp_tools: &[],
        maturity: "stable",
        surfaces: &["native", "library", "webizen"],
    },
    CapabilityDescriptor {
        name: "Optimization",
        domain: "mathematics",
        operations: &[
            "Nelder-Mead",
            "bounded Newton-Raphson",
            "Levenberg-Marquardt",
            "hill climbing",
            "simulated annealing",
            "artificial bee colony",
        ],
        mcp_tools: &[],
        maturity: "stable",
        surfaces: &["native", "library"],
    },
    CapabilityDescriptor {
        name: "GeometricAlgebra",
        domain: "mathematics",
        operations: &[
            "dot/cross/angle",
            "geometric product",
            "outer product",
            "rotors",
            "translators",
            "multivectors",
            "SIMD kernels",
        ],
        mcp_tools: &["geometric_algebra_op"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "ComputationalGeometry",
        domain: "geometry",
        operations: &[
            "robust geometric predicates",
            "convex hulls",
            "half-edge topology graphs",
            "10D geometry feature encoding",
            ".10d quantized mesh geometry",
            "Delaunay triangulation",
            "Voronoi diagrams",
            "nearest-site query",
            "primitive generation (box, sphere, cylinder, plane)",
            "T·R·S transform composition",
            "scene graph assembly",
            ".10d asset export with μ provenance",
            "VR filtration + persistent homology",
            "CkNN graph Laplacian",
            "natural-neighbour interpolation",
        ],
        mcp_tools: &["computational_geometry", "geometry_manifests"],
        maturity: "partial",
        surfaces: &["native", "wasm-scientific", "mcp", "webizen", "renderer"],
    },
    CapabilityDescriptor {
        name: "ComputerVision",
        domain: "vision",
        operations: &[
            "classical CV (blur, edges, features, flow, morph)",
            "classical + tiled super-resolution (nearest/bilinear/bicubic/lanczos)",
            "Forge shared_gpu nearest + bicubic resize when Cool",
            "bio histopathology / radiomics / DICOM-lite",
            "MeshIR export / quality cleanup / class→σ map",
            "local CBIR proxy embeddings (aHash/dHash/hist)",
        ],
        mcp_tools: &["computer_vision"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen", "qualia-vision"],
    },
    CapabilityDescriptor {
        name: "NumberTheory",
        domain: "mathematics",
        operations: &[
            "primality",
            "factorization",
            "divisors",
            "GCD/LCM",
            "modular powers/inverses",
            "Chinese remainder theorem",
            "totient",
            "Mobius",
            "partitions",
            "Stirling numbers",
            "Catalan numbers",
        ],
        mcp_tools: &[],
        maturity: "stable",
        surfaces: &["native", "library"],
    },
    CapabilityDescriptor {
        name: "SpecialFunctionsAndTransforms",
        domain: "mathematics",
        operations: &[
            "Bessel",
            "Airy",
            "zeta",
            "Legendre",
            "Chebyshev",
            "Hermite",
            "Laguerre",
            "DFT/IDFT",
            "Laplace transform",
            "Z-transform",
            "unit conversion",
            "dimensional analysis",
            "vector calculus",
        ],
        mcp_tools: &[],
        maturity: "stable",
        surfaces: &["native", "library"],
    },
    CapabilityDescriptor {
        name: "Statistics",
        domain: "statistics",
        operations: &[
            "descriptive statistics",
            "robust statistics",
            "Pearson/Spearman/Kendall correlation",
            "histograms",
            "linear regression",
            "normal/t/chi-square/F distributions",
            "t-tests",
            "ANOVA",
            "chi-square tests",
            "non-parametric tests",
            "multiple testing",
            "anomaly detection",
            "entropy/KL/mutual information",
        ],
        mcp_tools: &["statistical_analysis"],
        maturity: "partial",
        surfaces: &["native", "mcp", "library"],
    },
    CapabilityDescriptor {
        name: "MachineLearning",
        domain: "machine-learning",
        // NOTE: these classical-ML operations are implemented in the
        // `solvers::learning` tree (60+ files), NOT in
        // `specialized_libs::machine_learning`, which the `ml_inference`
        // tool below routes to (that file serves the GGUF-load + MLP-forward
        // inference path, plus int8 quant/prune/distill). Per the field doc
        // above, listing an operation here does not imply an MCP route to it.
        operations: &[
            "linear/ridge/lasso/Bayesian regression",
            "KNN/naive Bayes/SVM/discriminant classification",
            "k-means/GMM/hierarchical clustering",
            "decision trees/random forests/boosting/BART",
            "PCA/SOM",
            "Gaussian processes",
            "HMM/Kalman",
            "survival analysis",
            "MCMC/variational inference",
            "resampling",
            "active learning",
            "bandits",
            "knowledge-graph embeddings",
        ],
        mcp_tools: &["ml_inference"],
        maturity: "fail-closed",
        surfaces: &["native", "library"],
    },
    CapabilityDescriptor {
        name: "PhysicsAndODE",
        domain: "physics",
        operations: &[
            "thermodynamics",
            "CFD",
            "molecular dynamics",
            "RK4 ODE",
            "Thomas-Fermi DFT",
            "PINN binding affinity",
            "distributed simulation",
        ],
        mcp_tools: &["ode_solve", "qpu_dft"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "Bioinformatics",
        domain: "bioscience",
        operations: &[
            "Smith-Waterman",
            "Needleman-Wunsch",
            "DNA/protein alignment",
            "k-mer frequencies",
            "MinHash/Jaccard",
            "UPGMA",
            "FASTA validation",
            "DNA translation",
            "isoelectric point",
            "peptide cleavage",
            "fingerprint similarity",
        ],
        mcp_tools: &["bioinformatics_align"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "OrganicChemistry",
        domain: "chemistry",
        operations: &[
            "SMILES/InChI validation",
            "formula and molecular weight",
            "LogP/TPSA",
            "Lipinski/Veber/Ghose/Egan",
            "functional groups",
            "pKa",
            "chiral centers",
            "circular fingerprints",
            "reaction kinetics",
            "equilibrium",
            "green metrics",
            "BBB permeation",
            "isotope distributions",
        ],
        mcp_tools: &["chemical_analysis", "chemical_descriptors"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "ClinicalRisk",
        domain: "clinical",
        operations: &[
            "Framingham",
            "CHA2DS2-VASc",
            "SCORE2",
            "SOFA",
            "eGFR/creatinine clearance",
            "drug interactions",
            "contraindications",
            "FHIR validation",
            "longitudinal trends",
            "gene expression",
            "one-compartment pharmacokinetics",
        ],
        mcp_tools: &["medical_score", "clinical_risk"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "FinancialModeling",
        domain: "economics",
        operations: &[
            "Black-Scholes option pricing and Greeks",
            "portfolio VaR",
            "Sharpe/Sortino",
            "maximum drawdown",
            "Monte Carlo risk",
        ],
        mcp_tools: &["financial_model"],
        maturity: "partial",
        surfaces: &["native", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "EngineeringAnalysis",
        domain: "engineering",
        operations: &[
            "structural analysis",
            "thermal conduction",
            "linear dynamics",
            "stress/displacement",
            "fatigue",
            "fluid analysis",
        ],
        mcp_tools: &["engineering_analysis_op"],
        maturity: "partial",
        surfaces: &["native", "mcp"],
    },
    CapabilityDescriptor {
        name: "CausalFuzzyAndControl",
        domain: "reasoning",
        operations: &[
            "causal reachability",
            "but-for and overdetermined cause",
            "do-intervention",
            "counterfactuals",
            "fuzzy t-norms/conorms",
            "Mamdani/Sugeno inference",
            "type-2 fuzzy sets",
            "PID/control feedback",
        ],
        mcp_tools: &[],
        maturity: "stable",
        surfaces: &["native", "library", "webizen"],
    },
    CapabilityDescriptor {
        name: "ContractsIdentityAndConsensus",
        domain: "governance",
        operations: &[
            "contract formation",
            "capacity",
            "delegation/revocation",
            "responsibility",
            "jural chains",
            "value flow",
            "identity fabric",
            "BFT quorum",
            "Lamport/vector clocks",
            "legal composition",
        ],
        mcp_tools: &[
            "values_check",
            "values_evaluate",
            "jural_correlate",
            "deontic_govern",
            "mcp_cooperate",
        ],
        maturity: "partial",
        surfaces: &["native", "wasm-logic", "mcp", "webizen"],
    },
    CapabilityDescriptor {
        name: "QuantumAndCryptographic",
        domain: "quantum-security",
        operations: &[
            "QUBO formulation",
            "classical pre-solve",
            "QAOA/SPSA",
            "QPU job dispatch",
            "DFT bridge",
            "sign/encrypt/verify",
            "ML-DSA credentials",
            "zero-knowledge proof plumbing",
            "quantum biology orchestration",
        ],
        mcp_tools: &["qpu_optimize", "qpu_dft", "qpu_status"],
        maturity: "experimental",
        surfaces: &["native", "mcp", "webizen"],
    },
];

/// Bare-metal 40-byte continuous statement container for the Qualia engine.
/// Fully optimized for zero-copy memory operations on post-2020 architectures.
#[repr(C, align(16))]
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Zeroize,
    bytemuck::Pod,
    bytemuck::Zeroable,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct NQuin {
    /// Subject identifier code reference index
    pub subject: u64,
    /// Predicate relation code reference index
    pub predicate: u64,
    /// Object value or entity code reference index
    pub object: u64,
    /// Graph Context identifier code reference index
    pub context: u64,
    /// The Fifth Vector: Metadata, Policy bitmasks, and geometric traits
    pub metadata: u64,
    /// The Sixth Vector: ECC Parity and Checksum bits (making the Quin 48 bytes)
    pub parity: u64,
}

impl NQuin {
    const NESTED_BIT_MASK: u64 = 1 << 63;

    #[inline(always)]
    pub fn is_subject_nested(&self) -> bool {
        (self.subject & Self::NESTED_BIT_MASK) != 0
    }

    #[inline(always)]
    pub fn get_subject_literal_id(&self) -> u64 {
        self.subject & !Self::NESTED_BIT_MASK
    }

    // ── Metadata field bit-layout (v3, post §4.1 migration) ─────────────────
    //
    // [63:60]  Quin Type  — 4-bit nibble; bits [62:61] are the routing lane
    // [59:56]  Sensitivity tier — 4-bit ODRL layer (0=PUBLIC..4=FIDUCIARY)
    // [55:32]  Reserved — 24 bits for Phase 2 temporal/spatial flags
    // [31:0]   Lamport clock — 32-bit logical clock for CRDT ordering
    //
    // LoRA context triggers have been moved to LoRAAdapterManager's side-table.

    // Routing lane reads bits [62:61] — sub-bits of the Quin Type nibble.
    const LANE_MASK: u64 = 0b11 << 61;
    const SHIFT_LANE: u32 = 61;

    #[inline(always)]
    pub fn identify_routing_lane(&self) -> PermissiveRoutingLane {
        // Since it's packed, taking reference to field might be unsafe in some contexts,
        // but passing self by reference and copying the field is usually fine,
        // though `self.metadata` directly copies if it's Copy.
        // Let's use `let metadata = { self.metadata };` to safely copy if needed,
        // but `self.metadata` usually works if we don't take a reference to it.
        let metadata = self.metadata;
        let lane_bits = (metadata & Self::LANE_MASK) >> Self::SHIFT_LANE;
        match lane_bits {
            0x01 => PermissiveRoutingLane::EnforcePermissiveCommons,
            0x02 => PermissiveRoutingLane::EnforceBilateralMicroCommons,
            0x03 => PermissiveRoutingLane::SpatiotemporalAmbiguous,
            _ => PermissiveRoutingLane::PassthroughStandard,
        }
    }

    pub const SENSITIVITY_PUBLIC: u8 = 0x00;
    pub const SENSITIVITY_RESTRICTED: u8 = 0x01;
    pub const SENSITIVITY_CLASSIFIED: u8 = 0x02;

    #[inline(always)]
    pub fn get_sensitivity_byte(&self) -> u8 {
        (self.context >> 56) as u8
    }

    #[inline(always)]
    pub fn set_sensitivity_byte(&mut self, sensitivity: u8) {
        // Clear top 8 bits
        self.context &= 0x00FF_FFFF_FFFF_FFFF;
        // Set new sensitivity
        self.context |= (sensitivity as u64) << 56;
    }

    // ── Quin Type (bits [63:60], via the FrameLayout ABI) ────────────────────
    // Canonical home: `frame_layout::{quin_type, with_quin_type}`. Bits [62:61] of
    // this nibble double as the permissive-routing lane on routed quins (an
    // intentional, role-exclusive overlay documented in frame_layout). It is NOT
    // relocated lower — every lower slot lands inside the tensor-bake clock [32:60].
    #[inline(always)]
    pub fn get_quin_type(&self) -> u8 {
        crate::frame_layout::quin_type(self.metadata)
    }

    /// Write the 4-bit Quin Type nibble into bits [63:60], preserving all other bits.
    #[inline(always)]
    pub fn set_quin_type(&mut self, quin_type: u8) {
        self.metadata = crate::frame_layout::with_quin_type(self.metadata, quin_type);
    }

    // ── Sensitivity tier (bits [59:56]) — ODRL layer ─────────────────────────

    pub const SENSITIVITY_TIER_PUBLIC: u8 = 0x00;
    pub const SENSITIVITY_TIER_PROFESSIONAL: u8 = 0x01;
    pub const SENSITIVITY_TIER_LEGAL: u8 = 0x02;
    pub const SENSITIVITY_TIER_MEDICAL: u8 = 0x03;
    pub const SENSITIVITY_TIER_FIDUCIARY: u8 = 0x04;

    /// Read the 4-bit ODRL sensitivity tier from bits [59:56].
    #[inline(always)]
    pub fn get_sensitivity_tier(&self) -> u8 {
        ((self.metadata >> 56) & 0xF) as u8
    }

    /// Write the 4-bit ODRL sensitivity tier into bits [59:56], preserving all other bits.
    #[inline(always)]
    pub fn set_sensitivity_tier(&mut self, tier: u8) {
        self.metadata = (self.metadata & !(0xFu64 << 56)) | ((tier as u64 & 0xF) << 56);
    }

    // ── Lamport clock (bits [31:0]) ───────────────────────────────────────────

    /// Extracts the 32-bit Lamport logical clock from bits [31:0].
    #[inline(always)]
    pub fn extract_lamport_clock(&self) -> u32 {
        (self.metadata & 0xFFFF_FFFF) as u32
    }

    /// Sets the 32-bit Lamport logical clock in bits [31:0], preserving all upper bits.
    #[inline(always)]
    pub fn set_lamport_clock(&mut self, clock: u32) {
        self.metadata = (self.metadata & !0xFFFF_FFFFu64) | (clock as u64);
    }

    /// Returns bits [31:0] of the metadata field.
    /// After the v3 migration this is the Lamport clock; call `extract_lamport_clock()`
    /// directly for clarity in new code.
    #[inline(always)]
    pub fn extract_clean_metadata_value(&self) -> u64 {
        self.metadata & 0xFFFF_FFFF
    }

    /// XOR parity over the five semantic fields. Store in `parity` at creation time;
    /// call `verify_ecc_parity()` to confirm integrity.
    #[inline(always)]
    pub fn calculate_parity(
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        metadata: u64,
    ) -> u64 {
        subject ^ predicate ^ object ^ context ^ metadata
    }

    #[inline(always)]
    pub fn verify_ecc_parity(&self) -> bool {
        self.parity
            == Self::calculate_parity(
                self.subject,
                self.predicate,
                self.object,
                self.context,
                self.metadata,
            )
    }

    /// Recalculate and store ECC parity after mutating any field.
    #[inline(always)]
    pub fn recalculate_parity(&mut self) {
        self.parity = Self::calculate_parity(
            self.subject,
            self.predicate,
            self.object,
            self.context,
            self.metadata,
        );
    }

    #[inline(always)]
    pub fn new_conduct_violation(reason: &[u8]) -> Self {
        let mut quin = Self::default();
        quin.predicate = 0x42_0000_0000_0000; // q42:conductViolation
        let mut obj_bytes = [0u8; 8];
        let len = core::cmp::min(reason.len(), 8);
        obj_bytes[..len].copy_from_slice(&reason[..len]);
        quin.object = u64::from_le_bytes(obj_bytes);
        quin.parity = Self::calculate_parity(
            quin.subject,
            quin.predicate,
            quin.object,
            quin.context,
            quin.metadata,
        );
        quin
    }
}

pub const MODALITY_FLAG_LLM_TENSOR: u8 = 0b1001;
pub const MODALITY_FLAG_DENSE_PHYSICS: u8 = 0b1000;
/// CLIP / mmproj vision encoder tensors in a multimodal GGUF bundle.
pub const MODALITY_FLAG_VISION_TENSOR: u8 = 0b1010;

pub trait QuinPointerExt {
    fn extract_modality_flag(&self) -> u8;
    fn extract_byte_offset(&self) -> u64;
}

impl QuinPointerExt for NQuin {
    #[inline(always)]
    fn extract_modality_flag(&self) -> u8 {
        (self.object >> 60) as u8
    }

    #[inline(always)]
    fn extract_byte_offset(&self) -> u64 {
        self.object & 0x0FFF_FFFF_FFFF_FFFF
    }
}

pub const QUINS_PER_BLOCK: usize = 850;
pub const BLOCK_MULTIPLIER_SIZE: usize = 40960; // Exact alignment across 10 sectors

#[repr(C, align(4096))]
pub struct QualiaSuperBlock {
    /// Monotonically increasing sequencing page tracker index ID
    pub block_sequence_id: u64,
    /// Binary token identifying the decentralized micro-commons owner DID root node
    pub storage_owner_did: u64,
    /// Active, filled quin array items within current page focus
    pub active_quin_count: u64,
    /// Validation value checksum bit flags
    pub validation_checksum: u32,
    /// Hard-coded sector configuration properties context (and FEA bounds)
    pub hardware_profile_flags: u32,
    /// Identifier for attached 3D voxel/tetrahedra FEA structural mesh layer
    pub fea_mesh_index_id: u64,
    /// Fixed trailing block buffer space to force page-header normalization
    pub layout_padding: [u8; 120], // Adjusted padding to maintain exactly 160 bytes header
    /// Contiguous un-padded sequential database array zones
    pub quin_ledger: [NQuin; QUINS_PER_BLOCK],
}

pub mod archive;

/// Target lanes configuration identifiers for Qualia data pipelines
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]

/// Standard payload mask denoting Ambient Telemetry (Passthrough routing) Path: Local sensor traces, timeline events, and internal files
pub enum PermissiveRoutingLane {
    /// Passthrough Fast Path: Local sensor traces, timeline events, and internal files
    PassthroughStandard = 0x00,
    /// Enforces Permissive Commons compensation milestone evaluation gates
    EnforcePermissiveCommons = 0x01,
    /// Enforces absolute multi-signatory safeguards for sensitive personal links
    EnforceBilateralMicroCommons = 0x02,
    /// Triggers GPU/NPU to run bounding hull math and linguistic semantic checking
    SpatiotemporalAmbiguous = 0x03,
}

// Bitwise parameters checked for targeted DID tracks
pub const MASK_AUTHENTICATED_NATURAL_PERSON: u16 = 0b0000_0001;
pub const MASK_BILATERAL_IDENTITY_LOCKED: u16 = 0b0000_0010;
pub const MASK_COMMERCIAL_BILLABLE_GATE: u16 = 0b0000_0100;
pub const MASK_WORK_OBLIGATION_SATISFIED: u16 = 0b0000_1000;

#[inline(always)]
pub fn evaluate_permissive_runtime_gate(
    entry_policy_mask: u16,
    requesting_agent_signature_flags: u16,
) -> bool {
    // If permissive commons work metrics or cost recoupments are met, data opens at zero cost
    if (entry_policy_mask & MASK_WORK_OBLIGATION_SATISFIED) != 0 {
        return true;
    }

    // Halt corporate analytics data mining if programmatic micro-payment ticks fail
    if (requesting_agent_signature_flags & MASK_COMMERCIAL_BILLABLE_GATE) != 0
        && (entry_policy_mask & MASK_COMMERCIAL_BILLABLE_GATE) != 0
    {
        return false;
    }

    // Multi-signatory guardian/ward validation constraints check
    if (entry_policy_mask & MASK_BILATERAL_IDENTITY_LOCKED) != 0
        && (requesting_agent_signature_flags & MASK_AUTHENTICATED_NATURAL_PERSON) == 0
    {
        return false;
    }

    true
}

pub struct QuinIncrementalScanner<'a> {
    pub file_descriptor: &'a File,
    pub block_sector_offsets: &'a [u64],
    pub current_cursor: usize,
    pub agent_signature_attributes: u16,
    /// Stack pre-allocated workspace ensures the app memory footprint remains flatlined
    pub allocated_working_buffer: QualiaSuperBlock,
}

impl<'a> Stream for QuinIncrementalScanner<'a> {
    type Item = Result<Vec<NQuin>, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.current_cursor >= self.block_sector_offsets.len() {
            return Poll::Ready(None); // Stream scan sequence completed
        }

        let file_offset = self.block_sector_offsets[self.current_cursor];
        if file_offset == 0 {
            return Poll::Ready(None);
        }

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::FileExt;

            // Unpack layout buffer straight into register space via raw block copy paths
            let destination_ptr = &mut self.allocated_working_buffer as *mut _ as *mut u8;
            let byte_slice =
                unsafe { std::slice::from_raw_parts_mut(destination_ptr, BLOCK_MULTIPLIER_SIZE) };

            if let Err(e) = self.file_descriptor.read_exact_at(byte_slice, file_offset) {
                return Poll::Ready(Some(Err(e)));
            }
        }

        #[cfg(target_family = "windows")]
        {
            use std::os::windows::fs::FileExt;

            let destination_ptr = &mut self.allocated_working_buffer as *mut _ as *mut u8;
            let byte_slice =
                unsafe { std::slice::from_raw_parts_mut(destination_ptr, BLOCK_MULTIPLIER_SIZE) };

            let mut bytes_read = 0;
            while bytes_read < BLOCK_MULTIPLIER_SIZE {
                match self.file_descriptor.seek_read(
                    &mut byte_slice[bytes_read..],
                    file_offset + bytes_read as u64,
                ) {
                    Ok(0) => {
                        return Poll::Ready(Some(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "failed to fill whole buffer",
                        ))))
                    }
                    Ok(n) => bytes_read += n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(e) => return Poll::Ready(Some(Err(e))),
                }
            }
        }

        // Taking a reference to a field of a packed struct is fine if we just extract it.
        // Wait, we can't take a reference to a packed struct element without caution.
        // Using `std::ptr::addr_of!` or just making a local copy is safe.
        // `self.allocated_working_buffer.quin_ledger[0]` copies the 40-byte struct because it implements Copy.
        let sampling_quin = self.allocated_working_buffer.quin_ledger[0];

        if !sampling_quin.verify_ecc_parity() {
            return Poll::Ready(Some(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Quin failed ECC parity check - Sector corrupted",
            ))));
        }

        match sampling_quin.identify_routing_lane() {
            PermissiveRoutingLane::EnforcePermissiveCommons => {
                let bitmask = sampling_quin.extract_clean_metadata_value() as u16;
                if !evaluate_permissive_runtime_gate(bitmask, self.agent_signature_attributes) {
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Target resource permissive commons access criteria unfulfilled",
                    ))));
                }
            }
            PermissiveRoutingLane::EnforceBilateralMicroCommons => {
                let _relation_token = sampling_quin.extract_clean_metadata_value();
                // Core evaluation checks require signature presence before output emission
                if (self.agent_signature_attributes & MASK_AUTHENTICATED_NATURAL_PERSON) == 0 {
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Protected Bilateral Micro-Commons authorization token missing",
                    ))));
                }
            }
            PermissiveRoutingLane::PassthroughStandard => {
                // Directly bypasses permission check matrices for regular local database paths
            }
            PermissiveRoutingLane::SpatiotemporalAmbiguous => {
                // Routed to the Geometric Pruning Pipeline and Agent Orchestrator
            }
        }

        self.current_cursor += 1;
        let elements_in_frame = self.allocated_working_buffer.active_quin_count as usize;

        // Cannot take a slice of an unaligned array. However, `NQuin` is 40 bytes, which is a multiple of 8.
        // But `#[repr(C, packed)]` causes the elements in `quin_ledger` to be tightly packed with 1-byte alignment.
        // But since it's 40 bytes (multiple of 8), they end up exactly where they would be if aligned to 8!
        // We can safely create a Vec by copying element by element to avoid unaligned reference warnings, or just use `to_vec()` if it compiles.
        // Let's use a safe iterator to copy:
        let mut emitted_vector_slice = Vec::with_capacity(elements_in_frame);
        for i in 0..elements_in_frame {
            emitted_vector_slice.push(self.allocated_working_buffer.quin_ledger[i]);
        }

        Poll::Ready(Some(Ok(emitted_vector_slice)))
    }
}

impl Drop for QualiaSuperBlock {
    fn drop(&mut self) {
        // Safe volatile memory scrubbing to clear tracking signatures.
        unsafe {
            std::ptr::write_volatile(self as *mut _, std::mem::zeroed());
        }
    }
}

// mcp_* modules now live in mcp/ (see MODULE_REORG_PLAN.md). The `pub mod mcp;`
// declaration + path-preserving re-exports are below, near the old mcp_server line.
// (asset_bridge moved to render::assets in Phase 0.2a)
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod deontic_logic;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod epistemic;
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
pub mod sync;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod wal;

// The model-inference runtime: reads GGUF weight files and runs the tensor program on the GPU.
// It is a *runtime*, not an "engine" — the mathematics it executes lives in `crate::solvers`
// (GEMM, activations/softmax/normalization, attention, RoPE, FFN), each proven equal to the
// kernels here. `inference_runtime` is the honest name; `gguf_bridge` is retained (the directory
// rename is deferred — it is a shared performance lane — but the honest name is available).
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod gguf_bridge;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use gguf_bridge as inference_runtime;
/// Phase 4: AOT GGUF → P64 LLM-weight container compiler.
/// Phase 6 / task #12: safetensor (+ MLX) source parsing + dtype gate for the streaming transcoder.
/// Task #12 / STELLAR §A: BitNet b1.58 ternary quantization codec (compression during transcode).
/// Task #12 / STELLAR §A: tensor-name → engine GEMM-role mapping + the ternary (FFN-only) policy.
/// Task #12 / STELLAR §A: native GPU dispatch of the ternary GEMM kernel + on-device parity test.
/// STELLAR §A A1a: GPU top-K reduction — CPU oracle + host merge + the WGSL kernel.
/// STELLAR §A A1a: native GPU dispatch of the top-K reduction + on-device parity test.
/// STELLAR §A AH-track H0: host topology + capability sensor (enumerate all adapters; discrete vs unified).
/// STELLAR §A AH-track H1(a): cross-circuit GEMV benchmark → measured capability matrix (CPU/iGPU/GPU).
/// STELLAR §A AH-track H2: residency + device-priority planner (discovery → employment plan, D31).
/// STELLAR §A AH-track H1(a) cache: CBOR hardware passport (cache discovery, fast-boot skip, D26).
/// STELLAR §A A0 (D17/D22): shared native LLM benchmark harness — the one measurement
/// surface for the existing F16/Q8 path and the future ternary/top-k paths.
/// STELLAR §A W2 (D17): per-kernel GPU timestamp profiler for the LLM forward/decode path.
/// STELLAR §A W3: in-project GPU↔CPU kernel-parity oracle (error metrics + synthetic quant weights).
/// STELLAR §A W1: in-project quality oracle (perplexity / KL / coherence + the quant quality gate).
/// STELLAR §A AWQ: activation-statistics capture (the AWQ forward hook) for calibrated quantization.
// mcp/ category (moved from crate root). Re-exports keep crate::mcp_server and
// crate::mcp_cooperation paths stable for qualia-cli and intra-crate callers.
pub mod mcp;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use mcp::mcp_cooperation;
#[cfg(not(target_arch = "wasm32"))]
pub use mcp::mcp_server;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod ode_solver;
#[cfg(not(target_arch = "wasm32"))]
pub mod qpu_ingress;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod quantum_dft;

#[cfg(target_arch = "wasm32")]
pub mod wasm_edge;

#[cfg(target_arch = "wasm32")]
pub mod wasm_storage;

/// A zero-allocation compile-time hashing function for Q-Turtle macros.
/// Uses FNV-1a, then truncates to 60 bits so the result is a pure IDENTIFIER:
/// the top 4 bits [60..63] are reserved as the role-keyed type/modality/tag
/// overlay (inline datatype tags in the Object register, the deontic
/// DEFEATER_BIT in the Predicate, etc.). This is the ONE canonical term-identity
/// hash — it MUST stay bit-for-bit identical to `lexicon::generate_60bit_token`
/// (same FNV constants + the same 0x0FFF_FFFF_FFFF_FFFF mask) so compile-time-baked
/// URIs and runtime-parsed/ingested URIs share a single hash space and JOIN.
pub const fn q_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = hash ^ (bytes[i] as u64);
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    // Truncate to 60 bits (top 4 reserved for the type/modality/tag overlay).
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

/// Advanced 2026 Q-Turtle Macro
/// Translates terse semantic triples into physical 48-byte hardware Quins
/// strictly at compile time. Eliminates runtime string allocations entirely.
#[macro_export]
macro_rules! q_turtle {
    ($s:expr, $p:expr, $o:expr) => {
        $crate::NQuin {
            subject: $crate::q_hash($s),
            predicate: $crate::q_hash($p),
            object: $crate::q_hash($o),
            context: 0,
            metadata: 0b01 << 61, // Default to Permissive Commons routing
            parity: 0,
        }
    };
}

// Tests for Antigravity Validation Pipeline
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualia_spatial_val() {
        use crate::domains::geospatial::spatial::{embed_h3_context, SpatiotemporalQuadTree};

        let h3_index = 0x000a1072b59ffff; // Mock H3 cell index payload
        let context_val = embed_h3_context(h3_index, 10, 42);
        let expected_context =
            ((10u64 & 0x0F) << 59) | ((42u64 & 0x7F) << 52) | (h3_index & 0x000F_FFFF_FFFF_FFFF);
        assert_eq!(
            context_val, expected_context,
            "Failed to embed H3 index into context"
        );

        let quad_tree = SpatiotemporalQuadTree::new((0.0, 0.0, 100.0, 100.0));

        let results = quad_tree.query_region(10.0, 10.0, 20.0, 20.0, 0, 0);
        // We expect it to be empty since it's a structural mock
        assert_eq!(
            results.len(),
            0,
            "SpatiotemporalQuadTree placeholder query failed"
        );
    }

    #[test]
    fn qualia_logic_val() {
        use crate::modalities::logic::core::{WebizenCompiler, WebizenVM};
        let q = NQuin {
            subject: 0,
            predicate: 100,
            object: 18,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        let mut vm = WebizenVM::new();
        // Use the Compiler mock to generate bytecode for the constraint:
        // Must have predicate 100 and object 18.
        let bytecode = WebizenCompiler::compile_mock_constraint();
        vm.load_bytecode(&bytecode);

        let result = vm.execute_constraint(&q);
        assert_eq!(
            result, true,
            "Webizen VM failed to validate constraint byte-code"
        );
    }

    #[test]
    fn qualia_webizen_guardianship() {
        use crate::modalities::logic::core::{WebizenOpcode, WebizenVM};

        // 0b11 << 61 signals SpatiotemporalAmbiguous for bounding logic
        let q = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0b11 << 61 | 500,
            parity: 0,
        };

        let mut vm = WebizenVM::new();
        let bytecode = vec![
            WebizenOpcode::EvalMetadataMask(499), // Try to match exactly 499 on the lower 16 bits
            WebizenOpcode::HaltIfFalse,
        ];
        vm.load_bytecode(&bytecode);

        let result = vm.execute_constraint(&q);
        assert_eq!(
            result, false,
            "Webizen VM failed to deny mismatched EvalMetadataMask"
        );
    }

    #[test]
    fn qualia_ldp_rdf_star_mapping() {
        use crate::solid_ldp::SolidLdpFacade;
        let q = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 4,
            metadata: 0b11 << 61 | 555,
            parity: 0,
        };

        let rdf_output = SolidLdpFacade::serialize_to_rdf_star(&q);

        // Ensure it mapped to RDF quads with context
        assert!(rdf_output.contains("GRAPH <urn:qualia:context:4>"));
        // Ensure RDF-star reification with GeoSPARQL WKT is present because it's SpatiotemporalAmbiguous
        assert!(rdf_output.contains("geo:asWKT"));
        assert!(rdf_output.contains("qualia:hardwareIntegrity \"VERIFIED_ECC_PASS\""));
    }

    #[test]
    fn qualia_vector_density() {
        use crate::domains::mathematical::geometric::{
            extract_spatial_projection, VectorSectorMap,
        };
        let q = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 42,
            parity: 0,
        };
        let projection = extract_spatial_projection(&q);

        let sector_map = VectorSectorMap {
            sector_id: 2,
            active: true,
        }; // 42 % 10 = 2
        assert_eq!(
            sector_map.contains(projection),
            true,
            "VectorSectorMap failed to include point within bounding hull"
        );

        let out_of_bounds_map = VectorSectorMap {
            sector_id: 3,
            active: true,
        };
        assert_eq!(
            out_of_bounds_map.contains(projection),
            false,
            "VectorSectorMap failed to prune out-of-bounds point"
        );
    }

    #[test]
    fn qualia_validate_volatile_drop() {
        let mut block = Box::new(unsafe { std::mem::zeroed::<QualiaSuperBlock>() });
        block.block_sequence_id = 12345;
        assert_eq!(block.block_sequence_id, 12345);
        drop(block);
    }

    #[test]
    fn qualia_validate_quin() {
        assert_eq!(
            std::mem::size_of::<NQuin>(),
            48,
            "NQuin must be exactly 48 bytes"
        );
    }

    #[test]
    fn qualia_validate_ecc() {
        let mut q = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        assert_eq!(q.verify_ecc_parity(), true, "Valid ECC parity should pass");

        q.parity = u64::MAX;
        assert_eq!(
            q.verify_ecc_parity(),
            false,
            "Corrupted ECC parity should fail"
        );
    }

    #[test]
    fn qualia_validate_alignment() {
        assert_eq!(
            std::mem::size_of::<QualiaSuperBlock>(),
            40960,
            "QualiaSuperBlock must be exactly 40960 bytes (10 sectors)"
        );
        assert_eq!(
            std::mem::align_of::<QualiaSuperBlock>(),
            4096,
            "QualiaSuperBlock must be page aligned (4096 bytes)"
        );
    }

    #[test]
    fn qualia_validate_routing() {
        // Routing lane = bits[62:61]; Lamport clock = bits[31:0] (v3 layout).
        // Values set in bits[31:0] are the Lamport clock; routing lane bits stay in [62:61].

        let q1 = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0b00u64 << 61 | 12345,
            parity: 0,
        };
        assert_eq!(
            q1.identify_routing_lane(),
            PermissiveRoutingLane::PassthroughStandard
        );
        assert_eq!(q1.extract_lamport_clock(), 12345);

        let q2 = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0b01u64 << 61 | 67890,
            parity: 0,
        };
        assert_eq!(
            q2.identify_routing_lane(),
            PermissiveRoutingLane::EnforcePermissiveCommons
        );
        assert_eq!(q2.extract_lamport_clock(), 67890);

        let q3 = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0b10u64 << 61 | 42,
            parity: 0,
        };
        assert_eq!(
            q3.identify_routing_lane(),
            PermissiveRoutingLane::EnforceBilateralMicroCommons
        );
        assert_eq!(q3.extract_lamport_clock(), 42);

        let q4 = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0b11u64 << 61 | 999,
            parity: 0,
        };
        assert_eq!(
            q4.identify_routing_lane(),
            PermissiveRoutingLane::SpatiotemporalAmbiguous
        );
        assert_eq!(q4.extract_lamport_clock(), 999);
    }

    #[test]
    fn nquin_metadata_v3_layout() {
        let mut q = NQuin::default();

        // Quin type nibble
        q.set_quin_type(0b1001);
        assert_eq!(q.get_quin_type(), 0b1001);
        // Routing lane reads bits[62:61] = bits 2:1 of the nibble = 0b00 (from 0b1001)
        assert_eq!(
            q.identify_routing_lane(),
            PermissiveRoutingLane::PassthroughStandard
        );

        // Sensitivity tier
        q.set_sensitivity_tier(NQuin::SENSITIVITY_TIER_MEDICAL);
        assert_eq!(q.get_sensitivity_tier(), NQuin::SENSITIVITY_TIER_MEDICAL);

        // Lamport clock
        q.set_lamport_clock(0xDEAD_BEEF);
        assert_eq!(q.extract_lamport_clock(), 0xDEAD_BEEF);

        // Ensure fields don't bleed into each other
        assert_eq!(q.get_quin_type(), 0b1001);
        assert_eq!(q.get_sensitivity_tier(), NQuin::SENSITIVITY_TIER_MEDICAL);
        assert_eq!(q.extract_lamport_clock(), 0xDEAD_BEEF);

        // Parity still works after mutations
        q.recalculate_parity();
        assert!(q.verify_ecc_parity());
    }

    #[test]
    fn engine_version_matches_cargo_pkg_version() {
        assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!ENGINE_VERSION.is_empty());
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod p2p;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod domains;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod solvers;

#[cfg(target_os = "linux")]
extern crate io_uring;
