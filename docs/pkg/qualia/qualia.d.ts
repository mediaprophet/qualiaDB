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
     * Replace the person-authored body fit. Pass JSON matching wellfare `BodyFit`.
     * Applied on the next `load_body_*` upload. Empty / invalid JSON resets to identity.
     */
    set_body_fit_json(json: string): void;
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

export class QualiaStore {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Clear all stored quints.
     */
    clear(): void;
    /**
     * Parse a CBOR-LD byte array (CBOR array of 4 or 5 unsigned integers) and
     * insert the resulting quin. Returns true on success, false on parse error.
     *
     * The qualiaDB binary gatekeeper (cbor_compiler.rs) requires this format:
     *   CBOR array header (0x84 or 0x85) followed by 4–5 CBOR unsigned integers.
     * All values are Lexicon-compressed u64 IDs assigned by the JS Lexicon.
     */
    insert_from_cbor_ld(data: Uint8Array): boolean;
    /**
     * Insert a quint (s, p, o, c, m).  Returns true on success.
     */
    insert_quin(s: bigint, p: bigint, o: bigint, c: bigint, m: bigint): boolean;
    /**
     * Total number of quints stored.
     */
    len(): number;
    constructor();
    /**
     * Return all quints in the given context as a flat Float64Array.
     */
    query_context(c: bigint): Float64Array;
    /**
     * Return all quints with the given predicate as a flat Float64Array.
     */
    query_predicate(p: bigint): Float64Array;
    /**
     * Return all quints with the given subject as a flat Float64Array
     * (groups of 5: [s,p,o,c,m, s,p,o,c,m, ...]).
     */
    query_subject(s: bigint): Float64Array;
}

/**
 * In-memory RDF store exposed to JS.  Load Turtle, run SPARQL SELECT/ASK/CONSTRUCT.
 */
export class WasmHealthStore {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Load a Turtle document into the store (appends — call on a fresh store to replace).
     */
    load_turtle(turtle: string): void;
    constructor();
    /**
     * Execute a SPARQL query; returns JSON SPARQL results string.
     */
    query(sparql: string): string;
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

/**
 * Evaluates input-output multipliers and total requirements via the Leontief inverse (I - A)^(-1).
 */
export function calculate_leontief_multipliers_wasm(val: any): any;

/**
 * Evaluates distributional and welfare metrics (Gini, Atkinson index, Palma ratio,
 * mean, median, P10, P90) and emits an auditable `CalculationReceipt`.
 */
export function calculate_welfare_metrics_wasm(val: any): any;

/**
 * Symbolic derivative. Input `{ expr, var }` (e.g. `{ "expr":"x^3 - 2*x^2 + 5",
 * "var":"x" }`) → `{ derivative }`. The result is simplified, then rendered with the
 * `Expr` `Display` (fully parenthesised). Errors on a parse failure.
 */
export function cas_differentiate_wasm(val: any): any;

/**
 * Numerically evaluate an expression given variable bindings. Input
 * `{ expr, bindings }` where `bindings` is an object of `name -> number`
 * (e.g. `{ "expr":"x^2 + 3*x + 2", "bindings":{ "x":4 } }`) → `{ value }`.
 * Errors if a referenced variable is unbound, or the result is non-finite
 * (division by zero, √negative, ln of a non-positive value).
 */
export function cas_evaluate_wasm(val: any): any;

/**
 * Distribute products over sums and expand small (≤ 8) positive integer powers, so the
 * result has no product/power over an additive child. Value-preserving. Input
 * `{ expr }` → `{ expanded }`. Errors on a parse failure.
 */
export function cas_expand_wasm(val: any): any;

/**
 * Factor a real quadratic `a·x² + b·x + c` into `a·(x − r₁)·(x — r₂)` (roots snapped to
 * integers/halves when numerically close). Input `{ a, b, c, var }` (`var` defaults to
 * `"x"`) → `{ factored }`. Errors when `a = 0` or the discriminant is negative (no real
 * factorisation).
 */
export function cas_factor_wasm(val: any): any;

/**
 * Algebraic simplification (constant folding + identity elimination, to a bounded
 * fixpoint). Input `{ expr }` → `{ simplified }`. Errors on a parse failure.
 */
export function cas_simplify_wasm(val: any): any;

/**
 * Symbolic roots of `a·x² + b·x + c = 0` as `(-b ± √(b²−4ac)) / (2a)` (simplified
 * `Expr` strings), plus their numeric values when the discriminant is non-negative.
 * Input `{ a, b, c }` → `{ roots:[{ expr, value }] }`. For `a = 0, b ≠ 0` returns the
 * single linear root `-c/b`; for `a = 0, b = 0` returns an empty list. A complex /
 * non-finite root value is reported as `null`.
 */
export function cas_solve_quadratic_wasm(val: any): any;

/**
 * But-for / reachability causation (`causal::caused`).
 */
export function causal_caused_wasm(val: any): any;

export function check_drug_interactions_wasm(val: any): any;

/**
 * Description-logic subsumption check (`check_subsumption_quin`).
 */
export function check_subsumption_wasm(val: any): any;

/**
 * Compiles a query string (SPARQL WHERE-clause or N-Triples pattern) to a JSON
 * description of the Webizen VM bytecode program.  Useful for playground inspection
 * and benchmarking the compilation pipeline without supplying a database.
 */
export function compile_query_to_json(query: string): string;

/**
 * Compile Turtle / N3 `sh:NodeShape` documents into ShapeSpec-compatible JSON (UE-050).
 */
export function compile_shacl_turtle_wasm(turtle: string): any;

export function compute_framingham_risk_wasm(val: any): any;

export function compute_molecular_descriptors_wasm(val: any): any;

/**
 * Evaluates ordinary least squares regression with complete diagnostics and receipt.
 */
export function compute_ols_diagnostics_wasm(val: any): any;

/**
 * Stateless PID controller step.
 * Returns { output, new_error, new_integral } for chaining into the next step.
 */
export function compute_pid_step_wasm(val: any): any;

export function compute_reaction_metrics_wasm(val: any): any;

export function compute_thermochemistry_wasm(val: any): any;

export function create_canvas(width: number, height: number): HTMLCanvasElement;

/**
 * AEAD decrypt + verify. Input `{ algorithm, key:{text|hex}, nonce:{text|hex},
 * ciphertext:{text|hex}, aad?:{text|hex} }` → `{ algorithm, plaintext_hex,
 * plaintext_utf8?, bytes }`. Fails closed on a bad tag / wrong key, nonce, or aad.
 */
export function crypto_aead_decrypt(val: any): any;

/**
 * AEAD encrypt. Input `{ algorithm, key:{text|hex}, nonce:{text|hex},
 * plaintext:{text|hex}, aad?:{text|hex} }` → `{ algorithm, ciphertext_hex, bytes }`.
 * `algorithm` ∈ aes256gcm | chacha20poly1305 | xchacha20poly1305. Key is 32 bytes;
 * nonce 12 (24 for xchacha). The caller owns the nonce — NEVER reuse a (key, nonce).
 */
export function crypto_aead_encrypt(val: any): any;

/**
 * BLAKE3 digest (256-bit).
 */
export function crypto_blake3(val: any): any;

/**
 * HKDF-SHA256 key derivation (RFC 5869). Input
 * `{ ikm:{text|hex}, salt?:{text|hex}, info?:{text|hex}, length }` →
 * `{ algorithm, okm_hex, length }`. `length` is output bytes (1..=8160).
 */
export function crypto_hkdf_sha256(val: any): any;

/**
 * SHA-256 digest of `{ text } | { hex }` → `{ algorithm, hex, bytes }`.
 */
export function crypto_sha256(val: any): any;

/**
 * SHA3-256 (Keccak) digest.
 */
export function crypto_sha3_256(val: any): any;

/**
 * SHA-512 digest.
 */
export function crypto_sha512(val: any): any;

export function design_encode_wasm(json: string): any;

export function detect_functional_groups_wasm(val: any): any;

/**
 * Enforces the rights ontology prior to transmission (e.g., checking DID constraints)
 */
export function enforce_rights_ontology(subject_did: bigint): boolean;

/**
 * Enumerate ASP stable-model world contexts (`enumerate_stable_models`).
 */
export function enumerate_stable_models_wasm(val: any): any;

/**
 * Query the browser's storage quota and current OPFS usage (bytes).
 *
 * Returns `{ quota: number, usage: number, available: number }`.
 * On mobile PWA the quota is typically 60 % of free disk space (Chrome) or
 * up to 1 GB on iOS Safari. Call this before a large ingest to check headroom.
 */
export function estimate_browser_storage(): Promise<any>;

/**
 * Evaluate deontic norms in a quin frame (`evaluate_deontic_contract`).
 */
export function evaluate_deontic_wasm(val: any): any;

/**
 * Evaluate epistemic claims (`evaluate_epistemic_frame`).
 */
export function evaluate_epistemic_wasm(val: any): any;

export function evaluate_lipinski_wasm(val: any): any;

/**
 * Evaluate an LTL formula against a quin trace (`evaluate_ltl_trace`).
 */
export function evaluate_ltl_trace_wasm(val: any): any;

/**
 * Evaluate all 7 N3 clinical rules against a Turtle document.
 *
 * Returns a JSON array of triggered patterns:
 * `[{"pattern":"ChronicSleepDebt","confidence":"high","routingLane":2,"n3Source":"sleep_debt.n3"},...]`
 *
 * Empty array = no concerns found in the supplied health data.
 * Routing lane 2 = BilateralMicroCommons (N3Logic implication rules requiring identity context).
 * Routing lane 0 = PassthroughStandard (simple threshold flags).
 */
export function evaluate_n3_rules(turtle: string): string;

/**
 * Exact sum `a + b`. Input `{ a: String, b: String }` -> `{ result }`.
 */
export function exact_bigint_add(val: any): any;

/**
 * Truncated division with remainder: `a = quotient*b + remainder`, remainder
 * taking the sign of `a` (toward-zero truncation, matching Rust `/` and `%`).
 * Input `{ a: String, b: String }` -> `{ quotient, remainder }`. Fails closed
 * (`Err`) when `b` is zero.
 */
export function exact_bigint_divmod(val: any): any;

/**
 * Factorial `n!` as an exact decimal string. Input `{ n: u32 }` ->
 * `{ result }`. Computed from the wasm-clean `BigInt` primitives (the same
 * `mul` loop the solver's `factorial_100_known_value` test uses), so e.g.
 * `n = 100` returns the full 158-digit value with no overflow.
 */
export function exact_bigint_factorial(val: any): any;

/**
 * Greatest common divisor `gcd(a, b)` (always non-negative; `gcd(0,0) = 0`).
 * Input `{ a: String, b: String }` -> `{ result }`.
 */
export function exact_bigint_gcd(val: any): any;

/**
 * Exact product `a * b`. Input `{ a: String, b: String }` -> `{ result }`.
 */
export function exact_bigint_mul(val: any): any;

/**
 * Exact integer power `base ^ exp`. Input `{ base: String, exp: u32 }` ->
 * `{ result }`. `base` is an arbitrary-precision decimal string; e.g.
 * `base = "2", exp = 100` returns `1267650600228229401496703205376`.
 */
export function exact_bigint_pow(val: any): any;

/**
 * Exact rational sum `a + b`, returned reduced and sign-normalised as `"p/q"`
 * (q > 0). Inputs are `"p/q"` strings (a bare `"p"` is read as `p/1`). Input
 * `{ a: String, b: String }` -> `{ result }`. E.g. `"1/3" + "1/6" = "1/2"`.
 */
export function exact_rational_add(val: any): any;

/**
 * Exact rational product `a * b`, returned reduced and sign-normalised as
 * `"p/q"` (q > 0). Inputs are `"p/q"` strings (a bare `"p"` is read as `p/1`).
 * Input `{ a: String, b: String }` -> `{ result }`. E.g. `"3/4" * "1/4" =
 * "3/16"`.
 */
export function exact_rational_mul(val: any): any;

export function execute_ntriples_query(query: string, db_bytes: Uint8Array, max_results: number): string;

export function export_tensor_buffer_wasm(json: string): any;

export function export_tensor_slice_wasm(max_nodes: number): any;

/**
 * Forward-chaining defeasible inference engine.
 * Input: `{ facts: ["bird", "penguin"], rules: [{ head: "flies", body: ["bird"], defeaters: ["penguin"] }, ...] }`
 * Output: `{ inferred: ["swims"] }`
 */
export function forward_chain_wasm(val: any): any;

/**
 * Fuzzy t-norm (Gödel min / Łukasiewicz / product).
 */
export function fuzzy_t_norm_wasm(val: any): any;

/**
 * Compute the 2-D convex hull of a point set.
 *
 * `points` is a flat `[x0, y0, x1, y1, ...]` array.
 * Returns `{ indices, vertex_count, hull_points }` where `hull_points`
 * is a flat `[x0, y0, ...]` array of hull vertices in order.
 *
 * Over the 5-point fixture `[[0,0],[1,0],[0.5,0.5],[1,1],[0,1]]` this
 * returns `indices = [0,1,3,4]`, `vertex_count = 4` — identical to the
 * native `execute_geometry_tool_json` test.
 */
export function geometry_convex_hull_2(val: any): any;

/**
 * Compute the Delaunay triangulation of a 2-D point set.
 */
export function geometry_delaunay_2(val: any): any;

/**
 * Execute any geometry tool via the JSON boundary — same function as
 * `execute_geometry_tool_json` on native. This is the full op surface
 * (`orientation_2`, `convex_hull_2`, `triangle_topology`, `mesh_topology`,
 * `delaunay_2`, `voronoi_2`, `nearest_site`).
 */
export function geometry_execute_json(args: string): string;

/**
 * Find the nearest site to a query point (brute-force).
 *
 * Returns the index of the nearest site, or -1 if the point set is empty.
 */
export function geometry_nearest_site(points: Float64Array, qx: number, qy: number): number;

/**
 * Robust 2-D orientation predicate.
 *
 * Returns `"clockwise"`, `"collinear"`, or `"counter_clockwise"` —
 * identical to the native `orientation_2` sign.
 */
export function geometry_orientation_2(ax: number, ay: number, bx: number, by: number, cx: number, cy: number): string;

/**
 * Numeric orientation sign (-1, 0, 1) for machine consumption.
 */
export function geometry_orientation_2_sign(ax: number, ay: number, bx: number, by: number, cx: number, cy: number): number;

/**
 * Compute the Voronoi diagram of a 2-D point set.
 */
export function geometry_voronoi_2(val: any): any;

export function geosparql_operation_wasm(json: string): any;

/**
 * Structured engine metadata for browser UIs and diagnostics.
 */
export function get_engine_info(): any;

/**
 * Returns the qualia-core-db crate version baked in at compile time (matches daemon `/health`).
 */
export function get_engine_version(): string;

/**
 * Machine-readable SHACL capability and constraint coverage manifest (QW-05).
 */
export function get_shacl_capability_manifest_wasm(): any;

/**
 * Poll WebGPU engine init stage text (same surface as the LLM package).
 */
export function get_webgpu_init_status(): string;

/**
 * Fuzzy RDF graph similarity (Ma, Li & Ma) — degree-aware Jaccard and Dice over two
 * sets of weighted triples. Terms are interned term ids (non-negative integers);
 * degrees are membership values in `[0,1]`. Two empty graphs are defined as 1.0.
 *
 * Input `{ g1:[[s,p,o,degree],..], g2:[[s,p,o,degree],..] }` ->
 * `{ jaccard, dice }`.
 */
export function graph_fuzzy_similarity(val: any): any;

/**
 * Knowledge-graph link prediction: score a set of candidate tails for a fixed
 * (head, relation) under TransE / DistMult / ComplEx / RotatE and rank them by
 * plausibility (higher = better). Input
 * `{ model, head:[f64], relation:[f64], candidates:[[f64],…], p?, top_k? }` →
 * `{ model, rank, ranking:[{index, score}] }` sorted best-first.
 */
export function graph_kge_predict(val: any): any;

/**
 * Knowledge-graph embedding plausibility score for a single triple
 * `(head, relation, tail)` under one of the four embedding families. Higher = more
 * plausible (translational models return the negative distance). Vector layout by
 * model (rank `k`):
 * * `transe` / `distmult` — head, relation, tail are length `k`.
 * * `complex` — all three length `2k` (`[re(0..k), im(k..2k)]`).
 * * `rotate` — head/tail length `2k` (`[re, im]`); relation length `k` (phase angles).
 *
 * `k` is inferred from the vector lengths; mismatched lengths fail closed.
 *
 * Input `{ model, head:[f64], relation:[f64], tail:[f64], p? }` -> `{ score, model,
 * rank }`. `p` (1 or 2) is the TransE norm order (default 2); ignored by other models.
 */
export function graph_kge_score(val: any): any;

/**
 * Single-source single-target shortest path over a directed, non-negative weighted
 * graph (Dijkstra, the engine's exact reference). The distance comes straight from
 * `solvers::graph_opt::dijkstra`; the node sequence is reconstructed by backtracking
 * on that distance field (`dist[u] + w == dist[v]`), so the math stays owned by the
 * solver.
 *
 * Input `{ edges:[[u,v,w],..], source, target, n? }` ->
 * `{ distance, reachable, path:[node,..] }` (path empty and reachable=false when
 * `target` is unreachable; `distance` is then null).
 */
export function graph_shortest_path(val: any): any;

/**
 * Spreading activation (Kornai, *Vector Semantics*) — propagate activation from seed
 * concepts through directed weighted edges, decaying each hop and pruning below a
 * threshold. Returns per-node total activation and a top-k relevance ranking.
 *
 * Input `{ edges:[[u,v,w],..], seeds:[[node,activation],..], decay, threshold?,
 * max_hops?, top_k?, n? }` -> `{ activation:[f64;n], ranking:[node,..] }`.
 */
export function graph_spreading_activation(val: any): any;

export function heart_rate_turtle_from_csv(content: string): string;

/**
 * Construct the volumetric renderer on the shared WebGPU device (no canvas).
 */
export function init_offscreen_renderer(width: number, height: number, particle_cap: number): Promise<void>;

export function init_panic_hook(): void;

/**
 * Create the process-wide WebGPU device used by graph accel, offscreen
 * rendering, and LLM decode. Idempotent.
 */
export function init_shared_webgpu(): Promise<void>;

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
 * Return the pinned Qualia JSON-LD 1.1 context + SHA-256 digest (UE-012).
 *
 * Packages should embed `context` and record `digest` on receipts — do not
 * fetch remote `@context` URLs at admission time.
 */
export function jsonld_context_digest_wasm(): any;

/**
 * Hohfeld correlative position for a jural opcode.
 */
export function jural_correlative_wasm(val: any): any;

/**
 * Determinant of a square matrix via LU (partial pivoting).
 * Input `{ rows, cols, data }` (rows==cols) → `{ determinant }`.
 */
export function la_determinant_wasm(val: any): any;

/**
 * Symmetric eigendecomposition (cyclic Jacobi). Input `{ rows, cols, data }`
 * (square, symmetric) → `{ eigenvalues:[..], eigenvectors:{rows,cols,data} }`
 * where eigenvector `j` is column `j` of the row-major `eigenvectors` matrix.
 */
export function la_eigen_symmetric_wasm(val: any): any;

/**
 * General (non-symmetric) eigenvalues via the characteristic polynomial.
 * Input `{ rows, cols, data }` (square) → `{ eigenvalues:[{re,im}] }`.
 */
export function la_eigenvalues_wasm(val: any): any;

/**
 * `C = A · B`. Input `{ a:{rows,cols,data}, b:{rows,cols,data} }`,
 * output `{ rows, cols, data }`. Errors on a shape mismatch (`a.cols != b.rows`).
 */
export function la_matmul_wasm(val: any): any;

/**
 * All complex roots of a real polynomial (Durand–Kerner). Input
 * `{ coeffs:[cₙ,…,c₁,c₀] }` (descending) → `{ degree, roots:[{re,im}] }`.
 */
export function la_polynomial_roots_wasm(val: any): any;

/**
 * Solve `A · x = b` for a square `A` via LU. Input `{ a:{rows,cols,data}, b:[..] }`
 * (b length == a.rows) → `{ x:[..] }`. Errors if `A` is singular.
 */
export function la_solve_wasm(val: any): any;

/**
 * Thin SVD `A = U·Σ·Vᵀ`. Input `{ rows, cols, data }` →
 * `{ singular_values:[..], u:{rows,cols,data}, v:{rows,cols,data} }`
 * (`u` is m×n, `v` is n×n; singular vectors are columns; values descending).
 */
export function la_svd_wasm(val: any): any;

/**
 * Transpose. Input `{ rows, cols, data }` → output `{ rows:cols, cols:rows, data }`.
 */
export function la_transpose_wasm(val: any): any;

/**
 * Capability names available in this WASM build.
 */
export function list_capabilities_wasm(): any;

/**
 * Names that are intentionally native-only (UE-035/044/045) — never stubbed in browser.
 */
export function list_native_only_capabilities_wasm(): any;

/**
 * Airy functions `Ai(x)` and `Bi(x)` (both, from one Maclaurin-series evaluation).
 * Input `{ x }` -> `{ ai, bi }`.
 */
export function num_airy_wasm(val: any): any;

/**
 * Euler's totient `phi(n)`, the Mobius `mu(n)`, divisor count `d(n)` and divisor sum
 * `sigma(n)` — the classic multiplicative arithmetic functions, all from the prime
 * factorization. Input `{ n }` -> `{ totient, mobius, divisor_count, divisor_sum }`.
 */
export function num_arithmetic_functions_wasm(val: any): any;

/**
 * Modified Bessel function of the first kind `I_n(x)`, integer order. Defined for all
 * real `x`. Input `{ n, x }` -> `{ value }`.
 */
export function num_bessel_i_wasm(val: any): any;

/**
 * Bessel function of the first kind `J_n(x)`, integer order (any sign), defined for all
 * real `x`. Input `{ n, x }` -> `{ value }`.
 */
export function num_bessel_j_wasm(val: any): any;

/**
 * Modified Bessel function of the second kind `K_n(x)`, integer order `n >= 0`. Requires
 * `x > 0`. Input `{ n, x }` -> `{ value }`; errors for `x <= 0`.
 */
export function num_bessel_k_wasm(val: any): any;

/**
 * Bessel function of the second kind `Y_n(x)`, integer order `n >= 0`. Requires `x > 0`
 * (singular at the origin) and `J_0(x) != 0`. Input `{ n, x }` -> `{ value }`; errors
 * for `x <= 0` or an ill-posed Wronskian solve.
 */
export function num_bessel_y_wasm(val: any): any;

/**
 * Binomial coefficient `C(n, k)` (exact integer at every step). Result is returned as a
 * decimal **string** since it may exceed `f64`/`u53` precision. Errors (fail closed) on
 * `u128` overflow. Input `{ n, k }` -> `{ value }` (value is a string).
 */
export function num_binomial_wasm(val: any): any;

/**
 * The `n`-th Catalan number, plus the Stirling numbers `S(n,k)` (second kind) and
 * `c(n,k)` (unsigned first kind). All exact integers as decimal **strings**; errors
 * (fail closed) on `u128` overflow. Input `{ n, k }` ->
 * `{ catalan, stirling_second, stirling_first }`.
 */
export function num_combinatorics_wasm(val: any): any;

/**
 * Natural cubic spline through `(xs, ys)` (xs strictly increasing), evaluated at each
 * query in `queries`. Errors on insufficient data, unsorted/duplicate nodes, or a
 * singular tridiagonal system. Input `{ xs:[..], ys:[..], queries:[..] }` ->
 * `{ values:[..] }`.
 */
export function num_cubic_spline_wasm(val: any): any;

/**
 * All positive divisors of `n`, ascending. Input `{ n }` -> `{ divisors:[..] }`.
 */
export function num_divisors_wasm(val: any): any;

/**
 * Factorial `n!` as an exact integer (decimal **string**; `f64` cannot hold it).
 * Errors (fail closed) for `n >= 35` (`35!` overflows `u128`).
 * Input `{ n }` -> `{ value }` (value is a string).
 */
export function num_factorial_wasm(val: any): any;

/**
 * Greatest common divisor and least common multiple of `a` and `b`.
 * Input `{ a, b }` -> `{ gcd, lcm }`.
 */
export function num_gcd_lcm_wasm(val: any): any;

/**
 * Deterministic Miller-Rabin primality test (exact for all `u64`).
 * Input `{ n }` -> `{ prime }`.
 */
export function num_is_prime_wasm(val: any): any;

/**
 * Evaluate the Lagrange interpolating polynomial through `(xs, ys)` at `x`. Errors on
 * empty/mismatched data or duplicate nodes. Input `{ xs:[..], ys:[..], x }` -> `{ value }`.
 */
export function num_lagrange_eval_wasm(val: any): any;

/**
 * Piecewise-linear interpolation of `(xs, ys)` (xs strictly increasing) at `x` (clamped
 * to the endpoints outside the range). Input `{ xs:[..], ys:[..], x }` -> `{ value }`.
 */
export function num_linear_interp_wasm(val: any): any;

/**
 * Minimize a built-in benchmark objective with the Nelder-Mead simplex method
 * (derivative-free, deterministic, zero-allocation `[f64; 4]` simplex).
 *
 * `objective` is one of `"sphere" | "rosenbrock" | "booth" | "matyas" | "sum_abs"`.
 * `start` is the initial 4-D point (missing components default to 0, extras ignored).
 * `max_iterations` (optional, default 1000) and `tolerance` (optional, default 1e-6)
 * configure the solver. Input
 * `{ objective, start:[..], max_iterations?, tolerance? }` ->
 * `{ best_point:[4], best_value, iterations, converged }`. Errors on an unknown objective.
 */
export function num_minimize_wasm(val: any): any;

/**
 * Modular multiplicative inverse: the `x` with `a*x ≡ 1 (mod m)`. Errors (fail closed)
 * when `gcd(a, m) != 1`. Input `{ a, m }` -> `{ inverse }`.
 */
export function num_mod_inverse_wasm(val: any): any;

/**
 * `(base^exp) mod modulus` by repeated squaring (overflow-safe via `u128`).
 * Input `{ base, exp, modulus }` -> `{ value }`.
 */
export function num_mod_pow_wasm(val: any): any;

/**
 * Newton divided-difference interpolation: build the coefficients from `(xs, ys)` and
 * evaluate the interpolant at `x`. Input `{ xs:[..], ys:[..], x }` ->
 * `{ value, coefficients:[..] }`.
 */
export function num_newton_eval_wasm(val: any): any;

/**
 * Smallest prime strictly greater than `n`. Input `{ n }` -> `{ next_prime }`.
 */
export function num_next_prime_wasm(val: any): any;

/**
 * Classical orthogonal polynomial `P_n(x)` by three-term recurrence. `kind` is one of
 * `"legendre" | "chebyshev_t" | "chebyshev_u" | "hermite" | "laguerre"`.
 * Input `{ kind, n, x }` -> `{ value }`; errors on an unknown kind.
 */
export function num_orthopoly_wasm(val: any): any;

/**
 * Number of integer partitions `p(n)` (ways to write `n` as an unordered sum of positive
 * integers). Input `{ n }` -> `{ value }`.
 */
export function num_partitions_wasm(val: any): any;

/**
 * Least-squares polynomial fit of degree `degree` to `(xs, ys)` (via the normal
 * equations). Returns coefficients in **ascending** order `[c0, c1, ..., c_degree]` (so
 * the polynomial is `sum c_k x^k`). Optionally evaluates the fit at each `queries` value.
 * Errors on too few points, `degree + 1 > n`, or a singular system.
 * Input `{ xs:[..], ys:[..], degree, queries?:[..] }` ->
 * `{ coefficients:[..], values:[..] }`.
 */
export function num_poly_fit_wasm(val: any): any;

/**
 * Prime factorization (trial division then Pollard's rho), correct across all `u64`.
 * Input `{ n }` -> `{ factors:[{ prime, exponent }] }`. Empty for `n < 2`.
 */
export function num_prime_factorize_wasm(val: any): any;

/**
 * Riemann zeta function `zeta(s)` for real `s > 1` (Euler-Maclaurin). Input `{ s }` ->
 * `{ value }`; errors for `s <= 1` (needs analytic continuation, out of this domain).
 */
export function num_zeta_wasm(val: any): any;

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

/**
 * Package exposure receipt: context + shapes + vibe AST digests (UE-053).
 */
export function package_exposure_manifest_wasm(shapes_json: string, vibe_program_cbor?: Uint8Array | null): any;

export function parse_cbor_ld_wasm(payload: Uint8Array): any;

export function parse_csv_wasm(val: any): any;

export function parse_heart_rate_csv_json(content: string): any;

export function parse_json_mapping_wasm(val: any): any;

export function parse_json_wasm(payload: string): any;

/**
 * Parse JSON-LD 1.1 text into packed quins (Civics primary semantic format).
 *
 * Profile: `application/ld+json`. Context must be embedded/pinned by the caller;
 * this binding does not fetch remote contexts.
 */
export function parse_jsonld_wasm(payload: string): any;

export function parse_n3logic_wasm(payload: string): any;

export function parse_sleep_csv_json(content: string): any;

export function parse_steps_csv_json(content: string): any;

export function parse_turtle_wasm(payload: string): any;

export function parse_weight_csv_json(content: string): any;

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

export function predict_receptor_binding_wasm(): number;

/**
 * Performs topological pruning and validates meshes prior to physics offloading
 */
export function prune_and_validate_mesh(mesh_id: bigint): boolean;

/**
 * RDFC-1.0 graph hash — **honest fail-closed** until a conforming implementation ships (UE-013).
 *
 * Never returns a digest labelled as RDFC-1.0. Optionally includes a
 * `provisional_spo_sha256` under profile `qualia:provisional-spo-sha256-v1`
 * for scaffolding only.
 */
export function rdfc10_graph_hash_wasm(val: any): any;

/**
 * Read a cached SuperBlock from the OPFS vault.
 *
 * Returns the raw 40 960 bytes as `Uint8Array`, or `null` if the block has not
 * been written yet (cache miss). Callers should fall back to an HTTP Range
 * request (see the JS `VFS` class) on cache miss.
 */
export function read_opfs_block(block_index: number): Promise<any>;

/**
 * Resolves two conflicting NQuin entries using Last-Writer-Wins semantics.
 * The Lamport clock is encoded in the metadata field; on ties, higher object wins.
 */
export function resolve_lww_wasm(local_val: any, remote_val: any): any;

/**
 * Route contradictions into an isolated context (`route_paraconsistent`).
 */
export function route_paraconsistent_wasm(val: any): any;

export function run_semantic_simulation(val: any): any;

export function sample_browser_telemetry_wasm(): any;

/**
 * Bounded stride sample of packed 48-byte Quins. Browser graphs cannot mmap
 * `.q42` files; this is the WASM-safe equivalent of `mmap_sample_quins`.
 */
export function sample_packed_quins_wasm(db_bytes: Uint8Array, max_quins: number): Uint8Array;

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

export function sleep_turtle_from_csv(content: string): string;

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

/**
 * One-way ANOVA F-test for equality of `k` group means. Input
 * `{ groups:[[..],[..],..] }` (≥ 2 groups, each non-empty, total > k) →
 * `{ f_statistic, p_value, df_between, df_within, ss_between, ss_within,
 * ms_between, ms_within }`. Errors on degenerate input.
 */
export function stats_anova_wasm(val: any): any;

/**
 * Pearson χ² goodness-of-fit test, `Σ(Oᵢ−Eᵢ)²/Eᵢ`, dof = k−1. Input
 * `{ observed:[..], expected:[..] }` (equal length ≥ 2, all expected > 0) →
 * `{ statistic, p_value, dof }`. Errors on length mismatch, len < 2, or a
 * non-positive expected count.
 */
export function stats_chi_square_gof_wasm(val: any): any;

/**
 * χ² test of independence on an R×C contingency table of counts. Input
 * `{ table:[[..],[..],..] }` (≥ 2 rows, ≥ 2 cols, rectangular, grand total > 0) →
 * `{ statistic, p_value, dof }` with `dof = (R−1)(C−1)`. Errors on a ragged or
 * undersized table.
 */
export function stats_chi_square_independence_wasm(val: any): any;

/**
 * χ² (chi-squared) distribution pdf/cdf at `x` with `k` degrees of freedom, plus
 * the upper-tail p-value. Input `{ x:f64, k:f64, p?:f64 }` (`k` > 0, `x` ≥ 0) →
 * `{ pdf, cdf, upper_p, quantile }`. `quantile` is the inverse-cdf at `p` when
 * supplied (0<p<1), else `null`.
 */
export function stats_chi_squared_dist_wasm(val: any): any;

/**
 * Pearson, Spearman, and Kendall correlation of two equal-length series, plus the
 * two-sided p-value for the Pearson coefficient. Input `{ x:[..], y:[..] }` →
 * `{ pearson, spearman, kendall, pearson_p_value }`. Each coefficient is `null`
 * when undefined (lengths differ, or n < 2); `pearson_p_value` is `null` for n < 3.
 */
export function stats_correlation_wasm(val: any): any;

/**
 * Full descriptive summary of a sample. Input `{ data:[..], sample?:bool }`
 * (`sample` defaults to `true` → Bessel-corrected variance/std) →
 * `{ n, sum, mean, variance, std_dev, min, max, median, q1, q3, skewness, kurtosis }`.
 * `variance`/`std_dev` are `null` when n < 2 in sample mode (no residual dof);
 * `skewness`/`kurtosis` are excess-kurtosis (Fisher) conventions.
 */
export function stats_describe_wasm(val: any): any;

/**
 * Fisher–Snedecor F-distribution: pdf and cdf at x with (d1, d2) degrees of
 * freedom, plus the inverse-cdf quantile when an optional `p` is supplied.
 * Input `{ x, d1, d2, p? }` → `{ pdf, cdf, quantile? }`.
 */
export function stats_fisher_f_wasm(val: any): any;

/**
 * Friedman test for k treatments across n blocks (e.g. classifiers × datasets).
 * Input `{ blocks:[[m1,…,mk], …] }` (each block length k, higher = better) →
 * `{ chi_square, chi_p_value, df, iman_davenport_f, f_p_value }`.
 */
export function stats_friedman_wasm(val: any): any;

/**
 * Simple (one-predictor) OLS linear regression of `y` on `x`. Input
 * `{ x:[..], y:[..] }` (equal length, n ≥ 3, x not constant) →
 * `{ slope, intercept, r_squared, residual_std_error, slope_std_error, slope_t,
 * slope_p_value, intercept_std_error, intercept_p_value, n }`. Errors on length
 * mismatch, n < 3, or zero-variance `x`.
 */
export function stats_linear_regression_wasm(val: any): any;

/**
 * McNemar's test for two paired binary classifiers. Input `{ b, c }` — the
 * discordant counts (b = first right / second wrong, c = first wrong / second
 * right) — → `{ statistic, p_value, dof }`. Continuity-corrected χ², dof 1.
 */
export function stats_mcnemar_wasm(val: any): any;

/**
 * Normal (Gaussian) distribution pdf/cdf/quantile at one point. Input
 * `{ x:f64, mu?:f64, sigma?:f64, p?:f64 }` (`mu` defaults 0, `sigma` defaults 1,
 * must be > 0) → `{ pdf, cdf, quantile }`. `pdf`/`cdf` are evaluated at `x`;
 * `quantile` is `Φ⁻¹(p)` when `p` is supplied (0<p<1), else `null`.
 */
export function stats_normal_wasm(val: any): any;

/**
 * One-sample t-test of the sample mean against `mu`. Input `{ data:[..], mu:f64 }`
 * → `{ t_statistic, p_value, degrees_of_freedom, ci_lower, ci_upper }`
 * (95% CI around the sample mean, t critical value). Errors if n < 2.
 */
export function stats_one_sample_t_wasm(val: any): any;

/**
 * Paired t-test (one-sample t-test of the paired differences against 0). Input
 * `{ a:[..], b:[..] }` (equal length) → `{ t_statistic, p_value,
 * degrees_of_freedom, ci_lower, ci_upper }`. Errors if lengths differ or n < 2.
 */
export function stats_paired_t_wasm(val: any): any;

/**
 * Linear-interpolated quantile (numpy "linear" / R type-7). Input
 * `{ data:[..], q:0.0..1.0 }` → `{ quantile }`. `q` is clamped to `[0,1]`.
 */
export function stats_quantile_wasm(val: any): any;

/**
 * Student's t-distribution pdf/cdf at `t` with `nu` degrees of freedom, plus the
 * two-sided p-value. Input `{ t:f64, nu:f64, p?:f64 }` (`nu` > 0) →
 * `{ pdf, cdf, two_sided_p, quantile }`. `quantile` is the inverse-cdf at `p`
 * when supplied (0<p<1), else `null`.
 */
export function stats_students_t_wasm(val: any): any;

/**
 * Two-sample t-test of `mean(a) − mean(b) = 0`. Input
 * `{ a:[..], b:[..], equal_var?:bool }` (`equal_var` defaults to `false` → the
 * Welch test; `true` → pooled Student) → `{ t_statistic, p_value,
 * degrees_of_freedom, mean_difference, ci_lower, ci_upper }`. Errors if either
 * sample has n < 2.
 */
export function stats_two_sample_t_wasm(val: any): any;

export function steps_turtle_from_csv(content: string): string;

/**
 * STIT: did agent bring about content?
 */
export function stit_brought_about_wasm(val: any): any;

/**
 * Look up a CODATA / SI-2019 physical constant by name, returning its value (in coherent
 * SI base units) and its physical dimension as the 7-vector.
 *
 * Input `{ name }` → `{ name, symbol, description, value, dimension:{..} }`.
 * Accepted names are those from `units_list_constants` (canonical name or symbol alias).
 */
export function units_constant(val: any): any;

/**
 * Convert a magnitude between two named units of the **same** physical dimension.
 * Affine (Celsius/Fahrenheit) and linear scales are both handled. Fails closed if the
 * units have different dimensions (e.g. `m` → `s`).
 *
 * Input `{ value, from, to }` → `{ value, from, to, dimension:{..} }`.
 */
export function units_convert(val: any): any;

/**
 * List every CODATA constant available to `units_constant`, with value, symbol,
 * description and dimension. Takes an empty object `{}`.
 * Input `{}` → `{ constants:[{name,symbol,description,value,dimension}] }`.
 */
export function units_list_constants(_val: any): any;

/**
 * List every unit the engine can convert between, with a human label and its dimension
 * 7-vector. Takes an empty object `{}`. Input `{}` → `{ units:[{symbol,label,dimension}] }`.
 */
export function units_list_units(_val: any): any;

/**
 * Multiply or divide two dimensioned quantities, composing their dimensions. Each
 * quantity is `{ value, unit }`; the unit string is resolved to its SI factor so the
 * result value is in coherent SI base units, and the result dimension is returned as the
 * 7-vector. `divide` fails closed on a zero divisor.
 *
 * Input `{ a:{value,unit}, b:{value,unit}, op:"multiply"|"divide" }`
 * → `{ value, dimension:{..} }`.
 */
export function units_quantity_op(val: any): any;

export function validate_fasta_wasm(val: any): any;

export function validate_fhir_observation_wasm(val: any): any;

/**
 * Evaluate a named policy constraint against a single quint (s,p,o,c,m).
 *
 * Supported constraint names:
 *   "cooperative_obligation" — PermissiveCommons work obligation gate (lane 1)
 *   "guardian_identity"      — BilateralMicroCommons guardian auth gate (lane 2)
 *   "commercial_block"       — BilateralMicroCommons anti-commercial gate (lane 2)
 *
 * Returns JSON: `{"passed":bool,"routingLane":N}`
 */
export function validate_health_quin(constraint: string, s: bigint, p: bigint, o: bigint, c: bigint, m: bigint): string;

/**
 * Validate a Turtle document against built-in health shapes (SPARQL ASK constraints).
 * Returns a JSON string: `{"valid":bool,"checked":N,"violations":[{"shape":"...","message":"..."}]}`
 */
export function validate_health_turtle(turtle: string): string;

export function validate_shacl_constraint_wasm(val: any): any;

/**
 * Validates raw packed 48-byte Quins against a list of JSON ShapeSpecs.
 */
export function validate_shacl_graph_wasm(db_bytes: Uint8Array, shapes_json: string): any;

/**
 * Full graph SHACL validation from N3/N-Triples data and JSON ShapeSpecs.
 * Returns the complete `ValidationReport` preserving conforms, focus node, path,
 * severity, and constraint component.
 */
export function validate_shacl_json_wasm(data_n3: string, shapes_json: string): any;

/**
 * Values abuse-check (agency.n3 G1/G1' personhood guard) — WASM surface for Civics.
 */
export function values_check_wasm(val: any): any;

/**
 * Consent non-coerced guard (`capacity::detect_duress` inverted).
 */
export function values_consent_non_coerced_wasm(val: any): any;

/**
 * Harm-below-ceiling guard (wasm-safe numeric; CAS marginal-harm stays native).
 */
export function values_harm_below_ceiling_wasm(val: any): any;

/**
 * Serialize vault biometric records (JSON array from wf-biometrics IDB store) → Turtle.
 */
export function vault_biometrics_to_turtle(json: string): string;

/**
 * Serialize vault diet log entries (JSON array from wf-dl IDB store) → Turtle.
 */
export function vault_diet_to_turtle(json: string): string;

/**
 * Serialize vault medication records (JSON array from wf-meds IDB store) → Turtle.
 */
export function vault_meds_to_turtle(json: string): string;

/**
 * Verify a signed law package (JSON) against an Ed25519 public key.
 */
export function verify_law_package_wasm(json: string, public_key: Uint8Array): boolean;

/**
 * Validate ECC parity for every NQuin in a raw SuperBlock.
 *
 * Returns JSON: `{"valid":bool,"total":N,"bad":[indices...]}`
 * A non-empty `bad` array indicates sector corruption.
 */
export function verify_superblock_ecc(block_bytes: Uint8Array): string;

export function wasm_convex_hull_2d(points_flat: Float64Array): Uint32Array;

export function wasm_delaunay_triangulation_2d(points_flat: Float64Array): Uint32Array;

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

export function weight_turtle_from_csv(content: string): string;

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

/**
 * Forward discrete Fourier transform `X[k] = Σ_n x[n] e^{-2πi kn/N}`
 * (un-normalized, forward sign convention). f64-exact CPU reference path.
 *
 * Input `{ data:[..] }` (real signal) OR `{ re:[..], im:[..] }` (complex signal).
 * Output `{ re:[..], im:[..], magnitude:[..], n }`.
 */
export function xform_dft(val: any): any;

/**
 * Inverse discrete Fourier transform `x[n] = (1/N) Σ_k X[k] e^{+2πi kn/N}`.
 * Round-trips `xform_dft` to ~1e-9.
 *
 * Input the spectrum as `{ re:[..], im:[..] }` (complex bins) OR `{ data:[..] }`
 * (real bins → imaginary parts taken as 0).
 * Output `{ re:[..], im:[..], magnitude:[..], n }` — the recovered samples.
 */
export function xform_idft(val: any): any;

/**
 * Numerical Laplace transform `L{f}(s) = ∫₀^∞ e^{-st} f(t) dt` by Simpson
 * quadrature, for a built-in time-function family (so a deterministic kernel
 * crosses the JS boundary instead of an arbitrary closure):
 * * `"one"`   → f(t)=1            (closed form 1/s)
 * * `"t"`     → f(t)=t            (1/s²)
 * * `"exp"`   → f(t)=e^{a·t}      (1/(s-a) for s>a)
 * * `"poly"`  → f(t)=tⁿ           (n!/s^{n+1}); supply `n`
 * * `"sin"`   → f(t)=sin(a·t)     (a/(s²+a²))
 * * `"cos"`   → f(t)=cos(a·t)     (s/(s²+a²))
 * `a` defaults to 1, `n` defaults to 1. Requires `s>0`, `t_max>0`, even `steps≥2`.
 *
 * Input `{ fn, s, t_max, steps, a?, n? }`. Output `{ value, s, t_max, steps }`.
 */
export function xform_laplace_numeric(val: any): any;

/**
 * Symbolic Laplace transform of a polynomial in `t` from the table the CAS can
 * represent: a sum of `coeff · t^power` terms (constants are `power = 0`).
 * Returns the resulting `Expr` in `s` as a pretty string and, when `s` is
 * supplied, its numeric value `L{f}(s)`. Fails closed (`NotTransformable`) on
 * anything outside constants / integer powers / their linear combinations.
 *
 * Input `{ terms:[{coeff, power}, ..], s? }`. Output `{ expr, value? }`.
 */
export function xform_laplace_table(val: any): any;

/**
 * Closed form of the geometric `aⁿ u[n]` Z-transform `X(z) = 1/(1 - a z^{-1})`
 * (valid for `|z| > |a|`). Fails closed where the denominator vanishes / at `z = 0`.
 *
 * Input `{ a, z_re, z_im }`. Output `{ re, im, magnitude }`.
 */
export function xform_z_geometric(val: any): any;

/**
 * Z-transform of a finite causal sequence evaluated at a complex point `z`:
 * `X(z) = Σ_{n=0}^{N-1} x[n] z^{-n}`. Fails closed at `z = 0`.
 *
 * Input `{ x:[..], z_re, z_im }`. Output `{ re, im, magnitude }`.
 */
export function xform_z_transform(val: any): any;

/**
 * Closed form of the unit-step `u[n]` Z-transform `X(z) = z/(z-1)`
 * (valid for `|z| > 1`). Fails closed at `z = 0` or `z = 1`.
 *
 * Input `{ z_re, z_im }`. Output `{ re, im, magnitude }`.
 */
export function xform_z_unit_step(val: any): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly geometry_convex_hull_2: (a: any) => [number, number, number];
    readonly geometry_delaunay_2: (a: any) => [number, number, number];
    readonly geometry_execute_json: (a: number, b: number) => [number, number, number, number];
    readonly geometry_nearest_site: (a: number, b: number, c: number, d: number) => number;
    readonly geometry_orientation_2: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly geometry_orientation_2_sign: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly geometry_voronoi_2: (a: any) => [number, number, number];
    readonly stats_anova_wasm: (a: any) => [number, number, number];
    readonly stats_chi_square_gof_wasm: (a: any) => [number, number, number];
    readonly stats_chi_square_independence_wasm: (a: any) => [number, number, number];
    readonly stats_chi_squared_dist_wasm: (a: any) => [number, number, number];
    readonly stats_correlation_wasm: (a: any) => [number, number, number];
    readonly stats_describe_wasm: (a: any) => [number, number, number];
    readonly stats_fisher_f_wasm: (a: any) => [number, number, number];
    readonly stats_friedman_wasm: (a: any) => [number, number, number];
    readonly stats_linear_regression_wasm: (a: any) => [number, number, number];
    readonly stats_mcnemar_wasm: (a: any) => [number, number, number];
    readonly stats_normal_wasm: (a: any) => [number, number, number];
    readonly stats_one_sample_t_wasm: (a: any) => [number, number, number];
    readonly stats_paired_t_wasm: (a: any) => [number, number, number];
    readonly stats_quantile_wasm: (a: any) => [number, number, number];
    readonly stats_students_t_wasm: (a: any) => [number, number, number];
    readonly stats_two_sample_t_wasm: (a: any) => [number, number, number];
    readonly wasm_convex_hull_2d: (a: number, b: number) => [number, number, number];
    readonly wasm_delaunay_triangulation_2d: (a: number, b: number) => [number, number, number];
    readonly xform_dft: (a: any) => [number, number, number];
    readonly xform_idft: (a: any) => [number, number, number];
    readonly xform_laplace_numeric: (a: any) => [number, number, number];
    readonly xform_laplace_table: (a: any) => [number, number, number];
    readonly xform_z_geometric: (a: any) => [number, number, number];
    readonly xform_z_transform: (a: any) => [number, number, number];
    readonly xform_z_unit_step: (a: any) => [number, number, number];
    readonly __wbg_federatednodemanager_free: (a: number, b: number) => void;
    readonly __wbg_get_wasmoffloadintent_opcode: (a: number) => number;
    readonly __wbg_get_wasmoffloadintent_payload_size: (a: number) => number;
    readonly __wbg_get_wasmoffloadintent_priority: (a: number) => number;
    readonly __wbg_qualiaportal_free: (a: number, b: number) => void;
    readonly __wbg_set_wasmoffloadintent_opcode: (a: number, b: number) => void;
    readonly __wbg_set_wasmoffloadintent_payload_size: (a: number, b: number) => void;
    readonly __wbg_set_wasmoffloadintent_priority: (a: number, b: number) => void;
    readonly __wbg_wasmoffloadintent_free: (a: number, b: number) => void;
    readonly __wbg_webengine_free: (a: number, b: number) => void;
    readonly black_scholes_wasm: (a: any) => [number, number, number];
    readonly calculate_leontief_multipliers_wasm: (a: any) => [number, number, number];
    readonly calculate_welfare_metrics_wasm: (a: any) => [number, number, number];
    readonly compute_molecular_descriptors_wasm: (a: any) => [number, number, number];
    readonly compute_ols_diagnostics_wasm: (a: any) => [number, number, number];
    readonly compute_pid_step_wasm: (a: any) => [number, number, number];
    readonly compute_reaction_metrics_wasm: (a: any) => [number, number, number];
    readonly compute_thermochemistry_wasm: (a: any) => [number, number, number];
    readonly create_canvas: (a: number, b: number) => [number, number, number];
    readonly detect_functional_groups_wasm: (a: any) => [number, number, number];
    readonly enforce_rights_ontology: (a: bigint) => number;
    readonly evaluate_lipinski_wasm: (a: any) => [number, number, number];
    readonly federatednodemanager_discover_capabilities: (a: number) => number;
    readonly federatednodemanager_new: () => number;
    readonly federatednodemanager_offload_intent: (a: number, b: number) => [number, number, number, number];
    readonly intercept_computational_opcode: (a: number, b: number) => number;
    readonly intercept_pharmacogenomics_intent: (a: number, b: number) => number;
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
    readonly qualiaportal_set_body_fit_json: (a: number, b: number, c: number) => void;
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
    readonly run_semantic_simulation: (a: any) => [number, number, number];
    readonly serialize_float64_array: (a: number, b: number) => any;
    readonly serialize_float_array: (a: number, b: number) => any;
    readonly simulate_gbm_path_wasm: (a: any) => [number, number, number];
    readonly solve_ode_exponential_decay_wasm: (a: any) => [number, number, number];
    readonly solve_sat_wasm: (a: any) => [number, number, number];
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
    readonly qualiaportal_selected_node_index: (a: number) => number;
    readonly init_panic_hook: () => void;
    readonly prune_and_validate_mesh: (a: bigint) => number;
    readonly check_drug_interactions_wasm: (a: any) => [number, number, number];
    readonly compute_framingham_risk_wasm: (a: any) => [number, number, number];
    readonly exact_bigint_add: (a: any) => [number, number, number];
    readonly exact_bigint_divmod: (a: any) => [number, number, number];
    readonly exact_bigint_factorial: (a: any) => [number, number, number];
    readonly exact_bigint_gcd: (a: any) => [number, number, number];
    readonly exact_bigint_mul: (a: any) => [number, number, number];
    readonly exact_bigint_pow: (a: any) => [number, number, number];
    readonly exact_rational_add: (a: any) => [number, number, number];
    readonly exact_rational_mul: (a: any) => [number, number, number];
    readonly validate_fhir_observation_wasm: (a: any) => [number, number, number];
    readonly cas_differentiate_wasm: (a: any) => [number, number, number];
    readonly cas_evaluate_wasm: (a: any) => [number, number, number];
    readonly cas_expand_wasm: (a: any) => [number, number, number];
    readonly cas_factor_wasm: (a: any) => [number, number, number];
    readonly cas_simplify_wasm: (a: any) => [number, number, number];
    readonly cas_solve_quadratic_wasm: (a: any) => [number, number, number];
    readonly compile_query_to_json: (a: number, b: number) => [number, number];
    readonly compile_shacl_turtle_wasm: (a: number, b: number) => [number, number, number];
    readonly execute_ntriples_query: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly forward_chain_wasm: (a: any) => [number, number, number];
    readonly get_engine_info: () => [number, number, number];
    readonly get_engine_version: () => [number, number];
    readonly get_shacl_capability_manifest_wasm: () => any;
    readonly get_webgpu_init_status: () => [number, number];
    readonly init_offscreen_renderer: (a: number, b: number, c: number) => any;
    readonly init_shared_webgpu: () => any;
    readonly jsonld_context_digest_wasm: () => [number, number, number];
    readonly list_capabilities_wasm: () => [number, number, number];
    readonly list_native_only_capabilities_wasm: () => [number, number, number];
    readonly package_exposure_manifest_wasm: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly parse_cbor_ld_wasm: (a: number, b: number) => any;
    readonly parse_csv_wasm: (a: any) => [number, number, number];
    readonly parse_json_mapping_wasm: (a: any) => [number, number, number];
    readonly parse_json_wasm: (a: number, b: number) => any;
    readonly parse_jsonld_wasm: (a: number, b: number) => [number, number, number];
    readonly parse_n3logic_wasm: (a: number, b: number) => any;
    readonly parse_turtle_wasm: (a: number, b: number) => any;
    readonly rdfc10_graph_hash_wasm: (a: any) => [number, number, number];
    readonly resolve_lww_wasm: (a: any, b: any) => [number, number, number];
    readonly sample_packed_quins_wasm: (a: number, b: number, c: number) => [number, number, number, number];
    readonly serialize_csv_wasm: (a: any) => [number, number, number];
    readonly serialize_json_wasm: (a: any) => [number, number, number];
    readonly serialize_rdf_wasm: (a: any) => [number, number, number];
    readonly validate_shacl_constraint_wasm: (a: any) => [number, number, number];
    readonly validate_shacl_graph_wasm: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly validate_shacl_json_wasm: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly verify_law_package_wasm: (a: number, b: number, c: number, d: number) => number;
    readonly align_sequences_wasm: (a: any) => [number, number, number];
    readonly causal_caused_wasm: (a: any) => [number, number, number];
    readonly check_subsumption_wasm: (a: any) => [number, number, number];
    readonly crypto_aead_decrypt: (a: any) => [number, number, number];
    readonly crypto_aead_encrypt: (a: any) => [number, number, number];
    readonly crypto_blake3: (a: any) => [number, number, number];
    readonly crypto_hkdf_sha256: (a: any) => [number, number, number];
    readonly crypto_sha256: (a: any) => [number, number, number];
    readonly crypto_sha3_256: (a: any) => [number, number, number];
    readonly crypto_sha512: (a: any) => [number, number, number];
    readonly enumerate_stable_models_wasm: (a: any) => [number, number, number];
    readonly evaluate_deontic_wasm: (a: any) => [number, number, number];
    readonly evaluate_epistemic_wasm: (a: any) => [number, number, number];
    readonly evaluate_ltl_trace_wasm: (a: any) => [number, number, number];
    readonly fuzzy_t_norm_wasm: (a: any) => [number, number, number];
    readonly graph_fuzzy_similarity: (a: any) => [number, number, number];
    readonly graph_kge_predict: (a: any) => [number, number, number];
    readonly graph_kge_score: (a: any) => [number, number, number];
    readonly graph_shortest_path: (a: any) => [number, number, number];
    readonly graph_spreading_activation: (a: any) => [number, number, number];
    readonly jural_correlative_wasm: (a: any) => [number, number, number];
    readonly route_paraconsistent_wasm: (a: any) => [number, number, number];
    readonly stit_brought_about_wasm: (a: any) => [number, number, number];
    readonly validate_fasta_wasm: (a: any) => [number, number, number];
    readonly values_check_wasm: (a: any) => [number, number, number];
    readonly values_consent_non_coerced_wasm: (a: any) => [number, number, number];
    readonly values_harm_below_ceiling_wasm: (a: any) => [number, number, number];
    readonly predict_receptor_binding_wasm: () => number;
    readonly estimate_browser_storage: () => any;
    readonly is_opfs_block_cached: (a: number) => any;
    readonly pack_quins_into_superblock: (a: bigint, b: bigint, c: number, d: number) => [number, number, number];
    readonly read_opfs_block: (a: number) => any;
    readonly units_constant: (a: any) => [number, number, number];
    readonly units_convert: (a: any) => [number, number, number];
    readonly units_list_constants: (a: any) => [number, number, number];
    readonly units_list_units: (a: any) => [number, number, number];
    readonly units_quantity_op: (a: any) => [number, number, number];
    readonly verify_superblock_ecc: (a: number, b: number) => [number, number];
    readonly write_opfs_block: (a: number, b: number, c: number) => any;
    readonly design_encode_wasm: (a: number, b: number) => [number, number, number];
    readonly export_tensor_buffer_wasm: (a: number, b: number) => [number, number, number];
    readonly export_tensor_slice_wasm: (a: number) => [number, number, number];
    readonly geosparql_operation_wasm: (a: number, b: number) => [number, number, number];
    readonly sample_browser_telemetry_wasm: () => [number, number, number];
    readonly spatial_encode_wasm: (a: number, b: number) => [number, number, number];
    readonly la_determinant_wasm: (a: any) => [number, number, number];
    readonly la_eigen_symmetric_wasm: (a: any) => [number, number, number];
    readonly la_eigenvalues_wasm: (a: any) => [number, number, number];
    readonly la_matmul_wasm: (a: any) => [number, number, number];
    readonly la_polynomial_roots_wasm: (a: any) => [number, number, number];
    readonly la_solve_wasm: (a: any) => [number, number, number];
    readonly la_svd_wasm: (a: any) => [number, number, number];
    readonly la_transpose_wasm: (a: any) => [number, number, number];
    readonly num_airy_wasm: (a: any) => [number, number, number];
    readonly num_arithmetic_functions_wasm: (a: any) => [number, number, number];
    readonly num_bessel_i_wasm: (a: any) => [number, number, number];
    readonly num_bessel_j_wasm: (a: any) => [number, number, number];
    readonly num_bessel_k_wasm: (a: any) => [number, number, number];
    readonly num_bessel_y_wasm: (a: any) => [number, number, number];
    readonly num_binomial_wasm: (a: any) => [number, number, number];
    readonly num_combinatorics_wasm: (a: any) => [number, number, number];
    readonly num_cubic_spline_wasm: (a: any) => [number, number, number];
    readonly num_divisors_wasm: (a: any) => [number, number, number];
    readonly num_factorial_wasm: (a: any) => [number, number, number];
    readonly num_gcd_lcm_wasm: (a: any) => [number, number, number];
    readonly num_is_prime_wasm: (a: any) => [number, number, number];
    readonly num_lagrange_eval_wasm: (a: any) => [number, number, number];
    readonly num_linear_interp_wasm: (a: any) => [number, number, number];
    readonly num_minimize_wasm: (a: any) => [number, number, number];
    readonly num_mod_inverse_wasm: (a: any) => [number, number, number];
    readonly num_mod_pow_wasm: (a: any) => [number, number, number];
    readonly num_newton_eval_wasm: (a: any) => [number, number, number];
    readonly num_next_prime_wasm: (a: any) => [number, number, number];
    readonly num_orthopoly_wasm: (a: any) => [number, number, number];
    readonly num_partitions_wasm: (a: any) => [number, number, number];
    readonly num_poly_fit_wasm: (a: any) => [number, number, number];
    readonly num_prime_factorize_wasm: (a: any) => [number, number, number];
    readonly num_zeta_wasm: (a: any) => [number, number, number];
    readonly __wbg_qualiastore_free: (a: number, b: number) => void;
    readonly qualiastore_clear: (a: number) => void;
    readonly qualiastore_insert_from_cbor_ld: (a: number, b: number, c: number) => number;
    readonly qualiastore_insert_quin: (a: number, b: bigint, c: bigint, d: bigint, e: bigint, f: bigint) => number;
    readonly qualiastore_len: (a: number) => number;
    readonly qualiastore_new: () => number;
    readonly qualiastore_query_context: (a: number, b: bigint) => any;
    readonly qualiastore_query_predicate: (a: number, b: bigint) => any;
    readonly qualiastore_query_subject: (a: number, b: bigint) => any;
    readonly __wbg_wasmhealthstore_free: (a: number, b: number) => void;
    readonly evaluate_n3_rules: (a: number, b: number) => [number, number];
    readonly heart_rate_turtle_from_csv: (a: number, b: number) => [number, number, number, number];
    readonly parse_heart_rate_csv_json: (a: number, b: number) => [number, number, number];
    readonly parse_sleep_csv_json: (a: number, b: number) => [number, number, number];
    readonly parse_steps_csv_json: (a: number, b: number) => [number, number, number];
    readonly parse_weight_csv_json: (a: number, b: number) => [number, number, number];
    readonly sleep_turtle_from_csv: (a: number, b: number) => [number, number, number, number];
    readonly steps_turtle_from_csv: (a: number, b: number) => [number, number, number, number];
    readonly validate_health_quin: (a: number, b: number, c: bigint, d: bigint, e: bigint, f: bigint, g: bigint) => [number, number];
    readonly validate_health_turtle: (a: number, b: number) => [number, number];
    readonly vault_biometrics_to_turtle: (a: number, b: number) => [number, number, number, number];
    readonly vault_diet_to_turtle: (a: number, b: number) => [number, number, number, number];
    readonly vault_meds_to_turtle: (a: number, b: number) => [number, number, number, number];
    readonly wasmhealthstore_load_turtle: (a: number, b: number, c: number) => [number, number];
    readonly wasmhealthstore_new: () => [number, number, number];
    readonly wasmhealthstore_query: (a: number, b: number, c: number) => [number, number, number, number];
    readonly weight_turtle_from_csv: (a: number, b: number) => [number, number, number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h8803f8c799f93ab4: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h040d78aefb789e25: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h040d78aefb789e25_2: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h040d78aefb789e25_3: (a: number, b: number, c: any) => [number, number];
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
