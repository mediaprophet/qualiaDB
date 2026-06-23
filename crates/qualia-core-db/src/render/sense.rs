//! Phase 4 — the **sense path** (the input twin of the renderer; STELLAR §D).
//!
//! The renderer projects the manifold → percept (output). The sense path is its twin: a physical
//! signal → the manifold → a **discrete Fact** the values/epistemic layer can reason over. It is
//! the same `∫Ψ > τ → Fact` bridge ([`crate::modalities::manifold_logic`]) the legal-logic layer
//! already uses, with a microphone front-end.
//!
//! ## What this is (honest scope)
//! * **Acoustic (microphone) is the live band** — real forward DSP: a Hann-windowed DFT of PCM
//!   samples → magnitude bins → dominant tonal bin. (There is no forward STFT elsewhere in the
//!   crate; the `audio::stft_bake` path *synthesises* spectra for the sidecar raster, it does not
//!   *analyse* a captured signal. So this is the analysis primitive.)
//! * **RF / Wi-Fi CSI is DEFERRED** ([`band_available`]) — it needs an SDR / radio plus explicit
//!   hardware permission, and may never be available in-browser. Documented, not stubbed-as-done.
//!
//! ## The rails (RENDERER_DEFINITION §8; the memories) — load-bearing here
//! * **Every sense runs under the deontic/standpoint gate.** [`sense_permitted`] **fails closed**:
//!   no Active `PERMIT` consent for *this agent + this environment* ⇒ **refused**. That is
//!   *surveillance-refusal by construction* — the default is to **not** capture. An Active `FORBID`
//!   always wins.
//! * **Own-environment + consent.** The consent norm binds `(agent) PERMIT capture(environment)`;
//!   sensing is authorised per environment the agent holds consent for.
//! * **Biometrics never leave the device.** The pipeline emits **only a discrete symbolic Fact**
//!   (an acoustic-tone-detected quin + the dominant frequency / energy scalars). The raw samples
//!   are never stored, returned, or embedded — there is no voiceprint, no audio, in the output.
//! * **Delegates, does not reinvent the logic.** The continuous→discrete decision is
//!   `manifold_logic::continuous_to_fact`; the consent decision is `logic::deontic`. Only the
//!   forward DSP and the field-packing live here.

use crate::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID, OP_PERMIT,
};
use crate::modalities::manifold_logic::{continuous_to_fact, integrate_abs};
use crate::{q_hash, NQuin};

/// Number of DFT magnitude bins computed for a captured frame (analysis resolution).
pub const SENSE_BINS: usize = 64;

/// Max consent norms evaluated in one [`sense_permitted`] pass (stack-bounded, zero-heap).
pub const MAX_SENSE_NORMS: usize = 32;

/// Predicate stamp for a percept→fact quin.
pub const P_PERCEIVED: u64 = q_hash("urn:qualia:sense:perceived");
/// Property-path for a capture/sense action governed by a consent norm.
pub const P_SENSE_CAPTURE: u64 = q_hash("urn:qualia:sense:capture");
/// The discrete fact emitted when acoustic energy crosses the bridge threshold.
pub const FACT_ACOUSTIC_TONE: u64 = q_hash("urn:qualia:sense:acousticToneDetected");

// ── forward DSP (the microphone analysis — real, not synthesis) ──────────────────────────────────

/// Hann window coefficient for sample `n` of a `len`-sample frame (reduces spectral leakage).
#[inline]
pub fn hann(n: usize, len: usize) -> f64 {
    if len <= 1 {
        return 1.0;
    }
    let x = (2.0 * core::f64::consts::PI * n as f64) / (len as f64 - 1.0);
    0.5 - 0.5 * x.cos()
}

/// Forward windowed DFT magnitudes: for each bin `k` in `0..out.len()`, write `|X_k|` of the
/// Hann-windowed real `samples`. Zero-heap — results go in the caller's `out` slice.
pub fn dft_magnitudes(samples: &[f64], out: &mut [f64]) {
    let n = samples.len().max(1);
    for (k, mag) in out.iter_mut().enumerate() {
        let w = -2.0 * core::f64::consts::PI * k as f64 / n as f64;
        let mut re = 0.0;
        let mut im = 0.0;
        for (i, &s) in samples.iter().enumerate() {
            let ws = s * hann(i, samples.len());
            let ang = w * i as f64;
            re += ws * ang.cos();
            im += ws * ang.sin();
        }
        *mag = (re * re + im * im).sqrt();
    }
}

/// Centre frequency (Hz) of DFT bin `bin` for an `fft_size`-point transform at `sample_rate`.
#[inline]
pub fn bin_to_hz(bin: usize, sample_rate: f64, fft_size: usize) -> f64 {
    bin as f64 * sample_rate / fft_size.max(1) as f64
}

/// The dominant **tonal** bin (skipping DC bin 0): `(bin, magnitude)`, or `None` if there is no
/// non-DC bin.
pub fn dominant_bin(mags: &[f64]) -> Option<(usize, f64)> {
    let mut best: Option<(usize, f64)> = None;
    for (i, &m) in mags.iter().enumerate().skip(1) {
        match best {
            Some((_, bm)) if bm >= m => {}
            _ => best = Some((i, m)),
        }
    }
    best
}

// ── consent gate (deontic; fail-closed; surveillance-refusal) ────────────────────────────────────

/// Build a consent norm for sensing: `(agent) OPCODE capture(environment)` in a `frame`, optionally
/// expiring. Consent is `OP_PERMIT`; a prohibition is `OP_FORBID`.
pub fn sense_norm(agent: u64, opcode: u8, environment: u64, frame: u64, expiry_unix32: u32) -> NQuin {
    compile_norm_quin(agent, opcode, P_SENSE_CAPTURE, environment, frame, expiry_unix32, false)
}

/// **The sense gate.** `true` iff `agent` has Active consent to capture `environment`: there is an
/// Active `PERMIT` bound to `(agent, environment)` **and** no Active `FORBID` bound to it.
///
/// **Fails closed:** no consent norm (or an over-capacity / unevaluable set) ⇒ `false` (refuse).
/// This is the surveillance-refusal default — nothing is sensed without explicit consent.
pub fn sense_permitted(agent: u64, environment: u64, norms: &[NQuin], now_unix: u32) -> bool {
    if norms.len() > MAX_SENSE_NORMS {
        return false; // fail closed
    }
    let mut out = [DeonticVerdict::default(); MAX_SENSE_NORMS];
    let n = match evaluate_deontic_contract(norms, now_unix, &mut out) {
        Ok(n) => n,
        Err(_) => return false, // fail closed
    };
    let mut consented = false;
    for v in &out[..n] {
        if v.status != DeonticStatus::Active || v.norm.subject != agent || v.norm.object != environment
        {
            continue;
        }
        match v.opcode {
            OP_FORBID => return false, // an active prohibition always wins → refuse
            OP_PERMIT => consented = true,
            _ => {}
        }
    }
    consented
}

// ── percept → fact ───────────────────────────────────────────────────────────────────────────────

/// Pack the percept scalars (dominant Hz, integrated energy) into one `metadata` word — the only
/// quantitative residue that leaves the sense path. `hz` in the high 32 bits, `energy` in the low.
#[inline]
pub fn pack_percept(hz: f32, energy: f32) -> u64 {
    ((hz.to_bits() as u64) << 32) | energy.to_bits() as u64
}

/// Inverse of [`pack_percept`].
#[inline]
pub fn unpack_percept(metadata: u64) -> (f32, f32) {
    (
        f32::from_bits((metadata >> 32) as u32),
        f32::from_bits((metadata & 0xFFFF_FFFF) as u32),
    )
}

/// Build the discrete percept→fact NQuin: *where* sensed (`subject = environment`), the percept
/// stamp ([`P_PERCEIVED`]), *what* (`object = fact_id`), the consenting standpoint (`context`), and
/// the dominant-Hz / energy scalars in `metadata`. No raw audio — only this discrete fact.
pub fn perceived_fact_quin(
    environment: u64,
    standpoint: u64,
    fact_id: u64,
    dominant_hz: f32,
    energy: f32,
) -> NQuin {
    let metadata = pack_percept(dominant_hz, energy);
    let mut q = NQuin {
        subject: environment,
        predicate: P_PERCEIVED,
        object: fact_id,
        context: standpoint,
        metadata,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
    q
}

/// The outcome of a gated sense attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SenseOutcome {
    /// The consent gate denied capture (surveillance-refusal). No signal was turned into a fact.
    Refused,
    /// Consented, but the integrated signal did not cross the bridge threshold — no fact.
    BelowThreshold,
    /// A discrete percept→fact NQuin was emitted (carries no raw audio).
    Fact(NQuin),
}

/// **The Phase-4 pipeline.** Microphone PCM `samples` → (consent gate) → forward DSP → the
/// `∫Ψ > τ → Fact` bridge → a discrete Fact NQuin, or a refusal.
///
/// Order is deliberate: the **consent gate runs first** — if `agent` has no Active consent to
/// capture `environment`, the function returns [`SenseOutcome::Refused`] and the signal is never
/// turned into anything. When consented, the integrated time-domain energy is thresholded by the
/// inherited bridge; on a crossing, the dominant tonal frequency (forward DFT) is attached to a
/// discrete fact. Raw `samples` never leave this call.
#[allow(clippy::too_many_arguments)]
pub fn sense_acoustic_to_fact(
    samples: &[f64],
    sample_rate: f64,
    threshold: f64,
    agent: u64,
    environment: u64,
    standpoint: u64,
    norms: &[NQuin],
    now_unix: u32,
) -> SenseOutcome {
    // 1) Consent gate FIRST — surveillance-refusal default.
    if !sense_permitted(agent, environment, norms, now_unix) {
        return SenseOutcome::Refused;
    }
    // 2) The inherited bridge decides IF a percept crosses into a fact (∫Ψ > τ).
    match continuous_to_fact(samples, threshold, FACT_ACOUSTIC_TONE) {
        None => SenseOutcome::BelowThreshold,
        Some(fact_id) => {
            // 3) Forward DSP decides WHAT (the dominant tonal frequency).
            let mut mags = [0.0_f64; SENSE_BINS];
            dft_magnitudes(samples, &mut mags);
            let dominant_hz = dominant_bin(&mags)
                .map(|(bin, _)| bin_to_hz(bin, sample_rate, samples.len()))
                .unwrap_or(0.0);
            let energy = integrate_abs(samples);
            SenseOutcome::Fact(perceived_fact_quin(
                environment,
                standpoint,
                fact_id,
                dominant_hz as f32,
                energy as f32,
            ))
        }
    }
}

// ── sense bands (acoustic live; RF deferred) ─────────────────────────────────────────────────────

/// The physical bands the sense path can ingest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenseBand {
    /// Microphone acoustic — **live** (this module).
    Acoustic,
    /// RF / Wi-Fi CSI — **deferred**: needs an SDR / radio + explicit hardware permission, may
    /// never be available in-browser (STELLAR §D). Honestly not implemented.
    RadioFrequency,
}

/// Whether a band is available for live sensing. Only [`SenseBand::Acoustic`] is; RF is deferred.
#[inline]
pub fn band_available(band: SenseBand) -> bool {
    matches!(band, SenseBand::Acoustic)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent() -> u64 {
        q_hash("did:example:listener")
    }
    fn environment() -> u64 {
        q_hash("urn:qualia:env:my-room")
    }
    fn standpoint() -> u64 {
        q_hash("urn:qualia:frame:home")
    }

    /// A pure sine of `freq` Hz at `sr`, `n` samples, amplitude 1.0.
    fn sine(freq: f64, sr: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| (2.0 * core::f64::consts::PI * freq * i as f64 / sr).sin())
            .collect()
    }

    #[test]
    fn dft_detects_a_pure_tone() {
        // 1000 Hz @ 16 kHz, 256-pt DFT → bin spacing 62.5 Hz → exact bin 16.
        let s = sine(1000.0, 16000.0, 256);
        let mut mags = [0.0; SENSE_BINS];
        dft_magnitudes(&s, &mut mags);
        let (bin, _) = dominant_bin(&mags).unwrap();
        assert_eq!(bin, 16);
        assert!((bin_to_hz(bin, 16000.0, 256) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn percept_pack_round_trip() {
        let (hz, e) = unpack_percept(pack_percept(1000.0, 162.5));
        assert!((hz - 1000.0).abs() < 1e-3 && (e - 162.5).abs() < 1e-3);
    }

    #[test]
    fn rf_band_is_deferred() {
        assert!(band_available(SenseBand::Acoustic));
        assert!(!band_available(SenseBand::RadioFrequency));
    }

    /// RAIL: surveillance-refusal — with no consent norm, a loud signal yields NO fact.
    #[test]
    fn no_consent_refuses_even_a_loud_signal() {
        let s = sine(1000.0, 16000.0, 256);
        let out = sense_acoustic_to_fact(&s, 16000.0, 50.0, agent(), environment(), standpoint(), &[], 100);
        assert_eq!(out, SenseOutcome::Refused);
    }

    /// PHASE-4 ACCEPTANCE: consented mic frame → STFT/DFT → a discrete Fact NQuin via the bridge.
    #[test]
    fn consented_tone_emits_a_discrete_fact() {
        let s = sine(1000.0, 16000.0, 256);
        let permit = sense_norm(agent(), OP_PERMIT, environment(), standpoint(), 0);
        let out =
            sense_acoustic_to_fact(&s, 16000.0, 50.0, agent(), environment(), standpoint(), &[permit], 100);
        match out {
            SenseOutcome::Fact(q) => {
                // The fact is the discrete percept — WHAT (tone), WHERE (env), no raw audio.
                assert_eq!(q.object, FACT_ACOUSTIC_TONE);
                assert_eq!(q.subject, environment());
                assert_eq!(q.predicate, P_PERCEIVED);
                assert_eq!(q.parity, q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata);
                let (hz, energy) = unpack_percept(q.metadata);
                assert!((hz - 1000.0).abs() < 1.0, "dominant ~1000 Hz, got {hz}");
                assert!(energy > 50.0);
            }
            other => panic!("expected a Fact, got {other:?}"),
        }
    }

    /// Consented but silent → the bridge does not cross → no fact.
    #[test]
    fn consented_silence_yields_no_fact() {
        let silence = [0.0_f64; 256];
        let permit = sense_norm(agent(), OP_PERMIT, environment(), standpoint(), 0);
        let out = sense_acoustic_to_fact(
            &silence, 16000.0, 50.0, agent(), environment(), standpoint(), &[permit], 100,
        );
        assert_eq!(out, SenseOutcome::BelowThreshold);
    }

    /// An Active FORBID overrides consent → refused even with a strong signal.
    #[test]
    fn active_forbid_overrides_consent() {
        let s = sine(1000.0, 16000.0, 256);
        let permit = sense_norm(agent(), OP_PERMIT, environment(), standpoint(), 0);
        let forbid = sense_norm(agent(), OP_FORBID, environment(), standpoint(), 0);
        let out = sense_acoustic_to_fact(
            &s, 16000.0, 50.0, agent(), environment(), standpoint(), &[permit, forbid], 100,
        );
        assert_eq!(out, SenseOutcome::Refused);
    }

    /// Consent for a DIFFERENT environment does not authorise this one (own-environment).
    #[test]
    fn consent_is_per_environment() {
        let s = sine(1000.0, 16000.0, 256);
        let other_env = q_hash("urn:qualia:env:someone-elses-room");
        let permit = sense_norm(agent(), OP_PERMIT, other_env, standpoint(), 0);
        let out =
            sense_acoustic_to_fact(&s, 16000.0, 50.0, agent(), environment(), standpoint(), &[permit], 100);
        assert_eq!(out, SenseOutcome::Refused);
    }
}
