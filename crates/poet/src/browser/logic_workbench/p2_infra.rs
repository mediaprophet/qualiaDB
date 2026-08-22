//! P2 infrastructure panels: bytecode/VM inspector, SLG arena inspector,
//! forge compute probe, compute profile, privacy/HE/DP, model lifecycle,
//! inference monitor, GGUF tokenizer, P64 weight inspector.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_logic_notification, show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

pub(super) fn append_panels(document: &Document, content: &Element) {
    content
        .append_child(&build_bytecode_vm_panel(document))
        .unwrap();
    content
        .append_child(&build_slg_arena_panel(document))
        .unwrap();
    content
        .append_child(&build_forge_compute_panel(document))
        .unwrap();
    content
        .append_child(&build_compute_profile_panel(document))
        .unwrap();
    content
        .append_child(&build_privacy_panel(document))
        .unwrap();
    content
        .append_child(&build_model_lifecycle_panel(document))
        .unwrap();
    content
        .append_child(&build_inference_monitor_panel(document))
        .unwrap();
    content
        .append_child(&build_gguf_tokenizer_panel(document))
        .unwrap();
    content
        .append_child(&build_p64_weight_panel(document))
        .unwrap();
}

pub(super) fn wire_all(document: &Document) {
    wire_bytecode_vm_panel(document);
    wire_slg_arena_panel(document);
    wire_forge_compute_panel(document);
    wire_compute_profile_panel(document);
    wire_privacy_panel(document);
    wire_model_lifecycle_panel(document);
    wire_inference_monitor_panel(document);
    wire_gguf_tokenizer_panel(document);
    wire_p64_weight_panel(document);
}

pub(super) fn build_bytecode_vm_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "bytecode_vm", false);
    panel
        .append_child(&make_section_label(
            document,
            "Bytecode / VM Inspector \u{2014} opcode trace, register state, execution stats",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "bytecode-vm-input",
            "# WebizenVM bytecode inspection\n# MSB=1 \u{2192} did:q42 topological pointer (direct jump)\n# MSB=0 \u{2192} FNV-1a dictionary hash (lexicon lookup)\n# Registers: 16, Bytecode buffer: 64 bytes\n\n# Opcodes: OP_END, OP_HALT_IF_FALSE, OP_MATCH_SUBJECT/PREDICATE/OBJECT,\n# OP_EVAL_PERMIT/OBLIGATE/FORBID, OP_HALT_VIOLATION\n\n# Query: trace execution of program X?\n# Query: dump register state after step 10?",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "bytecode-vm-trace",
            "\u{1F50D} Trace Execution",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "bytecode-vm-stats",
            "\u{1F4CA} Execution Stats",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "bytecode-vm-results",
            "Click \"Trace Execution\" to inspect VM state (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_bytecode_vm_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("bytecode-vm-trace") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "bytecode-vm-results", "bytecode-vm");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("bytecode-vm-stats") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(
                &doc,
                "Execution stats: 42 matches, 128 cycles, 3 direct jumps (mock)",
            );
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_slg_arena_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "slg_arena", false);
    panel
        .append_child(&make_section_label(
            document,
            "SLG Arena Inspector \u{2014} 42MB ring buffer, 917,504 Quin slots, rule registry",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "slg-arena-input",
            "# SLG Arena parameters\n# ARENA_SIZE = 42 * 1024 * 1024 (42MB)\n# QUIN_SIZE = 48, MAX_SLOTS = 917,504\n# RECENT_SLOT_RING = 512, MAX_RULE_VARS = 16\n# MAX_GUARD_CONCLUSIONS = 256, MAX_PREMISE_DEPTH = 16\n# MAX_FIXPOINT_ROUNDS = 16\n\n# Query: slot usage summary?\n# Query: rule registry dump?\n# Query: fixpoint round count?",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "slg-arena-inspect",
            "\u{1F4CB} Inspect Arena",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "slg-arena-results",
            "Click \"Inspect Arena\" to view slot usage (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_slg_arena_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("slg-arena-inspect") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "slg-arena-results", "slg-arena");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_forge_compute_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "forge_compute", false);
    panel
        .append_child(&make_section_label(
            document,
            "Forge Compute Probe \u{2014} WGSL/PTX/CUDA/HLSL/MSL/SPIR-V, Top-K, GEMM, FFT, certification",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "forge-compute-op",
        &[
            ("top_k", "Top-K"),
            ("gemm", "GEMM"),
            ("fft", "FFT"),
            ("certify", "Certify Kernel"),
            ("roofline", "Roofline Estimate"),
            ("autotune", "Auto-Tune"),
            ("validate", "Naga Validate"),
        ],
    ))
    .unwrap();
    row.append_child(&make_select(
        document,
        "forge-compute-backend",
        &[
            ("wgsl", "WGSL (WebGPU)"),
            ("ptx", "PTX (NVIDIA)"),
            ("cuda_c", "CUDA-C"),
            ("hlsl", "HLSL (DirectX)"),
            ("msl", "MSL (Metal)"),
            ("spirv", "SPIR-V"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "forge-compute-input",
            "# Forge compute probe parameters\n# Top-K: input_size=4096, k=10\n# GEMM: M=512, N=512, K=512, dtype=f16\n# FFT: size=1024, inverse=false\n# Certify: kernel_name, tolerance=1e-4\n# Roofline: adapter_name, compute_units",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "forge-compute-evaluate",
            "\u{1F525} Run Probe",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "forge-compute-results",
            "Click \"Run Probe\" to execute forge compute (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_forge_compute_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("forge-compute-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "forge-compute-results", "forge-compute");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_compute_profile_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "compute_profile", false);
    panel
        .append_child(&make_section_label(
            document,
            "Compute Profile \u{2014} GPU adapter, compute units, memory, features",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "compute-profile-input",
            "# Compute profile query\n# Returns: adapter name, compute units, VRAM, features\n# Supports: WebGPU, CUDA, Metal, DirectX\n\n# Query: get_qualia_compute_profile?\n# Query: list available adapters?",
            "80px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "compute-profile-evaluate",
            "\u{1F4BB} Get Profile",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "compute-profile-results",
            "Click \"Get Profile\" to query compute capabilities (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_compute_profile_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("compute-profile-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "compute-profile-results", "compute-profile");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_privacy_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "privacy", false);
    panel
        .append_child(&make_section_label(
            document,
            "Privacy / HE / DP \u{2014} BFV homomorphic encryption, differential privacy, secure aggregation",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "privacy-op",
        &[
            ("bfv_encrypt", "BFV Encrypt"),
            ("bfv_decrypt", "BFV Decrypt"),
            ("bfv_add", "BFV Homomorphic Add"),
            ("bfv_mul", "BFV Homomorphic Multiply"),
            ("dp_laplace", "DP Laplace Mechanism"),
            ("dp_gaussian", "DP Gaussian Mechanism"),
            ("dp_accounting", "DP Accounting (basic/advanced/RDP)"),
            ("secure_agg", "Secure Aggregation"),
            ("key_rotate", "Key Rotation"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "privacy-input",
            "# Privacy engine parameters\n# BFV: plaintext=[1,2,3,4], key_index=0\n# DP Laplace: sensitivity=1.0, epsilon=0.1\n# DP Gaussian: sensitivity=1.0, epsilon=0.1, delta=1e-5\n# Secure Aggregation: participants=5, threshold=3\n# Key rotation: key_index=0, new_index=1",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "privacy-evaluate",
            "\u{1F510} Execute",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "privacy-results",
            "Click \"Execute\" to run privacy computation (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_privacy_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("privacy-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "privacy-results", "privacy");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_model_lifecycle_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "model_lifecycle", false);
    panel
        .append_child(&make_section_label(
            document,
            "Model Lifecycle \u{2014} Discovered \u{2192} MappedToDisk \u{2192} StreamingVRAM \u{2192} Active \u{2192} Scrubbing",
        ))
        .unwrap();
    panel
        .append_child(&make_text_input(
            document,
            "model-lifecycle-name",
            "Model name (e.g. llama-3-8b)",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "model-lifecycle-input",
            "# Model lifecycle query\n# State machine: Discovered \u{2192} MappedToDisk \u{2192} StreamingVRAM \u{2192} Active \u{2192} Scrubbing\n# Thermal governor: Cool / Warm / Critical\n# Orchestration: Committed / Blocked / Failed\n\n# Query: lifecycle status for llama-3-8b?\n# Query: thermal state?\n# Query: orchestrate inference?",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "model-lifecycle-evaluate",
            "\u{1F4E6} Get Status",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "model-lifecycle-evict",
            "\u{1F9F9} Evict Model",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "model-lifecycle-results",
            "Click \"Get Status\" to check model lifecycle (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_model_lifecycle_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("model-lifecycle-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "model-lifecycle-results", "model-lifecycle");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("model-lifecycle-evict") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(&doc, "Model eviction: async scrubbing initiated (mock)");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_inference_monitor_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "inference_monitor", false);
    panel
        .append_child(&make_section_label(
            document,
            "Inference Monitor \u{2014} LLM telemetry, token throughput, KV cache, prefill/decode",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "inference-monitor-input",
            "# Inference monitor query\n# Telemetry: tokens/sec, prefill latency, decode latency\n# KV cache: slots used, slots total, eviction count\n# Memory: VRAM allocated, VRAM peak\n\n# Query: get LLM telemetry?\n# Query: KV cache utilization?",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "inference-monitor-evaluate",
            "\u{1F4CA} Get Telemetry",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "inference-monitor-results",
            "Click \"Get Telemetry\" to view inference stats (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_inference_monitor_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("inference-monitor-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "inference-monitor-results", "inference-monitor");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_gguf_tokenizer_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "gguf_tokenizer", false);
    panel
        .append_child(&make_section_label(
            document,
            "GGUF Tokenizer Inspector \u{2014} vocab, BPE merges, special tokens, chat templates",
        ))
        .unwrap();
    panel
        .append_child(&make_text_input(
            document,
            "gguf-tokenizer-model",
            "Model file (e.g. llama-3-8b.gguf)",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "gguf-tokenizer-input",
            "# GGUF tokenizer inspection\n# GgufTokenizer: vocab, bos_token_id, eos_token_id, add_bos_token\n# Pre-types: BPE, SentencePiece, Llama3, ChatML\n# Stop tokens: up to 8\n\n# Query: vocab size?\n# Query: tokenize \"Hello world\"?\n# Query: detect chat family?\n# Query: list special tokens?",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "gguf-tokenizer-evaluate",
            "\u{1F9F8} Inspect Tokenizer",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "gguf-tokenizer-results",
            "Click \"Inspect Tokenizer\" to view tokenizer details (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_gguf_tokenizer_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("gguf-tokenizer-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "gguf-tokenizer-results", "gguf-tokenizer");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_p64_weight_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "p64_weight", false);
    panel
        .append_child(&make_section_label(
            document,
            "P64 Weight Inspector \u{2014} 64B headers, tensor entries, CRC-32C, 16KB pages",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "p64-weight-input",
            "# P64 weight container inspection\n# MAGIC = b\"p64\\0\", VERSION = 4\n# DEFAULT_PAGE_LOG2 = 14 (16KB pages)\n# Layout: P64WeightHeader(64B) + P64TensorEntry[](64B each) + pad + tensor blob\n\n# Query: header inspection?\n# Query: tensor entry list?\n# Query: CRC-32C verification?",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "p64-weight-evaluate",
            "\u{1F4E6} Inspect Container",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "p64-weight-results",
            "Click \"Inspect Container\" to view P64 structure (mock).",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_p64_weight_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("p64-weight-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "p64-weight-results", "p64-weight");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
