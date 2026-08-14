const WORKER_URL = new URL('./qualia-cpu-worker.js?v=0.0.30-mobile-performance1', import.meta.url);

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
    #nextId = 1;
    #pending = new Map();
    #ready = false;
    #details = null;

    get ready() {
        return this.#ready;
    }

    get details() {
        return this.#details;
    }

    #ensureWorker() {
        if (this.#worker) return this.#worker;
        const worker = new Worker(WORKER_URL, { type: 'module', name: 'qualia-cpu-wasm' });
        worker.onmessage = ({ data }) => {
            const pending = this.#pending.get(data?.id);
            if (!pending) return;
            if (data.type === 'token') {
                pending.onToken?.(String(data.piece || ''));
                return;
            }
            this.#pending.delete(data.id);
            if (data.type === 'result') pending.resolve(data.value);
            else pending.reject(new Error(data.error || 'CPU worker request failed'));
        };
        worker.onerror = (event) => {
            const error = new Error(event.message || 'CPU worker crashed');
            for (const pending of this.#pending.values()) pending.reject(error);
            this.#pending.clear();
            this.#ready = false;
            this.#details = null;
        };
        this.#worker = worker;
        return worker;
    }

    #request(type, payload = {}, transfer = [], onToken = null) {
        const worker = this.#ensureWorker();
        const id = this.#nextId++;
        return new Promise((resolve, reject) => {
            this.#pending.set(id, { resolve, reject, onToken });
            try {
                worker.postMessage({ id, type, ...payload }, transfer);
            } catch (error) {
                this.#pending.delete(id);
                reject(error);
            }
        });
    }

    async initialize(modelBytes) {
        const modelBuffer = transferableModelBuffer(modelBytes);
        this.#details = await this.#request('init', { modelBuffer }, [modelBuffer]);
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
            this.#pending.clear();
            this.#ready = false;
            this.#details = null;
        }
    }
}
