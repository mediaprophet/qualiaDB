use dioxus::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use qualia_core_db::NQuin;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default)]
struct NQuin {
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
    parity: u64,
}
use serde::Deserialize;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{EventSource, MessageEvent};

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct HardwareTelemetry {
    cpu: String,
    ram: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(
        event: &str,
        handler: &js_sys::Function,
    ) -> Result<js_sys::Function, wasm_bindgen::JsValue>;
}

use crate::canvas_editor::{
    clamp_pane_origin, clamp_pane_size, grid_metrics, new_workspace_shell, pixel_delta_to_grid,
    qprime_elevation_css, snap_u16, CanvasEditorMode, PaneInteraction, WorkspaceHistory,
};
use crate::components::ontology_import_wizard::{OntologyImportWizard, OntologyLayoutSuggestion};
use crate::components::selection_sidebar::SelectionSidebar;
use crate::pane_generator;
use crate::pane_registry::{
    builtin_pane_definitions, category_label, find_pane, PaneCategory, PaneDefinition,
};
use crate::render::motion::Spring;
use crate::render::motion_loop::{
    spawn_ui_motion_loop, step_mode_pulse_spring, trigger_mode_pulse,
};
use crate::theme_engine;
use crate::theme_engine::{
    builtin_theme_catalog, collect_stylesheets, join_theme_classes, render_scope_tokens,
    resolve_theme, theme_binding_provenance, theme_selection_pulse, ResolvedTheme, ThemeBinding,
};

pub use crate::canvas_model::{
    CoordinateSpace, LayerBehavior, LayoutStrategy, Page, PanePlacement, PresentationMode, UiMode,
    WebizenWorkspace,
};

// ─────────────────────────────────────────────────────────────
// Default pane layouts for known QApps
// ─────────────────────────────────────────────────────────────

fn p(cid: &str, x: u16, y: u16, w: u16, h: u16) -> PanePlacement {
    PanePlacement {
        component_id: cid.to_string(),
        x,
        y,
        w,
        h,
        data_bindings: vec![],
        binds_rpc: None,
        requires_capability: vec![],
        ui_mode: None,
        layer: LayerBehavior::Docked,
        anchor: None,
        min_w_points: 0,
        min_h_points: 0,
        supported_presentations: vec![],
        theme: ThemeBinding::default(),
    }
}

fn app_display_name(app_id: &str) -> &'static str {
    match app_id {
        "context-studio" => "Context Studio",
        "qapp-studio" => "QApp Studio",
        "profile-identity" => "Profile & Identity",
        "hardware-config" => "Hardware Configurator",
        "chat" => "Neuro-Symbolic Chat",
        "llm-harness" => "LLM Model Harness",
        "lora-manager" => "LoRA Adapter Manager",
        "agent-config" => "Agent Configuration",
        "inference-monitor" => "Inference Monitor",
        "model-lifecycle" => "Model Lifecycle",
        "ontology-builder" => "Ontology Builder",
        "sparql-explorer" => "SPARQL Explorer",
        "n3-logic-studio" => "N3 Logic Studio",
        "rdf-star-editor" => "RDF-Star Editor",
        "solid-browser" => "Solid LDP Browser",
        "physics-sim" => "Physics Simulator",
        "chemistry-modeler" => "Chemistry Modeler",
        "ode-lab" => "ODE & Calculus Lab",
        "matrix-lab" => "Matrix & Linear Algebra",
        "stats-lab" => "Statistical Analysis Lab",
        "bioinformatics-lab" => "Bioinformatics Lab",
        "qpu-optimizer" => "QPU Optimizer",
        "quantum-dft" => "Quantum DFT Lab",
        "qaoa-explorer" => "QAOA Explorer",
        "qpu-providers" => "QPU Provider Manager",
        "health-vitals" => "Health Vital Monitor",
        "clinical-risk" => "Clinical Risk Scorer",
        "dicom-viewer" => "DICOM Viewer",
        "anatomy-browser" => "Anatomy Context Browser",
        "comorbidity" => "Comorbidity Analyzer",
        "portfolio" => "Portfolio Analyzer",
        "risk-engine" => "Risk Engine",
        "gbm-sim" => "GBM Simulator",
        "tax-schema" => "Tax Schema Editor",
        "agreements" => "Agreements & Rights",
        "key-vault" => "Key Vault Manager",
        "zk-studio" => "ZK Proof Studio",
        "deontic-editor" => "Deontic Logic Editor",
        "shacl-validator" => "SHACL Validator",
        "wal-inspector" => "WAL Inspector",
        "q42-volume" => "Q42 Volume Manager",
        "provenance-graph" => "Provenance Graph",
        "storage-config" => "Storage Driver Config",
        "webtorrent" => "WebTorrent Seeder",
        "p2p-dashboard" => "P2P Node Dashboard",
        "ebpf-filter" => "eBPF Filter Manager",
        "acoustic-ble" => "Acoustic BLE Mesh",
        "mcp-inspector" => "MCP Tool Inspector",
        "benchmark" => "Benchmark Harness",
        "cli-bridge" => "CLI Bridge",
        "extension-bus" => "Extension Bus",
        "nexus" => "Nexus — Quantum Research Cooperative",
        _ => "QApp Editor",
    }
}

// Grid is 96 × 64 points. Helper layout constants:
//   Left 2/3:  x=0,  w=62
//   Right 1/3: x=64, w=30
//   Top half:  y=0,  h=30
//   Bot half:  y=32, h=30
//   Full:      x=0, y=0, w=94, h=62
fn default_panes_for_app(app_id: &str) -> Vec<PanePlacement> {
    match app_id {
        "context-studio" => vec![
            p("contextual-workspace", 0, 0, 62, 62),
            p("neuro-symbolic-chat", 64, 0, 30, 30),
            p("sparql-explorer", 64, 32, 30, 30),
        ],
        "chat" => vec![
            p("neuro-symbolic-chat", 0, 0, 62, 62),
            p("inference-monitor", 64, 0, 30, 30),
            p("lora-manager", 64, 32, 30, 30),
        ],
        "llm-harness" => vec![
            p("llm-harness", 0, 0, 56, 62),
            p("inference-monitor", 58, 0, 36, 20),
            p("model-lifecycle", 58, 22, 36, 20),
            p("lora-manager", 58, 44, 36, 18),
        ],
        "lora-manager" => vec![
            p("lora-manager", 0, 0, 56, 36),
            p("llm-harness", 0, 38, 56, 24),
            p("inference-monitor", 58, 0, 36, 30),
            p("agent-config", 58, 32, 36, 30),
        ],
        "agent-config" => vec![
            p("agent-config", 0, 0, 56, 40),
            p("inference-monitor", 58, 0, 36, 30),
            p("model-lifecycle", 58, 32, 36, 30),
        ],
        "inference-monitor" => vec![
            p("inference-monitor", 0, 0, 94, 30),
            p("system-diagnostics", 0, 32, 46, 30),
            p("benchmark-harness", 48, 32, 46, 30),
        ],
        "model-lifecycle" => vec![
            p("model-lifecycle", 0, 0, 56, 36),
            p("llm-harness", 58, 0, 36, 20),
            p("lora-manager", 58, 22, 36, 20),
            p("inference-monitor", 0, 38, 56, 24),
        ],
        "ontology-builder" => vec![
            p("contextual-workspace", 0, 0, 56, 62),
            p("n3-logic-studio", 58, 0, 36, 30),
            p("sparql-explorer", 58, 32, 36, 30),
        ],
        "sparql-explorer" => vec![
            p("sparql-explorer", 0, 0, 62, 62),
            p("provenance-graph", 64, 0, 30, 30),
            p("n3-logic-studio", 64, 32, 30, 30),
        ],
        "n3-logic-studio" => vec![
            p("n3-logic-studio", 0, 0, 62, 62),
            p("shacl-validator", 64, 0, 30, 30),
            p("deontic-logic-editor", 64, 32, 30, 30),
        ],
        "rdf-star-editor" => vec![
            p("rdf-star-editor", 0, 0, 62, 62),
            p("provenance-graph", 64, 0, 30, 30),
            p("sparql-explorer", 64, 32, 30, 30),
        ],
        "solid-browser" => vec![
            p("solid-ldp-browser", 0, 0, 62, 62),
            p("sparql-explorer", 64, 0, 30, 30),
            p("key-vault-manager", 64, 32, 30, 30),
        ],
        "physics-sim" => vec![
            p("physics-simulator", 0, 0, 62, 62),
            p("statistical-analysis", 64, 0, 30, 30),
            p("ode-solver", 64, 32, 30, 30),
        ],
        "chemistry-modeler" => vec![
            p("chemistry-modeler", 0, 0, 62, 62),
            p("bioinformatics-lab", 64, 0, 30, 30),
            p("statistical-analysis", 64, 32, 30, 30),
        ],
        "ode-lab" => vec![
            p("ode-solver", 0, 0, 62, 62),
            p("matrix-lab", 64, 0, 30, 30),
            p("statistical-analysis", 64, 32, 30, 30),
        ],
        "matrix-lab" => vec![
            p("matrix-lab", 0, 0, 62, 62),
            p("ode-solver", 64, 0, 30, 30),
            p("statistical-analysis", 64, 32, 30, 30),
        ],
        "stats-lab" => vec![
            p("statistical-analysis", 0, 0, 62, 62),
            p("matrix-lab", 64, 0, 30, 30),
            p("provenance-graph", 64, 32, 30, 30),
        ],
        "bioinformatics-lab" => vec![
            p("bioinformatics-lab", 0, 0, 62, 62),
            p("chemistry-modeler", 64, 0, 30, 30),
            p("statistical-analysis", 64, 32, 30, 30),
        ],
        "qpu-optimizer" => vec![
            p("qpu-optimizer", 0, 0, 56, 36),
            p("qaoa-explorer", 0, 38, 56, 24),
            p("qpu-providers", 58, 0, 36, 30),
            p("statistical-analysis", 58, 32, 36, 30),
        ],
        "quantum-dft" => vec![
            p("quantum-dft", 0, 0, 62, 62),
            p("qpu-optimizer", 64, 0, 30, 30),
            p("qpu-providers", 64, 32, 30, 30),
        ],
        "qaoa-explorer" => vec![
            p("qaoa-explorer", 0, 0, 62, 62),
            p("qpu-optimizer", 64, 0, 30, 30),
            p("statistical-analysis", 64, 32, 30, 30),
        ],
        "qpu-providers" => vec![
            p("qpu-providers", 0, 0, 56, 40),
            p("qpu-optimizer", 58, 0, 36, 30),
            p("qaoa-explorer", 58, 32, 36, 30),
        ],
        "health-vitals" => vec![
            p("health-vital-monitor", 0, 0, 46, 30),
            p("clinical-risk-scorer", 0, 32, 46, 30),
            p("dicom-viewer", 48, 0, 46, 30),
            p("comorbidity-analyzer", 48, 32, 46, 30),
        ],
        "clinical-risk" => vec![
            p("clinical-risk-scorer", 0, 0, 62, 62),
            p("health-vital-monitor", 64, 0, 30, 30),
            p("comorbidity-analyzer", 64, 32, 30, 30),
        ],
        "dicom-viewer" => vec![
            p("dicom-viewer", 0, 0, 62, 62),
            p("health-vital-monitor", 64, 0, 30, 30),
            p("clinical-risk-scorer", 64, 32, 30, 30),
        ],
        "anatomy-browser" => vec![
            p("health-vital-monitor", 0, 0, 46, 30),
            p("dicom-viewer", 48, 0, 46, 30),
            p("clinical-risk-scorer", 0, 32, 46, 30),
            p("comorbidity-analyzer", 48, 32, 46, 30),
        ],
        "comorbidity" => vec![
            p("comorbidity-analyzer", 0, 0, 62, 62),
            p("clinical-risk-scorer", 64, 0, 30, 30),
            p("health-vital-monitor", 64, 32, 30, 30),
        ],
        "portfolio" => vec![
            p("portfolio-analyzer", 0, 0, 62, 36),
            p("risk-engine", 0, 38, 62, 24),
            p("gbm-simulator", 64, 0, 30, 30),
            p("statistical-analysis", 64, 32, 30, 30),
        ],
        "risk-engine" => vec![
            p("risk-engine", 0, 0, 62, 62),
            p("portfolio-analyzer", 64, 0, 30, 30),
            p("gbm-simulator", 64, 32, 30, 30),
        ],
        "gbm-sim" => vec![
            p("gbm-simulator", 0, 0, 62, 62),
            p("risk-engine", 64, 0, 30, 30),
            p("statistical-analysis", 64, 32, 30, 30),
        ],
        "tax-schema" => vec![
            p("shacl-validator", 0, 0, 56, 40),
            p("deontic-logic-editor", 58, 0, 36, 30),
            p("agreements-rights", 58, 32, 36, 30),
        ],
        "agreements" => vec![
            p("agreements-rights", 0, 0, 62, 62),
            p("deontic-logic-editor", 64, 0, 30, 30),
            p("shacl-validator", 64, 32, 30, 30),
        ],
        "key-vault" => vec![
            p("key-vault-manager", 0, 0, 62, 62),
            p("zk-proof-studio", 64, 0, 30, 30),
            p("agreements-rights", 64, 32, 30, 30),
        ],
        "zk-studio" => vec![
            p("zk-proof-studio", 0, 0, 62, 62),
            p("key-vault-manager", 64, 0, 30, 30),
            p("deontic-logic-editor", 64, 32, 30, 30),
        ],
        "deontic-editor" => vec![
            p("deontic-logic-editor", 0, 0, 62, 62),
            p("shacl-validator", 64, 0, 30, 30),
            p("agreements-rights", 64, 32, 30, 30),
        ],
        "shacl-validator" => vec![
            p("shacl-validator", 0, 0, 62, 62),
            p("n3-logic-studio", 64, 0, 30, 30),
            p("deontic-logic-editor", 64, 32, 30, 30),
        ],
        "wal-inspector" => vec![
            p("wal-inspector", 0, 0, 62, 36),
            p("provenance-graph", 64, 0, 30, 62),
            p("q42-volume-manager", 0, 38, 62, 24),
        ],
        "q42-volume" => vec![
            p("q42-volume-manager", 0, 0, 62, 36),
            p("wal-inspector", 64, 0, 30, 30),
            p("storage-driver-config", 64, 32, 30, 30),
            p("provenance-graph", 0, 38, 62, 24),
        ],
        "provenance-graph" => vec![
            p("provenance-graph", 0, 0, 62, 62),
            p("wal-inspector", 64, 0, 30, 30),
            p("sparql-explorer", 64, 32, 30, 30),
        ],
        "storage-config" => vec![
            p("storage-driver-config", 0, 0, 56, 40),
            p("system-diagnostics", 58, 0, 36, 40),
            p("wal-inspector", 0, 42, 56, 20),
        ],
        "webtorrent" => vec![
            p("webtorrent-seeder", 0, 0, 62, 36),
            p("p2p-dashboard", 64, 0, 30, 30),
            p("ebpf-filter-manager", 64, 32, 30, 30),
            p("provenance-graph", 0, 38, 62, 24),
        ],
        "p2p-dashboard" => vec![
            p("p2p-dashboard", 0, 0, 62, 62),
            p("webtorrent-seeder", 64, 0, 30, 30),
            p("ebpf-filter-manager", 64, 32, 30, 30),
        ],
        "ebpf-filter" => vec![
            p("ebpf-filter-manager", 0, 0, 62, 62),
            p("p2p-dashboard", 64, 0, 30, 30),
            p("system-diagnostics", 64, 32, 30, 30),
        ],
        "acoustic-ble" => vec![
            p("acoustic-ble-mesh", 0, 0, 62, 62),
            p("p2p-dashboard", 64, 0, 30, 30),
            p("ebpf-filter-manager", 64, 32, 30, 30),
        ],
        "mcp-inspector" => vec![
            p("mcp-inspector", 0, 0, 62, 62),
            p("system-diagnostics", 64, 0, 30, 30),
            p("benchmark-harness", 64, 32, 30, 30),
        ],
        "benchmark" => vec![
            p("benchmark-harness", 0, 0, 62, 62),
            p("inference-monitor", 64, 0, 30, 30),
            p("system-diagnostics", 64, 32, 30, 30),
        ],
        "cli-bridge" => vec![
            p("cli-bridge", 0, 0, 62, 62),
            p("mcp-inspector", 64, 0, 30, 30),
            p("system-diagnostics", 64, 32, 30, 30),
        ],
        "nexus" => vec![
            p("nexus-canvas", 0, 0, 62, 62),
            p("sparql-explorer", 64, 0, 30, 30),
            p("provenance-graph", 64, 32, 30, 30),
        ],
        "extension-bus" => vec![
            p("extension-bus", 0, 0, 56, 40),
            p("system-diagnostics", 58, 0, 36, 30),
            p("mcp-inspector", 58, 32, 36, 30),
        ],
        "profile-identity" => vec![
            p("contextual-workspace", 0, 0, 62, 62),
            p("key-vault-manager", 64, 0, 30, 30),
            p("agreements-rights", 64, 32, 30, 30),
        ],
        "hardware-config" => vec![
            p("system-diagnostics", 0, 0, 56, 40),
            p("storage-driver-config", 58, 0, 36, 30),
            p("agent-config", 58, 32, 36, 30),
            p("benchmark-harness", 0, 42, 56, 20),
        ],
        _ => vec![p(app_id, 0, 0, 94, 62)],
    }
}

fn default_presentation_mode(app_id: Option<&str>) -> PresentationMode {
    match app_id {
        Some("physics-simulator") => PresentationMode::Spatial,
        _ => PresentationMode::GridBound,
    }
}

// ─────────────────────────────────────────────────────────────
// The main DynamicPage component
// ─────────────────────────────────────────────────────────────

#[component]
pub fn DynamicPage(path: Vec<String>, #[props(default)] app_id: Option<String>) -> Element {
    let mock_quin = use_signal(|| NQuin {
        subject: 0,
        predicate: 0,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    });

    let app_id_init = app_id.clone();
    let initial_workspace = {
        let (name, panes) = match app_id_init.as_deref() {
            Some(aid) => (
                app_display_name(aid).to_string(),
                default_panes_for_app(aid),
            ),
            None => ("New Canvas".to_string(), vec![]),
        };
        let mut ws = new_workspace_shell(name, panes);
        if let Some(page) = ws.pages.first_mut() {
            page.presentation_mode = default_presentation_mode(app_id_init.as_deref());
        }
        ws
    };
    let mut workspace = use_signal({
        let initial = initial_workspace.clone();
        move || initial
    });
    let mut history = use_signal({
        let initial = initial_workspace.clone();
        move || WorkspaceHistory::new(initial)
    });
    let selected_pane_index = use_signal(|| None::<usize>);
    let editor_mode = use_signal(|| CanvasEditorMode::Edit);
    let pane_interaction = use_signal(|| None::<PaneInteraction>);
    let drag_anchor = use_signal(|| (0.0_f64, 0.0_f64));
    let canvas_extent = use_signal(|| (800.0_f64, 520.0_f64));
    let pane_palette = use_signal(builtin_pane_definitions);
    let selection_spring = use_signal(|| Spring::new(0.0));
    let selection_scale = use_signal(|| 1.0_f64);
    let mode_pulse_spring = use_signal(|| Spring::new(0.0));
    let mode_pulse_scale = use_signal(|| 0.0_f64);
    let save_status = use_signal(|| String::new());
    let mut deploy_revision = use_signal(|| Option::<u64>::None);
    let mut deploy_history = use_signal(Vec::<DeployHistoryRow>::new);
    let replay_status = use_signal(|| String::new());
    let replaying_revision = use_signal(|| None::<u64>);
    let mut motion_theme_class = use_signal(|| String::new());
    let mut pane_prompt = use_signal(|| String::new());
    let mut generate_status = use_signal(|| String::new());
    let generate_nonce = use_signal(|| 0u32);
    let mut telemetry_logs = use_signal(Vec::<String>::new);
    let global_theme = consume_context::<Signal<ResolvedTheme>>();

    use_effect(move || {
        let global_class = global_theme().class_name.clone();
        let ws = workspace.read().clone();
        let catalog = if ws.themes.is_empty() {
            builtin_theme_catalog()
        } else {
            ws.themes.clone()
        };
        let app = resolve_theme(Some(&ws.app_theme), &catalog);
        let env = resolve_theme(Some(&ws.environment_theme), &catalog);
        let class = global_class
            .or(app.class_name)
            .or(env.class_name)
            .unwrap_or_else(|| "theme-fiduciary-dark".to_string());
        motion_theme_class.set(class);
    });

    let generate_path = format!("/{}", path.join("/"));
    use_effect({
        let generate_path = generate_path.clone();
        move || {
            if generate_nonce() == 0 {
                return;
            }
            let prompt = pane_prompt();
            if prompt.trim().is_empty() {
                generate_status.set("Describe a pane layout first.".to_string());
                return;
            }

            let palette_snapshot = pane_palette.read().clone();
            let palette_ids: Vec<String> = palette_snapshot
                .iter()
                .map(|d| d.component_id.clone())
                .collect();
            let path_for_apply = generate_path.clone();

            let mut apply_plan = move |plan: pane_generator::PaneGenerationPlan| {
                history.write().push(workspace.read().clone());
                let mut ws = workspace.write();
                if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path_for_apply) {
                    page.panes = plan.panes;
                    page.presentation_mode = plan.presentation;
                }
                generate_status.set(plan.summary);
            };

            if crate::endpoints::is_native_host() {
                generate_status.set("Generating layout…".to_string());
                spawn(async move {
                    let plan = match pane_generator::fetch_plan_from_prompt(&prompt, &palette_ids)
                        .await
                    {
                        Ok(plan) => plan,
                        Err(_) => {
                            pane_generator::generate_panes_from_prompt(&prompt, &palette_snapshot)
                        }
                    };
                    apply_plan(plan);
                });
            } else {
                let plan = pane_generator::generate_panes_from_prompt(&prompt, &palette_snapshot);
                apply_plan(plan);
            }
        }
    });

    use_effect(move || {
        let mut selection_spring = selection_spring;
        let mut selection_scale = selection_scale;
        let mut mode_pulse_spring = mode_pulse_spring;
        let mut mode_pulse_scale = mode_pulse_scale;
        let selected_pane_index = selected_pane_index;
        let motion_theme_class = motion_theme_class;
        let global_theme = global_theme;
        spawn_ui_motion_loop(move |dt| {
            let theme_class = motion_theme_class.read().clone();
            let theme_ref = if theme_class.is_empty() {
                None
            } else {
                Some(theme_class.as_str())
            };
            let selected = selected_pane_index.read().is_some();
            let scale =
                theme_selection_pulse(&mut selection_spring.write(), selected, &global_theme());
            selection_scale.set(scale);
            let pulse = step_mode_pulse_spring(&mut mode_pulse_spring.write(), theme_ref, dt);
            mode_pulse_scale.set(pulse);
        });
    });

    #[cfg(not(target_arch = "wasm32"))]
    let mut spatial_contract_nodes =
        use_signal(|| crate::render::render_stack_revision().saturating_sub(1));
    #[cfg(target_arch = "wasm32")]
    let spatial_contract_nodes =
        use_signal(|| crate::render::render_stack_revision().saturating_sub(1));

    #[cfg(not(target_arch = "wasm32"))]
    use_effect({
        let generate_path = generate_path.clone();
        move || {
            let ws = workspace.read().clone();
            let page_path = generate_path.clone();
            let page = ws
                .pages
                .iter()
                .find(|p| p.url_path == page_path || page_path == "/")
                .or_else(|| ws.pages.first());
            if let Some(page) = page {
                let contract = crate::render::render_contract_from_panes(&page.panes);
                spatial_contract_nodes.set(contract.element_count());
                let draw_count = crate::render::rasterize_scene_draw_count(&page.panes);
                let (_tensor_count, opacity) = crate::render::tensor_buffer_digest(&[]);
                telemetry_logs.write().push(format!(
                "Spatial preview: {} elements, {draw_count} draw ops (tensor opacity {opacity:.2})",
                contract.element_count()
            ));
                let panes_snapshot = page.panes.clone();
                spawn(async move {
                    if let Some(png_len) =
                        crate::render::native_headless_png_byte_len(&panes_snapshot, 960, 540).await
                    {
                        telemetry_logs
                            .write()
                            .push(format!("Native GPU preview frame: {png_len} bytes"));
                    }
                });
            }
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        if !crate::endpoints::is_native_host() {
            return;
        }
        for surface in crate::endpoints::all_host_surfaces() {
            telemetry_logs.write().push(format!(
                "Surface: {}",
                crate::endpoints::host_surface_label(surface)
            ));
        }
        spawn(async move {
            if let Ok(resp) = reqwest::get(crate::endpoints::telemetry_url()).await {
                if let Ok(txt) = resp.text().await {
                    if !txt.trim().is_empty() {
                        telemetry_logs.write().push(txt);
                    }
                }
            }
        });
    });

    // ── Boot Rehydration ───────────────────────────────────
    // Only talk to the local daemon when one can exist (native / Tauri webview).
    // In the plain-browser demo this would just spam connection-refused errors.
    use_effect(move || {
        if !crate::endpoints::is_native_host() {
            return;
        }
        spawn(async move {
            if let Ok(res) = reqwest::get(crate::endpoints::manifest_url()).await {
                if let Ok(data) = res.json::<WebizenWorkspace>().await {
                    if !data.pages.is_empty() {
                        workspace.set(data.clone());
                        history.set(WorkspaceHistory::new(data));
                    }
                }
            }
            #[derive(Deserialize)]
            struct UndoChainResponse {
                manifests: Vec<WebizenWorkspace>,
            }
            if let Ok(res) = reqwest::get(crate::endpoints::manifest_undo_chain_url()).await {
                if res.status().is_success() {
                    if let Ok(chain) = res.json::<UndoChainResponse>().await {
                        if let Some(h) = WorkspaceHistory::from_manifest_entries(chain.manifests) {
                            let current = h.current().clone();
                            workspace.set(current);
                            history.set(h);
                        }
                    }
                }
            }
            if let Ok(res) = reqwest::get(crate::endpoints::manifest_history_url()).await {
                if res.status().is_success() {
                    if let Ok(rows) = res.json::<Vec<DeployHistoryRow>>().await {
                        if let Some(last) = rows.last() {
                            deploy_revision.set(Some(last.revision));
                        }
                        deploy_history.set(rows);
                    }
                }
            }
        });
    });

    // ── Native Handshake Probe ─────────────────────────────
    let mut is_native_llm_active = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        // Skip the native-LLM probe entirely outside a daemon-capable host.
        if !crate::endpoints::is_native_host() {
            return;
        }
        if let Ok(ws) = web_sys::WebSocket::new(crate::endpoints::NATIVE_WS) {
            let onopen = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                is_native_llm_active.set(true);
            }) as Box<dyn FnMut(web_sys::Event)>);
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        let ws_url = crate::endpoints::native_handshake_ws();
        let reachable = crate::endpoints::probe_native_handshake_port();
        is_native_llm_active.set(reachable);
        if !reachable {
            telemetry_logs
                .write()
                .push(format!("Native LLM handshake offline ({ws_url})"));
        }
    });

    // 🔴🔴 Telemetry SSE 🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if !crate::endpoints::is_native_host() {
            return;
        }
        if let Ok(es) = EventSource::new(&crate::endpoints::telemetry_url()) {
            let callback = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(txt) = e.data().as_string() {
                    telemetry_logs.write().push(txt.clone());
                    if telemetry_logs.read().len() > 10 {
                        telemetry_logs.write().remove(0);
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);

            es.set_onmessage(Some(callback.as_ref().unchecked_ref()));
            callback.forget();
        }
    });

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if !crate::endpoints::is_native_host() {
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            let callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |event: JsValue| {
                let payload = js_sys::Reflect::get(&event, &JsValue::from_str("payload"));
                if let Ok(payload) = payload {
                    if let Ok(hw) = serde_wasm_bindgen::from_value::<HardwareTelemetry>(payload) {
                        let line = format!("CPU {} | RAM {}", hw.cpu, hw.ram);
                        telemetry_logs.write().push(line);
                        if telemetry_logs.read().len() > 10 {
                            telemetry_logs.write().remove(0);
                        }
                    }
                }
            }));
            if tauri_listen("hardware-telemetry", callback.as_ref().unchecked_ref())
                .await
                .is_ok()
            {
                callback.forget();
            }
        });
    });

    // ── Drag-and-Drop: handle drop on the canvas ───────────
    let on_canvas_drop = {
        let mut workspace = workspace.clone();
        move |evt: Event<DragData>| {
            evt.prevent_default();
            // Retrieve the component_id that was set in drag_start
            let dt = evt.data().data_transfer();
            if let Some(component_id) = dt.get_data("application/x-qualia-pane-id") {
                if !component_id.is_empty() {
                    let snapshot = workspace.read().clone();
                    history.write().push(snapshot);
                    let mut ws = workspace.write();
                    if let Some(page) = ws.pages.first_mut() {
                        if let Some(new_pane) =
                            build_pane_placement(&component_id, page.panes.len())
                        {
                            page.panes.push(new_pane);
                        }
                    }
                }
            }
        }
    };

    let on_canvas_dragover = |evt: Event<DragData>| {
        evt.prevent_default();
    };

    let record_workspace = {
        let mut history = history.clone();
        let workspace = workspace.clone();
        move || {
            history.write().push(workspace.read().clone());
            if crate::endpoints::is_native_host() {
                let ws = workspace.read().clone();
                let stack_index = history.read().stack_index();
                spawn(async move {
                    if let Ok(json) = serde_json::to_string(&ws) {
                        let url = crate::endpoints::manifest_undo_frame_url(stack_index);
                        let _ = reqwest::Client::new()
                            .post(url)
                            .header("Content-Type", "application/json")
                            .body(json)
                            .send()
                            .await;
                    }
                });
            }
        }
    };

    let undo_workspace = {
        let mut history = history.clone();
        let mut workspace = workspace.clone();
        move |_| {
            if let Some(ws) = history.write().undo() {
                workspace.set(ws);
            }
        }
    };

    let redo_workspace = {
        let mut history = history.clone();
        let mut workspace = workspace.clone();
        move |_| {
            if let Some(ws) = history.write().redo() {
                workspace.set(ws);
            }
        }
    };

    let toggle_editor_mode = {
        let mut editor_mode = editor_mode.clone();
        move |_| {
            editor_mode.set(match editor_mode() {
                CanvasEditorMode::Edit => CanvasEditorMode::Preview,
                CanvasEditorMode::Preview => CanvasEditorMode::Edit,
            });
        }
    };

    let current_path_for_mode = format!("/{}", path.join("/"));
    let pulse_toolbar = {
        let mut mode_pulse_spring = mode_pulse_spring.clone();
        move || trigger_mode_pulse(&mut mode_pulse_spring.write())
    };
    let switch_grid_mode = {
        let mut workspace = workspace.clone();
        let mut history = history.clone();
        let current_path = current_path_for_mode.clone();
        let mut pulse_toolbar = pulse_toolbar.clone();
        move |_| {
            pulse_toolbar();
            history.write().push(workspace.read().clone());
            let mut ws = workspace.write();
            if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == current_path) {
                page.presentation_mode = PresentationMode::GridBound;
            }
        }
    };
    let switch_node_mode = {
        let mut workspace = workspace.clone();
        let mut history = history.clone();
        let current_path = current_path_for_mode.clone();
        let mut pulse_toolbar = pulse_toolbar.clone();
        move |_| {
            pulse_toolbar();
            history.write().push(workspace.read().clone());
            let mut ws = workspace.write();
            if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == current_path) {
                page.presentation_mode = PresentationMode::NodeRelational;
            }
        }
    };
    let switch_spatial_mode = {
        let mut workspace = workspace.clone();
        let mut history = history.clone();
        let current_path = current_path_for_mode;
        let mut pulse_toolbar = pulse_toolbar.clone();
        move |_| {
            pulse_toolbar();
            history.write().push(workspace.read().clone());
            let mut ws = workspace.write();
            if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == current_path) {
                page.presentation_mode = PresentationMode::Spatial;
            }
        }
    };

    let finish_interaction = {
        let mut pane_interaction = pane_interaction.clone();
        let mut record_workspace = record_workspace.clone();
        move || {
            if pane_interaction().is_some() {
                record_workspace();
                pane_interaction.set(None);
            }
        }
    };

    let on_canvas_mousemove = {
        let mut workspace = workspace.clone();
        let pane_interaction = pane_interaction.clone();
        let drag_anchor = drag_anchor.clone();
        let canvas_extent = canvas_extent.clone();
        let current_path = format!("/{}", path.join("/"));
        move |evt: Event<MouseData>| {
            if editor_mode() != CanvasEditorMode::Edit {
                return;
            }
            let Some(interaction) = pane_interaction() else {
                return;
            };
            let coords = evt.data().client_coordinates();
            let (ax, ay) = drag_anchor();
            let (cw, ch) = canvas_extent();
            let mut ws = workspace.write();
            let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == current_path) else {
                return;
            };
            let (grid_w, grid_h, snap) = grid_metrics(page);
            let (dx, dy) =
                pixel_delta_to_grid(coords.x - ax, coords.y - ay, cw, ch, grid_w, grid_h);
            match interaction {
                PaneInteraction::Drag {
                    idx,
                    orig_x,
                    orig_y,
                } => {
                    if let Some(pane) = page.panes.get_mut(idx) {
                        let (nx, ny) = clamp_pane_origin(
                            orig_x as i32 + dx,
                            orig_y as i32 + dy,
                            pane.w,
                            pane.h,
                            grid_w,
                            grid_h,
                        );
                        pane.x = snap_u16(nx, snap);
                        pane.y = snap_u16(ny, snap);
                    }
                }
                PaneInteraction::Resize {
                    idx,
                    orig_w,
                    orig_h,
                } => {
                    if let Some(pane) = page.panes.get_mut(idx) {
                        let (nw, nh) = clamp_pane_size(
                            orig_w as i32 + dx,
                            orig_h as i32 + dy,
                            pane.x,
                            pane.y,
                            grid_w,
                            grid_h,
                        );
                        pane.w = snap_u16(nw, snap);
                        pane.h = snap_u16(nh, snap);
                    }
                }
            }
        }
    };

    let on_canvas_mouseup = {
        let mut finish_interaction = finish_interaction.clone();
        move |_: Event<MouseData>| {
            finish_interaction();
        }
    };

    // ── Persist workspace to local settings portal (survives restart) ──
    let save_workspace = {
        let workspace = workspace.clone();
        let mut save_status = save_status.clone();
        let mut deploy_revision = deploy_revision.clone();
        let mut deploy_history = deploy_history.clone();
        move |_| {
            save_status.set("Saving…".to_string());
            spawn(async move {
                let current_workspace = workspace.read().clone();
                let payload = match serde_json::to_string(&current_workspace) {
                    Ok(json) => json,
                    Err(err) => {
                        save_status.set(format!("Serialize failed: {err}"));
                        return;
                    }
                };

                let client = reqwest::Client::new();
                match client
                    .post(crate::endpoints::manifest_url())
                    .header("Content-Type", "application/json")
                    .body(payload)
                    .send()
                    .await
                {
                    Ok(res) if res.status().is_success() => {
                        if let Ok(history_res) = client
                            .get(crate::endpoints::manifest_history_url())
                            .send()
                            .await
                        {
                            if history_res.status().is_success() {
                                if let Ok(rows) = history_res.json::<Vec<DeployHistoryRow>>().await
                                {
                                    deploy_history.set(rows.clone());
                                    if let Some(last) = rows.last() {
                                        deploy_revision.set(Some(last.revision));
                                        save_status.set(format!(
                                            "Saved · WAL rev #{} ({} pane quins)",
                                            last.revision, last.pane_count
                                        ));
                                        return;
                                    }
                                }
                            }
                        }
                        save_status.set("Workspace saved locally.".to_string());
                    }
                    Ok(res) => {
                        save_status.set(format!("Save failed ({})", res.status()));
                    }
                    Err(err) => {
                        save_status.set(format!("Save unreachable: {err}"));
                    }
                }
            });
        }
    };

    let apply_ontology_layout = {
        let mut workspace = workspace.clone();
        let mut history = history.clone();
        let current_path = format!("/{}", path.join("/"));
        move |suggestion: OntologyLayoutSuggestion| {
            history.write().push(workspace.read().clone());
            let mut ws = workspace.write();
            if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == current_path) {
                page.panes = suggestion.panes;
                page.presentation_mode = suggestion.presentation;
                page.name = suggestion.label;
            }
        }
    };

    let restore_deploy_revision = {
        let mut workspace = workspace.clone();
        let mut history = history.clone();
        let mut replay_status = replay_status.clone();
        let mut replaying_revision = replaying_revision.clone();
        let mut deploy_revision = deploy_revision.clone();
        move |revision: u64| {
            replaying_revision.set(Some(revision));
            replay_status.set(format!("Restoring WAL rev #{revision}…"));
            spawn(async move {
                let client = reqwest::Client::new();
                let url = crate::endpoints::manifest_replay_url(revision);
                match client.post(&url).send().await {
                    Ok(res) if res.status().is_success() => {
                        match res.json::<WebizenWorkspace>().await {
                            Ok(data) => {
                                workspace.set(data.clone());
                                history.set(WorkspaceHistory::new(data));
                                deploy_revision.set(Some(revision));
                                replay_status
                                    .set(format!("Restored WAL rev #{revision} into the editor."));
                            }
                            Err(err) => {
                                replay_status.set(format!("Restore parse failed: {err}"));
                            }
                        }
                    }
                    Ok(res) => {
                        replay_status.set(format!("Restore failed ({})", res.status()));
                    }
                    Err(err) => {
                        replay_status.set(format!("Restore unreachable: {err}"));
                    }
                }
                replaying_revision.set(None);
            });
        }
    };

    let select_pane_from_graph = {
        let mut selected_pane_index = selected_pane_index.clone();
        move |idx: usize| selected_pane_index.set(Some(idx))
    };

    let add_companion_pane = {
        let mut workspace = workspace.clone();
        let mut history = history.clone();
        let current_path = format!("/{}", path.join("/"));
        move |component_id: String| {
            history.write().push(workspace.read().clone());
            let mut ws = workspace.write();
            if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == current_path) {
                if let Some(pane) = build_pane_placement(&component_id, page.panes.len()) {
                    page.panes.push(pane);
                }
            }
        }
    };

    // ── Delete selected pane ───────────────────────────────
    let delete_selected_pane = {
        let mut workspace = workspace.clone();
        let mut selected_pane_index = selected_pane_index.clone();
        let mut record_workspace = record_workspace.clone();
        move |_| {
            let idx_opt = *selected_pane_index.read();
            if let Some(idx) = idx_opt {
                record_workspace();
                let mut ws = workspace.write();
                if let Some(page) = ws.pages.first_mut() {
                    if idx < page.panes.len() {
                        page.panes.remove(idx);
                    }
                }
                selected_pane_index.set(None);
            }
        }
    };

    // ── Routing ────────────────────────────────────────────
    let current_path = format!("/{}", path.join("/"));
    let ws = workspace.read();
    let current_page = ws
        .pages
        .iter()
        .find(|p| p.url_path == current_path)
        .cloned();
    let theme_catalog = if ws.themes.is_empty() {
        builtin_theme_catalog()
    } else {
        ws.themes.clone()
    };
    let mut environment_theme = resolve_theme(Some(&ws.environment_theme), &theme_catalog);
    environment_theme.tokens.extend(ws.theme_tokens.clone());
    let app_theme = resolve_theme(Some(&ws.app_theme), &theme_catalog);
    let page_theme = current_page
        .as_ref()
        .map(|page| resolve_theme(Some(&page.theme), &theme_catalog))
        .unwrap_or_default();
    let pane_themes = current_page
        .as_ref()
        .map(|page| {
            page.panes
                .iter()
                .map(|pane| resolve_theme(Some(&pane.theme), &theme_catalog))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let all_themes = std::iter::once(&environment_theme)
        .chain(std::iter::once(&app_theme))
        .chain(std::iter::once(&page_theme))
        .chain(pane_themes.iter())
        .collect::<Vec<_>>();
    let theme_stylesheets = collect_stylesheets(&all_themes);
    let mut theme_css = String::new();
    if let Some(css) = render_scope_tokens(":root", &environment_theme) {
        theme_css.push_str(&css);
    }
    if let Some(css) = render_scope_tokens(".webizen-studio-shell", &app_theme) {
        theme_css.push_str(&css);
    }
    if let Some(css) = render_scope_tokens(".webizen-page-shell", &page_theme) {
        theme_css.push_str(&css);
    }
    for (idx, pane_theme) in pane_themes.iter().enumerate() {
        if let Some(css) = render_scope_tokens(
            &format!(".webizen-module-pane[data-pane-index='{}']", idx),
            pane_theme,
        ) {
            theme_css.push_str(&css);
        }
    }

    let can_undo = history.read().can_undo();
    let can_redo = history.read().can_redo();
    let undo_opacity = if can_undo { "1" } else { "0.4" };
    let redo_opacity = if can_redo { "1" } else { "0.4" };
    let edit_mode_bg = if editor_mode() == CanvasEditorMode::Edit {
        "rgba(245,158,11,0.15)"
    } else {
        "var(--qualia-surface)"
    };
    let grid_mode_active = current_page
        .as_ref()
        .map(|p| p.presentation_mode == PresentationMode::GridBound)
        .unwrap_or(false);
    let node_mode_active = current_page
        .as_ref()
        .map(|p| p.presentation_mode == PresentationMode::NodeRelational)
        .unwrap_or(false);
    let spatial_mode_active = current_page
        .as_ref()
        .map(|p| p.presentation_mode == PresentationMode::Spatial)
        .unwrap_or(false);
    let mode_btn_style = |active: bool| {
        if active {
            "padding: 0.25rem 0.6rem; font-size: 0.68rem; border-radius: 999px; border: 1px solid var(--qualia-accent); background: rgba(245,158,11,0.12); color: var(--qualia-text); cursor: pointer;"
        } else {
            "padding: 0.25rem 0.6rem; font-size: 0.68rem; border-radius: 999px; border: 1px solid var(--qualia-border); background: transparent; color: var(--qualia-text); cursor: pointer;"
        }
    };
    let grid_btn_style = mode_btn_style(grid_mode_active);
    let node_btn_style = mode_btn_style(node_mode_active);
    let spatial_btn_style = mode_btn_style(spatial_mode_active);
    let toolbar_pulse_scale = 1.0 + mode_pulse_scale();
    let toolbar_pulse_style =
        format!("transform: scale({toolbar_pulse_scale:.4}); transform-origin: right center;");

    // ── Create New Page ────────────────────────────────────
    let create_new_page = {
        let mut workspace = workspace.clone();
        move |_| {
            let mut ws = workspace.write();
            let p_len = ws.pages.len() + 1;
            ws.pages.push(Page {
                url_path: format!("/page-{}", p_len),
                name: format!("New Page {}", p_len),
                layout_strategy: LayoutStrategy::default(),
                panes: vec![],
                presentation_mode: PresentationMode::GridBound,
                coordinate_space: CoordinateSpace::GlobalCartesian,
                pan_and_zoom: true,
                theme: ThemeBinding::default(),
            });
        }
    };

    rsx! {
        for href in theme_stylesheets.iter() {
            document::Link { rel: "stylesheet", href: "{href}" }
        }

        style { "{theme_css}{qprime_elevation_css()}" }

        // Template picker — shown when no app is selected (empty canvas)
        {if app_id.is_none() && workspace.read().pages.iter().all(|p| p.panes.is_empty()) {
            let templates: &[&'static str] = &[
                "context-studio", "chat", "llm-harness", "lora-manager",
                "agent-config", "inference-monitor", "model-lifecycle",
                "ontology-builder", "sparql-explorer", "n3-logic-studio",
                "rdf-star-editor", "solid-browser", "physics-sim",
                "chemistry-modeler", "ode-lab", "matrix-lab", "stats-lab",
                "bioinformatics-lab", "qpu-optimizer", "quantum-dft",
                "qaoa-explorer", "qpu-providers", "health-vitals",
                "clinical-risk", "dicom-viewer", "anatomy-browser",
                "comorbidity", "portfolio", "risk-engine", "gbm-sim",
                "tax-schema", "agreements", "key-vault", "zk-studio",
                "deontic-editor", "shacl-validator", "wal-inspector",
                "q42-volume", "provenance-graph", "storage-config",
                "webtorrent", "p2p-dashboard", "ebpf-filter",
                "acoustic-ble", "mcp-inspector", "benchmark",
                "cli-bridge", "extension-bus", "nexus",
            ];
            rsx! {
                div {
                    style: "flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:1.5rem;padding:2rem;max-width:1100px;margin:0 auto;",
                    h1 { style: "font-size:1.6rem;margin:0;", "QApp Studio" }
                    p { style: "color:var(--qualia-text-muted,#888);margin:0;", "Select a template to start building, or create a blank canvas." }
                    div {
                        style: "display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:0.75rem;width:100%;",
                        for tmpl in templates.iter() {
                            {let tmpl = tmpl.to_string();
                            rsx! {
                                button {
                                    key: "{tmpl}",
                                    r#type: "button",
                                    onclick: {
                                        let mut ws = workspace.clone();
                                        move |_| {
                                            let name = app_display_name(&tmpl).to_string();
                                            let panes = default_panes_for_app(&tmpl);
                                            let mut new_ws = new_workspace_shell(name, panes);
                                            if let Some(page) = new_ws.pages.first_mut() {
                                                page.presentation_mode = default_presentation_mode(Some(&tmpl));
                                            }
                                            ws.set(new_ws);
                                        }
                                    },
                                    style: "padding:1rem;border:1px solid var(--qualia-border,#333);border-radius:10px;background:var(--qualia-surface,#1a1a1a);cursor:pointer;text-align:left;transition:border-color 0.2s;",
                                    onmouseenter: move |e| e.stop_propagation(),
                                    div {
                                        style: "font-size:0.9rem;font-weight:600;margin-bottom:0.25rem;",
                                        {app_display_name(&tmpl)}
                                    }
                                    div {
                                        style: "font-size:0.75rem;color:var(--qualia-text-muted,#888);",
                                        "{tmpl}"
                                    }
                                }
                            }}
                        }
                    }
                    button {
                        r#type: "button",
                        onclick: {
                            let mut ws = workspace.clone();
                            move |_| {
                                let mut new_ws = new_workspace_shell("Blank Canvas".to_string(), vec![]);
                                if let Some(page) = new_ws.pages.first_mut() {
                                    page.presentation_mode = PresentationMode::GridBound;
                                }
                                ws.set(new_ws);
                            }
                        },
                        style: "padding:0.6rem 1.5rem;border:1px dashed var(--qualia-border,#555);border-radius:8px;background:transparent;color:var(--qualia-text-muted,#aaa);cursor:pointer;",
                        "+ Blank Canvas"
                    }
                }
            }
        } else {
            rsx! {}
        }}

        div {
            class: "{join_theme_classes(\"webizen-studio-shell\", &app_theme)}",
            "data-theme-scope": "app",
            "data-theme": "{app_theme.theme_key.clone().unwrap_or_default()}",
            style: "flex: 1; display: grid; grid-template-columns: 240px 1fr 280px; gap: 0; height: calc(100vh - 60px);",

            // ════════════════════════════════════════════════
            // LEFT SIDEBAR: Pages + Component Palette
            // ════════════════════════════════════════════════
            div {
                style: "background: var(--qualia-surface, #111); border-right: 1px solid var(--qualia-border, #333); padding: 1rem; overflow-y: auto; display: flex; flex-direction: column; gap: 0.75rem;",

                // Page Navigation
                div {
                    style: "margin-bottom: 0.5rem;",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;",
                        h3 {
                            style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0;",
                            "Pages"
                        }
                        button {
                            style: "background: transparent; border: 1px solid var(--qualia-border, #444); color: var(--qualia-accent, #0ff); padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.65rem; cursor: pointer;",
                            onclick: create_new_page,
                            "+ New"
                        }
                    }
                    for page in ws.pages.iter() {
                        a {
                            href: "{page.url_path}",
                            style: "display: block; padding: 0.4rem 0.6rem; color: var(--qualia-accent, #0ff); text-decoration: none; border-radius: 4px; font-size: 0.85rem; transition: background 0.15s;",
                            onmouseenter: |_| {},
                            "{page.name}"
                        }
                    }
                }

                OntologyImportWizard {
                    on_apply: apply_ontology_layout,
                }

                // Component Palette — draggable items grouped by category
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Components"
                    }

                    // Group by category
                    {render_palette_category(&pane_palette.read(), PaneCategory::Computational)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Intelligence)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Knowledge)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Governance)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Network)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Data)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::DataDisplay)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::DataInput)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Layout)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::Media)}
                    {render_palette_category(&pane_palette.read(), PaneCategory::System)}
                }
            }

            // ════════════════════════════════════════════════
            // CENTER: The Rendered Page Canvas (Drop Target)
            // ════════════════════════════════════════════════
            div {
                style: "background: var(--qualia-bg, #0a0a0a); padding: 1rem; overflow-y: auto;",
                ondrop: on_canvas_drop,
                ondragover: on_canvas_dragover,

                if let Some(page) = current_page.clone() {
                    div {
                        class: "{join_theme_classes(\"webizen-page-shell\", &page_theme)}",
                        "data-theme-scope": "page",
                        "data-theme": "{page_theme.theme_key.clone().unwrap_or_default()}",

                        // Editor toolbar + page title
                        div {
                            class: "webizen-canvas-toolbar",
                            style: "margin-bottom: 0.75rem; display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 0.6rem;",
                            div {
                                style: "display: flex; align-items: center; gap: 0.45rem; flex-wrap: wrap;",
                                h2 {
                                    style: "margin: 0; font-size: 1.1rem; color: var(--qualia-text, #eee); white-space: nowrap;",
                                    "{page.name}"
                                }
                                button {
                                    style: "padding: 0.25rem 0.55rem; font-size: 0.68rem; border-radius: 6px; border: 1px solid var(--qualia-border); background: var(--qualia-surface); color: var(--qualia-text); cursor: pointer; opacity: {undo_opacity};",
                                    disabled: !can_undo,
                                    onclick: undo_workspace,
                                    "Undo"
                                }
                                button {
                                    style: "padding: 0.25rem 0.55rem; font-size: 0.68rem; border-radius: 6px; border: 1px solid var(--qualia-border); background: var(--qualia-surface); color: var(--qualia-text); cursor: pointer; opacity: {redo_opacity};",
                                    disabled: !can_redo,
                                    onclick: redo_workspace,
                                    "Redo"
                                }
                                button {
                                    style: "padding: 0.25rem 0.65rem; font-size: 0.68rem; border-radius: 6px; border: 1px solid var(--qualia-accent); background: {edit_mode_bg}; color: var(--qualia-text); cursor: pointer;",
                                    onclick: toggle_editor_mode,
                                    if editor_mode() == CanvasEditorMode::Edit { "Edit mode" } else { "Preview mode" }
                                }
                            }
                            div {
                                style: "display: flex; gap: 0.35rem; flex-wrap: wrap; {toolbar_pulse_style}",
                                button {
                                    style: "{grid_btn_style}",
                                    onclick: switch_grid_mode,
                                    "Grid"
                                }
                                button {
                                    style: "{node_btn_style}",
                                    onclick: switch_node_mode,
                                    "Nodes"
                                }
                                button {
                                    style: "{spatial_btn_style}",
                                    onclick: switch_spatial_mode,
                                    "Spatial"
                                }
                            }
                        }

                        div {
                            style: "margin-bottom: 1rem; display: flex; justify-content: space-between; align-items: center; gap: 1rem;",

                            // D2.1: Keyword pane planner (prompt bar)
                            div {
                                style: "flex: 1; display: flex; flex-direction: column; gap: 0.35rem;",
                                div {
                                    style: "display: flex; gap: 0.5rem;",
                                    input {
                                        r#type: "text",
                                        value: "{pane_prompt()}",
                                        placeholder: "Describe a pane layout (e.g. 'Health tracker with chart and inputs')...",
                                        style: "flex: 1; background: var(--qualia-surface, #222); border: 1px solid var(--qualia-border, #444); color: white; padding: 0.4rem 0.6rem; border-radius: 4px; font-size: 0.85rem;",
                                        oninput: move |e: Event<FormData>| pane_prompt.set(e.value()),
                                        onkeydown: {
                                            let mut generate_nonce = generate_nonce.clone();
                                            move |e: Event<KeyboardData>| {
                                                if e.key() == Key::Enter {
                                                    generate_nonce.set(generate_nonce() + 1);
                                                }
                                            }
                                        },
                                    }
                                    button {
                                        style: "background: var(--qualia-accent, #0ff); color: black; border: none; padding: 0 1rem; border-radius: 4px; font-weight: bold; cursor: pointer;",
                                        onclick: {
                                            let mut generate_nonce = generate_nonce.clone();
                                            move |_| generate_nonce.set(generate_nonce() + 1)
                                        },
                                        "Generate"
                                    }
                                }
                                if !generate_status.read().is_empty() {
                                    span {
                                        style: "font-size: 0.68rem; color: var(--qualia-accent);",
                                        "{generate_status.read()}"
                                    }
                                }
                            }

                            span {
                                style: "font-size: 0.7rem; color: var(--qualia-text-muted, #666); background: var(--qualia-surface, #222); padding: 0.2rem 0.6rem; border-radius: 12px; white-space: nowrap;",
                                "{page.panes.len()} panes"
                            }
                        }

                        div {
                            style: "display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 0.9rem;",
                            span {
                                style: "font-size: 0.68rem; color: var(--qualia-text, #ddd); background: rgba(255,255,255,0.05); border: 1px solid var(--qualia-border, #333); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                "{presentation_mode_label(&page.presentation_mode)}"
                            }
                            span {
                                style: "font-size: 0.68rem; color: var(--qualia-text-muted, #aaa); background: rgba(255,255,255,0.03); border: 1px solid var(--qualia-border, #333); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                "{coordinate_space_label(&page.coordinate_space)}"
                            }
                            span {
                                style: "font-size: 0.68rem; color: var(--qualia-text-muted, #aaa); background: rgba(255,255,255,0.03); border: 1px solid var(--qualia-border, #333); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                "{layout_strategy_label(&page.layout_strategy)}"
                            }
                            if page.pan_and_zoom {
                                span {
                                    style: "font-size: 0.68rem; color: var(--qualia-accent, #0ff); background: rgba(6, 182, 212, 0.08); border: 1px solid rgba(6, 182, 212, 0.35); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                    "Pan/Zoom Ready"
                                }
                            }
                            if spatial_contract_nodes() > 0 {
                                span {
                                    style: "font-size: 0.68rem; color: var(--qualia-accent, #0ff); background: rgba(6, 182, 212, 0.08); border: 1px solid rgba(6, 182, 212, 0.35); padding: 0.2rem 0.55rem; border-radius: 999px;",
                                    "{spatial_contract_nodes()} GPU elements"
                                }
                            }
                        }

                        match page.presentation_mode {
                            PresentationMode::NodeRelational => rsx! {
                                crate::render::node_graph::NodeGraphCanvas { page: page.clone() }
                            },
                            PresentationMode::Spatial => rsx! {
                                crate::render::spatial_bridge::SpatialBridgeCanvas { page: page.clone() }
                            },
                            _ => rsx! {
                                // Workspace canvas
                                div {
                                    style: "{canvas_container_style(&page)}",
                                    onmousemove: on_canvas_mousemove,
                                    onmouseup: on_canvas_mouseup,
                                    onmouseleave: on_canvas_mouseup,

                                    for (idx, pane) in page.panes.iter().enumerate().filter(|(_, pane)| matches!(pane.layer, LayerBehavior::Docked)) {
                                        {render_placed_pane(
                                            &page,
                                            pane,
                                            idx,
                                            &selected_pane_index,
                                            pane_themes.get(idx).cloned().unwrap_or_default(),
                                            editor_mode(),
                                            &pane_interaction,
                                            &drag_anchor,
                                            selection_scale(),
                                        )}
                                    }

                                    if page.panes.iter().any(|pane| !matches!(pane.layer, LayerBehavior::Docked)) {
                                        div {
                                            style: "position: absolute; inset: 0; pointer-events: none;",
                                            for (idx, pane) in page.panes.iter().enumerate().filter(|(_, pane)| !matches!(pane.layer, LayerBehavior::Docked)) {
                                                {render_placed_pane(
                                                    &page,
                                                    pane,
                                                    idx,
                                                    &selected_pane_index,
                                                    pane_themes.get(idx).cloned().unwrap_or_default(),
                                                    editor_mode(),
                                                    &pane_interaction,
                                                    &drag_anchor,
                                                    selection_scale(),
                                                )}
                                            }
                                        }
                                    }

                                    if page.panes.is_empty() {
                                        div {
                                            style: "position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; min-height: 300px; border: 2px dashed var(--qualia-border, #333); border-radius: 12px; color: var(--qualia-text-muted, #555);",
                                            "Drag components from the palette to build your app"
                                        }
                                    }
                                }

                                if page.panes.iter().any(|pane| pane.min_w_points > 0 || pane.min_h_points > 0) {
                                    div {
                                        style: "margin-top: 0.75rem; font-size: 0.73rem; color: var(--qualia-text-muted, #777);",
                                        "Point-grid panes can declare minimum working area and layered behavior independently of their current presentation mode."
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // No page found
                    div {
                        style: "display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 400px; color: var(--qualia-text-muted, #555);",
                        p { "No page mapped to this route." }
                        p { style: "font-size: 0.8rem;", "Navigate to / to see the Human-Centric Dashboard." }
                    }
                }
            }

            // ════════════════════════════════════════════════
            // RIGHT SIDEBAR: Inspector + Telemetry
            // ════════════════════════════════════════════════
            div {
                style: "background: var(--qualia-surface, #111); border-left: 1px solid var(--qualia-border, #333); padding: 1rem; overflow-y: auto; display: flex; flex-direction: column; gap: 1rem;",

                if let Some(page) = current_page.clone() {
                    SelectionSidebar {
                        page: page.clone(),
                        selected_idx: *selected_pane_index.read(),
                        palette: pane_palette.read().clone(),
                        on_select_pane: select_pane_from_graph,
                        on_add_component: add_companion_pane,
                    }
                }

                // Property Inspector
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Property Inspector"
                    }

                    if let Some(idx) = *selected_pane_index.read() {
                        if let Some(page) = ws.pages.iter().find(|page| page.url_path == current_path) {
                            if let Some(pane) = page.panes.get(idx) {
                                div {
                                    style: "background: var(--qualia-bg, #0a0a0a); border-radius: 6px; padding: 0.75rem; font-size: 0.8rem;",
                                    div {
                                        style: "margin-bottom: 0.5rem;",
                                        span { style: "color: var(--qualia-text-muted, #888);", "Component: " }
                                        span { style: "color: var(--qualia-accent, #0ff);", "{pane.component_id}" }
                                    }
                                    div {
                                        style: "display: flex; flex-wrap: wrap; gap: 0.3rem; margin-bottom: 0.55rem;",
                                        span {
                                            style: "font-size: 0.62rem; padding: 0.12rem 0.45rem; border-radius: 999px; background: rgba(245,158,11,0.12); color: var(--qualia-accent); border: 1px solid rgba(245,158,11,0.25);",
                                            "{theme_binding_provenance(&pane.theme)}"
                                        }
                                        if pane.binds_rpc.is_some() {
                                            span {
                                                style: "font-size: 0.62rem; padding: 0.12rem 0.45rem; border-radius: 999px; background: rgba(59,130,246,0.12); color: #93c5fd; border: 1px solid rgba(59,130,246,0.25);",
                                                "Shared via RPC"
                                            }
                                        }
                                        if !pane.data_bindings.is_empty() {
                                            span {
                                                style: "font-size: 0.62rem; padding: 0.12rem 0.45rem; border-radius: 999px; background: rgba(16,185,129,0.1); color: #6ee7b7; border: 1px solid rgba(16,185,129,0.22);",
                                                "Ontology bound"
                                            }
                                        }
                                    }
                                    if editor_mode() == CanvasEditorMode::Edit {
                                        label {
                                            style: "display: block; margin-bottom: 0.5rem; font-size: 0.68rem; color: var(--qualia-text-muted);",
                                            "Theme preset"
                                            select {
                                                value: "{pane.theme.theme_id.clone().unwrap_or_default()}",
                                                style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.25rem;",
                                                onchange: {
                                                    let mut workspace = workspace.clone();
                                                    let mut record_workspace = record_workspace.clone();
                                                    let path = current_path.clone();
                                                    move |e: Event<FormData>| {
                                                        record_workspace();
                                                        let mut ws = workspace.write();
                                                        if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                            if let Some(p) = page.panes.get_mut(idx) {
                                                                let v = e.value();
                                                                p.theme.theme_id = if v.is_empty() {
                                                                    None
                                                                } else {
                                                                    Some(v)
                                                                };
                                                            }
                                                        }
                                                    }
                                                },
                                                option { value: "", "Inherit workspace" }
                                                for qid in theme_engine::QPRIME_PRESET_IDS {
                                                    option {
                                                        value: "{qid}",
                                                        selected: pane.theme.theme_id.as_deref() == Some(*qid),
                                                        "{theme_engine::theme_label(qid)}"
                                                    }
                                                }
                                            }
                                        }
                                        div {
                                            style: "display: grid; grid-template-columns: 1fr 1fr; gap: 0.35rem; margin-bottom: 0.6rem;",
                                            label {
                                                style: "font-size: 0.68rem; color: var(--qualia-text-muted);",
                                                "X"
                                                input {
                                                    r#type: "number",
                                                    value: "{pane.x}",
                                                    style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.2rem;",
                                                    onchange: {
                                                        let mut workspace = workspace.clone();
                                                        let mut record_workspace = record_workspace.clone();
                                                        let path = current_path.clone();
                                                        move |e: Event<FormData>| {
                                                            if let Ok(v) = e.value().parse::<u16>() {
                                                                record_workspace();
                                                                let mut ws = workspace.write();
                                                                if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                                    if let Some(p) = page.panes.get_mut(idx) {
                                                                        p.x = v;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    },
                                                }
                                            }
                                            label {
                                                style: "font-size: 0.68rem; color: var(--qualia-text-muted);",
                                                "Y"
                                                input {
                                                    r#type: "number",
                                                    value: "{pane.y}",
                                                    style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.2rem;",
                                                    onchange: {
                                                        let mut workspace = workspace.clone();
                                                        let mut record_workspace = record_workspace.clone();
                                                        let path = current_path.clone();
                                                        move |e: Event<FormData>| {
                                                            if let Ok(v) = e.value().parse::<u16>() {
                                                                record_workspace();
                                                                let mut ws = workspace.write();
                                                                if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                                    if let Some(p) = page.panes.get_mut(idx) {
                                                                        p.y = v;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    },
                                                }
                                            }
                                            label {
                                                style: "font-size: 0.68rem; color: var(--qualia-text-muted);",
                                                "W"
                                                input {
                                                    r#type: "number",
                                                    value: "{pane.w}",
                                                    style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.2rem;",
                                                    onchange: {
                                                        let mut workspace = workspace.clone();
                                                        let mut record_workspace = record_workspace.clone();
                                                        let path = current_path.clone();
                                                        move |e: Event<FormData>| {
                                                            if let Ok(v) = e.value().parse::<u16>() {
                                                                record_workspace();
                                                                let mut ws = workspace.write();
                                                                if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                                    if let Some(p) = page.panes.get_mut(idx) {
                                                                        p.w = v.max(4);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    },
                                                }
                                            }
                                            label {
                                                style: "font-size: 0.68rem; color: var(--qualia-text-muted);",
                                                "H"
                                                input {
                                                    r#type: "number",
                                                    value: "{pane.h}",
                                                    style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.2rem;",
                                                    onchange: {
                                                        let mut workspace = workspace.clone();
                                                        let mut record_workspace = record_workspace.clone();
                                                        let path = current_path.clone();
                                                        move |e: Event<FormData>| {
                                                            if let Ok(v) = e.value().parse::<u16>() {
                                                                record_workspace();
                                                                let mut ws = workspace.write();
                                                                if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                                    if let Some(p) = page.panes.get_mut(idx) {
                                                                        p.h = v.max(4);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    },
                                                }
                                            }
                                        }
                                    } else {
                                        div {
                                            style: "margin-bottom: 0.5rem;",
                                            span { style: "color: var(--qualia-text-muted, #888);", "Position: " }
                                            span { "({pane.x}, {pane.y}) — {pane.w}×{pane.h}" }
                                        }
                                    }
                                    if editor_mode() == CanvasEditorMode::Edit {
                                        label {
                                            style: "display: block; margin-bottom: 0.5rem; font-size: 0.68rem; color: var(--qualia-text-muted);",
                                            "Layer"
                                            select {
                                                value: "{layer_behavior_value(&pane.layer)}",
                                                style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.25rem;",
                                                onchange: {
                                                    let mut workspace = workspace.clone();
                                                    let mut record_workspace = record_workspace.clone();
                                                    let path = current_path.clone();
                                                    move |e: Event<FormData>| {
                                                        if let Some(layer) = layer_behavior_from_value(&e.value()) {
                                                            record_workspace();
                                                            let mut ws = workspace.write();
                                                            if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                                if let Some(p) = page.panes.get_mut(idx) {
                                                                    p.layer = layer;
                                                                }
                                                            }
                                                        }
                                                    }
                                                },
                                                option { value: "docked", "Docked" }
                                                option { value: "floating", "Floating Overlay" }
                                                option { value: "modal", "Modal Overlay" }
                                                option { value: "full", "Full Canvas" }
                                            }
                                        }
                                        label {
                                            style: "display: block; margin-bottom: 0.5rem; font-size: 0.68rem; color: var(--qualia-text-muted);",
                                            "Anchor (pane id)"
                                            input {
                                                r#type: "text",
                                                value: "{pane.anchor.clone().unwrap_or_default()}",
                                                placeholder: "e.g. sidebar-nav",
                                                style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.2rem;",
                                                onchange: {
                                                    let mut workspace = workspace.clone();
                                                    let mut record_workspace = record_workspace.clone();
                                                    let path = current_path.clone();
                                                    move |e: Event<FormData>| {
                                                        record_workspace();
                                                        let mut ws = workspace.write();
                                                        if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                            if let Some(p) = page.panes.get_mut(idx) {
                                                                let v = e.value().trim().to_string();
                                                                p.anchor = if v.is_empty() { None } else { Some(v) };
                                                            }
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                        label {
                                            style: "display: block; margin-bottom: 0.5rem; font-size: 0.68rem; color: var(--qualia-text-muted);",
                                            "Data bindings (comma-separated)"
                                            input {
                                                r#type: "text",
                                                value: "{pane.data_bindings.join(\", \")}",
                                                placeholder: "sparql:…, n3:…",
                                                style: "width: 100%; margin-top: 0.15rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; padding: 0.2rem;",
                                                onchange: {
                                                    let mut workspace = workspace.clone();
                                                    let mut record_workspace = record_workspace.clone();
                                                    let path = current_path.clone();
                                                    move |e: Event<FormData>| {
                                                        record_workspace();
                                                        let mut ws = workspace.write();
                                                        if let Some(page) = ws.pages.iter_mut().find(|p| p.url_path == path) {
                                                            if let Some(p) = page.panes.get_mut(idx) {
                                                                p.data_bindings = e
                                                                    .value()
                                                                    .split(',')
                                                                    .map(|s| s.trim().to_string())
                                                                    .filter(|s| !s.is_empty())
                                                                    .collect();
                                                            }
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    } else {
                                        div {
                                            style: "margin-bottom: 0.5rem;",
                                            span { style: "color: var(--qualia-text-muted, #888);", "Layer: " }
                                            span { "{layer_behavior_label(&pane.layer)}" }
                                        }
                                        if let Some(anchor) = &pane.anchor {
                                            div {
                                                style: "margin-bottom: 0.5rem;",
                                                span { style: "color: var(--qualia-text-muted, #888);", "Anchor: " }
                                                span { "{anchor}" }
                                            }
                                        }
                                    }
                                    if pane.min_w_points > 0 || pane.min_h_points > 0 {
                                        div {
                                            style: "margin-bottom: 0.5rem;",
                                            span { style: "color: var(--qualia-text-muted, #888);", "Minimum area: " }
                                            span { "{pane.min_w_points}x{pane.min_h_points} points" }
                                        }
                                    }
                                    if !pane.supported_presentations.is_empty() {
                                        div {
                                            style: "margin-bottom: 0.5rem;",
                                            span { style: "color: var(--qualia-text-muted, #888);", "Supported views: " }
                                            span { "{supported_presentations_summary(&pane.supported_presentations)}" }
                                        }
                                    }
                                    if editor_mode() != CanvasEditorMode::Edit {
                                        div {
                                            span { style: "color: var(--qualia-text-muted, #888);", "Bindings: " }
                                            span {
                                                if pane.data_bindings.is_empty() {
                                                    "None"
                                                } else {
                                                    "{pane.data_bindings.join(\", \")}"
                                                }
                                            }
                                        }
                                    }
                                    if let Some(rpc) = &pane.binds_rpc {
                                        div {
                                            style: "margin-top: 0.5rem; padding-top: 0.5rem; border-top: 1px solid var(--qualia-border, #444);",
                                            span { style: "color: var(--qualia-text-muted, #888);", "RPC: " }
                                            span { style: "color: #ffaa00;", "{rpc}" }
                                        }
                                        div {
                                            span { style: "color: var(--qualia-text-muted, #888);", "UI Mode: " }
                                            span {
                                                if pane.ui_mode == Some(UiMode::IFrameSandbox) { "IFrame Sandbox" } else { "Native Dioxus" }
                                            }
                                        }
                                    }
                                }

                                button {
                                    style: "margin-top: 0.5rem; width: 100%; padding: 0.4rem; background: var(--qualia-danger, #c00); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.8rem;",
                                    onclick: delete_selected_pane,
                                    "Remove Pane"
                                }
                            }
                        }
                    } else {
                        p {
                            style: "color: var(--qualia-text-muted, #555); font-size: 0.8rem;",
                            "Click a pane on the canvas to inspect its properties."
                        }
                    }
                }

                // SPARQL Binding Area
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Data Binding"
                    }
                    div {
                        style: "background: var(--qualia-bg, #0a0a0a); padding: 0.5rem; border-radius: 4px;",
                        p { style: "font-size: 0.75rem; color: var(--qualia-text-muted, #666);", "SPARQL / N3Logic query binding" }
                        dl {
                            style: "display:grid;grid-template-columns:auto 1fr;gap:0.2rem 0.5rem;margin:0;font:0.68rem/1.35 monospace;color:var(--qualia-success,#0f0);",
                            dt { "Subject" }
                            dd { style: "margin:0;overflow-wrap:anywhere;", "{mock_quin.read().subject:#018x}" }
                            dt { "Predicate" }
                            dd { style: "margin:0;overflow-wrap:anywhere;", "{mock_quin.read().predicate:#018x}" }
                            dt { "Object" }
                            dd { style: "margin:0;overflow-wrap:anywhere;", "{mock_quin.read().object:#018x}" }
                            dt { "Context" }
                            dd { style: "margin:0;overflow-wrap:anywhere;", "{mock_quin.read().context:#018x}" }
                            dt { "Metadata" }
                            dd { style: "margin:0;overflow-wrap:anywhere;", "{mock_quin.read().metadata:#018x}" }
                            dt { "Parity" }
                            dd { style: "margin:0;overflow-wrap:anywhere;", "{mock_quin.read().parity:#018x}" }
                        }
                    }
                }

                // Live Telemetry
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Live Telemetry"
                    }
                    div {
                        style: "height: 120px; overflow-y: auto; background: var(--qualia-bg, #000); color: var(--qualia-success, #0f0); padding: 0.5rem; font-family: monospace; font-size: 0.7rem; border-radius: 4px;",
                        for log in telemetry_logs.read().iter() {
                            div { "{log}" }
                        }
                        if telemetry_logs.read().is_empty() {
                            div { style: "color: #444;", "Waiting for telemetry stream..." }
                        }
                    }
                }

                // LLM Engine Panel
                div {
                    h3 {
                        style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                        "Native LLM Engine"
                    }
                    if *is_native_llm_active.read() {
                        div {
                            style: "background: rgba(16, 185, 129, 0.1); border: 1px solid #10b981; padding: 0.5rem; border-radius: 4px;",
                            span { style: "color: #10b981; font-size: 0.75rem; font-weight: bold;", "● Webizen Server Connected" }
                            p { style: "color: var(--qualia-text-muted); font-size: 0.7rem; margin: 0.3rem 0 0 0;", "Native offload is available for deeper inference, while the WASM workspace continues to run locally." }
                        }
                    } else {
                        div {
                            style: "background: rgba(255, 170, 0, 0.1); border: 1px solid #ffaa00; padding: 0.5rem; border-radius: 4px;",
                            span { style: "color: #ffaa00; font-size: 0.75rem; font-weight: bold;", "○ Standalone WASM Mode" }
                            p { style: "color: var(--qualia-text-muted); font-size: 0.7rem; margin: 0.3rem 0 0 0;", "The studio still runs locally in-browser; launch a Webizen Server only when you want native offload." }
                        }
                    }
                }

                // Deploy WAL history + restore
                if crate::endpoints::is_native_host() && !deploy_history.read().is_empty() {
                    div {
                        h3 {
                            style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0 0 0.5rem 0;",
                            "Deploy history"
                        }
                        div {
                            style: "display: flex; flex-direction: column; gap: 0.35rem; max-height: 140px; overflow-y: auto;",
                            for row in deploy_history.read().iter().rev().take(6) {
                                div {
                                    key: "rev-{row.revision}",
                                    style: "display: flex; align-items: center; justify-content: space-between; gap: 0.4rem; padding: 0.35rem 0.45rem; border-radius: 6px; border: 1px solid var(--qualia-border); background: var(--qualia-bg); font-size: 0.68rem;",
                                    span {
                                        style: "font-family: monospace; color: var(--qualia-text);",
                                        title: "manifest #{row.manifest_hash:016x}",
                                        "rev #{row.revision} · {row.pane_count} panes · ts {row.unix_ts}"
                                    }
                                    button {
                                        style: "padding: 0.15rem 0.4rem; font-size: 0.6rem; border-radius: 5px; border: 1px solid var(--qualia-accent); background: rgba(245,158,11,0.08); color: var(--qualia-text); cursor: pointer;",
                                        disabled: replaying_revision() == Some(row.revision),
                                        onclick: {
                                            let rev = row.revision;
                                            let mut restore = restore_deploy_revision.clone();
                                            move |_| restore(rev)
                                        },
                                        if replaying_revision() == Some(row.revision) { "…" } else { "Restore" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Save workspace (local settings portal + disk + Quin WAL)
                if !save_status.read().is_empty() {
                    p {
                        style: "font-size: 0.72rem; color: var(--qualia-text-muted, #888); margin: 0;",
                        "{save_status.read()}"
                    }
                }
                if !replay_status.read().is_empty() {
                    p {
                        style: "font-size: 0.68rem; color: var(--qualia-accent); margin: 0;",
                        "{replay_status.read()}"
                    }
                }
                if let Some(rev) = *deploy_revision.read() {
                    div {
                        style: "font-size: 0.68rem; padding: 0.35rem 0.5rem; border-radius: 6px; border: 1px solid var(--qualia-border); color: var(--qualia-accent); background: rgba(245,158,11,0.06);",
                        "Provenance: studio-workspace.wal rev #{rev}"
                    }
                }
                button {
                    style: "margin-top: auto; width: 100%; padding: 0.6rem; background: linear-gradient(135deg, var(--qualia-accent, #0ff), var(--qualia-primary, #06f)); color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; font-size: 0.85rem; letter-spacing: 0.05em; transition: opacity 0.2s;",
                    onclick: save_workspace,
                    "Save Workspace"
                }
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct DeployHistoryRow {
    revision: u64,
    unix_ts: u32,
    pane_count: u16,
    manifest_hash: u64,
}

// ─────────────────────────────────────────────────────────────
// Helper: render a palette category section with draggable items
// ─────────────────────────────────────────────────────────────

fn render_palette_category(palette: &[PaneDefinition], category: PaneCategory) -> Element {
    let items: Vec<&PaneDefinition> = palette.iter().filter(|p| p.category == category).collect();
    if items.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            style: "margin-bottom: 0.75rem;",
            div {
                style: "font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--qualia-text-muted, #666); margin-bottom: 0.3rem;",
                "{category_label(&category)}"
            }
            for item in items.iter() {
                div {
                    style: "padding: 0.35rem 0.5rem; margin-bottom: 2px; background: var(--qualia-bg, #0a0a0a); border: 1px solid var(--qualia-border, #2a2a2a); border-radius: 4px; cursor: grab; font-size: 0.8rem; display: flex; align-items: center; gap: 0.4rem; transition: border-color 0.15s, background 0.15s; user-select: none;",
                    draggable: "true",
                    "data-component-id": "{item.component_id}",
                    ondragstart: {
                        let cid = item.component_id.clone();
                        move |evt: Event<DragData>| {
                            let dt = evt.data().data_transfer();
                            let _ = dt.set_data("application/x-qualia-pane-id", &cid);
                        }
                    },
                    // Icon placeholder
                    span {
                        style: "width: 14px; height: 14px; border-radius: 3px; background: var(--qualia-accent, #0ff); opacity: 0.5; flex-shrink: 0;",
                    }
                    span { "{item.display_name}" }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Helper: render a placed pane on the canvas with selection
// ─────────────────────────────────────────────────────────────

fn presentation_mode_label(mode: &PresentationMode) -> &'static str {
    match mode {
        PresentationMode::GridBound => "Grid View",
        PresentationMode::NodeRelational => "Node View",
        PresentationMode::Spatial => "Spatial View",
    }
}

fn coordinate_space_label(space: &CoordinateSpace) -> &'static str {
    match space {
        CoordinateSpace::GlobalCartesian => "Global Coordinates",
        CoordinateSpace::RelativeAnchored => "Relative Anchors",
    }
}

fn layout_strategy_label(layout: &LayoutStrategy) -> &'static str {
    match layout {
        LayoutStrategy::PointGrid { .. } => "Point Grid",
        LayoutStrategy::CssGrid { .. } => "Legacy CSS Grid",
        LayoutStrategy::FlexBox => "Flex Layout",
        LayoutStrategy::Masonry => "Masonry Layout",
    }
}

fn layer_behavior_label(layer: &LayerBehavior) -> &'static str {
    match layer {
        LayerBehavior::Docked => "Docked",
        LayerBehavior::FloatingOverlay => "Floating Overlay",
        LayerBehavior::ModalOverlay => "Modal Overlay",
        LayerBehavior::FullCanvas => "Full Canvas",
    }
}

fn layer_behavior_value(layer: &LayerBehavior) -> &'static str {
    match layer {
        LayerBehavior::Docked => "docked",
        LayerBehavior::FloatingOverlay => "floating",
        LayerBehavior::ModalOverlay => "modal",
        LayerBehavior::FullCanvas => "full",
    }
}

fn layer_behavior_from_value(value: &str) -> Option<LayerBehavior> {
    match value {
        "docked" => Some(LayerBehavior::Docked),
        "floating" => Some(LayerBehavior::FloatingOverlay),
        "modal" => Some(LayerBehavior::ModalOverlay),
        "full" => Some(LayerBehavior::FullCanvas),
        _ => None,
    }
}

fn supported_presentations_summary(modes: &[PresentationMode]) -> String {
    modes
        .iter()
        .map(presentation_mode_label)
        .collect::<Vec<_>>()
        .join(", ")
}

fn canvas_container_style(page: &Page) -> String {
    match &page.layout_strategy {
        LayoutStrategy::PointGrid {
            width_points,
            height_points,
            snap_step,
            gutter: _,
        } => format!(
            "position: relative; min-height: 520px; height: min(76vh, {}px); overflow: hidden; border: 1px solid var(--qualia-border, #333); border-radius: 16px; background-color: rgba(10,10,10,0.45); background-image: linear-gradient(rgba(255,255,255,0.05) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.05) 1px, transparent 1px); background-size: calc(100% / {}) calc(100% / {});",
            (*height_points as u32).saturating_mul(10),
            width_points.max(snap_step),
            height_points.max(snap_step),
        ),
        LayoutStrategy::CssGrid { cols, rows, gap } => format!(
            "display: grid; position: relative; grid-template-columns: repeat({}, 1fr); grid-template-rows: repeat({}, 80px); gap: {}px; min-height: 400px;",
            cols, rows, gap
        ),
        LayoutStrategy::FlexBox => {
            "display: flex; position: relative; flex-direction: column; gap: 1rem; min-height: 400px;"
                .to_string()
        }
        LayoutStrategy::Masonry => {
            "display: block; position: relative; min-height: 400px; column-width: 280px; column-gap: 1rem;"
                .to_string()
        }
    }
}

fn build_pane_placement(component_id: &str, existing_count: usize) -> Option<PanePlacement> {
    if component_id.is_empty() {
        return None;
    }
    let (default_w, default_h) = find_pane(component_id)
        .map(|p| (p.default_w, p.default_h))
        .unwrap_or((4, 2));
    Some(PanePlacement {
        component_id: component_id.to_string(),
        x: 4,
        y: 4 + (existing_count as u16).saturating_mul(6),
        w: (default_w as u16).saturating_mul(6),
        h: (default_h as u16).saturating_mul(6),
        data_bindings: vec![],
        binds_rpc: if component_id == "custom-web-module" {
            Some(crate::endpoints::MODULE_RPC_WS.into())
        } else {
            None
        },
        requires_capability: vec![],
        ui_mode: if component_id == "custom-web-module" {
            Some(UiMode::IFrameSandbox)
        } else {
            None
        },
        layer: if component_id == "custom-web-module" {
            LayerBehavior::FloatingOverlay
        } else {
            LayerBehavior::Docked
        },
        anchor: None,
        min_w_points: (default_w as u16).saturating_mul(4),
        min_h_points: (default_h as u16).saturating_mul(4),
        supported_presentations: vec![PresentationMode::GridBound],
        theme: ThemeBinding::default(),
    })
}

fn pane_style_for_layout(
    page: &Page,
    pane: &PanePlacement,
    is_selected: bool,
    selection_scale: f64,
) -> String {
    let border_color = if is_selected {
        "var(--qualia-accent, #0ff)"
    } else {
        "var(--qualia-border, #333)"
    };
    let bg = if is_selected {
        "rgba(0, 255, 255, 0.08)"
    } else {
        "var(--qualia-surface, #181818)"
    };
    let shadow = if is_selected {
        "0 0 0 1px rgba(6, 182, 212, 0.25), 0 16px 36px rgba(0, 0, 0, 0.28)"
    } else {
        "0 12px 26px rgba(0, 0, 0, 0.18)"
    };
    let transform = if is_selected {
        format!("transform: scale({selection_scale:.4}); transform-origin: center center;")
    } else {
        String::new()
    };

    match (&page.layout_strategy, &pane.layer) {
        (LayoutStrategy::CssGrid { .. }, LayerBehavior::Docked) => format!(
            "grid-column: {} / span {}; grid-row: {} / span {}; background: {}; border: 1px solid {}; border-radius: 10px; padding: 0.75rem; cursor: pointer; transition: border-color 0.2s, background 0.2s, transform 0.12s ease-out; display: flex; flex-direction: column; justify-content: space-between; min-height: 88px; box-shadow: {}; {transform}",
            pane.x.max(1),
            pane.w.max(1),
            pane.y.max(1),
            pane.h.max(1),
            bg,
            border_color,
            shadow,
        ),
        _ => {
            let (grid_w, grid_h, gutter) = match &page.layout_strategy {
                LayoutStrategy::PointGrid {
                    width_points,
                    height_points,
                    gutter,
                    ..
                } => (*width_points as f32, *height_points as f32, *gutter),
                _ => (100.0, 100.0, 2),
            };
            let left = (pane.x as f32 / grid_w).clamp(0.0, 1.0) * 100.0;
            let top = (pane.y as f32 / grid_h).clamp(0.0, 1.0) * 100.0;
            let width = (pane.w.max(1) as f32 / grid_w).clamp(0.08, 1.0) * 100.0;
            let height = (pane.h.max(1) as f32 / grid_h).clamp(0.08, 1.0) * 100.0;
            let min_width_px = pane.min_w_points.max(12) as u32 * 6;
            let min_height_px = pane.min_h_points.max(10) as u32 * 5;

            match pane.layer {
                LayerBehavior::Docked => format!(
                    "position: absolute; left: {:.3}%; top: {:.3}%; width: calc({:.3}% - {}px); height: calc({:.3}% - {}px); min-width: {}px; min-height: {}px; background: {}; border: 1px solid {}; border-radius: 12px; padding: 0.75rem; cursor: pointer; transition: border-color 0.2s, background 0.2s, transform 0.12s ease-out; display: flex; flex-direction: column; justify-content: space-between; box-shadow: {}; overflow: hidden; {transform}",
                    left,
                    top,
                    width,
                    gutter,
                    height,
                    gutter,
                    min_width_px,
                    min_height_px,
                    bg,
                    border_color,
                    shadow,
                ),
                LayerBehavior::FloatingOverlay => format!(
                    "position: absolute; pointer-events: auto; left: {:.3}%; top: {:.3}%; width: calc({:.3}% - {}px); height: calc({:.3}% - {}px); min-width: {}px; min-height: {}px; background: color-mix(in srgb, {} 92%, black 8%); border: 1px solid {}; border-radius: 14px; padding: 0.75rem; cursor: pointer; display: flex; flex-direction: column; justify-content: space-between; box-shadow: 0 22px 50px rgba(0, 0, 0, 0.35); backdrop-filter: blur(14px); z-index: 30; overflow: hidden;",
                    left,
                    top,
                    width,
                    gutter,
                    height,
                    gutter,
                    min_width_px,
                    min_height_px,
                    bg,
                    border_color,
                ),
                LayerBehavior::ModalOverlay => format!(
                    "position: absolute; pointer-events: auto; left: 50%; top: 50%; width: min({:.3}%, 760px); height: min({:.3}%, 560px); min-width: {}px; min-height: {}px; transform: translate(-50%, -50%); background: color-mix(in srgb, {} 94%, black 6%); border: 1px solid {}; border-radius: 16px; padding: 0.9rem; cursor: pointer; display: flex; flex-direction: column; justify-content: space-between; box-shadow: 0 28px 80px rgba(0, 0, 0, 0.45); backdrop-filter: blur(16px); z-index: 45; overflow: hidden;",
                    width.max(28.0),
                    height.max(24.0),
                    min_width_px.max(280),
                    min_height_px.max(220),
                    bg,
                    border_color,
                ),
                LayerBehavior::FullCanvas => format!(
                    "position: absolute; pointer-events: auto; inset: 0; background: {}; border: 1px solid {}; border-radius: 14px; padding: 0.9rem; cursor: pointer; display: flex; flex-direction: column; justify-content: space-between; box-shadow: 0 22px 50px rgba(0, 0, 0, 0.32); z-index: 55; overflow: hidden;",
                    bg, border_color,
                ),
            }
        }
    }
}

fn render_placed_pane(
    page: &Page,
    pane: &PanePlacement,
    idx: usize,
    selected: &Signal<Option<usize>>,
    theme: ResolvedTheme,
    editor_mode: CanvasEditorMode,
    pane_interaction: &Signal<Option<PaneInteraction>>,
    drag_anchor: &Signal<(f64, f64)>,
    selection_scale: f64,
) -> Element {
    let is_selected = *selected.read() == Some(idx);
    let scale = if is_selected { selection_scale } else { 1.0 };
    let editing = editor_mode == CanvasEditorMode::Edit;

    let element_tag = find_pane(&pane.component_id)
        .map(|p| p.element_tag.clone())
        .unwrap_or_else(|| pane.component_id.clone());

    let mut selected = selected.clone();
    let mut pane_interaction = pane_interaction.clone();
    let mut drag_anchor = drag_anchor.clone();
    let pane_x = pane.x;
    let pane_y = pane.y;
    let pane_w = pane.w;
    let pane_h = pane.h;

    rsx! {
        div {
            class: "{join_theme_classes(\"webizen-module-pane\", &theme)}",
            "data-theme-scope": "module",
            "data-theme": "{theme.theme_key.clone().unwrap_or_default()}",
            "data-pane-index": "{idx}",
            "data-selected": if is_selected { "true" } else { "false" },
            style: "{pane_style_for_layout(page, pane, is_selected, scale)}",
            onclick: move |_| {
                selected.set(Some(idx));
            },

            if editing {
                div {
                    style: "height: 14px; margin: -0.35rem -0.35rem 0.35rem; border-radius: 8px 8px 0 0; background: rgba(255,255,255,0.04); cursor: grab; display: flex; align-items: center; justify-content: center;",
                    onmousedown: move |evt: Event<MouseData>| {
                        evt.stop_propagation();
                        let c = evt.data().client_coordinates();
                        drag_anchor.set((c.x, c.y));
                        pane_interaction.set(Some(PaneInteraction::Drag {
                            idx,
                            orig_x: pane_x,
                            orig_y: pane_y,
                        }));
                    },
                    span {
                        style: "width: 28px; height: 3px; border-radius: 99px; background: var(--qualia-border); opacity: 0.8;",
                    }
                }
            }

            div {
                style: if editing { "pointer-events: none; flex: 1; overflow: hidden;" } else { "flex: 1; overflow: hidden;" },
                crate::components::qapp_dispatcher::QAppDispatcher {
                    element_tag: element_tag.clone(),
                }
            }

            if editing && is_selected {
                div {
                    style: "position: absolute; right: 6px; bottom: 6px; width: 12px; height: 12px; border-radius: 2px; border: 1px solid var(--qualia-accent); background: rgba(245,158,11,0.25); cursor: nwse-resize; z-index: 5;",
                    onmousedown: move |evt: Event<MouseData>| {
                        evt.stop_propagation();
                        let c = evt.data().client_coordinates();
                        drag_anchor.set((c.x, c.y));
                        pane_interaction.set(Some(PaneInteraction::Resize {
                            idx,
                            orig_w: pane_w,
                            orig_h: pane_h,
                        }));
                    },
                }
            }
        }
    }
}
