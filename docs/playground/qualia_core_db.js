/* @ts-self-types="./qualia_core_db.d.ts" */

/**
 * The Federated Node Manager handles discovery and WebRTC offloading
 */
export class FederatedNodeManager {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        FederatedNodeManagerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_federatednodemanager_free(ptr, 0);
    }
    /**
     * Probes the local network/IPC for an installed 64-bit native daemon
     * @returns {boolean}
     */
    discover_capabilities() {
        const ret = wasm.federatednodemanager_discover_capabilities(this.__wbg_ptr);
        return ret !== 0;
    }
    constructor() {
        const ret = wasm.federatednodemanager_new();
        this.__wbg_ptr = ret;
        FederatedNodeManagerFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Attempts to route a heavy mathematical payload to the native daemon
     * @param {WasmOffloadIntent} intent
     * @returns {string}
     */
    offload_intent(intent) {
        let deferred2_0;
        let deferred2_1;
        try {
            _assertClass(intent, WasmOffloadIntent);
            const ret = wasm.federatednodemanager_offload_intent(this.__wbg_ptr, intent.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
}
if (Symbol.dispose) FederatedNodeManager.prototype[Symbol.dispose] = FederatedNodeManager.prototype.free;

/**
 * Portal tier: 0 = CPU canvas2d fallback, 1 = tensor projection, 2 = WebGPU ambient.
 */
export class QualiaPortal {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        QualiaPortalFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_qualiaportal_free(ptr, 0);
    }
    /**
     * @returns {boolean}
     */
    acoustic_enabled() {
        const ret = wasm.qualiaportal_acoustic_enabled(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * SharedArrayBuffer byte length for zero-copy U3 handoff (requires COOP/COEP).
     * @returns {number}
     */
    acoustic_sab_byte_length() {
        const ret = wasm.qualiaportal_acoustic_sab_byte_length(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Whether a baked STFT/CQT sidecar is pinned on the portal.
     * @returns {boolean}
     */
    acoustic_sidecar_pinned() {
        const ret = wasm.qualiaportal_acoustic_sidecar_pinned(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Serialized `AcousticUniform` bytes for AudioWorklet `SharedArrayBuffer` handoff.
     * @returns {Uint8Array}
     */
    acoustic_uniform_bytes() {
        const ret = wasm.qualiaportal_acoustic_uniform_bytes(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Phenomenal U3 float uniform count (18 scalars + 64 preview bins).
     * @returns {number}
     */
    acoustic_uniform_float_count() {
        const ret = wasm.qualiaportal_acoustic_uniform_float_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Flat `f32` uniform for AudioWorklet message port (18 scalars + 64 preview bins).
     * @returns {Float32Array}
     */
    acoustic_uniform_floats() {
        const ret = wasm.qualiaportal_acoustic_uniform_floats(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Phase 2 — drive the loaded mesh artefact with a kinematic joint (visible physics). `kind` is
     * `"prismatic"` (slide) or anything else = `"revolute"` (spin); `(ax,ay,az)` is the axis
     * (normalised here; defaults to +Y if zero); `rate` is rad/s (revolute) or units/s (prismatic).
     * @param {string} kind
     * @param {number} ax
     * @param {number} ay
     * @param {number} az
     * @param {number} rate
     */
    animate_artefact(kind, ax, ay, az, rate) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.qualiaportal_animate_artefact(this.__wbg_ptr, ptr0, len0, ax, ay, az, rate);
    }
    /**
     * Phase 2 — whether the artefact's proposed motion is currently being refused (clamped).
     * @returns {boolean}
     */
    artefact_refused() {
        const ret = wasm.qualiaportal_artefact_refused(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Cold-bake CQT sidecar (log-spaced bins) for selected tensor node.
     * @param {number} frames
     * @returns {Uint8Array}
     */
    bake_cqt_sidecar_demo(frames) {
        const ret = wasm.qualiaportal_bake_cqt_sidecar_demo(this.__wbg_ptr, frames);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Cold-bake STFT sidecar for selected tensor node; pins bytes for hot frame reads.
     * @param {number} frames
     * @returns {Uint8Array}
     */
    bake_stft_sidecar_demo(frames) {
        const ret = wasm.qualiaportal_bake_stft_sidecar_demo(this.__wbg_ptr, frames);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Phase 5 (affordability rail) — whether a device tier (`0`=Full, `1`=Eco, `2`=Reserve)
     * collapses a qapp's 3D scene to its 2D pane under the budget rule. Pure (no state change);
     * the qapp planner (`render::authoring`) uses the same `OperationalMode::supports_3d` source.
     * @param {number} mode_code
     * @returns {boolean}
     */
    budget_collapses_3d(mode_code) {
        const ret = wasm.qualiaportal_budget_collapses_3d(this.__wbg_ptr, mode_code);
        return ret !== 0;
    }
    /**
     * @returns {number}
     */
    camera_pitch() {
        const ret = wasm.qualiaportal_camera_pitch(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    camera_yaw() {
        const ret = wasm.qualiaportal_camera_yaw(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    camera_zoom() {
        const ret = wasm.qualiaportal_camera_zoom(this.__wbg_ptr);
        return ret;
    }
    /**
     * Wavefunction collapse — set node `q` to 0 in the resident session manifold.
     * @param {number} index
     */
    collapse_node_q(index) {
        const ret = wasm.qualiaportal_collapse_node_q(this.__wbg_ptr, index);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Pending ICP commands in the SPSC ring.
     * @returns {number}
     */
    control_pending() {
        const ret = wasm.qualiaportal_control_pending(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Allocate zeroed acoustic SAB with Q3AS header.
     * @returns {SharedArrayBuffer}
     */
    create_acoustic_sab() {
        const ret = wasm.qualiaportal_create_acoustic_sab(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Phase 2 — visible **deterministic refusal**: slide the artefact along +X (prismatic joint)
     * into a world bound; the admission gate refuses poses that would leave the bound, so the
     * artefact deterministically halts at the wall instead of passing through.
     */
    demo_artefact_refusal() {
        wasm.qualiaportal_demo_artefact_refusal(this.__wbg_ptr);
    }
    /**
     * Drain up to `max` control commands and apply to this portal. Returns count applied.
     * @param {number} max
     * @returns {number}
     */
    drain_control_commands(max) {
        const ret = wasm.qualiaportal_drain_control_commands(this.__wbg_ptr, max);
        return ret >>> 0;
    }
    /**
     * Drain pending sonic tokens into a JS `BigUint64Array` or `Array` of token raw values.
     * @param {number} max
     * @returns {any}
     */
    drain_sonic_tokens(max) {
        const ret = wasm.qualiaportal_drain_sonic_tokens(this.__wbg_ptr, max);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} json
     * @returns {any}
     */
    encode_geometry(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_encode_geometry(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @returns {number}
     */
    epistemic_q() {
        const ret = wasm.qualiaportal_epistemic_q(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {any | undefined}
     */
    last_parsed() {
        const ret = wasm.qualiaportal_last_parsed(this.__wbg_ptr);
        return ret;
    }
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
     * @param {Uint8Array} bytes
     * @returns {any}
     */
    load_10d(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_10d(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * S5.1 colour-by-load — like [`load_10d`] but paints the whole organ mesh a single uniform linear
     * RGBA. The host resolves each organ's body-system percept
     * (`qualia-client-core … AnatomyViewReport::paint_organs`) and passes that system's σ-derived colour
     * (`OrganPercept.percept.rgba`) here, so the 3D body is coloured by accumulated burden. Same
     * governance fail-closed as `load_10d`. (Deliberately parallels `load_10d` rather than sharing a
     * refactored helper: the portal path is wasm+GPU-only and not runtime-testable here, so the proven
     * `load_10d` is left untouched — unify them in the browser-test pass when the anatomy GLBs land.)
     * @param {Uint8Array} bytes
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {any}
     */
    load_10d_colored(bytes, r, g, b, a) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_10d_colored(this.__wbg_ptr, ptr0, len0, r, g, b, a);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
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
     * @param {Uint8Array} bytes
     * @returns {any}
     */
    load_body_from_qualia_bundle(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_body_from_qualia_bundle(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Like [`Self::load_body_from_qualia_bundle`] but honours the **mixer's per-body-system
     * channels**: `system_levels` is a JS object `{ <system_id>: <level 0..1> }`. An organ whose
     * system level is ≤ 0 is omitted (muted); otherwise its colour alpha is scaled by the level.
     * (The mesh pipeline is currently opaque, so a nonzero level acts as show; smooth opacity lands
     * when the mesh pipeline gains alpha blending — mixer plan P2.) An absent/empty map shows every
     * system at full — so `load_body_from_qualia_bundle` is exactly this with no mixer applied.
     * @param {Uint8Array} bytes
     * @param {any} system_levels
     * @param {any} disabled_parts
     * @returns {any}
     */
    load_body_from_qualia_bundle_mixed(bytes, system_levels, disabled_parts) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_body_from_qualia_bundle_mixed(this.__wbg_ptr, ptr0, len0, system_levels, disabled_parts);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
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
     * @param {Array<any>} organs
     * @returns {any}
     */
    load_body_organs_colored(organs) {
        const ret = wasm.qualiaportal_load_body_organs_colored(this.__wbg_ptr, organs);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} json
     * @returns {any}
     */
    load_json_scene(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_json_scene(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {Uint8Array} bytes
     * @returns {any}
     */
    load_q42(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_q42(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} root_id
     */
    mount_qapp(root_id) {
        const ptr0 = passStringToWasm0(root_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_mount_qapp(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Frame the camera on a tensor node (`Maps_to_node`).
     * @param {number} index
     */
    navigate_to_node(index) {
        const ret = wasm.qualiaportal_navigate_to_node(this.__wbg_ptr, index);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {HTMLCanvasElement} canvas
     */
    constructor(canvas) {
        const ret = wasm.qualiaportal_new(canvas);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        QualiaPortalFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Select at pixel; returns index immediately on CPU fallback, else `-1` until next `tick`.
     * @param {number} x
     * @param {number} y
     * @param {number} canvas_w
     * @param {number} canvas_h
     * @returns {number}
     */
    observe_node_at(x, y, canvas_w, canvas_h) {
        const ret = wasm.qualiaportal_observe_node_at(this.__wbg_ptr, x, y, canvas_w, canvas_h);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0];
    }
    /**
     * @returns {number}
     */
    operational_mode() {
        const ret = wasm.qualiaportal_operational_mode(this.__wbg_ptr);
        return ret;
    }
    /**
     * Read a `.hmc` pack's **manifest** without rendering — the list of parts the UI builds its
     * dynamic system + part selectors from. Returns a JS array of `{ key, label, system, systems }`
     * (one per `.10d` entry), so the demo can offer per-system *and* per-part select/deselect driven by
     * what is actually in the loaded pack, not a hardcoded list. Read-only.
     * @param {Uint8Array} bytes
     * @returns {any}
     */
    pack_manifest(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_pack_manifest(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Returns selected tensor index, or `-1` if none / pick still pending.
     * @returns {number}
     */
    poll_selected_node() {
        const ret = wasm.qualiaportal_poll_selected_node(this.__wbg_ptr);
        return ret;
    }
    /**
     * Phase 1.4 — the **2D view** of the resident manifold: each tensor node's `project(.., Plane2D)`
     * shadow as a flat `[x0,y0,x1,y1,...]` array (world units, ~[-1,1]). The 3D scene draws the same
     * nodes through the GPU projector (the `Volume3D` view); both are the *one* manifold projection
     * seen two ways (see `manifold_project`). JS paints this on the 2D companion canvas.
     * @param {number} time
     * @returns {Float32Array}
     */
    project_resident_plane2d(time) {
        const ret = wasm.qualiaportal_project_resident_plane2d(this.__wbg_ptr, time);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Publish phenomenal uniform + pending sonic tokens into SAB.
     * @param {SharedArrayBuffer} sab
     */
    publish_acoustic_sab(sab) {
        const ret = wasm.qualiaportal_publish_acoustic_sab(this.__wbg_ptr, sab);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Push a packed Interface Control Plane command (`PortalControlCommand` raw `u64`).
     * @param {bigint} raw
     * @returns {boolean}
     */
    push_control_command(raw) {
        const ret = wasm.qualiaportal_push_control_command(this.__wbg_ptr, raw);
        return ret !== 0;
    }
    /**
     * @param {bigint} raw
     * @returns {boolean}
     */
    push_sonic_token_raw(raw) {
        const ret = wasm.qualiaportal_push_sonic_token_raw(this.__wbg_ptr, raw);
        return ret !== 0;
    }
    /**
     * @param {HTMLCanvasElement} canvas
     * @param {number} width
     * @param {number} height
     */
    resize(canvas, width, height) {
        const ret = wasm.qualiaportal_resize(this.__wbg_ptr, canvas, width, height);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {any}
     */
    sample_telemetry() {
        const ret = wasm.qualiaportal_sample_telemetry(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Queue GPU picking at canvas pixel `(x, y)`. Result available after the next `tick`.
     * @param {number} x
     * @param {number} y
     * @param {number} canvas_w
     * @param {number} canvas_h
     */
    select_node_at(x, y, canvas_w, canvas_h) {
        const ret = wasm.qualiaportal_select_node_at(this.__wbg_ptr, x, y, canvas_w, canvas_h);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {number}
     */
    selected_node_index() {
        const ret = wasm.qualiaportal_selected_node_index(this.__wbg_ptr);
        return ret;
    }
    /**
     * Enable or mute U3 AcousticPlane (automatically off in Reserve mode).
     * @param {boolean} enabled
     */
    set_acoustic_enabled(enabled) {
        wasm.qualiaportal_set_acoustic_enabled(this.__wbg_ptr, enabled);
    }
    /**
     * Enable/disable the **ambient particle field** — the mixer's "ambient" channel. Off by default
     * (a plain mesh/anatomy view has no use for the decorative random cloud); a Tensor10D upload
     * turns it on automatically because the particles then encode epistemic nodes.
     * @param {boolean} on
     */
    set_ambient_enabled(on) {
        wasm.qualiaportal_set_ambient_enabled(this.__wbg_ptr, on);
    }
    /**
     * Orbit camera IPC from the UI shell (yaw/pitch in radians, zoom = eye distance).
     * @param {number} yaw
     * @param {number} pitch
     * @param {number} zoom
     */
    set_camera(yaw, pitch, zoom) {
        const ret = wasm.qualiaportal_set_camera(this.__wbg_ptr, yaw, pitch, zoom);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {string} mode
     */
    set_display_mode(mode) {
        const ptr0 = passStringToWasm0(mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_set_display_mode(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Human-Centric observer standpoint IPC (independent of camera lens).
     *
     * `standpoint_class`: 0=spectator, 1=ephemeral, 2=identifier (DID), 3=vault.
     * `identifier_did`: empty for spectator/ephemeral; supply DID IRI to bind a verified
     * identifier. Vault standpoints require a sealed local data plane (not exposed here).
     * @param {number} standpoint_class
     * @param {number} epistemic_q
     * @param {number} t_slice
     * @param {number} t_window
     * @param {string} identifier_did
     */
    set_standpoint(standpoint_class, epistemic_q, t_slice, t_window, identifier_did) {
        const ptr0 = passStringToWasm0(identifier_did, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_set_standpoint(this.__wbg_ptr, standpoint_class, epistemic_q, t_slice, t_window, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {Float32Array} floats
     */
    set_telemetry(floats) {
        const ptr0 = passArrayF32ToWasm0(floats, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_set_telemetry(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} t_slice
     * @param {number} t_window
     */
    set_temporal_slice(t_slice, t_window) {
        wasm.qualiaportal_set_temporal_slice(this.__wbg_ptr, t_slice, t_window);
    }
    /**
     * @returns {number}
     */
    sonic_token_pending() {
        const ret = wasm.qualiaportal_sonic_token_pending(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {string} json
     * @returns {any}
     */
    spatial_encode(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_spatial_encode(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @returns {number}
     */
    standpoint_class() {
        const ret = wasm.qualiaportal_standpoint_class(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Phase 2 — freeze the artefact (joint → identity, no world clamp).
     */
    stop_artefact_animation() {
        wasm.qualiaportal_stop_artefact_animation(this.__wbg_ptr);
    }
    /**
     * @returns {number}
     */
    t_slice() {
        const ret = wasm.qualiaportal_t_slice(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    t_window() {
        const ret = wasm.qualiaportal_t_window(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {HTMLCanvasElement} canvas
     * @param {number} dt_ms
     */
    tick(canvas, dt_ms) {
        const ret = wasm.qualiaportal_tick(this.__wbg_ptr, canvas, dt_ms);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {number}
     */
    tier() {
        const ret = wasm.qualiaportal_tier(this.__wbg_ptr);
        return ret;
    }
    /**
     * Import a 3D mesh asset (OBJ / STL / GLB bytes) and render it as a solid surface (Phase 1.2).
     * The mesh is centred on its bounding-box centroid and scaled so its largest extent is ~1.6
     * units — fitting the orbit camera's default frame (eye at distance 3.5, looking at the origin)
     * — then uploaded to the GPU. `hint` is an optional lowercase extension ("obj"/"stl"/"glb");
     * empty = sniff from the bytes. Returns the triangle count (0 if the GPU path isn't active).
     * @param {Uint8Array} bytes
     * @param {string} hint
     * @returns {number}
     */
    upload_mesh_asset(bytes, hint) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(hint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_upload_mesh_asset(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * @param {Uint8Array} bytes
     */
    upload_tensor_buffer(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_upload_tensor_buffer(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) QualiaPortal.prototype[Symbol.dispose] = QualiaPortal.prototype.free;

/**
 * WASM edge offload descriptor — distinct from governance [`crate::llm_agent::AgentIntent`].
 */
export class WasmOffloadIntent {
    static __wrap(ptr) {
        const obj = Object.create(WasmOffloadIntent.prototype);
        obj.__wbg_ptr = ptr;
        WasmOffloadIntentFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmOffloadIntentFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmoffloadintent_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get opcode() {
        const ret = wasm.__wbg_get_wasmoffloadintent_opcode(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get payload_size() {
        const ret = wasm.__wbg_get_wasmoffloadintent_payload_size(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get priority() {
        const ret = wasm.__wbg_get_wasmoffloadintent_priority(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} arg0
     */
    set opcode(arg0) {
        wasm.__wbg_set_wasmoffloadintent_opcode(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set payload_size(arg0) {
        wasm.__wbg_set_wasmoffloadintent_payload_size(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set priority(arg0) {
        wasm.__wbg_set_wasmoffloadintent_priority(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} opcode
     * @param {number} priority
     * @param {number} payload_size
     */
    constructor(opcode, priority, payload_size) {
        const ret = wasm.wasmoffloadintent_new(opcode, priority, payload_size);
        this.__wbg_ptr = ret;
        WasmOffloadIntentFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {number} opcode
     * @param {number} priority
     * @param {string} payload
     * @returns {WasmOffloadIntent}
     */
    static with_string_payload(opcode, priority, payload) {
        const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmoffloadintent_with_string_payload(opcode, priority, ptr0, len0);
        return WasmOffloadIntent.__wrap(ret);
    }
}
if (Symbol.dispose) WasmOffloadIntent.prototype[Symbol.dispose] = WasmOffloadIntent.prototype.free;

/**
 * Legacy alias — prefer `QualiaPortal`.
 */
export class WebEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WebEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_webengine_free(ptr, 0);
    }
    /**
     * @returns {any | undefined}
     */
    last_parsed() {
        const ret = wasm.webengine_last_parsed(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {string} json
     * @returns {any}
     */
    load_json_scene(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webengine_load_json_scene(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {Uint8Array} bytes
     * @returns {any}
     */
    load_q42(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webengine_load_q42(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} root_id
     */
    mount_qapp(root_id) {
        const ptr0 = passStringToWasm0(root_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webengine_mount_qapp(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    constructor() {
        const ret = wasm.webengine_new();
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WebEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    render_to_canvas() {
        const ret = wasm.webengine_render_to_canvas(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) WebEngine.prototype[Symbol.dispose] = WebEngine.prototype.free;

/**
 * @param {any} val
 * @returns {any}
 */
export function align_sequences_wasm(val) {
    const ret = wasm.align_sequences_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Black-Scholes European option pricing with full Greeks.
 * @param {any} val
 * @returns {any}
 */
export function black_scholes_wasm(val) {
    const ret = wasm.black_scholes_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Symbolic derivative. Input `{ expr, var }` (e.g. `{ "expr":"x^3 - 2*x^2 + 5",
 * "var":"x" }`) → `{ derivative }`. The result is simplified, then rendered with the
 * `Expr` `Display` (fully parenthesised). Errors on a parse failure.
 * @param {any} val
 * @returns {any}
 */
export function cas_differentiate_wasm(val) {
    const ret = wasm.cas_differentiate_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Numerically evaluate an expression given variable bindings. Input
 * `{ expr, bindings }` where `bindings` is an object of `name -> number`
 * (e.g. `{ "expr":"x^2 + 3*x + 2", "bindings":{ "x":4 } }`) → `{ value }`.
 * Errors if a referenced variable is unbound, or the result is non-finite
 * (division by zero, √negative, ln of a non-positive value).
 * @param {any} val
 * @returns {any}
 */
export function cas_evaluate_wasm(val) {
    const ret = wasm.cas_evaluate_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Distribute products over sums and expand small (≤ 8) positive integer powers, so the
 * result has no product/power over an additive child. Value-preserving. Input
 * `{ expr }` → `{ expanded }`. Errors on a parse failure.
 * @param {any} val
 * @returns {any}
 */
export function cas_expand_wasm(val) {
    const ret = wasm.cas_expand_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Factor a real quadratic `a·x² + b·x + c` into `a·(x − r₁)·(x — r₂)` (roots snapped to
 * integers/halves when numerically close). Input `{ a, b, c, var }` (`var` defaults to
 * `"x"`) → `{ factored }`. Errors when `a = 0` or the discriminant is negative (no real
 * factorisation).
 * @param {any} val
 * @returns {any}
 */
export function cas_factor_wasm(val) {
    const ret = wasm.cas_factor_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Algebraic simplification (constant folding + identity elimination, to a bounded
 * fixpoint). Input `{ expr }` → `{ simplified }`. Errors on a parse failure.
 * @param {any} val
 * @returns {any}
 */
export function cas_simplify_wasm(val) {
    const ret = wasm.cas_simplify_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Symbolic roots of `a·x² + b·x + c = 0` as `(-b ± √(b²−4ac)) / (2a)` (simplified
 * `Expr` strings), plus their numeric values when the discriminant is non-negative.
 * Input `{ a, b, c }` → `{ roots:[{ expr, value }] }`. For `a = 0, b ≠ 0` returns the
 * single linear root `-c/b`; for `a = 0, b = 0` returns an empty list. A complex /
 * non-finite root value is reported as `null`.
 * @param {any} val
 * @returns {any}
 */
export function cas_solve_quadratic_wasm(val) {
    const ret = wasm.cas_solve_quadratic_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function check_drug_interactions_wasm(val) {
    const ret = wasm.check_drug_interactions_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} input_json
 * @returns {string}
 */
export function clinical_risk(input_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.clinical_risk(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Compile a flat GGUF byte image into a canonical P64 LLM-weight container.
 * Run once at ingest and cache the result in OPFS.
 * @param {Uint8Array} gguf
 * @param {number} page_log2
 * @returns {Uint8Array}
 */
export function compileGgufToP64(gguf, page_log2) {
    const ret = wasm.compileGgufToP64(gguf, page_log2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Historical export retained for browser compatibility. Emits P64 bytes.
 * @param {Uint8Array} gguf
 * @param {number} page_log2
 * @returns {Uint8Array}
 */
export function compileGgufToQ42(gguf, page_log2) {
    const ret = wasm.compileGgufToQ42(gguf, page_log2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Compiles a query string (SPARQL WHERE-clause or N-Triples pattern) to a JSON
 * description of the Webizen VM bytecode program.  Useful for playground inspection
 * and benchmarking the compilation pipeline without supplying a database.
 * @param {string} query
 * @returns {string}
 */
export function compile_query_to_json(query) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.compile_query_to_json(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {any} val
 * @returns {any}
 */
export function compute_framingham_risk_wasm(val) {
    const ret = wasm.compute_framingham_risk_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function compute_molecular_descriptors_wasm(val) {
    const ret = wasm.compute_molecular_descriptors_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Stateless PID controller step.
 * Returns { output, new_error, new_integral } for chaining into the next step.
 * @param {any} val
 * @returns {any}
 */
export function compute_pid_step_wasm(val) {
    const ret = wasm.compute_pid_step_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function compute_reaction_metrics_wasm(val) {
    const ret = wasm.compute_reaction_metrics_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function compute_thermochemistry_wasm(val) {
    const ret = wasm.compute_thermochemistry_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {number} width
 * @param {number} height
 * @returns {HTMLCanvasElement}
 */
export function create_canvas(width, height) {
    const ret = wasm.create_canvas(width, height);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * AEAD decrypt + verify. Input `{ algorithm, key:{text|hex}, nonce:{text|hex},
 * ciphertext:{text|hex}, aad?:{text|hex} }` → `{ algorithm, plaintext_hex,
 * plaintext_utf8?, bytes }`. Fails closed on a bad tag / wrong key, nonce, or aad.
 * @param {any} val
 * @returns {any}
 */
export function crypto_aead_decrypt(val) {
    const ret = wasm.crypto_aead_decrypt(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * AEAD encrypt. Input `{ algorithm, key:{text|hex}, nonce:{text|hex},
 * plaintext:{text|hex}, aad?:{text|hex} }` → `{ algorithm, ciphertext_hex, bytes }`.
 * `algorithm` ∈ aes256gcm | chacha20poly1305 | xchacha20poly1305. Key is 32 bytes;
 * nonce 12 (24 for xchacha). The caller owns the nonce — NEVER reuse a (key, nonce).
 * @param {any} val
 * @returns {any}
 */
export function crypto_aead_encrypt(val) {
    const ret = wasm.crypto_aead_encrypt(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * BLAKE3 digest (256-bit).
 * @param {any} val
 * @returns {any}
 */
export function crypto_blake3(val) {
    const ret = wasm.crypto_blake3(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * HKDF-SHA256 key derivation (RFC 5869). Input
 * `{ ikm:{text|hex}, salt?:{text|hex}, info?:{text|hex}, length }` →
 * `{ algorithm, okm_hex, length }`. `length` is output bytes (1..=8160).
 * @param {any} val
 * @returns {any}
 */
export function crypto_hkdf_sha256(val) {
    const ret = wasm.crypto_hkdf_sha256(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * SHA-256 digest of `{ text } | { hex }` → `{ algorithm, hex, bytes }`.
 * @param {any} val
 * @returns {any}
 */
export function crypto_sha256(val) {
    const ret = wasm.crypto_sha256(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * SHA3-256 (Keccak) digest.
 * @param {any} val
 * @returns {any}
 */
export function crypto_sha3_256(val) {
    const ret = wasm.crypto_sha3_256(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * SHA-512 digest.
 * @param {any} val
 * @returns {any}
 */
export function crypto_sha512(val) {
    const ret = wasm.crypto_sha512(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} json
 * @returns {any}
 */
export function design_encode_wasm(json) {
    const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.design_encode_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function detect_functional_groups_wasm(val) {
    const ret = wasm.detect_functional_groups_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Enforces the rights ontology prior to transmission (e.g., checking DID constraints)
 * @param {bigint} subject_did
 * @returns {boolean}
 */
export function enforce_rights_ontology(subject_did) {
    const ret = wasm.enforce_rights_ontology(subject_did);
    return ret !== 0;
}

/**
 * Query the browser's storage quota and current OPFS usage (bytes).
 *
 * Returns `{ quota: number, usage: number, available: number }`.
 * On mobile PWA the quota is typically 60 % of free disk space (Chrome) or
 * up to 1 GB on iOS Safari. Call this before a large ingest to check headroom.
 * @returns {Promise<any>}
 */
export function estimate_browser_storage() {
    const ret = wasm.estimate_browser_storage();
    return ret;
}

/**
 * @param {any} val
 * @returns {any}
 */
export function evaluate_lipinski_wasm(val) {
    const ret = wasm.evaluate_lipinski_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Exact sum `a + b`. Input `{ a: String, b: String }` -> `{ result }`.
 * @param {any} val
 * @returns {any}
 */
export function exact_bigint_add(val) {
    const ret = wasm.exact_bigint_add(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Truncated division with remainder: `a = quotient*b + remainder`, remainder
 * taking the sign of `a` (toward-zero truncation, matching Rust `/` and `%`).
 * Input `{ a: String, b: String }` -> `{ quotient, remainder }`. Fails closed
 * (`Err`) when `b` is zero.
 * @param {any} val
 * @returns {any}
 */
export function exact_bigint_divmod(val) {
    const ret = wasm.exact_bigint_divmod(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Factorial `n!` as an exact decimal string. Input `{ n: u32 }` ->
 * `{ result }`. Computed from the wasm-clean `BigInt` primitives (the same
 * `mul` loop the solver's `factorial_100_known_value` test uses), so e.g.
 * `n = 100` returns the full 158-digit value with no overflow.
 * @param {any} val
 * @returns {any}
 */
export function exact_bigint_factorial(val) {
    const ret = wasm.exact_bigint_factorial(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Greatest common divisor `gcd(a, b)` (always non-negative; `gcd(0,0) = 0`).
 * Input `{ a: String, b: String }` -> `{ result }`.
 * @param {any} val
 * @returns {any}
 */
export function exact_bigint_gcd(val) {
    const ret = wasm.exact_bigint_gcd(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Exact product `a * b`. Input `{ a: String, b: String }` -> `{ result }`.
 * @param {any} val
 * @returns {any}
 */
export function exact_bigint_mul(val) {
    const ret = wasm.exact_bigint_mul(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Exact integer power `base ^ exp`. Input `{ base: String, exp: u32 }` ->
 * `{ result }`. `base` is an arbitrary-precision decimal string; e.g.
 * `base = "2", exp = 100` returns `1267650600228229401496703205376`.
 * @param {any} val
 * @returns {any}
 */
export function exact_bigint_pow(val) {
    const ret = wasm.exact_bigint_pow(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Exact rational sum `a + b`, returned reduced and sign-normalised as `"p/q"`
 * (q > 0). Inputs are `"p/q"` strings (a bare `"p"` is read as `p/1`). Input
 * `{ a: String, b: String }` -> `{ result }`. E.g. `"1/3" + "1/6" = "1/2"`.
 * @param {any} val
 * @returns {any}
 */
export function exact_rational_add(val) {
    const ret = wasm.exact_rational_add(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Exact rational product `a * b`, returned reduced and sign-normalised as
 * `"p/q"` (q > 0). Inputs are `"p/q"` strings (a bare `"p"` is read as `p/1`).
 * Input `{ a: String, b: String }` -> `{ result }`. E.g. `"3/4" * "1/4" =
 * "3/16"`.
 * @param {any} val
 * @returns {any}
 */
export function exact_rational_mul(val) {
    const ret = wasm.exact_rational_mul(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} query
 * @param {Uint8Array} db_bytes
 * @param {number} max_results
 * @returns {string}
 */
export function execute_ntriples_query(query, db_bytes, max_results) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(db_bytes, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.execute_ntriples_query(ptr0, len0, ptr1, len1, max_results);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {string} json
 * @returns {any}
 */
export function export_tensor_buffer_wasm(json) {
    const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.export_tensor_buffer_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {number} max_nodes
 * @returns {any}
 */
export function export_tensor_slice_wasm(max_nodes) {
    const ret = wasm.export_tensor_slice_wasm(max_nodes);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Forward-chaining defeasible inference engine.
 * Input: `{ facts: ["bird", "penguin"], rules: [{ head: "flies", body: ["bird"], defeaters: ["penguin"] }, ...] }`
 * Output: `{ inferred: ["swims"] }`
 * @param {any} val
 * @returns {any}
 */
export function forward_chain_wasm(val) {
    const ret = wasm.forward_chain_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} input_json
 * @returns {string}
 */
export function geometric_algebra_operation(input_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.geometric_algebra_operation(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

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
 * @param {any} val
 * @returns {any}
 */
export function geometry_convex_hull_2(val) {
    const ret = wasm.geometry_convex_hull_2(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Compute the Delaunay triangulation of a 2-D point set.
 * @param {any} val
 * @returns {any}
 */
export function geometry_delaunay_2(val) {
    const ret = wasm.geometry_delaunay_2(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Execute any geometry tool via the JSON boundary — same function as
 * `execute_geometry_tool_json` on native. This is the full op surface
 * (`orientation_2`, `convex_hull_2`, `triangle_topology`, `mesh_topology`,
 * `delaunay_2`, `voronoi_2`, `nearest_site`).
 * @param {string} args
 * @returns {string}
 */
export function geometry_execute_json(args) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(args, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.geometry_execute_json(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Find the nearest site to a query point (brute-force).
 *
 * Returns the index of the nearest site, or -1 if the point set is empty.
 * @param {Float64Array} points
 * @param {number} qx
 * @param {number} qy
 * @returns {number}
 */
export function geometry_nearest_site(points, qx, qy) {
    const ptr0 = passArrayF64ToWasm0(points, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.geometry_nearest_site(ptr0, len0, qx, qy);
    return ret;
}

/**
 * Robust 2-D orientation predicate.
 *
 * Returns `"clockwise"`, `"collinear"`, or `"counter_clockwise"` —
 * identical to the native `orientation_2` sign.
 * @param {number} ax
 * @param {number} ay
 * @param {number} bx
 * @param {number} by
 * @param {number} cx
 * @param {number} cy
 * @returns {string}
 */
export function geometry_orientation_2(ax, ay, bx, by, cx, cy) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.geometry_orientation_2(ax, ay, bx, by, cx, cy);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Numeric orientation sign (-1, 0, 1) for machine consumption.
 * @param {number} ax
 * @param {number} ay
 * @param {number} bx
 * @param {number} by
 * @param {number} cx
 * @param {number} cy
 * @returns {number}
 */
export function geometry_orientation_2_sign(ax, ay, bx, by, cx, cy) {
    const ret = wasm.geometry_orientation_2_sign(ax, ay, bx, by, cx, cy);
    return ret;
}

/**
 * Compute the Voronoi diagram of a 2-D point set.
 * @param {any} val
 * @returns {any}
 */
export function geometry_voronoi_2(val) {
    const ret = wasm.geometry_voronoi_2(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} json
 * @returns {any}
 */
export function geosparql_operation_wasm(json) {
    const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.geosparql_operation_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Crate semver baked in at compile time.
 * @returns {string}
 */
export function getEngineVersion() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.getEngineVersion();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Diagnostic: vocab size of the resident model tokenizer (0 if engine empty / parse failed).
 * Used by the online demo to show "garbage risk" before generate.
 * @returns {number}
 */
export function getResidentTokenizerVocab() {
    const ret = wasm.getResidentTokenizerVocab();
    return ret >>> 0;
}

/**
 * Structured engine metadata for browser UIs and diagnostics.
 * @returns {any}
 */
export function get_engine_info() {
    const ret = wasm.get_engine_info();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Returns the qualia-core-db crate version baked in at compile time (matches daemon `/health`).
 * @returns {string}
 */
export function get_engine_version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_engine_version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Fuzzy RDF graph similarity (Ma, Li & Ma) — degree-aware Jaccard and Dice over two
 * sets of weighted triples. Terms are interned term ids (non-negative integers);
 * degrees are membership values in `[0,1]`. Two empty graphs are defined as 1.0.
 *
 * Input `{ g1:[[s,p,o,degree],..], g2:[[s,p,o,degree],..] }` ->
 * `{ jaccard, dice }`.
 * @param {any} val
 * @returns {any}
 */
export function graph_fuzzy_similarity(val) {
    const ret = wasm.graph_fuzzy_similarity(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Knowledge-graph link prediction: score a set of candidate tails for a fixed
 * (head, relation) under TransE / DistMult / ComplEx / RotatE and rank them by
 * plausibility (higher = better). Input
 * `{ model, head:[f64], relation:[f64], candidates:[[f64],…], p?, top_k? }` →
 * `{ model, rank, ranking:[{index, score}] }` sorted best-first.
 * @param {any} val
 * @returns {any}
 */
export function graph_kge_predict(val) {
    const ret = wasm.graph_kge_predict(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

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
 * @param {any} val
 * @returns {any}
 */
export function graph_kge_score(val) {
    const ret = wasm.graph_kge_score(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

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
 * @param {any} val
 * @returns {any}
 */
export function graph_shortest_path(val) {
    const ret = wasm.graph_shortest_path(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Spreading activation (Kornai, *Vector Semantics*) — propagate activation from seed
 * concepts through directed weighted edges, decaying each hop and pruning below a
 * threshold. Returns per-node total activation and a top-k relevance ranking.
 *
 * Input `{ edges:[[u,v,w],..], seeds:[[node,activation],..], decay, threshold?,
 * max_hops?, top_k?, n? }` -> `{ activation:[f64;n], ranking:[node,..] }`.
 * @param {any} val
 * @returns {any}
 */
export function graph_spreading_activation(val) {
    const ret = wasm.graph_spreading_activation(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Phase 2B: async WebGPU decode — yields to the browser event loop on every `map_async`.
 * Returns a JS `Promise`; use `await inferWasmAsync(...)` from module code.
 * @param {string} prompt
 * @param {Function} on_token
 * @returns {Promise<string>}
 */
export function inferWasmAsync(prompt, on_token) {
    const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.inferWasmAsync(ptr0, len0, on_token);
    return ret;
}

/**
 * Stream token deltas to `on_token` (UTF-8 string chunks) while decoding.
 * @param {string} prompt
 * @param {Function} on_token
 * @returns {Promise<string>}
 */
export function inferWasmStreaming(prompt, on_token) {
    const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.inferWasmStreaming(ptr0, len0, on_token);
    return ret;
}

/**
 * Streaming inference with optional graph context for provenance hashing.
 * @param {string} prompt
 * @param {string} graph_context
 * @param {Function} on_token
 * @returns {Promise<string>}
 */
export function inferWasmStreamingWithContext(prompt, graph_context, on_token) {
    const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(graph_context, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.inferWasmStreamingWithContext(ptr0, len0, ptr1, len1, on_token);
    return ret;
}

/**
 * Same as `infer_wasm` but accepts optional graph-context bytes for provenance hashing.
 * @param {string} prompt
 * @param {string} graph_context
 * @returns {Promise<string>}
 */
export function inferWasmWithContext(prompt, graph_context) {
    const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(graph_context, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.inferWasmWithContext(ptr0, len0, ptr1, len1);
    return ret;
}

/**
 * Run autoregressive inference (non-streaming). Prompt must include any chat template tokens.
 * @param {string} prompt
 * @returns {Promise<string>}
 */
export function infer_wasm(prompt) {
    const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.infer_wasm(ptr0, len0);
    return ret;
}

export function init_panic_hook() {
    wasm.init_panic_hook();
}

/**
 * Load a GGUF or P64 model into the resident browser WebGPU engine.
 * @param {Uint8Array} model_data
 * @returns {Promise<void>}
 */
export function initialize_webgpu_engine(model_data) {
    const ret = wasm.initialize_webgpu_engine(model_data);
    return ret;
}

/**
 * Intercepts heavy computational opcodes and constructs a WASM offload intent.
 * @param {number} opcode
 * @param {number} payload_size
 * @returns {WasmOffloadIntent | undefined}
 */
export function intercept_computational_opcode(opcode, payload_size) {
    const ret = wasm.intercept_computational_opcode(opcode, payload_size);
    return ret === 0 ? undefined : WasmOffloadIntent.__wrap(ret);
}

/**
 * @param {string} smiles
 * @returns {WasmOffloadIntent}
 */
export function intercept_pharmacogenomics_intent(smiles) {
    const ptr0 = passStringToWasm0(smiles, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.intercept_pharmacogenomics_intent(ptr0, len0);
    return WasmOffloadIntent.__wrap(ret);
}

/**
 * Returns true when a GGUF or P64 model has been loaded via `initialize_webgpu_engine`.
 * @returns {boolean}
 */
export function isWebgpuEngineReady() {
    const ret = wasm.isWebgpuEngineReady();
    return ret !== 0;
}

/**
 * Check whether a SuperBlock is cached in the OPFS vault.
 * Returns `true` if the `.qblk` file exists, `false` otherwise.
 * @param {number} block_index
 * @returns {Promise<boolean>}
 */
export function is_opfs_block_cached(block_index) {
    const ret = wasm.is_opfs_block_cached(block_index);
    return ret;
}

/**
 * Determinant of a square matrix via LU (partial pivoting).
 * Input `{ rows, cols, data }` (rows==cols) → `{ determinant }`.
 * @param {any} val
 * @returns {any}
 */
export function la_determinant_wasm(val) {
    const ret = wasm.la_determinant_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Symmetric eigendecomposition (cyclic Jacobi). Input `{ rows, cols, data }`
 * (square, symmetric) → `{ eigenvalues:[..], eigenvectors:{rows,cols,data} }`
 * where eigenvector `j` is column `j` of the row-major `eigenvectors` matrix.
 * @param {any} val
 * @returns {any}
 */
export function la_eigen_symmetric_wasm(val) {
    const ret = wasm.la_eigen_symmetric_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * General (non-symmetric) eigenvalues via the characteristic polynomial.
 * Input `{ rows, cols, data }` (square) → `{ eigenvalues:[{re,im}] }`.
 * @param {any} val
 * @returns {any}
 */
export function la_eigenvalues_wasm(val) {
    const ret = wasm.la_eigenvalues_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * `C = A · B`. Input `{ a:{rows,cols,data}, b:{rows,cols,data} }`,
 * output `{ rows, cols, data }`. Errors on a shape mismatch (`a.cols != b.rows`).
 * @param {any} val
 * @returns {any}
 */
export function la_matmul_wasm(val) {
    const ret = wasm.la_matmul_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * All complex roots of a real polynomial (Durand–Kerner). Input
 * `{ coeffs:[cₙ,…,c₁,c₀] }` (descending) → `{ degree, roots:[{re,im}] }`.
 * @param {any} val
 * @returns {any}
 */
export function la_polynomial_roots_wasm(val) {
    const ret = wasm.la_polynomial_roots_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Solve `A · x = b` for a square `A` via LU. Input `{ a:{rows,cols,data}, b:[..] }`
 * (b length == a.rows) → `{ x:[..] }`. Errors if `A` is singular.
 * @param {any} val
 * @returns {any}
 */
export function la_solve_wasm(val) {
    const ret = wasm.la_solve_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Thin SVD `A = U·Σ·Vᵀ`. Input `{ rows, cols, data }` →
 * `{ singular_values:[..], u:{rows,cols,data}, v:{rows,cols,data} }`
 * (`u` is m×n, `v` is n×n; singular vectors are columns; values descending).
 * @param {any} val
 * @returns {any}
 */
export function la_svd_wasm(val) {
    const ret = wasm.la_svd_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Transpose. Input `{ rows, cols, data }` → output `{ rows:cols, cols:rows, data }`.
 * @param {any} val
 * @returns {any}
 */
export function la_transpose_wasm(val) {
    const ret = wasm.la_transpose_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Capability names available in this WASM build.
 * @returns {any}
 */
export function list_capabilities_wasm() {
    const ret = wasm.list_capabilities_wasm();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Airy functions `Ai(x)` and `Bi(x)` (both, from one Maclaurin-series evaluation).
 * Input `{ x }` -> `{ ai, bi }`.
 * @param {any} val
 * @returns {any}
 */
export function num_airy_wasm(val) {
    const ret = wasm.num_airy_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Euler's totient `phi(n)`, the Mobius `mu(n)`, divisor count `d(n)` and divisor sum
 * `sigma(n)` — the classic multiplicative arithmetic functions, all from the prime
 * factorization. Input `{ n }` -> `{ totient, mobius, divisor_count, divisor_sum }`.
 * @param {any} val
 * @returns {any}
 */
export function num_arithmetic_functions_wasm(val) {
    const ret = wasm.num_arithmetic_functions_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Modified Bessel function of the first kind `I_n(x)`, integer order. Defined for all
 * real `x`. Input `{ n, x }` -> `{ value }`.
 * @param {any} val
 * @returns {any}
 */
export function num_bessel_i_wasm(val) {
    const ret = wasm.num_bessel_i_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Bessel function of the first kind `J_n(x)`, integer order (any sign), defined for all
 * real `x`. Input `{ n, x }` -> `{ value }`.
 * @param {any} val
 * @returns {any}
 */
export function num_bessel_j_wasm(val) {
    const ret = wasm.num_bessel_j_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Modified Bessel function of the second kind `K_n(x)`, integer order `n >= 0`. Requires
 * `x > 0`. Input `{ n, x }` -> `{ value }`; errors for `x <= 0`.
 * @param {any} val
 * @returns {any}
 */
export function num_bessel_k_wasm(val) {
    const ret = wasm.num_bessel_k_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Bessel function of the second kind `Y_n(x)`, integer order `n >= 0`. Requires `x > 0`
 * (singular at the origin) and `J_0(x) != 0`. Input `{ n, x }` -> `{ value }`; errors
 * for `x <= 0` or an ill-posed Wronskian solve.
 * @param {any} val
 * @returns {any}
 */
export function num_bessel_y_wasm(val) {
    const ret = wasm.num_bessel_y_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Binomial coefficient `C(n, k)` (exact integer at every step). Result is returned as a
 * decimal **string** since it may exceed `f64`/`u53` precision. Errors (fail closed) on
 * `u128` overflow. Input `{ n, k }` -> `{ value }` (value is a string).
 * @param {any} val
 * @returns {any}
 */
export function num_binomial_wasm(val) {
    const ret = wasm.num_binomial_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The `n`-th Catalan number, plus the Stirling numbers `S(n,k)` (second kind) and
 * `c(n,k)` (unsigned first kind). All exact integers as decimal **strings**; errors
 * (fail closed) on `u128` overflow. Input `{ n, k }` ->
 * `{ catalan, stirling_second, stirling_first }`.
 * @param {any} val
 * @returns {any}
 */
export function num_combinatorics_wasm(val) {
    const ret = wasm.num_combinatorics_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Natural cubic spline through `(xs, ys)` (xs strictly increasing), evaluated at each
 * query in `queries`. Errors on insufficient data, unsorted/duplicate nodes, or a
 * singular tridiagonal system. Input `{ xs:[..], ys:[..], queries:[..] }` ->
 * `{ values:[..] }`.
 * @param {any} val
 * @returns {any}
 */
export function num_cubic_spline_wasm(val) {
    const ret = wasm.num_cubic_spline_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * All positive divisors of `n`, ascending. Input `{ n }` -> `{ divisors:[..] }`.
 * @param {any} val
 * @returns {any}
 */
export function num_divisors_wasm(val) {
    const ret = wasm.num_divisors_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Factorial `n!` as an exact integer (decimal **string**; `f64` cannot hold it).
 * Errors (fail closed) for `n >= 35` (`35!` overflows `u128`).
 * Input `{ n }` -> `{ value }` (value is a string).
 * @param {any} val
 * @returns {any}
 */
export function num_factorial_wasm(val) {
    const ret = wasm.num_factorial_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Greatest common divisor and least common multiple of `a` and `b`.
 * Input `{ a, b }` -> `{ gcd, lcm }`.
 * @param {any} val
 * @returns {any}
 */
export function num_gcd_lcm_wasm(val) {
    const ret = wasm.num_gcd_lcm_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Deterministic Miller-Rabin primality test (exact for all `u64`).
 * Input `{ n }` -> `{ prime }`.
 * @param {any} val
 * @returns {any}
 */
export function num_is_prime_wasm(val) {
    const ret = wasm.num_is_prime_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Evaluate the Lagrange interpolating polynomial through `(xs, ys)` at `x`. Errors on
 * empty/mismatched data or duplicate nodes. Input `{ xs:[..], ys:[..], x }` -> `{ value }`.
 * @param {any} val
 * @returns {any}
 */
export function num_lagrange_eval_wasm(val) {
    const ret = wasm.num_lagrange_eval_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Piecewise-linear interpolation of `(xs, ys)` (xs strictly increasing) at `x` (clamped
 * to the endpoints outside the range). Input `{ xs:[..], ys:[..], x }` -> `{ value }`.
 * @param {any} val
 * @returns {any}
 */
export function num_linear_interp_wasm(val) {
    const ret = wasm.num_linear_interp_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

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
 * @param {any} val
 * @returns {any}
 */
export function num_minimize_wasm(val) {
    const ret = wasm.num_minimize_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Modular multiplicative inverse: the `x` with `a*x ≡ 1 (mod m)`. Errors (fail closed)
 * when `gcd(a, m) != 1`. Input `{ a, m }` -> `{ inverse }`.
 * @param {any} val
 * @returns {any}
 */
export function num_mod_inverse_wasm(val) {
    const ret = wasm.num_mod_inverse_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * `(base^exp) mod modulus` by repeated squaring (overflow-safe via `u128`).
 * Input `{ base, exp, modulus }` -> `{ value }`.
 * @param {any} val
 * @returns {any}
 */
export function num_mod_pow_wasm(val) {
    const ret = wasm.num_mod_pow_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Newton divided-difference interpolation: build the coefficients from `(xs, ys)` and
 * evaluate the interpolant at `x`. Input `{ xs:[..], ys:[..], x }` ->
 * `{ value, coefficients:[..] }`.
 * @param {any} val
 * @returns {any}
 */
export function num_newton_eval_wasm(val) {
    const ret = wasm.num_newton_eval_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Smallest prime strictly greater than `n`. Input `{ n }` -> `{ next_prime }`.
 * @param {any} val
 * @returns {any}
 */
export function num_next_prime_wasm(val) {
    const ret = wasm.num_next_prime_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Classical orthogonal polynomial `P_n(x)` by three-term recurrence. `kind` is one of
 * `"legendre" | "chebyshev_t" | "chebyshev_u" | "hermite" | "laguerre"`.
 * Input `{ kind, n, x }` -> `{ value }`; errors on an unknown kind.
 * @param {any} val
 * @returns {any}
 */
export function num_orthopoly_wasm(val) {
    const ret = wasm.num_orthopoly_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Number of integer partitions `p(n)` (ways to write `n` as an unordered sum of positive
 * integers). Input `{ n }` -> `{ value }`.
 * @param {any} val
 * @returns {any}
 */
export function num_partitions_wasm(val) {
    const ret = wasm.num_partitions_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Least-squares polynomial fit of degree `degree` to `(xs, ys)` (via the normal
 * equations). Returns coefficients in **ascending** order `[c0, c1, ..., c_degree]` (so
 * the polynomial is `sum c_k x^k`). Optionally evaluates the fit at each `queries` value.
 * Errors on too few points, `degree + 1 > n`, or a singular system.
 * Input `{ xs:[..], ys:[..], degree, queries?:[..] }` ->
 * `{ coefficients:[..], values:[..] }`.
 * @param {any} val
 * @returns {any}
 */
export function num_poly_fit_wasm(val) {
    const ret = wasm.num_poly_fit_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Prime factorization (trial division then Pollard's rho), correct across all `u64`.
 * Input `{ n }` -> `{ factors:[{ prime, exponent }] }`. Empty for `n < 2`.
 * @param {any} val
 * @returns {any}
 */
export function num_prime_factorize_wasm(val) {
    const ret = wasm.num_prime_factorize_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Riemann zeta function `zeta(s)` for real `s > 1` (Euler-Maclaurin). Input `{ s }` ->
 * `{ value }`; errors for `s <= 1` (needs analytic continuation, out of this domain).
 * @param {any} val
 * @returns {any}
 */
export function num_zeta_wasm(val) {
    const ret = wasm.num_zeta_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} input_json
 * @returns {string}
 */
export function ode_solver(input_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.ode_solver(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {string} input_json
 * @returns {string}
 */
export function organic_chemistry(input_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.organic_chemistry(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Current P64 container version for OPFS cache invalidation.
 * @returns {number}
 */
export function p64FormatVersion() {
    const ret = wasm.p64FormatVersion();
    return ret;
}

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
 * @param {bigint} seq_id
 * @param {bigint} owner_did
 * @param {Uint8Array} raw_quin_bytes
 * @returns {Uint8Array}
 */
export function pack_quins_into_superblock(seq_id, owner_did, raw_quin_bytes) {
    const ptr0 = passArray8ToWasm0(raw_quin_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.pack_quins_into_superblock(seq_id, owner_did, ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} payload
 * @returns {any}
 */
export function parse_cbor_ld_wasm(payload) {
    const ptr0 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_cbor_ld_wasm(ptr0, len0);
    return ret;
}

/**
 * @param {any} val
 * @returns {any}
 */
export function parse_csv_wasm(val) {
    const ret = wasm.parse_csv_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function parse_json_mapping_wasm(val) {
    const ret = wasm.parse_json_mapping_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} payload
 * @returns {any}
 */
export function parse_json_wasm(payload) {
    const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_json_wasm(ptr0, len0);
    return ret;
}

/**
 * @param {string} payload
 * @returns {any}
 */
export function parse_n3logic_wasm(payload) {
    const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_n3logic_wasm(ptr0, len0);
    return ret;
}

/**
 * @param {string} payload
 * @returns {any}
 */
export function parse_turtle_wasm(payload) {
    const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_turtle_wasm(ptr0, len0);
    return ret;
}

/**
 * Create the WebGPU device + surface asynchronously and stash it for the render loop to adopt.
 * JS calls this **once, awaited**, right after constructing the portal and **before** the render
 * loop starts — the canvas must still be context-free (no 2d context yet) so the WebGPU surface
 * can bind to it. Returns `true` if the GPU path is now armed; on `false`/throw the portal keeps
 * the canvas2d fallback.
 * @param {HTMLCanvasElement} canvas
 * @returns {Promise<boolean>}
 */
export function portal_init_webgpu(canvas) {
    const ret = wasm.portal_init_webgpu(canvas);
    return ret;
}

/**
 * @returns {number}
 */
export function predict_receptor_binding_wasm() {
    const ret = wasm.predict_receptor_binding_wasm();
    return ret;
}

/**
 * Performs topological pruning and validates meshes prior to physics offloading
 * @param {bigint} mesh_id
 * @returns {boolean}
 */
export function prune_and_validate_mesh(mesh_id) {
    const ret = wasm.prune_and_validate_mesh(mesh_id);
    return ret !== 0;
}

/**
 * Historical export retained for browser compatibility. Returns P64_VERSION.
 * @returns {number}
 */
export function q42FormatVersion() {
    const ret = wasm.q42FormatVersion();
    return ret;
}

/**
 * Read a cached SuperBlock from the OPFS vault.
 *
 * Returns the raw 40 960 bytes as `Uint8Array`, or `null` if the block has not
 * been written yet (cache miss). Callers should fall back to an HTTP Range
 * request (see the JS `VFS` class) on cache miss.
 * @param {number} block_index
 * @returns {Promise<any>}
 */
export function read_opfs_block(block_index) {
    const ret = wasm.read_opfs_block(block_index);
    return ret;
}

/**
 * Release resident model weights and tear down the WebGPU engine instance.
 * @returns {Promise<void>}
 */
export function releaseWebgpuEngine() {
    const ret = wasm.releaseWebgpuEngine();
    return ret;
}

/**
 * Resolves two conflicting NQuin entries using Last-Writer-Wins semantics.
 * The Lamport clock is encoded in the metadata field; on ties, higher object wins.
 * @param {any} local_val
 * @param {any} remote_val
 * @returns {any}
 */
export function resolve_lww_wasm(local_val, remote_val) {
    const ret = wasm.resolve_lww_wasm(local_val, remote_val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function run_semantic_simulation(val) {
    const ret = wasm.run_semantic_simulation(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @returns {any}
 */
export function sample_browser_telemetry_wasm() {
    const ret = wasm.sample_browser_telemetry_wasm();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} input_json
 * @returns {string}
 */
export function sequence_alignment(input_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.sequence_alignment(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {any} val
 * @returns {any}
 */
export function serialize_csv_wasm(val) {
    const ret = wasm.serialize_csv_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Continuous Mathematical Serialization into Float64Array
 * @param {Float64Array} data
 * @returns {Float64Array}
 */
export function serialize_float64_array(data) {
    const ptr0 = passArrayF64ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.serialize_float64_array(ptr0, len0);
    return ret;
}

/**
 * Packs an array of floats into a Uint8Array strictly typed buffer to avoid IEEE-754 truncation
 * @param {Float32Array} data
 * @returns {Uint8Array}
 */
export function serialize_float_array(data) {
    const ptr0 = passArrayF32ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.serialize_float_array(ptr0, len0);
    return ret;
}

/**
 * @param {any} val
 * @returns {any}
 */
export function serialize_json_wasm(val) {
    const ret = wasm.serialize_json_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function serialize_rdf_wasm(val) {
    const ret = wasm.serialize_rdf_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Simulates a GBM price path and returns the full series together with
 * min_price, max_price, and final_price.
 * @param {any} val
 * @returns {any}
 */
export function simulate_gbm_path_wasm(val) {
    const ret = wasm.simulate_gbm_path_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Solves dy/dt = -k·y via classical RK4, returning t_values, y_values, and final_y.
 * @param {any} val
 * @returns {any}
 */
export function solve_ode_exponential_decay_wasm(val) {
    const ret = wasm.solve_ode_exponential_decay_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Bounded DPLL SAT solver.
 * Input: `{ clauses: [[1, 2, -3], [-1, 3], ...] }` (signed literal convention).
 * Output: `{ satisfiable: bool, assignment: { "1": true, "2": false, ... } }`
 * @param {any} val
 * @returns {any}
 */
export function solve_sat_wasm(val) {
    const ret = wasm.solve_sat_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} json
 * @returns {any}
 */
export function spatial_encode_wasm(json) {
    const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.spatial_encode_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * One-way ANOVA F-test for equality of `k` group means. Input
 * `{ groups:[[..],[..],..] }` (≥ 2 groups, each non-empty, total > k) →
 * `{ f_statistic, p_value, df_between, df_within, ss_between, ss_within,
 * ms_between, ms_within }`. Errors on degenerate input.
 * @param {any} val
 * @returns {any}
 */
export function stats_anova_wasm(val) {
    const ret = wasm.stats_anova_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Pearson χ² goodness-of-fit test, `Σ(Oᵢ−Eᵢ)²/Eᵢ`, dof = k−1. Input
 * `{ observed:[..], expected:[..] }` (equal length ≥ 2, all expected > 0) →
 * `{ statistic, p_value, dof }`. Errors on length mismatch, len < 2, or a
 * non-positive expected count.
 * @param {any} val
 * @returns {any}
 */
export function stats_chi_square_gof_wasm(val) {
    const ret = wasm.stats_chi_square_gof_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * χ² test of independence on an R×C contingency table of counts. Input
 * `{ table:[[..],[..],..] }` (≥ 2 rows, ≥ 2 cols, rectangular, grand total > 0) →
 * `{ statistic, p_value, dof }` with `dof = (R−1)(C−1)`. Errors on a ragged or
 * undersized table.
 * @param {any} val
 * @returns {any}
 */
export function stats_chi_square_independence_wasm(val) {
    const ret = wasm.stats_chi_square_independence_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * χ² (chi-squared) distribution pdf/cdf at `x` with `k` degrees of freedom, plus
 * the upper-tail p-value. Input `{ x:f64, k:f64, p?:f64 }` (`k` > 0, `x` ≥ 0) →
 * `{ pdf, cdf, upper_p, quantile }`. `quantile` is the inverse-cdf at `p` when
 * supplied (0<p<1), else `null`.
 * @param {any} val
 * @returns {any}
 */
export function stats_chi_squared_dist_wasm(val) {
    const ret = wasm.stats_chi_squared_dist_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Pearson, Spearman, and Kendall correlation of two equal-length series, plus the
 * two-sided p-value for the Pearson coefficient. Input `{ x:[..], y:[..] }` →
 * `{ pearson, spearman, kendall, pearson_p_value }`. Each coefficient is `null`
 * when undefined (lengths differ, or n < 2); `pearson_p_value` is `null` for n < 3.
 * @param {any} val
 * @returns {any}
 */
export function stats_correlation_wasm(val) {
    const ret = wasm.stats_correlation_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Full descriptive summary of a sample. Input `{ data:[..], sample?:bool }`
 * (`sample` defaults to `true` → Bessel-corrected variance/std) →
 * `{ n, sum, mean, variance, std_dev, min, max, median, q1, q3, skewness, kurtosis }`.
 * `variance`/`std_dev` are `null` when n < 2 in sample mode (no residual dof);
 * `skewness`/`kurtosis` are excess-kurtosis (Fisher) conventions.
 * @param {any} val
 * @returns {any}
 */
export function stats_describe_wasm(val) {
    const ret = wasm.stats_describe_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Fisher–Snedecor F-distribution: pdf and cdf at x with (d1, d2) degrees of
 * freedom, plus the inverse-cdf quantile when an optional `p` is supplied.
 * Input `{ x, d1, d2, p? }` → `{ pdf, cdf, quantile? }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_fisher_f_wasm(val) {
    const ret = wasm.stats_fisher_f_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Friedman test for k treatments across n blocks (e.g. classifiers × datasets).
 * Input `{ blocks:[[m1,…,mk], …] }` (each block length k, higher = better) →
 * `{ chi_square, chi_p_value, df, iman_davenport_f, f_p_value }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_friedman_wasm(val) {
    const ret = wasm.stats_friedman_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Simple (one-predictor) OLS linear regression of `y` on `x`. Input
 * `{ x:[..], y:[..] }` (equal length, n ≥ 3, x not constant) →
 * `{ slope, intercept, r_squared, residual_std_error, slope_std_error, slope_t,
 * slope_p_value, intercept_std_error, intercept_p_value, n }`. Errors on length
 * mismatch, n < 3, or zero-variance `x`.
 * @param {any} val
 * @returns {any}
 */
export function stats_linear_regression_wasm(val) {
    const ret = wasm.stats_linear_regression_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * McNemar's test for two paired binary classifiers. Input `{ b, c }` — the
 * discordant counts (b = first right / second wrong, c = first wrong / second
 * right) — → `{ statistic, p_value, dof }`. Continuity-corrected χ², dof 1.
 * @param {any} val
 * @returns {any}
 */
export function stats_mcnemar_wasm(val) {
    const ret = wasm.stats_mcnemar_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Normal (Gaussian) distribution pdf/cdf/quantile at one point. Input
 * `{ x:f64, mu?:f64, sigma?:f64, p?:f64 }` (`mu` defaults 0, `sigma` defaults 1,
 * must be > 0) → `{ pdf, cdf, quantile }`. `pdf`/`cdf` are evaluated at `x`;
 * `quantile` is `Φ⁻¹(p)` when `p` is supplied (0<p<1), else `null`.
 * @param {any} val
 * @returns {any}
 */
export function stats_normal_wasm(val) {
    const ret = wasm.stats_normal_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * One-sample t-test of the sample mean against `mu`. Input `{ data:[..], mu:f64 }`
 * → `{ t_statistic, p_value, degrees_of_freedom, ci_lower, ci_upper }`
 * (95% CI around the sample mean, t critical value). Errors if n < 2.
 * @param {any} val
 * @returns {any}
 */
export function stats_one_sample_t_wasm(val) {
    const ret = wasm.stats_one_sample_t_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Paired t-test (one-sample t-test of the paired differences against 0). Input
 * `{ a:[..], b:[..] }` (equal length) → `{ t_statistic, p_value,
 * degrees_of_freedom, ci_lower, ci_upper }`. Errors if lengths differ or n < 2.
 * @param {any} val
 * @returns {any}
 */
export function stats_paired_t_wasm(val) {
    const ret = wasm.stats_paired_t_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Linear-interpolated quantile (numpy "linear" / R type-7). Input
 * `{ data:[..], q:0.0..1.0 }` → `{ quantile }`. `q` is clamped to `[0,1]`.
 * @param {any} val
 * @returns {any}
 */
export function stats_quantile_wasm(val) {
    const ret = wasm.stats_quantile_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Student's t-distribution pdf/cdf at `t` with `nu` degrees of freedom, plus the
 * two-sided p-value. Input `{ t:f64, nu:f64, p?:f64 }` (`nu` > 0) →
 * `{ pdf, cdf, two_sided_p, quantile }`. `quantile` is the inverse-cdf at `p`
 * when supplied (0<p<1), else `null`.
 * @param {any} val
 * @returns {any}
 */
export function stats_students_t_wasm(val) {
    const ret = wasm.stats_students_t_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Two-sample t-test of `mean(a) − mean(b) = 0`. Input
 * `{ a:[..], b:[..], equal_var?:bool }` (`equal_var` defaults to `false` → the
 * Welch test; `true` → pooled Student) → `{ t_statistic, p_value,
 * degrees_of_freedom, mean_difference, ci_lower, ci_upper }`. Errors if either
 * sample has n < 2.
 * @param {any} val
 * @returns {any}
 */
export function stats_two_sample_t_wasm(val) {
    const ret = wasm.stats_two_sample_t_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {string} input_json
 * @returns {string}
 */
export function thermodynamics_mcmc(input_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.thermodynamics_mcmc(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Look up a CODATA / SI-2019 physical constant by name, returning its value (in coherent
 * SI base units) and its physical dimension as the 7-vector.
 *
 * Input `{ name }` → `{ name, symbol, description, value, dimension:{..} }`.
 * Accepted names are those from `units_list_constants` (canonical name or symbol alias).
 * @param {any} val
 * @returns {any}
 */
export function units_constant(val) {
    const ret = wasm.units_constant(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Convert a magnitude between two named units of the **same** physical dimension.
 * Affine (Celsius/Fahrenheit) and linear scales are both handled. Fails closed if the
 * units have different dimensions (e.g. `m` → `s`).
 *
 * Input `{ value, from, to }` → `{ value, from, to, dimension:{..} }`.
 * @param {any} val
 * @returns {any}
 */
export function units_convert(val) {
    const ret = wasm.units_convert(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * List every CODATA constant available to `units_constant`, with value, symbol,
 * description and dimension. Takes an empty object `{}`.
 * Input `{}` → `{ constants:[{name,symbol,description,value,dimension}] }`.
 * @param {any} _val
 * @returns {any}
 */
export function units_list_constants(_val) {
    const ret = wasm.units_list_constants(_val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * List every unit the engine can convert between, with a human label and its dimension
 * 7-vector. Takes an empty object `{}`. Input `{}` → `{ units:[{symbol,label,dimension}] }`.
 * @param {any} _val
 * @returns {any}
 */
export function units_list_units(_val) {
    const ret = wasm.units_list_units(_val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Multiply or divide two dimensioned quantities, composing their dimensions. Each
 * quantity is `{ value, unit }`; the unit string is resolved to its SI factor so the
 * result value is in coherent SI base units, and the result dimension is returned as the
 * 7-vector. `divide` fails closed on a zero divisor.
 *
 * Input `{ a:{value,unit}, b:{value,unit}, op:"multiply"|"divide" }`
 * → `{ value, dimension:{..} }`.
 * @param {any} val
 * @returns {any}
 */
export function units_quantity_op(val) {
    const ret = wasm.units_quantity_op(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function validate_fasta_wasm(val) {
    const ret = wasm.validate_fasta_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function validate_fhir_observation_wasm(val) {
    const ret = wasm.validate_fhir_observation_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {any} val
 * @returns {any}
 */
export function validate_shacl_constraint_wasm(val) {
    const ret = wasm.validate_shacl_constraint_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Validate ECC parity for every NQuin in a raw SuperBlock.
 *
 * Returns JSON: `{"valid":bool,"total":N,"bad":[indices...]}`
 * A non-empty `bad` array indicates sector corruption.
 * @param {Uint8Array} block_bytes
 * @returns {string}
 */
export function verify_superblock_ecc(block_bytes) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(block_bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.verify_superblock_ecc(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {Float64Array} points_flat
 * @returns {Uint32Array}
 */
export function wasm_convex_hull_2d(points_flat) {
    const ptr0 = passArrayF64ToWasm0(points_flat, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.wasm_convex_hull_2d(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Float64Array} points_flat
 * @returns {Uint32Array}
 */
export function wasm_delaunay_triangulation_2d(points_flat) {
    const ptr0 = passArrayF64ToWasm0(points_flat, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.wasm_delaunay_triangulation_2d(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Polls the local Webizen for pending agreements waiting for the user's signature.
 * @returns {string}
 */
export function webizen_poll_agreements() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.webizen_poll_agreements();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Proposes a new M:N Guardianship agreement to the local WebRTC mesh.
 * @param {Array<any>} _nominated_guardians
 * @param {string} principal
 * @param {string} domain
 * @param {number} threshold
 * @returns {bigint}
 */
export function webizen_propose_agreement(_nominated_guardians, principal, domain, threshold) {
    const ptr0 = passStringToWasm0(principal, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(domain, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.webizen_propose_agreement(_nominated_guardians, ptr0, len0, ptr1, len1, threshold);
    return BigInt.asUintN(64, ret);
}

/**
 * Signs a pending agreement, advancing its state machine and triggering WebRTC peer sync.
 * @param {bigint} _agreement_id
 * @param {string} _private_key_mock
 */
export function webizen_sign_agreement(_agreement_id, _private_key_mock) {
    const ptr0 = passStringToWasm0(_private_key_mock, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    wasm.webizen_sign_agreement(_agreement_id, ptr0, len0);
}

/**
 * Write a SuperBlock to the OPFS vault at `block_index`.
 *
 * `block_bytes` must be exactly `BLOCK_MULTIPLIER_SIZE` (40 960) bytes — use
 * `pack_quins_into_superblock()` to produce correctly-structured blocks.
 *
 * File name: `block_XXXXXXXX.qblk` (zero-padded 8-digit decimal index).
 * Compatible with the naming convention used by the JS VFS class.
 * @param {number} block_index
 * @param {Uint8Array} block_bytes
 * @returns {Promise<void>}
 */
export function write_opfs_block(block_index, block_bytes) {
    const ptr0 = passArray8ToWasm0(block_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.write_opfs_block(block_index, ptr0, len0);
    return ret;
}

/**
 * Forward discrete Fourier transform `X[k] = Σ_n x[n] e^{-2πi kn/N}`
 * (un-normalized, forward sign convention). f64-exact CPU reference path.
 *
 * Input `{ data:[..] }` (real signal) OR `{ re:[..], im:[..] }` (complex signal).
 * Output `{ re:[..], im:[..], magnitude:[..], n }`.
 * @param {any} val
 * @returns {any}
 */
export function xform_dft(val) {
    const ret = wasm.xform_dft(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Inverse discrete Fourier transform `x[n] = (1/N) Σ_k X[k] e^{+2πi kn/N}`.
 * Round-trips `xform_dft` to ~1e-9.
 *
 * Input the spectrum as `{ re:[..], im:[..] }` (complex bins) OR `{ data:[..] }`
 * (real bins → imaginary parts taken as 0).
 * Output `{ re:[..], im:[..], magnitude:[..], n }` — the recovered samples.
 * @param {any} val
 * @returns {any}
 */
export function xform_idft(val) {
    const ret = wasm.xform_idft(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

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
 * @param {any} val
 * @returns {any}
 */
export function xform_laplace_numeric(val) {
    const ret = wasm.xform_laplace_numeric(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Symbolic Laplace transform of a polynomial in `t` from the table the CAS can
 * represent: a sum of `coeff · t^power` terms (constants are `power = 0`).
 * Returns the resulting `Expr` in `s` as a pretty string and, when `s` is
 * supplied, its numeric value `L{f}(s)`. Fails closed (`NotTransformable`) on
 * anything outside constants / integer powers / their linear combinations.
 *
 * Input `{ terms:[{coeff, power}, ..], s? }`. Output `{ expr, value? }`.
 * @param {any} val
 * @returns {any}
 */
export function xform_laplace_table(val) {
    const ret = wasm.xform_laplace_table(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Closed form of the geometric `aⁿ u[n]` Z-transform `X(z) = 1/(1 - a z^{-1})`
 * (valid for `|z| > |a|`). Fails closed where the denominator vanishes / at `z = 0`.
 *
 * Input `{ a, z_re, z_im }`. Output `{ re, im, magnitude }`.
 * @param {any} val
 * @returns {any}
 */
export function xform_z_geometric(val) {
    const ret = wasm.xform_z_geometric(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Z-transform of a finite causal sequence evaluated at a complex point `z`:
 * `X(z) = Σ_{n=0}^{N-1} x[n] z^{-n}`. Fails closed at `z = 0`.
 *
 * Input `{ x:[..], z_re, z_im }`. Output `{ re, im, magnitude }`.
 * @param {any} val
 * @returns {any}
 */
export function xform_z_transform(val) {
    const ret = wasm.xform_z_transform(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Closed form of the unit-step `u[n]` Z-transform `X(z) = z/(z-1)`
 * (valid for `|z| > 1`). Fails closed at `z = 0` or `z = 1`.
 *
 * Input `{ z_re, z_im }`. Output `{ re, im, magnitude }`.
 * @param {any} val
 * @returns {any}
 */
export function xform_z_unit_step(val) {
    const ret = wasm.xform_z_unit_step(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_fdd633d4bb5dd76a: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Number_c4bdf66bb78f7977: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_Window_afcc911b2f9c92e2: function(arg0) {
            const ret = arg0.Window;
            return ret;
        },
        __wbg_WorkerGlobalScope_5d19ebc889ff397e: function(arg0) {
            const ret = arg0.WorkerGlobalScope;
            return ret;
        },
        __wbg___wbindgen_bigint_get_as_i64_d9e915702856f831: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_edaed31a367ce1bd: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_8a447059637473e2: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_4990f46af709e33c: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_90b5ccfe67c78460: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_function_acc5528be2b923f2: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_6d937fbfb6478470: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_0beba4a1980d3eea: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_1fca8072260dd261: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_721f8decd50c87a3: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_4e8c38722cb8ff51: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_4b9aba9e5b3c4582: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_1cc01dd708740256: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_71bb4348194e31f0: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_ea4887a5f8f9a9db: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_33c39e13d73b25f6: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_addColorStop_d8d26268addcc37f: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addColorStop(arg1, getStringFromWasm0(arg2, arg3));
        }, arguments); },
        __wbg_appendChild_acb7691406591783: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_arc_74cf0c033e9df542: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.arc(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_arrayBuffer_e3174a1300c67c95: function(arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        },
        __wbg_beginComputePass_431a159006c13c7c: function(arg0, arg1) {
            const ret = arg0.beginComputePass(arg1);
            return ret;
        },
        __wbg_beginPath_c99b5be3516a2077: function(arg0) {
            arg0.beginPath();
        },
        __wbg_beginRenderPass_aa22c432e793359a: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.beginRenderPass(arg1);
            return ret;
        }, arguments); },
        __wbg_byteLength_e497be232a57cf88: function(arg0) {
            const ret = arg0.byteLength;
            return ret;
        },
        __wbg_call_5575218572ead796: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_call_8e98ed2f3c86c4b5: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_close_966124c5dc910fa4: function(arg0) {
            const ret = arg0.close();
            return ret;
        },
        __wbg_configure_0e4789c0f6b35c8e: function() { return handleError(function (arg0, arg1) {
            arg0.configure(arg1);
        }, arguments); },
        __wbg_copyBufferToBuffer_5e2cd8f10ae78183: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.copyBufferToBuffer(arg1, arg2, arg3, arg4);
        }, arguments); },
        __wbg_copyBufferToBuffer_ca30deb8de65f5d5: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.copyBufferToBuffer(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_copyTextureToBuffer_ed6e67a77ecb768d: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.copyTextureToBuffer(arg1, arg2, arg3);
        }, arguments); },
        __wbg_createBindGroupLayout_49a7e2b3d076afcf: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBindGroupLayout(arg1);
            return ret;
        }, arguments); },
        __wbg_createBindGroup_655c6e6c0258530e: function(arg0, arg1) {
            const ret = arg0.createBindGroup(arg1);
            return ret;
        },
        __wbg_createBuffer_0726dd2ab09ea1d2: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBuffer(arg1);
            return ret;
        }, arguments); },
        __wbg_createCommandEncoder_ec1f40f0cb4d09df: function(arg0, arg1) {
            const ret = arg0.createCommandEncoder(arg1);
            return ret;
        },
        __wbg_createComputePipeline_2545c47f08715810: function(arg0, arg1) {
            const ret = arg0.createComputePipeline(arg1);
            return ret;
        },
        __wbg_createElement_9e23ac95e40e302c: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createLinearGradient_e941e9b32e45fd4d: function(arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.createLinearGradient(arg1, arg2, arg3, arg4);
            return ret;
        },
        __wbg_createPipelineLayout_2c8cd4528b06c108: function(arg0, arg1) {
            const ret = arg0.createPipelineLayout(arg1);
            return ret;
        },
        __wbg_createRenderPipeline_cf98d4d699bfb03c: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createRenderPipeline(arg1);
            return ret;
        }, arguments); },
        __wbg_createSampler_c8ffb3c8d565f704: function(arg0, arg1) {
            const ret = arg0.createSampler(arg1);
            return ret;
        },
        __wbg_createShaderModule_2e44fc7677c6288b: function(arg0, arg1) {
            const ret = arg0.createShaderModule(arg1);
            return ret;
        },
        __wbg_createTexture_1bac74c999b8a48e: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createTexture(arg1);
            return ret;
        }, arguments); },
        __wbg_createView_ceaf2f5881adbd34: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createView(arg1);
            return ret;
        }, arguments); },
        __wbg_createWritable_fe536097cf251da6: function(arg0) {
            const ret = arg0.createWritable();
            return ret;
        },
        __wbg_dispatchWorkgroups_afb2344298c62227: function(arg0, arg1, arg2, arg3) {
            arg0.dispatchWorkgroups(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0);
        },
        __wbg_document_2634180a4c694068: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_done_b62d4a7d2286852a: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_drawIndexed_d31913e79d58fbac: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.drawIndexed(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5 >>> 0);
        },
        __wbg_draw_6877f98847e1e36c: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.draw(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_end_c36889de8ddef882: function(arg0) {
            arg0.end();
        },
        __wbg_end_f99ebed53d4e198a: function(arg0) {
            arg0.end();
        },
        __wbg_entries_c261c3fa1f281256: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_estimate_1b62d27c90cb9fd8: function() { return handleError(function (arg0) {
            const ret = arg0.estimate();
            return ret;
        }, arguments); },
        __wbg_features_2b07a28fe18ad0ce: function(arg0) {
            const ret = arg0.features;
            return ret;
        },
        __wbg_features_d4d2020319592cbe: function(arg0) {
            const ret = arg0.features;
            return ret;
        },
        __wbg_fillRect_3c420f5077df8d3b: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.fillRect(arg1, arg2, arg3, arg4);
        },
        __wbg_fillText_cdea0ac33ff3d2d1: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.fillText(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_fill_b39141050e50c461: function(arg0) {
            arg0.fill();
        },
        __wbg_finish_4d91de5e927dd13f: function(arg0, arg1) {
            const ret = arg0.finish(arg1);
            return ret;
        },
        __wbg_finish_6e06b68ab68cd9f6: function(arg0) {
            const ret = arg0.finish();
            return ret;
        },
        __wbg_fromCodePoint_93fb75ffd4cdf384: function() { return handleError(function (arg0) {
            const ret = String.fromCodePoint(arg0 >>> 0);
            return ret;
        }, arguments); },
        __wbg_getBindGroupLayout_d38545dfc4eab8d5: function(arg0, arg1) {
            const ret = arg0.getBindGroupLayout(arg1 >>> 0);
            return ret;
        },
        __wbg_getContext_486aab500e1c34c9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getContext_70c2d1bed75d4122: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getCurrentTexture_20714d1bd9051cab: function() { return handleError(function (arg0) {
            const ret = arg0.getCurrentTexture();
            return ret;
        }, arguments); },
        __wbg_getDirectory_d73e4f2473279f77: function(arg0) {
            const ret = arg0.getDirectory();
            return ret;
        },
        __wbg_getElementById_c7aba6b93b34bf01: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getFileHandle_01abdcb9df490ed0: function(arg0, arg1, arg2) {
            const ret = arg0.getFileHandle(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_getFileHandle_fdf8a7ba5211ee45: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getFileHandle(getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        },
        __wbg_getFile_52d8d185c309296e: function(arg0) {
            const ret = arg0.getFile();
            return ret;
        },
        __wbg_getMappedRange_d0bf3141224111b6: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getMappedRange(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_getPreferredCanvasFormat_8b57039d1801a506: function(arg0) {
            const ret = arg0.getPreferredCanvasFormat();
            return (__wbindgen_enum_GpuTextureFormat.indexOf(ret) + 1 || 102) - 1;
        },
        __wbg_getRandomValues_cc7f052a444bb2ce: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_get_197a3fe98f169e38: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_37b48b8fa52d1f2c: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_9a29be2cb383ed9a: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_dddb90ff5d27a080: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_unchecked_54a4374c38e08460: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_6412cf3094599694: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_gpu_2ccc250735d24a2a: function(arg0) {
            const ret = arg0.gpu;
            return ret;
        },
        __wbg_has_0c97053e877f47cc: function(arg0, arg1, arg2) {
            const ret = arg0.has(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_height_a04613570d793df2: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_instanceof_ArrayBuffer_2a7bb09fee70c2da: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Blob_204c5c5bad0fb849: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Blob;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_CanvasRenderingContext2d_d0cab9e931424c52: function(arg0) {
            let result;
            try {
                result = arg0 instanceof CanvasRenderingContext2D;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_FileSystemDirectoryHandle_b02c76e3b2655b0c: function(arg0) {
            let result;
            try {
                result = arg0 instanceof FileSystemDirectoryHandle;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_FileSystemFileHandle_6b3a14582880afd2: function(arg0) {
            let result;
            try {
                result = arg0 instanceof FileSystemFileHandle;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_FileSystemWritableFileStream_c73e53d043da9da6: function(arg0) {
            let result;
            try {
                result = arg0 instanceof FileSystemWritableFileStream;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_File_2d5bf7d3a7b931e9: function(arg0) {
            let result;
            try {
                result = arg0 instanceof File;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuOutOfMemoryError_6429c750997f1c8d: function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUOutOfMemoryError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuValidationError_75fa3611f065f4df: function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUValidationError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlCanvasElement_8ce29a370a2b10a4: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLCanvasElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_StorageEstimate_9a407e5e1042f4a0: function(arg0) {
            let result;
            try {
                result = arg0 instanceof StorageEstimate;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_f080092dc70f5d58: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_0d356b88a2f77c42: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_145a34fd0a38d37b: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isArray_ffd528e87c3c8cef: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isSafeInteger_a3389a198582f5f6: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_cc47ba25a2be735a: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_label_7ed42f25f841996b: function(arg0, arg1) {
            const ret = arg1.label;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_length_4875795b8939c6b6: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_589238bdcf171f0e: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_c6054974c0a6cdb9: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_limits_20c6f56636df7d38: function(arg0) {
            const ret = arg0.limits;
            return ret;
        },
        __wbg_lineTo_2a649fce185f0bf0: function(arg0, arg1, arg2) {
            arg0.lineTo(arg1, arg2);
        },
        __wbg_log_6b5af08dd293697f: function(arg0) {
            console.log(arg0);
        },
        __wbg_mapAsync_52b01fa9e8f765fd: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.mapAsync(arg1 >>> 0, arg2, arg3);
            return ret;
        },
        __wbg_maxBindGroupsPlusVertexBuffers_33e5006b23e20478: function(arg0) {
            const ret = arg0.maxBindGroupsPlusVertexBuffers;
            return ret;
        },
        __wbg_maxBindGroups_f6d26f3a67826666: function(arg0) {
            const ret = arg0.maxBindGroups;
            return ret;
        },
        __wbg_maxBindingsPerBindGroup_edab2e8dabbf6060: function(arg0) {
            const ret = arg0.maxBindingsPerBindGroup;
            return ret;
        },
        __wbg_maxBufferSize_bbc69284c14aa7da: function(arg0) {
            const ret = arg0.maxBufferSize;
            return ret;
        },
        __wbg_maxColorAttachmentBytesPerSample_63ebe4f81de2f34c: function(arg0) {
            const ret = arg0.maxColorAttachmentBytesPerSample;
            return ret;
        },
        __wbg_maxColorAttachments_aed8c38beabf3a5c: function(arg0) {
            const ret = arg0.maxColorAttachments;
            return ret;
        },
        __wbg_maxComputeInvocationsPerWorkgroup_2d964564c37f1c65: function(arg0) {
            const ret = arg0.maxComputeInvocationsPerWorkgroup;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeX_a3e3206570da184f: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeX;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeY_dffa4a62244b7563: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeY;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeZ_976ebcb760f6d07d: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeZ;
            return ret;
        },
        __wbg_maxComputeWorkgroupStorageSize_2e8dbece6e532e2a: function(arg0) {
            const ret = arg0.maxComputeWorkgroupStorageSize;
            return ret;
        },
        __wbg_maxComputeWorkgroupsPerDimension_bb7d36b4d20c80f4: function(arg0) {
            const ret = arg0.maxComputeWorkgroupsPerDimension;
            return ret;
        },
        __wbg_maxDynamicStorageBuffersPerPipelineLayout_1ca859cb96a414e0: function(arg0) {
            const ret = arg0.maxDynamicStorageBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxDynamicUniformBuffersPerPipelineLayout_e968f2c8cd8f4d46: function(arg0) {
            const ret = arg0.maxDynamicUniformBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxInterStageShaderVariables_138ac882c4d6a3d3: function(arg0) {
            const ret = arg0.maxInterStageShaderVariables;
            return ret;
        },
        __wbg_maxSampledTexturesPerShaderStage_bb3e6b2698321fa6: function(arg0) {
            const ret = arg0.maxSampledTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxSamplersPerShaderStage_98c00a1829fa414b: function(arg0) {
            const ret = arg0.maxSamplersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageBufferBindingSize_e500e31f479e669e: function(arg0) {
            const ret = arg0.maxStorageBufferBindingSize;
            return ret;
        },
        __wbg_maxStorageBuffersPerShaderStage_eb663f6d7521b6a7: function(arg0) {
            const ret = arg0.maxStorageBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageTexturesPerShaderStage_bb3ad93b53e618c0: function(arg0) {
            const ret = arg0.maxStorageTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxTextureArrayLayers_2a56d05fb261c99a: function(arg0) {
            const ret = arg0.maxTextureArrayLayers;
            return ret;
        },
        __wbg_maxTextureDimension1D_84590c1d4770d319: function(arg0) {
            const ret = arg0.maxTextureDimension1D;
            return ret;
        },
        __wbg_maxTextureDimension2D_7f2b5c8b2727e3fc: function(arg0) {
            const ret = arg0.maxTextureDimension2D;
            return ret;
        },
        __wbg_maxTextureDimension3D_7f3babddf55c32a6: function(arg0) {
            const ret = arg0.maxTextureDimension3D;
            return ret;
        },
        __wbg_maxUniformBufferBindingSize_d80a09e23c0b284c: function(arg0) {
            const ret = arg0.maxUniformBufferBindingSize;
            return ret;
        },
        __wbg_maxUniformBuffersPerShaderStage_0b8b2de676fa740e: function(arg0) {
            const ret = arg0.maxUniformBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxVertexAttributes_a693dd921316649b: function(arg0) {
            const ret = arg0.maxVertexAttributes;
            return ret;
        },
        __wbg_maxVertexBufferArrayStride_f256d91f281076cb: function(arg0) {
            const ret = arg0.maxVertexBufferArrayStride;
            return ret;
        },
        __wbg_maxVertexBuffers_70ab564b25d5ac20: function(arg0) {
            const ret = arg0.maxVertexBuffers;
            return ret;
        },
        __wbg_message_4ada57a3710f1502: function(arg0, arg1) {
            const ret = arg1.message;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_minStorageBufferOffsetAlignment_3248ed00dcdbf79f: function(arg0) {
            const ret = arg0.minStorageBufferOffsetAlignment;
            return ret;
        },
        __wbg_minUniformBufferOffsetAlignment_3b9fa4caae03e903: function(arg0) {
            const ret = arg0.minUniformBufferOffsetAlignment;
            return ret;
        },
        __wbg_moveTo_8973531c3399ba16: function(arg0, arg1, arg2) {
            arg0.moveTo(arg1, arg2);
        },
        __wbg_navigator_017bc45e84c473cc: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_navigator_935098efd1dc7fe5: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_2e117a478906f062: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_3444eb7412549f0b: function() {
            const ret = new Map();
            return ret;
        },
        __wbg_new_36e147a8ced3c6e0: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_55041f0354b8ea99: function() { return handleError(function (arg0, arg1) {
            const ret = new OffscreenCanvas(arg0 >>> 0, arg1 >>> 0);
            return ret;
        }, arguments); },
        __wbg_new_81880fb5002cb255: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_dcab74c3ef13eacf: function(arg0) {
            const ret = new SharedArrayBuffer(arg0 >>> 0);
            return ret;
        },
        __wbg_new_e66a4b7758dd2e5c: function(arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_543b875b27789a8f: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_6696ab0c133d3f19: function(arg0, arg1) {
            const ret = new Float32Array(getArrayF32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_98e57cb2fe2e6a5d: function(arg0, arg1) {
            const ret = new Float64Array(getArrayF64FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_00a409eb4ec4f2d9: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h243b5e59773a58aa(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_typed_a3f38c885cf92575: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_with_byte_offset_and_length_f2b65504a914f37a: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_new_with_length_9b650f44b5c44a4e: function(arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return ret;
        },
        __wbg_new_with_length_ecf42de9ee25651c: function(arg0) {
            const ret = new Uint32Array(arg0 >>> 0);
            return ret;
        },
        __wbg_next_0c4066e251d2eff9: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_next_402fa10b59ab20c3: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_onSubmittedWorkDone_270d6b5a45520e79: function(arg0) {
            const ret = arg0.onSubmittedWorkDone();
            return ret;
        },
        __wbg_popErrorScope_8e7f4cbff8b758dd: function(arg0) {
            const ret = arg0.popErrorScope();
            return ret;
        },
        __wbg_prototypesetcall_d721637c7ca66eb8: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_pushErrorScope_fa23d1206bc26dce: function(arg0, arg1) {
            arg0.pushErrorScope(__wbindgen_enum_GpuErrorFilter[arg1]);
        },
        __wbg_push_f724b5db8acf89d2: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_querySelectorAll_ffda3c891a9eb29a: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelectorAll(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_queueMicrotask_1c9b3800e321a967: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_311744e534a929a3: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queue_adce34608fd0c893: function(arg0) {
            const ret = arg0.queue;
            return ret;
        },
        __wbg_requestAdapter_2e6718811c735a57: function(arg0, arg1) {
            const ret = arg0.requestAdapter(arg1);
            return ret;
        },
        __wbg_requestDevice_ab46d0519ea1cc34: function(arg0, arg1) {
            const ret = arg0.requestDevice(arg1);
            return ret;
        },
        __wbg_resolve_d82363d90af6928a: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_send_d3ba4386db8a6937: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_setAttribute_8bccfbabf2a83682: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setBindGroup_268fd1714fff0ef5: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setBindGroup(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
        }, arguments); },
        __wbg_setBindGroup_3e4ce136bc833ea1: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setBindGroup(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
        }, arguments); },
        __wbg_setBindGroup_c22a1b95c0b17f37: function(arg0, arg1, arg2) {
            arg0.setBindGroup(arg1 >>> 0, arg2);
        },
        __wbg_setBindGroup_f0de6cb2c7dbfc2c: function(arg0, arg1, arg2) {
            arg0.setBindGroup(arg1 >>> 0, arg2);
        },
        __wbg_setIndexBuffer_7f3cf667b4d71566: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setIndexBuffer(arg1, __wbindgen_enum_GpuIndexFormat[arg2], arg3, arg4);
        },
        __wbg_setPipeline_c41bf46790f27f9e: function(arg0, arg1) {
            arg0.setPipeline(arg1);
        },
        __wbg_setPipeline_d73f019e98c76d2d: function(arg0, arg1) {
            arg0.setPipeline(arg1);
        },
        __wbg_setVertexBuffer_1e448859663dd400: function(arg0, arg1, arg2, arg3) {
            arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3);
        },
        __wbg_setVertexBuffer_7cf533d694e747f3: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3, arg4);
        },
        __wbg_set_0bf1fca872bc6d18: function(arg0, arg1, arg2) {
            arg0.set(getArrayU8FromWasm0(arg1, arg2));
        },
        __wbg_set_272b80acaf9a75e8: function(arg0, arg1, arg2) {
            arg0.set(arg1, arg2 >>> 0);
        },
        __wbg_set_4564f7dc44fcb0c9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_652044250ad0332b: function(arg0, arg1, arg2) {
            arg0.set(getArrayU32FromWasm0(arg1, arg2));
        },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_9a1d61e17de7054c: function(arg0, arg1, arg2) {
            const ret = arg0.set(arg1, arg2);
            return ret;
        },
        __wbg_set_a_88262a42340d0b1c: function(arg0, arg1) {
            arg0.a = arg1;
        },
        __wbg_set_access_9a5092f05dc45fad: function(arg0, arg1) {
            arg0.access = __wbindgen_enum_GpuStorageTextureAccess[arg1];
        },
        __wbg_set_address_mode_u_9e2695575a219e33: function(arg0, arg1) {
            arg0.addressModeU = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_v_f479b2e6cccbcac4: function(arg0, arg1) {
            arg0.addressModeV = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_w_46273e153230180d: function(arg0, arg1) {
            arg0.addressModeW = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_alpha_bfd2df62e7bc581b: function(arg0, arg1) {
            arg0.alpha = arg1;
        },
        __wbg_set_alpha_mode_df805952892caa9c: function(arg0, arg1) {
            arg0.alphaMode = __wbindgen_enum_GpuCanvasAlphaMode[arg1];
        },
        __wbg_set_alpha_to_coverage_enabled_8b5dc2b0a225b3b2: function(arg0, arg1) {
            arg0.alphaToCoverageEnabled = arg1 !== 0;
        },
        __wbg_set_array_layer_count_7312f0f31af94e7c: function(arg0, arg1) {
            arg0.arrayLayerCount = arg1 >>> 0;
        },
        __wbg_set_array_stride_f64_27ffaf4fffd74e61: function(arg0, arg1) {
            arg0.arrayStride = arg1;
        },
        __wbg_set_aspect_0d453bca3d012f02: function(arg0, arg1) {
            arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_aspect_4962514fe99e68e6: function(arg0, arg1) {
            arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_attributes_7537844a7e6dafdc: function(arg0, arg1, arg2) {
            arg0.attributes = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_b_c47befe0af3261eb: function(arg0, arg1) {
            arg0.b = arg1;
        },
        __wbg_set_base_array_layer_f176bb9f1b37b342: function(arg0, arg1) {
            arg0.baseArrayLayer = arg1 >>> 0;
        },
        __wbg_set_base_mip_level_1df145d9f8db32a9: function(arg0, arg1) {
            arg0.baseMipLevel = arg1 >>> 0;
        },
        __wbg_set_beginning_of_pass_write_index_0d4fa06109208ad7: function(arg0, arg1) {
            arg0.beginningOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_beginning_of_pass_write_index_e9f5d016947893bd: function(arg0, arg1) {
            arg0.beginningOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_bind_group_layouts_5a9cfea401c020ab: function(arg0, arg1, arg2) {
            arg0.bindGroupLayouts = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_binding_155b0440b4307793: function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        },
        __wbg_set_binding_f74df3510792aba1: function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        },
        __wbg_set_blend_7493c2066c3e9970: function(arg0, arg1) {
            arg0.blend = arg1;
        },
        __wbg_set_buffer_9c01e3b6d6765ea2: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffer_c3410572051920ba: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffer_ef7f75306cf663ed: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffers_7d0d8f507699e956: function(arg0, arg1, arg2) {
            arg0.buffers = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_bytes_per_row_c54ca96953f35774: function(arg0, arg1) {
            arg0.bytesPerRow = arg1 >>> 0;
        },
        __wbg_set_className_19e05f9bbe754550: function(arg0, arg1, arg2) {
            arg0.className = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_clear_value_gpu_color_dict_6211425789c76e59: function(arg0, arg1) {
            arg0.clearValue = arg1;
        },
        __wbg_set_code_b4f37f81f45b5b25: function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_color_83aa977526e88cbb: function(arg0, arg1) {
            arg0.color = arg1;
        },
        __wbg_set_color_attachments_581fdb3310e4abfa: function(arg0, arg1, arg2) {
            arg0.colorAttachments = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_compare_cd9b62cdb92eb580: function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_compare_f36b34abfaa08ccb: function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_compute_d0c2d276b6d4b18d: function(arg0, arg1) {
            arg0.compute = arg1;
        },
        __wbg_set_count_069a4eac409bac55: function(arg0, arg1) {
            arg0.count = arg1 >>> 0;
        },
        __wbg_set_create_b9be7a200245a2da: function(arg0, arg1) {
            arg0.create = arg1 !== 0;
        },
        __wbg_set_cull_mode_fc649853947a3d0c: function(arg0, arg1) {
            arg0.cullMode = __wbindgen_enum_GpuCullMode[arg1];
        },
        __wbg_set_dc601f4a69da0bc2: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_depth_bias_clamp_1c0d695df7f092e5: function(arg0, arg1) {
            arg0.depthBiasClamp = arg1;
        },
        __wbg_set_depth_bias_d7cd16096242a657: function(arg0, arg1) {
            arg0.depthBias = arg1;
        },
        __wbg_set_depth_bias_slope_scale_c4e52ec743ef55ba: function(arg0, arg1) {
            arg0.depthBiasSlopeScale = arg1;
        },
        __wbg_set_depth_clear_value_beda3ec5b1a5c43a: function(arg0, arg1) {
            arg0.depthClearValue = arg1;
        },
        __wbg_set_depth_compare_0c8631eb2eae98e3: function(arg0, arg1) {
            arg0.depthCompare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_depth_fail_op_668155ae33d3c06f: function(arg0, arg1) {
            arg0.depthFailOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_depth_load_op_511c513eab4e56a9: function(arg0, arg1) {
            arg0.depthLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_depth_or_array_layers_89371305ed0bd962: function(arg0, arg1) {
            arg0.depthOrArrayLayers = arg1 >>> 0;
        },
        __wbg_set_depth_read_only_7f41a74741c144ec: function(arg0, arg1) {
            arg0.depthReadOnly = arg1 !== 0;
        },
        __wbg_set_depth_stencil_97506c7bea4f53da: function(arg0, arg1) {
            arg0.depthStencil = arg1;
        },
        __wbg_set_depth_stencil_attachment_73b79e8b4e948222: function(arg0, arg1) {
            arg0.depthStencilAttachment = arg1;
        },
        __wbg_set_depth_store_op_c89f33b39b43361c: function(arg0, arg1) {
            arg0.depthStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_depth_write_enabled_ce89750042940350: function(arg0, arg1) {
            arg0.depthWriteEnabled = arg1 !== 0;
        },
        __wbg_set_device_e275d1d4f3c9eb74: function(arg0, arg1) {
            arg0.device = arg1;
        },
        __wbg_set_dimension_868eee80f4b90011: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureDimension[arg1];
        },
        __wbg_set_dimension_e325282e613ca0a4: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_dst_factor_ec7407f19be1aff9: function(arg0, arg1) {
            arg0.dstFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_end_of_pass_write_index_0d546e46b86ea069: function(arg0, arg1) {
            arg0.endOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_end_of_pass_write_index_49a6ddbb2b888bfa: function(arg0, arg1) {
            arg0.endOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_entries_86a29dd6291c95e7: function(arg0, arg1, arg2) {
            arg0.entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entries_a12aca1e458b0456: function(arg0, arg1, arg2) {
            arg0.entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_207540f042015ce5: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_5f26aacbe4c545eb: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_e87e79251dd3144f: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_external_texture_386483d8dd82ab56: function(arg0, arg1) {
            arg0.externalTexture = arg1;
        },
        __wbg_set_fail_op_92f716dbc88b6973: function(arg0, arg1) {
            arg0.failOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_fillStyle_01152e00b5737643: function(arg0, arg1) {
            arg0.fillStyle = arg1;
        },
        __wbg_set_font_e2bce6175ef42bc3: function(arg0, arg1, arg2) {
            arg0.font = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_format_1fcaa7d60546b490: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_2c1414a817c213f8: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_533f9ffa7eef563d: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_5d2f25cc93654ecc: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuVertexFormat[arg1];
        },
        __wbg_set_format_5ff53724ed6cedf2: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_815efd4dc4817bbb: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_e52bdcca880d2c8e: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_fragment_8b780f00a0b0e6f3: function(arg0, arg1) {
            arg0.fragment = arg1;
        },
        __wbg_set_front_face_28ffdf524eedce5b: function(arg0, arg1) {
            arg0.frontFace = __wbindgen_enum_GpuFrontFace[arg1];
        },
        __wbg_set_g_5983abfc46e0cf4e: function(arg0, arg1) {
            arg0.g = arg1;
        },
        __wbg_set_has_dynamic_offset_62bc230bdb7c54d0: function(arg0, arg1) {
            arg0.hasDynamicOffset = arg1 !== 0;
        },
        __wbg_set_height_14335c4047cf9c1b: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_height_ad5056ea051acd78: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_height_ef298446b359b0c5: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_innerHTML_30fcedd016ac76ab: function(arg0, arg1, arg2) {
            arg0.innerHTML = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_08d9be3e4719c226: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_17eb9fe3a02f62b0: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_48e6b787d256f621: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_547d0d4aec39fbe9: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_5ee7427342869829: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_60ad96c811e0d109: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_6db0393d3fdc90a5: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_72bb4f41ef0cb893: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_79387decda299036: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_9556af8b5cda3c9d: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_d010f237b26f2c55: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_e16e2dbe51349c7f: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_e3944e54881b8c50: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_e922700240417ab5: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_ef44793ddf4455c5: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_layout_41021cfd2a2f62df: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_50ab727f44b38f26: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_913d53c17194c989: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_gpu_auto_layout_mode_0a5a185b3d52b726: function(arg0, arg1) {
            arg0.layout = __wbindgen_enum_GpuAutoLayoutMode[arg1];
        },
        __wbg_set_layout_gpu_auto_layout_mode_aeba193938b47882: function(arg0, arg1) {
            arg0.layout = __wbindgen_enum_GpuAutoLayoutMode[arg1];
        },
        __wbg_set_load_op_99661da6c4eab9b0: function(arg0, arg1) {
            arg0.loadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_lod_max_clamp_dd2d9f9f052f4f44: function(arg0, arg1) {
            arg0.lodMaxClamp = arg1;
        },
        __wbg_set_lod_min_clamp_6d20c97916baeb93: function(arg0, arg1) {
            arg0.lodMinClamp = arg1;
        },
        __wbg_set_mag_filter_b5adebc99cb938e1: function(arg0, arg1) {
            arg0.magFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mapped_at_creation_81b586dc90a50347: function(arg0, arg1) {
            arg0.mappedAtCreation = arg1 !== 0;
        },
        __wbg_set_mask_70a8a59ce09e5997: function(arg0, arg1) {
            arg0.mask = arg1 >>> 0;
        },
        __wbg_set_max_anisotropy_2beada0e2db62c45: function(arg0, arg1) {
            arg0.maxAnisotropy = arg1;
        },
        __wbg_set_min_binding_size_f64_5005a6904cdf43da: function(arg0, arg1) {
            arg0.minBindingSize = arg1;
        },
        __wbg_set_min_filter_c72f17375e135f0a: function(arg0, arg1) {
            arg0.minFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mip_level_count_534caaa7e68e68b8: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mip_level_count_776c8c218b65bc08: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mip_level_f7ac79e8c54f59ad: function(arg0, arg1) {
            arg0.mipLevel = arg1 >>> 0;
        },
        __wbg_set_mipmap_filter_5bf66195a3639700: function(arg0, arg1) {
            arg0.mipmapFilter = __wbindgen_enum_GpuMipmapFilterMode[arg1];
        },
        __wbg_set_mode_9990b3393ba469ae: function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_GpuCanvasToneMappingMode[arg1];
        },
        __wbg_set_module_5db9b76ee2dd2a59: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_module_d0e2098713606cae: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_module_f02e076ca7e7daf8: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_multisample_37ddafe88b5cd466: function(arg0, arg1) {
            arg0.multisample = arg1;
        },
        __wbg_set_multisampled_7913fd7183272840: function(arg0, arg1) {
            arg0.multisampled = arg1 !== 0;
        },
        __wbg_set_offset_f64_28c24dc15000932e: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_offset_f64_89f0ce01a689839e: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_offset_f64_b562d1367e34ef93: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_operation_62ce44e1728c4047: function(arg0, arg1) {
            arg0.operation = __wbindgen_enum_GpuBlendOperation[arg1];
        },
        __wbg_set_origin_gpu_origin_3d_dict_631c04520718091f: function(arg0, arg1) {
            arg0.origin = arg1;
        },
        __wbg_set_pass_op_cf02fa088d6352a7: function(arg0, arg1) {
            arg0.passOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_power_preference_8fdca0b7af640d49: function(arg0, arg1) {
            arg0.powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
        },
        __wbg_set_primitive_43c23761a55b4088: function(arg0, arg1) {
            arg0.primitive = arg1;
        },
        __wbg_set_query_set_41de86d2401aee04: function(arg0, arg1) {
            arg0.querySet = arg1;
        },
        __wbg_set_query_set_4889dd944d5ec0fd: function(arg0, arg1) {
            arg0.querySet = arg1;
        },
        __wbg_set_r_c6f4c68f4804d655: function(arg0, arg1) {
            arg0.r = arg1;
        },
        __wbg_set_required_features_1baf274a8669db60: function(arg0, arg1, arg2) {
            arg0.requiredFeatures = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_required_limits_871ed33c68613dcb: function(arg0, arg1) {
            arg0.requiredLimits = arg1;
        },
        __wbg_set_resolve_target_gpu_texture_view_b19a4f2debf79b96: function(arg0, arg1) {
            arg0.resolveTarget = arg1;
        },
        __wbg_set_resource_5ae7b5e67924f234: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_buffer_binding_e5dbca063e7cb67b: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_texture_view_eb46c355d51ad7e5: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_rows_per_image_5011f97318ee71af: function(arg0, arg1) {
            arg0.rowsPerImage = arg1 >>> 0;
        },
        __wbg_set_sample_count_eb86a8b18545b54f: function(arg0, arg1) {
            arg0.sampleCount = arg1 >>> 0;
        },
        __wbg_set_sample_type_c32e1dfff94e63eb: function(arg0, arg1) {
            arg0.sampleType = __wbindgen_enum_GpuTextureSampleType[arg1];
        },
        __wbg_set_sampler_c0e1258543a33bce: function(arg0, arg1) {
            arg0.sampler = arg1;
        },
        __wbg_set_shader_location_7e1832a74f912217: function(arg0, arg1) {
            arg0.shaderLocation = arg1 >>> 0;
        },
        __wbg_set_size_f64_6bcd40704bf4cfdc: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_f64_8b8f6bba5d678162: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_gpu_extent_3d_dict_7e42e1c98fa36434: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_src_factor_9bfe84af9b7b5cac: function(arg0, arg1) {
            arg0.srcFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_stencil_back_85b22f1db5b1940a: function(arg0, arg1) {
            arg0.stencilBack = arg1;
        },
        __wbg_set_stencil_clear_value_42be608809151e2a: function(arg0, arg1) {
            arg0.stencilClearValue = arg1 >>> 0;
        },
        __wbg_set_stencil_front_525526164a798a44: function(arg0, arg1) {
            arg0.stencilFront = arg1;
        },
        __wbg_set_stencil_load_op_31838c036993098a: function(arg0, arg1) {
            arg0.stencilLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_stencil_read_mask_5cc26495e8b3ae82: function(arg0, arg1) {
            arg0.stencilReadMask = arg1 >>> 0;
        },
        __wbg_set_stencil_read_only_bf1d0c1897e25c62: function(arg0, arg1) {
            arg0.stencilReadOnly = arg1 !== 0;
        },
        __wbg_set_stencil_store_op_e6be1cbc3a8fc210: function(arg0, arg1) {
            arg0.stencilStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_stencil_write_mask_d9cb40ec4b4bee5b: function(arg0, arg1) {
            arg0.stencilWriteMask = arg1 >>> 0;
        },
        __wbg_set_step_mode_a97bb24714da41a9: function(arg0, arg1) {
            arg0.stepMode = __wbindgen_enum_GpuVertexStepMode[arg1];
        },
        __wbg_set_storage_texture_939a097db4b18bd4: function(arg0, arg1) {
            arg0.storageTexture = arg1;
        },
        __wbg_set_store_op_b5fdf672436f13f3: function(arg0, arg1) {
            arg0.storeOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_strip_index_format_9f787be6c5fc9e87: function(arg0, arg1) {
            arg0.stripIndexFormat = __wbindgen_enum_GpuIndexFormat[arg1];
        },
        __wbg_set_strokeStyle_77f54c809146a711: function(arg0, arg1) {
            arg0.strokeStyle = arg1;
        },
        __wbg_set_targets_c38bd200c836d66f: function(arg0, arg1, arg2) {
            arg0.targets = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_textContent_5c5fef072bd24f7a: function(arg0, arg1, arg2) {
            arg0.textContent = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_texture_1f64653a5d2d7b4d: function(arg0, arg1) {
            arg0.texture = arg1;
        },
        __wbg_set_texture_9dcedde1bb31eda6: function(arg0, arg1) {
            arg0.texture = arg1;
        },
        __wbg_set_timestamp_writes_59c2d19ed8aecd97: function(arg0, arg1) {
            arg0.timestampWrites = arg1;
        },
        __wbg_set_timestamp_writes_98bed1a8bbc6682d: function(arg0, arg1) {
            arg0.timestampWrites = arg1;
        },
        __wbg_set_tone_mapping_b3464f1baa4cff92: function(arg0, arg1) {
            arg0.toneMapping = arg1;
        },
        __wbg_set_topology_da25f2cc5af203d2: function(arg0, arg1) {
            arg0.topology = __wbindgen_enum_GpuPrimitiveTopology[arg1];
        },
        __wbg_set_type_ccf8472d40abcddf: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_GpuSamplerBindingType[arg1];
        },
        __wbg_set_type_d09829f59932a0fc: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_GpuBufferBindingType[arg1];
        },
        __wbg_set_unclipped_depth_04524a2b44e1e3c1: function(arg0, arg1) {
            arg0.unclippedDepth = arg1 !== 0;
        },
        __wbg_set_usage_a137f82ca163b0a9: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_b2a2935f37bf3d08: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_ba5b0f8b333ab325: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_ddd42599bbba7779: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_vertex_0be5d146f9ff6f36: function(arg0, arg1) {
            arg0.vertex = arg1;
        },
        __wbg_set_view_dimension_0df554032f1f3a85: function(arg0, arg1) {
            arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_view_dimension_4818d4c18ce5815e: function(arg0, arg1) {
            arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_view_formats_4347dc8363331086: function(arg0, arg1, arg2) {
            arg0.viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_formats_5797d2fff3c11808: function(arg0, arg1, arg2) {
            arg0.viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_gpu_texture_view_9b2d86b6b99d9fd9: function(arg0, arg1) {
            arg0.view = arg1;
        },
        __wbg_set_view_gpu_texture_view_c0f35f8857c25206: function(arg0, arg1) {
            arg0.view = arg1;
        },
        __wbg_set_visibility_9570b037224c4cc2: function(arg0, arg1) {
            arg0.visibility = arg1 >>> 0;
        },
        __wbg_set_width_031bdecd763c5855: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_width_9f685402c2cbee70: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_width_f9e631f4ee129e5c: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_write_mask_d45279e56abbfcb5: function(arg0, arg1) {
            arg0.writeMask = arg1 >>> 0;
        },
        __wbg_set_x_876d592971db129a: function(arg0, arg1) {
            arg0.x = arg1 >>> 0;
        },
        __wbg_set_y_2b1f5ac0dd5586a5: function(arg0, arg1) {
            arg0.y = arg1 >>> 0;
        },
        __wbg_set_z_ef005d82bc9d24e3: function(arg0, arg1) {
            arg0.z = arg1 >>> 0;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_THIS_2fee5048bcca5938: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_ce44e66a4935da8c: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_44f6e0cb5e67cdad: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_168f178805d978fe: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_storage_a0b279da98719bb8: function(arg0) {
            const ret = arg0.storage;
            return ret;
        },
        __wbg_stroke_d0c2cfbe28711bcb: function(arg0) {
            arg0.stroke();
        },
        __wbg_submit_ce44115121cd166c: function(arg0, arg1, arg2) {
            arg0.submit(getArrayJsValueViewFromWasm0(arg1, arg2));
        },
        __wbg_then_05edfc8a4fea5106: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_591b6b3a75ee817a: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_unconfigure_0a07a0a40de8988d: function(arg0) {
            arg0.unconfigure();
        },
        __wbg_unmap_adaf93276fdf9aaf: function(arg0) {
            arg0.unmap();
        },
        __wbg_value_49f783bb59765962: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_width_c8740d5bdf596189: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbg_writeBuffer_8b5bd251a89198bc: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.writeBuffer(arg1, arg2, getArrayU8FromWasm0(arg3, arg4), arg5, arg6);
        }, arguments); },
        __wbg_write_4dde130ecd70a0b5: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.write(getArrayU8FromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 1055, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h8803f8c799f93ab4);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUDevice")], shim_idx: 470, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("any")], shim_idx: 470, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283_2);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("undefined")], shim_idx: 470, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283_3);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000008: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000009: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./qualia_core_db_bg.js": import0,
    };
}

function wasm_bindgen__convert__closures_____invoke__h8803f8c799f93ab4(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h8803f8c799f93ab4(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283_2(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283_2(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283_3(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h15d54f7d5fe0b283_3(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h243b5e59773a58aa(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h243b5e59773a58aa(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_GpuAddressMode = ["clamp-to-edge", "repeat", "mirror-repeat"];


const __wbindgen_enum_GpuAutoLayoutMode = ["auto"];


const __wbindgen_enum_GpuBlendFactor = ["zero", "one", "src", "one-minus-src", "src-alpha", "one-minus-src-alpha", "dst", "one-minus-dst", "dst-alpha", "one-minus-dst-alpha", "src-alpha-saturated", "constant", "one-minus-constant", "src1", "one-minus-src1", "src1-alpha", "one-minus-src1-alpha"];


const __wbindgen_enum_GpuBlendOperation = ["add", "subtract", "reverse-subtract", "min", "max"];


const __wbindgen_enum_GpuBufferBindingType = ["uniform", "storage", "read-only-storage"];


const __wbindgen_enum_GpuCanvasAlphaMode = ["opaque", "premultiplied"];


const __wbindgen_enum_GpuCanvasToneMappingMode = ["standard", "extended"];


const __wbindgen_enum_GpuCompareFunction = ["never", "less", "equal", "less-equal", "greater", "not-equal", "greater-equal", "always"];


const __wbindgen_enum_GpuCullMode = ["none", "front", "back"];


const __wbindgen_enum_GpuErrorFilter = ["validation", "out-of-memory", "internal"];


const __wbindgen_enum_GpuFilterMode = ["nearest", "linear"];


const __wbindgen_enum_GpuFrontFace = ["ccw", "cw"];


const __wbindgen_enum_GpuIndexFormat = ["uint16", "uint32"];


const __wbindgen_enum_GpuLoadOp = ["load", "clear"];


const __wbindgen_enum_GpuMipmapFilterMode = ["nearest", "linear"];


const __wbindgen_enum_GpuPowerPreference = ["low-power", "high-performance"];


const __wbindgen_enum_GpuPrimitiveTopology = ["point-list", "line-list", "line-strip", "triangle-list", "triangle-strip"];


const __wbindgen_enum_GpuSamplerBindingType = ["filtering", "non-filtering", "comparison"];


const __wbindgen_enum_GpuStencilOperation = ["keep", "zero", "replace", "invert", "increment-clamp", "decrement-clamp", "increment-wrap", "decrement-wrap"];


const __wbindgen_enum_GpuStorageTextureAccess = ["write-only", "read-only", "read-write"];


const __wbindgen_enum_GpuStoreOp = ["store", "discard"];


const __wbindgen_enum_GpuTextureAspect = ["all", "stencil-only", "depth-only"];


const __wbindgen_enum_GpuTextureDimension = ["1d", "2d", "3d"];


const __wbindgen_enum_GpuTextureFormat = ["r8unorm", "r8snorm", "r8uint", "r8sint", "r16unorm", "r16snorm", "r16uint", "r16sint", "r16float", "rg8unorm", "rg8snorm", "rg8uint", "rg8sint", "r32uint", "r32sint", "r32float", "rg16unorm", "rg16snorm", "rg16uint", "rg16sint", "rg16float", "rgba8unorm", "rgba8unorm-srgb", "rgba8snorm", "rgba8uint", "rgba8sint", "bgra8unorm", "bgra8unorm-srgb", "rgb9e5ufloat", "rgb10a2uint", "rgb10a2unorm", "rg11b10ufloat", "rg32uint", "rg32sint", "rg32float", "rgba16unorm", "rgba16snorm", "rgba16uint", "rgba16sint", "rgba16float", "rgba32uint", "rgba32sint", "rgba32float", "stencil8", "depth16unorm", "depth24plus", "depth24plus-stencil8", "depth32float", "depth32float-stencil8", "bc1-rgba-unorm", "bc1-rgba-unorm-srgb", "bc2-rgba-unorm", "bc2-rgba-unorm-srgb", "bc3-rgba-unorm", "bc3-rgba-unorm-srgb", "bc4-r-unorm", "bc4-r-snorm", "bc5-rg-unorm", "bc5-rg-snorm", "bc6h-rgb-ufloat", "bc6h-rgb-float", "bc7-rgba-unorm", "bc7-rgba-unorm-srgb", "etc2-rgb8unorm", "etc2-rgb8unorm-srgb", "etc2-rgb8a1unorm", "etc2-rgb8a1unorm-srgb", "etc2-rgba8unorm", "etc2-rgba8unorm-srgb", "eac-r11unorm", "eac-r11snorm", "eac-rg11unorm", "eac-rg11snorm", "astc-4x4-unorm", "astc-4x4-unorm-srgb", "astc-5x4-unorm", "astc-5x4-unorm-srgb", "astc-5x5-unorm", "astc-5x5-unorm-srgb", "astc-6x5-unorm", "astc-6x5-unorm-srgb", "astc-6x6-unorm", "astc-6x6-unorm-srgb", "astc-8x5-unorm", "astc-8x5-unorm-srgb", "astc-8x6-unorm", "astc-8x6-unorm-srgb", "astc-8x8-unorm", "astc-8x8-unorm-srgb", "astc-10x5-unorm", "astc-10x5-unorm-srgb", "astc-10x6-unorm", "astc-10x6-unorm-srgb", "astc-10x8-unorm", "astc-10x8-unorm-srgb", "astc-10x10-unorm", "astc-10x10-unorm-srgb", "astc-12x10-unorm", "astc-12x10-unorm-srgb", "astc-12x12-unorm", "astc-12x12-unorm-srgb"];


const __wbindgen_enum_GpuTextureSampleType = ["float", "unfilterable-float", "depth", "sint", "uint"];


const __wbindgen_enum_GpuTextureViewDimension = ["1d", "2d", "2d-array", "cube", "cube-array", "3d"];


const __wbindgen_enum_GpuVertexFormat = ["uint8", "uint8x2", "uint8x4", "sint8", "sint8x2", "sint8x4", "unorm8", "unorm8x2", "unorm8x4", "snorm8", "snorm8x2", "snorm8x4", "uint16", "uint16x2", "uint16x4", "sint16", "sint16x2", "sint16x4", "unorm16", "unorm16x2", "unorm16x4", "snorm16", "snorm16x2", "snorm16x4", "float16", "float16x2", "float16x4", "float32", "float32x2", "float32x3", "float32x4", "uint32", "uint32x2", "uint32x3", "uint32x4", "sint32", "sint32x2", "sint32x3", "sint32x4", "unorm10-10-10-2", "unorm8x4-bgra"];


const __wbindgen_enum_GpuVertexStepMode = ["vertex", "instance"];
const FederatedNodeManagerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_federatednodemanager_free(ptr, 1));
const QualiaPortalFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_qualiaportal_free(ptr, 1));
const WasmOffloadIntentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmoffloadintent_free(ptr, 1));
const WebEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_webengine_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayJsValueViewFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    return result;
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('qualia_core_db_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
