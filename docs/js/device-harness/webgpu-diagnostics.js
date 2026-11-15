import { boundedText } from './report.js';

export const ADAPTER_DIAGNOSTICS_VERSION = 1;

export async function probeWebGpuAdapters(navigator, realm, timeoutMs = 2500) {
  const result = { realm, api_available: !!navigator?.gpu, gpu_execution: false, attempts: [] };
  if (!result.api_available) return result;
  for (const featureLevel of ['core', 'compatibility']) {
    const request = { featureLevel, forceFallbackAdapter: false };
    let timer;
    try {
      const attempt = await Promise.race([
        Promise.resolve().then(() => navigator.gpu.requestAdapter(request)).then(adapter => {
          if (!adapter) return { request, outcome: 'null' };
          const info = adapter.info;
          return { request, outcome: 'adapter',
            info: Object.fromEntries(['vendor', 'architecture', 'device', 'description'].map(key => [key, boundedText(info?.[key], 256)])),
            is_fallback_adapter: info?.isFallbackAdapter ?? adapter.isFallbackAdapter ?? null,
            core_features_and_limits: adapter.features?.has('core-features-and-limits') ?? null };
        }).catch(error => ({ request, outcome: 'error', error: boundedText(error?.message ?? error, 512) })),
        new Promise(resolve => { timer = setTimeout(() => resolve({ request, outcome: 'timeout' }), timeoutMs); }),
      ]);
      result.attempts.push(attempt);
      if (attempt.outcome === 'timeout') break;
    } finally { clearTimeout(timer); }
  }
  return result;
}

export function probeWebGlRenderer(document) {
  let gl;
  try {
    gl = document.createElement('canvas').getContext('webgl');
    if (!gl) return { outcome: 'unavailable', gpu_execution: false };
    const extension = gl.getExtension('WEBGL_debug_renderer_info');
    const renderer = boundedText(extension ? gl.getParameter(extension.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER), 512);
    const version = boundedText(gl.getParameter(gl.VERSION), 256);
    return { outcome: 'observed', gpu_execution: false, source: 'WebGL renderer inventory; no compute workload',
      unmasked: !!extension, renderer, version,
      powervr_25_1_string_observed: /PowerVR.*\b25\.1(?:\D|$)/i.test(`${renderer} ${version}`),
      limitation: 'WebGL strings do not establish the WebGPU driver or its blocklist status' };
  } catch (error) {
    return { outcome: 'error', gpu_execution: false, error: boundedText(error?.message ?? error, 256) };
  } finally {
    try { gl?.getExtension('WEBGL_lose_context')?.loseContext(); } catch {}
  }
}

export function summarizeAdapterDiagnostics(diagnostics) {
  const available = probe => probe?.attempts?.some(attempt => attempt.outcome === 'adapter');
  const refused = probe => probe?.api_available && probe.attempts?.length === 2
    && probe.attempts.every(attempt => attempt.outcome === 'null');
  if (available(diagnostics.worker)) return 'raw_worker_adapter_available_after_rust_refusal';
  if (available(diagnostics.window)) return 'window_adapter_available_worker_unavailable';
  if (refused(diagnostics.worker) && refused(diagnostics.window)) return 'browser_refused_core_and_compatibility_in_both_contexts';
  return 'inconclusive';
}
