/**
 * Phone ICP client — pairing, relay publish, context/graph consumption, vault.
 */

import { createIcpRelay, ICP_ROLE, parseEnvelopeContent } from './qualia-icp-relay.js';
import {
    loadPairing,
    savePairingAsync,
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
import {
    getVaultStatus,
    initVault,
    pickVaultFolder,
} from './qualia-icp-vault.js';
import { loadQualiaPortal } from './qualia-shell.js';

const MENU_HOME = 1;

/**
 * @param {object} opts
 * @param {HTMLElement} opts.root
 * @param {object} [opts.portal]
 * @param {HTMLElement} [opts.linkBadge]
 * @param {HTMLElement} [opts.contextStrip]
 * @param {HTMLElement} [opts.controlPanel]
 * @param {HTMLElement} [opts.vaultPanel]
 * @param {HTMLCanvasElement} [opts.graphCanvas]
 * @param {HTMLElement} [opts.deckPad]
 * @param {HTMLElement} [opts.pairSection]
 */
export function mountIcpPhone(opts) {
    const {
        root,
        portal: portalIn,
        linkBadge,
        contextStrip,
        controlPanel,
        vaultPanel,
        graphCanvas,
        deckPad,
        pairSection,
    } = opts;

    let pairing = loadPairing();
    let relay = null;
    let linked = false;
    let vaultReady = false;
    let navIndex = 0;
    let graphLens = null;
    let portal = portalIn;
    let wasmMod = null;
    const cleanups = [];

    function updateBadge() {
        if (!linkBadge) return;
        let label = 'Unlinked';
        if (vaultReady) label = 'Vault ready';
        else if (linked) label = 'Linked';
        linkBadge.textContent = label;
        linkBadge.className = `text-xs px-2 py-1 rounded-full ${
            vaultReady || linked ? 'bg-emerald-500/20 text-emerald-300' : 'bg-slate-800 text-slate-400'
        }`;
        if (pairSection) {
            pairSection.classList.toggle('hidden', linked);
        }
    }

    async function refreshVaultUi() {
        if (!vaultPanel) return;
        const status = await getVaultStatus();
        vaultReady = status.ready;
        updateBadge();
        const q = status.quota;
        const quotaLine = q
            ? `Storage: ${Math.round(q.usage / 1024 / 1024)} / ${Math.round(q.quota / 1024 / 1024)} MB`
            : 'Storage: OPFS probe unavailable';
        vaultPanel.innerHTML = `
            <p class="text-xs text-white/60 mb-2">${status.ready ? 'Sovereign vault active' : 'Initialize on-device OPFS vault'}</p>
            <p class="text-[10px] text-white/45 mb-3 font-mono">${quotaLine}</p>
            ${status.manifest ? `<p class="text-[10px] text-emerald-400/80 mb-2">${status.manifest.identifier_did}</p>` : ''}
            <button type="button" data-icp-vault-init class="w-full py-2 mb-2 rounded-lg bg-emerald-600/30 text-emerald-300 text-sm border border-emerald-500/40">
                ${status.ready ? 'Refresh vault' : 'Init vault (identifier)'}
            </button>
            <button type="button" data-icp-vault-promote class="w-full py-2 mb-2 rounded-lg bg-slate-800 text-slate-300 text-sm border border-slate-600" ${status.ready ? '' : 'disabled'}>
                Promote to vault standpoint
            </button>
            ${status.folder_picker_supported ? `
            <button type="button" data-icp-vault-folder class="w-full py-2 rounded-lg bg-slate-800 text-slate-300 text-sm border border-slate-600">
                Link local folder
            </button>` : ''}
        `;
        vaultPanel.querySelector('[data-icp-vault-init]')?.addEventListener('click', () => initVaultFlow(false));
        vaultPanel.querySelector('[data-icp-vault-promote]')?.addEventListener('click', () => initVaultFlow(true));
        vaultPanel.querySelector('[data-icp-vault-folder]')?.addEventListener('click', () => pickVaultFolder().then(refreshVaultUi).catch(console.warn));
    }

    async function ensurePortal() {
        if (portal) return portal;
        let canvas = document.getElementById('icp-vault-canvas');
        if (!canvas) {
            canvas = document.createElement('canvas');
            canvas.id = 'icp-vault-canvas';
            canvas.width = 1;
            canvas.height = 1;
            canvas.className = 'sr-only';
            canvas.setAttribute('aria-hidden', 'true');
            document.body.appendChild(canvas);
        }
        const loaded = await loadQualiaPortal(canvas);
        portal = loaded.portal;
        wasmMod = loaded.mod;
        return portal;
    }

    async function initVaultFlow(promoteVault) {
        try {
            await ensurePortal();
            await initVault({ portal, wasmMod, promoteVault });
            vaultReady = true;
            updateBadge();
            await refreshVaultUi();
        } catch (e) {
            console.warn('[icp-vault]', e);
        }
    }

    async function publishCommand(raw) {
        if (portal) pushControl(portal, raw);
        if (!relay) return false;
        await relay.publish(ICP_ROLE.COMMAND, { raw: raw.toString() });
        return true;
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

    function connectRelay(payload) {
        pairing = payload;
        savePairingAsync(payload).catch(() => {});
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
                    updateBadge();
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
            .then(() => { linked = true; updateBadge(); })
            .catch(() => updateBadge());
    }

    if (pairing?.session_id) {
        connectRelay(pairing);
    }
    updateBadge();
    refreshVaultUi().catch(() => {});

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
        refreshVaultUi,
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
            updateBadge();
        },
        destroy() {
            relay?.stopPolling();
            cleanups.forEach((fn) => { if (typeof fn === 'function') fn(); });
        },
    };
}

/**
 * Wire pairing UI (paste + optional camera).
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