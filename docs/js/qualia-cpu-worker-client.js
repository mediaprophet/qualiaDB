const WORKER_URL = new URL('./qualia-cpu-worker.js?v=0.0.40-mobile-performance1', import.meta.url);
import { EtalonWorkerTransport } from './etalon-worker-transport.js';

function transferableModelBuffer(modelBytes) {
    if (modelBytes instanceof ArrayBuffer) return modelBytes;
    if (!ArrayBuffer.isView(modelBytes)) {
        throw new TypeError('Qualia CPU worker requires an ArrayBuffer or typed array');
    }
    if (
        modelBytes.byteOffset === 0 &&
        modelBytes.byteLength === modelBytes.buffer.byteLength &&
        modelBytes.buffer instanceof ArrayBuffer
    ) {
        return modelBytes.buffer;
    }
    return modelBytes.buffer.slice(modelBytes.byteOffset, modelBytes.byteOffset + modelBytes.byteLength);
}

export class QualiaCpuWorkerClient {
    #worker = null;
    #transport = null;
    #nextId = 1;
    #pending = new Map();
    #ready = false;
    #details = null;
    #modelByteBudget;
    #developerMode;

    constructor({ modelByteBudget = 1024 * 1024 * 1024, developerMode = false } = {}) {
        if (!Number.isSafeInteger(modelByteBudget) || modelByteBudget < 1) {
            throw new Error('CPU worker model budget must be a positive safe integer');
        }
        this.#modelByteBudget = modelByteBudget;
        this.#developerMode = developerMode === true;
    }

    get ready() {
        return this.#ready;
    }

    get details() {
        return this.#details;
    }

    #ensureWorker() {
        if (this.#worker) return this.#worker;
        const worker = new Worker(WORKER_URL, { type: 'module', name: 'qualia-cpu-wasm' });
        this.#transport = new EtalonWorkerTransport(worker, (data) => {
            const pending = this.#pending.get(data?.id);
            if (!pending) return;
            if (data.type === 'token') {
                pending.onToken?.(String(data.piece || ''));
                return;
            }
            this.#pending.delete(data.id);
            if (data.type === 'result') pending.resolve(data.value);
            else pending.reject(new Error(data.error || 'CPU worker request failed'));
        }, true, this.#modelByteBudget);
        worker.onmessage = ({ data }) => {
            if (this.#worker === worker) this.#transport?.handle(data);
        };
        const fail = (event) => {
            if (this.#worker !== worker) return;
            const error = new Error(event.message || 'CPU worker crashed');
            for (const pending of this.#pending.values()) pending.reject(error);
            this.#pending.clear();
            this.#ready = false;
            this.#details = null;
            worker.terminate();
            this.#worker = null;
            this.#transport = null;
        };
        worker.onerror = fail;
        worker.onmessageerror = fail;
        this.#worker = worker;
        return worker;
    }

    #request(type, payload = {}, transfer = [], onToken = null) {
        this.#ensureWorker();
        const id = this.#nextId++;
        return new Promise((resolve, reject) => {
            if (!Number.isSafeInteger(id) || this.#pending.size >= 128) {
                reject(new Error('CPU worker request capacity exhausted'));
                return;
            }
            this.#pending.set(id, { resolve, reject, onToken });
            try {
                this.#transport.send({ id, type, ...payload }, transfer);
            } catch (error) {
                this.#pending.delete(id);
                reject(error);
            }
        });
    }

    async initialize(modelBytes) {
        if (!this.#developerMode) {
            throw new Error('CPU browser inference requires explicit developerMode: true; Sentinel token acceptance is unavailable');
        }
        const modelBuffer = transferableModelBuffer(modelBytes);
        this.#details = await this.#request('init', {
            modelBuffer,
            executionMode: 'developer',
            acknowledgeUnguarded: true,
        }, [modelBuffer]);
        this.#ready = this.#details?.backend === 'cpu-wasm';
        if (!this.#ready) throw new Error('Qualia CPU worker selected an unexpected backend');
        return this.#details;
    }

    infer(prompt, maxTokens, onToken) {
        if (!this.#ready) return Promise.reject(new Error('Qualia CPU worker is not initialized'));
        return this.#request('infer', { prompt, maxTokens }, [], onToken);
    }

    async release() {
        if (!this.#worker) return;
        try {
            await this.#request('release');
        } finally {
            this.#worker.terminate();
            this.#worker = null;
            this.#transport = null;
            for (const pending of this.#pending.values()) pending.reject(new Error('CPU worker released'));
            this.#pending.clear();
            this.#ready = false;
            this.#details = null;
        }
    }
}
