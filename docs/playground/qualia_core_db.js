export class AgentIntent {
    static __wrap(ptr) {
        const obj = Object.create(AgentIntent.prototype);
        obj.__wbg_ptr = ptr;
        AgentIntentFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        AgentIntentFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_agentintent_free(ptr, 0);
    }
    /**
     * @param {number} opcode
     * @param {number} priority
     * @param {number} payload_size
     */
    constructor(opcode, priority, payload_size) {
        const ret = wasm.agentintent_new(opcode, priority, payload_size);
        this.__wbg_ptr = ret;
        AgentIntentFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {number} opcode
     * @param {number} priority
     * @param {string} payload
     * @returns {AgentIntent}
     */
    static with_string_payload(opcode, priority, payload) {
        const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.agentintent_with_string_payload(opcode, priority, ptr0, len0);
        return AgentIntent.__wrap(ret);
    }
    /**
     * @returns {number}
     */
    get opcode() {
        const ret = wasm.__wbg_get_agentintent_opcode(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get payload_size() {
        const ret = wasm.__wbg_get_agentintent_payload_size(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get priority() {
        const ret = wasm.__wbg_get_agentintent_priority(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} arg0
     */
    set opcode(arg0) {
        wasm.__wbg_set_agentintent_opcode(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set payload_size(arg0) {
        wasm.__wbg_set_agentintent_payload_size(this.__wbg_ptr, arg0);
    }
    /**
     * @param {number} arg0
     */
    set priority(arg0) {
        wasm.__wbg_set_agentintent_priority(this.__wbg_ptr, arg0);
    }
}
if (Symbol.dispose) AgentIntent.prototype[Symbol.dispose] = AgentIntent.prototype.free;

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
     * @param {AgentIntent} intent
     * @returns {string}
     */
    offload_intent(intent) {
        let deferred2_0;
        let deferred2_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(intent, AgentIntent);
            wasm.federatednodemanager_offload_intent(retptr, this.__wbg_ptr, intent.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
            var ptr1 = r0;
            var len1 = r1;
            if (r3) {
                ptr1 = 0; len1 = 0;
                throw takeObject(r2);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export3(deferred2_0, deferred2_1, 1);
        }
    }
}
if (Symbol.dispose) FederatedNodeManager.prototype[Symbol.dispose] = FederatedNodeManager.prototype.free;

export class QualiaWasmBridge {
    static __wrap(ptr) {
        const obj = Object.create(QualiaWasmBridge.prototype);
        obj.__wbg_ptr = ptr;
        QualiaWasmBridgeFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        QualiaWasmBridgeFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_qualiawasmbridge_free(ptr, 0);
    }
    /**
     * Dispatches a query over the SharedArrayBuffer.
     * The offset points to the byte location in the SharedArrayBuffer where the engine should
     * write the zero-allocation raw result pointers for the Main UI thread to read synchronously.
     * @param {number} buffer_offset
     * @param {bigint} _query_id
     */
    dispatch_query(buffer_offset, _query_id) {
        wasm.qualiawasmbridge_dispatch_query(this.__wbg_ptr, buffer_offset, _query_id);
    }
    /**
     * @param {WebAssembly.Memory} memory
     */
    constructor(memory) {
        const ret = wasm.qualiawasmbridge_new(addHeapObject(memory));
        this.__wbg_ptr = ret;
        QualiaWasmBridgeFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Initializes the Qualia-DB engine on the WebWorker thread.
     * Expects the WebAssembly.Memory object to be pre-allocated to exactly 512MB.
     * @param {WebAssembly.Memory} memory
     * @returns {QualiaWasmBridge}
     */
    static qualia_init(memory) {
        const ret = wasm.qualiawasmbridge_qualia_init(addHeapObject(memory));
        return QualiaWasmBridge.__wrap(ret);
    }
}
if (Symbol.dispose) QualiaWasmBridge.prototype[Symbol.dispose] = QualiaWasmBridge.prototype.free;

/**
 * @param {string} query
 * @returns {string}
 */
export function compile_query_to_json(query) {
    let deferred2_0;
    let deferred2_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.compile_query_to_json(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred2_0 = r0;
        deferred2_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export3(deferred2_0, deferred2_1, 1);
    }
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
 * Execute a single N-Triples pattern query against a pre-loaded SuperBlock.
 *
 * # Arguments
 * * `query`    — N-Triples pattern, e.g. `?s <http://…/writtenRep> "dog"@en`
 * * `db_bytes` — raw bytes of one or more 40,960-byte SuperBlocks.  The JS
 *               caller passes a `Uint8Array` view; wasm-bindgen copies it in.
 * * `max_results` — upper bound on the Quins returned (default: 256).
 *
 * # Returns
 * A JSON string:
 * ```json
 * {
 *   "matches": [{"s":"u64str","p":"u64str","o":"u64str","c":"u64str","m":"u64str"}, …],
 *   "vm_cycles": 1234,
 *   "direct_jump_ops": 0,
 *   "lexicon_lookup_ops": 6
 * }
 * ```
 * All u64 field values are serialised as **decimal strings** so the JS side
 * can parse them losslessly with `BigInt(v)` without IEEE-754 truncation.
 *
 * On error a `{"error":"..."}` object is returned instead.
 * @param {string} query
 * @param {Uint8Array} db_bytes
 * @param {number} max_results
 * @returns {string}
 */
export function execute_ntriples_query(query, db_bytes, max_results) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(db_bytes, wasm.__wbindgen_export);
        const len1 = WASM_VECTOR_LEN;
        wasm.execute_ntriples_query(retptr, ptr0, len0, ptr1, len1, max_results);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred3_0 = r0;
        deferred3_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export3(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Intercepts heavy computational opcodes and constructs an AgentIntent to offload them
 * @param {number} opcode
 * @param {number} payload_size
 * @returns {AgentIntent | undefined}
 */
export function intercept_computational_opcode(opcode, payload_size) {
    const ret = wasm.intercept_computational_opcode(opcode, payload_size);
    return ret === 0 ? undefined : AgentIntent.__wrap(ret);
}

/**
 * @param {string} smiles
 * @returns {AgentIntent}
 */
export function intercept_pharmacogenomics_intent(smiles) {
    const ptr0 = passStringToWasm0(smiles, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.intercept_pharmacogenomics_intent(ptr0, len0);
    return AgentIntent.__wrap(ret);
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
 * Continuous Mathematical Serialization into Float64Array
 * @param {Float64Array} data
 * @returns {Float64Array}
 */
export function serialize_float64_array(data) {
    const ptr0 = passArrayF64ToWasm0(data, wasm.__wbindgen_export);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.serialize_float64_array(ptr0, len0);
    return takeObject(ret);
}

/**
 * Packs an array of floats into a Uint8Array strictly typed buffer to avoid IEEE-754 truncation
 * @param {Float32Array} data
 * @returns {Uint8Array}
 */
export function serialize_float_array(data) {
    const ptr0 = passArrayF32ToWasm0(data, wasm.__wbindgen_export);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.serialize_float_array(ptr0, len0);
    return takeObject(ret);
}

/**
 * Polls the local Webizen for pending agreements waiting for the user's signature.
 * @returns {string}
 */
export function webizen_poll_agreements() {
    let deferred1_0;
    let deferred1_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.webizen_poll_agreements(retptr);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred1_0 = r0;
        deferred1_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export3(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Proposes a new M:N Guardianship agreement to the local WebRTC mesh.
 * @param {Array<any>} nominated_guardians
 * @param {string} principal
 * @param {string} domain
 * @param {number} threshold
 * @returns {bigint}
 */
export function webizen_propose_agreement(nominated_guardians, principal, domain, threshold) {
    const ptr0 = passStringToWasm0(principal, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(domain, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.webizen_propose_agreement(addHeapObject(nominated_guardians), ptr0, len0, ptr1, len1, threshold);
    return BigInt.asUintN(64, ret);
}

/**
 * Signs a pending agreement, advancing its state machine and triggering WebRTC peer sync.
 * @param {bigint} agreement_id
 * @param {string} _private_key_mock
 */
export function webizen_sign_agreement(agreement_id, _private_key_mock) {
    const ptr0 = passStringToWasm0(_private_key_mock, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len0 = WASM_VECTOR_LEN;
    wasm.webizen_sign_agreement(agreement_id, ptr0, len0);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_1506f2235d1bdba0: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_buffer_a1f116eb4fdb1531: function(arg0) {
            const ret = getObject(arg0).buffer;
            return addHeapObject(ret);
        },
        __wbg_new_from_slice_18fa1f71286d66b8: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return addHeapObject(ret);
        },
        __wbg_new_from_slice_3c93d0bc613de8f0: function(arg0, arg1) {
            const ret = new Float64Array(getArrayF64FromWasm0(arg0, arg1));
            return addHeapObject(ret);
        },
        __wbg_new_with_byte_offset_and_length_281228aa9c9441ef: function(arg0, arg1, arg2) {
            const ret = new Uint32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_set_index_88b4ef962117fc1b: function(arg0, arg1, arg2) {
            getObject(arg0)[arg1 >>> 0] = arg2 >>> 0;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_object_drop_ref: function(arg0) {
            takeObject(arg0);
        },
    };
    return {
        __proto__: null,
        "./qualia_core_db_bg.js": import0,
    };
}

const AgentIntentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_agentintent_free(ptr, 1));
const FederatedNodeManagerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_federatednodemanager_free(ptr, 1));
const QualiaWasmBridgeFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_qualiawasmbridge_free(ptr, 1));

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
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

function getObject(idx) { return heap[idx]; }

let heap = new Array(1024).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

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

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
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
