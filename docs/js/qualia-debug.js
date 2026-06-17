/**
 * Opt-in portal/docs boot diagnostics.
 * Enable: ?debug=1 | ?qualia_debug=1 | localStorage.setItem('qualia_debug','1')
 */
const PREFIX = '[Qualia]';

function queryFlag() {
    if (typeof window === 'undefined') return false;
    try {
        const q = new URLSearchParams(window.location.search);
        return q.has('debug') || q.get('qualia_debug') === '1';
    } catch {
        return false;
    }
}

function storageFlag() {
    if (typeof window === 'undefined') return false;
    try {
        return window.localStorage?.getItem('qualia_debug') === '1';
    } catch {
        return false;
    }
}

export function isQualiaDebugEnabled() {
    if (typeof window !== 'undefined' && window.__QUALIA_DEBUG__ === true) return true;
    return queryFlag() || storageFlag();
}

export function setQualiaDebug(enabled = true) {
    if (typeof window === 'undefined') return;
    window.__QUALIA_DEBUG__ = !!enabled;
    try {
        if (enabled) window.localStorage?.setItem('qualia_debug', '1');
        else window.localStorage?.removeItem('qualia_debug');
    } catch { /* ignore */ }
}

export function debugLog(...args) {
    if (!isQualiaDebugEnabled()) return;
    console.log(PREFIX, ...args);
}

export function debugWarn(...args) {
    if (!isQualiaDebugEnabled()) return;
    console.warn(PREFIX, ...args);
}

export function debugError(...args) {
    if (!isQualiaDebugEnabled()) return;
    console.error(PREFIX, ...args);
}

export function debugGroup(label, fn) {
    if (!isQualiaDebugEnabled()) return fn?.();
    console.groupCollapsed(`${PREFIX} ${label}`);
    try {
        return fn?.();
    } finally {
        console.groupEnd();
    }
}

export async function debugGroupAsync(label, fn) {
    if (!isQualiaDebugEnabled()) return fn?.();
    console.groupCollapsed(`${PREFIX} ${label}`);
    try {
        return await fn?.();
    } finally {
        console.groupEnd();
    }
}

export function debugTime(label) {
    if (!isQualiaDebugEnabled()) {
        return { end: () => {} };
    }
    const key = `${PREFIX} ${label}`;
    console.time(key);
    return {
        end(extra) {
            console.timeEnd(key);
            if (extra !== undefined) debugLog(label, extra);
        },
    };
}

/** Snapshot useful for WASM / COI / Pages troubleshooting. */
export function debugEnv(extra = {}) {
    if (!isQualiaDebugEnabled()) return null;
    const sw = navigator.serviceWorker?.controller;
    const snap = {
        page: window.location.href,
        crossOriginIsolated: window.crossOriginIsolated,
        serviceWorker: sw ? sw.scriptURL : null,
        userAgent: navigator.userAgent,
        ...extra,
    };
    debugLog('environment', snap);
    return snap;
}

if (typeof window !== 'undefined' && isQualiaDebugEnabled()) {
    debugLog('debug enabled — disable with localStorage.removeItem("qualia_debug")');
    debugEnv();
}