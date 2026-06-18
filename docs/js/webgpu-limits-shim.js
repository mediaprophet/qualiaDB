// WebGPU device-limit compatibility shim for Qualia's native WASM engine.
//
// Qualia's WASM build links wgpu 0.19, whose WebGPU backend serializes the
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
