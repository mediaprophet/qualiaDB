/**
 * Worker-owned browser geometry model runner.
 *
 * This executes independent deterministic geometry workloads. It does not
 * render to the main-thread Canvas and does not claim native GPU execution.
 */
import { runGeometryLive } from './geometry-bench.js';

self.onmessage = async (event) => {
    try {
        const { type, algorithm, count } = event.data || {};
        if (type === 'ready') {
            self.postMessage({ type: 'ready' });
            return;
        }
        if (type === 'sample') {
            const started = performance.now();
            const result = await runGeometryLive(algorithm, count);
            self.postMessage({
                type: 'sample',
                elapsedMs: performance.now() - started,
                result,
            });
            return;
        }
        throw new Error(`Unsupported geometry worker message: ${type}`);
    } catch (error) {
        self.postMessage({ error: error?.message || String(error) });
    }
};
