# Human-centric identity, biometric sovereignty & rights-gated sensing

**Status:** design-and-considerations document (2026-07-04). A cross-cutting concern, not a build tracker — it
informs three plans rather than being owned by one. Nothing here is built; it records the principles,
use-cases, requirements, and honest limits that the computer-vision, computational-geometry, and
capability-dispatch work must absorb.
**Owner / lane:** Claude (Opus 4.8). Records implications *for* the CV and CGAL lanes; does not edit their docs
(reciprocal cross-references into those are a coordinated follow-up, not a barge).
**Informs:**
- **CV** — [`native-visual-intelligence-and-generative-3d.md`](native-visual-intelligence-and-generative-3d.md)
  (§11.2 there already says face/biometric recognition "require separate plans, policies, consent, threat
  models" — this is that consideration-set).
- **CGAL / geometry** — [`native-computational-geometry.md`](native-computational-geometry.md) (the geometric +
  spectral substrate biometrics actually run on).
- **Dispatch** — [`native-capability-ontology-and-dispatch.md`](native-capability-ontology-and-dispatch.md) (the
  rights-gated capability routing; the five improvements in §10 below weave into its D-tasks).

This is the QualiaDB / Webizen architecture and Timothy's long W3C work in this space. W3C VC/DID, CGAL, and
biometric prior art are instruments this work directs — never parents of it.

---

## 1. Foundational principle: a DID is an identifier, not an identity

Everything below rests on a distinction the wider market routinely corrupts: a **Decentralized Identifier is an
*identifier*** — a handle whose controller can prove control — **not an identity.** A **Verifiable Credential /
Verifiable Claim** is an *attestation about a subject*, selectively disclosable. **Identity** is neither: it is a
contextual, probabilistic inference over a *fabric* of many identifiers and claims — the enumerated state that
must never fully resolve.

The W3C VC/DID stack was *designed* to build exactly that fabric: many identifiers (pairwise DIDs exist
specifically to *prevent* cross-context correlation), many attested claims, assurance computed as a *confidence
relation over the set*. Marketing a DID *as* "your digital identity" inverts the design intent — it re-licenses
the one-canonical-ID collapse the technology was built to prevent, and that collapse is the on-ramp to
institutional ownership and surveillance. **The naming is not innocent; it is the wedge.** The tell between
careless and bad-faith use is whether they *accept the correction* when shown the design intent (pairwise DIDs,
selective disclosure, subject-held claims) — the careless will; the bad-faith resist, because non-collapse is
precisely what their product needs to defeat.

Qualia holds the correct model *structurally*:
`crates/qualia-core-db/src/modalities/logic/shacl_extensions/identity.rs` models identity as a bounded set of
attested `IdentifierBinding`s with a noisy-OR confidence, and rejects any binding claiming certainty
(`DefinitiveCollapse`) — "the out-of-band remainder is what keeps the person free." Identifiers and claims are
*bindings*; identity is the never-collapsing probabilistic *fabric* over them.

Corollary — **"identity" is legitimate only for the fabric/process** (the enumerated state, the wallet that
stewards it, the confidence-relation over the set), never for *the identifier*. The correction is not "never say
identity"; it is "identity is the never-resolving fabric, the DID is one thread in it."

## 2. The identifier fabric scales — and proliferation is expected

A human-centric-stewardship environment ends up with **more** identifiers, not fewer — face, gait, phonetics,
keystroke cadence, device radios (Bluetooth / WiFi), and more — to enhance capability, security, and safety. The
architecture already fits this:

- **Each identifier is another enumerated-state binding.** `IdentifierBinding` + the noisy-OR aggregate already
  model identity as a bounded set with a confidence relation; more identifiers raise aggregate confidence but the
  `DefinitiveCollapse` guard still forbids certainty no matter how many. (Flag: `MAX_IDENTITY_BINDINGS = 32` may
  need to grow or become per-context.) Fusion across weak signals stays an *epistemic claim* on `q` / `α`, each
  identifier's provenance/consent preserved (you cannot fuse identifiers the subject never consented to
  contribute); conflicts take the paraconsistent path.
- **Each identifier type is a capability the same gate routes.** Keystroke cadence = a temporal rhythm signature
  (`t` + spectral); gait = a spatiotemporal trajectory (`x,y,z,t`); phonetics = a spectral signature (`σ` / CQT —
  the *auditory* lane); device radios = network-layer identifiers. Identity is a fusion across geometry, spectral
  (vision **and** audition), temporal, and network capability families.

**Why proliferation makes sovereignty more essential, not less.** The same many identifiers are *empowerment*
when the person is their principal steward (gated, minimal-disclosure, attestable) and a *panopticon* when
institutions hold them. The stakes scale with the identifier count, so the sovereignty inversion is what makes
the proliferation safe to have.

## 3. Biometric sovereignty: the inversion

Governments and enterprises today build libraries of natural people's biometrics that become institutionally
owned assets driving surveillance — a rights inversion, treating a person's biometric identifiers (and, by
extension, their things) as institutional property rather than as inalienably theirs. The inversion: the
**data-subject is the principal owner**; a consuming agent (a traffic network, a supermarket CCTV) may *perform*
a biometric function only by *asking*, under verifiable credentials and context clauses, against information the
subject controls. The person holds the reference; the agent brings a probe + credential; the rights ontology
decides; a *scoped answer* returns — not the template.

1. **Locus, not extraction, is the architecture.** Traditional: capture → extract template → institutional
   library → match. Sovereign: the reference stays in the subject's store; the agent's probe + credentials route
   a *federated query* to the subject's locus (their device, or a consented enclave); the capability runs
   *there*; only a scoped answer crosses back. This is *why the kernels must be sovereign-executable* —
   native-first dispatch + WASM-in-the-browser + zero-heap / 128 MB. If biometric geometry can only run on an
   institutional GPU, the person cannot hold it. Enforcement seam: the SHACL `route_is_local` / `shapes_for_locus`
   model.
2. **Biometrics *are* geometry + spectral capabilities — reuse, don't silo.** Face = a landmark
   point-configuration → distances / mesh; fingerprint = minutiae point-set + topology; iris = a spectral
   signature (`σ`); gait = a spatiotemporal trajectory. These map onto the substrate being built: point-set /
   kNN, registration, `full_distance` in a feature manifold, the `σ` lane, alpha-complex / TDA. Biometric
   matching is *(geometry + spectral) capabilities routed through the rights gate* — not a new biometric engine.
   (None exists in tree; greenfield.)
3. **Exact geometry becomes ZK-provable geometry — the data-minimization superpower.** A biometric predicate is
   often geometric ("probe within distance ε of an enrolled template"). Exact predicates turn a geometric
   decision into a *provable sign* rather than a floating-point maybe — and a provable sign is what a ZK proof
   can wrap (real Groth16 in `crypto/zk_proofs.rs`; threshold/range in `zk_predicates.rs`). So an agent can be
   told "this probe matches *someone in the authorized set* under threshold τ" **without** revealing which
   template, the template, or the identity.
4. **A biometric match is the sharpest epistemic-claim case — never a fact.** A match is a claim with confidence
   / calibration on `q` / `α`, never auto-promoted, always contestable, with the human-attestation /
   paraconsistent path preserved. A false match with real-world consequences is exactly what the Sentinel /
   DenyRollback + epistemic-honesty machinery exist to catch. Determinism + provenance receipts make the
   decision *reproducible and auditable* — a civil-liberties property, because institutional biometric systems
   are black boxes.
5. **The gate must be un-bypassable.** The capability must be *unreachable except through a directive carrying
   the agent's VC + the context clause + the subject's consent*, validated fail-closed (`CredentialGate`;
   Critical never degrades). Exposing biometric geometry as a raw callable would defeat all of this; routing it
   as a gated capability is what makes the rights *structural*, not policy-by-politeness.
6. **The clauses are deontic + contextual.** "Where they are and who is asking" = deontic rules over (agent-VC,
   location, purpose, retention) — the N3 rights ontology + deontic logic the inference path already runs.
   SPARQL-MM carries the media-fragment / spatial side (exists, needs repair before exposure); the federated
   "query the subject's store" side is a SPARQL-FED-*like* direction that **is not built yet**.

## 4. Harder cases: authority override, cross-institutional resolution, personhood

The "subject consents, agent asks" model is the *floor*. PEPs, witness protection, intelligence functions, and
court-ordered monitoring / exclusion (ankle-bracelets, keep-away-from-schools orders) force three shifts — and
they *strengthen* the sovereignty argument.

1. **Multi-principal, obligation-weighted resolution — not consent-only.** The rights ontology must express
   *obligations* and *prohibitions* with precedence, not only subject-granted *permissions* — deontic logic
   (`deontic.rs`) the inference gate already runs. A court exclusion order is an authority-attested *obligation*
   + a scoped *permission*, jurisdiction- and time-bound. Witness protection is the inverse: a **non-derogable
   prohibition** against resolving identity/location — the `Critical`-never-degrades rule in `identity.rs`. "The
   appropriate response" is a deontic *resolution* over rules from several principals, not a boolean.
2. **Cross-institutional federated resolution, authority-weighted, minimal-disclosure preserved.** Deciding the
   response often means querying *other* institutional systems (court / protection / offender registries), each
   carrying its own authority-attestation so the answer's weight depends on it and conflicting obligations must
   be *ordered*. Minimal disclosure still governs: "is there an exclusion order for this person at this place?"
   returns a ZK-predicate / boolean, never a record dump. This is the discovery layer (deferred) plus an
   authority-weighted trust-and-precedence layer.
3. **Personhood semantics as a relational, attested, expiring, contestable overlay — never a collapse.** Minor,
   offender-under-order, protected-witness, PEP, incapacitated, deceased, natural-person-vs-entity — each
   modulates what is permitted, but each is an assertion *about* the person (own provenance, authority, expiry,
   contestability) on the `w` axis — never a permanent institutional stamp. An offender is still a person with
   the out-of-band remainder; witness protection is a non-derogable overlay, not a redefinition. Minors route
   through guardianship (`guardianship.rs`); escalated authority (judge / court) is an authority-attestation VC
   (`adr-authority-attestation-guardianship-model.md`).

## 5. Bi-directional accountability → choice

Because the query runs at / through the subject's locus, **the act of querying is itself a receipted event on the
subject's side** — sensing is bi-directional. Traditional surveillance is one-way; the sovereign inversion gives
the person a *counter-record*: who interrogated their fabric, when, under what claimed authority, granted or
refused. That record is *actionable* — it powers ordinary choice (shop there again or not; warn others; escalate)
without first winning a slow, asymmetric lawsuit.

This is the answer to the "there's always another subscriber, so the repercussions are minor" problem: it
**re-creates a consequence — lost patronage and reputation — where the legal system is too slow or too weak**,
peer-to-peer and offline-first, without depending on a strong regulator. It completes the honest limit of §3:
legibility is not only for courts; it feeds the person's *everyday* decisions, which in aggregate are faster,
more distributed enforcement than litigation.

**Design requirements (on the gate + receipt):** subject-side delivery (the receipt goes *into the subject's
store*, retrievable — a delivery guarantee, not theory); log refused attempts too ("this store tried, and was
refused"); intelligible and accessible ("Store X checked your face on [date] claiming loss-prevention; no consent
on file"). **The guard:** the freedom to act on the log must not itself be surveilled — a person must be able to
walk away without being re-identified *for* walking away, or the "choice" becomes a retaliation vector (the
out-of-band remainder, applied to exit). **Residual boundary:** bi-directionality is strong when the query
touches the subject's locus or a participating registry; weaker against a fully off-grid illegal library the
person learns of only via audit, leak, or disclosure — the architecture shrinks that region, it does not
eliminate it.

## 6. Spatiotemporal correlation for record refutation & source attribution (the highest-risk capability)

An erroneous / disputed record carries a claimed *time*, *place*, and *identity*. The subject's own sovereign
fabric — a timestamped GIS trail + local radio-environment logs (which BT / WiFi identifiers were in range) — can
be **cross-correlated against it** to either *refute* it ("I was elsewhere, with different neighbours") or
*attribute its source* ("the spoofed identifier co-occurs with device Z at place L, time T"). Real use-cases:
alibi / false-record refutation; geospatial-or-temporal-impossibility fraud detection; **unwanted-tracker /
stalking detection** (one identifier co-present across disjoint place-times *is* a follower); GPS-spoof
detection.

**It maps onto the substrate:** spatiotemporal correlation over `x,y,z` (GIS) + `t` (the witnessed provenance
ledger) + the identifier lane (a radio environment is a *set signature* of a place-time). The geometry spatial
query layer (BVH / kd-tree / box-join), the hash-chained WAL / `t`-ledger (which makes the person's log
*attestable*), and determinism + receipts (a *reproducible* refutation) are exactly what it needs. Greenfield on
that substrate.

**Why it is the highest-risk capability — the same inversion with extra care.** "Correlate timestamps + GIS +
radio logs to figure out where it came from" *is* the core technique of mass surveillance and commercial location
analytics; the same maths that gives a person an alibi gives an institution everyone's movements and *co-presence
graph* (who was near whom). So the inversion is the whole safety case:

- **The movement / co-presence log is the single most sensitive data class** — most strongly sovereign-held,
  encrypted, non-egressing (Sanctuary-vault class). The capability *raises* the encryption / no-silent-egress
  bar, it does not relax it.
- **Correlation reaching past the subject's own fabric is itself a rights-gated query.** You may refute *your
  own* record freely; the BT / WiFi identifiers of *other* people's devices in your log are *their* identifiers —
  attributing a source that fingers another device is a query against *their* fabric, needing authority / consent
  (or a court). Co-presence must not become a private dragnet.
- **Default output is a minimal, reproducible refutation predicate, not a movement dump** — "the fabric is
  inconsistent with record R" (yes / no, ZK-provable where possible), never "here is everywhere the subject has
  been."

## 7. Voluntary disclosure credentials, and the counter-record against institutional abuse

**Voluntary "availability" credentials — with a hard epistemic guard.** A person may pre-author, in their values
credentials, a scoped permission: "if my fabric shows I was present at a serious incident of class X, authority Y
may contact me." A subject-authored, revocable deontic rule. **The guard is non-negotiable:** presence is not
culpability and volunteering is not innocence — such a credential is an *availability offer*, never an *innocence
proof* (the volunteer may be the perpetrator), and its **absence must never carry adverse inference** (the right
to silence is preserved; else the guilty learn to volunteer and the silent are presumed guilty — both corrupt).
Presence-info is evidence for a competent human process, never a machine verdict. Every subject-authored
disclosure rule inherits the same "offer, not proof; absence not adverse" discipline.

**The counter-record against documentation asymmetry — the core accountability case.** Public-sector wrongdoing
(law enforcement, departmental staff, contractors — including *private* contractors engaged specifically so harm
can be done with accountability laundered away) depends on an asymmetry: the powerful actor keeps the record and
its testimonial stands unchallenged, because the natural person has *no records or computational system of their
own*. A sovereign, attestable, reproducible record **rebalances that evidentiary asymmetry** — and cuts through
the private-contractor laundering, because it documents what was done and its effects *independent of which entity
is nominally responsible*, so the victim need not first win "whose fault legally." This improves not only
human-rights outcomes but *productivity and public-expenditure performance*: unaccountable harm, wrongful
outcomes, corruption, and the distrust and litigation they generate are enormous deadweight costs, and legible
records reduce them — the human-rights case and the fiscal case in one.

Three requirements this sharpens on the record: **third-party-attestable, not self-assertable** (hash-chained WAL
/ witnessed `t`-ledger + external anchoring, so it survives scrutiny and is not forgeable by the person either);
**resilient to the bad actor's countermeasures** — confiscation (distributed backup), discreditation
(witnessing), and *compelled disclosure to find and suppress it* (encryption + the Sanctuary decoy-lane /
duress-PIN posture); **usable by the vulnerable** (accessibility-first is a rights requirement here — the primary
victims are often the least technical).

## 8. Biometric fabric as sovereign multi-factor unlock (why user-auth was deliberately delayed)

The biometric-sovereignty work is *also* how the person authenticates to their **own** qualia-db: if a person can
prove it is their face, phonetic signature, typing cadence, etc., then *enough of those tests together* define an
unlock. This is the *good* locus for biometrics — local authentication, template never leaving the device — the
exact opposite of an institutional matching library; same technology, inverted sovereignty.

**Multi-sig (M-of-N over the fabric), not one biometric.** Unlocking on a single definitive biometric would be a
`DefinitiveCollapse` in another guise — a single point of failure and a coercion target. The unlock is a
*threshold over a fabric* of factors (face + voice + cadence + device + a withheld secret), mirroring the
enumerated-identity model and the guardianship M-of-N pattern already in tree: any one factor may degrade or be
spoofed without breaking the whole; an attacker must defeat M at once.

**Why user-auth was deliberately delayed — and correctly.** Building the unlock before the biometric-sovereignty
model was resolved would risk baking in the wrong thing (a single-biometric unlock; a collapsing / leakable
template store). Sequencing the identity model first is the same "sovereignty first, then the capability"
discipline — a principled delay, not a slip.

**Honest technical caveats (the traps):**

- **Biometrics are not revocable** — you cannot reissue your face. A biometric must be *one input to* key
  derivation, never the key itself, with a recovery path, combined with a knowledge / possession factor (PIN,
  device key). M-of-N limits the blast radius of any one factor's compromise.
- **Fuzzy input needs fuzzy extractors** — biometric readings are never bit-identical, so they cannot be hashed
  directly into a stable key; stable key material from noisy biometrics requires fuzzy extractors / secure
  sketches. That is the concrete cryptographic piece.
- **Coercion / liveness** — a forced face, a photo, a voice recording; a withheld-secret factor in the M-of-N
  plus the Sanctuary duress-PIN / decoy posture are the mitigations (they can compel your face but not a secret
  you refuse), and anti-spoofing is an ongoing arms race, not a solved problem.

## 9. Implications for the CV and CGAL work (the reason this was raised)

These are requirements and implications the two build lanes should absorb — recorded here, not edited into their
docs (that's a coordinated follow-up).

**For CGAL / computational geometry (`native-computational-geometry.md`):**

- Biometric matching is *(geometry + spectral) capabilities*, not a silo — point-set / kNN (P6.1), registration,
  `full_distance` in a feature manifold, the `σ` spectral lane, alpha-complex / TDA are the biometric primitives.
- The **exact-predicate ladder (P1)** has a second payoff no one designed it for: an exact geometric decision is
  a *provable sign*, which is the substrate for **ZK-provable biometric predicates** (distance-below-threshold
  without revealing the template). Privacy-preserving matching is downstream of exact geometry.
- **Spatiotemporal correlation** (§6) is the geometry **spatial-query layer (P3: BVH / kd-tree / box-join)** over
  `x,y,z,t` + the `t`-ledger — the record-refutation / anti-stalking capability.
- **Determinism-as-attestability** (the whole P1 / `.10d` canonical-bytes discipline) is what makes a match /
  refutation *reproducible* — a civil-liberties property, not just an engineering one.
- **Sovereign-executable** (native-first + WASM + zero-heap / 128 MB) is the *enabling condition* for locus (§3.1)
  — it is why the geometry kernels must run on hardware the person owns.

**For CV / vision (`native-visual-intelligence-and-generative-3d.md`):**

- Its "a detection is an observation, not ground truth" rule is *load-bearing* for biometric matches (§3.4):
  claim with confidence, never auto-promoted, always contestable, human-attestation preserved.
- Its §11.2 ("no implicit identity recognition; face recognition requires a separate plan, policy, consent,
  threat model") points *here* — this document is that consideration-set.
- **Capture fail-closed** (its §4.3 / §11.1) is the ingress side of the un-bypassable gate (§3.5).
- The **disclosure-tier** default (boolean / ZK-predicate / scoped-attribute / full) belongs on any vision
  capability that touches a person, returning the least the requesting VC is entitled to.

## 10. Improvements this injects into the dispatch layer

These weave into `native-capability-ontology-and-dispatch.md` D-tasks (recorded in its §12 pointer):

1. **Locus / sovereignty axis in capability metadata (D0.2)** — device affinity says *what silicon*; a locus
   constraint says *whose sovereignty domain* a capability may run in / over a data class. `route_is_local` is
   the enforcement seam.
2. **Disclosure tier as a first-class capability property (D0.2)** — minimal result form (boolean / ZK-predicate
   / scoped-attribute / full); the gate returns the *least* the requesting VC is entitled to.
3. **Receipts cite the authorizing clause (D5.1)** — the exact deontic rule + VC + context clause, not only input
   hashes — so "on what basis, under whose authority?" is answerable and reproducible (and holds the *authority*
   accountable: an override with no receipt is itself a detectable violation).
4. **Consent epoch in the plan-cache key (D2.2)** — memoization respects revocation; a cached result tied to a
   since-revoked consent invalidates.
5. **Sensitivity high-water mark on derived plans (planner invariant)** — a plan inherits the most-restrictive
   class of its inputs; a biometric-derived intermediate cannot be laundered to a lower sensitivity.

## 11. ⚑ What's yours to decide (legal / policy / ethics — I will not invent these)

Consolidated curation datums from the sections above:

1. **Which identifier and biometric families are in scope** — an ethics / invasiveness call (keystroke dynamics
   and device-radio tracking are far more insidious than a consented face-match), not a capability question.
2. **The precedence ordering** of conflicting obligations across authorities and jurisdictions (§4).
3. **The personhood-status vocabulary** — which statuses exist, how each is attested, scoped, and expired (§4);
   some of this is the reserved vocabulary you keep the right to coin.
4. **Voluntary-disclosure-credential rules and their legal treatment** — "offer, not proof; absence not adverse"
   as a hard invariant (§7).
5. **The non-negotiable protection guarantees for the movement / co-presence log** — retention, encryption,
   compelled-access posture — *before* the §6 correlation capability is built (highest-stakes).
6. **The biometric-unlock M-of-N policy** — acceptable factor set, recovery / fallback path, duress /
   compelled-unlock posture (§8).

## 12. Honest limits (consolidated)

- This governs the **compliant, interoperable path**. It does not physically stop an institution that unlawfully
  captures and stores biometrics or movement data out-of-band. What it does: makes the rights-respecting path the
  default and the easy one (sovereign locus, gated, minimal-disclosure, attestable), inverts who holds the asset,
  and makes non-compliant use *legible and contestable* — standing, provenance, and a reproducible basis to
  contest. It does not abolish the physics of cameras or radios. Claiming otherwise would be dishonest.
- **Nothing here is built.** The identity model (`identity.rs`), guardianship, authority-attestation, ZK
  primitives, SPARQL-MM, and the Sanctuary vault exist as substrate; biometric matching, federated
  cross-institutional query, spatiotemporal correlation, and the biometric unlock are greenfield on top of them.
- Anti-spoofing / liveness is an arms race, not a solved problem (§8). Attestability reduces the *impact* of a
  spoof (it is contestable and reproducible) but does not prevent one.
