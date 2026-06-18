/* tslint:disable */
/* eslint-disable */

/**
 * The Federated Node Manager handles discovery and WebRTC offloading
 */
export class FederatedNodeManager {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Probes the local network/IPC for an installed 64-bit native daemon
     */
    discover_capabilities(): boolean;
    constructor();
    /**
     * Attempts to route a heavy mathematical payload to the native daemon
     */
    offload_intent(intent: WasmOffloadIntent): string;
}

/**
 * Portal tier: 0 = CPU canvas2d fallback, 1 = tensor projection, 2 = WebGPU ambient.
 */
export class QualiaPortal {
    free(): void;
    [Symbol.dispose](): void;
    acoustic_enabled(): boolean;
    /**
     * SharedArrayBuffer byte length for zero-copy U3 handoff (requires COOP/COEP).
     */
    acoustic_sab_byte_length(): number;
    /**
     * Whether a baked STFT/CQT sidecar is pinned on the portal.
     */
    acoustic_sidecar_pinned(): boolean;
    /**
     * Serialized `AcousticUniform` bytes for AudioWorklet `SharedArrayBuffer` handoff.
     */
    acoustic_uniform_bytes(): Uint8Array;
    /**
     * Phenomenal U3 float uniform count (18 scalars + 64 preview bins).
     */
    acoustic_uniform_float_count(): number;
    /**
     * Flat `f32` uniform for AudioWorklet message port (18 scalars + 64 preview bins).
     */
    acoustic_uniform_floats(): Float32Array;
    /**
     * Cold-bake CQT sidecar (log-spaced bins) for selected tensor node.
     */
    bake_cqt_sidecar_demo(frames: number): Uint8Array;
    /**
     * Cold-bake STFT sidecar for selected tensor node; pins bytes for hot frame reads.
     */
    bake_stft_sidecar_demo(frames: number): Uint8Array;
    camera_pitch(): number;
    camera_yaw(): number;
    camera_zoom(): number;
    /**
     * Wavefunction collapse — set node `q` to 0 in the resident session manifold.
     */
    collapse_node_q(index: number): void;
    /**
     * Pending ICP commands in the SPSC ring.
     */
    control_pending(): number;
    /**
     * Allocate zeroed acoustic SAB with Q3AS header.
     */
    create_acoustic_sab(): SharedArrayBuffer;
    /**
     * Drain up to `max` control commands and apply to this portal. Returns count applied.
     */
    drain_control_commands(max: number): number;
    /**
     * Drain pending sonic tokens into a JS `BigUint64Array` or `Array` of token raw values.
     */
    drain_sonic_tokens(max: number): any;
    encode_geometry(json: string): any;
    epistemic_q(): number;
    last_parsed(): any | undefined;
    load_json_scene(json: string): any;
    load_q42(bytes: Uint8Array): any;
    mount_qapp(root_id: string): void;
    /**
     * Frame the camera on a tensor node (`Maps_to_node`).
     */
    navigate_to_node(index: number): void;
    constructor(canvas: HTMLCanvasElement);
    /**
     * Select at pixel; returns index immediately on CPU fallback, else `-1` until next `tick`.
     */
    observe_node_at(x: number, y: number, canvas_w: number, canvas_h: number): number;
    operational_mode(): number;
    /**
     * Returns selected tensor index, or `-1` if none / pick still pending.
     */
    poll_selected_node(): number;
    /**
     * Publish phenomenal uniform + pending sonic tokens into SAB.
     */
    publish_acoustic_sab(sab: SharedArrayBuffer): void;
    /**
     * Push a packed Interface Control Plane command (`PortalControlCommand` raw `u64`).
     */
    push_control_command(raw: bigint): boolean;
    push_sonic_token_raw(raw: bigint): boolean;
    resize(canvas: HTMLCanvasElement, width: number, height: number): void;
    sample_telemetry(): any;
    /**
     * Queue GPU picking at canvas pixel `(x, y)`. Result available after the next `tick`.
     */
    select_node_at(x: number, y: number, canvas_w: number, canvas_h: number): void;
    selected_node_index(): number;
    /**
     * Enable or mute U3 AcousticPlane (automatically off in Reserve mode).
     */
    set_acoustic_enabled(enabled: boolean): void;
    /**
     * Orbit camera IPC from the UI shell (yaw/pitch in radians, zoom = eye distance).
     */
    set_camera(yaw: number, pitch: number, zoom: number): void;
    set_display_mode(mode: string): void;
    /**
     * Human-Centric observer standpoint IPC (independent of camera lens).
     *
     * `standpoint_class`: 0=spectator, 1=ephemeral, 2=identifier (DID), 3=vault.
     * `identifier_did`: empty for spectator/ephemeral; supply DID IRI to bind a verified
     * identifier. Vault standpoints require a sealed local data plane (not exposed here).
     */
    set_standpoint(standpoint_class: number, epistemic_q: number, t_slice: number, t_window: number, identifier_did: string): void;
    set_telemetry(floats: Float32Array): void;
    sonic_token_pending(): number;
    spatial_encode(json: string): any;
    standpoint_class(): number;
    t_slice(): number;
    t_window(): number;
    tick(canvas: HTMLCanvasElement, dt_ms: number): void;
    tier(): number;
    upload_tensor_buffer(bytes: Uint8Array): void;
}

/**
 * WASM edge offload descriptor — distinct from governance [`crate::llm_agent::AgentIntent`].
 */
export class WasmOffloadIntent {
    free(): void;
    [Symbol.dispose](): void;
    constructor(opcode: number, priority: number, payload_size: number);
    static with_string_payload(opcode: number, priority: number, payload: string): WasmOffloadIntent;
    opcode: number;
    payload_size: number;
    priority: number;
}

/**
 * Legacy alias — prefer `QualiaPortal`.
 */
export class WebEngine {
    free(): void;
    [Symbol.dispose](): void;
    last_parsed(): any | undefined;
    load_json_scene(json: string): any;
    load_q42(bytes: Uint8Array): any;
    mount_qapp(root_id: string): void;
    constructor();
    render_to_canvas(): void;
}

export function align_sequences_wasm(val: any): any;

/**
 * Black-Scholes European option pricing with full Greeks.
 */
export function black_scholes_wasm(val: any): any;

export function check_drug_interactions_wasm(val: any): any;

/**
 * Compiles a query string (SPARQL WHERE-clause or N-Triples pattern) to a JSON
 * description of the Webizen VM bytecode program.  Useful for playground inspection
 * and benchmarking the compilation pipeline without supplying a database.
 */
export function compile_query_to_json(query: string): string;

export function compute_framingham_risk_wasm(val: any): any;

export function compute_molecular_descriptors_wasm(val: any): any;

/**
 * Stateless PID controller step.
 * Returns { output, new_error, new_integral } for chaining into the next step.
 */
export function compute_pid_step_wasm(val: any): any;

export function compute_reaction_metrics_wasm(val: any): any;

export function compute_thermochemistry_wasm(val: any): any;

export function create_canvas(width: number, height: number): HTMLCanvasElement;

export function design_encode_wasm(json: string): any;

export function detect_functional_groups_wasm(val: any): any;

/**
 * Enforces the rights ontology prior to transmission (e.g., checking DID constraints)
 */
export function enforce_rights_ontology(subject_did: bigint): boolean;

/**
 * Query the browser's storage quota and current OPFS usage (bytes).
 *
 * Returns `{ quota: number, usage: number, available: number }`.
 * On mobile PWA the quota is typically 60 % of free disk space (Chrome) or
 * up to 1 GB on iOS Safari. Call this before a large ingest to check headroom.
 */
export function estimate_browser_storage(): Promise<any>;

export function evaluate_lipinski_wasm(val: any): any;

export function execute_ntriples_query(query: string, db_bytes: Uint8Array, max_results: number): string;

export function export_tensor_buffer_wasm(json: string): any;

export function export_tensor_slice_wasm(max_nodes: number): any;

/**
 * Forward-chaining defeasible inference engine.
 * Input: `{ facts: ["bird", "penguin"], rules: [{ head: "flies", body: ["bird"], defeaters: ["penguin"] }, ...] }`
 * Output: `{ inferred: ["swims"] }`
 */
export function forward_chain_wasm(val: any): any;

export function geosparql_operation_wasm(json: string): any;

/**
 * Crate semver baked in at compile time.
 */
export function getEngineVersion(): string;

/**
 * Structured engine metadata for browser UIs and diagnostics.
 */
export function get_engine_info(): any;

/**
 * Returns the qualia-core-db crate version baked in at compile time (matches daemon `/health`).
 */
export function get_engine_version(): string;

/**
 * Phase 2B: async WebGPU decode — yields to the browser event loop on every `map_async`.
 * Returns a JS `Promise`; use `await inferWasmAsync(...)` from module code.
 */
export function inferWasmAsync(prompt: string, on_token: Function): Promise<string>;

/**
 * Stream token deltas to `on_token` (UTF-8 string chunks) while decoding.
 */
export function inferWasmStreaming(prompt: string, on_token: Function): Promise<string>;

/**
 * Streaming inference with optional graph context for provenance hashing.
 */
export function inferWasmStreamingWithContext(prompt: string, graph_context: string, on_token: Function): Promise<string>;

/**
 * Same as `infer_wasm` but accepts optional graph-context bytes for provenance hashing.
 */
export function inferWasmWithContext(prompt: string, graph_context: string): Promise<string>;

/**
 * Run autoregressive inference (non-streaming). Prompt must include any chat template tokens.
 */
export function infer_wasm(prompt: string): Promise<string>;

export function init_panic_hook(): void;

/**
 * Load a GGUF model into the resident browser WebGPU engine.
 */
export function initialize_webgpu_engine(gguf_data: Uint8Array): Promise<void>;

/**
 * Intercepts heavy computational opcodes and constructs a WASM offload intent.
 */
export function intercept_computational_opcode(opcode: number, payload_size: number): WasmOffloadIntent | undefined;

export function intercept_pharmacogenomics_intent(smiles: string): WasmOffloadIntent;

/**
 * Returns true when a GGUF model has been loaded via `initialize_webgpu_engine`.
 */
export function isWebgpuEngineReady(): boolean;

/**
 * Check whether a SuperBlock is cached in the OPFS vault.
 * Returns `true` if the `.qblk` file exists, `false` otherwise.
 */
export function is_opfs_block_cached(block_index: number): Promise<boolean>;

/**
 * Capability names available in this WASM build.
 */
export function list_capabilities_wasm(): any;

/**
 * Pack raw NQuin field bytes into a fully-structured SuperBlock with correct ECC parity.
 *
 * `raw_quin_bytes` must be `N × 48` bytes where each 48-byte chunk contains the
 * five semantic `u64` fields (40 bytes) followed by 8 placeholder bytes (ignored —
 * ECC is computed here). `N` must not exceed `QUINS_PER_BLOCK` (850).
 *
 * Returns exactly `BLOCK_MULTIPLIER_SIZE` (40 960) bytes, ready to write to OPFS.
 * This is the canonical packing path — **the JS ingest worker must call this**
 * instead of reimplementing the SuperBlock layout in JavaScript.
 */
export function pack_quins_into_superblock(seq_id: bigint, owner_did: bigint, raw_quin_bytes: Uint8Array): Uint8Array;

export function parse_cbor_ld_wasm(payload: Uint8Array): any;

export function parse_csv_wasm(val: any): any;

export function parse_json_mapping_wasm(val: any): any;

export function parse_json_wasm(payload: string): any;

export function parse_n3logic_wasm(payload: string): any;

export function parse_turtle_wasm(payload: string): any;

export function predict_receptor_binding_wasm(): number;

/**
 * Performs topological pruning and validates meshes prior to physics offloading
 */
export function prune_and_validate_mesh(mesh_id: bigint): boolean;

/**
 * Read a cached SuperBlock from the OPFS vault.
 *
 * Returns the raw 40 960 bytes as `Uint8Array`, or `null` if the block has not
 * been written yet (cache miss). Callers should fall back to an HTTP Range
 * request (see the JS `VFS` class) on cache miss.
 */
export function read_opfs_block(block_index: number): Promise<any>;

/**
 * Release resident GGUF weights and tear down the WebGPU engine instance.
 */
export function releaseWebgpuEngine(): Promise<void>;

/**
 * Resolves two conflicting NQuin entries using Last-Writer-Wins semantics.
 * The Lamport clock is encoded in the metadata field; on ties, higher object wins.
 */
export function resolve_lww_wasm(local_val: any, remote_val: any): any;

export function run_semantic_simulation(val: any): any;

export function sample_browser_telemetry_wasm(): any;

export function serialize_csv_wasm(val: any): any;

/**
 * Continuous Mathematical Serialization into Float64Array
 */
export function serialize_float64_array(data: Float64Array): Float64Array;

/**
 * Packs an array of floats into a Uint8Array strictly typed buffer to avoid IEEE-754 truncation
 */
export function serialize_float_array(data: Float32Array): Uint8Array;

export function serialize_json_wasm(val: any): any;

export function serialize_rdf_wasm(val: any): any;

/**
 * Simulates a GBM price path and returns the full series together with
 * min_price, max_price, and final_price.
 */
export function simulate_gbm_path_wasm(val: any): any;

/**
 * Solves dy/dt = -k·y via classical RK4, returning t_values, y_values, and final_y.
 */
export function solve_ode_exponential_decay_wasm(val: any): any;

/**
 * Bounded DPLL SAT solver.
 * Input: `{ clauses: [[1, 2, -3], [-1, 3], ...] }` (signed literal convention).
 * Output: `{ satisfiable: bool, assignment: { "1": true, "2": false, ... } }`
 */
export function solve_sat_wasm(val: any): any;

export function spatial_encode_wasm(json: string): any;

export function validate_fasta_wasm(val: any): any;

export function validate_fhir_observation_wasm(val: any): any;

export function validate_shacl_constraint_wasm(val: any): any;

/**
 * Validate ECC parity for every NQuin in a raw SuperBlock.
 *
 * Returns JSON: `{"valid":bool,"total":N,"bad":[indices...]}`
 * A non-empty `bad` array indicates sector corruption.
 */
export function verify_superblock_ecc(block_bytes: Uint8Array): string;

/**
 * Polls the local Webizen for pending agreements waiting for the user's signature.
 */
export function webizen_poll_agreements(): string;

/**
 * Proposes a new M:N Guardianship agreement to the local WebRTC mesh.
 */
export function webizen_propose_agreement(_nominated_guardians: Array<any>, principal: string, domain: string, threshold: number): bigint;

/**
 * Signs a pending agreement, advancing its state machine and triggering WebRTC peer sync.
 */
export function webizen_sign_agreement(_agreement_id: bigint, _private_key_mock: string): void;

/**
 * Write a SuperBlock to the OPFS vault at `block_index`.
 *
 * `block_bytes` must be exactly `BLOCK_MULTIPLIER_SIZE` (40 960) bytes — use
 * `pack_quins_into_superblock()` to produce correctly-structured blocks.
 *
 * File name: `block_XXXXXXXX.qblk` (zero-padded 8-digit decimal index).
 * Compatible with the naming convention used by the JS VFS class.
 */
export function write_opfs_block(block_index: number, block_bytes: Uint8Array): Promise<void>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly getEngineVersion: () => [number, number];
    readonly inferWasmAsync: (a: number, b: number, c: any) => any;
    readonly inferWasmStreaming: (a: number, b: number, c: any) => any;
    readonly inferWasmStreamingWithContext: (a: number, b: number, c: number, d: number, e: any) => any;
    readonly inferWasmWithContext: (a: number, b: number, c: number, d: number) => any;
    readonly infer_wasm: (a: number, b: number) => any;
    readonly initialize_webgpu_engine: (a: any) => any;
    readonly isWebgpuEngineReady: () => number;
    readonly releaseWebgpuEngine: () => any;
    readonly __wbg_qualiaportal_free: (a: number, b: number) => void;
    readonly design_encode_wasm: (a: number, b: number) => [number, number, number];
    readonly estimate_browser_storage: () => any;
    readonly export_tensor_buffer_wasm: (a: number, b: number) => [number, number, number];
    readonly export_tensor_slice_wasm: (a: number) => [number, number, number];
    readonly geosparql_operation_wasm: (a: number, b: number) => [number, number, number];
    readonly is_opfs_block_cached: (a: number) => any;
    readonly pack_quins_into_superblock: (a: bigint, b: bigint, c: number, d: number) => [number, number, number];
    readonly qualiaportal_acoustic_enabled: (a: number) => number;
    readonly qualiaportal_acoustic_sab_byte_length: (a: number) => number;
    readonly qualiaportal_acoustic_sidecar_pinned: (a: number) => number;
    readonly qualiaportal_acoustic_uniform_bytes: (a: number) => [number, number, number];
    readonly qualiaportal_acoustic_uniform_float_count: (a: number) => number;
    readonly qualiaportal_acoustic_uniform_floats: (a: number) => [number, number, number];
    readonly qualiaportal_bake_cqt_sidecar_demo: (a: number, b: number) => [number, number, number];
    readonly qualiaportal_bake_stft_sidecar_demo: (a: number, b: number) => [number, number, number];
    readonly qualiaportal_camera_pitch: (a: number) => number;
    readonly qualiaportal_camera_yaw: (a: number) => number;
    readonly qualiaportal_camera_zoom: (a: number) => number;
    readonly qualiaportal_collapse_node_q: (a: number, b: number) => [number, number];
    readonly qualiaportal_control_pending: (a: number) => number;
    readonly qualiaportal_create_acoustic_sab: (a: number) => [number, number, number];
    readonly qualiaportal_drain_control_commands: (a: number, b: number) => number;
    readonly qualiaportal_drain_sonic_tokens: (a: number, b: number) => [number, number, number];
    readonly qualiaportal_encode_geometry: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_epistemic_q: (a: number) => number;
    readonly qualiaportal_last_parsed: (a: number) => any;
    readonly qualiaportal_load_json_scene: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_load_q42: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_mount_qapp: (a: number, b: number, c: number) => [number, number];
    readonly qualiaportal_navigate_to_node: (a: number, b: number) => [number, number];
    readonly qualiaportal_new: (a: any) => [number, number, number];
    readonly qualiaportal_observe_node_at: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly qualiaportal_operational_mode: (a: number) => number;
    readonly qualiaportal_poll_selected_node: (a: number) => number;
    readonly qualiaportal_publish_acoustic_sab: (a: number, b: any) => [number, number];
    readonly qualiaportal_push_control_command: (a: number, b: bigint) => number;
    readonly qualiaportal_push_sonic_token_raw: (a: number, b: bigint) => number;
    readonly qualiaportal_resize: (a: number, b: any, c: number, d: number) => [number, number];
    readonly qualiaportal_sample_telemetry: (a: number) => [number, number, number];
    readonly qualiaportal_select_node_at: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly qualiaportal_set_acoustic_enabled: (a: number, b: number) => void;
    readonly qualiaportal_set_camera: (a: number, b: number, c: number, d: number) => [number, number];
    readonly qualiaportal_set_display_mode: (a: number, b: number, c: number) => [number, number];
    readonly qualiaportal_set_standpoint: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly qualiaportal_set_telemetry: (a: number, b: number, c: number) => [number, number];
    readonly qualiaportal_sonic_token_pending: (a: number) => number;
    readonly qualiaportal_spatial_encode: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_standpoint_class: (a: number) => number;
    readonly qualiaportal_t_slice: (a: number) => number;
    readonly qualiaportal_t_window: (a: number) => number;
    readonly qualiaportal_tick: (a: number, b: any, c: number) => [number, number];
    readonly qualiaportal_tier: (a: number) => number;
    readonly qualiaportal_upload_tensor_buffer: (a: number, b: number, c: number) => [number, number];
    readonly read_opfs_block: (a: number) => any;
    readonly sample_browser_telemetry_wasm: () => [number, number, number];
    readonly spatial_encode_wasm: (a: number, b: number) => [number, number, number];
    readonly verify_superblock_ecc: (a: number, b: number) => [number, number];
    readonly write_opfs_block: (a: number, b: number, c: number) => any;
    readonly qualiaportal_selected_node_index: (a: number) => number;
    readonly __wbg_federatednodemanager_free: (a: number, b: number) => void;
    readonly __wbg_get_wasmoffloadintent_opcode: (a: number) => number;
    readonly __wbg_get_wasmoffloadintent_payload_size: (a: number) => number;
    readonly __wbg_get_wasmoffloadintent_priority: (a: number) => number;
    readonly __wbg_set_wasmoffloadintent_opcode: (a: number, b: number) => void;
    readonly __wbg_set_wasmoffloadintent_payload_size: (a: number, b: number) => void;
    readonly __wbg_set_wasmoffloadintent_priority: (a: number, b: number) => void;
    readonly __wbg_wasmoffloadintent_free: (a: number, b: number) => void;
    readonly __wbg_webengine_free: (a: number, b: number) => void;
    readonly create_canvas: (a: number, b: number) => [number, number, number];
    readonly enforce_rights_ontology: (a: bigint) => number;
    readonly federatednodemanager_discover_capabilities: (a: number) => number;
    readonly federatednodemanager_new: () => number;
    readonly federatednodemanager_offload_intent: (a: number, b: number) => [number, number, number, number];
    readonly intercept_computational_opcode: (a: number, b: number) => number;
    readonly intercept_pharmacogenomics_intent: (a: number, b: number) => number;
    readonly serialize_float64_array: (a: number, b: number) => any;
    readonly serialize_float_array: (a: number, b: number) => any;
    readonly wasmoffloadintent_new: (a: number, b: number, c: number) => number;
    readonly wasmoffloadintent_with_string_payload: (a: number, b: number, c: number, d: number) => number;
    readonly webengine_last_parsed: (a: number) => any;
    readonly webengine_load_json_scene: (a: number, b: number, c: number) => [number, number, number];
    readonly webengine_load_q42: (a: number, b: number, c: number) => [number, number, number];
    readonly webengine_mount_qapp: (a: number, b: number, c: number) => [number, number];
    readonly webengine_new: () => [number, number, number];
    readonly webengine_render_to_canvas: (a: number) => [number, number];
    readonly webizen_poll_agreements: () => [number, number];
    readonly webizen_propose_agreement: (a: any, b: number, c: number, d: number, e: number, f: number) => bigint;
    readonly webizen_sign_agreement: (a: bigint, b: number, c: number) => void;
    readonly init_panic_hook: () => void;
    readonly prune_and_validate_mesh: (a: bigint) => number;
    readonly align_sequences_wasm: (a: any) => [number, number, number];
    readonly black_scholes_wasm: (a: any) => [number, number, number];
    readonly check_drug_interactions_wasm: (a: any) => [number, number, number];
    readonly compile_query_to_json: (a: number, b: number) => [number, number];
    readonly compute_framingham_risk_wasm: (a: any) => [number, number, number];
    readonly compute_molecular_descriptors_wasm: (a: any) => [number, number, number];
    readonly compute_pid_step_wasm: (a: any) => [number, number, number];
    readonly compute_reaction_metrics_wasm: (a: any) => [number, number, number];
    readonly compute_thermochemistry_wasm: (a: any) => [number, number, number];
    readonly detect_functional_groups_wasm: (a: any) => [number, number, number];
    readonly evaluate_lipinski_wasm: (a: any) => [number, number, number];
    readonly execute_ntriples_query: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly forward_chain_wasm: (a: any) => [number, number, number];
    readonly get_engine_info: () => [number, number, number];
    readonly get_engine_version: () => [number, number];
    readonly list_capabilities_wasm: () => [number, number, number];
    readonly parse_cbor_ld_wasm: (a: number, b: number) => any;
    readonly parse_csv_wasm: (a: any) => [number, number, number];
    readonly parse_json_mapping_wasm: (a: any) => [number, number, number];
    readonly parse_json_wasm: (a: number, b: number) => any;
    readonly parse_n3logic_wasm: (a: number, b: number) => any;
    readonly parse_turtle_wasm: (a: number, b: number) => any;
    readonly resolve_lww_wasm: (a: any, b: any) => [number, number, number];
    readonly run_semantic_simulation: (a: any) => [number, number, number];
    readonly serialize_csv_wasm: (a: any) => [number, number, number];
    readonly serialize_json_wasm: (a: any) => [number, number, number];
    readonly serialize_rdf_wasm: (a: any) => [number, number, number];
    readonly simulate_gbm_path_wasm: (a: any) => [number, number, number];
    readonly solve_ode_exponential_decay_wasm: (a: any) => [number, number, number];
    readonly solve_sat_wasm: (a: any) => [number, number, number];
    readonly validate_fasta_wasm: (a: any) => [number, number, number];
    readonly validate_fhir_observation_wasm: (a: any) => [number, number, number];
    readonly validate_shacl_constraint_wasm: (a: any) => [number, number, number];
    readonly predict_receptor_binding_wasm: () => number;
    readonly wasm_bindgen__convert__closures_____invoke__h04e3064d3f666bd6: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h710533e233c29f5b: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h9b72b5b1e2f47c84: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h9b72b5b1e2f47c84_2: (a: number, b: number, c: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
