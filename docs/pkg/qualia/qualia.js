/* @ts-self-types="./qualia.d.ts" */

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
    /**
     * Creates the node manager.
     */
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
 * Browser canvas HUD. Place its transparent canvas above the Portal canvas.
 * Coordinates are in canvas pixels, independent of CSS scaling.
 */
export class QualiaHud {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        QualiaHudFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_qualiahud_free(ptr, 0);
    }
    /**
     * Move focus between enabled buttons and return the focused action ID.
     * @param {boolean} reverse
     * @returns {string}
     */
    focus_next(reverse) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.qualiahud_focus_next(this.__wbg_ptr, reverse);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Focused HUD action.
     * @returns {string}
     */
    focused_action() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.qualiahud_focused_action(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Return an action ID for a button under the pointer, in canvas pixels.
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    hit_test(x, y) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.qualiahud_hit_test(this.__wbg_ptr, x, y);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} viewport_w
     * @param {number} viewport_h
     */
    layout_items_flex(viewport_w, viewport_h) {
        wasm.qualiahud_layout_items_flex(this.__wbg_ptr, viewport_w, viewport_h);
    }
    /**
     * Creates `QualiaHud` with the given state.
     * @param {HTMLCanvasElement} canvas
     */
    constructor(canvas) {
        const ret = wasm.qualiahud_new(canvas);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        QualiaHudFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Repaint the current document. Uses no Rust heap allocations.
     */
    paint() {
        wasm.qualiahud_paint(this.__wbg_ptr);
    }
    /**
     * Sets `camera_near_plane`.
     * @param {number} near
     */
    set_camera_near_plane(near) {
        const ret = wasm.qualiahud_set_camera_near_plane(this.__wbg_ptr, near);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Use the same orbit camera as Portal to project world markers into HUD pixels.
     * @param {number} yaw
     * @param {number} pitch
     * @param {number} zoom
     * @param {number} x
     * @param {number} y
     * @param {number} z
     */
    set_camera_target(yaw, pitch, zoom, x, y, z) {
        wasm.qualiahud_set_camera_target(this.__wbg_ptr, yaw, pitch, zoom, x, y, z);
    }
    /**
     * Solves flex layout for all HUD items using the Etalon 2D vector HUD layout engine.
     * Updates position and dimensions for auto-sized items (`w == 0.0` or `h == 0.0`).
     * Replace the presentation document directly from binary W3C CBOR-LD (`0xD9 0x0501`).
     * @param {Uint8Array | number[]} bytes
     */
    set_document_cbor_ld(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiahud_set_document_cbor_ld(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Replace the presentation document. Bounds and size are checked before adoption.
     * @param {string} json
     */
    set_document_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiahud_set_document_json(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) QualiaHud.prototype[Symbol.dispose] = QualiaHud.prototype.free;

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
     * Whether the U3 AcousticPlane is effectively on — user flag AND
     * the operational-mode gate (off in Reserve).
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
     * Ambient light intensity.
     * @returns {number}
     */
    ambient_intensity() {
        const ret = wasm.qualiaportal_ambient_intensity(this.__wbg_ptr);
        return ret;
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
        const val2 = +ax;
        const val3 = +ay;
        const val4 = +az;
        const val5 = +rate;
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.qualiaportal_animate_artefact(this.__wbg_ptr, ptr0, len0, val2, val3, val4, val5);
    }
    /**
     * Serializes current animation state to JSON.
     * @returns {string}
     */
    animation_snapshot_json() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.qualiaportal_animation_snapshot_json(this.__wbg_ptr);
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
     * Cold-path Scene / Anatomy lifecycle receipt. Success requires a retained upload
     * and at least one presented renderer frame.
     * @returns {any}
     */
    body_render_receipt() {
        const ret = wasm.qualiaportal_body_render_receipt(this.__wbg_ptr);
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
     * Camera orbit pitch (radians).
     * @returns {number}
     */
    camera_pitch() {
        const ret = wasm.qualiaportal_camera_pitch(this.__wbg_ptr);
        return ret;
    }
    /**
     * X of the camera look-at target.
     * @returns {number}
     */
    camera_target_x() {
        const ret = wasm.qualiaportal_camera_target_x(this.__wbg_ptr);
        return ret;
    }
    /**
     * Y of the camera look-at target.
     * @returns {number}
     */
    camera_target_y() {
        const ret = wasm.qualiaportal_camera_target_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * Z of the camera look-at target.
     * @returns {number}
     */
    camera_target_z() {
        const ret = wasm.qualiaportal_camera_target_z(this.__wbg_ptr);
        return ret;
    }
    /**
     * Camera orbit yaw (radians).
     * @returns {number}
     */
    camera_yaw() {
        const ret = wasm.qualiaportal_camera_yaw(this.__wbg_ptr);
        return ret;
    }
    /**
     * Camera zoom (eye distance in world units).
     * @returns {number}
     */
    camera_zoom() {
        const ret = wasm.qualiaportal_camera_zoom(this.__wbg_ptr);
        return ret;
    }
    /**
     * Clears scene activities idle since `idle`.
     * @param {bigint} idle
     */
    clear_scene_activities(idle) {
        const ret = wasm.qualiaportal_clear_scene_activities(this.__wbg_ptr, idle);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
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
     * Compact ABI-v2 records by stable visible indices into caller-owned bytes, preserving IDs.
     * @param {Uint8Array | number[]} source_records
     * @param {Uint32Array | number[]} visible_indices
     * @param {Uint8Array} compacted_output
     * @returns {number}
     */
    compact_mesh_instance_records(source_records, visible_indices, compacted_output) {
        let ptr0, len0, ptr1, len1, ptr2, len2;
        try {
            ptr0 = passArray8ToWasm0(source_records, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passArray32ToWasm0(visible_indices, wasm.__wbindgen_malloc);
            len1 = WASM_VECTOR_LEN;
            ptr2 = passArray8ToWasm0(compacted_output, wasm.__wbindgen_malloc);
            len2 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr2) { wasm.__wbindgen_free(ptr2, len2 * 1, 1); }
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.qualiaportal_compact_mesh_instance_records(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, compacted_output);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Complete a host recovery attempt. The host reports `webgpu`, `webgl2`,
     * or `canvas2d`; recovered GPU backends re-upload the retained reduced
     * mesh and full semantic tensor on the next frame.
     * @param {string} backend
     */
    complete_graphics_recovery(backend) {
        const ptr0 = passStringToWasm0(backend, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.qualiaportal_complete_graphics_recovery(this.__wbg_ptr, ptr0, len0);
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
     * Select visible source instances without allocating. Bounds use three floats per record,
     * transforms use 16 column-major floats, and the view-projection matrix uses 16 floats.
     * Invalid bounds fail open; returned indices preserve source order.
     * @param {Float32Array | number[]} bounds_min
     * @param {Float32Array | number[]} bounds_max
     * @param {Float32Array | number[]} transforms_column_major
     * @param {Float32Array | number[]} view_projection_column_major
     * @param {Uint32Array} visible_indices
     * @returns {number}
     */
    cull_mesh_instance_indices(bounds_min, bounds_max, transforms_column_major, view_projection_column_major, visible_indices) {
        let ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4;
        try {
            ptr0 = passArrayF32ToWasm0(bounds_min, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passArrayF32ToWasm0(bounds_max, wasm.__wbindgen_malloc);
            len1 = WASM_VECTOR_LEN;
            ptr2 = passArrayF32ToWasm0(transforms_column_major, wasm.__wbindgen_malloc);
            len2 = WASM_VECTOR_LEN;
            ptr3 = passArrayF32ToWasm0(view_projection_column_major, wasm.__wbindgen_malloc);
            len3 = WASM_VECTOR_LEN;
            ptr4 = passArray32ToWasm0(visible_indices, wasm.__wbindgen_malloc);
            len4 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr4) { wasm.__wbindgen_free(ptr4, len4 * 4, 4); }
            if (ptr3) { wasm.__wbindgen_free(ptr3, len3 * 4, 4); }
            if (ptr2) { wasm.__wbindgen_free(ptr2, len2 * 4, 4); }
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
            throw e;
        }
        const ret = wasm.qualiaportal_cull_mesh_instance_indices(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, visible_indices);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
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
     * Field residency: 0 empty, 1 pending WebGPU, 2 uploaded, 3 unavailable.
     * Upload is not a claim that a browser frame has presented field pixels.
     * @returns {number}
     */
    emf_display_state() {
        const ret = wasm.qualiaportal_emf_display_state(this.__wbg_ptr);
        return ret;
    }
    /**
     * Spatially encode a JSON scene, upload the resulting tensor
     * buffer, and return the parsed scene record.
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
     * Standpoint epistemic-quality scalar ∈ [0, 1].
     * @returns {number}
     */
    epistemic_q() {
        const ret = wasm.qualiaportal_epistemic_q(this.__wbg_ptr);
        return ret;
    }
    /**
     * Dedicated GPU worker recovery session generation.
     * @returns {number}
     */
    etalon_session_generation() {
        const ret = wasm.qualiaportal_etalon_session_generation(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Operational transport mode for dedicated WebWorker IPC.
     * @returns {string}
     */
    etalon_transport_mode() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.qualiaportal_etalon_transport_mode(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Dedicated GPU worker steward lifecycle state (0=Initializing, 1=Ready, 2=Running, 3=Suspended, 4=Terminated).
     * @returns {number}
     */
    etalon_worker_state() {
        const ret = wasm.qualiaportal_etalon_worker_state(this.__wbg_ptr);
        return ret;
    }
    /**
     * Configured compensation in stops (EV), including while GPU rendering is unavailable.
     * @returns {number}
     */
    exposure_compensation() {
        const ret = wasm.qualiaportal_exposure_compensation(this.__wbg_ptr);
        return ret;
    }
    /**
     * True while an active WebGPU or WebGL2 path applies scene exposure and the SDR transform.
     * @returns {boolean}
     */
    exposure_transform_available() {
        const ret = wasm.qualiaportal_exposure_transform_available(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * The last hardware backend refused even reduced geometry. The host must
     * replace the context-bound canvas and settle on Canvas2D for this scene.
     * @returns {boolean}
     */
    graphics_recovery_canvas_only() {
        const ret = wasm.qualiaportal_graphics_recovery_canvas_only(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * True when the browser host must replace the context-bound canvas and
     * recreate a renderer before ticking this portal again.
     * @returns {boolean}
     */
    graphics_recovery_requested() {
        const ret = wasm.qualiaportal_graphics_recovery_requested(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * True only while the HDR bloom composite path is active.
     * @returns {boolean}
     */
    hdr_exposure_available() {
        const ret = wasm.qualiaportal_hdr_exposure_available(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Compatibility alias for hosts using the original HDR-specific API name.
     * @returns {number}
     */
    hdr_exposure_compensation() {
        const ret = wasm.qualiaportal_hdr_exposure_compensation(this.__wbg_ptr);
        return ret;
    }
    /**
     * True when the active WebGPU or WebGL2 scene target preserves radiance above SDR white.
     * @returns {boolean}
     */
    hdr_scene_available() {
        const ret = wasm.qualiaportal_hdr_scene_available(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Screen-space name chips for in-field labels. Ingest/JS receipt may allocate.
     * @param {number} canvas_w
     * @param {number} canvas_h
     * @returns {any}
     */
    holograph_label_overlays(canvas_w, canvas_h) {
        const ret = wasm.qualiaportal_holograph_label_overlays(this.__wbg_ptr, canvas_w, canvas_h);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Inspect physical interpretation and original PCM without applying a
     * display or listening conversion. Exact clock integers remain in metadata_json.
     * @param {Uint8Array | number[]} bytes
     * @returns {any}
     */
    inspect_emf(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_inspect_emf(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Drop accumulated temporal history (e.g. after a camera cut or
     * scene swap) so the next frame isn't ghosted.
     */
    invalidate_temporal_history() {
        wasm.qualiaportal_invalidate_temporal_history(this.__wbg_ptr);
    }
    /**
     * The most recently parsed scene payload, if any.
     * @returns {any | undefined}
     */
    last_parsed() {
        const ret = wasm.qualiaportal_last_parsed(this.__wbg_ptr);
        return ret;
    }
    /**
     * P9.2 — Load a `.10d` container asset: parse the section table, extract
     * the QuantizedMesh, optional SurfaceReading (per-vertex colour), optional
     * UV01, optional MAT3/MAT4, Tensor10DNodes (provenance), and mesh frames;
     * upload to the GPU. Texture bytes stay in HMC/deps (`load_texture_encoded`).
     *
     * **Governance fail-closed:** if the header carries
     * `FLAG_DEFAULT_DISPOSITION_REFUSE` (bit 0) and no attestation section is
     * present, the mesh is loaded for display but `description` marks it as
     * governance-refused — not citable as provenance until attested.
     *
     * Returns a JS object `{ vertex_count, triangle_count, provenance_mu, tier }`.
     * @param {Uint8Array | number[]} bytes
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
     * @param {Uint8Array | number[]} bytes
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @returns {any}
     */
    load_10d_colored(bytes, r, g, b, a) {
        const val2 = +r;
        const val3 = +g;
        const val4 = +b;
        const val5 = +a;
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_10d_colored(this.__wbg_ptr, ptr0, len0, val2, val3, val4, val5);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Load a multi-asset scene: `Array<{bytes: .10d, r,g,b,a, id?}>` (QG-20).
     * General scene asset loader for environment/game scenes, town layouts,
     * vehicles, and user props.
     * @param {Array<any>} assets
     * @returns {any}
     */
    load_assets_colored(assets) {
        const ret = wasm.qualiaportal_load_assets_colored(this.__wbg_ptr, assets);
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
     * @param {Uint8Array | number[]} bytes
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
     *
     * Decodes organs **in Rust** from the pack buffer — no per-organ JS `Uint8Array` materialisation.
     * That cut peak heap by ~1–2× pack size and is the phone-safe path.
     * @param {Uint8Array | number[]} bytes
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
     *
     * Prefer [`Self::load_body_from_qualia_bundle_mixed`] for packs — that path never materialises a
     * per-organ JS `Uint8Array` copy (critical on phones).
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
     * Load a sibling `Q42EMF64` field and return its dimensions, integrity,
     * exact interpretation JSON and original PCM tracks. The visible projection
     * is a declared derivative. V3 does not update implicit sigma sonification.
     * `frequency_hz` is null for v3; `kind` is volume when cell_count exceeds one.
     * @param {Uint8Array | number[]} bytes
     * @returns {any}
     */
    load_emf(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_load_emf(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Load one verified `.10d` mesh from an HMC bundle and retain its digest-backed
     * residency identity. Replaying the same asset returns the existing receipt without
     * rebuilding the GPU mesh; a different digest replaces the resident mesh.
     * @param {Uint8Array | number[]} hmc_bytes
     * @param {string} asset_key
     * @returns {any}
     */
    load_hmc_mesh(hmc_bytes, asset_key) {
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passArray8ToWasm0(hmc_bytes, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passStringToWasm0(asset_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.qualiaportal_load_hmc_mesh(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Parse a JSON scene payload, stash it as `last_parsed`, and
     * return the parsed record.
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
     * Parse a `.q42` (CBOR-LD) payload, stash it as `last_parsed`,
     * and return the parsed record.
     * @param {Uint8Array | number[]} bytes
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
     * Layout an RDF document as a holographic semantic graph and upload it to
     * the active theatre: WebGL2 (RGBA32F, required on mobile) or WebGPU (SSBO).
     *
     * `content_type` is a media type (`text/turtle`) or Qualia format id (`turtle`).
     * Scene construction uses the bundled cinema-graph.vibe (Qualia host Scene.*).
     * Returns `{ node_count, triple_count, truncated, backend, labels, layout }`.
     * @param {Uint8Array | number[]} payload
     * @param {string} content_type
     * @returns {any}
     */
    load_rdf_graph(payload, content_type) {
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passStringToWasm0(content_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.qualiaportal_load_rdf_graph(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * UTF-8 RDF document helper for `load_rdf_graph`.
     * @param {string} payload
     * @param {string} content_type
     * @returns {any}
     */
    load_rdf_graph_text(payload, content_type) {
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passStringToWasm0(content_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.qualiaportal_load_rdf_graph_text(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Hot-edit theatre: evaluate `vibe_src` (`construct_theatre`) then pack GPU nodes.
     * Empty `vibe_src` uses the bundled cinema-graph.vibe.
     * @param {string} payload
     * @param {string} content_type
     * @param {string} vibe_src
     * @returns {any}
     */
    load_rdf_graph_vibe(payload, content_type, vibe_src) {
        let ptr0, len0, ptr1, len1, ptr2, len2;
        try {
            ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passStringToWasm0(content_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len1 = WASM_VECTOR_LEN;
            ptr2 = passStringToWasm0(vibe_src, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len2 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr2) { wasm.__wbindgen_free(ptr2, len2 * 1, 1); }
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.qualiaportal_load_rdf_graph_vibe(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Generic scene asset alias for [`Self::load_assets_colored`] (QG-20).
     * @param {Array<any>} assets
     * @returns {any}
     */
    load_scene_assets(assets) {
        const ret = wasm.qualiaportal_load_scene_assets(this.__wbg_ptr, assets);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Hydrate one content-addressed PNG/JPEG/KTX2 dependency. Role 0 is sRGB
     * base colour, 1 a linear tangent-space normal, 2 linear packed data and
     * 3 sRGB alpha-masked colour. Returns false for an unavailable GPU backend
     * or a KTX2/BasisU payload the adapter cannot transcode (mesh still draws).
     * Call before scene submission so the material bind groups see residency.
     * @param {Uint8Array | number[]} digest
     * @param {string} mime
     * @param {Uint8Array | number[]} encoded
     * @param {number} role
     * @param {number} cutoff
     * @returns {boolean}
     */
    load_texture_encoded(digest, mime, encoded, role, cutoff) {
        const val4 = +role;
        const val5 = +cutoff;
        let ptr0, len0, ptr1, len1, ptr2, len2;
        try {
            ptr0 = passArray8ToWasm0(digest, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passStringToWasm0(mime, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len1 = WASM_VECTOR_LEN;
            ptr2 = passArray8ToWasm0(encoded, wasm.__wbindgen_malloc);
            len2 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr2) { wasm.__wbindgen_free(ptr2, len2 * 1, 1); }
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.qualiaportal_load_texture_encoded(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, val4, val5);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Byte stride of the packed mesh-instance record ABI exposed by `set_mesh_instances`.
     * @returns {number}
     */
    mesh_instance_record_stride() {
        const ret = wasm.qualiaportal_mesh_instance_record_stride(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Version of the packed ABI accepted by `set_mesh_instances`.
     * @returns {number}
     */
    mesh_instance_record_version() {
        const ret = wasm.qualiaportal_mesh_instance_record_version(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Mount a minimal qapp document into the DOM element `root_id`:
     * clears it, emits a header with the description line, and appends
     * the parsed scene (or a "load a scene first" hint).
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
     * Construct the portal on `canvas`: detect the device tier, mint a
     * session nonce + spectator standpoint, sample ambient telemetry,
     * and paint the first frame.
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
     * VRAM-ledger operational mode as its `u8` code
     * (0 = Full, 1 = Eco, 2 = Reserve).
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
     * @param {Uint8Array | number[]} bytes
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
     * Build the packed ABI from caller-owned column-major transforms and low/high ID words.
     * Output must hold `instance_count * mesh_instance_record_stride()` bytes.
     * @param {Float32Array | number[]} transforms_column_major
     * @param {Uint32Array | number[]} semantic_id_words
     * @param {Uint8Array} packed_output
     * @returns {number}
     */
    pack_mesh_instance_records(transforms_column_major, semantic_id_words, packed_output) {
        let ptr0, len0, ptr1, len1, ptr2, len2;
        try {
            ptr0 = passArrayF32ToWasm0(transforms_column_major, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passArray32ToWasm0(semantic_id_words, wasm.__wbindgen_malloc);
            len1 = WASM_VECTOR_LEN;
            ptr2 = passArray8ToWasm0(packed_output, wasm.__wbindgen_malloc);
            len2 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr2) { wasm.__wbindgen_free(ptr2, len2 * 1, 1); }
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
            throw e;
        }
        const ret = wasm.qualiaportal_pack_mesh_instance_records(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, packed_output);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Pick the nearest resource crystal at canvas pixels. `-1` on miss.
     * @param {number} x
     * @param {number} y
     * @param {number} canvas_w
     * @param {number} canvas_h
     * @returns {number}
     */
    pick_holograph_at(x, y, canvas_w, canvas_h) {
        const ret = wasm.qualiaportal_pick_holograph_at(this.__wbg_ptr, x, y, canvas_w, canvas_h);
        return ret;
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
     * A timed presentation cue from an accepted game event. This changes no
     * campaign facts; completed and pending cues survive scene replacement.
     * @param {bigint} token
     * @param {bigint} joint
     * @param {bigint} clip
     * @param {bigint} idle
     * @param {number} duration
     * @param {number} fade
     * @returns {number}
     */
    present_scene_activity(token, joint, clip, idle, duration, fade) {
        const ret = wasm.qualiaportal_present_scene_activity(this.__wbg_ptr, token, joint, clip, idle, duration, fade);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
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
     * Push a raw `u64` sonic token onto the acoustic ring; `false`
     * when the ring is full.
     * @param {bigint} raw
     * @returns {boolean}
     */
    push_sonic_token_raw(raw) {
        const ret = wasm.qualiaportal_push_sonic_token_raw(this.__wbg_ptr, raw);
        return ret !== 0;
    }
    /**
     * Draw EMF slice z=0, t=0 on a WebGPU
     * canvas no larger than 1024². The existing phase/sigma shader is a
     * diagnostic derivative, not a calibrated RGB conversion. The receipt
     * reports submission only; browser canvas readback is a separate check.
     * The amplitude gain is a finite diagnostic display multiplier in (0, 1e6].
     * It updates a shader uniform, never the retained physical cells.
     * @param {number} amplitude_gain
     * @returns {any}
     */
    render_emf_preview(amplitude_gain) {
        const ret = wasm.qualiaportal_render_emf_preview(this.__wbg_ptr, amplitude_gain);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Resize the canvas backing store (and the GPU surface when
     * active), then repaint.
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
     * Restores animation state from `json`.
     * @param {string} json
     */
    restore_animation_snapshot_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_restore_animation_snapshot_json(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Sample browser ambient telemetry (perf/memory probes) as JS.
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
     * Returns an attestation receipt of current animation state.
     * @returns {any}
     */
    scene_animation_receipt() {
        const ret = wasm.qualiaportal_scene_animation_receipt(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Generic scene alias for [`Self::body_render_receipt`] (QG-20).
     * @returns {any}
     */
    scene_render_receipt() {
        const ret = wasm.qualiaportal_scene_render_receipt(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Whether the backend supports the SSAO pass at all.
     * @returns {boolean}
     */
    screen_space_ao_available() {
        const ret = wasm.qualiaportal_screen_space_ao_available(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Whether SSAO is currently enabled on the GPU backend
     * (`false` without an active GPU).
     * @returns {boolean}
     */
    screen_space_ao_enabled() {
        const ret = wasm.qualiaportal_screen_space_ao_enabled(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * SSAO target height (0 without an active GPU pass).
     * @returns {number}
     */
    screen_space_ao_height() {
        const ret = wasm.qualiaportal_screen_space_ao_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * SSAO target width (0 without an active GPU pass).
     * @returns {number}
     */
    screen_space_ao_width() {
        const ret = wasm.qualiaportal_screen_space_ao_width(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Seeks scene animation to `time`.
     * @param {number} time
     */
    seek_scene_animation(time) {
        const ret = wasm.qualiaportal_seek_scene_animation(this.__wbg_ptr, time);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Spotlight a holographic resource (`-1` recedes the query light).
     * @param {number} index
     */
    select_holograph_index(index) {
        wasm.qualiaportal_select_holograph_index(this.__wbg_ptr, index);
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
     * Joint IDs group the pieces of one actor without game-specific naming.
     * @param {bigint} joint_id
     * @param {bigint} clip_id
     * @param {number} fade_seconds
     * @returns {number}
     */
    select_scene_animation_clip(joint_id, clip_id, fade_seconds) {
        const ret = wasm.qualiaportal_select_scene_animation_clip(this.__wbg_ptr, joint_id, clip_id, fade_seconds);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Selected tensor-node index, or `-1` while none/pick pending.
     * @returns {number}
     */
    selected_node_index() {
        const ret = wasm.qualiaportal_selected_node_index(this.__wbg_ptr);
        return ret;
    }
    /**
     * Selected mesh semantic identity as hexadecimal, or an empty string while absent/pending.
     * @returns {string}
     */
    selected_semantic_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.qualiaportal_selected_semantic_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Enable or mute U3 AcousticPlane (automatically off in Reserve mode).
     * @param {boolean} enabled
     */
    set_acoustic_enabled(enabled) {
        wasm.qualiaportal_set_acoustic_enabled(this.__wbg_ptr, enabled);
    }
    /**
     * Enable/disable the **ambient particle field** — the mixer's "ambient" channel. Off by
     * default and opt-in: a plain mesh/anatomy view (or a game scene) keeps it off, and a
     * Tensor10D upload never forces it on. Hosts that want the tensor-node particle view
     * enable it explicitly.
     * @param {boolean} on
     */
    set_ambient_enabled(on) {
        wasm.qualiaportal_set_ambient_enabled(this.__wbg_ptr, on);
    }
    /**
     * Sets reduced-motion accessibility flag.
     * @param {boolean} reduced
     */
    set_animation_reduced_motion(reduced) {
        wasm.qualiaportal_set_animation_reduced_motion(this.__wbg_ptr, reduced);
    }
    /**
     * Toggle auto temporal smoothing; a change invalidates the
     * temporal history so the first frame isn't blended from stale data.
     * @param {boolean} enabled
     */
    set_auto_temporal_enabled(enabled) {
        wasm.qualiaportal_set_auto_temporal_enabled(this.__wbg_ptr, enabled);
    }
    /**
     * Replace the person-authored body fit. Pass JSON matching wellfare `BodyFit`.
     * Applied on the next `load_body_*` upload. Empty / invalid JSON resets to identity.
     * @param {string} json
     */
    set_body_fit_json(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.qualiaportal_set_body_fit_json(this.__wbg_ptr, ptr0, len0);
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
     * Shared native lens state for hardware rendering and CPU picking.
     * @param {number} near
     */
    set_camera_near_plane(near) {
        const ret = wasm.qualiaportal_set_camera_near_plane(this.__wbg_ptr, near);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * RTS camera pan IPC (moves look-at center point in world space).
     * @param {number} target_x
     * @param {number} target_y
     * @param {number} target_z
     */
    set_camera_pan(target_x, target_y, target_z) {
        const ret = wasm.qualiaportal_set_camera_pan(this.__wbg_ptr, target_x, target_y, target_z);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Full camera placement (yaw, pitch, zoom, and world-space target center).
     * @param {number} yaw
     * @param {number} pitch
     * @param {number} zoom
     * @param {number} target_x
     * @param {number} target_y
     * @param {number} target_z
     */
    set_camera_target(yaw, pitch, zoom, target_x, target_y, target_z) {
        const ret = wasm.qualiaportal_set_camera_target(this.__wbg_ptr, yaw, pitch, zoom, target_x, target_y, target_z);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Configure viewport background / sky clear color (r, g, b, a in 0.0..1.0).
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     */
    set_clear_color(r, g, b, a) {
        wasm.qualiaportal_set_clear_color(this.__wbg_ptr, r, g, b, a);
    }
    /**
     * Set the display mode by name (`DisplayMode::from_str`;
     * unrecognised names fall back to its default).
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
     * Upload packed IBL probe coefficients plus a fallback radiance
     * colour; `false`/error when the GPU backend can't take them.
     * @param {Float32Array | number[]} packed_probes
     * @param {Float32Array | number[]} fallback_radiance
     * @returns {boolean}
     */
    set_environment_probe_data(packed_probes, fallback_radiance) {
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passArrayF32ToWasm0(packed_probes, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passArrayF32ToWasm0(fallback_radiance, wasm.__wbindgen_malloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
            throw e;
        }
        const ret = wasm.qualiaportal_set_environment_probe_data(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Configure manual exposure compensation in stops (EV), clamped to -8…+8.
     * The configured value is retained while the GPU output path is unavailable.
     * @param {number} ev
     * @returns {boolean}
     */
    set_exposure_compensation(ev) {
        const ret = wasm.qualiaportal_set_exposure_compensation(this.__wbg_ptr, ev);
        return ret !== 0;
    }
    /**
     * Compatibility alias for hosts using the original HDR-specific API name.
     * @param {number} ev
     * @returns {boolean}
     */
    set_hdr_exposure_compensation(ev) {
        const ret = wasm.qualiaportal_set_hdr_exposure_compensation(this.__wbg_ptr, ev);
        return ret !== 0;
    }
    /**
     * Configure directional sun lighting (direction vector, sun intensity, and ambient intensity).
     * @param {number} sun_x
     * @param {number} sun_y
     * @param {number} sun_z
     * @param {number} sun_intensity
     * @param {number} ambient_intensity
     */
    set_lighting(sun_x, sun_y, sun_z, sun_intensity, ambient_intensity) {
        wasm.qualiaportal_set_lighting(this.__wbg_ptr, sun_x, sun_y, sun_z, sun_intensity, ambient_intensity);
    }
    /**
     * Replace the current WebGPU mesh instance stream from caller-owned packed records.
     * Each 128-byte record is the native `GpuInstanceRecord` ABI: column-major world transform,
     * inverse-transpose normal frame, low/high `u32` semantic identity words, orientation sign,
     * and zero padding. Callers may compact by CPU visibility indices before passing this slice.
     * @param {number} abi_version
     * @param {Uint8Array | number[]} packed_records
     * @returns {number}
     */
    set_mesh_instances(abi_version, packed_records) {
        const val2 = +abi_version;
        const ptr0 = passArray8ToWasm0(packed_records, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_set_mesh_instances(this.__wbg_ptr, val2, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Apply an authored atmospheric sky preset:
     * - 0: Cyber-Dark / Deep Void
     * - 1: Daylight (clear sky, high warm sun, balanced ambient)
     * - 2: Sunset / Golden Hour (warm orange sky, low golden sun)
     * - 3: Night / Moonlight (dark indigo sky, cold moonlight)
     * Keep the next `load_body_*` upload in authored coordinates.
     * Anatomy stays false (orbit frame). A town scene sets this so the
     * camera and the mesh share one frame.
     * @param {boolean} on
     */
    set_preserve_authored_frame(on) {
        wasm.qualiaportal_set_preserve_authored_frame(this.__wbg_ptr, on);
    }
    /**
     * Tune SSAO kernel parameters (world-space radius, strength,
     * depth bias); forwarded to the GPU when active.
     * @param {number} radius
     * @param {number} strength
     * @param {number} bias
     */
    set_screen_space_ao(radius, strength, bias) {
        wasm.qualiaportal_set_screen_space_ao(this.__wbg_ptr, radius, strength, bias);
    }
    /**
     * Enable the budgeted half-resolution screen-space AO pass when available.
     * @param {boolean} enabled
     */
    set_screen_space_ao_enabled(enabled) {
        wasm.qualiaportal_set_screen_space_ao_enabled(this.__wbg_ptr, enabled);
    }
    /**
     * Set the SSAO per-pixel sample count.
     * @param {number} samples
     */
    set_screen_space_ao_sample_count(samples) {
        wasm.qualiaportal_set_screen_space_ao_sample_count(this.__wbg_ptr, samples);
    }
    /**
     * Toggle the shadow pass on the GPU renderer when present.
     * @param {boolean} enabled
     */
    set_shadows_enabled(enabled) {
        wasm.qualiaportal_set_shadows_enabled(this.__wbg_ptr, enabled);
    }
    /**
     * Select an authored sky preset index
     * (0 = deep void, 1 = daylight, 2 = sunset, 3 = moonlight).
     * @param {number} preset
     */
    set_sky_preset(preset) {
        wasm.qualiaportal_set_sky_preset(this.__wbg_ptr, preset);
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
        const val2 = +standpoint_class;
        const val3 = +epistemic_q;
        const val4 = +t_slice;
        const val5 = +t_window;
        const ptr0 = passStringToWasm0(identifier_did, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_set_standpoint(this.__wbg_ptr, val2, val3, val4, val5, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Apply a flat `f32` telemetry sample block to the HUD model.
     * @param {Float32Array | number[]} floats
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
     * Replace the standpoint temporal slice/window and push it to the
     * GPU when active.
     * @param {number} t_slice
     * @param {number} t_window
     */
    set_temporal_slice(t_slice, t_window) {
        wasm.qualiaportal_set_temporal_slice(this.__wbg_ptr, t_slice, t_window);
    }
    /**
     * Show or hide semantic tensor nodes while retaining uploaded data for picking.
     * @param {boolean} on
     */
    set_tensor_projection_enabled(on) {
        wasm.qualiaportal_set_tensor_projection_enabled(this.__wbg_ptr, on);
    }
    /**
     * Replace the water surface mesh using world-space XYZ triples and triangle indices.
     * @param {Float32Array | number[]} positions_xyz
     * @param {Uint32Array | number[]} indices
     * @returns {boolean}
     */
    set_water_geometry(positions_xyz, indices) {
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passArrayF32ToWasm0(positions_xyz, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passArray32ToWasm0(indices, wasm.__wbindgen_malloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
            throw e;
        }
        const ret = wasm.qualiaportal_set_water_geometry(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Set water-surface appearance — RGBA colour, wave amplitude and
     * frequency, and mesh `quality` tier.
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} alpha
     * @param {number} wave_amplitude
     * @param {number} wave_frequency
     * @param {number} quality
     */
    set_water_surface(r, g, b, alpha, wave_amplitude, wave_frequency, quality) {
        wasm.qualiaportal_set_water_surface(this.__wbg_ptr, r, g, b, alpha, wave_amplitude, wave_frequency, quality);
    }
    /**
     * Set linear-light temperature/tint biases in stops, each bounded to -1…+1.
     * Temperature warms red while cooling blue; positive tint favours green.
     * The value is retained and replayed when a renderer is initialized or recovered.
     * @param {number} temperature_ev
     * @param {number} tint_ev
     * @returns {boolean}
     */
    set_white_balance(temperature_ev, tint_ev) {
        const ret = wasm.qualiaportal_set_white_balance(this.__wbg_ptr, temperature_ev, tint_ev);
        return ret !== 0;
    }
    /**
     * Whether the active backend supports the shadow pass.
     * @returns {boolean}
     */
    shadows_available() {
        const ret = wasm.qualiaportal_shadows_available(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Number of sonic tokens currently queued in the SPSC ring.
     * @returns {number}
     */
    sonic_token_pending() {
        const ret = wasm.qualiaportal_sonic_token_pending(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Spatially encode a JSON scene without uploading —
     * [`spatial_encode_wasm`] passthrough.
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
     * Current standpoint class (observer role code).
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
     * Directional (sun) light intensity.
     * @returns {number}
     */
    sun_intensity() {
        const ret = wasm.qualiaportal_sun_intensity(this.__wbg_ptr);
        return ret;
    }
    /**
     * Standpoint temporal-slice position ∈ [0, 1].
     * @returns {number}
     */
    t_slice() {
        const ret = wasm.qualiaportal_t_slice(this.__wbg_ptr);
        return ret;
    }
    /**
     * Standpoint temporal-window width ∈ (0, 1].
     * @returns {number}
     */
    t_window() {
        const ret = wasm.qualiaportal_t_window(this.__wbg_ptr);
        return ret;
    }
    /**
     * Temporal-graphics execution receipt for honesty reporting.
     * @returns {any}
     */
    temporal_graphics_receipt() {
        const ret = wasm.qualiaportal_temporal_graphics_receipt(this.__wbg_ptr);
        return ret;
    }
    /**
     * Per-frame advance: device-loss check, scene animation, ICP
     * control drain (≤16), time/telemetry/acoustic updates, camera-fly
     * interpolation, then paint.
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
     * Device tier — 0 = Canvas2D, 1 = WebGPU-capable, 2 = phenomenal
     * viewport (tensor upload confirmed).
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
     * @param {Uint8Array | number[]} bytes
     * @param {string} hint
     * @returns {number}
     */
    upload_mesh_asset(bytes, hint) {
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passStringToWasm0(hint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.qualiaportal_upload_mesh_asset(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Upload a packed tensor buffer: stores it, feeds the resident
     * substrate and VRAM ledger, forwards to the GPU (promoting the
     * tier to 2 on success), and updates the description line.
     * @param {Uint8Array | number[]} bytes
     */
    upload_tensor_buffer(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.qualiaportal_upload_tensor_buffer(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Configured temperature/tint bias in stops, including while graphics is unavailable.
     * @returns {Float32Array}
     */
    white_balance() {
        const ret = wasm.qualiaportal_white_balance(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Write posed XYZ triples into an authored scene span. Compact backends
     * receive the replay mesh's mapped positions, never original offsets.
     * @param {number} start
     * @param {Float32Array | number[]} xyz
     */
    write_part_vertices(start, xyz) {
        const val2 = +start;
        const ptr0 = passArrayF32ToWasm0(xyz, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.qualiaportal_write_part_vertices(this.__wbg_ptr, val2, ptr0, len0);
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
     * Creates `WasmOffloadIntent` with the given state.
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
     * Builder: `string-payload`.
     * @param {number} opcode
     * @param {number} priority
     * @param {string} payload
     * @returns {WasmOffloadIntent}
     */
    static with_string_payload(opcode, priority, payload) {
        const val2 = +opcode;
        const val3 = +priority;
        const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmoffloadintent_with_string_payload(val2, val3, ptr0, len0);
        return WasmOffloadIntent.__wrap(ret);
    }
}
if (Symbol.dispose) WasmOffloadIntent.prototype[Symbol.dispose] = WasmOffloadIntent.prototype.free;

export class WasmQ42Session {
    static __wrap(ptr) {
        const obj = Object.create(WasmQ42Session.prototype);
        obj.__wbg_ptr = ptr;
        WasmQ42SessionFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmQ42SessionFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmq42session_free(ptr, 0);
    }
    /**
     * Number of quins in the session's active state.
     * @returns {number}
     */
    active_quin_count() {
        const ret = wasm.wasmq42session_active_quin_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Advances the session clock to `tick` (Lamport-style; must be monotone).
     * @param {bigint} tick
     */
    advance_tick(tick) {
        wasm.wasmq42session_advance_tick(this.__wbg_ptr, tick);
    }
    /**
     * Commits staged mutations to the journal under `command_hash`/`actor_did`
     * provenance; returns the committed tick/sequence.
     * @param {bigint} command_hash
     * @param {bigint} actor_did
     * @returns {number}
     */
    commit_transaction(command_hash, actor_did) {
        const ret = wasm.wasmq42session_commit_transaction(this.__wbg_ptr, command_hash, actor_did);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * The session's current logical tick.
     * @returns {bigint}
     */
    current_tick() {
        const ret = wasm.wasmq42session_current_tick(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Export all currently active Quins as a flat contiguous 48-byte buffer.
     * @returns {Uint8Array}
     */
    export_active_quins_bytes() {
        const ret = wasm.wasmq42session_export_active_quins_bytes(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Exports the underlying journal's raw bytes (durable serialization).
     * @returns {Uint8Array}
     */
    export_journal_bytes() {
        const ret = wasm.wasmq42session_export_journal_bytes(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Load an existing durable Q42 session from journal bytes and optional base Quin snapshot bytes.
     * @param {Uint8Array | number[]} journal_bytes
     * @param {Uint8Array | number[]} base_quins_bytes
     * @returns {WasmQ42Session}
     */
    static load_from_bytes(journal_bytes, base_quins_bytes) {
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passArray8ToWasm0(journal_bytes, wasm.__wbindgen_malloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passArray8ToWasm0(base_quins_bytes, wasm.__wbindgen_malloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.wasmq42session_load_from_bytes(ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmQ42Session.__wrap(ret[0]);
    }
    /**
     * Creates a new durable Q42 session for `world_did_hash` over an
     * in-memory journal; `base_digest_bytes` supplies the 32-byte base digest
     * (shorter inputs error).
     * @param {bigint} world_did_hash
     * @param {Uint8Array | number[]} base_digest_bytes
     */
    constructor(world_did_hash, base_digest_bytes) {
        const val2 = BigInt.asIntN(64, world_did_hash);
        const ptr0 = passArray8ToWasm0(base_digest_bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmq42session_new(val2, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmQ42SessionFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Check if a Quin matching the given pattern exists in the active session (0 = wildcard).
     * @param {bigint} s
     * @param {bigint} p
     * @param {bigint} o
     * @returns {boolean}
     */
    query_has_quin(s, p, o) {
        const ret = wasm.wasmq42session_query_has_quin(this.__wbg_ptr, s, p, o);
        return ret !== 0;
    }
    /**
     * Query the object value for a given (subject, predicate) pair.
     * @param {bigint} s
     * @param {bigint} p
     * @returns {bigint | undefined}
     */
    query_object_for_predicate(s, p) {
        const ret = wasm.wasmq42session_query_object_for_predicate(this.__wbg_ptr, s, p);
        return ret[0] === 0 ? undefined : BigInt.asUintN(64, ret[1]);
    }
    /**
     * Rewind session state to a specific simulation/historical tick.
     * @param {Uint8Array | number[]} base_quins_bytes
     * @param {bigint} target_tick
     */
    rewind_to_tick(base_quins_bytes, target_tick) {
        const val2 = BigInt.asIntN(64, target_tick);
        const ptr0 = passArray8ToWasm0(base_quins_bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmq42session_rewind_to_tick(this.__wbg_ptr, ptr0, len0, val2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Discards all staged adds/removes in the open transaction.
     */
    rollback_staged() {
        wasm.wasmq42session_rollback_staged(this.__wbg_ptr);
    }
    /**
     * Stages an add of the quin built from `(s,p,o,c,m)` — parity is computed
     * as the XOR fold — into the current transaction.
     * @param {bigint} s
     * @param {bigint} p
     * @param {bigint} o
     * @param {bigint} c
     * @param {bigint} m
     */
    stage_add_quin(s, p, o, c, m) {
        wasm.wasmq42session_stage_add_quin(this.__wbg_ptr, s, p, o, c, m);
    }
    /**
     * Stages a remove of the quin built from `(s,p,o,c,m)` (XOR-fold parity
     * computed) in the current transaction.
     * @param {bigint} s
     * @param {bigint} p
     * @param {bigint} o
     * @param {bigint} c
     * @param {bigint} m
     */
    stage_remove_quin(s, p, o, c, m) {
        wasm.wasmq42session_stage_remove_quin(this.__wbg_ptr, s, p, o, c, m);
    }
}
if (Symbol.dispose) WasmQ42Session.prototype[Symbol.dispose] = WasmQ42Session.prototype.free;

export class WasmSimulationWorld {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSimulationWorldFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsimulationworld_free(ptr, 0);
    }
    /**
     * Compute group formation destinations for `count` agents (QG-11).
     * Returns interleaved destination coordinates: [x0, y0, x1, y1, ...].
     * @param {number} target_x
     * @param {number} target_y
     * @param {number} count
     * @param {Uint8Array | number[]} obstacles
     * @param {number} width
     * @param {number} height
     * @returns {Int32Array}
     */
    compute_group_formation(target_x, target_y, count, obstacles, width, height) {
        const val2 = +target_x;
        const val3 = +target_y;
        const val4 = +count;
        const val5 = +width;
        const val6 = +height;
        const ptr0 = passArray8ToWasm0(obstacles, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsimulationworld_compute_group_formation(this.__wbg_ptr, val2, val3, val4, ptr0, len0, val5, val6);
        var v7 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v7;
    }
    /**
     * Deterministic hash of the world state (for replay verification).
     * @returns {bigint}
     */
    compute_state_hash() {
        const ret = wasm.wasmsimulationworld_compute_state_hash(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * The current simulation tick.
     * @returns {bigint}
     */
    current_tick() {
        const ret = wasm.wasmsimulationworld_current_tick(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Deterministic 2D grid pathfinder (QG-11).
     * Returns interleaved waypoint coordinates: [x0, y0, x1, y1, ...].
     * @param {number} start_x
     * @param {number} start_y
     * @param {number} target_x
     * @param {number} target_y
     * @param {Uint8Array | number[]} obstacles
     * @param {number} width
     * @param {number} height
     * @returns {Int32Array}
     */
    find_path_grid(start_x, start_y, target_x, target_y, obstacles, width, height) {
        const val2 = +start_x;
        const val3 = +start_y;
        const val4 = +target_x;
        const val5 = +target_y;
        const val6 = +width;
        const val7 = +height;
        const ptr0 = passArray8ToWasm0(obstacles, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsimulationworld_find_path_grid(this.__wbg_ptr, val2, val3, val4, val5, ptr0, len0, val6, val7);
        var v8 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v8;
    }
    /**
     * X position of `entity_id` in millimetres, or `None` if unregistered.
     * @param {bigint} entity_id
     * @returns {bigint | undefined}
     */
    get_agent_pos_x_mm(entity_id) {
        const ret = wasm.wasmsimulationworld_get_agent_pos_x_mm(this.__wbg_ptr, entity_id);
        return ret[0] === 0 ? undefined : ret[1];
    }
    /**
     * Y position of `entity_id` in millimetres, or `None` if unregistered.
     * @param {bigint} entity_id
     * @returns {bigint | undefined}
     */
    get_agent_pos_y_mm(entity_id) {
        const ret = wasm.wasmsimulationworld_get_agent_pos_y_mm(this.__wbg_ptr, entity_id);
        return ret[0] === 0 ? undefined : ret[1];
    }
    /**
     * Creates a deterministic fixed-tick simulation world with RNG `seed`.
     * @param {bigint} seed
     */
    constructor(seed) {
        const ret = wasm.wasmsimulationworld_new(seed);
        this.__wbg_ptr = ret;
        WasmSimulationWorldFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Spatial box query returning matching entity IDs as strings (for u64 JS precision).
     * @param {bigint} min_x_mm
     * @param {bigint} min_y_mm
     * @param {bigint} max_x_mm
     * @param {bigint} max_y_mm
     * @returns {string[]}
     */
    query_agents_in_bounds(min_x_mm, min_y_mm, max_x_mm, max_y_mm) {
        const ret = wasm.wasmsimulationworld_query_agents_in_bounds(this.__wbg_ptr, min_x_mm, min_y_mm, max_x_mm, max_y_mm);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]);
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Registers `entity_id` at position `(x_mm, y_mm)`; `false` if the entity
     * table is full.
     * @param {bigint} entity_id
     * @param {bigint} x_mm
     * @param {bigint} y_mm
     * @returns {boolean}
     */
    register_agent(entity_id, x_mm, y_mm) {
        const ret = wasm.wasmsimulationworld_register_agent(this.__wbg_ptr, entity_id, x_mm, y_mm);
        return ret !== 0;
    }
    /**
     * Advances the simulation one tick, draining the command queue; returns
     * the number of receipts produced.
     * @returns {number}
     */
    step_tick() {
        const ret = wasm.wasmsimulationworld_step_tick(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Queues a `SimulationCommand` for `target_tick`; rejected commands
     * return a JS error describing the rejection reason.
     * @param {bigint} target_tick
     * @param {number} sequence
     * @param {bigint} actor_did
     * @param {number} command_opcode
     * @param {bigint} target_entity
     * @param {bigint} arg0
     * @param {bigint} arg1
     */
    submit_command(target_tick, sequence, actor_did, command_opcode, target_entity, arg0, arg1) {
        const ret = wasm.wasmsimulationworld_submit_command(this.__wbg_ptr, target_tick, sequence, actor_did, command_opcode, target_entity, arg0, arg1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) WasmSimulationWorld.prototype[Symbol.dispose] = WasmSimulationWorld.prototype.free;

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
     * Last parsed document handle.
     * @returns {any | undefined}
     */
    last_parsed() {
        const ret = wasm.webengine_last_parsed(this.__wbg_ptr);
        return ret;
    }
    /**
     * Loads a JSON scene document.
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
     * Loads a `.q42` volume from bytes.
     * @param {Uint8Array | number[]} bytes
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
     * Mounts a QApp under DOM `root_id`.
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
    /**
     * Creates `WebEngine` with the given state.
     */
    constructor() {
        const ret = wasm.webengine_new();
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WebEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Renders the loaded scene to the bound canvas.
     */
    render_to_canvas() {
        const ret = wasm.webengine_render_to_canvas(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) WebEngine.prototype[Symbol.dispose] = WebEngine.prototype.free;

/**
 * WASM bridge: align-sequences.
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
 * Evaluates input-output multipliers and total requirements via the Leontief inverse (I - A)^(-1).
 * @param {any} val
 * @returns {any}
 */
export function calculate_leontief_multipliers_wasm(val) {
    const ret = wasm.calculate_leontief_multipliers_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Evaluates distributional and welfare metrics (Gini, Atkinson index, Palma ratio,
 * mean, median, P10, P90) and emits an auditable `CalculationReceipt`.
 * @param {any} val
 * @returns {any}
 */
export function calculate_welfare_metrics_wasm(val) {
    const ret = wasm.calculate_welfare_metrics_wasm(val);
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
 * But-for / reachability causation (`causal::caused`).
 * @param {any} val
 * @returns {any}
 */
export function causal_caused_wasm(val) {
    const ret = wasm.causal_caused_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: check-drug-interactions.
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
 * Description-logic subsumption check (`check_subsumption_quin`).
 * @param {any} val
 * @returns {any}
 */
export function check_subsumption_wasm(val) {
    const ret = wasm.check_subsumption_wasm(val);
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
 * Compile Turtle / N3 `sh:NodeShape` documents into ShapeSpec-compatible JSON (UE-050).
 * @param {string} turtle
 * @returns {any}
 */
export function compile_shacl_turtle_wasm(turtle) {
    const ptr0 = passStringToWasm0(turtle, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.compile_shacl_turtle_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: compute-framingham-risk.
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
 * WASM bridge: compute-molecular-descriptors.
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
 * Evaluates ordinary least squares regression with complete diagnostics and receipt.
 * @param {any} val
 * @returns {any}
 */
export function compute_ols_diagnostics_wasm(val) {
    const ret = wasm.compute_ols_diagnostics_wasm(val);
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
 * WASM bridge: compute-reaction-metrics.
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
 * WASM bridge: compute-thermochemistry.
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
 * Creates `canvas`.
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
 * Dummy design. `{ categories:number[] }` → matrix + labels.
 * @param {any} val
 * @returns {any}
 */
export function design_dummies_wasm(val) {
    const ret = wasm.design_dummies_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM: encodes a design.
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
 * WASM bridge: detect-functional-groups.
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
 * WASM bridge: device-storage-policy.
 * @returns {any}
 */
export function device_storage_policy_wasm() {
    const ret = wasm.device_storage_policy_wasm();
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
 * Runs ontological entity verification over a `.10d` container and returns
 * the report as a JS value (`NULL` on serialization failure).
 * @param {Uint8Array | number[]} bytes
 * @returns {any}
 */
export function entity_verify_10d_wasm(bytes) {
    const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entity_verify_10d_wasm(ptr0, len0);
    return ret;
}

/**
 * Runs ontological entity verification over an `.hmc` bundle and returns
 * the report as a JS value (`NULL` on serialization failure).
 * @param {Uint8Array | number[]} bytes
 * @returns {any}
 */
export function entity_verify_bundle_wasm(bytes) {
    const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.entity_verify_bundle_wasm(ptr0, len0);
    return ret;
}

/**
 * Enumerate ASP stable-model world contexts (`enumerate_stable_models`).
 * @param {any} val
 * @returns {any}
 */
export function enumerate_stable_models_wasm(val) {
    const ret = wasm.enumerate_stable_models_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * Evaluate deontic norms in a quin frame (`evaluate_deontic_contract`).
 * @param {any} val
 * @returns {any}
 */
export function evaluate_deontic_wasm(val) {
    const ret = wasm.evaluate_deontic_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Evaluate epistemic claims (`evaluate_epistemic_frame`).
 * @param {any} val
 * @returns {any}
 */
export function evaluate_epistemic_wasm(val) {
    const ret = wasm.evaluate_epistemic_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: evaluate-inference-guard.
 * @param {any} value
 * @returns {any}
 */
export function evaluate_inference_guard_wasm(value) {
    const ret = wasm.evaluate_inference_guard_wasm(value);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: evaluate-lipinski.
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
 * Evaluate an LTL formula against a quin trace (`evaluate_ltl_trace`).
 * @param {any} val
 * @returns {any}
 */
export function evaluate_ltl_trace_wasm(val) {
    const ret = wasm.evaluate_ltl_trace_wasm(val);
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
 * Executes `ntriples-query`.
 * @param {string} query
 * @param {Uint8Array | number[]} db_bytes
 * @param {number} max_results
 * @returns {string}
 */
export function execute_ntriples_query(query, db_bytes, max_results) {
    let deferred4_0;
    let deferred4_1;
    try {
        const val3 = +max_results;
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passArray8ToWasm0(db_bytes, wasm.__wbindgen_malloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.execute_ntriples_query(ptr0, len0, ptr1, len1, val3);
        deferred4_0 = ret[0];
        deferred4_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * WASM: exports a tensor buffer.
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
 * WASM: exports a tensor slice.
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
 * Fuzzy t-norm (Gödel min / Łukasiewicz / product).
 * @param {any} val
 * @returns {any}
 */
export function fuzzy_t_norm_wasm(val) {
    const ret = wasm.fuzzy_t_norm_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * @param {Float64Array | number[]} points
 * @param {number} qx
 * @param {number} qy
 * @returns {number}
 */
export function geometry_nearest_site(points, qx, qy) {
    const val2 = +qx;
    const val3 = +qy;
    const ptr0 = passArrayF64ToWasm0(points, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.geometry_nearest_site(ptr0, len0, val2, val3);
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
 * WASM: runs a GeoSPARQL op.
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
 * Machine-readable SHACL capability and constraint coverage manifest (QW-05).
 * @returns {any}
 */
export function get_shacl_capability_manifest_wasm() {
    const ret = wasm.get_shacl_capability_manifest_wasm();
    return ret;
}

/**
 * Poll WebGPU engine init stage text (same surface as the LLM package).
 * Gets `webgpu-init-status`.
 * @returns {string}
 */
export function get_webgpu_init_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_webgpu_init_status();
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
 * Construct the volumetric renderer on the shared WebGPU device (no canvas).
 * @param {number} width
 * @param {number} height
 * @param {number} particle_cap
 * @returns {Promise<void>}
 */
export function init_offscreen_renderer(width, height, particle_cap) {
    const ret = wasm.init_offscreen_renderer(width, height, particle_cap);
    return ret;
}

/**
 * Initializes `panic-hook`.
 */
export function init_panic_hook() {
    wasm.init_panic_hook();
}

/**
 * Create the process-wide WebGPU device used by graph accel, offscreen
 * rendering, and LLM decode. Idempotent.
 * @returns {Promise<void>}
 */
export function init_shared_webgpu() {
    const ret = wasm.init_shared_webgpu();
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
 * `WasmOffloadIntent::intercept_pharmacogenomics_intent`.
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
 * Return the pinned Qualia JSON-LD 1.1 context + SHA-256 digest (UE-012).
 *
 * Packages should embed `context` and record `digest` on receipts — do not
 * fetch remote `@context` URLs at admission time.
 * @returns {any}
 */
export function jsonld_context_digest_wasm() {
    const ret = wasm.jsonld_context_digest_wasm();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Hohfeld correlative position for a jural opcode.
 * @param {any} val
 * @returns {any}
 */
export function jural_correlative_wasm(val) {
    const ret = wasm.jural_correlative_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * Lists the entries (key, kind, offset, length) of an HMC bundle as a JS
 * array; errors when the bundle fails to parse.
 * @param {Uint8Array | number[]} bundle_bytes
 * @returns {any}
 */
export function list_hmc_bundle_entries_wasm(bundle_bytes) {
    const ptr0 = passArray8ToWasm0(bundle_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.list_hmc_bundle_entries_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Names that are intentionally native-only (UE-035/044/045) — never stubbed in browser.
 * @returns {any}
 */
export function list_native_only_capabilities_wasm() {
    const ret = wasm.list_native_only_capabilities_wasm();
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
 * Multiple OLS. Input `{ x:number[][], y:number[], fit_intercept?:bool }` →
 * coefficients, SE, t, p, R², adj-R², F, residuals, fitted, n, k.
 * @param {any} val
 * @returns {any}
 */
export function ols_multiple_wasm(val) {
    const ret = wasm.ols_multiple_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * @param {Uint8Array | number[]} raw_quin_bytes
 * @returns {Uint8Array}
 */
export function pack_quins_into_superblock(seq_id, owner_did, raw_quin_bytes) {
    const val2 = BigInt.asIntN(64, seq_id);
    const val3 = BigInt.asIntN(64, owner_did);
    const ptr0 = passArray8ToWasm0(raw_quin_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.pack_quins_into_superblock(val2, val3, ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Package exposure receipt: context + shapes + vibe AST digests (UE-053).
 * @param {string} shapes_json
 * @param {Uint8Array | number[] | null} [vibe_program_cbor]
 * @returns {any}
 */
export function package_exposure_manifest_wasm(shapes_json, vibe_program_cbor) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passStringToWasm0(shapes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = isLikeNone(vibe_program_cbor) ? 0 : passArray8ToWasm0(vibe_program_cbor, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.package_exposure_manifest_wasm(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: parse-cbor-ld.
 * @param {Uint8Array | number[]} payload
 * @returns {any}
 */
export function parse_cbor_ld_wasm(payload) {
    const ptr0 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_cbor_ld_wasm(ptr0, len0);
    return ret;
}

/**
 * Parses CSV text per `CsvParseParams` field mappings into Quins via
 * `csv_parser::parse_csv_to_quins`, returning them as a JS value.
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
 * Parses JSON records per a mapping profile (JSONPath-ish field → predicate)
 * into Quins, returned as a JS value.
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
 * Parses a JSON array of flat `{s,p,o}` triples and returns them as a JS
 * array of `{subject,predicate,object}` objects; `JsValue::NULL` on parse
 * failure.
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
 * Parse JSON-LD 1.1 text into packed quins (Civics primary semantic format).
 *
 * Profile: `application/ld+json`. Context must be embedded/pinned by the caller;
 * this binding does not fetch remote contexts.
 * @param {string} payload
 * @returns {any}
 */
export function parse_jsonld_wasm(payload) {
    const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_jsonld_wasm(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: parse-n3logic.
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
 * Parse an RDF document by Solid/LDP Content-Type (or Qualia format id).
 * @param {string} content_type
 * @param {string} payload
 * @returns {any}
 */
export function parse_rdf_document_wasm(content_type, payload) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passStringToWasm0(content_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.parse_rdf_document_wasm(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: parse-turtle.
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
 * Parse and compile a yaml-ld-q42 document (workspace pages or HCF HypermediaDocument) into quins and lexicon.
 * @param {string} source
 * @param {bigint | null} [namespace]
 * @param {bigint | null} [lamport]
 * @returns {any}
 */
export function parse_yaml_ld_q42_wasm(source, namespace, lamport) {
    const val2 = BigInt.asIntN(64, (isLikeNone(namespace) ? BigInt(0) : namespace));
    const val3 = BigInt.asIntN(64, (isLikeNone(lamport) ? BigInt(0) : lamport));
    const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.parse_yaml_ld_q42_wasm(ptr0, len0, !isLikeNone(namespace), val2, !isLikeNone(lamport), val3);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: plan-device-storage.
 * @param {any} val
 * @returns {any}
 */
export function plan_device_storage_wasm(val) {
    const ret = wasm.plan_device_storage_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Page-side timeout. Stops a late surface from being adopted. If the canvas
 * was already claimed, [`portal_webgpu_canvas_claimed`] is true and the page
 * must replace that element before the proof tick.
 */
export function portal_abort_webgpu() {
    wasm.portal_abort_webgpu();
}

/**
 * Bind a hardware WebGL2 Anatomy renderer before `QualiaPortal` construction.
 * This is selected only after capability probing proves that WebGPU has no
 * usable adapter and WebGL2 context creation succeeds.
 * @param {HTMLCanvasElement} canvas
 * @returns {boolean}
 */
export function portal_init_webgl2(canvas) {
    const ret = wasm.portal_init_webgl2(canvas);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] !== 0;
}

/**
 * Create the WebGPU device + surface asynchronously and stash it for the render loop to adopt.
 * JS awaits this before constructing the portal and starting the render loop.
 * The canvas must still be context-free (no 2d context yet) so the WebGPU surface
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
 * True once WebGPU has called `getContext("webgpu")` on the init canvas.
 * A 2d tick on that same element cannot paint; replace it.
 * @returns {boolean}
 */
export function portal_webgpu_canvas_claimed() {
    const ret = wasm.portal_webgpu_canvas_claimed();
    return ret !== 0;
}

/**
 * WASM bridge: predict-receptor-binding.
 * @returns {number}
 */
export function predict_receptor_binding_wasm() {
    const ret = wasm.predict_receptor_binding_wasm();
    return ret;
}

/**
 * Probe browser graphics APIs without claiming the page's presentation canvas.
 *
 * `selected_backend` is a recommendation from API/adapter availability only;
 * actual initialization can still fail because of surface compatibility,
 * memory pressure, or device loss. Callers must use the result of
 * `portal_init_webgpu` / `portal_init_webgl2` as the final selection receipt.
 * @returns {Promise<any>}
 */
export function probe_portal_graphics() {
    const ret = wasm.probe_portal_graphics();
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
 * RDFC-1.0 graph hash — **honest fail-closed** until a conforming implementation ships (UE-013).
 *
 * Never returns a digest labelled as RDFC-1.0. Optionally includes a
 * `provisional_spo_sha256` under profile `qualia:provisional-spo-sha256-v1`
 * for scaffolding only.
 * @param {any} val
 * @returns {any}
 */
export function rdfc10_graph_hash_wasm(val) {
    const ret = wasm.rdfc10_graph_hash_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Reads entry `key` from an HMC bundle, verifying its checksum before
 * returning the raw bytes; errors on parse failure, missing key, or checksum
 * mismatch.
 * @param {Uint8Array | number[]} bundle_bytes
 * @param {string} key
 * @returns {Uint8Array}
 */
export function read_hmc_bundle_entry_wasm(bundle_bytes, key) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passArray8ToWasm0(bundle_bytes, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passStringToWasm0(key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.read_hmc_bundle_entry_wasm(ptr0, len0, ptr1, len1);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
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
 * Select the next recovery backend using the shared VibeScript policy.
 * @param {boolean} webgpu_retry_allowed
 * @param {boolean} webgl2_allowed
 * @returns {string}
 */
export function recommend_graphics_recovery(webgpu_retry_allowed, webgl2_allowed) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.recommend_graphics_recovery(webgpu_retry_allowed, webgl2_allowed);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
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
 * Route contradictions into an isolated context (`route_paraconsistent`).
 * @param {any} val
 * @returns {any}
 */
export function route_paraconsistent_wasm(val) {
    const ret = wasm.route_paraconsistent_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Runs `semantic-simulation`.
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
 * WASM: samples browser telemetry.
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
 * Bounded stride sample of packed 48-byte Quins. Browser graphs cannot mmap
 * `.q42` files; this is the WASM-safe equivalent of `mmap_sample_quins`.
 * @param {Uint8Array | number[]} db_bytes
 * @param {number} max_quins
 * @returns {Uint8Array}
 */
export function sample_packed_quins_wasm(db_bytes, max_quins) {
    const val2 = +max_quins;
    const ptr0 = passArray8ToWasm0(db_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.sample_packed_quins_wasm(ptr0, len0, val2);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
}

/**
 * Serializes the given Quins to CSV text following the provided column
 * profile, returned as a JS string.
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
 * @param {Float64Array | number[]} data
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
 * @param {Float32Array | number[]} data
 * @returns {Uint8Array}
 */
export function serialize_float_array(data) {
    const ptr0 = passArrayF32ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.serialize_float_array(ptr0, len0);
    return ret;
}

/**
 * Serializes the given Quins to a JSON payload following the provided
 * mapping profile, returned as a JS value.
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
 * Serialize quins to RDF. `format` accepts Qualia ids (`turtle`, `jsonld`, `n3`)
 * or Solid MIME types (`text/turtle`, `application/ld+json`, `text/n3`).
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
 * Negotiate Solid `Accept` → preferred RDF Content-Type.
 * @param {string} accept
 * @returns {any}
 */
export function solid_negotiate_accept_wasm(accept) {
    const ptr0 = passStringToWasm0(accept, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.solid_negotiate_accept_wasm(ptr0, len0);
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
 * WASM: encodes a spatial payload.
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
 * Breusch–Pagan. `{ residuals, x:number[][] }` (x = original predictors).
 * @param {any} val
 * @returns {any}
 */
export function stats_breusch_pagan_wasm(val) {
    const ret = wasm.stats_breusch_pagan_wasm(val);
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
 * Chow structural break. `{ x, y, break_index }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_chow_test_wasm(val) {
    const ret = wasm.stats_chow_test_wasm(val);
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
 * Durbin–Watson. `{ residuals:[..] }` → `{ statistic, approx_p_value }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_durbin_watson_wasm(val) {
    const ret = wasm.stats_durbin_watson_wasm(val);
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
 * Influence (leverage / Cook / studentized). `{ x, y }` — flags only, never drops.
 * @param {any} val
 * @returns {any}
 */
export function stats_influence_wasm(val) {
    const ret = wasm.stats_influence_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Jarque–Bera on a residual vector. `{ residuals:[..] }` → `{ statistic, p_value, skewness, excess_kurtosis }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_jarque_bera_wasm(val) {
    const ret = wasm.stats_jarque_bera_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * LDA. `{ x:number[][], y:number[] (int class labels) }` → classes + predictions.
 * @param {any} val
 * @returns {any}
 */
export function stats_lda_wasm(val) {
    const ret = wasm.stats_lda_wasm(val);
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
 * Binary logit. `{ x:number[][], y:number[] (0/1), fit_intercept?:bool }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_logit_wasm(val) {
    const ret = wasm.stats_logit_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Mahalanobis outliers. `{ x:number[][], alpha?:number }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_mahalanobis_outliers_wasm(val) {
    const ret = wasm.stats_mahalanobis_outliers_wasm(val);
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
 * Univariate outlier screen. `{ data:[..] }` → mean/median + 2σ/3σ indices.
 * @param {any} val
 * @returns {any}
 */
export function stats_outlier_screen_univariate_wasm(val) {
    const ret = wasm.stats_outlier_screen_univariate_wasm(val);
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
 * Ramsey RESET. `{ x, y, power_max?:number }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_ramsey_reset_wasm(val) {
    const ret = wasm.stats_ramsey_reset_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Residual runs test. `{ residuals:[..] }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_residual_runs_wasm(val) {
    const ret = wasm.stats_residual_runs_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Residual symmetry. `{ residuals:[..] }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_residual_symmetry_wasm(val) {
    const ret = wasm.stats_residual_symmetry_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Spurious-regression guard. `{ y, x?:number[] }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_spurious_guard_wasm(val) {
    const ret = wasm.stats_spurious_guard_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Backward stepwise (exploratory). `{ x, y, exit_alpha?, max_steps? }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_stepwise_backward_wasm(val) {
    const ret = wasm.stats_stepwise_backward_wasm(val);
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
 * VIF per predictor column. `{ x:number[][] }` → `{ vif:[..] }`.
 * @param {any} val
 * @returns {any}
 */
export function stats_vif_wasm(val) {
    const ret = wasm.stats_vif_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * STIT: did agent bring about content?
 * @param {any} val
 * @returns {any}
 */
export function stit_brought_about_wasm(val) {
    const ret = wasm.stit_brought_about_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Series transform. `{ values, kind: "log"|"log1p"|"sqrt"|"square"|"reciprocal"|"exp" }`.
 * @param {any} val
 * @returns {any}
 */
export function transform_series_wasm(val) {
    const ret = wasm.transform_series_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * WASM bridge: validate-fasta.
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
 * WASM bridge: validate-fhir-observation.
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
 * WASM bridge: validate-shacl-constraint.
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
 * Validates raw packed 48-byte Quins against a list of JSON ShapeSpecs.
 * @param {Uint8Array | number[]} db_bytes
 * @param {string} shapes_json
 * @returns {any}
 */
export function validate_shacl_graph_wasm(db_bytes, shapes_json) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passArray8ToWasm0(db_bytes, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passStringToWasm0(shapes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.validate_shacl_graph_wasm(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Full graph SHACL validation from N3/N-Triples data and JSON ShapeSpecs.
 * Returns the complete `ValidationReport` preserving conforms, focus node, path,
 * severity, and constraint component.
 * @param {string} data_n3
 * @param {string} shapes_json
 * @returns {any}
 */
export function validate_shacl_json_wasm(data_n3, shapes_json) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passStringToWasm0(data_n3, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passStringToWasm0(shapes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.validate_shacl_json_wasm(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Values abuse-check (agency.n3 G1/G1' personhood guard) — WASM surface for Civics.
 * @param {any} val
 * @returns {any}
 */
export function values_check_wasm(val) {
    const ret = wasm.values_check_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Consent non-coerced guard (`capacity::detect_duress` inverted).
 * @param {any} val
 * @returns {any}
 */
export function values_consent_non_coerced_wasm(val) {
    const ret = wasm.values_consent_non_coerced_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Harm-below-ceiling guard (wasm-safe numeric; CAS marginal-harm stays native).
 * @param {any} val
 * @returns {any}
 */
export function values_harm_below_ceiling_wasm(val) {
    const ret = wasm.values_harm_below_ceiling_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * WASM bridge: verify-backup-manifest.
 * @param {any} manifest
 * @param {string} payload_sha256_hex
 * @returns {any}
 */
export function verify_backup_manifest_wasm(manifest, payload_sha256_hex) {
    const ptr0 = passStringToWasm0(payload_sha256_hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.verify_backup_manifest_wasm(manifest, ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Verify a signed law package (JSON) against an Ed25519 public key.
 * @param {string} json
 * @param {Uint8Array | number[]} public_key
 * @returns {boolean}
 */
export function verify_law_package_wasm(json, public_key) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArray8ToWasm0(public_key, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.verify_law_package_wasm(ptr0, len0, ptr1, len1);
    return ret !== 0;
}

/**
 * Multiple OLS + verification report with Civics calculation receipt.
 * @param {any} val
 * @returns {any}
 */
export function verify_regression_model_receipt_wasm(val) {
    const ret = wasm.verify_regression_model_receipt_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Full verification report + soft/hard flags. `{ x, y, alpha?, strict? }`.
 * @param {any} val
 * @returns {any}
 */
export function verify_regression_model_wasm(val) {
    const ret = wasm.verify_regression_model_wasm(val);
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
 * @param {Uint8Array | number[]} block_bytes
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
 * WASM bridge: convex-hull-2d.
 * @param {Float64Array | number[]} points_flat
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
 * WASM bridge: delaunay-triangulation-2d.
 * @param {Float64Array | number[]} points_flat
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
    const val3 = +threshold;
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passStringToWasm0(principal, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passStringToWasm0(domain, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.webizen_propose_agreement(_nominated_guardians, ptr0, len0, ptr1, len1, val3);
    return BigInt.asUintN(64, ret);
}

/**
 * Signs a pending agreement, advancing its state machine and triggering WebRTC peer sync.
 * @param {bigint} _agreement_id
 * @param {string} _private_key_mock
 */
export function webizen_sign_agreement(_agreement_id, _private_key_mock) {
    const val2 = BigInt.asIntN(64, _agreement_id);
    const ptr0 = passStringToWasm0(_private_key_mock, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    wasm.webizen_sign_agreement(val2, ptr0, len0);
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
 * @param {Uint8Array | number[]} block_bytes
 * @returns {Promise<void>}
 */
export function write_opfs_block(block_index, block_bytes) {
    const val2 = +block_index;
    const ptr0 = passArray8ToWasm0(block_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.write_opfs_block(val2, ptr0, len0);
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
        __wbg_Error_208446807d0d1e30: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Number_ff192a72a0dcdf6d: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_Window_41c17a1c0ba5c428: function(arg0) {
            const ret = arg0.Window;
            return ret;
        },
        __wbg_WorkerGlobalScope_1a051b1fae97ad83: function(arg0) {
            const ret = arg0.WorkerGlobalScope;
            return ret;
        },
        __wbg___wbindgen_bigint_get_as_i64_11da4b52cd0aedef: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64((arg0 >>> 0) + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_4f1baf9a00be3c74: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_copy_to_typed_array_680b191e71920dc1: function(arg0, arg1, arg2) {
            new Uint8Array(arg2.buffer, arg2.byteOffset, arg2.byteLength).set(getArrayU8FromWasm0(arg0, arg1));
        },
        __wbg___wbindgen_debug_string_f249a2a76caa9a55: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_3f58a9d3b3a7eed2: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_5fa10641e039d3ef: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_function_741ff2bb612836f5: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_c22311b1cc228e3a: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_8b968c3ab8e96595: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_fc456fee5d3317de: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_be5b34249ff1fc15: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_38d7bfbe4cc84039: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_b2f214e2e4b30e36: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_934b640d0889af78: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64((arg0 >>> 0) + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_7ce8eee7fd327545: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_4c88d8bf264fd984: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_09736879a3f67559: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_activeTexture_89cef8c297ad0698: function(arg0, arg1) {
            arg0.activeTexture(arg1 >>> 0);
        },
        __wbg_addColorStop_5062bdb863ddbc7c: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addColorStop(arg1, getStringFromWasm0(arg2, arg3));
        }, arguments); },
        __wbg_appendChild_dbf7ad2f48413560: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_arc_df3f7eb77e59876f: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.arc(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_architecture_713551f5ae3afd6a: function(arg0, arg1) {
            const ret = arg1.architecture;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_arrayBuffer_5abbcb61bd48f427: function(arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        },
        __wbg_attachShader_3ed301cc92db59f6: function(arg0, arg1, arg2) {
            arg0.attachShader(arg1, arg2);
        },
        __wbg_beginPath_ab690dc0c7efe6dc: function(arg0) {
            arg0.beginPath();
        },
        __wbg_beginRenderPass_e0c3a1cc44f033f9: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.beginRenderPass(arg1);
            return ret;
        }, arguments); },
        __wbg_bindBuffer_cbc9075f76fffa51: function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        },
        __wbg_bindFramebuffer_79534f24b57b7b95: function(arg0, arg1, arg2) {
            arg0.bindFramebuffer(arg1 >>> 0, arg2);
        },
        __wbg_bindRenderbuffer_612f27d9b3b97b9f: function(arg0, arg1, arg2) {
            arg0.bindRenderbuffer(arg1 >>> 0, arg2);
        },
        __wbg_bindTexture_a23350e1b573ddfd: function(arg0, arg1, arg2) {
            arg0.bindTexture(arg1 >>> 0, arg2);
        },
        __wbg_bindVertexArray_cc64c48bf67ab8d4: function(arg0, arg1) {
            arg0.bindVertexArray(arg1);
        },
        __wbg_blendFuncSeparate_6db2212299fd7f1e: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_bufferData_9eb9c4c5053ac480: function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        },
        __wbg_bufferSubData_8acfdcd4b9f7edeb: function(arg0, arg1, arg2, arg3) {
            arg0.bufferSubData(arg1 >>> 0, arg2, arg3);
        },
        __wbg_byteLength_c3cb2602a1785d77: function(arg0) {
            const ret = arg0.byteLength;
            return ret;
        },
        __wbg_call_40b008ba32f73991: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_call_a410eb4b59ed5965: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_checkFramebufferStatus_03ce134350260eda: function(arg0, arg1) {
            const ret = arg0.checkFramebufferStatus(arg1 >>> 0);
            return ret;
        },
        __wbg_clearColor_195b5a731bca5836: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearColor(arg1, arg2, arg3, arg4);
        },
        __wbg_clearDepth_3dc1f52f96f29526: function(arg0, arg1) {
            arg0.clearDepth(arg1);
        },
        __wbg_clearRect_428f96a6ba73b2f4: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearRect(arg1, arg2, arg3, arg4);
        },
        __wbg_clear_ccb3d618da90fea3: function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        },
        __wbg_clip_26239db7c0b67972: function(arg0) {
            arg0.clip();
        },
        __wbg_closePath_e9e11e034c5d75a3: function(arg0) {
            arg0.closePath();
        },
        __wbg_close_e308d0007b7c4d58: function(arg0) {
            const ret = arg0.close();
            return ret;
        },
        __wbg_compileShader_11eb6754e9b15970: function(arg0, arg1) {
            arg0.compileShader(arg1);
        },
        __wbg_configure_cdfb5caf87853cfa: function() { return handleError(function (arg0, arg1) {
            arg0.configure(arg1);
        }, arguments); },
        __wbg_constructor_76ad3e2c50d259de: function(arg0) {
            const ret = arg0.constructor;
            return ret;
        },
        __wbg_copyTextureToBuffer_5da9980713e9c694: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.copyTextureToBuffer(arg1, arg2, arg3);
        }, arguments); },
        __wbg_copyTextureToTexture_402cb1fb11105631: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.copyTextureToTexture(arg1, arg2, arg3);
        }, arguments); },
        __wbg_createBindGroupLayout_f632141bc3211b33: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBindGroupLayout(arg1);
            return ret;
        }, arguments); },
        __wbg_createBindGroup_e21d6cba1e0d10e9: function(arg0, arg1) {
            const ret = arg0.createBindGroup(arg1);
            return ret;
        },
        __wbg_createBuffer_2c27ad5f899d418d: function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createBuffer_d276cc32f357885e: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBuffer(arg1);
            return ret;
        }, arguments); },
        __wbg_createCommandEncoder_79f87e24728f9c2f: function(arg0, arg1) {
            const ret = arg0.createCommandEncoder(arg1);
            return ret;
        },
        __wbg_createElement_de4a31ed2fb252be: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createFramebuffer_cdf686778c61014d: function(arg0) {
            const ret = arg0.createFramebuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createLinearGradient_d1048a4a2378e666: function(arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.createLinearGradient(arg1, arg2, arg3, arg4);
            return ret;
        },
        __wbg_createPipelineLayout_4750ef3322a506af: function(arg0, arg1) {
            const ret = arg0.createPipelineLayout(arg1);
            return ret;
        },
        __wbg_createProgram_ea784724f6f0074f: function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createRenderPipeline_a0026c65b1699246: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createRenderPipeline(arg1);
            return ret;
        }, arguments); },
        __wbg_createRenderbuffer_e45c14c918a72668: function(arg0) {
            const ret = arg0.createRenderbuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createSampler_64a236b481fcf11d: function(arg0, arg1) {
            const ret = arg0.createSampler(arg1);
            return ret;
        },
        __wbg_createShaderModule_01004c667ef863e9: function(arg0, arg1) {
            const ret = arg0.createShaderModule(arg1);
            return ret;
        },
        __wbg_createShader_9754a1e4ef374e84: function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createTexture_3287f26407473071: function(arg0) {
            const ret = arg0.createTexture();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createTexture_63b8ce4a0427eccd: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createTexture(arg1);
            return ret;
        }, arguments); },
        __wbg_createVertexArray_1293d2ffa8346be2: function(arg0) {
            const ret = arg0.createVertexArray();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_createView_a09be2d81f564454: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createView(arg1);
            return ret;
        }, arguments); },
        __wbg_createWritable_b7dd17aab7d14494: function(arg0) {
            const ret = arg0.createWritable();
            return ret;
        },
        __wbg_crypto_23f565a907d30078: function(arg0) {
            const ret = arg0.crypto;
            return ret;
        },
        __wbg_deleteBuffer_673584ba5a55669d: function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        },
        __wbg_deleteFramebuffer_ffdb1bee4c92ae4c: function(arg0, arg1) {
            arg0.deleteFramebuffer(arg1);
        },
        __wbg_deleteRenderbuffer_3a6e1676ab926e0f: function(arg0, arg1) {
            arg0.deleteRenderbuffer(arg1);
        },
        __wbg_deleteTexture_1653a9994a44215e: function(arg0, arg1) {
            arg0.deleteTexture(arg1);
        },
        __wbg_deleteVertexArray_2589b74b67ba20c8: function(arg0, arg1) {
            arg0.deleteVertexArray(arg1);
        },
        __wbg_depthFunc_04abd9f6ff980ba9: function(arg0, arg1) {
            arg0.depthFunc(arg1 >>> 0);
        },
        __wbg_depthMask_2ba7e83178c3cec3: function(arg0, arg1) {
            arg0.depthMask(arg1 !== 0);
        },
        __wbg_description_3b1c328df3985d9e: function(arg0, arg1) {
            const ret = arg1.description;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_device_8f641af33bb758e7: function(arg0, arg1) {
            const ret = arg1.device;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_disable_8a9e81e6d64d046b: function(arg0, arg1) {
            arg0.disable(arg1 >>> 0);
        },
        __wbg_document_3b5c3071b43e97ee: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_done_57e08d0be6e30c02: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_drawArraysInstanced_762b36f2147c7a85: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawArraysInstanced(arg1 >>> 0, arg2, arg3, arg4);
        },
        __wbg_drawArrays_a9c18db2e94f4a82: function(arg0, arg1, arg2, arg3) {
            arg0.drawArrays(arg1 >>> 0, arg2, arg3);
        },
        __wbg_drawElements_d76df94ac144b73b: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        },
        __wbg_drawIndexed_22002106f5e59a8c: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.drawIndexed(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5 >>> 0);
        },
        __wbg_draw_f13656312ca14c1d: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.draw(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_enableVertexAttribArray_eeecc6185af24227: function(arg0, arg1) {
            arg0.enableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_enable_e955d456ef6b93f7: function(arg0, arg1) {
            arg0.enable(arg1 >>> 0);
        },
        __wbg_end_9be5028697a2735e: function(arg0) {
            arg0.end();
        },
        __wbg_entries_7bb1459bd9b5c2b9: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_error_757e9472f8410341: function(arg0, arg1) {
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
        __wbg_estimate_b2044bb9452e73cd: function() { return handleError(function (arg0) {
            const ret = arg0.estimate();
            return ret;
        }, arguments); },
        __wbg_features_2d5935d661d3a67a: function(arg0) {
            const ret = arg0.features;
            return ret;
        },
        __wbg_features_e0c615f8645ccaf6: function(arg0) {
            const ret = arg0.features;
            return ret;
        },
        __wbg_fillRect_e1f0b9915220348b: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.fillRect(arg1, arg2, arg3, arg4);
        },
        __wbg_fillText_25a06a7940c957ed: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.fillText(getStringFromWasm0(arg1, arg2), arg3, arg4, arg5);
        }, arguments); },
        __wbg_fillText_89d353a9ad9f4a1a: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.fillText(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_fill_cb36fe21a8d46cd7: function(arg0) {
            arg0.fill();
        },
        __wbg_finish_334c2b9fed0efc1c: function(arg0) {
            const ret = arg0.finish();
            return ret;
        },
        __wbg_finish_619e5acd7c49f841: function(arg0, arg1) {
            const ret = arg0.finish(arg1);
            return ret;
        },
        __wbg_framebufferRenderbuffer_ffaba93222aea77c: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4);
        },
        __wbg_framebufferTexture2D_b6250a0c37eb48c2: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5);
        },
        __wbg_fromCodePoint_84793ae8d520c4b9: function() { return handleError(function (arg0) {
            const ret = String.fromCodePoint(arg0 >>> 0);
            return ret;
        }, arguments); },
        __wbg_getAttribute_c7b80d4010284561: function(arg0, arg1, arg2, arg3) {
            const ret = arg1.getAttribute(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_getContext_1938edb9a972f63d: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getContext_9d32151c28a36ed6: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2), arg3);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getContext_d93f904b714852ae: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getCurrentTexture_085901e617f7c490: function() { return handleError(function (arg0) {
            const ret = arg0.getCurrentTexture();
            return ret;
        }, arguments); },
        __wbg_getDirectory_50787a69f3a13110: function(arg0) {
            const ret = arg0.getDirectory();
            return ret;
        },
        __wbg_getElementById_7ee213db584f37b0: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getError_bb38cafe57b0913f: function(arg0) {
            const ret = arg0.getError();
            return ret;
        },
        __wbg_getExtension_30c10f49f67eb05d: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getFileHandle_1cb5562219601c4f: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getFileHandle(getStringFromWasm0(arg1, arg2), arg3);
            return ret;
        },
        __wbg_getFileHandle_d551aa420ea57cd0: function(arg0, arg1, arg2) {
            const ret = arg0.getFileHandle(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_getFile_ebc6243a91493807: function(arg0) {
            const ret = arg0.getFile();
            return ret;
        },
        __wbg_getMappedRange_7f068213fba7d9ec: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getMappedRange(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_getPreferredCanvasFormat_44f82c66f9f2dec7: function(arg0) {
            const ret = arg0.getPreferredCanvasFormat();
            return (__wbindgen_enum_GpuTextureFormat.indexOf(ret) + 1 || 102) - 1;
        },
        __wbg_getProgramInfoLog_01799fd7b07b7f3c: function(arg0, arg1, arg2) {
            const ret = arg1.getProgramInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_getProgramParameter_1e7562e7d66170c3: function(arg0, arg1, arg2) {
            const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getRandomValues_8b6f1f663e6f188d: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_getRandomValues_d2908e0b99612b91: function() { return handleError(function (arg0, arg1) {
            arg0.getRandomValues(arg1);
        }, arguments); },
        __wbg_getShaderInfoLog_3cfe5b3e097d0f2d: function(arg0, arg1, arg2) {
            const ret = arg1.getShaderInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_getShaderParameter_fc64069fbc010d19: function(arg0, arg1, arg2) {
            const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
            return ret;
        },
        __wbg_getUniformLocation_6352e0669c8f5393: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_0c55869fe53c7d14: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_2080c49015e35254: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_635eea311fa1a291: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_d7176cfb76d71114: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_e6325ad374eff861: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_fc601247060b7204: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_gpu_9e77ff6e3c319958: function(arg0) {
            const ret = arg0.gpu;
            return ret;
        },
        __wbg_has_2c4d5cf25ec8c1cf: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.has(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_height_ac6f1c3f49c13fe2: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_id_2aec05cae0b5abeb: function(arg0, arg1) {
            const ret = arg1.id;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_info_d293b2ec11de1d77: function(arg0) {
            const ret = arg0.info;
            return ret;
        },
        __wbg_instanceof_ArrayBuffer_12cdaa4161423e6b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Blob_56431c8f5387c12d: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Blob;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_CanvasRenderingContext2d_72a60be8aaadecab: function(arg0) {
            let result;
            try {
                result = arg0 instanceof CanvasRenderingContext2D;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_FileSystemDirectoryHandle_4d0d8d8275d0019c: function(arg0) {
            let result;
            try {
                result = arg0 instanceof FileSystemDirectoryHandle;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_FileSystemFileHandle_5a487c9c312d952b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof FileSystemFileHandle;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_FileSystemWritableFileStream_721031ffe57a5aec: function(arg0) {
            let result;
            try {
                result = arg0 instanceof FileSystemWritableFileStream;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_File_8b277b2875442560: function(arg0) {
            let result;
            try {
                result = arg0 instanceof File;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuInternalError_6b518b719e3c5f26: function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUInternalError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuOutOfMemoryError_7f136433b2485a47: function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUOutOfMemoryError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuValidationError_6c09ac57918825da: function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUValidationError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlCanvasElement_89f280683e987b9e: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLCanvasElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_StorageEstimate_a861777056b64331: function(arg0) {
            let result;
            try {
                result = arg0 instanceof StorageEstimate;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_d7aeb0dcb3108fea: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_WebGl2RenderingContext_de6feb460369d6c6: function(arg0) {
            let result;
            try {
                result = arg0 instanceof WebGL2RenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_5cf453f5639aa1a2: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_181eee9e42397260: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isArray_bcf1376d0d732d08: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isContextLost_dc5d602e27060cc9: function(arg0) {
            const ret = arg0.isContextLost();
            return ret;
        },
        __wbg_isFallbackAdapter_5390f5473e872226: function(arg0) {
            const ret = arg0.isFallbackAdapter;
            return ret;
        },
        __wbg_isSafeInteger_1a35ba4d31e71d38: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_e4c5e8f68ada7b58: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_label_81a2abe4d7fb3578: function(arg0, arg1) {
            const ret = arg1.label;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_length_186163212ffe79c3: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_b03a0fabb109b1e9: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_c702d06d28b96b72: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_limits_7bbe1a34161fc73f: function(arg0) {
            const ret = arg0.limits;
            return ret;
        },
        __wbg_limits_8f487fdb1bb1efa2: function(arg0) {
            const ret = arg0.limits;
            return ret;
        },
        __wbg_lineTo_91262b25fb37c461: function(arg0, arg1, arg2) {
            arg0.lineTo(arg1, arg2);
        },
        __wbg_linkProgram_99848b73f1ebab5f: function(arg0, arg1) {
            arg0.linkProgram(arg1);
        },
        __wbg_lost_8d2168b0605bc799: function(arg0) {
            const ret = arg0.lost;
            return ret;
        },
        __wbg_mapAsync_c9f05cf7113dfc3d: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.mapAsync(arg1 >>> 0, arg2, arg3);
            return ret;
        },
        __wbg_maxBindGroupsPlusVertexBuffers_062d3b674a2b26de: function(arg0) {
            const ret = arg0.maxBindGroupsPlusVertexBuffers;
            return ret;
        },
        __wbg_maxBindGroups_76c8840031a234f4: function(arg0) {
            const ret = arg0.maxBindGroups;
            return ret;
        },
        __wbg_maxBindingsPerBindGroup_3fb492647cb58636: function(arg0) {
            const ret = arg0.maxBindingsPerBindGroup;
            return ret;
        },
        __wbg_maxBufferSize_9df923e64cef912d: function(arg0) {
            const ret = arg0.maxBufferSize;
            return ret;
        },
        __wbg_maxColorAttachmentBytesPerSample_073b691d06f330eb: function(arg0) {
            const ret = arg0.maxColorAttachmentBytesPerSample;
            return ret;
        },
        __wbg_maxColorAttachments_8c0e408ab4dd0010: function(arg0) {
            const ret = arg0.maxColorAttachments;
            return ret;
        },
        __wbg_maxComputeInvocationsPerWorkgroup_722af4d4965ce944: function(arg0) {
            const ret = arg0.maxComputeInvocationsPerWorkgroup;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeX_66c672992ba6ac0b: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeX;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeY_ad17f517fd0efcdc: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeY;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeZ_54adecf2f48c9f2e: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeZ;
            return ret;
        },
        __wbg_maxComputeWorkgroupStorageSize_d6fbfc944e70e551: function(arg0) {
            const ret = arg0.maxComputeWorkgroupStorageSize;
            return ret;
        },
        __wbg_maxComputeWorkgroupsPerDimension_7f41ba41ccc5c1f9: function(arg0) {
            const ret = arg0.maxComputeWorkgroupsPerDimension;
            return ret;
        },
        __wbg_maxDynamicStorageBuffersPerPipelineLayout_9f3edad50a9c176d: function(arg0) {
            const ret = arg0.maxDynamicStorageBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxDynamicUniformBuffersPerPipelineLayout_e84279e5b096a774: function(arg0) {
            const ret = arg0.maxDynamicUniformBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxInterStageShaderVariables_255d63625b2f7ed5: function(arg0) {
            const ret = arg0.maxInterStageShaderVariables;
            return ret;
        },
        __wbg_maxSampledTexturesPerShaderStage_cd7a421c14e8c02f: function(arg0) {
            const ret = arg0.maxSampledTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxSamplersPerShaderStage_a0048181792e8579: function(arg0) {
            const ret = arg0.maxSamplersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageBufferBindingSize_d2407e7a6f0c3084: function(arg0) {
            const ret = arg0.maxStorageBufferBindingSize;
            return ret;
        },
        __wbg_maxStorageBuffersInFragmentStage_264080883ace3580: function(arg0) {
            const ret = arg0.maxStorageBuffersInFragmentStage;
            return ret;
        },
        __wbg_maxStorageBuffersInVertexStage_c86a41c2f757534a: function(arg0) {
            const ret = arg0.maxStorageBuffersInVertexStage;
            return ret;
        },
        __wbg_maxStorageBuffersPerShaderStage_e0e60ae884b3af7f: function(arg0) {
            const ret = arg0.maxStorageBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageTexturesInFragmentStage_163fc291b5b68f45: function(arg0) {
            const ret = arg0.maxStorageTexturesInFragmentStage;
            return ret;
        },
        __wbg_maxStorageTexturesInVertexStage_2759484771a41785: function(arg0) {
            const ret = arg0.maxStorageTexturesInVertexStage;
            return ret;
        },
        __wbg_maxStorageTexturesPerShaderStage_6a73a80dc8f357fe: function(arg0) {
            const ret = arg0.maxStorageTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxTextureArrayLayers_2f3cdbd8a2b8f39a: function(arg0) {
            const ret = arg0.maxTextureArrayLayers;
            return ret;
        },
        __wbg_maxTextureDimension1D_c698ab2620ee31ba: function(arg0) {
            const ret = arg0.maxTextureDimension1D;
            return ret;
        },
        __wbg_maxTextureDimension2D_f5c9496a6116b813: function(arg0) {
            const ret = arg0.maxTextureDimension2D;
            return ret;
        },
        __wbg_maxTextureDimension3D_e3a5e348dfbeef60: function(arg0) {
            const ret = arg0.maxTextureDimension3D;
            return ret;
        },
        __wbg_maxUniformBufferBindingSize_a0fa0c99aba5f8fc: function(arg0) {
            const ret = arg0.maxUniformBufferBindingSize;
            return ret;
        },
        __wbg_maxUniformBuffersPerShaderStage_f9ab59e9310f34b2: function(arg0) {
            const ret = arg0.maxUniformBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxVertexAttributes_503f8403b5183f2c: function(arg0) {
            const ret = arg0.maxVertexAttributes;
            return ret;
        },
        __wbg_maxVertexBufferArrayStride_428c428c3b7bf13c: function(arg0) {
            const ret = arg0.maxVertexBufferArrayStride;
            return ret;
        },
        __wbg_maxVertexBuffers_0ae978c6de6ae709: function(arg0) {
            const ret = arg0.maxVertexBuffers;
            return ret;
        },
        __wbg_message_0f501da4f052ced7: function(arg0, arg1) {
            const ret = arg1.message;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_message_8cfe1379662edfc1: function(arg0, arg1) {
            const ret = arg1.message;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_minStorageBufferOffsetAlignment_0535b9f0f62064e6: function(arg0) {
            const ret = arg0.minStorageBufferOffsetAlignment;
            return ret;
        },
        __wbg_minUniformBufferOffsetAlignment_76801a59f9bf1be0: function(arg0) {
            const ret = arg0.minUniformBufferOffsetAlignment;
            return ret;
        },
        __wbg_moveTo_b4cd3b0b7e1585a5: function(arg0, arg1, arg2) {
            arg0.moveTo(arg1, arg2);
        },
        __wbg_msCrypto_d5f06fad6bf1409b: function(arg0) {
            const ret = arg0.msCrypto;
            return ret;
        },
        __wbg_name_fd11f69c917c5670: function(arg0) {
            const ret = arg0.name;
            return ret;
        },
        __wbg_navigator_9641ab32fa612dd1: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_navigator_c17f443a6fe5d38d: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_13de428334042d26: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_1eadcf6827db58fd: function(arg0) {
            const ret = new SharedArrayBuffer(arg0 >>> 0);
            return ret;
        },
        __wbg_new_37eb45aaf5eeb820: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_7075786089b89c5e: function() {
            const ret = new Map();
            return ret;
        },
        __wbg_new_ba40181b3f24ac80: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__ha3a3f9b66eed9539(a, state0.b, arg0, arg1);
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
        __wbg_new_bb74001155cb8ca8: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_cd4348290afae586: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_from_slice_355c10c5d8f2addb: function(arg0, arg1) {
            const ret = new Float32Array(getArrayF32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_aaf85a9746711968: function(arg0, arg1) {
            const ret = new Float64Array(getArrayF64FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_b8daf545a41c6aa0: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_6b60b99917da27d6: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_typed_e09da9dae6ee5935: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__ha3a3f9b66eed9539(a, state0.b, arg0, arg1);
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
        __wbg_new_with_byte_offset_and_length_3cd32055a417f095: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_new_with_length_00818aa0a226a550: function(arg0) {
            const ret = new Uint32Array(arg0 >>> 0);
            return ret;
        },
        __wbg_new_with_length_01b143a7e5dd5a3e: function(arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return ret;
        },
        __wbg_next_178c1651c11432b3: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_next_af5ac6a08bb0a717: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_node_86166ff8ce1f2002: function(arg0) {
            const ret = arg0.node;
            return ret;
        },
        __wbg_of_e7ea0fcd535c1444: function(arg0, arg1) {
            const ret = Array.of(arg0, arg1);
            return ret;
        },
        __wbg_onSubmittedWorkDone_8f9f95b4b9f71221: function(arg0) {
            const ret = arg0.onSubmittedWorkDone();
            return ret;
        },
        __wbg_ownerDocument_adefd47fd596fe80: function(arg0) {
            const ret = arg0.ownerDocument;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_pixelStorei_6dd56b4da047c480: function(arg0, arg1, arg2) {
            arg0.pixelStorei(arg1 >>> 0, arg2);
        },
        __wbg_popErrorScope_af6d4deab74342f1: function(arg0) {
            const ret = arg0.popErrorScope();
            return ret;
        },
        __wbg_process_91f425a63f58e399: function(arg0) {
            const ret = arg0.process;
            return ret;
        },
        __wbg_prototypesetcall_13162965d1fe7aa3: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_pushErrorScope_180992d526be541e: function(arg0, arg1) {
            arg0.pushErrorScope(__wbindgen_enum_GpuErrorFilter[arg1]);
        },
        __wbg_push_c9f4893fc209f768: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_querySelectorAll_6c1bd81d448fb480: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelectorAll(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_queueMicrotask_1e8503a8a690f0a3: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_4a1a4d8dd332373b: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queue_3e1c7fdf7385476f: function(arg0) {
            const ret = arg0.queue;
            return ret;
        },
        __wbg_race_d17769ed306b5623: function(arg0) {
            const ret = Promise.race(arg0);
            return ret;
        },
        __wbg_randomFillSync_34b023059e3363de: function() { return handleError(function (arg0, arg1) {
            arg0.randomFillSync(arg1);
        }, arguments); },
        __wbg_reason_1d74c917c4ff02e5: function(arg0) {
            const ret = arg0.reason;
            return (__wbindgen_enum_GpuDeviceLostReason.indexOf(ret) + 1 || 3) - 1;
        },
        __wbg_rect_ab5601e64afdbc33: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.rect(arg1, arg2, arg3, arg4);
        },
        __wbg_renderbufferStorage_ebe429f577dae949: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
        },
        __wbg_replaceWith_b7c81aac863b2647: function() { return handleError(function (arg0, arg1) {
            arg0.replaceWith(arg1);
        }, arguments); },
        __wbg_requestAdapter_09d626b834f0dd06: function(arg0, arg1) {
            const ret = arg0.requestAdapter(arg1);
            return ret;
        },
        __wbg_requestDevice_d115a8ea9dde8e64: function(arg0, arg1) {
            const ret = arg0.requestDevice(arg1);
            return ret;
        },
        __wbg_require_9a7d8cc2e1d275c0: function() { return handleError(function () {
            const ret = module.require;
            return ret;
        }, arguments); },
        __wbg_resolve_dbf78ab4942478b0: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_restore_f322d9dcee2d3472: function(arg0) {
            arg0.restore();
        },
        __wbg_save_1375b4c2f3596b8b: function(arg0) {
            arg0.save();
        },
        __wbg_setAttribute_f75317b2b5cabfb4: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setBindGroup_7d104141bbef0fe5: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setBindGroup(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
        }, arguments); },
        __wbg_setBindGroup_d2aad0fd44a8cd80: function(arg0, arg1, arg2) {
            arg0.setBindGroup(arg1 >>> 0, arg2);
        },
        __wbg_setIndexBuffer_29ae2c853370cb09: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setIndexBuffer(arg1, __wbindgen_enum_GpuIndexFormat[arg2], arg3, arg4);
        },
        __wbg_setPipeline_7d45d9c71b0eb081: function(arg0, arg1) {
            arg0.setPipeline(arg1);
        },
        __wbg_setTimeout_9ef22b48643bd7de: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setTimeout(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_setVertexBuffer_7f91d33849bcba7b: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3, arg4);
        },
        __wbg_setVertexBuffer_8a99893b536cc564: function(arg0, arg1, arg2, arg3) {
            arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3);
        },
        __wbg_set_118fefd528ab0a79: function(arg0, arg1, arg2) {
            arg0.set(arg1, arg2 >>> 0);
        },
        __wbg_set_2793479888329cb6: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_8282f7e0d2d01336: function(arg0, arg1, arg2) {
            const ret = arg0.set(arg1, arg2);
            return ret;
        },
        __wbg_set_8e5ba6e4c8c3625e: function(arg0, arg1, arg2) {
            arg0.set(getArrayU32FromWasm0(arg1, arg2));
        },
        __wbg_set_a_ea4a959fd18d289d: function(arg0, arg1) {
            arg0.a = arg1;
        },
        __wbg_set_access_0ece114125c5794f: function(arg0, arg1) {
            arg0.access = __wbindgen_enum_GpuStorageTextureAccess[arg1];
        },
        __wbg_set_address_mode_u_af22909f5229fc63: function(arg0, arg1) {
            arg0.addressModeU = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_v_35ac13464ef5d488: function(arg0, arg1) {
            arg0.addressModeV = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_w_9c0db019c6c0c1fe: function(arg0, arg1) {
            arg0.addressModeW = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_alpha_b5a41fecad29ddb4: function(arg0, arg1) {
            arg0.alpha = arg1;
        },
        __wbg_set_alpha_mode_c389474d43539b4e: function(arg0, arg1) {
            arg0.alphaMode = __wbindgen_enum_GpuCanvasAlphaMode[arg1];
        },
        __wbg_set_alpha_to_coverage_enabled_ab0a636d7b352185: function(arg0, arg1) {
            arg0.alphaToCoverageEnabled = arg1 !== 0;
        },
        __wbg_set_array_layer_count_659a875eb924ee18: function(arg0, arg1) {
            arg0.arrayLayerCount = arg1 >>> 0;
        },
        __wbg_set_array_stride_f64_b5d71e363f6dc24c: function(arg0, arg1) {
            arg0.arrayStride = arg1;
        },
        __wbg_set_aspect_3d089d7039fa1c63: function(arg0, arg1) {
            arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_aspect_97098da2440988ca: function(arg0, arg1) {
            arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_attributes_14026731e9652b48: function(arg0, arg1, arg2) {
            arg0.attributes = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_b_f92a64ff7175e178: function(arg0, arg1) {
            arg0.b = arg1;
        },
        __wbg_set_base_array_layer_5a9bc37754e15611: function(arg0, arg1) {
            arg0.baseArrayLayer = arg1 >>> 0;
        },
        __wbg_set_base_mip_level_fb013ee617fc68d5: function(arg0, arg1) {
            arg0.baseMipLevel = arg1 >>> 0;
        },
        __wbg_set_beginning_of_pass_write_index_f0158e5cc0096232: function(arg0, arg1) {
            arg0.beginningOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_bind_group_layouts_ac8c73e7621608bc: function(arg0, arg1, arg2) {
            arg0.bindGroupLayouts = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_binding_28014ee72e7158aa: function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        },
        __wbg_set_binding_a33fbd37c886f242: function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        },
        __wbg_set_blend_a0b73a72e2233549: function(arg0, arg1) {
            arg0.blend = arg1;
        },
        __wbg_set_buffer_820d0f3ee03644f1: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffer_8ef87454e06fc904: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffer_b2122ad1e2284587: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_buffers_ae5966411250cd48: function(arg0, arg1, arg2) {
            arg0.buffers = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_bytes_per_row_74bc161a189080d2: function(arg0, arg1) {
            arg0.bytesPerRow = arg1 >>> 0;
        },
        __wbg_set_bytes_per_row_892db015d53e0ef5: function(arg0, arg1) {
            arg0.bytesPerRow = arg1 >>> 0;
        },
        __wbg_set_className_fcd90345b3a090af: function(arg0, arg1, arg2) {
            arg0.className = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_clear_value_gpu_color_dict_24de3a717c58b77d: function(arg0, arg1) {
            arg0.clearValue = arg1;
        },
        __wbg_set_code_d37f9c44f43f9f12: function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_color_attachments_d849049513579a93: function(arg0, arg1, arg2) {
            arg0.colorAttachments = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_color_ec5d944c7e8d0954: function(arg0, arg1) {
            arg0.color = arg1;
        },
        __wbg_set_compare_a4e8e56b9475077a: function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_compare_ae9b24a7d9467b56: function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_count_ed98ba1bc29a6eb8: function(arg0, arg1) {
            arg0.count = arg1 >>> 0;
        },
        __wbg_set_create_8034c70deebee24f: function(arg0, arg1) {
            arg0.create = arg1 !== 0;
        },
        __wbg_set_cull_mode_b43f074621e3c928: function(arg0, arg1) {
            arg0.cullMode = __wbindgen_enum_GpuCullMode[arg1];
        },
        __wbg_set_d2d33b52458e984a: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_default_queue_de2a9c977a56d718: function(arg0, arg1) {
            arg0.defaultQueue = arg1;
        },
        __wbg_set_depth_bias_ca341c16afef5956: function(arg0, arg1) {
            arg0.depthBias = arg1;
        },
        __wbg_set_depth_bias_clamp_b8af1d74995c9932: function(arg0, arg1) {
            arg0.depthBiasClamp = arg1;
        },
        __wbg_set_depth_bias_slope_scale_653e18432dc545d7: function(arg0, arg1) {
            arg0.depthBiasSlopeScale = arg1;
        },
        __wbg_set_depth_clear_value_ddb071807b691234: function(arg0, arg1) {
            arg0.depthClearValue = arg1;
        },
        __wbg_set_depth_compare_f23e635a19ee9a45: function(arg0, arg1) {
            arg0.depthCompare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_depth_fail_op_5616d153edcb159a: function(arg0, arg1) {
            arg0.depthFailOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_depth_load_op_8a40923b80f10f7d: function(arg0, arg1) {
            arg0.depthLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_depth_or_array_layers_2f6bf8235f6217d5: function(arg0, arg1) {
            arg0.depthOrArrayLayers = arg1 >>> 0;
        },
        __wbg_set_depth_read_only_3dfd2f931d324e52: function(arg0, arg1) {
            arg0.depthReadOnly = arg1 !== 0;
        },
        __wbg_set_depth_stencil_48afa55adf716ca6: function(arg0, arg1) {
            arg0.depthStencil = arg1;
        },
        __wbg_set_depth_stencil_attachment_41d2b2f21af0d222: function(arg0, arg1) {
            arg0.depthStencilAttachment = arg1;
        },
        __wbg_set_depth_store_op_3a43372fef429af9: function(arg0, arg1) {
            arg0.depthStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_depth_write_enabled_c1606fb0ee20d9f3: function(arg0, arg1) {
            arg0.depthWriteEnabled = arg1 !== 0;
        },
        __wbg_set_device_848a10dad582588b: function(arg0, arg1) {
            arg0.device = arg1;
        },
        __wbg_set_dimension_223c5c1d18f3b067: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureDimension[arg1];
        },
        __wbg_set_dimension_b07cced45bdbc27c: function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_dst_factor_8febdc7d58a0332a: function(arg0, arg1) {
            arg0.dstFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_e4b9a833d288759a: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_end_of_pass_write_index_c0e20a7602cdc51a: function(arg0, arg1) {
            arg0.endOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_entries_a16387faec0ed3e0: function(arg0, arg1, arg2) {
            arg0.entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entries_d28c00f0b097605b: function(arg0, arg1, arg2) {
            arg0.entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_5707072adae2af54: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_b17610397f1035e0: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_external_texture_9a20e226761e2a79: function(arg0, arg1) {
            arg0.externalTexture = arg1;
        },
        __wbg_set_fail_op_fd328373012cff8b: function(arg0, arg1) {
            arg0.failOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_feature_level_3a01366061c14f4e: function(arg0, arg1, arg2) {
            arg0.featureLevel = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_feb3714f475485ba: function(arg0, arg1, arg2) {
            arg0.set(getArrayU8FromWasm0(arg1, arg2));
        },
        __wbg_set_fillStyle_94e9d6bece177fdd: function(arg0, arg1) {
            arg0.fillStyle = arg1;
        },
        __wbg_set_fillStyle_95fde67582a9c695: function(arg0, arg1, arg2) {
            arg0.fillStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_font_aa0c0c514c73a5fa: function(arg0, arg1, arg2) {
            arg0.font = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_force_fallback_adapter_3206c1444fbab236: function(arg0, arg1) {
            arg0.forceFallbackAdapter = arg1 !== 0;
        },
        __wbg_set_format_5b658a7caee1f7c1: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuVertexFormat[arg1];
        },
        __wbg_set_format_7d6ca9e37cef68cf: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_7e9c061d793e5516: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_94b52d5278729194: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_9700503bed39b977: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_ae55814a64293a35: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_e4c737303bcbe058: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_fragment_7b1b57b395dbc0de: function(arg0, arg1) {
            arg0.fragment = arg1;
        },
        __wbg_set_front_face_7203fe137251675e: function(arg0, arg1) {
            arg0.frontFace = __wbindgen_enum_GpuFrontFace[arg1];
        },
        __wbg_set_g_787ebdfe9b4d0085: function(arg0, arg1) {
            arg0.g = arg1;
        },
        __wbg_set_has_dynamic_offset_00514612ac180079: function(arg0, arg1) {
            arg0.hasDynamicOffset = arg1 !== 0;
        },
        __wbg_set_height_ae0610d318c3428c: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_height_ae24c2f7d580ae0a: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_height_ee8c43ef1c84fd25: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_id_861bcd7afe9f86ad: function(arg0, arg1, arg2) {
            arg0.id = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_innerHTML_a9d3a267967b5117: function(arg0, arg1, arg2) {
            arg0.innerHTML = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_2d1cd1297bc401d8: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_51e8956942b48a12: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_68d94fd04fb78dee: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_7585d1d8ce51c4f6: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_80faaf167de3ab54: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_8cacfce97db5d34b: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_91a9bba3686a3cc6: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_a316a181df3601a8: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_afe7a2a106f8c390: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_bf6c7afe7dc5ec5e: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_dbd85794a8ca7cab: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_e00027fcd4b96118: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_e6506279e85c8277: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_e809c026ff8788e0: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_layout_51ca8922df056df0: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_79d50dfd04559146: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_gpu_auto_layout_mode_2f3f2a020003c8cd: function(arg0, arg1) {
            arg0.layout = __wbindgen_enum_GpuAutoLayoutMode[arg1];
        },
        __wbg_set_lineWidth_12f039d09cd36dad: function(arg0, arg1) {
            arg0.lineWidth = arg1;
        },
        __wbg_set_load_op_c256cc6384707edf: function(arg0, arg1) {
            arg0.loadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_lod_max_clamp_b33ca1a16ff0d031: function(arg0, arg1) {
            arg0.lodMaxClamp = arg1;
        },
        __wbg_set_lod_min_clamp_e881875c83cc6874: function(arg0, arg1) {
            arg0.lodMinClamp = arg1;
        },
        __wbg_set_mag_filter_47cc9a3f99e95935: function(arg0, arg1) {
            arg0.magFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mapped_at_creation_63cad28156727b4c: function(arg0, arg1) {
            arg0.mappedAtCreation = arg1 !== 0;
        },
        __wbg_set_mask_155ce32c67d531d4: function(arg0, arg1) {
            arg0.mask = arg1 >>> 0;
        },
        __wbg_set_max_anisotropy_32a9257b899ae96e: function(arg0, arg1) {
            arg0.maxAnisotropy = arg1;
        },
        __wbg_set_min_binding_size_f64_4acca004d3333ab2: function(arg0, arg1) {
            arg0.minBindingSize = arg1;
        },
        __wbg_set_min_filter_af1dfd0ac8ded0cc: function(arg0, arg1) {
            arg0.minFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mip_level_count_45190d3a6ff8e1f3: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mip_level_count_ed4622b4951515bb: function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mip_level_ecd94d7dee8fb526: function(arg0, arg1) {
            arg0.mipLevel = arg1 >>> 0;
        },
        __wbg_set_mipmap_filter_c4fab694b02f7b12: function(arg0, arg1) {
            arg0.mipmapFilter = __wbindgen_enum_GpuMipmapFilterMode[arg1];
        },
        __wbg_set_mode_95307665cde0c8a4: function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_GpuCanvasToneMappingMode[arg1];
        },
        __wbg_set_module_2458f4391f83fba8: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_module_b61332909ccceeeb: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_multisample_82275640b1e6506e: function(arg0, arg1) {
            arg0.multisample = arg1;
        },
        __wbg_set_multisampled_6f28d8d622d12d88: function(arg0, arg1) {
            arg0.multisampled = arg1 !== 0;
        },
        __wbg_set_offset_f64_30b85e5787c0c8d3: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_offset_f64_6053d969149c9a6b: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_offset_f64_a0a3ff72c1ce838e: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_offset_f64_a2cf3ac2d0e7e263: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_operation_daf6679f4c31226c: function(arg0, arg1) {
            arg0.operation = __wbindgen_enum_GpuBlendOperation[arg1];
        },
        __wbg_set_origin_gpu_origin_3d_dict_d0bd479de81f7eb4: function(arg0, arg1) {
            arg0.origin = arg1;
        },
        __wbg_set_pass_op_37a103362aa6b624: function(arg0, arg1) {
            arg0.passOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_power_preference_10b2912b532591cf: function(arg0, arg1) {
            arg0.powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
        },
        __wbg_set_primitive_9fc275881b0be5bd: function(arg0, arg1) {
            arg0.primitive = arg1;
        },
        __wbg_set_query_set_d90ced8d33771d3c: function(arg0, arg1) {
            arg0.querySet = arg1;
        },
        __wbg_set_r_904ca14b72b0a5ee: function(arg0, arg1) {
            arg0.r = arg1;
        },
        __wbg_set_required_features_bec2da1f0fcdc5a9: function(arg0, arg1, arg2) {
            arg0.requiredFeatures = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_required_limits_cda922cd350d55ca: function(arg0, arg1) {
            arg0.requiredLimits = arg1;
        },
        __wbg_set_resolve_target_gpu_texture_view_1771cafa778853e0: function(arg0, arg1) {
            arg0.resolveTarget = arg1;
        },
        __wbg_set_resource_db7fb1a2b8ffa209: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_buffer_binding_6d753e998a35a8e0: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_external_texture_94bc0cda734e1bb0: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_texture_view_d08374930cdbe8e2: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_rows_per_image_ab5ad5d35299a4b9: function(arg0, arg1) {
            arg0.rowsPerImage = arg1 >>> 0;
        },
        __wbg_set_rows_per_image_cefa0e1fd2c32956: function(arg0, arg1) {
            arg0.rowsPerImage = arg1 >>> 0;
        },
        __wbg_set_sample_count_d5695c7e94abf1b2: function(arg0, arg1) {
            arg0.sampleCount = arg1 >>> 0;
        },
        __wbg_set_sample_type_58b8861ddf0e65ce: function(arg0, arg1) {
            arg0.sampleType = __wbindgen_enum_GpuTextureSampleType[arg1];
        },
        __wbg_set_sampler_c2ae5f08d776390c: function(arg0, arg1) {
            arg0.sampler = arg1;
        },
        __wbg_set_shader_location_5e3589b64e229101: function(arg0, arg1) {
            arg0.shaderLocation = arg1 >>> 0;
        },
        __wbg_set_size_f64_1415e8313b3fe2ee: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_f64_2d61d550c44e9a4f: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_gpu_extent_3d_dict_42a13d86373a1828: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_src_factor_11ae58769201ab3c: function(arg0, arg1) {
            arg0.srcFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_stencil_back_1a61453cc13b6c48: function(arg0, arg1) {
            arg0.stencilBack = arg1;
        },
        __wbg_set_stencil_clear_value_7a1c747e15b9e640: function(arg0, arg1) {
            arg0.stencilClearValue = arg1 >>> 0;
        },
        __wbg_set_stencil_front_d0181e029615c7b8: function(arg0, arg1) {
            arg0.stencilFront = arg1;
        },
        __wbg_set_stencil_load_op_dccf6fe419ac65a2: function(arg0, arg1) {
            arg0.stencilLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_stencil_read_mask_8509525963a4e06c: function(arg0, arg1) {
            arg0.stencilReadMask = arg1 >>> 0;
        },
        __wbg_set_stencil_read_only_ea444db92316f567: function(arg0, arg1) {
            arg0.stencilReadOnly = arg1 !== 0;
        },
        __wbg_set_stencil_store_op_db12f32a354927a7: function(arg0, arg1) {
            arg0.stencilStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_stencil_write_mask_43a3f8ba64d04f18: function(arg0, arg1) {
            arg0.stencilWriteMask = arg1 >>> 0;
        },
        __wbg_set_step_mode_c40c2a3fc1f5e262: function(arg0, arg1) {
            arg0.stepMode = __wbindgen_enum_GpuVertexStepMode[arg1];
        },
        __wbg_set_storage_texture_a2c5cd99046c43db: function(arg0, arg1) {
            arg0.storageTexture = arg1;
        },
        __wbg_set_store_op_8e12a492b99172ec: function(arg0, arg1) {
            arg0.storeOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_strip_index_format_1fba573c4c01ac83: function(arg0, arg1) {
            arg0.stripIndexFormat = __wbindgen_enum_GpuIndexFormat[arg1];
        },
        __wbg_set_strokeStyle_b90c8a6e0d37c80d: function(arg0, arg1, arg2) {
            arg0.strokeStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_swizzle_9bdf23260a6cd663: function(arg0, arg1, arg2) {
            arg0.swizzle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_targets_d7d0161f1f39b556: function(arg0, arg1, arg2) {
            arg0.targets = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_textAlign_0a32c51401892c11: function(arg0, arg1, arg2) {
            arg0.textAlign = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textBaseline_ff2e3c5c84684b23: function(arg0, arg1, arg2) {
            arg0.textBaseline = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textContent_0177b2a6d9293c2c: function(arg0, arg1, arg2) {
            arg0.textContent = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_texture_4480ca078086da3c: function(arg0, arg1) {
            arg0.texture = arg1;
        },
        __wbg_set_texture_de8c015102b86032: function(arg0, arg1) {
            arg0.texture = arg1;
        },
        __wbg_set_timestamp_writes_b9d5f4929c0cb214: function(arg0, arg1) {
            arg0.timestampWrites = arg1;
        },
        __wbg_set_tone_mapping_ea414c22e43b1107: function(arg0, arg1) {
            arg0.toneMapping = arg1;
        },
        __wbg_set_topology_86b6620c50ca71b9: function(arg0, arg1) {
            arg0.topology = __wbindgen_enum_GpuPrimitiveTopology[arg1];
        },
        __wbg_set_type_b5b70f5d8c511020: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_GpuSamplerBindingType[arg1];
        },
        __wbg_set_type_e7e6baf2ea3bc57c: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_GpuBufferBindingType[arg1];
        },
        __wbg_set_unclipped_depth_9b2fb43ab2bbf4d9: function(arg0, arg1) {
            arg0.unclippedDepth = arg1 !== 0;
        },
        __wbg_set_usage_4faa2c842e506b0a: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_6f796a3e58aa9a92: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_a93520f1e6b8bb04: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_usage_f510bb60dcbe5d64: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_set_vertex_0508955120fad8b0: function(arg0, arg1) {
            arg0.vertex = arg1;
        },
        __wbg_set_view_dimension_3f9fa8b1ec4f7fca: function(arg0, arg1) {
            arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_view_dimension_cfb4383dbe28f193: function(arg0, arg1) {
            arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_view_formats_2f2ed172e2ff6ad0: function(arg0, arg1, arg2) {
            arg0.viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_formats_ae29cd4032270cfe: function(arg0, arg1, arg2) {
            arg0.viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_gpu_texture_view_0cd77e7b1eeac111: function(arg0, arg1) {
            arg0.view = arg1;
        },
        __wbg_set_view_gpu_texture_view_48ed262e8227739b: function(arg0, arg1) {
            arg0.view = arg1;
        },
        __wbg_set_visibility_ae0dae141a8e919f: function(arg0, arg1) {
            arg0.visibility = arg1 >>> 0;
        },
        __wbg_set_width_437a2b9dd43d467c: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_width_5f89a551b5472887: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_width_eb3001e1dcabb909: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_write_mask_1c9e8cc7a9366eeb: function(arg0, arg1) {
            arg0.writeMask = arg1 >>> 0;
        },
        __wbg_set_x_ac17ff4489cc5a7c: function(arg0, arg1) {
            arg0.x = arg1 >>> 0;
        },
        __wbg_set_y_18f83bab82640d86: function(arg0, arg1) {
            arg0.y = arg1 >>> 0;
        },
        __wbg_set_z_6f9933ce16bc0016: function(arg0, arg1) {
            arg0.z = arg1 >>> 0;
        },
        __wbg_shaderSource_519d0f56b613b2f9: function(arg0, arg1, arg2, arg3) {
            arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
        },
        __wbg_stack_575c4e0918581b9c: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_THIS_003ee833af9699fc: function() {
            const ret = typeof globalThis === 'undefined' ? undefined : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_e4293fb22cbe77da: function() {
            const ret = typeof global === 'undefined' ? undefined : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_53bb20457401453d: function() {
            const ret = typeof self === 'undefined' ? undefined : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_b5e0810bfecb7feb: function() {
            const ret = typeof window === 'undefined' ? undefined : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_storage_0cfb7cf2661c6520: function(arg0) {
            const ret = arg0.storage;
            return ret;
        },
        __wbg_strokeRect_92ff808375332184: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.strokeRect(arg1, arg2, arg3, arg4);
        },
        __wbg_stroke_19fa3f3ec093bc25: function(arg0) {
            arg0.stroke();
        },
        __wbg_subarray_58f388a9247ac0bb: function(arg0, arg1, arg2) {
            const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_subgroupMaxSize_9e249d50db296bba: function(arg0) {
            const ret = arg0.subgroupMaxSize;
            return ret;
        },
        __wbg_subgroupMinSize_0f1a89078d56a967: function(arg0) {
            const ret = arg0.subgroupMinSize;
            return ret;
        },
        __wbg_submit_f710a7f5b0e620de: function(arg0, arg1, arg2) {
            arg0.submit(getArrayJsValueViewFromWasm0(arg1, arg2));
        },
        __wbg_texImage2D_170b4cef4a56a388: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9 === 0 ? undefined : getArrayU8FromWasm0(arg9, arg10));
        }, arguments); },
        __wbg_texImage2D_5048d2cd8540da16: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texParameteri_e24e4fdc17153838: function(arg0, arg1, arg2, arg3) {
            arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        },
        __wbg_then_3a4f899454bcf8c9: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_then_656f253a5dd7d8ea: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_e3f7f2d365708be8: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_toString_55ffdf586349746b: function(arg0) {
            const ret = arg0.toString();
            return ret;
        },
        __wbg_uniform1f_fe417acc7c7a1dc5: function(arg0, arg1, arg2) {
            arg0.uniform1f(arg1, arg2);
        },
        __wbg_uniform1i_778a02a48ecdceed: function(arg0, arg1, arg2) {
            arg0.uniform1i(arg1, arg2);
        },
        __wbg_uniform2f_7e4c627c4106dd3f: function(arg0, arg1, arg2, arg3) {
            arg0.uniform2f(arg1, arg2, arg3);
        },
        __wbg_uniform3f_6f94179d13a10f48: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniform3f(arg1, arg2, arg3, arg4);
        },
        __wbg_uniformMatrix4fv_edd2b51dbb1caffd: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_unmap_c064a33ee7576b18: function(arg0) {
            arg0.unmap();
        },
        __wbg_useProgram_d9c24a7d8632c4c1: function(arg0, arg1) {
            arg0.useProgram(arg1);
        },
        __wbg_value_e0f328965e184931: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_values_d0f8968aef1ef9e1: function(arg0) {
            const ret = arg0.values();
            return ret;
        },
        __wbg_vendor_59f443cba4c5f18f: function(arg0, arg1) {
            const ret = arg1.vendor;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_versions_6e128a981b0df859: function(arg0) {
            const ret = arg0.versions;
            return ret;
        },
        __wbg_vertexAttribPointer_39b706eb26c50057: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        },
        __wbg_viewport_cdae9e8b2f33343d: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.viewport(arg1, arg2, arg3, arg4);
        },
        __wbg_width_babc5dc7654771b1: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbg_writeBuffer_c7a4215b00b9b970: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.writeBuffer(arg1, arg2, getArrayU8FromWasm0(arg3, arg4), arg5, arg6);
        }, arguments); },
        __wbg_writeTexture_3501ab09794c59a1: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.writeTexture(arg1, getArrayU8FromWasm0(arg2, arg3), arg4, arg5);
        }, arguments); },
        __wbg_write_69758b16c22f8278: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.write(getArrayU8FromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 2569, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h7b949ab170621146);
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUDevice")], shim_idx: 2142, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860);
            return ret;
        },
        __wbindgen_generic_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUDeviceLostInfo")], shim_idx: 2141, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h91c9f44592f0dd1a);
            return ret;
        },
        __wbindgen_generic_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("any")], shim_idx: 2142, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860_414);
            return ret;
        },
        __wbindgen_generic_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("undefined")], shim_idx: 2142, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860_415);
            return ret;
        },
        __wbindgen_generic_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 921, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__heca99f998697a495);
            return ret;
        },
        __wbindgen_generic_0000000000000007: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000008: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000009: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
            const ret = getArrayF32FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_000000000000000a: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U32)) -> NamedExternref("Uint32Array")`.
            const ret = getArrayU32FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_000000000000000b: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_000000000000000c: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_000000000000000d: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            table.set(0, undefined);
            table.set(1024, undefined);
            table.set(1025, null);
            table.set(1026, true);
            table.set(1027, false);
        },
    };
    return {
        __proto__: null,
        "./qualia_bg.js": import0,
    };
}

function wasm_bindgen__convert__closures_____invoke__heca99f998697a495(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__heca99f998697a495(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__h91c9f44592f0dd1a(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h91c9f44592f0dd1a(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h7b949ab170621146(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h7b949ab170621146(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860_414(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860_414(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860_415(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h38e7e642ab627860_415(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__ha3a3f9b66eed9539(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__ha3a3f9b66eed9539(arg0, arg1, arg2, arg3);
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


const __wbindgen_enum_GpuDeviceLostReason = ["unknown", "destroyed"];


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
const QualiaHudFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_qualiahud_free(ptr, 1));
const QualiaPortalFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_qualiaportal_free(ptr, 1));
const WasmOffloadIntentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmoffloadintent_free(ptr, 1));
const WasmQ42SessionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmq42session_free(ptr, 1));
const WasmSimulationWorldFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsimulationworld_free(ptr, 1));
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
    switch (typeof val) {
        // primitive types
        case 'number':
        case 'boolean':
        case 'undefined':
        return `${val}`;
        case 'string':
        return JSON.stringify(val);
        case 'bigint':
        return `${val}n`;
        case 'symbol':
        return val.description !== undefined
            ? `Symbol(${val.description})`
            : 'Symbol';
        case 'function':
        return typeof val.name === 'string' && val.name !== ''
            ? `Function(${val.name})`
            : 'Function';
        case 'object': {
            if (val === null) {
                return 'null';
            }
            // objects
            if (Array.isArray(val)) {
                return '[' + Array.from(val, debugString).join(', ') + ']';
            }
            // errors
            if (val instanceof Error) {
                return `${val.name}: ${val.message}\n${val.stack}`;
            }

            const toStringOutput = toString.call(val);
            // Test for built-in
            const className = /\[object ([^\]]+)\]/.exec(toStringOutput)?.[1];

            // Failed to match the standard '[object ClassName]'
            if (className === undefined) {
                return toStringOutput;
            }
            if (className === 'Object') {
                // we're a user defined class or Object
                // JSON.stringify avoids problems with cycles, and is generally much
                // easier than looping through ownProperties of `val`.
                try {
                    return 'Object(' + JSON.stringify(val) + ')';
                } catch (_) {
                    return 'Object';
                }
            }
            // TODO we could test for more things here, like `Set`s and `Map`s.
            return className;
        }
        // Per MDN: 'This list of values is exhaustive.'
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/typeof#description
    }
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
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

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(getUint8ArrayMemory0().subarray(ptr, ptr + len));
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

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
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

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(bytes) {
    numBytesDecoded += bytes.length;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
        cachedTextDecoder.decode();
        numBytesDecoded = bytes.length;
    }
    return cachedTextDecoder.decode(bytes);
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
    cachedInt32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

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

let __wbg_init_promise;
async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;
    if (__wbg_init_promise === undefined) {
        __wbg_init_promise = __wbg_init_once(module_or_path);
        __wbg_init_promise.catch(() => { __wbg_init_promise = undefined; });
    }
    return __wbg_init_promise;
}

async function __wbg_init_once(module_or_path) {

    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('qualia_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
