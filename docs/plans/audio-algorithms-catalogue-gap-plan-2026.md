# Audio Algorithms — Catalogue, Gap Analysis & Qualia Plan (2026)

**Status:** living plan — catalogue from principal + Gemini taxonomy, gap-mapped to Qualia  
**Date:** 2026-07-17  
**Branch:** `0.0.25+` · **Tree:** `C:\Projects\qualia-27062026` only  
**Architecture parent:** [`native-auditory-language-and-music-intelligence.md`](./native-auditory-language-and-music-intelligence.md)  
**Delivery swarm:** [`native-auditory-swarm-delivery.md`](./native-auditory-swarm-delivery.md)  
**ADRs:** [`audio-adrs/`](./audio-adrs/)  
**Progress log:** [`native-auditory-swarm-PROGRESS-LOG.md`](./native-auditory-swarm-PROGRESS-LOG.md) (append when executing)  

**Priority note (principal, 2026-07-17):** **UI product cut first.** This plan freezes the audio *algorithm objectives and gaps* so execution can resume without rediscovering the taxonomy. **Do not block UI waves on audio implementation.**

---

## 0. Purpose

The conversation asked: *in terms of audio algorithms, what am I missing?* — answered with an exhaustive Essentia-class MIR/DSP catalogue plus 2026 generative / full-duplex / AQA layers.

This document:

1. **Indexes** that taxonomy into Qualia-native domains.  
2. **Maps** each domain to current `qualia-core-db::audio` + `qualia-audio` state.  
3. **Declares** what is Present / Partial / Missing / Deferred / Non-goal.  
4. **Phases** implementation so agents do not re-invent Candle/Python/DAW stacks.  
5. **Separates** classical DSP (must own) from learned models (P64 + Forge + governance).

It does **not** replace the architecture plan; it is the **algorithm inventory + gap backlog**.

---

## 1. Qualia non-negotiables (audio algorithms)

| Rule | Consequence for algorithms |
|------|----------------------------|
| No Python in library hot path | Essentia C++/Python ports → pure Rust or audited FFI isolated behind cold boundary |
| No Candle/Burn as runtime owner | Neural pitch/ASR/vocoder = P64 weights + Forge/wgpu (or CPU reference) |
| Zero heap hot path | Streaming STFT, filters, pitch, production block: caller buffers |
| Frames authoritative; PCM not in NQuins | Features → quins are **proposals**; digests/media store hold PCM |
| Shared wgpu | Mel, Conv1D, vocoder decode share `shared_gpu` |
| Epistemic honesty | Pitch/class/transcript = proposals; human correct/reject already sketched |
| Oral languages first-class | ASR late; capture/annotation/language bundle early |
| Rights / consent | Capture, voice clone, biosense-adjacent audio fail-closed |

---

## 2. Current Qualia baseline (honest, 2026-07)

### 2.1 `qualia-core-db::audio`

| Module | Role | Status |
|--------|------|--------|
| `acoustic_plane` / Sonic Token | U3 sonification / control ABI | Partial (browser strong) |
| `stft` / `stft_bake` | Cold STFT + Q4AU bake | Present cold; not full streaming product |
| `cqt_bake` | Cold CQT | Present cold |
| `dsp_kernel` | Parametric sonify | Present first version |
| `hrtf` | Binaural | Present analytic + KemarLite |
| `tf_surface` / edit | Time–frequency geometry seam | Partial |
| Q4AU sidecar | 64-bin magnitude previews | Partial (no mel/MFCC planes yet) |

### 2.2 `qualia-audio`

| Area | Status |
|------|--------|
| WAV decode/encode, resample linear, mono convert | Present |
| Streaming STFT chunk, log-mel, energy, ZCR, CQT mono | Partial (CPU reference) |
| Music: onsets (energy flux), F0 (coarse YIN-like), tempo, chroma12, structure | Partial / reference quality |
| Production: delay, EQ/comp sample, tracks, session history | Partial bounded |
| Speech: phone decode scaffolding, weights hooks | Scaffold / early |
| AED / weighted events | Early |
| Generation: reference tone, two-stem separate reference | Reference only |
| Cross-modal media clock / correlations | Partial |
| Capture session types | Partial |

**Verdict:** foundation + MIR *stubs* exist; **not** Essentia-parity; **not** full-duplex SLM; AQA metrics largely **missing**.

---

## 3. Catalogue domains (from conversation) → Qualia map

Status legend: **P** Present · **∂** Partial · **M** Missing · **D** Deferred · **N** Non-goal (or external tool only)

### 3.1 Envelope / SFX descriptors

| Algorithm (Essentia-class) | Status | Qualia home / notes |
|----------------------------|--------|---------------------|
| Envelope (asymmetric LP) | M | `features/envelope.rs` — implement streaming envelope |
| LogAttackTime | M | Depends on envelope |
| MaxToTotal, MinToTotal, TCToTotal | M | Envelope ratios |
| AfterMaxToBeforeMaxEnergyRatio | M | Pitch-energy variant needs pitch track |
| DerivativeSFX, FlatnessSFX, StrongDecay | M | Envelope derivatives / shape |
| SpectralCentroidTime (time-domain centroid) | M | Can share envelope/energy path |

**Phase:** **A-DSP-1** (classical, zero-heap streaming)

### 3.2 Filters

| Algorithm | Status | Notes |
|-----------|--------|-------|
| LowPass / HighPass / BandPass / BandReject / AllPass (IIR) | M/∂ | Production may have simple EQ; need documented IIR bank |
| DCRemoval | M | Trivial 1st-order HP — do early |
| EqualLoudness | M | ReplayGain-adjacent; EqloudLoader parity |
| IIR generic | M | Coefficient API + caller buffer |
| MaxFilter (HGW) | M | Onset/peak utility |
| MedianFilter, MovingAverage | M | Smoothing |

**Phase:** **A-DSP-1** core IIR + DC; **A-DSP-2** equal-loudness + median/max

### 3.3 Pitch

| Algorithm | Status | Notes |
|-----------|--------|-------|
| PitchYin / PitchYinFFT | ∂ | Coarse lag search exists; needs YinFFT + confidence |
| PitchYinProbabilistic / Probabilities / HMM | M | Higher quality mono track |
| PitchMelodia / PredominantPitchMelodia | M | Music poly → mono melody |
| MultiPitchKlapuri / MultiPitchMelodia | M | Polyphonic |
| PitchSalienceFunction (+ Peaks) | M | Contour foundation |
| PitchContours / Melody variants | M | Contour tracking graph |
| PitchFilter, Vibrato | M | Post-process |
| PitchCREPE | D | Learned; P64 model path when weights exist |
| Audio2Pitch / Pitch2Midi / Audio2Midi | ∂/M | F0 exists; MIDI note ON/OFF buffer missing |
| PitchContourSegmentation | M | Note events from contours |

**Phase:** **A-PITCH-1** Yin/YinFFT + confidence + MIDI note events  
**A-PITCH-2** salience + contours + Melodia-class  
**A-PITCH-3** multi-pitch + CREPE (learned, gated)

### 3.4 Input / output

| Algorithm | Status | Notes |
|-----------|--------|-------|
| MonoLoader / EasyLoader / AudioLoader | ∂ | WAV path; multi-format codec audit open |
| EqloudLoader | M | Equal-loudness + replayGain |
| AudioWriter / MonoWriter | ∂ | WAV encode exists |
| MetadataReader | M | Tags + properties → quins/provenance |
| AudioOnsetsMarker | M | Diagnostic mix |
| Yaml Pool I/O | N | Essentia-specific; use Q42 / CBOR / JSON feature pools instead |

**Phase:** **A-IO-1** metadata + eqloud path; expand codecs only after licence audit

### 3.5 Standard DSP / framing

| Algorithm | Status | Notes |
|-----------|--------|-------|
| FrameCutter / FrameGenerator / OverlapAdd | ∂ | Streaming STFT implies framing; expose explicit API |
| Windowing | ∂ | Hann in STFT; general window table |
| FFT / IFFT / FFTC | ∂ | Forge FFT + CPU floor |
| DCT / IDCT | M | MFCC path |
| Resample | ∂ | Linear exists; quality resampler audit |
| AutoCorrelation / CrossCorrelation / WarpedAC | ∂/M | Pitch uses lag; formalise |
| PeakDetection | ∂ | Needed for spectral peaks |
| ZeroCrossingRate | P | `frame_zcr` |
| ConstantQ / NSGConstantQ (+ inverse) | ∂/M | `forward_cqt_mono` / cqt_bake; NSG deferred |
| Welch PSD | M | |
| Clipper / Scale / NoiseAdder / Trimmer / Slicer | M/∂ | Production/utilities |
| Stereo mux/demux/trim | M | Stereo path weak |
| Tensor pool bridges | N/∂ | Use Qualia tensors / NQuin refs, not Essentia Pool |

**Phase:** **A-DSP-0** (already partly done) complete framing + OLA + peak + DCT

### 3.6 Spectral features

| Algorithm | Status | Notes |
|-----------|--------|-------|
| PowerSpectrum, Flux, HFC, RollOff | M/∂ | Flux-like energy in onsets; formalise |
| MelBands, MFCC | ∂ | log-mel exists; full MFCC + DCT missing |
| BarkBands, BFCC, ERBBands, GFCC | M | Psychoacoustic banks |
| EnergyBand / EnergyBandRatio | M | |
| FlatnessDB, SpectralComplexity, SpectralContrast | M | |
| SpectralPeaks, MaxMagFreq | M | Pitch salience input |
| LPC | M | Speech formants |
| LogSpectrum, FrequencyBands | M | |
| Panning (L/R) | M | Stereo |
| BFCC etc. | D after Mel/MFCC | |

**Phase:** **A-SPEC-1** peaks + flux + rolloff + MFCC  
**A-SPEC-2** Bark/ERB/GFCC + contrast/complexity

### 3.7 Rhythm / tempo / onset (implied by music stack)

| Item | Status |
|------|--------|
| Onset detection | ∂ energy flux |
| Tempo | ∂ from onsets |
| Beat tracking / tempogram / BPM histogram | M |
| Rhythm descriptors (danceability-class) | D |

**Phase:** **A-RHY-1** spectral flux onset + tempogram + beat track

### 3.8 Tonal / music structure

| Item | Status |
|------|--------|
| Chroma 12 | ∂ |
| Key / scale / chord estimation | M |
| HPCP / tuning | M |
| Structure segments | ∂ proposal |
| Cover song / similarity | D |

**Phase:** **A-TONAL-1** HPCP + key + basic chords (assumptions declared, 12-TET optional)

### 3.9 Loudness / dynamics / dynamics SFX

| Item | Status |
|------|--------|
| RMS, loudness (EBU R128) | M |
| Dynamic complexity, LRA | M |
| ReplayGain | M |
| Crest factor / stats moments | M easy |

**Phase:** **A-LOUD-1** RMS + R128-class + ReplayGain metadata

### 3.10 Spatial / rendering

| Item | Status |
|------|--------|
| HRTF / binaural | P/∂ |
| Ambisonics / multi-channel render | M/D |
| Room impulse / reverb production | ∂ delay only |
| Distance / ITD/ILD model beyond HRTF | ∂ |

**Phase:** **A-SPATIAL-1** production reverb + complete HRTF datasets optional

### 3.11 Source separation

| Item | Status |
|------|--------|
| Two-stem reference separate | ∂ reference |
| Demucs-class / open-unmix style | D learned P64 |
| Speech enhancement / denoise | M/D |

**Phase:** **A-SEP-1** classical + one small open model path (gated)

### 3.12 Speech & language

| Item | Status |
|------|--------|
| VAD / segment | ∂ class speech-like |
| Phone decode scaffold | ∂ |
| Full ASR | D (architecture: not Whisper-as-architecture) |
| Diarization | M |
| Language resource bundles | ∂ types |
| Pronunciation / forced align | M |

**Phase:** per architecture § language ladder — governance → capture → AED → VAD → ASR late

### 3.13 Generative synthesis (2026 taxonomy from Gemini)

| Family | Status | Qualia stance |
|--------|--------|---------------|
| Parametric oscillators / MIDI synth | ∂ dsp_kernel | Keep as sonify/production |
| Neural vocoders (HiFi-GAN, BigVGAN) | M | P64 + Forge when weights exist |
| RVQ / FSQ audio tokens | M | Token ABI for full-duplex later |
| Latent diffusion text-to-audio | M/D | Cold offline generation; provenance required |
| Full-duplex native speech models (Moshi/TADA-class) | M/D | **Major programme**; joint semantic+acoustic tokens; sub-160ms target is aspirational on-device |
| Cascaded STT→LLM→TTS | N as architecture | May exist as **optional** bridge only (like Ollama for text) |

**Phase:** **A-GEN-0** honest reference synthesis + consent  
**A-GEN-1** discrete audio codec tokens + vocoder  
**A-GEN-2** full-duplex research path (principal-gated; not UI-wave)

### 3.14 Perceptual evaluation / AQA (2026)

| Family | Status | Notes |
|--------|--------|-------|
| Intrusive: ViSQOL, PESQ/POLQA | M | Port or pure-Rust subset; licence care |
| Non-intrusive MOS: DNSMOS, NISQA | M/D | Learned |
| Multi-axis MOS (production quality / enjoyment / complexity) | M/D | |
| Auditory LLM diagnostics | D | After native audio models |

**Phase:** **A-AQA-1** simple intrusive metrics + PESQ-class if licence OK  
**A-AQA-2** DNN MOS optional

### 3.15 Telecom / codecs (mentioned in full taxonomy)

| Item | Status |
|------|--------|
| Opus/AAC encode-decode | Cold adapter audit |
| Packet loss concealment research | N product default |
| Codec quality via AQA | Via A-AQA |

---

## 4. What you were “missing” (summary answer)

Relative to a full Essentia + 2026 generative stack, the **largest gaps** are:

1. **Streaming classical MIR completeness** — envelope SFX, full filter bank, YinFFT-quality pitch, spectral peaks→salience→contours, MFCC/Bark/ERB, R128 loudness, beat tracking.  
2. **Polyphonic / melody extraction** — Melodia, multi-pitch, contour systems.  
3. **Production-grade I/O** — metadata, equal-loudness load, stereo, better resampler.  
4. **Learned audio stack under Qualia ABI** — CREPE, neural vocoders, RVQ tokens, AED/ASR as P64 graphs (not Python).  
5. **Full-duplex spoken language models** — joint semantic+acoustic streams (state of art 2026); major separate programme.  
6. **AQA / MOS** — human-perceptual metrics beyond mean/variance.  
7. **Source separation & enhancement** beyond reference two-stem.  
8. **UI surfaces** for Listen / production (deferred to after current UI waves; `listen_workbench` exists as entry).

What you **already have** that many MIR stacks lack: graph/quin epistemic proposals, rights gates, shared 10D/σ perception with vision, Forge GPU path, no Python core, oral-language design hooks, production block skeleton, HRTF, Q4AU spectral sidecar.

---

## 5. Implementation waves (audio — execute after UI priority)

| Wave | Name | Deliverables | Depends |
|------|------|--------------|---------|
| **AU0** | Inventory freeze | This doc; feature registry table in `qualia-audio` (honesty Present/∂/M) | — |
| **AU1** | Streaming DSP core | Envelope + LogAttackTime + ratios; DC + LP/HP/BP; framing/OLA public; PeakDetection | AU0 |
| **AU2** | Pitch v1 | YinFFT + confidence; PitchFilter; Pitch2Midi ON/OFF; vibrato optional | AU1 |
| **AU3** | Spectral v1 | SpectralPeaks, Flux, HFC, RollOff, MelBands, MFCC | AU1 |
| **AU4** | Loudness + loaders | RMS, R128-class, Eqloud path, MetadataReader → provenance | AU1 |
| **AU5** | Music structure | Onset spectral flux, tempogram/beat, HPCP/key (assumptions flag) | AU2–3 |
| **AU6** | Contours / Melodia-class | Salience + contours + predominant F0 | AU2–3 |
| **AU7** | Production FX | Real IIR EQ banks, reverb, stereo; session history already | AU1 |
| **AU8** | Learned path | AED weights real; CREPE optional; speech encoder P64 | Architecture speech ladder |
| **AU9** | AQA | One intrusive + one no-ref path (or honest stub labels) | AU3–4 |
| **AU10** | Generative / full-duplex | Token codec + vocoder; full-duplex only with principal gate | AU8 + inference excellence |

**UI listen surface:** after Webizen UI U1–U4, wire `listen_workbench` to AU1–AU5 features with honesty chips.

---

## 6. Feature registry sketch (for code)

```text
// Future: crates/qualia-audio/src/capability_registry.rs
// Each row: id, domain, status, zero_heap_hot, streaming, test_name
// Status: Present | Partial | Missing | FeatureDisabled | NeedsWeights
```

Seed rows from §3 tables. MCP tool later: `audio_features` (mirror `computer_vision`).

---

## 7. Sub-agent lanes (when audio executes)

| Lane | Exclusive paths | Notes |
|------|-----------------|-------|
| **AU-DSP** | `qualia-audio/src/features/**` | Envelope, filters, spectral classical |
| **AU-PITCH** | `features/pitch*.rs`, `music.rs` pitch APIs | No production rewrite |
| **AU-CORE** | `qualia-core-db/src/audio/**` | STFT/CQT/Forge seam only if needed |
| **AU-SPEECH** | `speech.rs`, weights | After AU-DSP |
| **AU-PROD** | `production.rs`, `session_history.rs` | Real-time block rules |
| **AU-UI** | `listen_workbench.rs`, desktop commands | After UI U1–U4 |
| **AU-DOCS** | manuals + this plan updates | |

CLAIM in `coordination/NOTICES.md`. No second tensor runtime.

---

## 8. Explicit non-goals

- Vendoring full Essentia tree or requiring Python Essentia at runtime.  
- Claiming MOS/full-duplex “done” without models + measured latency.  
- PCM blobs inside NQuins.  
- Replacing Qualia inference with cascaded cloud STT/TTS as architecture.  
- Blocking the **UI programme** on AU waves (principal priority 2026-07-17).

---

## 9. Acceptance (programme-level for audio)

Reviewer says **yes** when:

1. AU1–AU5 classical features have unit tests + streaming caller-buffer APIs.  
2. Capability registry honesty matches UI/MCP labels.  
3. Pitch/melody paths declare assumptions (mono vs poly, 12-TET).  
4. Any learned path loads via P64/Forge and fails closed without weights.  
5. Listen UI shows real features or Partial — never fake MOS.  
6. Cross-modal still shares `media_time_ms` / digests with vision.

---

## 10. Source of the catalogue

- Principal-provided Essentia-class algorithm list (Envelope/SFX, Filters, Pitch, I/O, Standard, Spectral, … truncated mid-SpectralPeaks in chat paste; remaining Essentia domains treated as **in-scope inventory** until explicitly rejected: Rhythm, Tonal, Loudness, Duration, SFX, Stats, Extractor composites).  
- Gemini extension: telecom, spatial, separation, generative full-duplex, neural vocoders, RVQ/FSQ, latent diffusion, AQA (ViSQOL, PESQ, DNSMOS/NISQA, multi-axis MOS, auditory LLMs).

If a specific Essentia algorithm is needed and not named in §3, add a row under the matching domain and status **M** — do not invent a parallel catalogue file.

---

## 11. Change log

| Date | Note |
|------|------|
| 2026-07-17 | Initial catalogue + gap map from Gemini/Essentia conversation; phased AU0–AU10; UI-first deferral |

---

*Classical completeness first; learned and full-duplex second; always under Qualia ABI, rights, and measurement honesty.*
