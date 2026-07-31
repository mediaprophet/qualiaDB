# Perception & Library Progress Report

**Date:** 2026-07-17  
**Branch:** `0.0.28`  
**Canonical tree:** `C:\Projects\qualia-27062026`  
**Audience:** Timothy (principal) and follow-on agents  

This report records **what is implemented**, **what is honest-gated**, and **what still needs human or licensed input**. It is an engineering record, not marketing.

---

## 1. Status summary

| Area | State |
|------|--------|
| Vision library (`qualia-vision`) | **Done** for scaffold + first-release paths; foundation models **COMPLETE-WITH-GATE** |
| Audio library (`qualia-audio`) | **Done** for Ears MVP + reference DAW/FX/music/speech scaffolds |
| §15 / §18 first-release smokes | **Green** (automated) |
| Hypermedia Library catalogue (models + ontologies) | **Done** (this wave) |
| Listen DAW mixer UI | **Done** (reference strips + bounce; this wave) |
| Build / UAT packaging | **Open** (principal rebuild + dogfood) |
| Real eval corpora (H1 / HA1) | **Open** (principal) |
| Licensed GGUF / P64 / ASR / TTS weights | **Open** (licence + files) |
| A4 mesh FEA / safety certification | **Not a software-only deliverable** |

**P-FINAL (architecture):** phase matrix is **Done or COMPLETE-WITH-GATE**. Ready for **build/UAT on labelled substrate** — not for “foundation model quality” claims.

---

## 2. What “COMPLETE-WITH-GATE” means (reminder)

| Status | Meaning |
|--------|---------|
| **Done** | Path exists, tests exercise it, honesty labels correct, claim allowed at the *declared* bar. |
| **COMPLETE-WITH-GATE** | Implementation finished; **production / certification claim blocked** until a human gate (corpus, licence, partnership, policy). |
| **Incomplete** | Missing code — not acceptable as a silent gap. |

Closing a gate with policy (“synthetic-only forever”) is allowed and counts as Done under that policy.

---

## 3. What has been done

### 3.1 Vision (`qualia-vision` + pipeline + Studio Vision)

- ABI, preprocess, NMS, media store, Forge Pool/Resize/Conv, linear probe  
- Detector + bounded tracker, overlay + BMP, synthetic V7 train/test  
- Semantic quins + SPARQL-MM repair + vision SHACL  
- QVWT production-weight **seed** format (save/load disk)  
- Image generate (reference) + cancel + receipts; heightfield image→3D; MeshIR OBJ + `.10d`  
- Twin eligibility: viz-only by default; **elasticity preview** + **A1 closed-form bar stretch** (not mesh FEA, not A4)  
- Desktop/Studio: detect, reject/correct, G→S continuum, ensure QVWT, twin A1 demo  

**Honesty:** seed QVWT and reference generator are **not** foundation vision models.

### 3.2 Audio (`qualia-audio` + pipeline + Studio Listen)

- WAV, STFT/log-mel, CQT, streaming STFT  
- Reference VAD/events + **WeightedAedModel** seed `.qaed`  
- Capture intent + ring; desktop **cpal** mic  
- Live mic ring → AED; speech greedy phones + **streaming** decoder; unknown language → empty  
- U3 sonify “hear” path  
- Music: onset, F0, tempo, structure segments, chroma abstain without 12-TET  
- Production: mix, bounce, lowpass, **EQ / compressor / delay**, session history, automation lane  
- TTS: consent + **revoke**; reference 2-stem separation  
- Cross-modal: shared media clock, joint window, non-causal correlation  
- **Mixer UI** (Listen): 3 track strips (gain/pan/LP/EQ/comp/delay/mute/solo), Bounce, undo/redo demo, honesty banner  

**Honesty:** not commercial DAW, not licensed ASR/TTS, not demucs-class separation.

### 3.3 Hypermedia Library catalogue (this wave)

| Deliverable | Detail |
|-------------|--------|
| Module | `qualia-client-core::wellfair::perception_catalog` |
| Models catalogued | Vision seed QVWT, AED `.qaed`, speech `.qspk`, GGUF user slot |
| Ontologies catalogued | SHACL, Solid stack (LDP/ACL/terms/OIDC/pim/FOAF), PROV, SKOS, OWL, RDFS, Time, SOSA, Music, Consent, DC Terms |
| Seed weights | `ensure_*` materialises seed files under `{storage}/models/` when missing |
| Section | Library → **Software** with `honesty:seed_reference` flags on seed models |
| Commands | `library_seed_perception_assets`, `wellfair_seed_perception_library` |
| Index seed expand | `DEFAULT_BUNDLED_ONTOLOGIES` now includes prov/skos/owl/rdfs/time/sosa/music/consent |

Listen button: **Seed Library models**.

### 3.4 Measured (recent sessions)

| Suite | Result |
|-------|--------|
| `qualia-audio` | 38 tests passed (prior closeout) |
| `qualia-vision` | 45 tests passed (prior closeout) |
| `perception_catalog` | idempotent seed test |
| desktop / studio | compile checks used for command wiring |

Exact numbers for this wave: re-run after integrate if CI is the authority.

---

## 4. What still needs to be done (reminders)

### 4.1 Principal / human gates (agents cannot invent these)

| ID | Need | Why |
|----|------|-----|
| **H1** | Lawful vision eval set (path + labels + licence note) | Real detector metrics; never mix with synthetic scores |
| **HA1** | Lawful acoustic event corpus | Real AED metrics |
| **HA6 / HA7** | Speech / TTS model + voice licence & consent sources | Full ASR/TTS product claims |
| **Weights** | Licence-compatible GGUF/P64 (or offline convert notes) | Vision backbone, neural AED, etc. |
| **HA4** | Language/community partnership (if oral work is productised) | No silent orthography assumptions |
| **Policy** | Optional: “synthetic-only forever” on any COMPLETE-WITH-GATE phase | Closes gate without corpus |

**Minimum useful H1:** tens–hundreds of labelled images you may use, held-out split, class list.  
**Minimum useful HA1:** labelled WAV/FLAC clips with event classes aligned to the inventory.  
**Do not** put multi-GB weights or private corpora in git — local path + manifest + licence.

### 4.2 Product / engineering still open (agent-capable, prioritised)

| Item | Notes |
|------|--------|
| **Build/UAT** | Desktop rebuild, dogfood Listen + Vision + Library Software shelf |
| **Library UI polish** | Surface model/ontology filters in Library panel; seed on vault unlock if desired |
| **`.hmc` / semantic-library bridge** | Push CML from `qualia-semantic-library` into live graph; real `OntologyCompiler` vs mock shapes |
| **FLAC codec** | HA2 default was WAV; FLAC still optional |
| **Full DAW depth** | Plugin host, sample-accurate automation editor, real clip timeline, live meters from device |
| **Streaming product ASR** | Beyond phone seed matrix |
| **Forge A2000 cert recipes** | Hardware-gated, optional |
| **Mesh FEA (F3)** | Real tetra/hex elasticity + residuals — still **not A4** without human assurance process |
| **Eval harness wiring** | Load H1/HA1 packs when principal drops paths |

### 4.3 A4 mesh FEA (do not “finish” as a pure code task)

- **Fidelity** (how heavy the physics) ≠ **assurance** (how safe the decision is).  
- **A4** requires standards, evidence, independent check, **signed competent-human** review.  
- Software may: refuse unsupported FEA, attach evidence slots, run **A0–A2** screening.  
- Software must **not** emit “structure is certified safe” from a kernel.  
- Current vertical: **A1 bar stretch** after explicit elasticity-preview promote.

### 4.4 Hypermedia / semantic library next (beyond catalogue)

Already present in tree:

- Large `bundled/ontologies/` (W3C, Solid, FIBO, …)  
- Hypermedia Library sections (Secret / Software / Commons / …)  
- Offline `qualia-semantic-library` `.hmc` pipeline for document corpora  

Still valuable:

1. Faceted Library browse for `model://` and `ontology://`  
2. Optional startup: seed perception catalogue after ontology Index seed  
3. Wire GGUF discovery (existing model lifecycle) into the same catalogue  
4. Ingest ontology TTL into Index for every catalogued id that is not only a Library row  
5. Semantic-library → Qualia graph CML push (documented next seam in its README)

---

## 5. How to operate what landed

### Seed Library models + ontologies

- Desktop (storage ready): command `library_seed_perception_assets`  
- Or vault host: `wellfair_seed_perception_library`  
- Listen UI: **Seed Library models**  
- Browse: Library → **Software** for `model://webizen/*` and `ontology://webizen/*`

### Mixer

- Listen → **Mixer (reference)** strips → adjust → **Bounce mix**  
- Engine: `audio_mixer_bounce` / `ProcessPlan` FX chain  

### Perception smokes

- Vision: §15 smoke, disk QVWT detect  
- Audio: §18 smoke, Live AED (mic start first)  

---

## 6. Commits / lineage (indicative)

Recent perception line on `0.0.28` includes (among others):

- Auditory swarms A–X, full completion waves, cont 2–4 closeout (`9019f2e8` and predecessors)  
- This wave: `perception_catalog`, expanded bundled ontology defaults, Listen mixer UI, progress report  

Use `git log --oneline 0.0.28` for exact SHAs after this report’s commit.

---

## 7. Recommended next actions for Timothy

1. **Rebuild** desktop and open Library → Software after **Seed Library models**.  
2. **Dogfood** Listen mixer bounce + Live AED with mic.  
3. When ready for real metrics: drop **H1/HA1** under a local eval path and ask for harness wiring.  
4. When ready for quality: place **licence-compatible** GGUF/P64 under models and extend catalogue slots.  
5. Treat **A4** as process + evidence + people, not a missing button.

---

## 8. One-line bottom line

**Architecture and first-release perception paths are in the product with honest labels; Library now catalogues seed models and core ontologies; Listen has a reference mixer UI. Remaining work is corpora, licences, packaging/dogfood, and deeper product polish — not “missing phase zero.”**

*End of report.*
