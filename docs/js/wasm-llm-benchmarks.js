import {
    collectBrowserExecutionEnvironment,
    formatDeviceSummary,
    formatTopologySummary,
} from './benchmark-environment.js';
import { ensureCrossOriginIsolation } from './qualia-coi.js';

const TRANSFORMERS_CDN = 'https://cdn.jsdelivr.net/npm/@huggingface/transformers@3.7.6/+esm';
const WEBLLM_CDN = 'https://esm.run/@mlc-ai/web-llm';

const DEFAULT_PROMPT = 'Summarize why zero-copy data layouts matter for edge inference in 3 concise bullet points.';

const ENGINE_DEFS = [
    {
        id: 'webllm',
        label: 'WebLLM',
        family: 'MLC / WebGPU',
        live: true,
        kind: 'ready',
        description: 'Real browser adapter via the official WebLLM CDN path. Uses the prebuilt model list and prefers the smallest built-in instruct model it can discover.',
        controls: [
            {
                key: 'model',
                label: 'Model ID',
                type: 'text',
                value: 'auto-smallest-prebuilt',
                placeholder: 'auto-smallest-prebuilt',
            },
        ],
    },
    {
        id: 'transformersjs',
        label: 'Transformers.js',
        family: 'ONNX / WASM / WebGPU',
        live: true,
        kind: 'ready',
        description: 'Real pipeline-based adapter. Starts with a deliberately small text-generation baseline so the page stays usable in-browser.',
        controls: [
            {
                key: 'model',
                label: 'Model ID',
                type: 'text',
                value: 'Xenova/distilgpt2',
                placeholder: 'onnx-community/SmolLM2-360M-Instruct',
            },
            {
                key: 'device',
                label: 'Device',
                type: 'select',
                value: 'webgpu',
                options: [
                    { value: 'webgpu', label: 'webgpu' },
                    { value: 'wasm', label: 'wasm' },
                ],
            },
            {
                key: 'dtype',
                label: 'dtype',
                type: 'select',
                value: 'q4',
                options: [
                    { value: 'q4', label: 'q4' },
                    { value: 'q8', label: 'q8' },
                    { value: 'fp32', label: 'fp32' },
                ],
            },
        ],
    },
    {
        id: 'wllama',
        label: 'Wllama',
        family: 'llama.cpp / WASM',
        live: false,
        kind: 'shell',
        description: 'Official browser path exists, but this page intentionally leaves the bootstrap unpinned until we vendor or explicitly choose the ESM/CDN strategy for the wasm assets.',
        controls: [
            {
                key: 'repo',
                label: 'HF repo',
                type: 'text',
                value: 'ggml-org/models',
                placeholder: 'org/repo',
            },
            {
                key: 'file',
                label: 'HF file',
                type: 'text',
                value: 'tinyllamas/stories260K.gguf',
                placeholder: 'model.gguf',
            },
        ],
        waitingOn: 'Pin the browser import + wasm asset delivery path before enabling live runs here.',
    },
    {
        id: 'ratchet',
        label: 'Ratchet',
        family: 'Rust / WebGPU',
        live: false,
        kind: 'shell',
        description: 'Space reserved for a browser benchmark lane once a stable public browser bootstrap is chosen for this docs surface.',
        controls: [],
        waitingOn: 'Need a concrete browser package/bootstrap path for the docs page.',
    },
    {
        id: 'litert',
        label: 'LiteRT / TensorFlow.js',
        family: 'Edge browser ML',
        live: false,
        kind: 'shell',
        description: 'Reserved lane for a TensorFlow.js or LiteRT-backed browser benchmark path once the model packaging decision is made.',
        controls: [],
        waitingOn: 'Need a pinned browser LLM packaging path and comparable model selection.',
    },
    {
        id: 'qualia',
        label: 'Qualia',
        family: 'GGUF→.q42 / WebGPU (native)',
        live: true,
        kind: 'ready',
        description: 'Qualia native WASM + WebGPU engine. AOT-compiles GGUF→.q42 (16KB page-aligned, OPFS-cached, version-keyed) and boots zero-parse from the Q42W container, or boots a GGUF directly. Greedy/argmax decode (temperature ignored); fixed decode budget.',
        controls: [
            {
                key: 'model',
                label: 'Model URL',
                type: 'text',
                value: 'https://huggingface.co/HuggingFaceTB/SmolLM2-360M-Instruct-GGUF/resolve/main/smollm2-360m-instruct-q4_k_m.gguf',
                placeholder: 'https://…/model.gguf  (or same-origin models/x.gguf)',
            },
            {
                key: 'format',
                label: 'Container',
                type: 'select',
                value: 'q42',
                options: [
                    { value: 'q42', label: '.q42 (AOT, OPFS-cached)' },
                    { value: 'gguf', label: 'GGUF (direct)' },
                ],
            },
        ],
    },
];

const dom = {};
const state = {
    env: null,
    gpuName: 'Unavailable',
    engines: ENGINE_DEFS.map((def) => ({
        ...def,
        config: Object.fromEntries((def.controls || []).map((control) => [control.key, control.value])),
        statusText: def.live ? 'Not run yet' : def.waitingOn,
        errorText: '',
        running: false,
        result: null,
    })),
    adapters: new Map(),
    logLines: [],
};

function $(id) {
    return document.getElementById(id);
}

function escapeHtml(value) {
    return String(value)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

function formatMs(value) {
    if (value === null || value === undefined || Number.isNaN(value)) return '—';
    if (value < 1) return `${value.toFixed(1)} ms`;
    if (value < 1000) return `${value.toFixed(0)} ms`;
    return `${(value / 1000).toFixed(2)} s`;
}

function formatHeapDelta(value) {
    if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)} MB`;
}

function estimateTokens(text) {
    const trimmed = String(text || '').trim();
    if (!trimmed) return 0;
    return trimmed.split(/\s+/).length;
}

function readHeapMb() {
    const heapBytes = performance?.memory?.usedJSHeapSize;
    if (typeof heapBytes !== 'number') return null;
    return heapBytes / (1024 * 1024);
}

function toneClass(tone) {
    switch (tone) {
        case 'ok':
            return 'text-emerald-300';
        case 'warn':
            return 'text-amber-300';
        case 'error':
            return 'text-rose-300';
        default:
            return 'text-slate-300';
    }
}

function log(message, tone = 'info') {
    const stamp = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    state.logLines.push({ stamp, tone, message });
    if (state.logLines.length > 240) state.logLines.shift();
    renderLog();
}

function renderLog() {
    if (!dom.logOutput) return;
    dom.logOutput.innerHTML = state.logLines
        .map((entry) => `<span class="${toneClass(entry.tone)}">[${entry.stamp}] ${escapeHtml(entry.message)}</span>`)
        .join('\n');
    dom.logOutput.scrollTop = dom.logOutput.scrollHeight;
}

function getSharedConfig() {
    return {
        prompt: dom.promptInput?.value?.trim() || DEFAULT_PROMPT,
        systemPrompt: dom.systemPromptInput?.value?.trim() || '',
        maxTokens: Number(dom.maxTokensInput?.value || 48),
        temperature: Number(dom.temperatureInput?.value || 0.2),
    };
}

function setBadge(el, label, status) {
    if (!el) return;
    const cls = status === 'ready'
        ? 'status-pill status-ready'
        : status === 'error'
            ? 'status-pill status-error'
            : 'status-pill status-wait';
    el.className = cls;
    el.innerHTML = `<span class="w-2 h-2 rounded-full ${status === 'ready' ? 'bg-emerald-300' : status === 'error' ? 'bg-rose-300' : 'bg-amber-300'}"></span><span>${escapeHtml(label)}</span>`;
}

function renderEnvironment() {
    const env = state.env;
    if (!env) return;
    $('env-device').textContent = formatDeviceSummary(env) || 'Browser environment';
    $('env-topology').textContent = formatTopologySummary(env) || 'wasm main thread';
    $('env-coi').textContent = window.crossOriginIsolated ? 'enabled' : 'pending reload';
    $('env-gpu').textContent = state.gpuName || 'Unavailable';
    const heapLabel = performance?.memory ? 'heap api' : 'heap n/a';
    const simd = env?.device_manifest?.has_simd_wasm ? 'simd on' : 'simd off';
    $('env-heap').textContent = `${heapLabel} · ${simd}`;
}

function engineStatusClass(engine) {
    if (engine.errorText) return 'status-error';
    if (engine.kind === 'ready') return 'status-ready';
    if (engine.kind === 'shell') return 'status-shell';
    return 'status-wait';
}

function renderEngines() {
    if (!dom.engineGrid) return;
    dom.engineGrid.innerHTML = state.engines.map((engine) => {
        const controls = (engine.controls || []).map((control) => renderControl(engine, control)).join('');
        const buttonDisabled = engine.live ? '' : 'disabled';
        const buttonLabel = engine.running ? 'Running…' : engine.live ? 'Run benchmark' : 'Reserved';
        const statusText = engine.errorText || engine.statusText || 'Not run yet';
        return `
            <article class="engine-card glass-strong" id="engine-${engine.id}">
                <div class="flex flex-col gap-4">
                    <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                        <div class="max-w-xl">
                            <div class="flex items-center gap-3 flex-wrap">
                                <h3 class="text-2xl font-semibold tracking-tight">${escapeHtml(engine.label)}</h3>
                                <span class="${engineStatusClass(engine)} status-pill">${escapeHtml(engine.kind === 'ready' ? 'live adapter' : engine.kind === 'shell' ? 'adapter shell' : 'waiting')}</span>
                            </div>
                            <div class="text-sm text-cyan-200/80 mt-1">${escapeHtml(engine.family)}</div>
                            <p class="small-note mt-3">${escapeHtml(engine.description)}</p>
                        </div>
                        <div class="flex flex-wrap gap-3">
                            <button class="bench-btn bg-emerald-500/20 text-emerald-300 border border-emerald-400/25" data-run-engine="${engine.id}" ${buttonDisabled}>
                                <i class="fa-solid fa-play mr-2"></i>${escapeHtml(buttonLabel)}
                            </button>
                        </div>
                    </div>
                    ${controls ? `<div class="grid grid-cols-1 md:grid-cols-2 gap-4">${controls}</div>` : ''}
                    <div class="rounded-[22px] border border-white/8 bg-slate-950/45 p-4">
                        <div class="section-label mb-2">Status</div>
                        <div class="text-sm ${engine.errorText ? 'text-rose-300' : 'text-slate-300'}">${escapeHtml(statusText)}</div>
                    </div>
                </div>
            </article>
        `;
    }).join('');

    dom.engineGrid.querySelectorAll('[data-config-engine]').forEach((input) => {
        input.addEventListener('input', onEngineConfigInput);
        input.addEventListener('change', onEngineConfigInput);
    });

    dom.engineGrid.querySelectorAll('[data-run-engine]').forEach((button) => {
        button.addEventListener('click', () => runEngine(button.dataset.runEngine));
    });
}

function renderControl(engine, control) {
    const value = engine.config?.[control.key] ?? control.value ?? '';
    if (control.type === 'select') {
        const options = (control.options || [])
            .map((option) => `<option value="${escapeHtml(option.value)}"${option.value === value ? ' selected' : ''}>${escapeHtml(option.label)}</option>`)
            .join('');
        return `
            <label class="block">
                <span class="section-label mb-2 block">${escapeHtml(control.label)}</span>
                <select class="input-field mono" data-config-engine="${engine.id}" data-config-key="${control.key}">
                    ${options}
                </select>
            </label>
        `;
    }
    return `
        <label class="block">
            <span class="section-label mb-2 block">${escapeHtml(control.label)}</span>
            <input
                class="input-field mono"
                data-config-engine="${engine.id}"
                data-config-key="${control.key}"
                type="${escapeHtml(control.type || 'text')}"
                value="${escapeHtml(value)}"
                placeholder="${escapeHtml(control.placeholder || '')}">
        </label>
    `;
}

function renderResults() {
    if (!dom.resultsBody) return;
    const readyResults = state.engines.filter((engine) => engine.result);
    if (!readyResults.length) {
        dom.resultsBody.innerHTML = `
            <tr>
                <td colspan="7" class="py-8 text-center text-slate-500 text-sm">No completed engine runs yet.</td>
            </tr>
        `;
        return;
    }

    const maxTps = Math.max(...readyResults.map((engine) => engine.result?.tokensPerSecond || 0), 1);
    dom.resultsBody.innerHTML = readyResults.map((engine) => {
        const result = engine.result;
        const width = Math.max(6, Math.round(((result.tokensPerSecond || 0) / maxTps) * 100));
        return `
            <tr class="engine-row align-top">
                <td class="py-4 pr-4">
                    <div class="font-semibold">${escapeHtml(engine.label)}</div>
                    <div class="text-xs text-slate-500 mt-1">${escapeHtml(result.modelLabel || '')}</div>
                </td>
                <td class="py-4 pr-4 text-right mono">${formatMs(result.loadMs)}</td>
                <td class="py-4 pr-4 text-right mono">${formatMs(result.ttftMs)}</td>
                <td class="py-4 pr-4 text-right mono">${formatMs(result.generationMs)}</td>
                <td class="py-4 pr-4 text-right">
                    <div class="mono">${result.tokensPerSecond ? result.tokensPerSecond.toFixed(2) : '0.00'}</div>
                    <div class="result-bar-track mt-2"><div class="result-bar-fill" style="width:${width}%"></div></div>
                </td>
                <td class="py-4 pr-4 text-right mono">${formatHeapDelta(result.heapDeltaMb)}</td>
                <td class="py-4 pr-4 text-sm text-slate-300/85">${escapeHtml(result.summary)}</td>
            </tr>
        `;
    }).join('');
}

function onEngineConfigInput(event) {
    const engine = state.engines.find((item) => item.id === event.target.dataset.configEngine);
    if (!engine) return;
    engine.config[event.target.dataset.configKey] = event.target.value;
}

function getAdapter(engine) {
    if (!state.adapters.has(engine.id)) {
        if (engine.id === 'webllm') {
            state.adapters.set(engine.id, new WebLlmAdapter(engine));
        } else if (engine.id === 'transformersjs') {
            state.adapters.set(engine.id, new TransformersJsAdapter(engine));
        } else if (engine.id === 'qualia') {
            state.adapters.set(engine.id, new QualiaAdapter(engine));
        } else {
            state.adapters.set(engine.id, new PlaceholderAdapter(engine));
        }
    }
    return state.adapters.get(engine.id);
}

class PlaceholderAdapter {
    constructor(engine) {
        this.engine = engine;
    }

    async run() {
        throw new Error(this.engine.waitingOn || 'This engine is intentionally reserved.');
    }
}

class WebLlmAdapter {
    constructor(engine) {
        this.engine = engine;
        this.module = null;
        this.instance = null;
        this.loadedModel = null;
    }

    async loadModule() {
        if (!this.module) {
            this.module = await import(WEBLLM_CDN);
        }
        return this.module;
    }

    pickModel(modelList, requested) {
        if (!Array.isArray(modelList) || !modelList.length) return null;
        if (requested && requested !== 'auto-smallest-prebuilt') {
            const exact = modelList.find((item) => item.model_id === requested || item.model === requested);
            return exact?.model_id || exact?.model || requested;
        }

        const score = (record) => {
            const label = `${record?.model_id || ''} ${record?.model || ''}`.toLowerCase();
            let value = 1000;
            if (/0\.5b/.test(label)) value -= 240;
            if (/1b/.test(label)) value -= 160;
            if (/1\.5b/.test(label)) value -= 90;
            if (/2b/.test(label)) value -= 50;
            if (/q4/.test(label)) value -= 20;
            if (/instruct|chat/.test(label)) value -= 12;
            if (/3b|7b|8b|13b|70b/.test(label)) value += 200;
            return value;
        };

        const best = [...modelList].sort((a, b) => score(a) - score(b))[0];
        return best?.model_id || best?.model || null;
    }

    async ensureEngine(sharedConfig, hooks) {
        const mod = await this.loadModule();
        const modelList = mod.prebuiltAppConfig?.model_list || [];
        const requested = this.engine.config.model;
        const modelId = this.pickModel(modelList, requested);
        if (!modelId) {
            throw new Error('WebLLM did not expose a usable prebuilt model list.');
        }

        let loadMs = 0;
        if (!this.instance || this.loadedModel !== modelId) {
            const t0 = performance.now();
            hooks.progress(`loading ${modelId}`);
            this.instance = await mod.CreateMLCEngine(modelId, {
                initProgressCallback: (progress) => {
                    const text = progress?.text || progress?.status || JSON.stringify(progress);
                    hooks.progress(text);
                },
            });
            loadMs = performance.now() - t0;
            this.loadedModel = modelId;
        }

        return { engine: this.instance, modelId, loadMs };
    }

    async run(sharedConfig, hooks) {
        const heapStart = readHeapMb();
        const prepared = await this.ensureEngine(sharedConfig, hooks);
        const messages = [];
        if (sharedConfig.systemPrompt) {
            messages.push({ role: 'system', content: sharedConfig.systemPrompt });
        }
        messages.push({ role: 'user', content: sharedConfig.prompt });

        const start = performance.now();
        const stream = await prepared.engine.chat.completions.create({
            messages,
            max_tokens: sharedConfig.maxTokens,
            temperature: sharedConfig.temperature,
            stream: true,
        });

        let firstChunkAt = null;
        let outputText = '';
        for await (const chunk of stream) {
            const delta = chunk?.choices?.[0]?.delta?.content ?? '';
            if (!delta) continue;
            if (firstChunkAt === null) {
                firstChunkAt = performance.now();
            }
            outputText += delta;
            hooks.output(outputText);
        }
        const end = performance.now();
        const heapEnd = readHeapMb();
        const approxTokens = estimateTokens(outputText);
        const generationMs = end - start;
        const ttftMs = firstChunkAt === null ? generationMs : firstChunkAt - start;
        return {
            loadMs: prepared.loadMs,
            ttftMs,
            generationMs,
            outputText,
            approxTokens,
            tokensPerSecond: generationMs > 0 ? (approxTokens * 1000) / generationMs : 0,
            heapDeltaMb: heapStart !== null && heapEnd !== null ? heapEnd - heapStart : null,
            summary: `Generated ${approxTokens} estimated tokens via OpenAI-style streaming chat completions.`,
            modelLabel: prepared.modelId,
        };
    }
}

class TransformersJsAdapter {
    constructor(engine) {
        this.engine = engine;
        this.module = null;
        this.instance = null;
        this.loadedKey = null;
    }

    async loadModule() {
        if (!this.module) {
            this.module = await import(TRANSFORMERS_CDN);
        }
        return this.module;
    }

    async ensurePipeline(sharedConfig, hooks) {
        const mod = await this.loadModule();
        const model = this.engine.config.model || 'Xenova/distilgpt2';
        const requestedDevice = this.engine.config.device || 'webgpu';
        const device = requestedDevice === 'webgpu' && navigator.gpu ? 'webgpu' : 'wasm';
        const dtype = this.engine.config.dtype || (device === 'webgpu' ? 'fp32' : 'q4');
        const key = `${model}|${device}|${dtype}`;
        let loadMs = 0;

        if (!this.instance || this.loadedKey !== key) {
            hooks.progress(`loading ${model} on ${device}`);
            const t0 = performance.now();
            this.instance = await mod.pipeline('text-generation', model, {
                device: device === 'webgpu' ? 'webgpu' : undefined,
                dtype,
                progress_callback: (progress) => {
                    const tag = progress?.file || progress?.status || 'progress';
                    const pct = typeof progress?.progress === 'number' ? ` ${Math.round(progress.progress)}%` : '';
                    hooks.progress(`${tag}${pct}`);
                },
            });
            loadMs = performance.now() - t0;
            this.loadedKey = key;
        }

        return { pipeline: this.instance, model, device, dtype, loadMs };
    }

    async run(sharedConfig, hooks) {
        const heapStart = readHeapMb();
        const prepared = await this.ensurePipeline(sharedConfig, hooks);
        const start = performance.now();
        const output = await prepared.pipeline(sharedConfig.prompt, {
            max_new_tokens: sharedConfig.maxTokens,
            do_sample: sharedConfig.temperature > 0,
            temperature: Math.max(sharedConfig.temperature, 0.01),
            return_full_text: false,
        });
        const end = performance.now();
        const generated = Array.isArray(output) ? output[0]?.generated_text ?? '' : String(output ?? '');
        hooks.output(generated);
        const heapEnd = readHeapMb();
        const approxTokens = estimateTokens(generated);
        const generationMs = end - start;
        return {
            loadMs: prepared.loadMs,
            ttftMs: generationMs,
            generationMs,
            outputText: generated,
            approxTokens,
            tokensPerSecond: generationMs > 0 ? (approxTokens * 1000) / generationMs : 0,
            heapDeltaMb: heapStart !== null && heapEnd !== null ? heapEnd - heapStart : null,
            summary: `Non-streaming generation via pipeline('text-generation') on ${prepared.device} with ${prepared.dtype}.`,
            modelLabel: prepared.model,
        };
    }
}

class QualiaAdapter {
    constructor(engine) {
        this.engine = engine;
        this.mod = null;
        this.cache = null;
        this.loadedKey = null;
    }

    async loadModules() {
        if (!this.mod) {
            const mod = await import('../playground/qualia_core_db.js');
            await mod.default(); // initialise the wasm module
            mod.init_panic_hook?.();
            this.mod = mod;
            this.cache = await import('./opfs-model-cache.js');
        }
        return this.mod;
    }

    async ensureLoaded(hooks) {
        const mod = await this.loadModules();
        const url = this.engine.config.model || 'models/SmolLM2-360M-Instruct-Q4_K_M.gguf';
        const format = this.engine.config.format || 'q42';
        const key = `${url}|${format}`;
        let loadMs = 0;
        let source = 'resident';
        if (this.loadedKey !== key || !mod.isWebgpuEngineReady()) {
            if (mod.isWebgpuEngineReady()) {
                await mod.releaseWebgpuEngine();
            }
            const name = url.split('/').pop();
            const t0 = performance.now();
            let bytes;
            if (format === 'q42') {
                const r = await this.cache.loadOrCompileQ42(url, name, {
                    compile: mod.compileGgufToQ42,
                    formatVersion: mod.q42FormatVersion(),
                    onProgress: (loaded, total, phase) =>
                        hooks.progress(phase === 'download' && total ? `download ${Math.round((100 * loaded) / total)}%` : phase),
                });
                bytes = r.bytes;
                source = r.source;
            } else {
                const r = await this.cache.loadGgufCached(url, name, undefined, (loaded, total, phase) =>
                    hooks.progress(phase === 'download' && total ? `download ${Math.round((100 * loaded) / total)}%` : phase));
                bytes = r.bytes;
                source = r.source;
            }
            hooks.progress('initializing WebGPU engine');
            await mod.initialize_webgpu_engine(bytes);
            if (!mod.isWebgpuEngineReady()) {
                throw new Error('Qualia engine failed to initialise after load');
            }
            loadMs = performance.now() - t0;
            this.loadedKey = key;
        }
        return { mod, loadMs, source, url, format };
    }

    async run(sharedConfig, hooks) {
        const heapStart = readHeapMb();
        const prepared = await this.ensureLoaded(hooks);
        // SmolLM2 family is ChatML; build a comparable system+user turn.
        const sys = sharedConfig.systemPrompt || 'You are a concise assistant.';
        const prompt = `<|im_start|>system\n${sys}<|im_end|>\n<|im_start|>user\n${sharedConfig.prompt}<|im_end|>\n<|im_start|>assistant\n`;

        let firstAt = null;
        let outputText = '';
        const start = performance.now();
        await prepared.mod.inferWasmAsync(prompt, (piece) => {
            if (firstAt === null) firstAt = performance.now();
            outputText += piece;
            hooks.output(outputText);
        });
        const end = performance.now();
        const heapEnd = readHeapMb();
        const approxTokens = estimateTokens(outputText);
        const generationMs = end - start;
        const ttftMs = firstAt === null ? generationMs : firstAt - start;
        const fmt = prepared.format === 'q42' ? `.q42 AOT (${prepared.source})` : `GGUF (${prepared.source})`;
        return {
            loadMs: prepared.loadMs,
            ttftMs,
            generationMs,
            outputText,
            approxTokens,
            tokensPerSecond: generationMs > 0 ? (approxTokens * 1000) / generationMs : 0,
            heapDeltaMb: heapStart !== null && heapEnd !== null ? heapEnd - heapStart : null,
            summary: `Native GGUF→WebGPU via ${fmt}; greedy decode (temperature/max-tokens not applied).`,
            modelLabel: `${prepared.url.split('/').pop()} · ${prepared.format}`,
        };
    }
}

async function probeGpuAdapter() {
    if (!navigator.gpu) {
        state.gpuName = 'Unavailable';
        setBadge(dom.gpuBadge, 'WebGPU unavailable', 'error');
        return;
    }
    try {
        const adapter = await navigator.gpu.requestAdapter();
        if (!adapter) {
            state.gpuName = 'No adapter';
            setBadge(dom.gpuBadge, 'WebGPU no adapter', 'error');
            return;
        }
        state.gpuName = adapter.info?.description || adapter.info?.vendor || adapter.name || 'Adapter ready';
        setBadge(dom.gpuBadge, 'WebGPU ready', 'ready');
    } catch (error) {
        state.gpuName = 'Probe failed';
        setBadge(dom.gpuBadge, `WebGPU error`, 'error');
        log(`WebGPU probe failed: ${error.message}`, 'error');
    }
}

async function probeEnvironment() {
    try {
        await ensureCrossOriginIsolation({ quiet: true });
    } catch (error) {
        log(`COI bootstrap warning: ${error.message}`, 'warn');
    }
    setBadge(dom.coiBadge, window.crossOriginIsolated ? 'COI enabled' : 'COI pending reload', window.crossOriginIsolated ? 'ready' : 'wait');
    await probeGpuAdapter();
    state.env = await collectBrowserExecutionEnvironment({
        runner: 'browser-wasm-llm-harness',
        measurementPath: 'browser_llm_compare',
    });
    renderEnvironment();
    if (!window.crossOriginIsolated) {
        log('Cross-origin isolation is still pending. Reload once if you want SharedArrayBuffer-grade timings.', 'warn');
    }
    log(`Environment ready: ${formatDeviceSummary(state.env) || 'browser environment'}`, 'ok');
}

async function runEngine(engineId) {
    const engine = state.engines.find((item) => item.id === engineId);
    if (!engine || engine.running) return;
    const adapter = getAdapter(engine);
    const sharedConfig = getSharedConfig();
    engine.running = true;
    engine.errorText = '';
    engine.statusText = 'Preparing…';
    renderEngines();

    log(`Running ${engine.label}`, 'info');
    try {
        const result = await adapter.run(sharedConfig, {
            progress: (text) => {
                engine.statusText = text;
                renderEngines();
            },
            output: (text) => {
                const preview = text.length > 140 ? `${text.slice(0, 140)}…` : text;
                engine.statusText = `Streaming preview: ${preview || '…'}`;
                renderEngines();
            },
        });
        engine.result = result;
        engine.statusText = result.summary;
        log(`${engine.label} done · load ${formatMs(result.loadMs)} · ttft ${formatMs(result.ttftMs)} · gen ${formatMs(result.generationMs)}`, 'ok');
        if (result.outputText) {
            log(`${engine.label} output: ${result.outputText.slice(0, 220)}`, 'info');
        }
    } catch (error) {
        engine.errorText = error.message || String(error);
        engine.statusText = 'Run failed';
        log(`${engine.label} failed: ${engine.errorText}`, 'error');
    } finally {
        engine.running = false;
        renderEngines();
        renderResults();
    }
}

async function runAll() {
    const runnable = state.engines.filter((engine) => engine.live);
    for (const engine of runnable) {
        // eslint-disable-next-line no-await-in-loop
        await runEngine(engine.id);
    }
}

function clearResults() {
    for (const engine of state.engines) {
        engine.result = null;
        engine.errorText = '';
        engine.statusText = engine.live ? 'Not run yet' : engine.waitingOn;
        engine.running = false;
    }
    renderEngines();
    renderResults();
    log('Cleared benchmark results.', 'warn');
}

function buildReport() {
    return {
        generated_at: new Date().toISOString(),
        page: 'docs/benchmarks.html',
        environment: {
            device_summary: state.env ? formatDeviceSummary(state.env) : null,
            topology_summary: state.env ? formatTopologySummary(state.env) : null,
            cross_origin_isolated: window.crossOriginIsolated,
            webgpu_adapter: state.gpuName,
            heap_api_available: Boolean(performance?.memory),
            wasm_simd: Boolean(state.env?.device_manifest?.has_simd_wasm),
        },
        settings: getSharedConfig(),
        engines: state.engines.map((engine) => ({
            id: engine.id,
            label: engine.label,
            family: engine.family,
            config: engine.config,
            live: engine.live,
            kind: engine.kind,
            status: engine.errorText || engine.statusText,
            result: engine.result,
        })),
    };
}

async function copyText(text, successMessage) {
    try {
        await navigator.clipboard.writeText(text);
        log(successMessage, 'ok');
    } catch (error) {
        log(`Clipboard copy failed: ${error.message}`, 'error');
    }
}

function cacheDom() {
    dom.promptInput = $('prompt-input');
    dom.systemPromptInput = $('system-prompt-input');
    dom.maxTokensInput = $('max-tokens-input');
    dom.temperatureInput = $('temperature-input');
    dom.runAllBtn = $('run-all-btn');
    dom.copyReportBtn = $('copy-report-btn');
    dom.clearResultsBtn = $('clear-results-btn');
    dom.copyLogBtn = $('copy-log-btn');
    dom.resultsBody = $('results-body');
    dom.engineGrid = $('engine-grid');
    dom.logOutput = $('log-output');
    dom.coiBadge = $('coi-badge');
    dom.gpuBadge = $('gpu-badge');
}

function bindEvents() {
    dom.runAllBtn?.addEventListener('click', runAll);
    dom.clearResultsBtn?.addEventListener('click', clearResults);
    dom.copyReportBtn?.addEventListener('click', () => copyText(JSON.stringify(buildReport(), null, 2), 'Copied benchmark report JSON.'));
    dom.copyLogBtn?.addEventListener('click', () => copyText(state.logLines.map((line) => `[${line.stamp}] ${line.message}`).join('\n'), 'Copied harness log.'));
}

async function boot() {
    cacheDom();
    bindEvents();
    renderEngines();
    renderResults();
    log('Browser LLM benchmark harness booting.');
    await probeEnvironment();
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
} else {
    boot();
}
