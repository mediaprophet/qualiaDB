import { debugEnv, debugLog, debugWarn } from './qualia-debug.js';

/**
 * Cross-origin isolation for SharedArrayBuffer + AudioWorklet (Phase 7.4).
 * Registers coi-serviceworker.js — page reloads once with COOP/COEP headers.
 */
export async function ensureCrossOriginIsolation(options = {}) {
    if (typeof window === 'undefined') return false;
    if (window.crossOriginIsolated) {
        debugLog('COI already active');
        return true;
    }

    const { coiScript = new URL('./coi-serviceworker.js', import.meta.url).href, quiet = false } = options;

    if (!('serviceWorker' in navigator)) {
        debugWarn('COI: ServiceWorker unavailable');
        if (!quiet) console.warn('COI: ServiceWorker unavailable');
        return false;
    }

    // Never force shouldRegister:true — that overrides the one-shot reload guard in coi-serviceworker.js.
    window.coi = { quiet, shouldDeregister: () => false, ...window.coi };
    debugLog('COI bootstrap', { coiScript, quiet, coi: window.coi });
    debugEnv();

    // Idempotency: coi-serviceworker.js declares `let coepCredentialless` at
    // classic-script top level, so evaluating it twice in one document throws
    // "Identifier 'coepCredentialless' has already been declared". A page can
    // reach here after an inline <head> bootstrap (spatial/design-studio) or a
    // second ensureCrossOriginIsolation() call has already injected it — so skip
    // re-injection if the script is already present.
    const existing = document.querySelector(
        'script[data-coi-loader], script[src*="coi-serviceworker.js"]',
    );
    if (existing) {
        debugLog('COI script already injected, skipping');
        return window.crossOriginIsolated === true;
    }

    await new Promise((resolve, reject) => {
        const s = document.createElement('script');
        s.src = coiScript;
        s.async = false;
        s.dataset.coiLoader = '1';
        s.onload = resolve;
        s.onerror = () => reject(new Error('coi-serviceworker load failed'));
        document.head.appendChild(s);
    });

    const isolated = window.crossOriginIsolated === true;
    debugLog('COI ensure finished', { crossOriginIsolated: isolated });
    return isolated;
}

export function isCrossOriginIsolated() {
    return typeof window !== 'undefined' && window.crossOriginIsolated === true;
}