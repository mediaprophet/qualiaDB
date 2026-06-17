/**
 * ICP UI visibility rules and default interface selection.
 */

import { ICP_BREAKPOINTS } from './qualia-icp-profile.js';

export const ICP_INTERFACE = {
    DECK: 'deck',
    CONTROL: 'control',
    GRAPH: 'graph',
    POINTER: 'pointer',
};

/**
 * @param {import('./qualia-icp-profile.js').IcpDeviceProfile} profile
 * @param {string} component
 * @returns {boolean}
 */
export function shouldShow(profile, component) {
    if (!profile) return false;
    switch (component) {
        case 'swipe_pad':
            return profile.pointer_primary === 'coarse'
                || profile.form_factor === 'phone'
                || profile.width < ICP_BREAKPOINTS.lg;
        case 'tilt_toggle':
            return profile.orientation_sensor && (profile.paired_remote || profile.form_factor !== 'desktop');
        case 'voice_toggle':
            return profile.voice_capable && (profile.paired_remote || profile.standalone_pwa);
        case 'graph_lens':
            return profile.form_factor !== 'desktop'
                || profile.width < ICP_BREAKPOINTS.lg
                || profile.pointer_primary === 'coarse';
        case 'keyboard_help':
            return profile.keyboard_available && profile.hover_available;
        case 'install_companion':
            return profile.width >= ICP_BREAKPOINTS.lg && !profile.standalone_pwa;
        case 'pointer_orbit':
            return profile.pointer_primary === 'fine' && profile.form_factor === 'desktop';
        default:
            return true;
    }
}

/**
 * @param {import('./qualia-icp-profile.js').IcpDeviceProfile} profile
 * @param {object} [hints]
 * @returns {string}
 */
export function pickDefaultInterface(profile, hints = {}) {
    if (hints.default_interface) return hints.default_interface;
    if (profile.paired_remote || profile.standalone_pwa) return ICP_INTERFACE.DECK;
    if (profile.pointer_primary === 'fine' && profile.width >= ICP_BREAKPOINTS.lg) {
        return ICP_INTERFACE.POINTER;
    }
    if (profile.pointer_primary === 'coarse') return ICP_INTERFACE.DECK;
    return ICP_INTERFACE.CONTROL;
}

/**
 * @param {HTMLElement} root
 * @param {import('./qualia-icp-profile.js').IcpDeviceProfile} profile
 */
export function applyIcpLayoutClasses(root, profile) {
    if (!root || !profile) return;
    root.dataset.icpForm = profile.form_factor;
    root.dataset.icpPointer = profile.pointer_primary;
    root.classList.toggle('icp-touch-mode', profile.pointer_primary === 'coarse');
    root.classList.toggle('icp-desktop-mode', profile.pointer_primary === 'fine' && profile.width >= ICP_BREAKPOINTS.lg);
    root.classList.toggle('icp-reduce-motion', profile.prefers_reduced_motion);
}