/**
 * Ambient Intelligence visualization — cymatic telemetry field for GitHub Pages demos.
 * Mirrors webizen-render SystemTelemetry (48-byte GPU contract) in a canvas2d path
 * until webizen-web wgpu ships on Pages.
 */

export const TELEMETRY_KEYS = [
    ['memory_pressure', 'Memory pressure', 'Compresses the particle nebula toward a dense core'],
    ['network_ripple', 'Network ripple', 'Sweeping holographic waves through the volume'],
    ['baking_crystallization', 'Ontology baking', 'Morphs chaos into crystalline lattice order'],
    ['logic_flashes', 'Logic flashes', 'Sharp arcs when queries / rules resolve'],
    ['llm_heat', 'LLM heat', 'High-frequency vibration in the inference cluster'],
    ['quantum_activity', 'Quantum activity', 'Phase tunneling in superposed regions'],
    ['spectral_shift', 'Spectral shift', 'Chromatic drift across α/μ/σ payload bands'],
    ['temporal_pulse', 'Temporal pulse', 'Radial waves from provenance time-slices'],
    ['epistemic_density', 'Epistemic density', 'Clustering by knowledge certainty'],
    ['manifold_pressure', 'Manifold pressure', 'Overall radial breathing of the field'],
];

export function defaultTelemetry() {
    return {
        memory_pressure: 0.12,
        network_ripple: 0.08,
        baking_crystallization: 0.2,
        logic_flashes: 0,
        llm_heat: 0,
        quantum_activity: 0.15,
        spectral_shift: 0.25,
        temporal_pulse: 0.1,
        epistemic_density: 0.3,
        manifold_pressure: 0.18,
    };
}

function clamp01(v) {
    return Math.max(0, Math.min(1, v));
}

function lerp(a, b, t) {
    return a + (b - a) * t;
}

function hash(i) {
    const x = Math.sin(i * 127.1 + 311.7) * 43758.5453;
    return x - Math.floor(x);
}

export class AmbientViz {
    constructor(canvas, options = {}) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d', { alpha: true });
        this.particleCount = options.particleCount ?? 2400;
        this.telemetry = { ...defaultTelemetry(), ...(options.telemetry || {}) };
        this.targetTelemetry = { ...this.telemetry };
        this.time = 0;
        this.running = false;
        this.flashTimer = 0;
        this.flashPairs = [];
        this._resizeObserver = null;
        this._onResize = options.onResize;
        this._initParticles();
        this._bindResize();
    }

    _initParticles() {
        this.base = new Float32Array(this.particleCount * 3);
        this.lattice = new Float32Array(this.particleCount * 3);
        for (let i = 0; i < this.particleCount; i++) {
            const u = hash(i);
            const v = hash(i + 17);
            const w = hash(i + 41);
            const r = 0.35 + u * 0.65;
            const theta = v * Math.PI * 2;
            const phi = Math.acos(2 * w - 1);
            const x = r * Math.sin(phi) * Math.cos(theta);
            const y = r * Math.sin(phi) * Math.sin(theta);
            const z = r * Math.cos(phi);
            const o = i * 3;
            this.base[o] = x;
            this.base[o + 1] = y;
            this.base[o + 2] = z;

            const gx = (i % 20) / 19 - 0.5;
            const gy = (Math.floor(i / 20) % 20) / 19 - 0.5;
            const gz = (Math.floor(i / 400) % 6) / 5 - 0.5;
            this.lattice[o] = gx * 0.9;
            this.lattice[o + 1] = gy * 0.9;
            this.lattice[o + 2] = gz * 0.9;
        }
    }

    _bindResize() {
        const ro = new ResizeObserver(() => this.resize());
        ro.observe(this.canvas.parentElement || this.canvas);
        this._resizeObserver = ro;
        this.resize();
    }

    resize() {
        const parent = this.canvas.parentElement;
        if (!parent) return;
        const w = parent.clientWidth || 800;
        const h = parent.clientHeight || 520;
        const dpr = Math.min(window.devicePixelRatio || 1, 2);
        this.canvas.width = Math.floor(w * dpr);
        this.canvas.height = Math.floor(h * dpr);
        this.canvas.style.width = `${w}px`;
        this.canvas.style.height = `${h}px`;
        this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        this.width = w;
        this.height = h;
        this.cx = w * 0.5;
        this.cy = h * 0.5;
        this.scale = Math.min(w, h) * 0.38;
        this._onResize?.(w, h);
    }

    setTelemetry(partial) {
        for (const [k] of TELEMETRY_KEYS) {
            if (partial[k] !== undefined) {
                this.targetTelemetry[k] = clamp01(partial[k]);
            }
        }
    }

    /** Brief spike when Qualia WASM work completes (encode, spatial op, etc.) */
    pulse(metric, amount = 0.85) {
        if (this.targetTelemetry[metric] !== undefined) {
            this.targetTelemetry[metric] = Math.max(this.targetTelemetry[metric], amount);
        }
        if (metric === 'logic_flashes') {
            this._spawnFlashes(6 + Math.floor(amount * 8));
        }
    }

    _spawnFlashes(n) {
        this.flashPairs = [];
        for (let i = 0; i < n; i++) {
            this.flashPairs.push([
                Math.floor(hash(this.time + i) * this.particleCount),
                Math.floor(hash(this.time + i + 99) * this.particleCount),
            ]);
        }
        this.flashTimer = 0.35;
    }

    _smoothTelemetry(dt) {
        const rate = 1 - Math.pow(0.02, dt * 60);
        for (const [k] of TELEMETRY_KEYS) {
            this.telemetry[k] = lerp(this.telemetry[k], this.targetTelemetry[k], rate);
            if (k !== 'logic_flashes') {
                this.targetTelemetry[k] *= 0.992;
            }
        }
    }

    _project(x, y, z, t) {
        const tlm = this.telemetry;
        const breath = 1 + Math.sin(t * 1.2) * 0.04 * tlm.manifold_pressure;
        const compress = 1 - tlm.memory_pressure * 0.45;
        const ripple = Math.sin(t * 2.5 + x * 4 + z * 3) * tlm.network_ripple * 0.12;
        const heatJitter = Math.sin(t * 18 + y * 20) * tlm.llm_heat * 0.06;
        const quantum = Math.sin(t * 7 + x * 11) * tlm.quantum_activity * 0.05;
        const pulseR = Math.sin(t * 3 - Math.hypot(x, y) * 5) * tlm.temporal_pulse * 0.08;

        const px = (x * compress + ripple + heatJitter) * breath + pulseR;
        const py = (y * compress + quantum) * breath;
        const pz = (z * compress - ripple * 0.5) * breath;

        const focal = 2.8;
        const s = focal / (focal + pz);
        return {
            sx: this.cx + px * this.scale * s,
            sy: this.cy + py * this.scale * s,
            depth: pz,
            s,
        };
    }

    _spectralColor(i, depth, t) {
        const tlm = this.telemetry;
        const baseHue = 0.52 + hash(i) * 0.12 + tlm.spectral_shift * 0.35;
        const heatHue = tlm.llm_heat * 0.08 * Math.sin(t * 4 + i * 0.01);
        const hue = (baseHue + heatHue + depth * 0.05) % 1;
        const sat = 0.55 + tlm.epistemic_density * 0.35;
        const lit = 0.45 + (1 - depth) * 0.25 + tlm.llm_heat * 0.2;
        return `hsla(${Math.floor(hue * 360)}, ${Math.floor(sat * 100)}%, ${Math.floor(lit * 100)}%,`;
    }

    _tick(dt) {
        this.time += dt;
        this._smoothTelemetry(dt);
        if (this.flashTimer > 0) this.flashTimer -= dt;

        const ctx = this.ctx;
        const w = this.width;
        const h = this.height;
        const tlm = this.telemetry;
        const t = this.time;
        const crystal = tlm.baking_crystallization;

        // Pure-black, stronger per-frame fade: clears the additive ('lighter') particle
        // accumulation to a black background instead of letting warm hues build into a pink wash.
        ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
        ctx.fillRect(0, 0, w, h);

        const points = [];
        for (let i = 0; i < this.particleCount; i++) {
            const o = i * 3;
            let x = lerp(this.base[o], this.lattice[o], crystal);
            let y = lerp(this.base[o + 1], this.lattice[o + 1], crystal);
            let z = lerp(this.base[o + 2], this.lattice[o + 2], crystal);

            const cluster = hash(i + 3) < tlm.epistemic_density * 0.35;
            if (cluster) {
                const cx = Math.sin(t * 0.3) * 0.2;
                const cy = Math.cos(t * 0.25) * 0.15;
                x = lerp(x, cx, 0.35);
                y = lerp(y, cy, 0.35);
            }

            const drift = Math.sin(t * 0.8 + i * 0.002) * 0.02;
            x += drift;
            y += Math.cos(t * 0.6 + i * 0.003) * 0.02;

            const p = this._project(x, y, z, t);
            points.push({ ...p, i, z });
        }

        points.sort((a, b) => a.depth - b.depth);

        ctx.globalCompositeOperation = 'lighter';
        for (const p of points) {
            const alpha = 0.15 + p.s * 0.35 + tlm.llm_heat * 0.15;
            const r = (0.6 + p.s * 1.4) * (1 + tlm.manifold_pressure * 0.3);
            const color = this._spectralColor(p.i, p.z, t);
            ctx.beginPath();
            ctx.fillStyle = `${color}${alpha.toFixed(3)})`;
            ctx.arc(p.sx, p.sy, r, 0, Math.PI * 2);
            ctx.fill();
        }

        if (this.flashTimer > 0 && tlm.logic_flashes > 0.05) {
            const byIndex = new Array(this.particleCount);
            for (const p of points) byIndex[p.i] = p;
            ctx.strokeStyle = `rgba(52, 211, 153, ${(this.flashTimer / 0.35) * tlm.logic_flashes})`;
            ctx.lineWidth = 1.2;
            for (const [a, b] of this.flashPairs) {
                const pa = byIndex[a];
                const pb = byIndex[b];
                if (!pa || !pb) continue;
                ctx.beginPath();
                ctx.moveTo(pa.sx, pa.sy);
                ctx.lineTo(pb.sx, pb.sy);
                ctx.stroke();
            }
        }

        ctx.globalCompositeOperation = 'source-over';
    }

    start() {
        if (this.running) return;
        this.running = true;
        let last = performance.now();
        const loop = (now) => {
            if (!this.running) return;
            const dt = Math.min((now - last) / 1000, 0.05);
            last = now;
            this._tick(dt);
            this._raf = requestAnimationFrame(loop);
        };
        this._raf = requestAnimationFrame(loop);
    }

    stop() {
        this.running = false;
        if (this._raf) cancelAnimationFrame(this._raf);
    }

    destroy() {
        this.stop();
        this._resizeObserver?.disconnect();
    }
}

export function bindTelemetrySliders(container, viz, onChange) {
    if (!container) return;
    const telem = defaultTelemetry();
    container.innerHTML = TELEMETRY_KEYS.map(([key, label, hint]) => `
        <div class="mb-3">
            <div class="flex justify-between text-xs mb-1">
                <span class="text-white/70">${label}</span>
                <span class="text-emerald-400 font-mono" data-val="${key}">${(telem[key] * 100).toFixed(0)}%</span>
            </div>
            <input type="range" min="0" max="100" value="${Math.round(telem[key] * 100)}"
                data-telem="${key}" class="w-full accent-emerald-500" title="${hint}">
        </div>
    `).join('');

    container.querySelectorAll('input[data-telem]').forEach((input) => {
        input.addEventListener('input', () => {
            const key = input.dataset.telem;
            const v = parseInt(input.value, 10) / 100;
            viz.setTelemetry({ [key]: v });
            const valEl = container.querySelector(`[data-val="${key}"]`);
            if (valEl) valEl.textContent = `${input.value}%`;
            onChange?.(key, v);
        });
    });
}