const params = new URLSearchParams(location.search);
const session = params.get('lab') || '';
const includeText = params.get('labText') === '1';
const enabled = /^[A-Za-z0-9_-]{8,64}$/.test(session);
let sequence = 0;

function errorText(value) {
    return String(value?.stack || value?.message || value || 'unknown error').slice(0, 2000);
}

async function emit(type, detail = {}) {
    if (!enabled) return false;
    const event = {
        schema: 1,
        session,
        sequence: sequence++,
        clientTime: new Date().toISOString(),
        monotonicMs: Math.round(performance.now()),
        type,
        detail,
    };
    try {
        const response = await fetch(`/__qualia/mobile-log?lab=${encodeURIComponent(session)}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(event),
            keepalive: true,
        });
        return response.ok;
    } catch {
        return false;
    }
}

function memorySnapshot() {
    const memory = performance.memory;
    return memory ? {
        jsHeapLimit: memory.jsHeapSizeLimit,
        jsHeapTotal: memory.totalJSHeapSize,
        jsHeapUsed: memory.usedJSHeapSize,
    } : null;
}

async function captureEnvironment() {
    const detail = {
        href: location.href.replace(/([?&]lab=)[^&]+/, '$1…'),
        userAgent: navigator.userAgent,
        platform: navigator.platform,
        deviceMemoryGb: navigator.deviceMemory ?? null,
        hardwareConcurrency: navigator.hardwareConcurrency ?? null,
        maxTouchPoints: navigator.maxTouchPoints ?? null,
        secureContext: window.isSecureContext,
        crossOriginIsolated: window.crossOriginIsolated,
        webgpu: Boolean(navigator.gpu),
        viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
        screen: { width: screen.width, height: screen.height, colorDepth: screen.colorDepth },
        memory: memorySnapshot(),
    };
    if (navigator.gpu) {
        try {
            const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
            const info = adapter?.info || await adapter?.requestAdapterInfo?.();
            detail.adapter = info ? {
                vendor: info.vendor || '',
                architecture: info.architecture || '',
                device: info.device || '',
                description: info.description || '',
            } : null;
            detail.limits = adapter ? {
                maxBufferSize: Number(adapter.limits.maxBufferSize),
                maxStorageBufferBindingSize: Number(adapter.limits.maxStorageBufferBindingSize),
                maxComputeWorkgroupStorageSize: Number(adapter.limits.maxComputeWorkgroupStorageSize),
                maxComputeInvocationsPerWorkgroup: Number(adapter.limits.maxComputeInvocationsPerWorkgroup),
            } : null;
        } catch (error) {
            detail.adapterError = errorText(error);
        }
    }
    await emit('environment', detail);
}

window.addEventListener('error', (event) => {
    void emit('window_error', { message: errorText(event.error || event.message), source: event.filename || '' });
});
window.addEventListener('unhandledrejection', (event) => {
    void emit('unhandled_rejection', { message: errorText(event.reason) });
});
document.addEventListener('visibilitychange', () => {
    void emit('visibility', { state: document.visibilityState, memory: memorySnapshot() });
});
window.addEventListener('pagehide', () => {
    void emit('pagehide', { memory: memorySnapshot() });
});

export const mobileLab = {
    enabled,
    includeText,
    emit,
    errorText,
    memorySnapshot,
    textPreview(value) {
        return includeText ? String(value || '').slice(0, 512) : undefined;
    },
};

if (enabled) {
    void captureEnvironment();
}
