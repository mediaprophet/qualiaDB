/**
 * Interface Control Plane — device capability profile (feature detection).
 */

export const ICP_BREAKPOINTS = {
    lg: 1024,
    md: 768,
};

/** @typedef {'desktop'|'tablet'|'phone'} FormFactor */
/** @typedef {'fine'|'coarse'|'none'} PointerPrimary */

/**
 * @typedef {object} IcpDeviceProfile
 * @property {FormFactor} form_factor
 * @property {PointerPrimary} pointer_primary
 * @property {boolean} hover_available
 * @property {boolean} keyboard_available
 * @property {boolean} orientation_sensor
 * @property {boolean} motion_sensor
 * @property {boolean} voice_capable
 * @property {boolean} paired_remote
 * @property {number} wasm_tier
 * @property {boolean} prefers_reduced_motion
 * @property {boolean} standalone_pwa
 * @property {number} width
 */

function queryPointerFine() {
    try {
        return window.matchMedia('(pointer: fine)').matches;
    } catch {
        return true;
    }
}

function queryPointerCoarse() {
    try {
        return window.matchMedia('(pointer: coarse)').matches;
    } catch {
        return false;
    }
}

/**
 * @param {object} [opts]
 * @param {number} [opts.wasm_tier]
 * @param {boolean} [opts.paired_remote]
 * @returns {IcpDeviceProfile}
 */
export function detectIcpProfile(opts = {}) {
    const width = window.innerWidth || 1024;
    const standalone = window.matchMedia('(display-mode: standalone)').matches
        || window.navigator.standalone === true;

    let form_factor = 'desktop';
    if (width < ICP_BREAKPOINTS.md || standalone) {
        form_factor = 'phone';
    } else if (width < ICP_BREAKPOINTS.lg) {
        form_factor = 'tablet';
    }

    const fine = queryPointerFine();
    const coarse = queryPointerCoarse();
    let pointer_primary = 'none';
    if (fine && !coarse) pointer_primary = 'fine';
    else if (coarse) pointer_primary = 'coarse';
    else if (fine) pointer_primary = 'fine';

    const override = localStorage.getItem('qualia-icp-touch-mode');
    if (override === '1') pointer_primary = 'coarse';
    if (override === '0') pointer_primary = 'fine';

    const urlProfile = new URLSearchParams(window.location.search).get('icp_profile');
    if (urlProfile === 'tablet') form_factor = 'tablet';
    if (urlProfile === 'phone') form_factor = 'phone';

    return {
        form_factor,
        pointer_primary,
        hover_available: window.matchMedia('(hover: hover)').matches,
        keyboard_available: !('ontouchstart' in window) || fine,
        orientation_sensor: typeof window.DeviceOrientationEvent !== 'undefined',
        motion_sensor: typeof window.DeviceMotionEvent !== 'undefined',
        voice_capable: typeof window.speechSynthesis !== 'undefined',
        paired_remote: Boolean(opts.paired_remote),
        wasm_tier: opts.wasm_tier ?? 0,
        prefers_reduced_motion: window.matchMedia('(prefers-reduced-motion: reduce)').matches,
        standalone_pwa: standalone,
        width,
    };
}

let cachedProfile = null;
let resizeTimer = null;

/**
 * @param {(profile: IcpDeviceProfile) => void} onChange
 * @param {object} [opts]
 */
export function watchIcpProfile(onChange, opts = {}) {
    const emit = () => {
        cachedProfile = detectIcpProfile(opts);
        onChange(cachedProfile);
    };
    emit();
    window.addEventListener('resize', () => {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(emit, 120);
    });
    window.addEventListener('orientationchange', emit);
    return () => {
        clearTimeout(resizeTimer);
    };
}

export function getCachedIcpProfile() {
    return cachedProfile || detectIcpProfile();
}