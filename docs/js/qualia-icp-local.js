/**
 * Local Interface Control Plane — keyboard, wheel, optional swipe pad → portal ICP ring.
 */

import { detectIcpProfile, watchIcpProfile } from './qualia-icp-profile.js';
import {
    applyIcpLayoutClasses,
    pickDefaultInterface,
    shouldShow,
    ICP_INTERFACE,
} from './qualia-icp-rules.js';

/** ICP opcode mirrors (portal_control.rs) */
export const ICP_OP = {
    SET_CAMERA_DELTA: 0x60,
    NAVIGATE_INDEX: 0x61,
    COLLAPSE_Q: 0x62,
    SET_STANDPOINT_SCALAR: 0x63,
    MENU_ACTION: 0x64,
};

export const ICP_MAGIC = 0x8000_0000_0000_0000n;

const STANDPOINT_T_SLICE = 0;
const MENU_HOME = 1;

/**
 * @param {number} opcode
 * @param {number} channel
 * @param {number} index
 * @param {number} paramA i16
 * @param {number} paramB i8
 */
export function packControlCommand(opcode, channel = 0, index = 0, paramA = 0, paramB = 0) {
    return (BigInt(opcode & 0x7f)
        | (BigInt(channel & 0xff) << 8n)
        | (BigInt(index & 0xffff) << 16n)
        | (BigInt(paramA & 0xffff) << 32n)
        | (BigInt(paramB & 0xff) << 48n)
        | ICP_MAGIC) & 0xffff_ffff_ffff_ffffn;
}

/** @param {bigint|string|number} raw */
export function coerceControlRaw(raw) {
    if (typeof raw === 'bigint') return raw & 0xffff_ffff_ffff_ffffn;
    if (typeof raw === 'string') return BigInt(raw) & 0xffff_ffff_ffff_ffffn;
    return BigInt(raw >>> 0) | ICP_MAGIC;
}

export function packCameraDelta(dyaw, dpitch, dzoom) {
    const ya = Math.max(-32767, Math.min(32767, Math.round(dyaw * 1000)));
    const pi = Math.max(-32767, Math.min(32767, Math.round(dpitch * 1000)));
    const zo = Math.max(-127, Math.min(127, Math.round(dzoom * 1000)));
    return packControlCommand(ICP_OP.SET_CAMERA_DELTA, 0, pi & 0xffff, ya, zo);
}

/**
 * @param {object} portal QualiaPortal
 * @param {bigint|string|number} raw
 * @returns {boolean}
 */
export function pushControl(portal, raw) {
    if (portal?.push_control_command) {
        return portal.push_control_command(coerceControlRaw(raw));
    }
    return false;
}

/**
 * Mount local HID → ICP on primary viewport.
 * @param {object} opts
 * @param {object} opts.portal
 * @param {HTMLCanvasElement} [opts.canvas]
 * @param {HTMLElement} [opts.root]
 * @param {HTMLElement} [opts.deckPad]
 */
export function mountLocalIcp(opts) {
    const { portal, canvas, root = document.body, deckPad } = opts;
    if (!portal) return () => {};

    const cleanups = [];

    const onKey = (ev) => {
        if (ev.target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(ev.target.tagName)) return;
        let raw = null;
        switch (ev.key) {
            case 'ArrowLeft':
                raw = packCameraDelta(-0.08, 0, 0);
                break;
            case 'ArrowRight':
                raw = packCameraDelta(0.08, 0, 0);
                break;
            case 'ArrowUp':
                raw = packCameraDelta(0, -0.05, 0);
                break;
            case 'ArrowDown':
                raw = packCameraDelta(0, 0.05, 0);
                break;
            case '+':
            case '=':
                raw = packCameraDelta(0, 0, -0.15);
                break;
            case '-':
            case '_':
                raw = packCameraDelta(0, 0, 0.15);
                break;
            case '[':
                raw = packControlCommand(
                    ICP_OP.SET_STANDPOINT_SCALAR,
                    STANDPOINT_T_SLICE,
                    0,
                    -50,
                    0,
                );
                break;
            case ']':
                raw = packControlCommand(
                    ICP_OP.SET_STANDPOINT_SCALAR,
                    STANDPOINT_T_SLICE,
                    0,
                    50,
                    0,
                );
                break;
            case 'h':
            case 'H':
                raw = packControlCommand(ICP_OP.MENU_ACTION, 0, MENU_HOME, 0, 0);
                break;
            default:
                break;
        }
        if (raw != null) {
            ev.preventDefault();
            pushControl(portal, raw);
        }
    };
    document.addEventListener('keydown', onKey);
    cleanups.push(() => document.removeEventListener('keydown', onKey));

    if (canvas) {
        const onWheel = (ev) => {
            ev.preventDefault();
            const dz = ev.deltaY > 0 ? 0.12 : -0.12;
            pushControl(portal, packCameraDelta(0, 0, dz));
        };
        canvas.addEventListener('wheel', onWheel, { passive: false });
        cleanups.push(() => canvas.removeEventListener('wheel', onWheel));
    }

    if (deckPad && shouldShow(detectIcpProfile({ wasm_tier: portal.tier?.() }), 'swipe_pad')) {
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
            pushControl(portal, packCameraDelta(dx * 0.5, dy * 0.35, 0));
        };
        deckPad.addEventListener('pointerdown', onDown);
        deckPad.addEventListener('pointerup', onUp);
        cleanups.push(() => {
            deckPad.removeEventListener('pointerdown', onDown);
            deckPad.removeEventListener('pointerup', onUp);
        });
    }

    const stopWatch = watchIcpProfile((profile) => {
        applyIcpLayoutClasses(root, profile);
        const iface = pickDefaultInterface(profile);
        root.dataset.icpInterface = iface;
        root.classList.toggle('icp-mode-deck', iface === ICP_INTERFACE.DECK);
        root.classList.toggle('icp-mode-control', iface === ICP_INTERFACE.CONTROL);
        root.classList.toggle('icp-mode-graph', iface === ICP_INTERFACE.GRAPH);
        root.classList.toggle('icp-form-phone', profile.form_factor === 'phone');
    }, { wasm_tier: portal.tier?.() ?? 0 });

    cleanups.push(stopWatch);

    return () => cleanups.forEach((fn) => fn());
}