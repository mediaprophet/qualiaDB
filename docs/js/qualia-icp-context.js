/**
 * Bounded ContextFrame + GraphLensFrame builders (JS glue; Rust codec later).
 */

const TENSOR_HEADER_BYTES = 32;
const TENSOR_STRIDE = 40;
const TENSOR_MAGIC = 0x5134_322a;

/**
 * @param {object} opts
 * @param {number} [opts.revision]
 * @param {number} [opts.focusIndex]
 * @param {string} [opts.focusLabel]
 * @param {number} [opts.tier]
 * @param {object} [opts.icp_hints]
 */
export function buildContextFrame(opts = {}) {
    const frame = {
        revision: opts.revision ?? 0,
        focus_index: opts.focusIndex ?? -1,
        focus_label: opts.focusLabel || 'Spatial view',
        tier: opts.tier ?? 0,
        icp_hints: opts.icp_hints || defaultIcpHints(),
        menus: opts.menus || defaultMenus(),
        sliders: opts.sliders || defaultSliders(),
    };
    const json = JSON.stringify(frame);
    if (json.length > 4096) {
        frame.menus = frame.menus.slice(0, 8);
        frame.focus_label = frame.focus_label.slice(0, 48);
    }
    return frame;
}

function defaultIcpHints() {
    return {
        show_swipe_pad: true,
        show_tilt: false,
        show_voice: true,
        default_interface: 'deck',
        touch_min_px: 44,
    };
}

function defaultMenus() {
    return [
        { id: 1, label: 'Explore', parent: 0 },
        { id: 2, label: 'Graph topology', parent: 1, action: 'facet:graph' },
        { id: 3, label: 'Health vault', parent: 0 },
        { id: 4, label: 'Sleep', parent: 3, action: 'facet:sleep' },
    ];
}

function defaultSliders() {
    return [
        { id: 't_slice', label: 'Temporal slice', min: 0, max: 1, value: 0.5 },
        { id: 't_window', label: 'Window', min: 0.02, max: 1, value: 0.08 },
        { id: 'epistemic_q', label: 'Epistemic aperture', min: 0, max: 1, value: 1 },
    ];
}

/**
 * @param {Uint8Array|ArrayBuffer|null} buffer
 * @param {number} [maxNodes]
 */
export function buildGraphLensFromTensor(buffer, maxNodes = 256) {
    const nodes = parseTensorNodes(buffer, maxNodes);
    return {
        revision: Date.now(),
        projection: 'x_y',
        node_count: nodes.length,
        nodes: nodes.map((n, i) => ({
            index: i,
            x: n.x,
            y: n.y,
            z: n.z,
            selected: false,
        })),
        edges: [],
    };
}

function parseTensorNodes(buffer, maxNodes) {
    if (!buffer) return [];
    const view = buffer instanceof Uint8Array
        ? new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength)
        : new DataView(buffer);
    if (view.byteLength < TENSOR_HEADER_BYTES) return [];
    if (view.getUint32(0, true) !== TENSOR_MAGIC) return [];
    const nodeCount = view.getUint32(8, true);
    const stride = view.getUint32(12, true);
    if (stride !== TENSOR_STRIDE) return [];
    const pts = [];
    let offset = TENSOR_HEADER_BYTES;
    const limit = Math.min(nodeCount, maxNodes);
    for (let i = 0; i < limit; i++) {
        if (offset + stride > view.byteLength) break;
        pts.push({
            x: view.getFloat32(offset + 12, true),
            y: view.getFloat32(offset + 16, true),
            z: view.getFloat32(offset + 20, true),
        });
        offset += stride;
    }
    return pts;
}

const SLIDER_KIND = { t_slice: 0, t_window: 1, epistemic_q: 2 };

/**
 * @param {object} frame
 * @param {object} ui
 * @param {object} [handlers]
 * @param {(menu: object) => void} [handlers.onMenu]
 * @param {(id: string, value: number, prev: number) => void} [handlers.onSlider]
 */
export function applyContextFrameToUi(frame, ui, handlers = {}) {
    if (!frame || !ui) return;
    if (ui.contextStrip && frame.focus_label) {
        ui.contextStrip.textContent = frame.focus_label;
    }
    if (!ui.controlPanel) return;

    const menuHtml = (frame.menus || []).map((m) => {
        const indent = m.parent ? 'pl-4' : '';
        return `<button type="button" class="block w-full text-left py-2 px-2 rounded-lg bg-slate-800/60 mb-1 text-sm ${indent}"
            data-icp-menu="${m.id}">${m.label}</button>`;
    }).join('');

    const sliderHtml = (frame.sliders || []).map((s) => `
        <label class="block text-xs text-white/60 mb-1">${s.label}</label>
        <input type="range" min="${s.min}" max="${s.max}" step="0.01"
            value="${s.value}" data-icp-slider="${s.id}" class="w-full mb-3">
    `).join('');

    ui.controlPanel.innerHTML = (menuHtml || '<p class="text-xs text-white/50 mb-2">No menus</p>') + sliderHtml;

    ui.controlPanel.querySelectorAll('[data-icp-menu]').forEach((btn) => {
        btn.addEventListener('click', () => {
            const id = Number(btn.dataset.icpMenu);
            const menu = frame.menus?.find((m) => m.id === id);
            if (menu && handlers.onMenu) handlers.onMenu(menu);
        });
    });

    const sliderState = new Map();
    ui.controlPanel.querySelectorAll('[data-icp-slider]').forEach((input) => {
        const id = input.dataset.icpSlider;
        sliderState.set(id, Number(input.value));
        input.addEventListener('input', () => {
            const prev = sliderState.get(id) ?? Number(input.value);
            const value = Number(input.value);
            sliderState.set(id, value);
            if (handlers.onSlider) handlers.onSlider(id, value, prev);
        });
    });
}

export { SLIDER_KIND };

/**
 * @param {HTMLCanvasElement} canvas
 * @param {object} lens
 * @param {number} [selectedIndex]
 */
export function drawGraphLens(canvas, lens, selectedIndex = -1) {
    if (!canvas || !lens?.nodes?.length) return;
    const rect = canvas.getBoundingClientRect();
    const w = Math.max(rect.width, 1);
    const h = Math.max(rect.height, 1);
    canvas.width = w * devicePixelRatio;
    canvas.height = h * devicePixelRatio;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
    ctx.fillStyle = 'rgba(0,0,0,0.35)';
    ctx.fillRect(0, 0, w, h);

    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;
    for (const n of lens.nodes) {
        minX = Math.min(minX, n.x);
        maxX = Math.max(maxX, n.x);
        minY = Math.min(minY, n.y);
        maxY = Math.max(maxY, n.y);
    }
    const pad = 0.1;
    const spanX = Math.max(maxX - minX, 1e-6);
    const spanY = Math.max(maxY - minY, 1e-6);

    for (const n of lens.nodes) {
        const px = pad * w + ((n.x - minX) / spanX) * (1 - 2 * pad) * w;
        const py = pad * h + ((n.y - minY) / spanY) * (1 - 2 * pad) * h;
        const r = n.index === selectedIndex ? 5 : 3;
        ctx.beginPath();
        ctx.arc(px, py, r, 0, Math.PI * 2);
        ctx.fillStyle = n.index === selectedIndex ? '#34d399' : '#60a5fa';
        ctx.fill();
    }
}

/**
 * Map canvas tap to nearest node index.
 */
export function pickGraphLensNode(canvas, lens, clientX, clientY) {
    if (!canvas || !lens?.nodes?.length) return -1;
    const rect = canvas.getBoundingClientRect();
    const w = rect.width;
    const h = rect.height;
    const x = clientX - rect.left;
    const y = clientY - rect.top;

    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;
    for (const n of lens.nodes) {
        minX = Math.min(minX, n.x);
        maxX = Math.max(maxX, n.x);
        minY = Math.min(minY, n.y);
        maxY = Math.max(maxY, n.y);
    }
    const pad = 0.1;
    const spanX = Math.max(maxX - minX, 1e-6);
    const spanY = Math.max(maxY - minY, 1e-6);

    let best = -1;
    let bestDist = 24;
    for (const n of lens.nodes) {
        const px = pad * w + ((n.x - minX) / spanX) * (1 - 2 * pad) * w;
        const py = pad * h + ((n.y - minY) / spanY) * (1 - 2 * pad) * h;
        const d = Math.hypot(px - x, py - y);
        if (d < bestDist) {
            bestDist = d;
            best = n.index;
        }
    }
    return best;
}