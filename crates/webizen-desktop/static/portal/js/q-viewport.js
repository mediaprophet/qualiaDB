/**
 * <q-viewport> — Declarative GPU viewport custom element (plan §7.3 W5).
 *
 * A Web Component that mounts a <canvas>, detects the GPU backend (WebGPU
 * first, WebGL2 fallback), initializes the Qualia portal, and runs a reactive
 * frame loop driving Render.gpu_render_frame via VibeScript capability.invoke.
 *
 * ## Attributes
 *
 * - `width` (default: 800) — canvas width in physical pixels
 * - `height` (default: 600) — canvas height in physical pixels
 * - `target-fps` (default: 60) — target frame rate
 * - `auto-mount` (default: true) — whether to auto-mount on connect
 *
 * ## Events
 *
 * - `q-viewport:mount` — fired when the GPU is initialized
 * - `q-viewport:frame` — fired on each frame render
 * - `q-viewport:unmount` — fired when the GPU is destroyed
 * - `q-viewport:error` — fired on errors
 *
 * ## Usage
 *
 * ```html
 * <q-viewport width="1024" height="768" target-fps="60"></q-viewport>
 * ```
 *
 * ## Backend selection
 *
 * 1. Probes `navigator.gpu` for WebGPU availability.
 * 2. Falls back to WebGL2 via canvas.getContext('webgl2').
 * 3. Falls back to Canvas 2D if no GPU context is available.
 *
 * The frame loop calls `Render.gpu_render_frame` through the VibeScript
 * `capability.invoke` system, which is exposed via the WASM module.
 */
class QViewport extends HTMLElement {
  constructor() {
    super();
    this._canvas = null;
    this._gpuHandle = null;
    this._backend = 'none';
    this._frameCount = 0;
    this._simTime = 0.0;
    this._running = false;
    this._rafId = null;
    this._camera = { yaw: 0.0, pitch: -0.3, zoom: 1.0 };
    this._dragging = false;
    this._lastX = 0;
    this._lastY = 0;
  }

  static get observedAttributes() {
    return ['width', 'height', 'target-fps', 'auto-mount'];
  }

  get width() { return parseInt(this.getAttribute('width') || '800', 10); }
  get height() { return parseInt(this.getAttribute('height') || '600', 10); }
  get targetFps() { return parseInt(this.getAttribute('target-fps') || '60', 10); }
  get autoMount() { return this.getAttribute('auto-mount') !== 'false'; }

  connectedCallback() {
    this.style.display = 'block';
    this.style.position = 'relative';
    this.style.overflow = 'hidden';
    this.style.background = '#000';

    // Create the canvas element.
    this._canvas = document.createElement('canvas');
    this._canvas.width = this.width;
    this._canvas.height = this.height;
    this._canvas.style.width = '100%';
    this._canvas.style.height = '100%';
    this._canvas.style.cursor = 'grab';
    this.appendChild(this._canvas);

    // Bind mouse events for camera control.
    this._canvas.addEventListener('mousedown', this._onMouseDown.bind(this));
    this._canvas.addEventListener('mousemove', this._onMouseMove.bind(this));
    this._canvas.addEventListener('mouseup', this._onMouseUp.bind(this));
    this._canvas.addEventListener('mouseleave', this._onMouseUp.bind(this));
    this._canvas.addEventListener('wheel', this._onWheel.bind(this));

    // Observe resize.
    if (typeof ResizeObserver !== 'undefined') {
      this._resizeObserver = new ResizeObserver(this._onResize.bind(this));
      this._resizeObserver.observe(this);
    }

    if (this.autoMount) {
      this.mount();
    }
  }

  disconnectedCallback() {
    this.unmount();
    if (this._resizeObserver) {
      this._resizeObserver.disconnect();
      this._resizeObserver = null;
    }
  }

  attributeChangedCallback(name, oldVal, newVal) {
    if (oldVal === newVal) return;
    if (name === 'width' || name === 'height') {
      if (this._canvas) {
        this._canvas.width = this.width;
        this._canvas.height = this.height;
      }
      if (this._gpuHandle !== null) {
        this._invokeVibeScript(`Render.gpu_resize`, {
          handle: this._gpuHandle,
          width: this.width,
          height: this.height,
        });
      }
    }
  }

  /**
   * Detect the available GPU backend.
   * @returns {Promise<string>} 'webgpu' | 'webgl2' | 'canvas2d' | 'none'
   */
  async detectBackend() {
    // Probe WebGPU.
    if (typeof navigator !== 'undefined' && navigator.gpu) {
      try {
        const adapter = await navigator.gpu.requestAdapter();
        if (adapter) return 'webgpu';
      } catch (e) {
        console.warn('[q-viewport] WebGPU probe failed:', e);
      }
    }
    // Probe WebGL2.
    if (this._canvas) {
      const gl = this._canvas.getContext('webgl2');
      if (gl) {
        gl.dispose && gl.dispose();
        return 'webgl2';
      }
    }
    // Canvas 2D fallback.
    if (this._canvas && this._canvas.getContext('2d')) {
      return 'canvas2d';
    }
    return 'none';
  }

  /**
   * Mount the GPU viewport — detect backend, init GPU, start frame loop.
   */
  async mount() {
    try {
      this._backend = await this.detectBackend();
      console.log(`[q-viewport] Backend: ${this._backend}`);

      if (this._backend === 'none') {
        this._emitError('No GPU backend available');
        return;
      }

      // Init GPU via VibeScript capability.invoke.
      const initResult = await this._invokeVibeScript('Render.gpu_init', {
        width: this.width,
        height: this.height,
        particle_cap: 4096,
      });

      if (initResult && initResult.handle !== undefined) {
        this._gpuHandle = initResult.handle;
        this._running = true;
        this._frameCount = 0;
        this._simTime = 0.0;

        // Send initial camera state.
        await this._invokeVibeScript('Render.gpu_set_camera', {
          handle: this._gpuHandle,
          yaw: this._camera.yaw,
          pitch: this._camera.pitch,
          zoom: this._camera.zoom,
        });

        this._emit('q-viewport:mount', { handle: this._gpuHandle, backend: this._backend });
        this._startFrameLoop();
      } else {
        this._emitError('gpu_init did not return a handle');
      }
    } catch (e) {
      this._emitError(`Mount failed: ${e}`);
    }
  }

  /**
   * Unmount the GPU viewport — stop frame loop, destroy GPU.
   */
  async unmount() {
    this._running = false;
    if (this._rafId !== null) {
      cancelAnimationFrame(this._rafId);
      this._rafId = null;
    }
    if (this._gpuHandle !== null) {
      try {
        await this._invokeVibeScript('Render.gpu_destroy', {
          handle: this._gpuHandle,
        });
      } catch (e) {
        console.warn('[q-viewport] gpu_destroy failed:', e);
      }
      this._gpuHandle = null;
      this._emit('q-viewport:unmount', {});
    }
  }

  /**
   * Start the reactive frame loop.
   */
  _startFrameLoop() {
    const frameDt = this.targetFps > 0 ? 1.0 / this.targetFps : 1.0 / 60.0;
    const tick = async () => {
      if (!this._running || this._gpuHandle === null) return;

      try {
        await this._invokeVibeScript('Render.gpu_render_frame', {
          handle: this._gpuHandle,
          time: this._simTime,
        });
        this._frameCount++;
        this._simTime += frameDt;
        this._emit('q-viewport:frame', {
          frame: this._frameCount,
          time: this._simTime,
        });
      } catch (e) {
        this._running = false;
        this._emitError(`Frame error: ${e}`);
        return;
      }

      if (this._running) {
        this._rafId = requestAnimationFrame(tick);
      }
    };
    this._rafId = requestAnimationFrame(tick);
  }

  /**
   * Call a VibeScript capability.invoke via the WASM runtime.
   * @param {string} invokeId — e.g. "Render.gpu_init"
   * @param {object} args — arguments to pass
   * @returns {Promise<object>} — the result value
   */
  async _invokeVibeScript(invokeId, args) {
    // The WASM runtime exposes a global `qualia_invoke` function.
    if (typeof window !== 'undefined' && window.qualia_invoke) {
      return await window.qualia_invoke(invokeId, args);
    }
    // Fallback: use the WASM module directly if available.
    if (typeof window !== 'undefined' && window.qualia_wasm) {
      const wasm = await window.qualia_wasm;
      if (wasm && wasm.invoke_capability) {
        return await wasm.invoke_capability(invokeId, JSON.stringify(args));
      }
    }
    throw new Error('No VibeScript invoke runtime available (window.qualia_invoke or window.qualia_wasm)');
  }

  _emit(name, detail) {
    this.dispatchEvent(new CustomEvent(name, { detail, bubbles: true }));
  }

  _emitError(msg) {
    console.error(`[q-viewport] ${msg}`);
    this._emit('q-viewport:error', { message: msg });
  }

  _onMouseDown(e) {
    this._dragging = true;
    this._lastX = e.clientX;
    this._lastY = e.clientY;
    this._canvas.style.cursor = 'grabbing';
  }

  _onMouseMove(e) {
    if (!this._dragging) return;
    const dx = e.clientX - this._lastX;
    const dy = e.clientY - this._lastY;
    this._lastX = e.clientX;
    this._lastY = e.clientY;
    this._camera.yaw += dx * 0.01;
    this._camera.pitch += dy * 0.01;
    this._camera.pitch = Math.max(-1.5, Math.min(1.5, this._camera.pitch));
    this._updateCamera();
  }

  _onMouseUp() {
    this._dragging = false;
    if (this._canvas) this._canvas.style.cursor = 'grab';
  }

  _onWheel(e) {
    e.preventDefault();
    const delta = e.deltaY;
    this._camera.zoom *= delta > 0 ? 0.9 : 1.1;
    this._camera.zoom = Math.max(0.1, Math.min(10.0, this._camera.zoom));
    this._updateCamera();
  }

  _updateCamera() {
    if (this._gpuHandle === null) return;
    this._invokeVibeScript('Render.gpu_set_camera', {
      handle: this._gpuHandle,
      yaw: this._camera.yaw,
      pitch: this._camera.pitch,
      zoom: this._camera.zoom,
    });
  }

  _onResize(entries) {
    for (const entry of entries) {
      const { width, height } = entry.contentRect;
      const w = Math.floor(width);
      const h = Math.floor(height);
      if (w > 0 && h > 0 && this._canvas) {
        this._canvas.width = w;
        this._canvas.height = h;
        if (this._gpuHandle !== null) {
          this._invokeVibeScript('Render.gpu_resize', {
            handle: this._gpuHandle,
            width: w,
            height: h,
          });
        }
      }
    }
  }
}

// Register the custom element.
if (typeof customElements !== 'undefined' && !customElements.get('q-viewport')) {
  customElements.define('q-viewport', QViewport);
}

// Export for module usage.
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { QViewport };
}
if (typeof window !== 'undefined') {
  window.QViewport = QViewport;
}
