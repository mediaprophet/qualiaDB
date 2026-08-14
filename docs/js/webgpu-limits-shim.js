// WebGPU device-limit compatibility shim for Qualia's native WASM engine.
//
// REVIEW(wasm-mobile-2026-08-02 F8): the runtime now links wgpu 30; retain this
// global monkey-patch only while a current-browser requestDevice receipt proves
// it is still necessary, then move adapter policy into the shared capability layer.
// The workaround originated with wgpu 0.19, whose WebGPU backend serialized the
// `maxInterStageShaderComponents` device limit into `GPUAdapter.requestDevice()`.
// That limit was removed from the WebGPU specification, and current Chrome/Edge
// reject `requestDevice()` with:
//
//   OperationError: Failed to execute 'requestDevice' on 'GPUAdapter':
//   The limit "maxInterStageShaderComponents" with a non-undefined value is not recognized.
//
// In a wasm32 build that rejection surfaces as a Rust panic inside the async
// engine-init future (gguf_bridge.rs `QTensorEngine::new_async().expect(...)`),
// which aborts the module and leaves the `initialize_webgpu_engine` promise
// pending forever — the page hangs on "Initialising Qualia WebGPU engine…".
//
// This shim wraps `requestDevice` and drops any requested limit the running
// browser no longer recognizes, so init succeeds on current and future engines
// alike. It is a no-op once the wasm crate is rebuilt against a newer wgpu that
// stops sending the removed limit. Load this as a classic <script> (which runs
// before deferred module scripts) so the patch is in place before the engine
// requests a device.
(function () {
  if (typeof navigator === 'undefined' || !navigator.gpu) return;

  // Android Chrome can expose both its Vulkan/Core and OpenGL ES/Compatibility
  // implementations. Try Compatibility first on Android: a failing Vulkan probe may
  // crash or disable the GPU process before the GLES adapter is requested. Qualia's
  // browser kernels stay within the compatibility compute/storage-buffer subset.
  // Desktop keeps wgpu's requested order and only falls back when it returns null.
  if (typeof GPU !== 'undefined') {
    const gpuProto = GPU.prototype;
    if (!gpuProto.__qualiaAdapterPatched) {
      const originalRequestAdapter = gpuProto.requestAdapter;
      gpuProto.requestAdapter = async function (options) {
        const isAndroid = /Android/i.test(navigator.userAgent || '');
        const alreadyCompatibility = options?.featureLevel === 'compatibility';
        const alreadyFallback = options?.forceFallbackAdapter === true;

        if (isAndroid && !alreadyCompatibility && !alreadyFallback) {
          console.debug('[qualia] Android WebGPU: trying compatibility adapter before Vulkan/Core');
          const compatibilityAdapter = await originalRequestAdapter.call(this, {
            featureLevel: 'compatibility',
          });
          if (compatibilityAdapter) return compatibilityAdapter;
        }

        const adapter = await originalRequestAdapter.call(this, options);
        if (adapter || alreadyFallback) return adapter;

        console.debug('[qualia] Requested WebGPU adapter unavailable; retrying default adapter');
        const defaultAdapter = await originalRequestAdapter.call(this);
        if (defaultAdapter) return defaultAdapter;

        if (!alreadyCompatibility) {
          console.debug('[qualia] WebGPU core adapter unavailable; trying compatibility adapter');
          const compatibilityAdapter = await originalRequestAdapter.call(this, {
            featureLevel: 'compatibility',
          });
          if (compatibilityAdapter) return compatibilityAdapter;
        }

        console.debug('[qualia] WebGPU compatibility adapter unavailable; trying low-power adapter');
        const lowPowerAdapter = await originalRequestAdapter.call(this, {
          powerPreference: 'low-power',
        });
        if (lowPowerAdapter) return lowPowerAdapter;

        console.debug('[qualia] WebGPU hardware adapters unavailable; trying software fallback adapter');
        return originalRequestAdapter.call(this, { forceFallbackAdapter: true });
      };
      Object.defineProperty(gpuProto, '__qualiaAdapterPatched', { value: true });
    }
  }

  if (typeof GPUAdapter === 'undefined') return;
  const proto = GPUAdapter.prototype;
  if (proto.__qualiaLimitsPatched) return;

  const originalRequestDevice = proto.requestDevice;
  proto.requestDevice = function (descriptor) {
    if (descriptor && descriptor.requiredLimits) {
      const supported = this.limits; // GPUSupportedLimits for this adapter
      const filtered = {};
      let dropped = 0;
      for (const key in descriptor.requiredLimits) {
        const value = descriptor.requiredLimits[key];
        if (value !== undefined && key in supported) {
          filtered[key] = value;
        } else if (value !== undefined) {
          dropped++;
        }
      }
      if (dropped > 0) {
        console.debug(`[qualia] webgpu-limits-shim: dropped ${dropped} unrecognized device limit(s) before requestDevice`);
      }
      descriptor = Object.assign({}, descriptor, { requiredLimits: filtered });
    }
    return originalRequestDevice.call(this, descriptor);
  };

  Object.defineProperty(proto, '__qualiaLimitsPatched', { value: true });
})();
