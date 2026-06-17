/**
 * Cross-origin isolation for SharedArrayBuffer + AudioWorklet (Phase 7.4).
 * Registers coi-serviceworker.js — page reloads once with COOP/COEP headers.
 */
export async function ensureCrossOriginIsolation(options = {}) {
    if (typeof window === 'undefined') return false;
    if (window.crossOriginIsolated) return true;

    const { coiScript = new URL('./coi-serviceworker.js', import.meta.url).href, quiet = false } = options;

    if (!('serviceWorker' in navigator)) {
        if (!quiet) console.warn('COI: ServiceWorker unavailable');
        return false;
    }

    // Never force shouldRegister:true — that overrides the one-shot reload guard in coi-serviceworker.js.
    window.coi = { quiet, shouldDeregister: () => false, ...window.coi };

    await new Promise((resolve, reject) => {
        const s = document.createElement('script');
        s.src = coiScript;
        s.async = false;
        s.onload = resolve;
        s.onerror = () => reject(new Error('coi-serviceworker load failed'));
        document.head.appendChild(s);
    });

    return window.crossOriginIsolated === true;
}

export function isCrossOriginIsolated() {
    return typeof window !== 'undefined' && window.crossOriginIsolated === true;
}