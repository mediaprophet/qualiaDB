# ADR 0011 — Human-centric consent, accountability, and post-incapacity/death disposition (welfare & fairness)

- **Status:** Accepted (domain models built + tested; real-crypto/storage wiring deferred — see §Consequences).
- **Date:** 2026-07-06
- **Direction:** Timothy Charles Holborn, across a design dialogue on the welfare + fairness (wellfair)
  provisions in webizen-desktop.
- **Supersedes / relates:** builds on ADR 0003 (permissive-commons billing gates), ADR 0004 (bilateral
  guardianship scrubbing); realises `docs/plans/social-worker-support-and-accountability.md`,
  `human-centric-care-relationships.md`, `post-death-continuity-and-self-definition.md`,
  `social-fabric-distributed-memory-custody.md`, and the `reproductive-continuum` guardianship model.

---

## Context

Wellfair (welfare + fairness) must serve people at their most vulnerable — including those seeking protection
*from* powerful actors (a politically-exposed person, a political donor), those in the hands of helpers
(social workers, clinicians, facility staff) who may be diligent, overwhelmed, or hostile, and those who die
or are incapacitated. Several hard, interacting requirements emerged, none of which a naïve design meets:

1. **Consent must be real and revocable** — a person grants an agent access to sensitive data, and can take
   it back, with the data *actually* becoming inaccessible (not a flag flip).
2. **Accountability must be durable** — a person revoking consent must not thereby erase a helper's
   accountability; a helper must not hold the person's data hostage *for* accountability; a betrayer must not
   be able to delete the evidence.
3. **Records must resist deletion** — for the murdered, the disappeared, the discredited, and for evidence,
   erasure is often the wrong-doer's goal.
4. **Privacy must be preserved *and* selectively liftable by the person** — because privacy is sometimes
   weaponised against the person (a psychiatric committal used to discredit).
5. **Authority must be legitimate but never unilateral** — a court or statutory authority may act, but under
   checks, accountable to the person and to the democratic-legal order.
6. **Betrayal must be knowable and attributable** — including by a staffer, not only the office-holder.
7. **Death and incapacity need governed, reversible disposition** — validated, not by an abusable button.
8. **Fairness in negligence** — a helper who could not reasonably have known is not negligent; one who failed
   to check *given the means* and caused harm is.

**Grounding stances (non-negotiable, and the reason the naïve designs fail):**

- **Not "self-sovereign".** Sovereignty is a *collective-political* concept — popular sovereignty (democracy),
  state sovereignty (jurisdiction, international law: UDHR/ICCPR/CRPD are inter-state instruments), and the
  self-determination of peoples. The individual is a **dignity-bearing rights-holder *within* a
  democratic-legal order**, not a sovereign above it. "Self-sovereign identity" both misappropriates the
  concept and corrodes the order that protects the person; it cannot model an infant, a developmental
  gradient of agency, a lifelong-supported adult, or lawful authority. (See ADR context in the reproductive
  guardianship model: personhood is *relational and developmental*, stewarded, never atomic.)
- **Human-centric, dignity both ways.** The technology serves a good human relationship; it cannot
  manufacture one or fix a bad actor. Legibility is principal-held, **never a public rating/reputation
  weapon** — dignity for helpers too. Its everyday job is to **exonerate the diligent**.
- **Forum internum / Sanctuary.** A person's inward self-assessment and their most sensitive records are
  most-restrictive, non-default-disclosed. Classification is intrinsic to the type, not a policy note.
- **Honest-contingent, grounded-or-refused, never a verdict.** The system *enables* truthful transparency and
  fair assessment; it cannot compel honesty, and it never issues an automated determination of guilt.

---

## Decision

Build a **human-centric consent & accountability fabric** as a set of composable domain primitives, each with
its invariants encoded in the type system and tested, and each deferring its real cryptography/storage to a
coordinated composition step. The primitives (all in `crates/qualia-client-core/src/`, plus the reproductive
guardianship model in `crates/wellfare-core/src/anatomy/`):

### D1 — Encrypted permissive-commons payload (`consent_credential::EncryptedCommonsPayload`)

The sensitive payload is a **content-addressed ciphertext replicated across a permissive commons** (the
person's chosen friends/storers) so **it cannot be deleted** (anti-erasure — not by a betrayer, a hostile
actor, or accidental loss), yet is **opaque without a credential**. *Wide storage ≠ wide access.* Unilateral
deletion by one storer leaves the copies others hold. This resolves *anti-deletion vs privacy vs
access-control* simultaneously.

### D2 — Consent credential with crypto-enforced revocation (`consent_credential::ConsentCredential`)

Access is granted via **envelope encryption**: the credential carries the **wrapped data key**. **Revoke is
crypto-enforced, not a flag** — it *destroys the wrapped key*, so there is no key to decrypt with (the key
does not even survive serialisation). Because you cannot delete bytes others hold, **revocation is *access*,
not deletion**. The person's ultimate erasure is **crypto-shredding** (`is_crypto_shredded`): once *no*
credential grants a live key, the payload is permanently unreadable though the commons bytes survive.
*(Supersedes the flag-`revoke` in `wellfair/consent_store.rs` — coordinate.)*

### D3 — Court/authority credentials + multi-sig, no unilateral authority (`consent_credential::{CredentialAuthority, Authorization}`)

A credential's authority may be the **subject**, a **court** (to support proceedings/audit), or another
attested **authority**. A credential may be **multi-signature**: an exercise then requires **(a) instigation
by a participating party** — *no outside/authority actor acts alone* — **and (b) a threshold of party
signatures**. Even a valid court credential, if multi-sig, cannot be exercised without a participating party
setting it in motion and the threshold signing. Revocation still beats any authorised exercise. This is the
check on authority: *unable to act without instigation of one of the participating parties.*

### D4 — Durable, attestable conduct record (`consent_credential::{ConductRecord, Attestation}`)

How and why an agent acted (access / decision / request / escalation / **omission**) **persists after
revocation and after the payload is gone**. It binds to the payload **commitment** (not the payload), so it
proves the agent acted on a specific datum **without retaining or re-exposing it**, and carries an
**`Attestation`** — a signature (ed25519/ML-DSA) and/or a **zero-knowledge proof** (over the real
`crypto/zk_proofs` Groth16/BLS12-381) — so a court can audit *that the agent acted, on what basis, when*,
without the private data.

### D5 — Disclosure traceability: betrayal is knowable + attributable (`disclosure_trace`)

The retaliation case: a person "**cc**"s a **transparency credential** to an oversight authority (MP,
minister); if that authority **or their staff** leaks to the perpetrator, it must be knowable.
`TransparencyCc` durably records that the authority was informed; `DisclosureEvent` records who accessed
what/when/under-which-credential and **by whom, including a delegate** (`acting_delegate_did`) — so
`accountable_actor()` **attributes a staff leak to the specific staffer**, not just the office. Onward shares
are recorded; a per-recipient **fingerprint** + `trace_leak` traces a leaked copy to its source; the trace
itself lives in the un-deletable commons. Load-bearing for **UN / World-Bank development-funding** and
**human-rights-support** anti-corruption / anti-retaliation.

### D6 — Dead-man switch: governed, reversible post-death disposition (`dead_mans_switch`)

If the principal is *considered dead*, their data may be made public or subject to their own prior rules
(erasure-prevention / right-to-truth). The trigger is **gamified, not an abusable button**: a **liveness
lapse** (no "still here" for X) **and** a **quorum of distinct participating parties** attest
(`NoContact/BelievedDead/Abandon`), enacted by the friends who hold the dataset. `Disposition::{MakePublic,
ReleaseTo, SelfDefinedRules}` — the person's prior self-definition governs. **Reversible:** the principal
showing up alive resets and un-fires it.

### D7 — Incapacity switch + discrediting-counter (`incapacity_switch`)

Involuntary psychiatric admission / serious injury are *more common than death* and are **reversible**. An
`IncapacitySwitch` activates a pre-designated advocate under a **corroborated** trigger (party-quorum **and**
optionally an official instrument — a committal order / medical record) and **reverses on recovery**. The
sharp part: a psychiatric committal is often **weaponised to discredit** ("no-one believes them"), *leveraged
off privacy*. The counter is `TransparencyInvocation`: the person (or, only during a validated active
incapacity, their advocate) **chooses** to make a **scoped** set of durable prior-events records transparent
— reframing madness as retaliation. It is the person's **choice** (privacy never forcibly lifted) and
**honest-contingent** (the system enables truthful transparency; it cannot compel honesty or make a dishonest
record true — but durability means they cannot delete what they don't disclose).

### D8 — Duty of inquiry: expectations that *define* negligence (`duty_of_inquiry`)

Facility staff often *cannot understand* specialised/international/secrecy-bound work — a real specialist's
work can look like grandiosity. Fairness cannot require them to *understand*, but *can* require them to
**check the means when means are available**. `assess(DutyOfInquiry, ConductAgainstDuty)` classifies:
**`NoFault`** (means not accessible — could not have known), **`Diligent`** (accessible means checked),
**`UncheckedNoHarm`** (accessible means unchecked but no harm — a shortfall, honestly not inflated), and
**`Negligent`** (accessible means unchecked **and** a harmful act followed — *"failure to check given the
means, then acts that cause further injury"*). Malfeasance (checked/knew and harmed) is the intent case,
beyond this classifier.

### D9 — The accountability spectrum + the Six Vectors (`docs/plans/social-worker-support-and-accountability.md`)

Helpers (social workers especially) are **supported to help** *and* **fairly accountable** across four loci
kept apart: **helped**, **unable/no-fault** (the *system* failed — worker exonerated, systemic failure
surfaced), **negligence** (per D8), **malfeasance** (court/political/government — D4/D5 evidence). The
mechanism is the **Six Vectors of Transparency** (Who/When/Why/What/Where/**Cost**) applied to a human worker;
the **Cost** vector (requests ↔ system answers, and omissions) is what separates no-fault from negligence.
Authority is **consensual** (care-relationship) or **statutory** (proportionately-required-by-law, accountable
to the person *and* the democratic-legal order).

---

## Consequences

**What this enables.** A person can grant/cc scoped access to helpers and authorities over a durable,
un-deletable, credential-gated commons payload; revoke access by key-destruction (with crypto-shredding as
final erasure); hold even a court to *no unilateral action* (multi-sig + party instigation); retain a durable,
court-auditable conduct trail that survives revocation and re-exposes nothing; make any betrayal — by an
authority *or their staff* — knowable and attributable; govern death and (reversibly) incapacity by validated
rules enacted by chosen friends; counter weaponised discrediting on their own terms; and be assessed for
negligence *fairly*, distinguishing "couldn't have known" from "didn't check, and harm followed."

**Built (domain models + invariants + tests, this session):** `consent_credential` (12), `disclosure_trace`
(6), `dead_mans_switch` (7), `incapacity_switch` (6), `duty_of_inquiry` (5); the **`accountability_ledger`**
(5 — the tamper-evident signed-WAL, with **real** `sha2` + `ed25519`: append-only, hash-chained,
per-entry-signed; `verify()` detects content-modification, deletion/reorder (broken chain), and forged
signatures, naming the offending entry); the reproductive guardianship / stewardship-commons +
agency-gradient (`wellfare-core::anatomy::birth`, 7) that grounds the "not self-sovereign" stance; the
score-card as forum-internum selfhood content wired to the desktop.

**Wired to the desktop (2026-07-06):** the ledger + consent-credential + conduct/audit loop is now reachable
from the app, not domain-models-only — `qualia-client-core/src/accountability_store.rs` (persistence; every
act written into the signed hash-chained ledger — grant/revoke/conduct; 4 tests) + 8 host-API methods
(`wellfair/api.rs`) + 8 Tauri commands (`webizen-desktop`) + a `WellfairAccountabilityPanel` Studio tab
(grant / record-conduct / per-credential revoke + audit-trail / live `verify()` integrity readout). Green on
all four targets (store tests 4/4; desktop build; studio host; studio wasm-check). The panel exposes the
*crypto-enforced-revocation*, *conduct-survives-revocation*, and *tamper-evidence* properties for real today;
the store composes the top-level modules and does **not** touch `wellfair/consent_store.rs` — that is a
separate *policy*-consent store (`ConsentGrantRecord`, an ABAC rule the `PolicyDecisionService` reads; it
holds **no key material**, so a `revoked` flag is correct for what it is), distinct from this
*envelope-encryption* credential. (Not another instrument's lane — the wellfair workstream is allocated to
this instrument.)

**Deferred (the honest remainder — coordinate with the consent/crypto/vault/seeder lanes):** the real
**envelope encryption** + key hierarchy (`vault`); the **crypto-revoke** upgrade to `wellfair/consent_store.rs`
(flag → key-destruction); **Shamir threshold / social recovery** and **key-release-on-enact**; the real
**ZK authorisation circuit** and **per-recipient watermark / traitor-tracing** (`crypto/zk_proofs`); the
**commons replication** of the ledger (swarm/WebTorrent/`/sync`/CRDT) — the signed-WAL tamper-*evidence* is
built (`accountability_ledger`), replication for anti-deletion *durability* composes on top. The domain shape
and invariants those must honour are now concrete — this is a specification, not a sketch.

**Risks / honesty boundaries.**
- `Disposition::MakePublic` is **irreversible in effect** once keys are released to a durable commons — so
  *which* dispositions are reversible, and the switch's grace/limits, are values calls (⚑ Timothy).
- Locus/negligence outputs are **proposals over evidence, never automated verdicts**; a court/ombudsman/
  political process decides.
- Transparency invocation is **honest-contingent** — the system cannot make an honest person of a dishonest
  one; it can only keep the record un-erasable and the disclosure recorded.
- None of this is a public reputation weapon; legibility is principal/authority-held.

**⚑ Where Timothy / legal / clinical input is required (out-of-band):** statutory-authority attestation model
per jurisdiction; the evidentiary/admissibility standard for the malfeasance case; the locus- and
negligence-classification criteria (sensitive, expert); the reversibility limits and switch trigger criteria;
key-management-after-death (Shamir/threshold across custodians); the anti-reputation-weapon legibility line.

---

## References

- Code: `qualia-client-core/src/{consent_credential, disclosure_trace, dead_mans_switch, incapacity_switch,
  duty_of_inquiry}.rs`; `wellfare-core/src/anatomy/{birth, scorecard, dyad, physiology, pathway}.rs`;
  `qualia-client-core/src/wellfair/anatomy_view.rs` (score-card surface) + `compute_scorecard` /
  `wellfair_compute_scorecard`.
- Plans: `social-worker-support-and-accountability.md`, `human-centric-care-relationships.md`,
  `post-death-continuity-and-self-definition.md`, `social-fabric-distributed-memory-custody.md`,
  `reproductive-continuum-and-maternal-fetal-dyad.md`, `epistemic-reasoning-and-investigative-pathways.md`.
- Standard: `docs/manuals/standards/init-draft-standards-wip-main/DigitalBirthRecord` (self-owned, biometric,
  guardianship-by-credential — the birth transition D-adjacent).
- Governance: `CLAUDE.md` §15 (fidelity to the principal; the Six Vectors of Transparency; the conduct audit
  log). Grounded in UDHR / ICCPR / CRPD.
