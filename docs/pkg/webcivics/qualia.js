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
 * Build a Solid leave/migrate bundle (Turtle / N-Quads / JSON-LD / ACL / manifest).
 *
 * Requires `sanctuary_choice`: `omit-sanctuary` | `reclassify-for-solid` when
 * sanctuary data is present (`Unset` fails closed). Classified never egresses.
 * Advertised on the `wasm-webcivics` profile as `solid-leave-migrate`.
 * @param {any} val
 * @returns {any}
 */
export function export_solid_migration_wasm(val) {
    const ret = wasm.export_solid_migration_wasm(val);
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
 * Solid RDF Sources → Qualia `webcivics.vault-backup.v1` JSON (return path).
 * @param {any} val
 * @returns {any}
 */
export function import_solid_to_qualia_backup_wasm(val) {
    const ret = wasm.import_solid_to_qualia_backup_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * Package exposure receipt: context + shapes + vibe AST digests (UE-053).
 * @param {string} shapes_json
 * @param {Uint8Array | null} [vibe_program_cbor]
 * @returns {any}
 */
export function package_exposure_manifest_wasm(shapes_json, vibe_program_cbor) {
    const ptr0 = passStringToWasm0(shapes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    var ptr1 = isLikeNone(vibe_program_cbor) ? 0 : passArray8ToWasm0(vibe_program_cbor, wasm.__wbindgen_malloc);
    var len1 = WASM_VECTOR_LEN;
    const ret = wasm.package_exposure_manifest_wasm(ptr0, len0, ptr1, len1);
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
    const ptr0 = passStringToWasm0(content_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.parse_rdf_document_wasm(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * Pre-flight sanctuary notice for leave UI (desktop / Databox / PWA).
 * Advertised on `wasm-webcivics` as part of `solid-leave-migrate`.
 * @param {any} val
 * @returns {any}
 */
export function plan_sanctuary_migration_wasm(val) {
    const ret = wasm.plan_sanctuary_migration_wasm(val);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
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
 * Bounded stride sample of packed 48-byte Quins. Browser graphs cannot mmap
 * `.q42` files; this is the WASM-safe equivalent of `mmap_sample_quins`.
 * @param {Uint8Array} db_bytes
 * @param {number} max_quins
 * @returns {Uint8Array}
 */
export function sample_packed_quins_wasm(db_bytes, max_quins) {
    const ptr0 = passArray8ToWasm0(db_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.sample_packed_quins_wasm(ptr0, len0, max_quins);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v2;
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
 * @param {Uint8Array} db_bytes
 * @param {string} shapes_json
 * @returns {any}
 */
export function validate_shacl_graph_wasm(db_bytes, shapes_json) {
    const ptr0 = passArray8ToWasm0(db_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(shapes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
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
    const ptr0 = passStringToWasm0(data_n3, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(shapes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
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
 * @param {Uint8Array} public_key
 * @returns {boolean}
 */
export function verify_law_package_wasm(json, public_key) {
    const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(public_key, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
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
        __wbg_arrayBuffer_e3174a1300c67c95: function(arg0) {
            const ret = arg0.arrayBuffer();
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
        __wbg_createWritable_fe536097cf251da6: function(arg0) {
            const ret = arg0.createWritable();
            return ret;
        },
        __wbg_done_b62d4a7d2286852a: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_entries_c261c3fa1f281256: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_estimate_1b62d27c90cb9fd8: function() { return handleError(function (arg0) {
            const ret = arg0.estimate();
            return ret;
        }, arguments); },
        __wbg_getDirectory_d73e4f2473279f77: function(arg0) {
            const ret = arg0.getDirectory();
            return ret;
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
        __wbg_getRandomValues_cc7f052a444bb2ce: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_get_197a3fe98f169e38: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
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
        __wbg_isSafeInteger_a3389a198582f5f6: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_cc47ba25a2be735a: function() {
            const ret = Symbol.iterator;
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
        __wbg_navigator_935098efd1dc7fe5: function(arg0) {
            const ret = arg0.navigator;
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
        __wbg_new_81880fb5002cb255: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_from_slice_543b875b27789a8f: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
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
        __wbg_next_0c4066e251d2eff9: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_next_402fa10b59ab20c3: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_now_d2e0afbad4edbe82: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_prototypesetcall_d721637c7ca66eb8: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_queueMicrotask_1c9b3800e321a967: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_311744e534a929a3: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_resolve_d82363d90af6928a: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_set_4564f7dc44fcb0c9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_9a1d61e17de7054c: function(arg0, arg1, arg2) {
            const ret = arg0.set(arg1, arg2);
            return ret;
        },
        __wbg_set_create_b9be7a200245a2da: function(arg0, arg1) {
            arg0.create = arg1 !== 0;
        },
        __wbg_set_dc601f4a69da0bc2: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
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
        __wbg_then_05edfc8a4fea5106: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_591b6b3a75ee817a: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_value_49f783bb59765962: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_write_4dde130ecd70a0b5: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.write(getArrayU8FromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 1072, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h8803f8c799f93ab4);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0) {
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

function wasm_bindgen__convert__closures_____invoke__h243b5e59773a58aa(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h243b5e59773a58aa(arg0, arg1, arg2, arg3);
}

const FederatedNodeManagerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_federatednodemanager_free(ptr, 1));
const WasmOffloadIntentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmoffloadintent_free(ptr, 1));

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

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
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
        module_or_path = new URL('qualia_webcivics_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
