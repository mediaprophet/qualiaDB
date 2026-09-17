import init, {
    getBrowserExecutionReceipt,
    getResidentTokenizerVocab,
    getWasmBackend,
    inferWasmAsyncMeasured,
    initializeCpuWasmEngine,
    isWasmEngineReady,
    releaseWebgpuEngine,
} from '../playground/qualia_core_db.js?v=0.0.33-mobile-performance1';

let wasmReady;

function ensureWasm() {
    wasmReady ||= init();
    return wasmReady;
}

function errorText(error) {
    return error instanceof Error ? error.message : String(error);
}

function respond(id, type, payload = {}) {
    self.postMessage({ id, type, ...payload });
}

self.onmessage = async ({ data }) => {
    const { id, type } = data || {};
    if (!Number.isSafeInteger(id) || typeof type !== 'string') return;

    try {
        await ensureWasm();
        if (type === 'init') {
            const modelBuffer = data.modelBuffer;
            if (!(modelBuffer instanceof ArrayBuffer) || modelBuffer.byteLength === 0) {
                throw new Error('CPU worker received no model bytes');
            }
            await initializeCpuWasmEngine(new Uint8Array(modelBuffer));
            if (!isWasmEngineReady()) throw new Error('CPU-WASM worker did not become ready');
            respond(id, 'result', {
                value: {
                    backend: getWasmBackend(),
                    vocab: getResidentTokenizerVocab(),
                    executionReceipt: {
                        ...getBrowserExecutionReceipt(),
                        executionHost: 'dedicated-worker',
                        workerCount: 1,
                        wasmSimd128: true,
                        packedQ8Gemv: true,
                    },
                },
            });
            return;
        }

        if (type === 'infer') {
            const onToken = (piece) => respond(id, 'token', { piece });
            const value = await inferWasmAsyncMeasured(
                String(data.prompt || ''),
                Number(data.maxTokens || 1),
                onToken,
            );
            respond(id, 'result', { value });
            return;
        }

        if (type === 'release') {
            await releaseWebgpuEngine();
            respond(id, 'result', { value: true });
            return;
        }

        throw new Error(`Unknown CPU worker request: ${type}`);
    } catch (error) {
        respond(id, 'error', { error: errorText(error) });
    }
};
