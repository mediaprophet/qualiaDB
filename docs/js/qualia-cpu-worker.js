import init, {
    beginBrowserWorkerRequest,
    configureBrowserWorkerTransport,
    configureBrowserExecutionMode,
    finishBrowserWorkerRequest,
    isBrowserWorkerRequestCurrent,
    getBrowserExecutionReceipt,
    getResidentTokenizerVocab,
    getWasmBackend,
    inferWasmAsyncMeasured,
    initializeCpuWasmEngine,
    isWasmEngineReady,
    releaseWebgpuEngine,
} from '../playground/qualia_core_db.js?v=0.0.40-mobile-performance1';
import { EtalonWorkerTransport } from './etalon-worker-transport.js';

let wasmReady;

function ensureWasm() {
    wasmReady ||= init().then(() => configureBrowserWorkerTransport(transport.shared));
    return wasmReady;
}

function errorText(error) {
    return error instanceof Error ? error.message : String(error);
}

function respond(id, type, payload = {}) {
    transport.send({ id, type, ...payload });
}

async function receive(data) {
    const { id, type } = data || {};
    if (!Number.isSafeInteger(id) || typeof type !== 'string') return;

    try {
        await ensureWasm();
        if (type === 'init') {
            if (data.executionMode !== 'developer' || data.acknowledgeUnguarded !== true) {
                throw new Error('CPU worker requires explicit developer mode acknowledgement; no Sentinel token acceptance');
            }
            configureBrowserExecutionMode('developer', true);
            const modelBuffer = data.modelBuffer;
            if (!(modelBuffer instanceof ArrayBuffer) || modelBuffer.byteLength === 0) {
                throw new Error('CPU worker received no model bytes');
            }
            if (modelBuffer.byteLength > transport.binaryByteBudget) throw new Error('CPU worker model byte budget exceeded');
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
            const request = beginBrowserWorkerRequest();
            try {
                const onToken = (piece) => {
                    if (isBrowserWorkerRequestCurrent(request)) respond(id, 'token', { piece });
                };
                const value = await inferWasmAsyncMeasured(
                    String(data.prompt || ''),
                    Number(data.maxTokens || 1),
                    onToken,
                );
                if (!finishBrowserWorkerRequest(request)) throw new Error('Stale worker inference completion');
                respond(id, 'result', { value });
            } finally {
                finishBrowserWorkerRequest(request);
            }
            return;
        }

        if (type === 'release') {
            configureBrowserExecutionMode('governed', false);
            await releaseWebgpuEngine();
            respond(id, 'result', { value: true });
            return;
        }

        throw new Error(`Unknown CPU worker request: ${type}`);
    } catch (error) {
        respond(id, 'error', { error: errorText(error) });
    }
}

const transport = new EtalonWorkerTransport(self, receive);
self.onmessage = ({ data }) => transport.handle(data);
