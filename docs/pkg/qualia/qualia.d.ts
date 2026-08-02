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
     * Phase 2 — drive the loaded mesh artefact with a kinematic joint (visible physics). `kind` is
     * `"prismatic"` (slide) or anything else = `"revolute"` (spin); `(ax,ay,az)` is the axis
     * (normalised here; defaults to +Y if zero); `rate` is rad/s (revolute) or units/s (prismatic).
     */
    animate_artefact(kind: string, ax: number, ay: number, az: number, rate: number): void;
    /**
     * Phase 2 — whether the artefact's proposed motion is currently being refused (clamped).
     */
    artefact_refused(): boolean;
    /**
     * Cold-bake CQT sidecar (log-spaced bins) for selected tensor node.
     */
    bake_cqt_sidecar_demo(frames: number): Uint8Array;
    /**
     * Cold-bake STFT sidecar for selected tensor node; pins bytes for hot frame reads.
     */
    bake_stft_sidecar_demo(frames: number): Uint8Array;
    /**
     * Cold-path Anatomy lifecycle receipt. Success requires a retained upload
     * and at least one presented renderer frame.
     */
    body_render_receipt(): any;
    /**
     * Phase 5 (affordability rail) — whether a device tier (`0`=Full, `1`=Eco, `2`=Reserve)
     * collapses a qapp's 3D scene to its 2D pane under the budget rule. Pure (no state change);
     * the qapp planner (`render::authoring`) uses the same `OperationalMode::supports_3d` source.
     */
    budget_collapses_3d(mode_code: number): boolean;
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
     * Phase 2 — visible **deterministic refusal**: slide the artefact along +X (prismatic joint)
     * into a world bound; the admission gate refuses poses that would leave the bound, so the
     * artefact deterministically halts at the wall instead of passing through.
     */
    demo_artefact_refusal(): void;
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
    /**
     * P9.2 — Load a `.10d` container asset: parse the section table, extract
     * the QuantizedMesh and Tensor10DNodes (provenance) sections, upload the
     * mesh to the GPU, and report node/triangle counts.
     *
     * **Governance fail-closed:** if the header carries
     * `FLAG_DEFAULT_DISPOSITION_REFUSE` (bit 0) and no attestation section is
     * present, the mesh is loaded for display but `description` marks it as
     * governance-refused — not citable as provenance until attested.
     *
     * Returns a JS object `{ vertex_count, triangle_count, provenance_mu, tier }`.
     */
    load_10d(bytes: Uint8Array): any;
    /**
     * S5.1 colour-by-load — like [`load_10d`] but paints the whole organ mesh a single uniform linear
     * RGBA. The host resolves each organ's body-system percept
     * (`qualia-client-core … AnatomyViewReport::paint_organs`) and passes that system's σ-derived colour
     * (`OrganPercept.percept.rgba`) here, so the 3D body is coloured by accumulated burden. Same
     * governance fail-closed as `load_10d`. (Deliberately parallels `load_10d` rather than sharing a
     * refactored helper: the portal path is wasm+GPU-only and not runtime-testable here, so the proven
     * `load_10d` is left untouched — unify them in the browser-test pass when the anatomy GLBs land.)
     */
    load_10d_colored(bytes: Uint8Array, r: number, g: number, b: number, a: number): any;
    /**
     * S5.8 (web) — load the whole body directly from a `.hmc` **anatomy pack**
     * bundle (see [`crate::bundle`]). Parses the bundle with the *shared* Rust
     * reader (the same code the native host uses — "one reader, both channels"),
     * reads each organ's sealed `.10d` plus its
     * [`AnatomyOrganMeta`](crate::render::anatomy_pack::AnatomyOrganMeta) (system
     * colour + anatomical position), and hands them to
     * [`Self::load_body_organs_colored`]. This is the pure-web render path — no
     * Tauri host / `webizen://` needed: the browser fetches one `.hmc` file and
     * renders the real body. Returns the same `{organs_loaded, organs_refused,
     * total_triangles}` summary.
     */
    load_body_from_qualia_bundle(bytes: Uint8Array): any;
    /**
     * Like [`Self::load_body_from_qualia_bundle`] but honours the **mixer's per-body-system
     * channels**: `system_levels` is a JS object `{ <system_id>: <level 0..1> }`. An organ whose
     * system level is ≤ 0 is omitted (muted); otherwise its colour alpha is scaled by the level.
     * (The mesh pipeline is currently opaque, so a nonzero level acts as show; smooth opacity lands
     * when the mesh pipeline gains alpha blending — mixer plan P2.) An absent/empty map shows every
     * system at full — so `load_body_from_qualia_bundle` is exactly this with no mixer applied.
     *
     * Decodes organs **in Rust** from the pack buffer — no per-organ JS `Uint8Array` materialisation.
     * That cut peak heap by ~1–2× pack size and is the phone-safe path.
     */
    load_body_from_qualia_bundle_mixed(bytes: Uint8Array, system_levels: any, disabled_parts: any): any;
    /**
     * S5.8 — load the **whole body** as a set of per-organ `.10d` meshes, each painted its body-system's
     * σ-derived RGBA, accumulated into one combined GPU mesh. This is the real-mesh render path.
     *
     * The CCF/HRA reference organs are authored in ONE shared body coordinate space (a brain's vertices
     * sit at the head, a bladder's at the pelvis, skin envelops the whole body), so this **preserves
     * each organ's TRUE position and relative size**: it accumulates the whole-body bounds across all
     * organs and applies **one global centre + scale**, rather than normalising each organ separately
     * (which would flatten proportions and shrink the full-body skin mesh to a dot). Governance
     * fail-closed per organ, as in `load_10d_colored`.
     *
     * `organs` is a JS `Array` of objects: `{ bytes: Uint8Array, r: f32, g: f32, b: f32, a: f32 }`
     * (per-organ colour). Any `x/y/z` fields are ignored — the mesh already carries its position.
     * Returns `{ organs_loaded, organs_refused, total_triangles }`.
     *
     * Prefer [`Self::load_body_from_qualia_bundle_mixed`] for packs — that path never materialises a
     * per-organ JS `Uint8Array` copy (critical on phones).
     */
    load_body_organs_colored(organs: Array<any>): any;
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
     * Read a `.hmc` pack's **manifest** without rendering — the list of parts the UI builds its
     * dynamic system + part selectors from. Returns a JS array of `{ key, label, system, systems }`
     * (one per `.10d` entry), so the demo can offer per-system *and* per-part select/deselect driven by
     * what is actually in the loaded pack, not a hardcoded list. Read-only.
     */
    pack_manifest(bytes: Uint8Array): any;
    /**
     * Returns selected tensor index, or `-1` if none / pick still pending.
     */
    poll_selected_node(): number;
    /**
     * Phase 1.4 — the **2D view** of the resident manifold: each tensor node's `project(.., Plane2D)`
     * shadow as a flat `[x0,y0,x1,y1,...]` array (world units, ~[-1,1]). The 3D scene draws the same
     * nodes through the GPU projector (the `Volume3D` view); both are the *one* manifold projection
     * seen two ways (see `manifold_project`). JS paints this on the 2D companion canvas.
     */
    project_resident_plane2d(time: number): Float32Array;
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
     * Enable/disable the **ambient particle field** — the mixer's "ambient" channel. Off by default
     * (a plain mesh/anatomy view has no use for the decorative random cloud); a Tensor10D upload
     * turns it on automatically because the particles then encode epistemic nodes.
     */
    set_ambient_enabled(on: boolean): void;
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
    set_temporal_slice(t_slice: number, t_window: number): void;
    sonic_token_pending(): number;
    spatial_encode(json: string): any;
    standpoint_class(): number;
    /**
     * Phase 2 — freeze the artefact (joint → identity, no world clamp).
     */
    stop_artefact_animation(): void;
    t_slice(): number;
    t_window(): number;
    tick(canvas: HTMLCanvasElement, dt_ms: number): void;
    tier(): number;
    /**
     * Import a 3D mesh asset (OBJ / STL / GLB bytes) and render it as a solid surface (Phase 1.2).
     * The mesh is centred on its bounding-box centroid and scaled so its largest extent is ~1.6
     * units — fitting the orbit camera's default frame (eye at distance 3.5, looking at the origin)
     * — then uploaded to the GPU. `hint` is an optional lowercase extension ("obj"/"stl"/"glb");
     * empty = sniff from the bytes. Returns the triangle count (0 if the GPU path isn't active).
     */
    upload_mesh_asset(bytes: Uint8Array, hint: string): number;
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

export function create_canvas(width: number, height: number): HTMLCanvasElement;

export function design_encode_wasm(json: string): any;

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

export function export_tensor_buffer_wasm(json: string): any;

export function export_tensor_slice_wasm(max_nodes: number): any;

export function geosparql_operation_wasm(json: string): any;

export function init_panic_hook(): void;

/**
 * Intercepts heavy computational opcodes and constructs a WASM offload intent.
 */
export function intercept_computational_opcode(opcode: number, payload_size: number): WasmOffloadIntent | undefined;

export function intercept_pharmacogenomics_intent(smiles: string): WasmOffloadIntent;

/**
 * Check whether a SuperBlock is cached in the OPFS vault.
 * Returns `true` if the `.qblk` file exists, `false` otherwise.
 */
export function is_opfs_block_cached(block_index: number): Promise<boolean>;

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

export function parse_json_wasm(payload: string): any;

/**
 * Bind a hardware WebGL2 Anatomy renderer before `QualiaPortal` construction.
 * This is selected only after capability probing proves that WebGPU has no
 * usable adapter and WebGL2 context creation succeeds.
 */
export function portal_init_webgl2(canvas: HTMLCanvasElement): boolean;

/**
 * Create the WebGPU device + surface asynchronously and stash it for the render loop to adopt.
 * JS calls this **once, awaited**, right after constructing the portal and **before** the render
 * loop starts — the canvas must still be context-free (no 2d context yet) so the WebGPU surface
 * can bind to it. Returns `true` if the GPU path is now armed; on `false`/throw the portal keeps
 * the canvas2d fallback.
 */
export function portal_init_webgpu(canvas: HTMLCanvasElement): Promise<boolean>;

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

export function sample_browser_telemetry_wasm(): any;

/**
 * Continuous Mathematical Serialization into Float64Array
 */
export function serialize_float64_array(data: Float64Array): Float64Array;

/**
 * Packs an array of floats into a Uint8Array strictly typed buffer to avoid IEEE-754 truncation
 */
export function serialize_float_array(data: Float32Array): Uint8Array;

export function spatial_encode_wasm(json: string): any;

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
    readonly estimate_browser_storage: () => any;
    readonly is_opfs_block_cached: (a: number) => any;
    readonly pack_quins_into_superblock: (a: bigint, b: bigint, c: number, d: number) => [number, number, number];
    readonly read_opfs_block: (a: number) => any;
    readonly verify_superblock_ecc: (a: number, b: number) => [number, number];
    readonly write_opfs_block: (a: number, b: number, c: number) => any;
    readonly __wbg_qualiaportal_free: (a: number, b: number) => void;
    readonly __wbg_webengine_free: (a: number, b: number) => void;
    readonly create_canvas: (a: number, b: number) => [number, number, number];
    readonly portal_init_webgl2: (a: any) => [number, number, number];
    readonly portal_init_webgpu: (a: any) => any;
    readonly qualiaportal_acoustic_enabled: (a: number) => number;
    readonly qualiaportal_acoustic_sab_byte_length: (a: number) => number;
    readonly qualiaportal_acoustic_sidecar_pinned: (a: number) => number;
    readonly qualiaportal_acoustic_uniform_bytes: (a: number) => [number, number, number];
    readonly qualiaportal_acoustic_uniform_float_count: (a: number) => number;
    readonly qualiaportal_acoustic_uniform_floats: (a: number) => [number, number, number];
    readonly qualiaportal_animate_artefact: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly qualiaportal_artefact_refused: (a: number) => number;
    readonly qualiaportal_bake_cqt_sidecar_demo: (a: number, b: number) => [number, number, number];
    readonly qualiaportal_bake_stft_sidecar_demo: (a: number, b: number) => [number, number, number];
    readonly qualiaportal_body_render_receipt: (a: number) => [number, number, number];
    readonly qualiaportal_budget_collapses_3d: (a: number, b: number) => number;
    readonly qualiaportal_camera_pitch: (a: number) => number;
    readonly qualiaportal_camera_yaw: (a: number) => number;
    readonly qualiaportal_camera_zoom: (a: number) => number;
    readonly qualiaportal_collapse_node_q: (a: number, b: number) => [number, number];
    readonly qualiaportal_control_pending: (a: number) => number;
    readonly qualiaportal_create_acoustic_sab: (a: number) => [number, number, number];
    readonly qualiaportal_demo_artefact_refusal: (a: number) => void;
    readonly qualiaportal_drain_control_commands: (a: number, b: number) => number;
    readonly qualiaportal_drain_sonic_tokens: (a: number, b: number) => [number, number, number];
    readonly qualiaportal_encode_geometry: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_epistemic_q: (a: number) => number;
    readonly qualiaportal_last_parsed: (a: number) => any;
    readonly qualiaportal_load_10d: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_load_10d_colored: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly qualiaportal_load_body_from_qualia_bundle: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_load_body_from_qualia_bundle_mixed: (a: number, b: number, c: number, d: any, e: any) => [number, number, number];
    readonly qualiaportal_load_body_organs_colored: (a: number, b: any) => [number, number, number];
    readonly qualiaportal_load_json_scene: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_load_q42: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_mount_qapp: (a: number, b: number, c: number) => [number, number];
    readonly qualiaportal_navigate_to_node: (a: number, b: number) => [number, number];
    readonly qualiaportal_new: (a: any) => [number, number, number];
    readonly qualiaportal_observe_node_at: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly qualiaportal_operational_mode: (a: number) => number;
    readonly qualiaportal_pack_manifest: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_poll_selected_node: (a: number) => number;
    readonly qualiaportal_project_resident_plane2d: (a: number, b: number) => [number, number];
    readonly qualiaportal_publish_acoustic_sab: (a: number, b: any) => [number, number];
    readonly qualiaportal_push_control_command: (a: number, b: bigint) => number;
    readonly qualiaportal_push_sonic_token_raw: (a: number, b: bigint) => number;
    readonly qualiaportal_resize: (a: number, b: any, c: number, d: number) => [number, number];
    readonly qualiaportal_sample_telemetry: (a: number) => [number, number, number];
    readonly qualiaportal_select_node_at: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly qualiaportal_set_acoustic_enabled: (a: number, b: number) => void;
    readonly qualiaportal_set_ambient_enabled: (a: number, b: number) => void;
    readonly qualiaportal_set_camera: (a: number, b: number, c: number, d: number) => [number, number];
    readonly qualiaportal_set_display_mode: (a: number, b: number, c: number) => [number, number];
    readonly qualiaportal_set_standpoint: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly qualiaportal_set_telemetry: (a: number, b: number, c: number) => [number, number];
    readonly qualiaportal_set_temporal_slice: (a: number, b: number, c: number) => void;
    readonly qualiaportal_sonic_token_pending: (a: number) => number;
    readonly qualiaportal_spatial_encode: (a: number, b: number, c: number) => [number, number, number];
    readonly qualiaportal_standpoint_class: (a: number) => number;
    readonly qualiaportal_stop_artefact_animation: (a: number) => void;
    readonly qualiaportal_t_slice: (a: number) => number;
    readonly qualiaportal_t_window: (a: number) => number;
    readonly qualiaportal_tick: (a: number, b: any, c: number) => [number, number];
    readonly qualiaportal_tier: (a: number) => number;
    readonly qualiaportal_upload_mesh_asset: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly qualiaportal_upload_tensor_buffer: (a: number, b: number, c: number) => [number, number];
    readonly webengine_last_parsed: (a: number) => any;
    readonly webengine_load_json_scene: (a: number, b: number, c: number) => [number, number, number];
    readonly webengine_load_q42: (a: number, b: number, c: number) => [number, number, number];
    readonly webengine_mount_qapp: (a: number, b: number, c: number) => [number, number];
    readonly webengine_new: () => [number, number, number];
    readonly webengine_render_to_canvas: (a: number) => [number, number];
    readonly qualiaportal_selected_node_index: (a: number) => number;
    readonly init_panic_hook: () => void;
    readonly design_encode_wasm: (a: number, b: number) => [number, number, number];
    readonly export_tensor_buffer_wasm: (a: number, b: number) => [number, number, number];
    readonly export_tensor_slice_wasm: (a: number) => [number, number, number];
    readonly geosparql_operation_wasm: (a: number, b: number) => [number, number, number];
    readonly sample_browser_telemetry_wasm: () => [number, number, number];
    readonly spatial_encode_wasm: (a: number, b: number) => [number, number, number];
    readonly parse_cbor_ld_wasm: (a: number, b: number) => any;
    readonly parse_json_wasm: (a: number, b: number) => any;
    readonly __wbg_federatednodemanager_free: (a: number, b: number) => void;
    readonly __wbg_get_wasmoffloadintent_opcode: (a: number) => number;
    readonly __wbg_get_wasmoffloadintent_payload_size: (a: number) => number;
    readonly __wbg_get_wasmoffloadintent_priority: (a: number) => number;
    readonly __wbg_set_wasmoffloadintent_opcode: (a: number, b: number) => void;
    readonly __wbg_set_wasmoffloadintent_payload_size: (a: number, b: number) => void;
    readonly __wbg_set_wasmoffloadintent_priority: (a: number, b: number) => void;
    readonly __wbg_wasmoffloadintent_free: (a: number, b: number) => void;
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
    readonly webizen_poll_agreements: () => [number, number];
    readonly webizen_propose_agreement: (a: any, b: number, c: number, d: number, e: number, f: number) => bigint;
    readonly webizen_sign_agreement: (a: bigint, b: number, c: number) => void;
    readonly prune_and_validate_mesh: (a: bigint) => number;
    readonly wasm_bindgen__convert__closures_____invoke__h8803f8c799f93ab4: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h0f3e9914b348256f: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h0f3e9914b348256f_2: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h0f3e9914b348256f_3: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h243b5e59773a58aa: (a: number, b: number, c: any, d: any) => void;
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
