# ADR (DRAFT) — Sanctuary Vault v2: CBOR-LD harmonization + coercion-response decoy mirroring

**Status:** DRAFT for Timothy's sign-off. Supersedes nothing yet; extends
[adr-sanctuary-threat-model](adr-sanctuary-threat-model.md) (D1–D6) with the format + decoy-model
decisions raised 2026-07-03.

**Origin:** Timothy's direction — (1) harmonize the vault to CBOR / CBOR-LD like the rest of the
system, and (2) a coercion-response decoy model where activity in the *decoy* session is surfaced in
the *real* lane (for the victim's awareness / evidence), the real user can curate the decoy to sustain
the coercer's false belief, and the whole thing has evidentiary value if the victim cannot escape or an
investigation is under way.

---

## 1. Is there "bifurcation in the q42 file structure" that helps? — Yes, but be precise

There are (at least) three distinct bifurcations in the q42 stack. They are **structural / semantic /
governance** bifurcations — routing and provenance — **not cryptographic confidentiality boundaries**.
That distinction is the crux.

| Bifurcation | Where | What it separates |
|---|---|---|
| **`w` Manifold Index / "Topological Bifurcation"** | `q42-10d-tensor-standard.md` §4.3 | Domains/manifolds (Medical / Legal / Personal), with cross-domain "wormhole" correlation between them |
| **Natural-person vs software-agent DID** | `Q42VolumeHeader` (quin `object` upper-4 bits) | *Who authored* a quin — provenance / agent type |
| **`wf:` bifurcated CRDT consensus** | `webizen-protocol-rfc.md` §Tri-Party | WellFair data (no auto-merge; manual Tri-Party Authorization) vs Commons `qp:` (Lamport LWW) |

**What this buys the decoy model:** the `w`-manifold bifurcation gives a clean, standards-native *place*
to model **real / decoy / audit as distinct manifolds** (distinct `w`), and the DID bifurcation gives a
ready-made **provenance tag** for "who wrote this decoy-session entry" — exactly what an evidentiary audit
log needs. The `wf:` bifurcated-CRDT + Tri-Party model is the right governance frame for consent over that
data.

**What it does NOT buy:** the manifold index is not a key boundary. Putting real in `w=0` and decoy in
`w=1` does **not** keep a coercer out of `w=0` — only *distinct keys* do. So the bifurcation is the
skeleton; **confidentiality and one-way visibility still come entirely from the crypto we layer on top.**
Anyone who claims the q42 bifurcation *is* the decoy isolation is wrong; it is the container, not the lock.

## 2. CBOR-LD harmonization (Timothy: yes)

**Decision (recommend: yes).** Move the vault serialization from `serde_json` to CBOR-LD, aligning with
the project standard (`ciborium` is already a core-db dep; there is a real `cbor_parser::parse_cbor_ld_stream`;
CBOR-LD is a first-class `SemanticFormat`). Also fix the mislabel where `companion_bundle` writes
`serde_json::to_vec` into a `package-manifest.cbor`.

**What it buys, honestly:** (a) consistency with the graph/ingestion path; (b) the lanes + audit log
become q42 quins/manifolds — expressible in the graph, provenance-tagged, governed by the `wf:` model;
(c) binary, so a coercer who opens the file in a text editor sees bytes, not the literal words `real` /
`decoy`. **What it does NOT buy:** cryptographic hiding. CBOR-LD is self-describing — anyone who *decodes*
it recovers the structure. So this is consistency + expressiveness + casual-readability, **not** deniability.

## 3. The coercion-response decoy model (Timothy's design)

The intent: **decoy activity → reported into the real lane; real user curates the decoy; evidentiary.**
There is a fundamental tension to resolve first, then two crypto primitives that resolve it cleanly.

**The tension:** in a decoy session you have the *decoy* PIN, never the *real* PIN, so the real lane's key
is never derived — you therefore **cannot write into the real lane from a decoy session.** A naive "log the
coercer's edits into the real lane" is impossible as stated. The fix is asymmetry.

### 3.1 Decoy → real: a write-only, blind audit channel (asymmetric crypto)

At setup, generate an **audit keypair**. The **public** key is available in *any* session (including
decoy); the **private** key lives only in the real lane (wrapped under the real key). Then:

- Every decoy-session mutation (add / edit / delete a note) appends an **audit record** — timestamp,
  action, a copy/diff of what the coercer wrote, and the DID-bifurcation provenance tag — **encrypted to
  the audit public key** into an append-only region.
- The decoy session can **write** these records (public-key encrypt) but **cannot read** them (no private
  key) — so the coercer cannot tell they are being logged, and cannot tamper-read or selectively delete
  with knowledge of content.
- On the next **real** unlock, the real lane unwraps the audit private key and **decrypts the coercer's
  activity** — the victim sees exactly what was added/changed under duress and decides: **keep** it,
  **keep it updated** (ongoing evidence if they can't yet escape / an investigation is live), or **archive**
  it. Append-only + provenance-signed = evidentiary.

This is the standard "write-only log / receiver-decrypts" pattern and it is sound.

### 3.2 Real → decoy: a one-way key hierarchy (curate the decoy)

To let the real user **maintain the decoy** (seed it, keep it lived-in, add plausible updates so the
coercer's re-unlock shows fresh, believable content and reinforces "I have the real one"):

- At setup, **wrap the decoy lane key under the real lane key.** In a real session the app can unwrap the
  decoy key and write into the decoy lane — *without* the user re-entering the decoy PIN.
- The reverse is impossible: the decoy lane holds nothing that unwraps toward real. **One-way: real
  governs decoy; decoy is sealed from real.**

Result: the victim, in a safe real session, curates the decoy; under coercion the decoy behaves as a
fully-live vault (it already does — confirmed in code); the coercer's own writes persist *and* the seeded
updates make it plausible.

## 4. The load-bearing trade-off (the one place I'd have you decide, not me)

Every enrichment in §3 — the wrapped decoy-key, the audit keypair, the append-only audit region — **adds
on-disk structure**. To a coercer who only *uses the app*, this is powerful and invisible. To an adversary
who *inspects the bytes* (forensics, a savvy partner with a hex editor), each added relationship is **more
evidence that a superior real lane exists behind the decoy.**

So there is a real fork, and it cannot be fully won both ways in one container:

- **A — Functional decoy (recommend for the intimate-partner / coercive-control threat).** Rich decoy
  mirroring + audit + curation. Optimizes for a coercer who *uses the device*. Accepts that a *forensic*
  inspector can see a two-tier structure exists. This matches the stated threat (a partner demanding the
  PIN), and delivers the evidentiary value.
- **B — Indistinguishable hidden volume (VeraCrypt-class).** No on-disk marker of a second lane at all;
  fixed-size opaque container. Optimizes for the *forensic/border* inspector. Costs the rich mirroring
  (a hidden volume can't safely host a real→decoy key hierarchy without leaking it), and is a much larger
  build.
- **C — Layered (both, later).** B on the outside (opaque container), A on the inside (rich model within
  the hidden volume). Most protective, most work; a distinct future workstream.

**My recommendation:** ship **A** now (it fits the coercive-control threat you're actually describing and
carries the evidentiary value), name its limit plainly in the UI, and keep **B/C** as a scoped future
option. **This is your call, not mine — it's a values decision about which victim you're protecting first.**

## 5. Argon2id + PIN policy (already built; ADR D1/D5)

Landed and format-independent (survives the CBOR-LD move unchanged — same per-lane `KdfDescriptor`):
Argon2id (64 MiB, t=3, p=1) for new vaults, a per-lane KDF descriptor (so old PBKDF2 vaults still open),
and a PIN-strength floor (min 6, blocks all-identical / sequential / common). **Lazy PBKDF2→Argon2id
re-derivation was intentionally NOT finished on the JSON format** — it folds into the v2 re-serialization
so we migrate the container **once**, not twice.

## 6. Decisions for Timothy

- [ ] **D-fmt:** approve CBOR-LD for the vault (recommend yes).
- [ ] **D-decoy-model:** approve the §3 asymmetric-audit + one-way-key-hierarchy design.
- [ ] **D-fork (§4):** **A (functional decoy)** now vs **B (hidden volume)** vs **C (layered)**. *The one
      that needs your judgement.*
- [ ] **D-retention:** default handling of the decoy-activity audit log — keep / prompt / auto-archive; and
      whether audit records are signed for evidentiary use (chain-of-custody).
- [ ] Confirm sequencing: land Argon2id/PIN now, then build v2 (CBOR-LD + migration + decoy mirroring) as one.
