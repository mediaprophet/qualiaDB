# Ingest Pipeline — Technical Companion to `RENDERER_DEFINITION.md` §12

> **Status:** design (task #12). The companion to [`RENDERER_DEFINITION.md`](RENDERER_DEFINITION.md) §12 —
> the definition stays architectural; this file carries the math, the cryptographic structures, and the
> execution boundaries for processing GGUF / MLX / safetensor assets into the q42 substrate.
>
> **Authorship & status of the numbers.** The design and its governing constraints are Timothy Holborn's;
> the formal write-up was worked out in dialogue using AI tools (Gemini, Claude) — *tools he uses; no tool
> authorship is claimed*. The numeric values in §5 **encode value choices and are pending Timothy's
> attestation** — they are conservative defaults to make the mechanism *runnable*, not settled policy.

---

## 0. Governing constraint — the affordability & honest-scope test (read first)

Everything below is subordinate to the objective: **improve the capabilities available to ordinary people
using technology on the devices they own.** Therefore every choice in this pipeline must pass:

1. **No food-vs-compute trade.** Nothing here may force a person to choose between living costs and the
   ability to use the tool.
2. **No $150k server on the user's side.** Capable hardware may be *used where it exists*, but the **user's
   device must never be required to be that hardware.**
3. **Honest scope.** This does **not** replace datacenter-scale compute. The value proposition is
   **sovereignty + governance + provenance + locality on hardware they control** — not raw capability.

**The architectural consequence (this governs §2–§6):**

```
HEAVY, ONCE, ELSEWHERE                          CHEAP, CONTINUOUS, ON-DEVICE
(dequant, permutation-align, DELLA,             (zero-heap JIT fold of tiny signed
 KL-fusion/training, consolidation)             deltas over a pre-compressed base)
        │                                                    ▲
        ▼                                                    │
  capable node (desktop / solar-surplus / guild)   ──distribute──►  phone · Intel-iGPU · no-GPU
        │            (WebTorrent / HF swarm)                         (the user pays only the fold)
        └───────────────── signed birth record + compressed base + signed deltas ───────────────┘
```

The user **never pays the merge cost.** The expensive convergence is computed once on whatever capable
hardware is available and **distributed** as a pre-converged, pre-compressed (ternary / KIVI) artifact. See
§6 for placement.

---

## 1. Cryptographic lineage & the model birth record

### 1.1 The ingestion triple

An incoming asset is structurally defined — *where + what + who* — by:

```
A = ⟨ H_c , D_signer , U_r ⟩
```

- **`H_c`** — immutable content hash (**BLAKE3**, the project's hash) of the raw tensor payload →
  content-addressed integrity (*what*).
- **`D_signer`** — the **DID** of the attesting entity → the cryptographic gate that verifies authorisation
  *before* any bytes reach dequantization (*who*).
- **`U_r`** — retrieval tuple `⟨URI, t_ingest⟩` → *where / when*. **The URI is a locator, not a trust
  anchor** (identifier ≠ identity); the **signature gates the merge.**

### 1.2 The model birth record (PROV-O / DigitalBirthRecord)

When parents are unified, the pipeline writes an immutable, signed derivation record to the history ledger —
a *birth certificate* for the offspring. It maps **genealogical provenance** and makes **no claim of
behavioural attribution** (§12.7).

```json
{
  "@context": ["https://webizen.org/q42", "https://www.w3.org/ns/prov", "https://w3id.org/security/v2"],
  "id": "urn:q42:lineage:birth-record:<blake3>",
  "type": "ModelDerivationRecord",
  "prov:wasDerivedFrom": [
    { "contentHash": "urn:blake3:<parent1>", "signer": "did:q42:agent1" },
    { "contentHash": "urn:blake3:<parent2>", "signer": "did:q42:agent2" }
  ],
  "prov:wasGeneratedBy": {
    "track": "TrackA_WeightSpace",
    "permutationMatrixHash": "urn:blake3:<perm>",
    "dellaThreshold": 1.5,
    "lambdaCoefficients": [0.6, 0.4]
  },
  "offspring": { "contentHash": "urn:blake3:<offspring>", "initialQuorumState": "q>0_Escrow" },
  "prov:wasAttributedTo": "did:q42:governance:gatekeeper",
  "proof": {
    "type": "DataIntegrityProof",
    "cryptosuite": "ed25519-2020 | ml-dsa-65",
    "verificationMethod": "did:q42:governance:gatekeeper#key-1",
    "proofValue": "<sig>"
  }
}
```

> **Crypto refinement (real stack).** Signatures use the project's actual suites — **Ed25519** for ordinary
> attestations, **ML-DSA-65 (FIPS-204)** for post-quantum-sensitive ones (`fiduciary_crypto.rs`) — not an
> Ed25519-only literal. The hash is **BLAKE3**.

---

## 2. The two ingestion tracks (math)

```
                       [Ingested tensor: W_incoming]
                                   │
                ┌──────────────────┴──────────────────┐
                ▼                                     ▼
       [Track A: Homogeneous]                [Track B: Heterogeneous]
       Permutation align → DELLA              (NO weight arithmetic)
       → JIT/AOT commutative fold             AOT distribution fusion (training)
                └──────────────────┬──────────────────┘
                                   ▼
                    [Idempotent set evaluation §3]
                                   ▼
                         [Active substrate]
```

### 2.1 Track A — weight-space (shared lineage)

**Step 1 — optimal-transport permutation alignment** (overcome permutation invariance before any add). For
each layer `l`, find `P^(l)` maximising cross-layer correlation:

```
max_{P^(l)}  Tr( (W_q42^(l))ᵀ · P^(l) · W_incoming^(l) )
```

**Step 2 — magnitude-aware delta extraction (DELLA).** With `W_aligned = P · W_incoming`, isolate the task
vector and sparsify below a dynamic σ-gate:

```
ΔW = DELLA( W_aligned − W_base , σ_t )
```

### 2.2 Track B — semantic-space fusion (different architecture / tokenizer)

Weight arithmetic is **blocked** (mismatched widths / heads / token spaces — no shared `W_base`). Fuse at the
**output-distribution** boundary by minimising KL over a shared calibration set `𝒟`:

```
min_{θ_q42}  E_{x∼𝒟} [ Σ_{m∈M}  w_m · D_KL( P_m(y|x) ‖ P_q42(y|x) ) ]
```

`w_m` = governance-attested weights. **Track B requires training (gradient steps) → strictly AOT, never the
runtime loop.** Its output is (part of) a new consolidated base, not a cheap JIT fold.

| | Track A — weight-space | Track B — semantic-space |
|---|---|---|
| Compatibility | strict lineage parity | heterogeneous topologies / tokenizers |
| Aligns | hidden units | conceptual distributions |
| Mechanism | OT permutation + DELLA | cross-distribution KL min |
| Cost / placement | light; JIT-foldable or AOT | heavy (inference + training); **AOT only** |

---

## 3. Idempotent functional convergence (the substrate join)

To eliminate manifold drift when deltas are multi-cast / re-gossiped across shards, accumulation is over a
**grow-only / OR-Set `𝒮` of unique content hashes** of cryptographically-cleared deltas — *not* raw repeated
addition (which would double-count):

```
W_active = W_base ⊕ Σ_{k ∈ 𝒮} ΔW_k
```

Because the sum is bound to the **unique** identifiers in `𝒮`, a duplicate packet is a no-op:

```
𝒮 ∪ { H_c,k } = 𝒮   ⟹   W_active^(t+1) = W_active^(t)
```

The join is **associative + commutative + idempotent** → evaluable out-of-order across distributed shards
without divergence, lineage auditable from each origin identifier.

---

## 4. Layered defenses against merge-hijacking (honest posture)

Passive binary scanning alone breeds **false confidence**. Defenses are layered, **trust-by-lineage first**:

1. **Primary — lineage verification (the reliable gate).** A tensor is blocked from compilation if `D_signer`
   lacks explicit authorisation in the permissions registry. *This is the load-bearing defense.*
2. **Secondary — structural metric audit (best-effort).** Tensors passing identity are scanned for anomalous
   statistical profiles (clipping, extreme hidden-activation outliers — common Trojan indicators) and
   **quarantined** on failure. *Catches some backdoors, not all (adaptive backdoors evade distribution
   tests) — it raises the bar, it does not guarantee.*
3. **Tertiary — runtime Sentinel monitoring.** During evaluation, structural collapses / out-of-distribution
   entropy spikes in attention maps isolate the step and fall back to a verified **non-derogable baseline**.

---

## 5. Conflict adjudication — proposed policy knobs (PENDING ATTESTATION)

The §12.6 adjudicator is runnable with the conservative defaults below. **These encode value choices and are
Timothy's to attest** — they are a safe starting proposal, not fixed law.

```
[Conflict captured] ─► [stakes ≥ STAKES_THRESHOLD?]
                              │
                ┌─────────────┴─────────────┐
                ▼ yes                       ▼ no
        [force human-review            [challenger α ≥ ALPHA_DOMINANCE_RATIO × base?]
         escrow  q>0]                        │
                                ┌────────────┴────────────┐
                                ▼ yes                     ▼ no
                          [auto-fold]            [freeze → escrow q>0]
```

- **`STAKES_THRESHOLD = 0.40`** *(proposed)* — any operation whose output-distribution shift alters safety
  boundaries or core domain definitions past a normalised 0.40 **disables autonomous merge** → immediate
  `q>0` escrow.
- **`ALPHA_DOMINANCE_RATIO = 2.5`** *(proposed)* — in low-stakes cases a challenger must show evidence-weighted
  confidence (α) ≥ 2.5× the base value to fold autonomously; else frozen.
- **`CONSENSUS_QUORUM_SIZE_N = 5`** *(proposed)* — autonomous `q>0 → q=0` promotion requires the **identical**
  delta independently signed by ≥ 5 distinct trusted identifiers.
- **`NON_DEROGABLE_SET`** *(proposed: the rights instruments + system safety rails)* — tensors attempting to
  modify indices mapped to these are **rejected instantly** and the signer flagged for governance audit.

> **Wisdom-out-of-band:** these four are the *policy*; the engine supplies the *mechanism*. They stay
> changeable by Timothy's attestation — the rule that resolves conflicts is itself attested by a person.

---

## 6. Compute placement & distribution (how §0 is actually met)

| Pass | Cost | Where it runs |
|---|---|---|
| Dequant → FP16/BF16 (never lift Q4) | heavy | capable node, AOT |
| Permutation align (OT/Hungarian) | heavy | capable node, AOT |
| DELLA delta extraction (Track A) | moderate | capable node, AOT (delta then distributable) |
| KL fusion / training (Track B) | heaviest | capable node / guild, AOT only |
| Consolidation / compaction | heavy | capable node, AOT (periodic) |
| **Signed-delta JIT fold** | **cheap, zero-heap, zero-copy** | **the user's device** |
| Inference over compressed base | tier-scaled (§G) | the user's device |

- **Distribute-once.** A merge/fusion done by *anyone* with compute (or a guild, §H) is gossiped as a signed
  artifact (WebTorrent / HF) → other nodes verify the birth record and adopt it with **TTFT ≈ 0**. Convergence
  the user benefits from is convergence they did **not** have to compute.
- **Energy-opportunistic (§H).** Capable nodes do the heavy passes on **surplus** energy (solar / grid peak);
  battery-constrained devices never run them.
- **Fits the cell model.** The on-device cost stays inside the 512 MB fractal-shard budget; heavy work is
  out-of-cell, off-device, AOT.

---

## 7. Pipeline versioning & the streaming transcoder (GGUF / safetensor / MLX → Q42)

**Keep the legacy path; version the new one.** The existing **GGUF → Q42W** path (the opaque weight container)
**stays in place** — it works for some GGUF cases; do not disturb it. The high-fidelity transcoder is added
**alongside** as a **new pipeline version** (version-gated in `detect.rs`; both coexist — additive, not a
replacement). Legacy keeps working; new work targets v2.

**High-fidelity sources only (the future default).** v2 ingests **Q8 or better** (F16 / BF16 / Q8) and adds
**safetensor + MLX** detection (currently absent from `detect.rs`) alongside high-fidelity GGUF. It does **not
lift Q4** — the loss is already baked in (§0, §2.1); lifting it just launders quantization noise into the
manifold. The §A compression (ternary / KIVI / W4A4) is applied **during** transcode, *from* the high-fidelity
source — we down-sample **once, deliberately**, instead of inheriting someone else's lossy Q4.

**Streaming encoder — never load the whole file into RAM.** The transcoder exploits the file's
**tensor-by-tensor** layout (header + named tensors at byte offsets):
1. **mmap the source** (`memmap2` / OS page cache) — zero heap for the bytes; pages fault in on use.
2. **Iterate tensors via the offset map**, processing **one tensor at a time** (and, within a large tensor,
   block-by-block). Where a quantization scale / zero-point or AWQ calibration is needed, do a **two-pass
   *per tensor*** (pass 1 = stats, pass 2 = quantize) — both bounded to one tensor, never the file.
3. **Flush page-aligned (16 KB) Q42 blocks to disk / OPFS incrementally** as each tensor encodes (STELLAR §A:
   ~256 MB chunks across Web Workers; flush immediately). Write the CBOR-LD semantic / provenance header + the
   birth record (§1).

**Peak memory ≈ the largest single tensor's working set — not the whole model.** A multi-GB F16/Q8 model
therefore transcodes on a memory-constrained device (or streams in via WebTorrent / HF) without ever
materialising the full file in RAM. Combine with **demand-paged mmap** to *run* results larger than physical
RAM.

**Rail-check:** bounded-memory transcode **is** the affordability test (no big-RAM machine required to ingest);
heavy and **AOT / off-hot-path / off-device + distributed** (§0, §6); the legacy path is preserved (additive).

---

*Companion to [`RENDERER_DEFINITION.md`](RENDERER_DEFINITION.md) §12. Refinements applied: idempotent
delta-set join (§3), Track-B-is-AOT-only (§2.2), layered defense with lineage-primary (§4), real crypto suites
(§1.2). Governing test: §0.*
