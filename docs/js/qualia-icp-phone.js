/**
 * Phone ICP client — pairing, relay publish, context/graph consumption.
 */

import { createIcpRelay, ICP_ROLE, parseEnvelopeContent } from './qualia-icp-relay.js';
import {
    loadPairing,
    savePairing,
    parsePairingPayload,
    clearPairing,
    startBarcodePairing,
} from './qualia-icp-session.js';
import {
    applyContextFrameToUi,
    drawGraphLens,
    pickGraphLensNode,
    SLIDER_KIND,
} from './qualia-icp-context.js';
import {
    packCameraDelta,
    packControlCommand,
    ICP_OP,
    pushControl,
} from './qualia-icp-local.js';

const MENU_HOME = 1;

/**
 * @param {object} opts
 * @param {HTMLElement} opts.root
 * @param {object} [opts.portal] optional local WASM portal
 * @param {HTMLElement} [opts.linkBadge]
 * @param {HTMLElement} [opts.contextStrip]
 * @param {HTMLElement} [opts.controlPanel]
 * @param {HTMLCanvasElement} [opts.graphCanvas]
 * @param {HTMLElement} [opts.deckPad]
 * @param {HTMLElement} [opts.pairSection]
 */
export function mountIcpPhone(opts) {
    const {
        root,
        portal,
        linkBadge,
        contextStrip,
        controlPanel,
        graphCanvas,
        deckPad,
        pairSection,
    } = opts;

    let pairing = loadPairing();
    let relay = null;
    let linked = false;
    let navIndex = 0;
    let graphLens = null;
    const cleanups = [];

    function setBadge(state) {
        if (linkBadge) {
            linkBadge.textContent = state;
            linkBadge.className = `text-xs px-2 py-1 rounded-full ${
                linked ? 'bg-emerald-500/20 text-emerald-300' : 'bg-slate-800 text-slate-400'
            }`;
        }
        if (pairSection) {
            pairSection.classList.toggle('hidden', linked);
        }
    }

    const controlHandlers = {
        onMenu: (menu) => {
            publishCommand(packControlCommand(ICP_OP.MENU_ACTION, 0, menu.id & 0xffff, 0, 0));
        },
        onSlider: (id, value, prev) => {
            const kind = SLIDER_KIND[id];
            if (kind == null) return;
            const delta = value - prev;
            const scaled = Math.max(-32767, Math.min(32767, Math.round(delta * 1000)));
            publishCommand(packControlCommand(ICP_OP.SET_STANDPOINT_SCALAR, kind, 0, scaled, 0));
        },
    };

    async function publishCommand(raw) {
        if (portal) pushControl(portal, raw);
        if (!relay) return false;
        await relay.publish(ICP_ROLE.COMMAND, { raw: raw.toString() });
        return true;
    }

    function connectRelay(payload) {
        pairing = payload;
        savePairing(payload);
        if (relay) relay.stopPolling();
        relay = createIcpRelay({
            base: payload.relay,
            sessionId: payload.session_id,
            authorDid: 'did:icp:phone',
            authorName: 'phone-console',
        });

        const unsub = relay.onMessage((envelope) => {
            if (envelope.author_did === relay.authorDid) return;
            const content = parseEnvelopeContent(envelope);
            switch (envelope.role) {
                case ICP_ROLE.HELLO:
                case ICP_ROLE.CONTEXT:
                    linked = true;
                    setBadge('Linked');
                    applyContextFrameToUi(content, { contextStrip, controlPanel }, controlHandlers);
                    break;
                case ICP_ROLE.GRAPH:
                    graphLens = content;
                    if (graphCanvas) drawGraphLens(graphCanvas, graphLens, navIndex);
                    break;
                case ICP_ROLE.PUSH:
                    if (content?.focus_label && contextStrip) {
                        contextStrip.textContent = content.focus_label;
                    }
                    break;
                default:
                    break;
            }
        });
        cleanups.push(unsub);
        relay.startPolling(400);
        relay.publish(ICP_ROLE.HELLO, { role: 'phone', session_id: payload.session_id })
            .then(() => setBadge('Linked'))
            .catch(() => setBadge('Relay error'));
    }

    if (pairing?.session_id) {
        connectRelay(pairing);
        setBadge('Linked');
    } else {
        setBadge('Unlinked');
    }

    // Deck pad swipe
    if (deckPad) {
        let lastX = 0;
        let lastY = 0;
        const onDown = (ev) => {
            lastX = ev.clientX;
            lastY = ev.clientY;
        };
        const onUp = (ev) => {
            const dx = (ev.clientX - lastX) / Math.max(window.innerWidth, 1);
            const dy = (ev.clientY - lastY) / Math.max(window.innerHeight, 1);
            if (Math.abs(dx) < 0.02 && Math.abs(dy) < 0.02) return;
            publishCommand(packCameraDelta(dx * 0.5, dy * 0.35, 0));
        };
        deckPad.addEventListener('pointerdown', onDown);
        deckPad.addEventListener('pointerup', onUp);
        cleanups.push(() => {
            deckPad.removeEventListener('pointerdown', onDown);
            deckPad.removeEventListener('pointerup', onUp);
        });
    }

    // Deck buttons
    root.querySelectorAll('[data-icp-btn]').forEach((btn) => {
        const handler = () => {
            const action = btn.dataset.icpBtn;
            let raw = null;
            switch (action) {
                case 'back':
                    navIndex = Math.max(0, navIndex - 1);
                    raw = packControlCommand(ICP_OP.NAVIGATE_INDEX, 0, navIndex, 0, 0);
                    break;
                case 'next':
                    navIndex += 1;
                    raw = packControlCommand(ICP_OP.NAVIGATE_INDEX, 0, navIndex, 0, 0);
                    break;
                case 'select':
                    raw = packControlCommand(ICP_OP.NAVIGATE_INDEX, 0, navIndex, 0, 0);
                    break;
                case 'home':
                    raw = packControlCommand(ICP_OP.MENU_ACTION, 0, MENU_HOME, 0, 0);
                    break;
                default:
                    break;
            }
            if (raw != null) publishCommand(raw);
        };
        btn.addEventListener('click', handler);
        cleanups.push(() => btn.removeEventListener('click', handler));
    });

    if (graphCanvas) {
        const onTap = (ev) => {
            if (!graphLens) return;
            const idx = pickGraphLensNode(graphCanvas, graphLens, ev.clientX, ev.clientY);
            if (idx < 0) return;
            navIndex = idx;
            publishCommand(packControlCommand(ICP_OP.NAVIGATE_INDEX, 0, navIndex, 0, 0));
            drawGraphLens(graphCanvas, graphLens, navIndex);
        };
        graphCanvas.addEventListener('click', onTap);
        cleanups.push(() => graphCanvas.removeEventListener('click', onTap));
    }

    return {
        connectRelay,
        pairFromText(text) {
            const payload = parsePairingPayload(text);
            if (!payload) return false;
            connectRelay(payload);
            return true;
        },
        unlink() {
            clearPairing();
            linked = false;
            relay?.stopPolling();
            relay = null;
            setBadge('Unlinked');
        },
        destroy() {
            relay?.stopPolling();
            cleanups.forEach((fn) => { if (typeof fn === 'function') fn(); });
        },
    };
}

/**
 * Wire pairing UI (paste + optional camera).
 * @param {object} opts
 * @param {HTMLElement} opts.pairSection
 * @param {(payload: object) => void} opts.onPair
 */
export async function mountPairingUi(opts) {
    const { pairSection, onPair } = opts;
    if (!pairSection) return () => {};

    const pasteBtn = pairSection.querySelector('[data-icp-paste-pair]');
    const scanBtn = pairSection.querySelector('[data-icp-scan-pair]');
    const video = pairSection.querySelector('[data-icp-scan-video]');
    let scanHandle = null;

    const tryPaste = async () => {
        let text = '';
        try {
            text = await navigator.clipboard.readText();
        } catch {
            text = window.prompt('Paste pairing JSON from desktop') || '';
        }
        const payload = parsePairingPayload(text);
        if (payload) {
            onPair(payload);
            return true;
        }
        return false;
    };

    if (pasteBtn) {
        pasteBtn.addEventListener('click', tryPaste);
    }
    if (scanBtn && video) {
        scanBtn.addEventListener('click', async () => {
            if (scanHandle) {
                scanHandle.stop();
                scanHandle = null;
                scanBtn.textContent = 'Scan QR';
                video.classList.add('hidden');
                return;
            }
            scanBtn.textContent = 'Stop scan';
            video.classList.remove('hidden');
            scanHandle = await startBarcodePairing({
                video,
                onPair: (payload) => {
                    onPair(payload);
                    scanHandle?.stop();
                    scanHandle = null;
                    scanBtn.textContent = 'Scan QR';
                    video.classList.add('hidden');
                },
            });
            if (!scanHandle.supported) {
                scanBtn.textContent = 'Scan QR';
                video.classList.add('hidden');
                await tryPaste();
            }
        });
    }

    return () => {
        scanHandle?.stop();
    };
}