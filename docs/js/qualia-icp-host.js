/**
 * Desktop ICP host — create session, poll relay, apply remote commands to portal.
 */

import { createIcpRelay, ICP_ROLE, parseEnvelopeContent } from './qualia-icp-relay.js';
import {
    createConsoleSession,
    defaultRelayBase,
    pairingToJson,
    renderPairingQr,
    savePairing,
} from './qualia-icp-session.js';
import { buildContextFrame, buildGraphLensFromTensor } from './qualia-icp-context.js';
import { pushControl } from './qualia-icp-local.js';

/**
 * @param {object} opts
 * @param {object} opts.portal QualiaPortal
 * @param {string} [opts.relayBase]
 * @param {HTMLElement} [opts.linkPanel]
 * @param {HTMLCanvasElement} [opts.qrCanvas]
 * @param {HTMLElement} [opts.statusEl]
 * @param {() => Uint8Array|null} [opts.getTensorBuffer]
 * @param {number} [opts.focusIndex]
 * @param {string} [opts.focusLabel]
 */
export function mountIcpHost(opts) {
    const {
        portal,
        relayBase = defaultRelayBase(),
        linkPanel,
        qrCanvas,
        statusEl,
        getTensorBuffer,
        focusIndex = -1,
        focusLabel = 'Spatial view',
    } = opts;

    if (!portal) return () => {};

    const session = createConsoleSession({ relayBase, origin: window.location.origin });
    savePairing(session);

    const relay = createIcpRelay({
        base: session.relay,
        sessionId: session.session_id,
        authorDid: 'did:icp:desktop',
        authorName: 'desktop-host',
    });

    let linked = false;
    let revision = 0;
    const cleanups = [];

    function setStatus(text) {
        if (statusEl) statusEl.textContent = text;
    }

    function showLinkPanel() {
        if (!linkPanel) return;
        linkPanel.classList.remove('hidden');
        const jsonEl = linkPanel.querySelector('[data-icp-pair-json]');
        if (jsonEl) jsonEl.textContent = pairingToJson(session);
        if (qrCanvas) renderPairingQr(qrCanvas, session);
        const copyBtn = linkPanel.querySelector('[data-icp-copy-pair]');
        if (copyBtn && !copyBtn.dataset.icpWired) {
            copyBtn.dataset.icpWired = '1';
            copyBtn.addEventListener('click', async () => {
                try {
                    await navigator.clipboard.writeText(pairingToJson(session));
                    copyBtn.textContent = 'Copied';
                    setTimeout(() => { copyBtn.textContent = 'Copy pairing JSON'; }, 1500);
                } catch {
                    copyBtn.textContent = 'Copy failed';
                }
            });
        }
    }

    async function pushContext() {
        revision += 1;
        const frame = buildContextFrame({
            revision,
            focusIndex,
            focusLabel,
            tier: portal.tier?.() ?? 0,
        });
        await relay.publish(ICP_ROLE.CONTEXT, frame);
    }

    async function pushGraph() {
        const buf = getTensorBuffer?.();
        if (!buf) return;
        const lens = buildGraphLensFromTensor(buf);
        await relay.publish(ICP_ROLE.GRAPH, lens);
    }

    async function pushHello() {
        await relay.publish(ICP_ROLE.HELLO, {
            role: 'desktop',
            session_id: session.session_id,
            tier: portal.tier?.() ?? 0,
        });
    }

    function applyRemoteCommand(content) {
        const raw = typeof content === 'object' ? content.raw : content;
        if (raw == null) return false;
        return pushControl(portal, raw);
    }

    const unsub = relay.onMessage(async (envelope) => {
        if (envelope.author_did === relay.authorDid) return;
        const content = parseEnvelopeContent(envelope);

        switch (envelope.role) {
            case ICP_ROLE.HELLO:
                linked = true;
                setStatus(`Linked · ${envelope.author_name || 'phone'}`);
                await pushHello();
                await pushContext();
                await pushGraph();
                break;
            case ICP_ROLE.COMMAND:
                applyRemoteCommand(content);
                await relay.publish(ICP_ROLE.PUSH, {
                    focus_index: focusIndex,
                    focus_label: focusLabel,
                    ack_lamport: envelope.lamport,
                });
                break;
            default:
                break;
        }
    });
    cleanups.push(unsub);

    relay.startPolling(350);
    cleanups.push(() => relay.stopPolling());

    showLinkPanel();
    setStatus('Waiting for phone…');
    pushHello().catch(() => {});

    const contextInterval = setInterval(() => {
        if (linked) {
            pushContext().catch(() => {});
            pushGraph().catch(() => {});
        }
    }, 5000);
    cleanups.push(() => clearInterval(contextInterval));

    return () => {
        cleanups.forEach((fn) => { if (typeof fn === 'function') fn(); });
    };
}

/**
 * Inject link-phone UI into a container if elements are missing.
 * @param {HTMLElement} mountEl
 */
export function ensureLinkPhoneUi(mountEl) {
    if (!mountEl) return null;
    const existing = mountEl.querySelector('[data-icp-link-panel]');
    if (existing) return existing;

    const panel = document.createElement('div');
    panel.dataset.icpLinkPanel = '1';
    panel.className = 'mt-4 p-3 rounded-xl border border-emerald-500/30 bg-emerald-500/5';
    panel.innerHTML = `
        <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-semibold text-emerald-400">Link phone</span>
            <span data-icp-status class="text-[10px] text-white/50">Unlinked</span>
        </div>
        <canvas data-icp-qr width="160" height="160" class="mx-auto mb-2 rounded bg-white"></canvas>
        <pre data-icp-pair-json class="text-[9px] text-white/40 overflow-auto max-h-24 p-2 bg-black/30 rounded"></pre>
        <button type="button" data-icp-copy-pair class="w-full mt-2 py-1.5 text-[10px] rounded bg-slate-800 text-emerald-300 border border-slate-600">Copy pairing JSON</button>
        <p class="text-[10px] text-white/45 mt-2">Scan QR or paste JSON on phone-console.html · relay uses this host</p>
    `;
    mountEl.appendChild(panel);
    return panel;
}

export { defaultRelayBase };