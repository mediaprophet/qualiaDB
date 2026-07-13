# Act X — Surfaces

> *The engine is everywhere.*

---

## Thesis

> **The engine ships as a desktop shell, a CLI, a browser-local ontology
> MCP, a mobile harness, a Solid-pod bridge, a wellfare/health library, a
> render SDK, and a Dioxus studio. Each surface is a thin adapter over the
> same engine.**

---

## Voice-over script

### Shot 1 — A grid of surfaces appears. Seven cells. Each is a different deployment. [SLOW]

> These are the surfaces. [PAUSE]
> Each one is a thin adapter over the same engine. [PAUSE]

### Shot 2 — Desktop shell. Tauri 2. Windows. macOS. Linux. [SLOW]

> The desktop shell is `webizen-desktop`. [PAUSE]
> It is Tauri two. [PAUSE]
> It runs on Windows, macOS, and Linux. [PAUSE]
> It has a system tray. It has a settings server. It has a telemetry
> bridge. [PAUSE]

### Shot 3 — CLI. qualia-cli. [ITEM]

> The CLI is `qualia-cli`. [PAUSE] [ITEM]
> Daemon management — start, stop, status, doctor. [PAUSE] [ITEM]
> MCP server management. [PAUSE] [ITEM]
> QPU provider management. [PAUSE] [ITEM]
> Resource catalog — list, show, download, import ontology. [PAUSE] [ITEM]
> Science — chemistry, biology, thermodynamics, geometric algebra. [PAUSE] [ITEM]
> Solve — matrix, determinant, eigenvalues, tensor contraction. [PAUSE] [ITEM]
> Shader — list kernels, generate, validate, certify, tune. [PAUSE] [ITEM]
> LLM lifecycle — list, load, status, eval, evict. [PAUSE] [ITEM]
> LLM testing — test models, validate, benchmark, generate report. [PAUSE] [ITEM]
> Evaluate — deontic, epistemic, paraconsistent, LTL, ASP, DL,
> probabilistic, linear logic, dialectical, diffusion, spatio-temporal,
> interval, graph topology. [END LIST] [PAUSE]

### Shot 4 — Browser-local ontology MCP. WASM. [ITEM]

> The browser-local ontology MCP is `webizen-lite-wasm`. [PAUSE] [ITEM]
> It is WASM. [PAUSE] [ITEM]
> It is two hundred and sixty-eight kilobytes raw. Ninety-five kilobytes
> gzipped. [PAUSE] [ITEM]
> It has eleven bounded MCP tools. [PAUSE] [ITEM]
> N3 inspection. Quin query. SHACL validation. Modal evaluation.
> Subsumption. Hashing. Governance. [END LIST] [PAUSE]
> It runs in the browser. It does not phone home. [PAUSE]

### Shot 5 — Mobile harness. Dioxus. iOS. Android. [SLOW]

> The mobile harness is `qualia-mobile-harness`. [PAUSE]
> It is Dioxus. [PAUSE]
> It runs on iOS and Android. [PAUSE]
> It has a QR scanner. It has directory access. [PAUSE]

### Shot 6 — Solid-pod bridge. LDP. OIDC. [SLOW]

> The Solid-pod bridge is `qualia-solid-bridge`. [PAUSE]
> It speaks LDP — Linked Data Platform. [PAUSE]
> It speaks OIDC — OpenID Connect. [PAUSE]
> It compiles WAC — Web Access Control — into bytecode. [PAUSE]
> It has an allocation firewall. [PAUSE]

### Shot 7 — Wellfare/health library. CSV. N3 rules. SHACL shapes. [SLOW]

> The wellfare library is `wellfare-core`. [PAUSE]
> It parses CSV exports from Samsung Health, Apple Health, and others. [PAUSE]
> Weight. Sleep. Heart rate. Steps. [PAUSE]
> It has N3 rules — tachycardia fires, sleep debt fires, adrenal
> fatigue fires. [PAUSE]
> It has SHACL shapes — invalid sleep efficiency is caught. [PAUSE]
> It has WASM bindings. [PAUSE]

### Shot 8 — Render SDK. wgpu 29. Native + WASM. [SLOW]

> The render SDK is `webizen-render`. [PAUSE]
> It is wgpu twenty-nine. [PAUSE]
> It runs natively and in WASM. [PAUSE]
> It is the same renderer on every surface. [PAUSE]

### Shot 9 — Dioxus studio. Themes. Panes. Routes. [SLOW]

> The studio is `webizen-studio`. [PAUSE]
> It is Dioxus. [PAUSE]
> It has themes. It has panes. It has routes. [PAUSE]
> It is the place where the surfaces are composed. [PAUSE]

### Shot 10 — Component harvester. Shoelace. Dioxus. [SLOW]

> The component harvester is `webizen-component-harvester`. [PAUSE]
> It reads a Shoelace custom-elements manifest. [PAUSE]
> It generates Dioxus components. [PAUSE]
> Forty-three components, generated automatically. [PAUSE]

### Shot 11 — Title card: **One engine. Many surfaces.** [SLOW]

> One engine. [PAUSE]
> Many surfaces. [PAUSE]
> The engine is the substrate. [PAUSE]
> The surfaces are the seams. [PAUSE]

---

## On-screen notes

- **Shot 1:** A 3x3 grid. Each cell is a surface. The cells are color-coded.
- **Shot 2:** The desktop shell. A Tauri 2 window. The system tray is visible.
- **Shot 3:** The CLI. A terminal. The commands are listed.
- **Shot 4:** A browser window. The WASM module is loaded. The eleven MCP tools are listed.
- **Shot 5:** A mobile device. The QR scanner is active.
- **Shot 6:** The Solid-pod bridge. An LDP endpoint. An OIDC flow.
- **Shot 7:** The wellfare library. A CSV file. An N3 rule. A SHACL shape.
- **Shot 8:** The render SDK. A scene. The same scene on every surface.
- **Shot 9:** The studio. A theme. A pane. A route.
- **Shot 10:** The component harvester. A custom-elements manifest. A generated Dioxus component.
- **Shot 11:** Title card.

---

## Source code anchors

- `crates/webizen-desktop/` — Tauri 2 desktop shell.
- `crates/webizen-desktop/src/main.rs` — `main`, `protocol_response`, `diffusion_frame_response`, `render_preview_response`, `webizen_protocol_response`.
- `crates/webizen-desktop/src/runtime.rs` — `RuntimeSnapshotRecord`, `LedgerHealthFingerprint`, `LedgerMetrics`, `DesktopLedgerSink`.
- `crates/webizen-desktop/src/settings_server.rs` — `SettingsServerState`, `health_handler`, `status_handler`, `probe_graph_daemon`.
- `crates/webizen-desktop/src/telemetry_bridge.rs` — `TelemetryBridge`.
- `crates/webizen-desktop/src/telemetry_hooks.rs` — `increment_inference_counter`, `increment_network_io_counter`, `increment_baking_counter`, `increment_query_resolve_counter`, `increment_quantum_activity_counter`.
- `crates/qualia-cli/src/main.rs` — `Cli`, `Commands`.
- `crates/qualia-cli/src/daemon.rs` — `DaemonOpts`, `DaemonAction`, `serve_foreground`, `start_service`, `stop_service`, `print_status`, `print_doctor`.
- `crates/qualia-cli/src/mcp.rs` — `McpTransport`, `McpAction`, `serve_tcp`.
- `crates/qualia-cli/src/qpu.rs` — `run_list_providers`, `run_configure`, `run_show`.
- `crates/qualia-cli/src/resources.rs` — `cmd_list`, `cmd_show`, `cmd_download`, `cmd_import_ontology`.
- `crates/qualia-cli/src/science.rs` — `run_chem_smiles`, `run_bio_align`, `run_thermo_gibbs`.
- `crates/qualia-cli/src/solve.rs` — `run_matrix_multiply`, `run_determinant`, `run_solve_system`, `run_eigenvalues`, `run_tensor_contract`.
- `crates/qualia-cli/src/shader.rs` — `list-kernels`, `generate`, `validate`, `certify`, `tune`.
- `crates/qualia-cli/src/llm_lifecycle.rs` — `run_list`, `run_load`, `run_status`, `run_eval`, `run_evict`.
- `crates/qualia-cli/src/llm_testing.rs` — `run_test_models`, `run_comprehensive_llm_test`, `run_validate_models`, `run_benchmark_models`, `run_generate_report`.
- `crates/qualia-cli/src/evaluate.rs` — `run_deontic`, `run_epistemic`, `run_paraconsistent`, `run_ltl`, `run_asp`, `run_dl`, `run_probabilistic`, `run_linear_logic`, `run_dialectical`, `run_diffusion`, `run_spatio_temporal`, `run_interval`, `run_graph_topology`.
- `crates/webizen-lite-wasm/src/lib.rs` — `mcp_jsonrpc`, `initialize`, `call_tool`, `ontology_capabilities`, `hash_iri`, `parse_n3`, `query_quins`, `validate_shacl`, `evaluate_deontic`, `evaluate_epistemic`, `route_paraconsistent`, `evaluate_ltl`, `check_subsumption`.
- `crates/qualia-mobile-harness/src/main.rs` — `main`, `startQrScanner`, `requestDirectoryAccess`.
- `crates/qualia-solid-bridge/src/ldp_translator.rs` — `ldp_routes`, `ldp_to_quins`, `compile_wac_to_bytecode`.
- `crates/qualia-solid-bridge/src/oidc_micro_idp.rs` — `oidc_routes`.
- `crates/qualia-solid-bridge/src/solid_proxy.rs` — `bridge_routes`, `start_proxy_daemon`.
- `crates/wellfare-core/src/parser.rs` — `parse_weight_csv`, `parse_sleep_csv`, `parse_heart_rate_csv`, `parse_steps_csv`.
- `crates/wellfare-core/src/n3_rules.rs` — `evaluate_n3_rules`, `test_tachycardia_fires`, `test_sleep_debt_fires`, `test_normal_data_no_flags`, `test_adrenal_fatigue_fires`.
- `crates/wellfare-core/src/shapes.rs` — `validate_turtle`, `test_valid_sleep_passes`, `test_invalid_efficiency_caught`.
- `crates/wellfare-core/src/wasm.rs` — `parse_weight_csv_json`, `WasmHealthStore`.
- `crates/wellfare-core/src/webizen.rs` — `WebizenVM`, `execute`, `check_threshold`, `evaluate_policy_constraint`.
- `crates/webizen-render/` — Render SDK.
- `crates/webizen-studio/src/main.rs` — `Route`, `DashboardRoute`, `ContextStudioRoute`, `QAppsRoute`, `BrowserRoute`, `StudioRoute`, `RenderPreviewRoute`, `SceneInteractionRoute`, `NexusRoute`, `SettingsRoute`, `AboutRoute`.
- `crates/webizen-studio/src/pane_registry.rs` — `PaneDefinition`, `PaneCategory`, `builtin_pane_definitions`.
- `crates/webizen-studio/src/theme_engine.rs` — `ThemeDefinition`, `builtin_theme_catalog`, `resolve_theme`.
- `crates/webizen-component-harvester/src/main.rs` — `CustomElementsManifest`, `map_ts_type_to_rust`, `generate_dioxus_macro`.
- `crates/webizen-component-harvester/generated_dioxus_components.rs` — 43 generated components.

---

## Duration

Approximately 90 seconds. This is the act where the viewer sees the engine everywhere.
