/**
 * Browser geometry measurement topology.
 *
 * Worker mode runs independent copies of the deterministic browser geometry
 * model. Aggregate timing includes worker scheduling and message delivery.
 */
import { runGeometryLive } from './geometry-bench.js';

const SAMPLE_COUNT = 5;

function workerCountFor(topology) {
    return topology === 'workers-2' ? 2 : topology === 'workers-4' ? 4 : 0;
}

function percentile(sorted, fraction) {
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))];
}

function summarize(samples) {
    const sorted = [...samples].sort((a, b) => a - b);
    const mean = sorted.reduce((sum, value) => sum + value, 0) / sorted.length;
    return { mean, p50: percentile(sorted, 0.5), p95: percentile(sorted, 0.95) };
}

function callWorker(worker, message) {
    return new Promise((resolve, reject) => {
        const onMessage = (event) => {
            cleanup();
            if (event.data?.error) reject(new Error(event.data.error));
            else resolve(event.data);
        };
        const onError = (event) => {
            cleanup();
            reject(event.error || new Error(event.message || 'Geometry worker failed.'));
        };
        const cleanup = () => {
            worker.removeEventListener('message', onMessage);
            worker.removeEventListener('error', onError);
        };
        worker.addEventListener('message', onMessage);
        worker.addEventListener('error', onError);
        worker.postMessage(message);
    });
}

export async function runGeometryWithTopology(algorithm, count, topology = 'single') {
    const workerCount = workerCountFor(topology);
    if (!workerCount) {
        const result = await runGeometryLive(algorithm, count);
        return {
            ...result,
            workerCount: 1,
            executionMode: 'single browser main thread',
            aggregate: null,
        };
    }

    const workers = Array.from({ length: workerCount }, () => new Worker(
        new URL('./geometry-bench-worker.js', import.meta.url),
        { type: 'module', name: 'qualia-geometry-model' },
    ));
    try {
        const setupStarted = performance.now();
        await Promise.all(workers.map((worker) => callWorker(worker, { type: 'ready' })));
        const setupMs = performance.now() - setupStarted;
        const wallTimes = [];
        const slowestWorkerTimes = [];
        let representative;

        for (let sample = 0; sample < SAMPLE_COUNT; sample++) {
            const started = performance.now();
            const results = await Promise.all(workers.map((worker) => callWorker(worker, {
                type: 'sample', algorithm, count,
            })));
            const wallElapsed = performance.now() - started;
            wallTimes.push(wallElapsed);
            slowestWorkerTimes.push(Math.max(...results.map((result) => result.elapsedMs)));
            representative = results[0].result;
        }

        const wall = summarize(wallTimes);
        const kernel = summarize(slowestWorkerTimes);
        const workItems = count * workerCount;
        return {
            ...representative,
            opsPerSec: Math.round((workItems * 1000) / Math.max(wall.mean, Number.EPSILON)),
            meanMs: +wall.mean.toFixed(3),
            workerCount,
            executionMode: `${workerCount} independent browser workers`,
            aggregate: {
                samples: SAMPLE_COUNT,
                setupMs,
                wall,
                slowestWorkerKernel: kernel,
            },
        };
    } finally {
        workers.forEach((worker) => worker.terminate());
    }
}
