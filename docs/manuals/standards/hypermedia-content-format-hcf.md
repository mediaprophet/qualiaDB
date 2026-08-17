# File Format & Linguistic Specifications: QualiaDB Hypermedia Ecosystem
> *"Pure Linked Data from Authoring to Wire: The End of XML, HTML, and JS Bloat"*

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Principal / inventor:** Timothy Charles Holborn &lt;timothy.holborn@gmail.com&gt;

QualiaDB defines a suite of specialized, high-performance binary, graph, and human-readable container formats optimized for decentralized knowledge representation, 3D spatial assets, LLM inference tables, universal linguistic expression, and hypermedia mindware.

---

## 1. The QualiaDB Core File Format Suite

| Format | Extension | Primary Role | Underlying Architecture |
| :--- | :--- | :--- | :--- |
| **Q42** | `.q42` | Primary Knowledge & Phonological Graph | Merkle-CRDT DAG, Nquins (`<<[ s p o g prov ]>>`), Phonemic Nodes, Sonic Tokens |
| **D10** | `.d10` | 10D Projector, Spatial Mesh & Articulatory EMF | 10D Dimensional manifold (`[α, μ, σ, t, ...]`), Vocal Tract Kinematics, 3D Mesh |
| **p64** | `.p64` | Columnar Tables & Neural Acoustic Tensors | 64-aligned continuous embeddings, discrete neural audio tokens (EnCodec), Forge weights |
| **HCF** | `.hcf` | Hypermedia Content Document | RDF-Native (`yaml-ld-q42` source / `CBOR-LD` binary) with Context Markup |
| **HMC** | `.hmc` | HyperMedia Container Archive | Cryptographic archive packaging HCF, Q42, D10, and p64 assets with Bao streaming |

---

## 2. Universal Linguistic Support: The Ontological Grapheme Architecture

Traditional computing architectures rely strictly on Unicode codepoints and standard BCP 47 language tags. As a result, thousands of indigenous, unencoded, endangered, ancient, and gestural/signed mother tongues cannot be natively digitized.

QualiaDB resolves this by treating language not as flat character strings, but as **first-class semantic nodes (Ontological Graphemes)** within the Q42 graph.

### A. The Semantic Grapheme Model
When a document contains text in an unencoded script, each glyph or semantic unit is represented as a structured node:

```yaml
# yaml-ld-q42 representation of an unencoded project grapheme
- "@id": "grapheme:project_glyph_042"
  "@type": "qualia:OntologicalGrapheme"
  "qualia:language":
    "@id": "lang:example_unencoded"
    "qualia:tag": "und-x-project"
    "qualia:family": "UnencodedPlaceholder"
  "qualia:visualGeometry": "asset://mesh/project_glyph_042.d10"  # 3D/2D vector geometry
  "qualia:strokeSequence": "asset://strokes/glyph_042.svg"       # Stroke order data
  "qualia:phoneticGrounding": "grapheme:project_phoneme_042"     # Linked phonological node in Q42
  "qualia:ipa": "ŋal"                                           # IPA approximation
  "qualia:puaMapping": "\\uE042"                                # Unicode Private Use Area fallback
  "qualia:semanticConcept": "concept:FreshwaterSpring"          # Direct ontological grounding
```

### B. Multi-Modal Rendering & Accessibility
1. **Visual Display:** Poet renders the character using direct GPU mesh rendering (`.d10`) or Bezier paths, entirely bypassing system font limitations.
2. **Audio Synthesis:** Non-written or oral-first languages synthesize speech directly through the generative articulatory pipeline without text-to-speech approximations.
3. **Gestural & Signed Languages:** Link directly to 3D skeletal animation kinematics in `.d10` representing spatial signs.

---

## 3. Native Auditory Intelligence: Generative Articulatory Speech Pipeline

Traditional audio files (WAV, MP3, AAC) are flat 2D air-pressure recordings over time ("acoustic exhaust"). They are immutable, difficult to manipulate semantically, and computationally disconnected from 3D visual rendering.

QualiaDB replaces flat playback with a **Generative Articulatory Speech & Auditory Pipeline** operating across the **`q42` $\leftrightarrow$ `p64` $\leftrightarrow$ `d10` Triad**:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                   Generative Articulatory Audio Pipeline                 │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │ 1. Q42 Phonological & Intent Graph                                 │  │
│  │    - Phonemic Nquins (IPA / Mother-Tongue Phone Nodes)             │  │
│  │    - Relational Prosody (Pitch, Stress, Emotion, Cadence)          │  │
│  │    - Sonic Tokens (8-byte compact event control ABI)               │  │
│  └─────────────────────────────────┬──────────────────────────────────┘  │
│                                    │ Semantic Intent & Prosody Map       │
│  ┌─────────────────────────────────▼──────────────────────────────────┐  │
│  │ 2. P64 Neural Acoustic Latents & Tensors                           │  │
│  │    - 64-aligned continuous embeddings / discrete EnCodec tokens    │  │
│  │    - Forge WGSL kernel compute graphs & residency planner          │  │
│  └─────────────────────────────────┬──────────────────────────────────┘  │
│                                    │ Continuous Acoustic Instructions    │
│  ┌─────────────────────────────────▼──────────────────────────────────┐  │
│  │ 3. D10 Articulatory Kinematics & The 10D EMF Projector             │  │
│  │    - U3 AcousticPlane (Tensor10D with shared spectral coordinate σ)│  │
│  │    - Virtual Vocal Tract (Glottal, tongue, lip kinematic manifold) │  │
│  │    - Emits physical acoustic wave & 100% synchronized 3D lip-sync  │  │
│  └────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────┘
```

### A. The Three Pipeline Pillars

1. **`q42` (The Phonological Graph):** Stores the *linguistic and phonetic intent*. Words, morphemes, and dialectal pronunciations are represented as phonemic nquins. Prosodic features (pitch contours, emphasis, emotional modulation) are stored as graph relations rather than baked into audio samples.
2. **`p64` (Neural Acoustic Tokens & Embeddings):** 64-byte aligned columnar tables store dense neural acoustic representations and ML weights. The Forge compute engine executes streaming transformations converting phonological graph sequences into continuous acoustic latents without third-party tensor bloat (no Candle/Burn runtime dependency).
3. **`d10` (Articulatory Kinematics & 10D EMF Projector):** The `.d10` container models the physical geometry and electromagnetic/spatial projection of the **virtual vocal tract**. Grounded in articulatory phonetics (glottalization, velum position, lingual resonance), the host engine projects the acoustic latents through the physical tract parameters to compute the localized sound wave.

### B. Architectural Superpowers of the Triad
- **Speaker Identity Portability:** An agent’s "voice" is a `.d10` vocal tract profile. Swapping the underlying `.q42` phonological graph (e.g. from English to an example unencoded language) passes through the same `.d10` physical tract, preserving the speaker's vocal timbre and identity across all languages.
- **Mathematically Perfect 3D Lip-Sync:** Because the `.d10` articulatory parameters represent the actual physical mouth/lip/jaw states used to generate the acoustics, 3D avatar facial animation and lip-sync are mathematically exact by construction.
- **Unified Spectral Manifold ($\sigma$):** QualiaDB's U3 AcousticPlane shares the spectral coordinate $\sigma$ and timeline $t$ across both visual projection and auditory rendering, enabling unified cross-modal perception ("eyes and ears" intelligence).

---

## 3. Hypermedia Content Format (HCF) (`.hcf`)

HCF eliminates legacy HTML, XML, and JavaScript in favor of **RDF-Native serializations**:
- **Authoring Layer:** Expressed in **`yaml-ld-q42`** — clean, indentation-based semantic documents without closing tags or DOM boilerplate.
- **Machine & Wire Layer:** Serialized as **CBOR-LD 1.0** — binary compressed Linked Data with shared term-dictionaries for zero-copy memory mapping on WASM, Mobile, and IoT.
- **Application language:** **Vibe** (Poet), not JavaScript. Cells in HCF are Vibe (`= …`); modules are `.vibe`. JSON and JS remain a *temporary host* (Tauri IPC, Dioxus-web glue), not the format.

The incremental cut from that host to this pair is in [`docs/plans/native-presentation-and-vibe-beyond-webview-2026-08-16.md`](../../plans/native-presentation-and-vibe-beyond-webview-2026-08-16.md) §0.1.

HTML, CSS, JSON-LD, Turtle, Markdown, and PDF are **optional projectors** (named-profile views) of HCF/CML — doors to the traditional WWW, not the design language of this environment. The HID is not confined to those formats. The **preferred bridge** is a proportionate WASM family member plus CBOR-LD / Q42 (plan §0.3), not HTML+RDFa. COF remains a text projector for agent token windows only. CSS-as-RDF lives in the W3C archive (`http://www.w3.org/ns/css#`); it is not Qualia’s style system. See the same plan §0.2–§0.3. Do not persist an emit as the document.

### A. Document Structure in `yaml-ld-q42`
```yaml
# ===========================================================================
# Clinical Case Study: Comorbidity & Spatiotemporal Telemetry
# Format: Hypermedia Content Format (HCF) v1.0 (yaml-ld-q42)
# ===========================================================================
"@context":
  "@vocab": "https://qualiadb.org/schema/hcf#"
  "snomed": "http://snomed.info/id/"
  "geo": "http://www.opengis.net/ont/geosparql#"
  "vibe": "https://qualiadb.org/schema/vibe#"
  "pulse": "https://qualiadb.org/schema/pulse#"
  "aura": "https://qualiadb.org/schema/aura#"

"@id": "doc:case_study_2026_01"
"@type": "HypermediaDocument"
"title": "Pediatric Comorbidity Analysis"
"ontology": "asset://graph/medical_ontology.q42"

"content":
  - "@type": "Section"
    "heading": "Patient Assessment"
    "blocks":
      - "@type": "Paragraph"
        "text": "Patient presented with acute symptoms of diabetic complication."
        "annotations":
          - "@type": "ContextAnnotation"
            "span": [24, 45]
            "target": "snomed:44054006"
            "aura:schema": "snomed:DiabetesMellitus"
            "prov:confidence": 0.98

      - "@type": "ReactiveCell"
        "@id": "cell:risk_score"
        "formula": "=COUNT(graph.match(?s, :hasCondition, snomed:44054006))"

  - "@type": "MultiModalContainer"
    "assets":
      - "@id": "asset:organ_model"
        "@type": "SpatialMesh"
        "src": "asset://mesh/pancreas_anatomy.d10"
      - "@id": "asset:embeddings"
        "@type": "TensorTable"
        "src": "asset://embeddings/clinical_vectors.p64"

# Homoiconic VibeScript AST embedded directly in the document graph
"scripts":
  - "@id": "script:telemetry_monitor"
    "@type": "vibe:PulseHandler"
    "vibe:onEvent": "pulse:on_pulse"
    "vibe:body": |
      on_pulse(stream) {
          if stream.has_alert() {
              pulse.broadcast("topic:alerts", "Telemetry anomaly detected");
              let clinic = graph.get_node("qualia:Clinic_A");
              aura.apply_schema(clinic, "snomed:EmergencyStatus");
          }
      }
```

---

## 4. HyperMedia Container (HMC) (`.hmc`)

The `.hmc` container packages all multi-modal assets into an immutable, cryptographically verifiable archive:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                     HyperMedia Container Archive (.hmc)                  │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │ 1. Container Manifest & Bao Outboard Hash Tree (BLAKE3 1-KiB)      │  │
│  │    - Header CID, DID Signatures, Permissive Commons Rights Scope   │  │
│  └─────────────────────────────────┬──────────────────────────────────┘  │
│                                    │                                     │
│  ┌─────────────────────────────────┼──────────────────────────────────┐  │
│  │                                 │                                  │  │
│  ▼                                 ▼                                  ▼  │
│┌──────────────────────┐ ┌──────────────────────┐ ┌──────────────────────┐│
││ HCF Document (.hcf)  │ │ Q42 Graph (.q42)     │ │ Spatial & Data       ││
││ (CBOR-LD binary AST) │ │ (Merkle-CRDT DAG)    │ │ (.d10 mesh, .p64)    ││
│└──────────────────────┘ └──────────────────────┘ └──────────────────────┘│
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Streaming & Distributed Synchronization Protocols

### A. Bao Verified Streaming (BLAKE3 Chunk Trees)
To support seamless, real-time access to large multi-modal assets (`.p64` embedding tensors and `.d10` 3D meshes) packaged within `.hmc` archives:
- **Zero Monolithic Downloads:** The HMC container stores an interleaved Bao outboard tree containing BLAKE3 parent hashes over 1-KiB leaf chunks.
- **Random-Access Verification:** Webizen, mobile, and IoT clients can seek to arbitrary byte offsets (e.g. streaming slice `[102400..204800]`), verifying each chunk against the container's root hash in $O(\log N)$ time before executing or rendering.

### B. Merkle-CRDT State Compaction & Tombstone Pruning
For `.q42` knowledge graphs subject to frequent edits and retractions across offline agents:
- **Tombstone Pruning:** Operational deletion records are periodically consolidated via state-based delta snapshots.
- **Epoch Hashing:** `qualia_core_db` computes a new convergent Merkle root for the active state, pruning historical operational tombstones while maintaining cryptographic provenance of the active graph.
