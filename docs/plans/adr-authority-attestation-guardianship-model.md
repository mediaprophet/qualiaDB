# ADR (draft) — Authority Attestation, Representation, and Guardianship-Role model

**Status:** Draft for Timothy's refinement (semantic vocabulary is his to coin)
**Date:** 2026-07-03
**Supersedes (design intent):** the flat `welfare_support::GovernmentLetter` record and the narrow
`clinical::ClinicalReport.author_label` string; folds guardianship into a role/purpose/basis model.

## 1. The correction

Records like a "government letter" or a "pathology report" are not leaf facts. They are **attestations**:
a *statement* that some **authorizing entity** stands behind, **enacted by a natural-person agent in a
capacity**, **scoped by a jurisdiction** (and department/branch), and **evidenced by one or more
representations**. "Government" is one upper-level `AuthorityType`, not the record kind.

The same shape describes a pathology report (company = authority; branch = department/location;
pathologist = agent-in-capacity) and, importantly, guardianship (a court/authorizing basis appoints a
role to an agent over a principal for a purpose).

## 2. Three separable axes

### 2.1 Assertor — *who stands behind the statement*
- **`Authority`** — the authorizing entity. `authority_type` (government | corporation | professional
  body | educational | community org | **individual practitioner** — an authority can be a single natural
  person), a link to a **`Jurisdiction`**, and 0..n **`Department`/branch/unit** sub-structure.
- **`Agent`** — the natural person who acted, in a **capacity/role** *for* the authority (the caseworker,
  the pathologist). The capacity is itself evidenced by the agent's **own credential** (e.g. professional
  registration) — so trust in the attestation composes from trust in the authority × the agent's capacity.
- **`Subject`** — the natural person the statement is about (usually the vault owner / principal).

### 2.2 Statement — *what is asserted*
The content (pathology finding, payment decision, guardianship appointment). Carries epistemic status and
an **evidence grade that is derived, not asserted**:
`assurance = authority-trust × agent-capacity-credential × representation-integrity`.
A plaintext PDF someone typed in is low grade; a signed VC from a trusted issuer via a registered agent is
high grade. This generalizes the clinical rule already shipped ("only a clinician-confirmed report maps to
clinician-observed evidence; never launder a self-report into authority-grade evidence").

### 2.3 Representation — *how it is evidenced* (0..n; may co-exist)
1. **`DocumentBlob`** — a PDF/letter, content-addressed in the blob store (already built).
2. **`Credential`** — the information as a verifiable credential (already built: `credentials.rs`).
3. **`Hybrid` / bound document** — a document blob **cryptographically bound** to a credential
   ("machine-readable credential baked into the PDF"): the credential asserts the document's content hash,
   so the human-readable PDF and its verifiable claim are one linked object. Binding = the credential
   carries the blob's `content_hash` (and/or the blob embeds the VC), verified on ingest.

The representation is orthogonal: the *same* attestation can arrive as a letter, a credential, or both.

## 3. Guardianship = attestation applied to roles (not a flat relation)

Guardianship is a **family of role-scoped, purpose-scoped, agent-mediated authorizations**, and its
authorizing basis *is itself an attestation* (a court order / enduring power of attorney / signed
delegation — Authority + Agent + Jurisdiction + Representation, per §2). A guardianship record binds:

- **principal** (the person under guardianship);
- **guardian agent(s)** (one or many — supports M:N);
- **`GuardianshipRole`** (financial administrator, medical decision-maker, legal proxy / POA, data
  custodian, welfare appointee, … — see §5, Timothy-curated);
- **purpose / scope** — bounded: *which* decisions, *which* data classes, *which* actions;
- **authorizing basis** — a reference to the attestation that establishes the role;
- **validity interval**, **consent-state** (the principal's consent where applicable), and
  **revocation limits**.

This plugs directly into the existing `PolicyService::Suspend { required_approvals }` and the suspended
transaction queue for M:N guardian approval, and into the Social Book's separate relationship / role /
delegation / proxy-consent objects. It never collapses to "X is guardian of Y."

## 4. What this unifies (why it's worth the refactor)

Government letter, pathology report, credential, and guardianship become **one shape** — a scoped,
agent-enacted, jurisdiction-bound, representation-evidenced statement by an authorizing entity, with
consent / validity / revocation — expressed in a shared vocabulary with specialized profiles. It also
unifies **`Jurisdiction`** with the cooperative plan's signed, versioned **jurisdiction packs** (§13.3):
the same jurisdiction entity an authority acts under is the one a tax pack is scoped to.

Reshaping of shipped work (generalize, don't discard):
- `welfare_support::GovernmentLetter` → `authority_attestation` (government = one `AuthorityType`); the
  government-letter-with-document-bytes path I just built becomes the `DocumentBlob` representation of it.
- `clinical::ClinicalReport.author_label` → structured `Authority`(pathology company) + `Agent`(pathologist,
  with registration credential) + `Department`(branch).
- `credentials.rs` → the `Credential` representation **and** the agent-capacity credential.
- `blob_store` → the `DocumentBlob` representation; hybrid = blob ⊕ credential bound by content hash.
- guardianship stub (`Suspend` type exists, unwired) → the role/purpose/basis delegation model above.

## 5. Vocabulary decisions reserved for Timothy (do NOT bake in until confirmed)

These are semantically/legally loaded and jurisdiction-varying — his to coin/curate:

1. **`AuthorityType` taxonomy** — the upper-level set (government / corporation / professional-body /
   educational / community-org / individual-practitioner / …) and their axioms.
2. **`GuardianshipRole` taxonomy** — the canonical roles and how they map across jurisdictions (this is
   the sensitive one; real legal meaning; must not be invented).
3. **Jurisdiction as a first-class entity** reusing the cooperative jurisdiction-pack model (recommended:
   yes) vs a lighter string for now.
4. Naming of the generalized record kind (`authority_attestation`? `attestation`? something he prefers).

## 6. Sequencing

1. Timothy refines §2–§5 and settles the §5 vocabulary.
2. Implement in `qualia-cooperative-core` (shared) — since authorities, jurisdictions, and agents are
   cross-domain, not health-specific — with `wellfare-core` compatibility re-exports.
3. Migrate the flat records to attestation profiles; keep old record ids traceable (per the cooperative
   plan's migration rules §20).
4. Wire guardianship into `PolicyService::Suspend` + Social Book + a Consent/guardian UI.

Until §5 is settled this stays a draft; the shipped flat records keep working in the meantime.

---

## 7. REFRAME (2026-07-03, Timothy) — supported agency, not custodial control

The §3 "guardianship" framing above was wrong in spirit: it inherited the legacy **warden** model
(strip capacity, install a proxy). The architecture's actual objective is the **opposite** — a *spectrum
of supported agency that amplifies personhood*. Grounded in the "Agency / Social Book" and "Digital Good
Samaritan" works, the governing principles are:

- **It amplifies, not diminishes.** Most delegations apply to *healthy, well people* who simply need
  structural or specialist support — an accountant, a clinical psychologist, an IT social worker, a
  work-peer who actually understands the work (a doctor is not well-suited to a work matter, and vice
  versa), or — when a person is isolated with no better option — a **software agent as a declared source
  of truth / advocate**. Crisis and impairment are cases, not the centre.
- **Selfhood ≠ personhood.** Delegation is only ever over *personhood* (socio-legal agency relations).
  *Selfhood* (inherent to the person) is never delegated — even for a child, even at stage zero.
- **Non-asymmetry.** Expectations, responsibilities, and permissions must never be asymmetrical; both
  parties carry accountability, and an evidence chain distinguishes honest best-effort mistakes from
  malice/negligence.
- **Values-anchored.** Relationships default to shared **value credentials** anchored to UN Human Rights
  instruments (for children, the UNCRC) — the semiotic boundary a delegation is tested against.
- **Parens patriae is the edge case,** heavily restricted and continuously audited — a flag, not the model.

`AuthorityType` is therefore reframed to three axes (**implemented** in `authority_type.rs`):
*Modality of Support* (Augmentative | Developmental | Advocacy | Automated) × *Trigger* (Persistent |
Declarative | Contingent) × *Accountability & Evidence* (Auditable Fiduciary | Mutual Consensus |
Values-Bound). Named relationships (professional delegation, developmental scaffolding, crisis-activated
Digital Good Samaritan, posthumous legacy, protective custodial) are **compositions** of these.

The role/domain taxonomy is likewise reframed from "GuardianshipRole" to **domains of agency** (the ~17
from the source doc: medical, financial, legal, education/training, residential, social welfare, personal
welfare, supervisory/protective, IT-social-work, digital-identity, data-privacy/consent, communication,
reputational, reproductive/biometric/genetic, digital-legacy, AI-proxy, civic/political — plus hybrids
and domain fabrics). Each is a domain of *personhood*.

### 7.1 Design decision — Trigger granularity (Q: crypto/event vs human consensus?)

**Both, composed.** A `Trigger` is a bounded boolean expression over primitive predicates:
- `VerifiableEvent(id)` — a cryptographically verifiable attestation/event;
- `TemporalWindow(from, to)` / `DeadmanSwitch(last_seen, timeout)` — time/liveness;
- `HumanConsensus { required_capacity/credential, m_of_n }` — **M signed human attestations** meeting a
  threshold (e.g. *2 registered physicians attest incapacity*). Each attester signs with a role
  credential, so subjective clinical judgment becomes *accountable and contestable* — it is part of the
  evidence chain, not a black box.

Composed with `And` / `Or` / `Not`. A crisis delegation might fire on
`VerifiableEvent(er_admission) AND HumanConsensus{2-of-N registered_physicians}`. Higher-stakes /
selfhood-adjacent domains require higher thresholds. Purely-cryptographic can't capture clinical
judgment; purely-subjective isn't accountable — the composition gives both. This reuses the existing
suspended-transaction queue + deontic logic for the M:N mechanics.

### 7.2 Design decision — Developmental transition of power as a child matures

A **monotonic, per-domain capacity-transfer schedule** — authority *flows from guardian to principal as
capacity grows*, never the reverse without due process:
- Each personhood domain moves through stages `GuardianSole → CoSigned(guardian+child) → PrincipalSole`.
  Co-signing is the scaffolding middle — it teaches and protects while sharing control.
- Each stage transition is gated by a `Trigger` (§7.1): an age/`TemporalWindow` milestone, **or** a
  capacity attestation (`HumanConsensus` — e.g. child + counsellor attest readiness), **or** the child's
  own declarative claim past a threshold age.
- **Progressive privacy by default:** as a domain reaches `PrincipalSole`, the guardian's *read* access
  to it is **revoked, not retained** — "do parents need to be privy to every thought? No." Visibility
  *decreases* over time.
- **Monotonic:** you can only move toward more principal autonomy. A rollback requires a *separate*
  Advocacy/Protective delegation with its own audit + due process — never a unilateral guardian reversal.
- Anchored to the **UNCRC** as the values credential a rollback is tested against; **selfhood is never
  delegated** even at `GuardianSole`.
- Every transition emits a signed `TransferEvent` (the evidence chain).

### 7.3 Next implementation (post-confirmation)

`agency_domain.rs` (the ~17 domains, extensible, sphere-tagged) and `agency_delegation.rs`
(`AgencyDelegation` = principal + agent(s) + domain + `AuthorityProfile` + `Trigger` +
values-anchor credential + scope + jurisdiction + precedence + validity + consent + evidence-chain ref;
plus the developmental transfer schedule and the ABAC evaluation with **selfhood default-deny** and the
non-asymmetry invariant). Naming moves from "guardianship" toward "agency/support" throughout.

## 8. Declared reliance & judgement provenance (2026-07-03, Timothy)

When a natural agent forms a judgement **for another person**, they must **declare which other agents
informed it**. This is evidentiary, with serious applications (liability apportionment, malpractice vs
honest error, insurance coverage, forensic root-cause). It is the mechanism that keeps the natural
person's authorship and responsibility intact while making their toolchain auditable — an agent can
neither hide that they used a tool nor offload responsibility onto it.

### 8.1 Agent types (weight and liability differ)
`AgentType` distinguishes: **natural person** (bears authorship + responsibility), **software agent / AI**
(tooling & provenance — never bears liability, but its use MUST be disclosed and scoped),
**organization**, **instrument** (has calibration/validation state), **dataset/source**. Modelled with
the extensible taxonomy so new agent kinds add without a schema change.

### 8.2 Two layers of declaration
- **Standing `RelianceDeclaration`** (capacity-level): an agent declares, as part of their role/capacity
  credential, which classes of agents/tools they *may* use and for what — enabling a principal to
  evaluate and **consent to the toolchain before relying on the agent**. ("Mary the clinical psychologist
  uses LLM X for drafting and consults specialist network Z.")
- **Per-judgement `JudgementProvenance`** (`informed_by`): the actual contributing agents for a specific
  attestation/decision. A `Reliance` records the `AgentRef`, the **nature** of reliance
  (diagnostic-support | drafting | data-source | consult | …), and **`within_validated_scope`** (was the
  tool/agent used inside its validated/insured competence — the source doc: "roles should be associated
  with whatever the agent is insured to cast judgements about"). Signed by the responsible agent.

### 8.3 Evidentiary rules
- Consequential-domain judgements **require** a `JudgementProvenance`; the responsible natural agent is
  `prov:wasAssociatedWith`, contributing agents are `prov:used` / `prov:wasInformedBy` (reuse PROV-O and
  `governance/provenance.rs`).
- The declaration is part of the **signed, immutable** attestation (evidence chain via the WAL/receipt
  path); a correction is a new attestation, never an edit.
- **Non-repudiation both ways:** the agent cannot later deny using a declared tool, and cannot be falsely
  accused. A judgement of consequence *lacking* required provenance is flagged lower-assurance; an
  **undeclared** reliance discovered later (e.g. a concealed AI diagnosis) is a serious integrity breach —
  the omission is itself evidence distinguishing malice/negligence from an honest mistake.
- Using an AI/tool **outside** its declared/validated scope for a consequential judgement is a red flag,
  surfaced to the principal and any reviewing agent.
- This composes with `HumanConsensus` triggers (§7.1): each attester's own judgement carries its own
  declared provenance, so an M-of-N incapacity finding is auditable down to each doctor's toolchain.

### 8.4 Open decisions before coding the agency layer
1. Naming: **"domains of agency"** (vs "guardianship roles") for the type names — confirm.
2. Values-anchor (UN-HR-instrument credential): **required** on every delegation, or optional-with-default?
3. Declared-reliance: **required** for all consequential-domain judgements (which domains count as
   consequential?), and what is the default assurance downgrade when it is absent?

## 9. Provenance as a DAG: procedure, knowledge-horizon, and permissioned disclosure (2026-07-03)

`informed_by` is not a flat list — it is a **DAG**. When agent A consults agent B, B's contribution
carries *B's own* `JudgementProvenance` (B's toolchain), nested under A's. Disclosure controls attach
**per node/edge**, not to the record as a whole — each contributor governs disclosure of *their* sub-
provenance, and the composite view is assembled per-viewer.

Each provenance node carries **three orthogonal dimensions**:

1. **Procedural-temporal trace** — the *order and timing* of steps/consultations (the procedure), a
   signed, append-only sequence (aligns with the "temporal file-systems" note and the Merkle-DAG history).
2. **Epistemic horizon** — a **content-addressed reference (Merkle root / checkpoint hash) to the
   information-state available to that agent at decision time**. This is the hindsight-resistance
   mechanism: evidentiary review reconstructs that exact info-state and asks whether the judgement was
   reasonable *given what was available then*, not given today's knowledge. Mandatory for consequential
   judgements. Reuses the existing checkpoint/Merkle-root primitive (every projection already carries its
   source revision, cooperative plan §8.3).
3. **Disclosure policy** — two independent axes:
   - *Who is asking* — the **process subject** (the natural person the decision is about) gets the
     **fullest default view** as a matter of agency and contestability; other agents / institutions get
     **proportionate, selective** disclosure keyed by (viewer role, relationship to subject, purpose) —
     ABAC, same evaluator as the delegation layer.
   - *What proof modality* — full reveal | **selective field disclosure** (reveal a subset of the DAG) |
     **property proof** (prove "≥2 licensed physicians", "tool within validated scope", "no undeclared
     AI" *without* revealing identities/tools).

**The subject's right-to-know** is the first-class default. It can be narrowed only by a contributor's
*justified* confidentiality — and even then the subject is owed a **proportionate proof** (the property,
not the raw detail: "you were assessed by 2 licensed specialists using validated tools; identities
withheld for reason X, contestable via path Y"). That balance — the subject's agency vs a contributor's
legitimate confidentiality — is precisely why proportionate/selective/ZK disclosure is needed rather
than all-or-nothing.

**ZK status (updated 2026-07-03, verified):** `crypto/zk_proofs.rs` now provides **real Groth16** over
BLS12-381 (arkworks 0.6, `zk-culling` is a *default* feature). Verified: 7/7 tests pass including two
**soundness** tests — a proof for `x·y=12` is rejected when the public result is falsified to 13, and a
false matrix product is rejected. The earlier "SHA-256 commitment, not a proof" note is **stale**. So the
"property proof" disclosure modality can use **genuine ZK**. The bounded remaining work is authoring the
specific *predicate circuits* (e.g. "≥ 2 valid licensed-physician attestations", "tool within validated
scope", "no undeclared AI") as R1CS arithmetic circuits over the existing `ArithmeticCircuit` builder —
the engine is real; each provenance predicate needs its circuit. Statements without a circuit yet fall
back to signed commitment + selective field disclosure, labelled as such (not "ZK").

Reuse: `governance/provenance.rs` (PROV-O), the checkpoint/Merkle roots (horizon), the credentials
selective-disclosure surface, and the sanctuary/policy ABAC evaluator (viewer permissions).

## 10. Result variation, instrument characteristics, and input veracity — the root-cause substrate (2026-07-03)

The same procedure can yield different results depending on the *characteristics of a toolchain
component* (a mis-calibrated instrument, a specific model/version, a tool's known error-rate/bias) or on
*what another agent provided* — accurate, mistaken, or maliciously false. So every provenance edge
(§9's DAG) carries, beyond the `AgentRef`:

1. **Contributor characteristics** — the result-affecting factors, so variation is *attributable*, not
   noise:
   - *Instrument:* version, calibration/validation state, known error-rate / bias / confidence,
     configuration, and its validated scope (ideally backed by an instrument validation credential).
   - *Software agent:* model + version + configuration.
   - *Natural person:* capacity + role credential.
2. **Input veracity — dual-timed.** Each contribution carries its veracity *as reasonably assessed at the
   time* (against the relying agent's epistemic horizon §9) **and** *as later determined* (hindsight /
   forensic review), over an extended epistemic vocabulary: `Accurate | Uncertain | Disputed | Refuted |
   Malicious` (extending the shipped `EpistemicStatus`). The **gap between at-time and later** is exactly
   where the malice / negligence / honest-error distinction lives:
   - reasonably-accurate-at-time but later-refuted → honest reliance;
   - detectably-false *within the agent's validated scope* but relied upon → negligence;
   - maliciously false and undetectable at the horizon → **root cause attaches to the source**, not the
     relying agent (provenance of harm).
3. **Assessment provenance (recursive).** A veracity characterisation is itself a judgement — it records
   *who* assessed it and *when*, with its own provenance node.

**Conflicting characterisations are held, not collapsed.** Two agents may characterise the same input
differently; the system preserves both **paraconsistently** (contradictory claims isolated, each with its
own provenance) and resolves them only through due process — never an automatic collapse. (Reuses the
engine's paraconsistent logic.)

**Root cause is traced through the DAG:** was the error introduced by a defective instrument, a
false/malicious source input, or the relying agent's mis-analysis? The DAG + characteristics + horizon
*is* the root-cause-analysis substrate — the basis for the doc's "distinguish malice/negligence from
best-effort mistakes" and for insurance (coverable when within validated scope).

**Honesty flag:** the system **records** this substrate faithfully and supports **human-reviewed**
adjudication. It does **not** auto-render a verdict — no VM/paraconsistent result is described as a legal
judgment; disputes require human review and appeal (reuses the cooperative plan's escrow/dispute
safeguards, §16.3). We build the evidentiary substrate; humans adjudicate.

Reuse: the shipped `EpistemicStatus` vocabulary (extended), paraconsistent logic, instrument validation
credentials, `governance/provenance.rs`, and the checkpoint horizon.
