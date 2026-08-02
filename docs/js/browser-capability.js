/**
 * Qualia browser capability contract.
 *
 * Capability discovery is cold-path work. It intentionally keeps WebGPU,
 * WebGL2, and CPU-WASM facilities independent: a phone may have accelerated
 * WebGL2 while Chrome suppresses every WebGPU adapter.
 */

export const BROWSER_CAPABILITY_SCHEMA = 'qualia.browser-capability.v1';

export const WEBGPU_ADAPTER_ATTEMPTS = Object.freeze([
  Object.freeze({ id: 'compatibility-default', options: Object.freeze({ featureLevel: 'compatibility' }) }),
  Object.freeze({ id: 'core-high-performance', options: Object.freeze({ featureLevel: 'core', powerPreference: 'high-performance' }) }),
  Object.freeze({ id: 'core-low-power', options: Object.freeze({ featureLevel: 'core', powerPreference: 'low-power' }) }),
  Object.freeze({ id: 'default', options: Object.freeze({}) }),
]);

const SIMD_PROBE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7b,
  0x03, 0x02, 0x01, 0x00,
  0x0a, 0x16, 0x01, 0x14, 0x00, 0xfd, 0x0c,
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  0x0b,
]);

const THREADS_PROBE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
  0x03, 0x02, 0x01, 0x00,
  0x05, 0x04, 0x01, 0x03, 0x01, 0x01,
  0x0a, 0x0d, 0x01, 0x0b, 0x00, 0x41, 0x00, 0x41, 0x00,
  0xfe, 0x00, 0x02, 0x00, 0x1a, 0x0b,
]);

const boundedError = (error) => String(error?.message || error || 'unknown').slice(0, 400);

function validateWasmFeature(webAssembly, bytes) {
  try {
    return Boolean(webAssembly?.validate?.(bytes));
  } catch {
    return false;
  }
}

function adapterInfo(adapter) {
  const info = adapter?.info;
  if (!info) return null;
  return {
    vendor: String(info.vendor || ''),
    architecture: String(info.architecture || ''),
    device: String(info.device || ''),
    description: String(info.description || ''),
  };
}

function selectedLimits(adapter) {
  const limits = adapter?.limits;
  if (!limits) return null;
  const keys = [
    'maxBufferSize',
    'maxStorageBufferBindingSize',
    'maxComputeInvocationsPerWorkgroup',
    'maxComputeWorkgroupStorageSize',
    'maxComputeWorkgroupsPerDimension',
  ];
  return Object.fromEntries(keys.map((key) => [key, Number(limits[key] || 0)]));
}

function probeWebGl2(documentObject) {
  const outcome = { state: 'api_absent', available: false, error: null };
  try {
    const canvas = documentObject?.createElement?.('canvas');
    if (!canvas?.getContext) return outcome;
    const gl = canvas.getContext('webgl2', {
      alpha: false,
      antialias: true,
      depth: true,
      failIfMajorPerformanceCaveat: false,
    });
    if (!gl) return { ...outcome, state: 'webgl2_context_unavailable' };
    let renderer = '';
    try {
      const ext = gl.getExtension?.('WEBGL_debug_renderer_info');
      renderer = ext ? String(gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) || '') : '';
    } catch {
      renderer = '';
    }
    gl.getExtension?.('WEBGL_lose_context')?.loseContext?.();
    return { state: 'available', available: true, renderer, error: null };
  } catch (error) {
    return { ...outcome, state: 'webgl2_context_unavailable', error: boundedError(error) };
  }
}

async function probeWebGpu(navigatorObject, attempts) {
  const gpu = navigatorObject?.gpu;
  const result = {
    apiPresent: Boolean(gpu),
    state: gpu ? 'adapter_unavailable' : 'api_absent',
    adapterAvailable: false,
    adapterAttempt: null,
    attempts: [],
    info: null,
    limits: null,
    features: [],
    device: { state: 'deferred_to_backend', acquired: false, error: null },
  };
  if (!gpu?.requestAdapter) return result;

  for (const attempt of attempts) {
    const record = { id: attempt.id, options: { ...attempt.options }, state: 'adapter_unavailable', error: null };
    try {
      const adapter = await gpu.requestAdapter({ ...attempt.options });
      if (!adapter) {
        result.attempts.push(record);
        continue;
      }
      record.state = 'available';
      result.attempts.push(record);
      result.state = 'available';
      result.adapterAvailable = true;
      result.adapterAttempt = attempt.id;
      result.info = adapterInfo(adapter);
      result.limits = selectedLimits(adapter);
      result.features = Array.from(adapter.features || [], String).sort();
      return result;
    } catch (error) {
      record.state = 'adapter_request_failed';
      record.error = boundedError(error);
      result.attempts.push(record);
    }
  }
  return result;
}

export function selectBrowserBackends(receipt) {
  const webgpu = receipt?.webgpu?.adapterAvailable === true;
  const webgl2 = receipt?.webgl2?.available === true;
  const wasm = receipt?.wasm?.available === true;
  return {
    llm: webgpu ? 'webgpu' : wasm ? 'cpu-wasm' : 'unsupported',
    llmReason: webgpu ? 'adapter_available' : wasm ? receipt?.webgpu?.state || 'webgpu_unavailable' : 'wasm_unavailable',
    anatomy: webgpu ? 'webgpu' : webgl2 ? 'webgl2' : 'unsupported',
    anatomyReason: webgpu ? 'adapter_available' : webgl2 ? receipt?.webgpu?.state || 'webgpu_unavailable' : 'no_hardware_renderer',
  };
}

export async function probeBrowserCapabilities(options = {}) {
  const globalObject = options.globalObject || globalThis;
  const navigatorObject = options.navigatorObject || globalObject.navigator || {};
  const documentObject = options.documentObject || globalObject.document || null;
  const webAssembly = options.webAssembly || globalObject.WebAssembly;
  const crossOriginIsolated = options.crossOriginIsolated ?? globalObject.crossOriginIsolated === true;
  const sharedArrayBuffer = typeof (options.SharedArrayBuffer || globalObject.SharedArrayBuffer) === 'function';
  const worker = typeof (options.Worker || globalObject.Worker) === 'function';
  const wasmAvailable = typeof webAssembly === 'object' && typeof webAssembly.validate === 'function';
  const simd = wasmAvailable && validateWasmFeature(webAssembly, SIMD_PROBE);
  const atomics = wasmAvailable && validateWasmFeature(webAssembly, THREADS_PROBE);

  const receipt = {
    schema: BROWSER_CAPABILITY_SCHEMA,
    engineVersion: String(options.engineVersion || 'unknown'),
    artifactHash: String(options.artifactHash || 'unknown'),
    sessionId: String(options.sessionId || ''),
    observedAt: new Date(options.now ?? Date.now()).toISOString(),
    secureContext: options.secureContext ?? globalObject.isSecureContext === true,
    crossOriginIsolated,
    webgpu: await probeWebGpu(navigatorObject, options.adapterAttempts || WEBGPU_ADAPTER_ATTEMPTS),
    webgl2: probeWebGl2(documentObject),
    wasm: {
      available: wasmAvailable,
      simd,
      worker,
      sharedArrayBuffer,
      atomics,
      threads: crossOriginIsolated && sharedArrayBuffer && atomics && worker,
      hardwareConcurrency: Math.max(1, Number(navigatorObject.hardwareConcurrency || 1)),
    },
    selection: null,
  };
  receipt.selection = selectBrowserBackends(receipt);
  return receipt;
}
let sharedProbe = null;

export function getBrowserCapabilityReceipt(options = {}) {
  if (options.refresh || !sharedProbe) {
    sharedProbe = probeBrowserCapabilities(options).catch((error) => {
      sharedProbe = null;
      throw error;
    });
  }
  return sharedProbe;
}

export function recordBackendDeviceOutcome(receipt, subsystem, outcome) {
  if (!receipt || !subsystem) return receipt;
  const target = subsystem === 'anatomy' ? 'anatomy' : 'llm';
  receipt[`${target}Backend`] = {
    backend: String(outcome?.backend || receipt.selection?.[target] || 'unsupported'),
    state: String(outcome?.state || 'unsupported'),
    error: outcome?.error ? boundedError(outcome.error) : null,
  };
  if (outcome?.backend === 'webgpu') {
    receipt.webgpu.device = {
      state: receipt[`${target}Backend`].state,
      acquired: receipt[`${target}Backend`].state === 'available',
      error: receipt[`${target}Backend`].error,
    };
  }
  return receipt;
}
