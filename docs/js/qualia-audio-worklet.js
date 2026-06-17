/**
 * U3 AcousticPlane — phenomenal binaural + inverse-STFT + SAB zero-copy (P-F1/P-F2/7.4).
 *
 * Stereo path: parametric carrier (σ parity) + overlap-add spectral grains + ITD delay line.
 */

const PREVIEW_BINS = 64;
const UNIFORM_FLOATS = 18 + PREVIEW_BINS;
const SAB_MAGIC = 0x51334153;
const SAB_FLOAT_MIRROR_OFFSET = 512;
const GRAIN_SAMPLES = 256;
const GRAIN_OVERLAP = 0.5;
const DELAY_LEN = 4096;

function fractSigma(sigma) {
    return sigma - Math.floor(sigma);
}

function sigmaToWavelengthNm(sigma) {
    return 400 + fractSigma(sigma) * 300;
}

function sigmaToCenterFrequencyHz(sigma) {
    const lambda = sigmaToWavelengthNm(sigma);
    const t = Math.min(1, Math.max(0, (lambda - 400) / 300));
    return Math.min(8000, Math.max(55, 1760 * (1 - t) + 110 * t));
}

function hann(i, n) {
    return 0.5 * (1 - Math.cos((2 * Math.PI * i) / Math.max(1, n - 1)));
}

function unpackUniform(floats) {
    if (!floats || floats.length < UNIFORM_FLOATS) return null;
    const bins = new Float32Array(PREVIEW_BINS);
    for (let i = 0; i < PREVIEW_BINS; i++) bins[i] = floats[18 + i];
    return {
        alpha: floats[0],
        mu: floats[1],
        position: [floats[2], floats[3], floats[4]],
        trackV: floats[5],
        manifoldW: floats[6],
        epistemicQ: floats[7],
        fmIndex: floats[8],
        frequencyHz: floats[9],
        enabled: floats[10] > 0,
        gainL: floats[11],
        gainR: floats[12],
        itdSeconds: floats[13],
        azimuth: floats[14],
        elevation: floats[15],
        roomDamp: floats[16],
        stftFrame: floats[17],
        previewBins: bins,
    };
}

function readSabUniform(sab, lastSeq) {
    if (!sab || sab.byteLength < SAB_FLOAT_MIRROR_OFFSET + UNIFORM_FLOATS * 4) return null;
    const view = new DataView(sab);
    const magic = view.getUint32(0, true);
    if (magic !== SAB_MAGIC) return null;
    const seq = view.getUint16(6, true);
    if (seq === lastSeq) return { seq, uniform: null };
    const floats = new Float32Array(sab, SAB_FLOAT_MIRROR_OFFSET, UNIFORM_FLOATS);
    return { seq, uniform: unpackUniform(floats) };
}

function inverseStftGrain(bins, phase, sampleRate, roomDamp, frameIdx = 0) {
    const n = bins.length;
    const out = new Float32Array(GRAIN_SAMPLES);
    const sigmaBias = frameIdx * 0.013;
    for (let i = 0; i < out.length; i++) {
        let sum = 0;
        const win = hann(i, out.length);
        for (let k = 0; k < n; k++) {
            const sigma = (k + 1) / n + sigmaBias;
            const freq = sigmaToCenterFrequencyHz(sigma) * (0.35 + bins[k] * 0.65);
            const amp = bins[k] * roomDamp * win;
            sum += amp * Math.sin(2 * Math.PI * freq * (i / sampleRate) + phase * (k + 1) * 0.09);
        }
        out[i] = sum / Math.sqrt(n);
    }
    return out;
}

function softLimit(x) {
    const a = Math.abs(x);
    if (a <= 0.85) return x;
    return Math.sign(x) * (0.85 + (a - 0.85) / (1 + (a - 0.85) * 4));
}

class QualiaAcousticProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        this.phase = 0;
        this.stftPhase = 0;
        this.grainPos = 0;
        this.grainA = null;
        this.grainB = null;
        this.crossfade = 1;
        this.frameIdx = 0;
        this.sab = options?.processorOptions?.sab ?? null;
        this.sabSeq = 0;
        this.delayL = new Float32Array(DELAY_LEN);
        this.delayR = new Float32Array(DELAY_LEN);
        this.delayWrite = 0;
        this.uniform = {
            alpha: 0.25,
            mu: 0,
            frequencyHz: 440,
            fmIndex: 0,
            enabled: true,
            gainL: 0.707,
            gainR: 0.707,
            itdSeconds: 0,
            roomDamp: 1,
            epistemicQ: 0.5,
            previewBins: new Float32Array(PREVIEW_BINS),
        };
        this.tokenQueue = [];

        this.port.onmessage = (ev) => {
            const msg = ev.data;
            if (!msg) return;
            if (msg.type === 'sab' && msg.buffer) this.sab = msg.buffer;
            if (msg.type === 'uniform' && msg.floats) {
                const u = unpackUniform(msg.floats);
                if (u) this.applyUniform(u);
            }
            if (msg.type === 'sidecar' && msg.bins) {
                this.uniform.previewBins = new Float32Array(msg.bins);
                this.scheduleGrain(this.uniform.previewBins, this.uniform.roomDamp);
            }
            if (msg.type === 'tokens' && Array.isArray(msg.raw)) {
                for (const raw of msg.raw) this.tokenQueue.push(raw);
            }
            if (msg.type === 'mute') this.uniform.enabled = !msg.mute;
        };
    }

    scheduleGrain(bins, roomDamp) {
        const next = inverseStftGrain(
            bins,
            this.stftPhase + this.uniform.stftFrame * 0.1,
            sampleRate,
            roomDamp,
            this.frameIdx,
        );
        this.frameIdx += 1;
        if (!this.grainA) {
            this.grainA = next;
            this.grainPos = 0;
            this.crossfade = 1;
            return;
        }
        this.grainB = next;
        this.crossfade = 0;
    }

    applyUniform(u) {
        this.uniform = u;
        this.scheduleGrain(u.previewBins, u.roomDamp);
    }

    sampleGrain() {
        if (!this.grainA) return 0;
        const pos = this.grainPos;
        const a = this.grainA[pos] ?? 0;
        if (!this.grainB || this.crossfade >= 1) return a;
        const b = this.grainB[pos] ?? 0;
        const t = this.crossfade;
        return a * (1 - t) + b * t;
    }

    advanceGrain() {
        if (!this.grainA) return;
        this.grainPos += 1;
        if (this.grainB) {
            this.crossfade = Math.min(1, this.crossfade + GRAIN_OVERLAP / GRAIN_SAMPLES);
        }
        if (this.grainPos >= GRAIN_SAMPLES) {
            this.stftPhase += 0.07;
            if (this.grainB) {
                this.grainA = this.grainB;
                this.grainB = null;
                this.crossfade = 1;
            }
            this.grainPos = 0;
            this.scheduleGrain(this.uniform.previewBins, this.uniform.roomDamp);
        }
    }

    process(inputs, outputs) {
        const output = outputs[0];
        if (!output || output.length < 2) return true;

        if (this.sab) {
            const sabRead = readSabUniform(this.sab, this.sabSeq);
            if (sabRead?.uniform) {
                this.sabSeq = sabRead.seq;
                this.applyUniform(sabRead.uniform);
            }
        }

        const chL = output[0];
        const chR = output[1];
        if (!this.uniform.enabled) {
            chL.fill(0);
            chR.fill(0);
            return true;
        }

        const sr = sampleRate;
        const u = this.uniform;
        const freq = u.frequencyHz || 440;
        const gain = Math.min(1, Math.max(0, u.alpha)) * 0.32;
        const fm = u.fmIndex * 0.012 + u.epistemicQ * 0.004;
        const stftMix = Math.min(0.55, gain + 0.08) * u.roomDamp;
        const itdSamples = Math.min(256, Math.max(0, Math.round(Math.abs(u.itdSeconds) * sr)));
        const panL = u.gainL;
        const panR = u.gainR;
        const itdSign = u.itdSeconds >= 0 ? 1 : -1;

        for (let i = 0; i < chL.length; i++) {
            const modPhase = this.phase * (1 + fm * Math.sin(this.phase * 6.283));
            let mono = Math.sin(modPhase * 2 * Math.PI) * gain;
            mono += this.sampleGrain() * stftMix;

            if (this.tokenQueue.length > 0 && i === 0) {
                const raw = this.tokenQueue.shift();
                const vel = ((raw >> 24) & 0xff) / 127;
                const pitch = ((raw >> 8) & 0xff) / 127;
                mono += vel * 0.06 * Math.sin(this.phase * 2 * Math.PI * (1 + pitch));
            }

            const w = this.delayWrite % DELAY_LEN;
            this.delayL[w] = mono * panL;
            this.delayR[w] = mono * panR;

            const readL = (w - itdSamples * itdSign + DELAY_LEN) % DELAY_LEN;
            const readR = (w + itdSamples * itdSign) % DELAY_LEN;
            chL[i] = softLimit(this.delayL[readL]);
            chR[i] = softLimit(this.delayR[readR]);

            this.phase += freq / sr;
            if (this.phase > 1) this.phase -= Math.floor(this.phase);
            this.delayWrite += 1;
            this.advanceGrain();
        }

        return true;
    }
}

registerProcessor('qualia-acoustic', QualiaAcousticProcessor);

export {
    PREVIEW_BINS,
    UNIFORM_FLOATS,
    SAB_FLOAT_MIRROR_OFFSET,
    unpackUniform,
    sigmaToCenterFrequencyHz,
    inverseStftGrain,
};