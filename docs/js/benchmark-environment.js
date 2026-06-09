/**
 * Schema v2 execution environment helpers for GitHub Pages benchmarks.
 * Non-identifying device fingerprint + topology labels for multi-cell daemon runs.
 */

export function detectHostClass() {
    const ua = navigator.userAgent || '';
    const platform = (navigator.platform || '').toLowerCase();
    if (/iPhone|iPad|Android/i.test(ua)) return 'ARM64_MOBILE';
    if (/Mac/i.test(platform) || /Mac OS X/i.test(ua)) {
        if (/ARM|Apple/i.test(ua) || platform === 'macarm64') return 'APPLE_SILICON';
        return 'X86_64_DESKTOP';
    }
    if (/Win/i.test(platform)) return 'X86_64_DESKTOP';
    if (/Linux/i.test(platform)) return 'X86_64_SERVER';
    return 'WASM_BROWSER';
}

export async function probeWasmSimd() {
    try {
        const bytes = new Uint8Array([
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7b,
            0x03, 0x02, 0x01, 0x00, 0x0a, 0x0a, 0x01, 0x08,
            0x00, 0xfd, 0x0f, 0x1a, 0x0b, 0x0b,
        ]);
        return WebAssembly.validate(bytes);
    } catch (_) {
        return false;
    }
}

export async function collectDeviceManifest() {
    const simd = await probeWasmSimd();
    return {
        host_class: detectHostClass(),
        cpu_arch: 'wasm32',
        os: navigator.platform || 'unknown',
        cpu_logical_cores: navigator.hardwareConcurrency || null,
        ram_reported_gb: navigator.deviceMemory ?? null,
        has_simd_wasm: simd,
        has_npu: false,
    };
}

export async function collectBrowserExecutionEnvironment({
    runner = 'wasm-browser',
    measurementPath = 'wasm_pipeline',
    engineVersion = null,
    daemonHealth = null,
} = {}) {
    const device_manifest = await collectDeviceManifest();
    const base = {
        runner,
        engine_version: engineVersion,
        memory_ceiling_mb: 512,
        measurement_path: measurementPath,
        topology: {
            mode: 'wasm_main_thread',
            worker_cells_configured: 1,
            worker_cells_active_during_run: 1,
            compute_swarm_enabled: false,
            cell_memory_floor_mb: 512,
            scheduling: 'serial',
        },
        device_manifest,
        collected_at: new Date().toISOString(),
    };

    if (daemonHealth?.execution_environment) {
        const d = daemonHealth.execution_environment;
        base.runner = d.runner || 'qualia-core-db daemon (browser client)';
        base.engine_version = daemonHealth.version || d.engine_version;
        base.measurement_path = 'daemon_http_query';
        base.topology = d.topology || base.topology;
        base.qualia_daemon_topology = d.topology;
    }

    return base;
}

export function formatTopologySummary(env) {
    if (!env?.topology) return '';
    const t = env.topology;
    const cells = t.worker_cells_configured ?? 1;
    const mode = t.mode || 'unknown';
    const swarm = t.compute_swarm_enabled ? ', swarm on' : '';
    if (cells > 1 || t.compute_swarm_enabled) {
        return `${cells} cell(s) · ${mode}${swarm}`;
    }
    return mode.replace(/_/g, ' ');
}

export function formatDeviceSummary(env) {
    const dm = env?.device_manifest;
    const ci = env?.ci_environment;
    if (!dm && !ci) return '';

    if (ci?.provider === 'github-actions') {
        const parts = ['GitHub Actions'];
        if (ci.runner_os) parts.push(ci.runner_os);
        if (ci.runner_arch) parts.push(ci.runner_arch);
        if (ci.runner_name && !ci.runner_name.startsWith('GitHub Actions')) {
            parts.push(ci.runner_name);
        }
        if (dm?.cpu_logical_cores) parts.push(`${dm.cpu_logical_cores} cores`);
        if (dm?.ram_reported_gb) parts.push(`${dm.ram_reported_gb} GB RAM`);
        return parts.join(' · ');
    }

    if (!dm) return '';
    const parts = [dm.host_class, dm.os].filter(Boolean);
    if (dm.cpu_logical_cores) parts.push(`${dm.cpu_logical_cores} cores`);
    if (dm.ram_reported_gb) parts.push(`${dm.ram_reported_gb} GB RAM`);
    return parts.join(' · ');
}

export function formatDaemonBadgeText(health) {
    if (!health) return 'Daemon Offline';
    const ver = health.version ? `v${health.version}` : '';
    const mode = health.dev_mode === true
        ? 'dev'
        : health.dev_mode === false
            ? 'standard'
            : null;
    const topo = health.execution_environment?.topology;
    if (topo) {
        const cells = topo.worker_cells_configured ?? 1;
        if (cells > 1 || topo.compute_swarm_enabled) {
            const swarm = topo.compute_swarm_enabled ? ' · swarm' : '';
            const modeLabel = mode ? ` · ${mode}` : '';
            return `Daemon ${ver}${modeLabel} · ${cells} cells${swarm}`.trim();
        }
    }
    return `Daemon ${ver}${mode ? ` · ${mode}` : ''}`.trim();
}
