# ADR (draft) — WellFair "Sanctuary" encrypted vault: threat model & open decisions

**Status:** DRAFT — awaiting Timothy's decisions (see the checklist in §7).
**Date:** 2026-07-03
**Task:** T2.1 (Sanctuary threat-model ADR) in `docs/plans/MASTER-EXECUTION-CHECKLIST.md`.
**Scope:** the encrypted-at-rest Sanctuary vault, its decoy lane, and the optional T1.2 OS-keychain
pepper — *not* the read-time projection filter in `wellfair::sanctuary` (which only hides journal rows
on read and is not a cryptographic boundary).

**Code this ADR describes (read before editing so claims stay accurate):**
- `crates/qualia-client-core/src/wellfair/sanctuary_vault.rs` — lanes, verifier, pepper folding, setup/unlock.
- `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` — KDF, AEAD, deterministic nonce derivation.
- `crates/qualia-core-db/src/crypto/sanctuary_keychain.rs` — OS-keychain pepper I/O.
- `crates/qualia-client-core/src/wellfair/api.rs` (~L799–862) — the `WellFair` state methods that expose it.

This ADR does **not** decide anything by itself. It states the threat model honestly, says what the
current design does and does not defend against, and lays out each open choice with options and a
*recommended* default that is Timothy's to accept, reject, or amend.

---

## 1. Context & assets

Sanctuary is the vault for the most sensitive things a WellFair user holds: notes about abuse, coercion,
health, legal or immigration status — material whose disclosure could cause real harm. Concretely, the
protected assets are:

1. **Sanctuary notes** (`SanctuaryVaultNote { id, body, created_at_unix }`) — free-text bodies, the
   sensitive payload.
2. **The lane keys** — 48 bytes of derived material per lane (32-byte AES key + 16-byte nonce tweak),
   never persisted; re-derived from the PIN on each call.
3. **The existence/among-lanes structure** — that a *decoy* lane exists at all, and which lane a given
   PIN opened, is itself sensitive under coercion.

### 1.1 The design as actually implemented

| Property | Implementation (verified in code) |
|---|---|
| Two independent lanes | **Real** and **Decoy** `LaneState`s, each with its own random salt, verifier, records blob, nonce counter. Different key → different ciphertext; a duress unlock only ever touches the decoy lane (`add_note`/`list_notes` route strictly by the lane the PIN opens). |
| Per-lane salt | 16 bytes from a v4 UUID (CSPRNG/`getrandom`-backed) — `random_salt()`. |
| KDF | **PBKDF2-HMAC-SHA256, 310 000 iterations** (`DEFAULT_PBKDF2_ITERATIONS`), producing 48 bytes split into a 32-byte AEAD key + 16-byte volume tweak. |
| AEAD | **AES-256-GCM** (`ALGO`), 16-byte tag. Nonce is **deterministic**: derived from the per-lane volume tweak XOR a monotonic `chunk_index` counter (`derive_chunk_nonce`), so a nonce never repeats under one key while counters are managed. Each lane also binds a domain-separating AAD (`wellfair:sanctuary:real` / `:decoy`). |
| PIN handling | The PIN is **never stored**, not even hashed. A per-lane **verifier** — the fixed magic `WELLFAIR-SANCTUARY-VERIFIER-v1` encrypted under the lane key — is decrypted to recognise which lane a PIN belongs to. Minimum PIN length is enforced at **4** characters; the two PINs must differ. |
| "No plaintext on disk" | When the vault is locked there is only ciphertext on disk; a test (`plaintext_never_appears_on_disk`) asserts a sensitive body never lands in the vault file. AEAD tamper-detection is tested (`tampered_ciphertext_fails_to_open`). Key material is `Zeroize`/`ZeroizeOnDrop` and its `Debug` is redacted. |
| Optional OS-keychain pepper (T1.2) | **Opt-in, off by default.** When wrapped, the KDF input becomes `effective_secret = SHA256("q42:sanctuary:pepper:v1" ‖ pepper ‖ pin)` where the 32-byte `pepper` lives in the platform keychain (Windows Credential Manager / macOS Keychain / Linux Secret Service). Disk + PIN alone then cannot open the vault. `setup_wrapped` returns a **one-time hex recovery code** (the pepper itself); `unlock_with_recovery` re-seats it on a new device. Unwrapped vaults derive exactly as before (`pepper = None` → PIN used verbatim). |

**Note on process model:** every operation re-reads the vault file, re-derives the key, and drops it —
there is no long-lived in-memory "unlocked session" object held by the vault module itself. Whatever UI
layer calls `list_notes`/`add_note` holds the PIN and plaintext transiently for the duration of the call.

---

## 2. Threat actors & scenarios

We model an adversary who may gain one or more of: the device (powered-off or running), the vault file,
the OS keychain, or coercive control over the user. The scenarios:

| # | Scenario | What the adversary has |
|---|---|---|
| A | **Device theft, at rest** (powered off / logged out) | The disk, hence the vault file and any keychain store, but not the PIN. |
| B | **Coercion / duress** | The user, compelled to unlock. Adversary can watch the screen and demand a PIN. |
| C | **Local malware / unlocked running session** | Code execution as the user while the app is (or has recently been) unlocked. |
| D | **Backup / sync exfiltration** | A copy of the vault file via cloud backup, sync, or file share. |
| E | **OS-keychain compromise** | The platform credential store's contents (e.g. from a running-session malware, or a backup that includes it). |
| F | **Brute-force of a weak PIN** | The vault file (and, if wrapping is on and also captured, the pepper), plus offline compute. |
| G | **Multi-device** | The user legitimately wants the vault on more than one machine. |

---

## 3. How the current design addresses each — and where it does not

### A. Device theft, at rest — **strong**
Locked, there is only AES-256-GCM ciphertext plus salts and an encrypted verifier. No PIN, no key, no
plaintext. Recovering notes requires deriving the key, which requires the PIN (and, if wrapped, the
pepper). **Residual:** the *existence* of two lanes is visible in the JSON structure (`real` and `decoy`
objects) — an adversary who reads the file knows a decoy scheme is in use (see §3-B).

### B. Coercion / duress — **partial (this is the decoy lane's whole purpose)**
The duress PIN opens the **decoy** lane and only ever writes/reads there; the real lane is never touched
by a duress unlock. Under compulsion the user surrenders the duress PIN and the adversary sees a
plausible-but-non-incriminating store. **Residual risks, honestly:**
- **The decoy is structurally visible.** The on-disk `VaultMeta` always contains both a `real` and a
  `decoy` `LaneState`. A technically literate adversary who inspects the file *knows a second lane exists*
  even before any PIN is given, and can demand a second PIN. Plausible deniability here is **social/UX-level,
  not cryptographic** — it does not resist an adversary who knows the scheme. (Contrast: hidden-volume
  designs like VeraCrypt aim to make the second volume indistinguishable from free space; this design does
  not.) This is a real limitation Timothy should decide how far to close (§6, *Decoy semantics*).
- **Decoy plausibility depends on content.** An empty decoy lane is not convincing. Nothing currently
  *encourages* or seeds decoy writes, so a decoy opened for the first time under duress looks empty.
- **Timing side-channel (minor).** A real PIN costs one PBKDF2 derivation; a decoy PIN costs two (real is
  tried first, then decoy) — see `open_lane`. At 310k iterations the ~2× difference on a wrong-then-right
  path is observable in principle. Low severity, but worth noting for a coercion-aware threat model.

### C. Local malware / unlocked running session — **weak (inherent, not a design flaw)**
While the app is unlocked, plaintext notes and the derived key exist in process memory; malware with code
execution as the user can read them, and can also keylog the PIN. No client-side vault defends against a
compromised host during use — this is a genuine ceiling, not something the vault can close. The design
*limits the window*: keys are re-derived per call and zeroized on drop; there is no persistent decrypted
cache. **Residual:** anything unlocked is exposed for as long as it is unlocked; there is currently no
auto-lock/idle timeout in this module (it is stateless, so any such policy lives in the UI layer).

### D. Backup / sync exfiltration — **strong for the notes, with one caveat**
A backed-up vault file is ciphertext; it reduces to scenario A/F. **Caveat:** if the *keychain* is
included in the same backup (E) and wrapping is on, the pepper travels with the ciphertext and the pepper's
protection is lost — the file+pepper pair is back to PIN-only strength (§3-F). Whether the OS keychain is
included in a given backup is platform- and configuration-dependent.

### E. OS-keychain compromise — **degrades wrapping to PIN-only; unwrapped vaults unaffected**
The pepper is a *second factor*, not the whole key: compromising the keychain gives the adversary the
pepper but they still need the PIN and the vault file. So keychain compromise **removes the wrapping
benefit** but does not by itself open the vault. Unwrapped vaults (the default) never touch the keychain
and are unaffected. **Residual:** the recovery code *is* the pepper in the clear — see §3 recovery custody.

### F. Brute-force of a weak PIN — **this is the weakest link, and it is by design-limitation not bug**
Given the vault file (and, if wrapped, the pepper), an adversary can mount an **offline** guessing attack:
for each candidate PIN, run PBKDF2-310k and test the verifier. Two honest problems:
- **PBKDF2 is not memory-hard.** It is CPU-work-hard only, so GPUs/ASICs/FPGAs parallelise it cheaply.
  310k SHA-256 iterations is a defensible OWASP-tier work factor against CPUs, but against a
  GPU/ASIC farm a **short numeric PIN is not safe** — a 4–6 digit PIN has far too little entropy
  (10^4–10^6) to survive offline attack regardless of the iteration count.
- **The 4-char minimum is very low.** `MIN_PIN_LEN = 4` permits e.g. a 4-digit numeric PIN. Iteration
  count cannot rescue a 4-digit secret from offline brute force.
- **No throttling/lockout exists.** Confirmed: there is no attempt counter, rate limit, or lockout in the
  vault module (grep for `lockout|throttle|attempts` finds nothing). That is *acceptable for offline
  attacks* (an offline attacker ignores app-level lockout anyway) but means there is also no defence
  against on-device online guessing.

The pepper (T1.2) is the current mitigation: with wrapping on and the keychain *not* also captured, the
offline attack is blocked entirely (the attacker lacks the pepper). But wrapping is off by default, so the
**default posture leans on PIN entropy alone** — hence the KDF and PIN-policy decisions in §6 matter.

### G. Multi-device — **works, but recovery-code custody is the sharp edge**
An unwrapped vault is just a file: copy it and the same PIN opens it anywhere. A *wrapped* vault's pepper
is device-local (in that machine's keychain), so a second device must be seeded via `unlock_with_recovery`
with the hex recovery code, which then re-stores the pepper locally. **Residual:** the recovery code is a
32-byte secret equal in power to the pepper; whoever holds it (and the file) holds the wrapping factor.
There is currently **no guidance or mechanism** for where/how the user stores it, and only **one** recovery
code exists per wrapped vault.

### Summary table

| Scenario | Current posture | Principal residual risk |
|---|---|---|
| A At-rest theft | Strong | Decoy existence is visible in the file |
| B Coercion/duress | Partial | Decoy is structurally visible; empty/implausible decoy; ~2× timing tell |
| C Unlocked session / malware | Weak (inherent) | No auto-lock; plaintext+key in memory while unlocked |
| D Backup exfiltration | Strong* | *Unless keychain is co-backed-up with a wrapped vault |
| E Keychain compromise | Degrades to PIN-only | Recovery code = pepper in the clear |
| F Weak-PIN brute force | **Weakest link** | PBKDF2 not memory-hard; 4-char min; no throttle |
| G Multi-device | Works | Recovery-code custody undefined; single code only |

---

## 4. Decisions for Timothy

Each is a genuine choice. A **recommended** option is given as an honest engineering default, but the
decision is Timothy's.

### D1 — Key-derivation function
- **Option A: keep PBKDF2-HMAC-SHA256 @ 310k.** Zero migration, already shipped, OWASP-tier against CPUs.
  Weak against GPU/ASIC offline attack on low-entropy PINs.
- **Option B: move to Argon2id (memory-hard).** Raises GPU/ASIC cost by orders of magnitude for the same
  user-visible latency, directly hardening the §3-F weak link. Cost: a new dependency (`argon2`), a
  parameter choice (memory/time/parallelism — e.g. 64 MiB, t=3, p=1 as a starting point), and a
  **migration path** — either re-derive lanes on next successful unlock (transparent, one-time), or
  version the `VaultMeta` and support both KDFs during a transition. Note the 42 MB `SlgArena` ceiling is
  unrelated (this is desktop-native, not VM), but Argon2 memory cost should still be sized to the target
  device.
- **Recommendation (Timothy's call):** **Option B (Argon2id)** for new vaults, with lazy re-derivation of
  existing ones on unlock. It closes the single most important residual (§3-F) at low UX cost and is the
  standard modern choice. If deferred, D5 (PIN policy) becomes materially more important.

### D2 — Keychain-wrapping default
- **Option A: keep opt-in / off (status quo).** No recovery-loss risk for users who never opt in; but the
  default posture leans on PIN entropy alone (§3-F).
- **Option B: default-on for new vaults.** Every new vault gets the pepper second factor; offline
  brute-force needs the keychain too. Cost: **every user now has a recovery code they can lose**, and a
  keychain reset/OS reinstall locks them out unless they kept it. For a population that may include people
  in crisis, unstable housing, or shared/borrowed devices, silent recovery-code loss could mean
  **permanent loss of their most sensitive records** — a serious dignity/harm consideration.
- **Recommendation (Timothy's call):** **keep opt-in (Option A)**, but pair it with a clear, non-technical
  in-app explanation and a strong nudge to enable it *when the user is in a position to safely store the
  recovery code*. Reassess to default-on only if D1→Argon2id is declined and PIN entropy stays low.

### D3 — Recovery-code UX
- Today: a single hex string returned once from `setup_wrapped`; custody entirely undefined.
- **Options (not mutually exclusive):** (a) present as grouped, human-transcribable chunks or a
  **BIP-39-style word list** rather than raw hex (far less error-prone to write down); (b) offer a
  **printable / QR** backup the user can store physically; (c) allow **multiple recovery codes** (e.g. one
  kept by the user, one by a trusted person) — this requires wrapping each as an independent
  pepper-carrier, a modest extension of the current single-pepper model; (d) an explicit
  "I have safely recorded this" confirmation gate before the vault is usable.
- **Recommendation (Timothy's call):** do **(a) word-list + (d) confirmation gate** first (biggest
  error-reduction for least complexity); offer **(b) QR/printable** as a follow-up; treat **(c) multiple
  codes** as a deliberate feature decision tied to the guardianship/representation work, not a quick win.

### D4 — Decoy-lane semantics
- The core question: how strong should deniability be, and how plausible must the decoy be?
- **Options:** (a) **status quo** — decoy is a real second lane, deniability is social/UX-level, its
  existence is visible in the file; (b) **encourage/seed decoy writes** so a duress unlock shows a
  believable store (e.g. prompt the user to add innocuous decoy notes at setup, or auto-populate plausible
  filler) — improves believability, still structurally visible; (c) **hidden-volume-grade deniability**
  (make the decoy indistinguishable from unused space, à la VeraCrypt) — a large redesign of the on-disk
  format and out of scope for a note vault, with its own well-known failure modes (backups/versioning can
  reveal the hidden volume). Also fix the ~2× timing tell in `open_lane` (§3-B) by always attempting both
  lanes (constant-work) regardless of which matches.
- **Recommendation (Timothy's call):** **(b) encourage decoy writes** + the **constant-work timing fix**,
  and **document plainly** that deniability is UX-level, not cryptographic, so no user over-relies on it in
  a high-threat setting. Full hidden-volume deniability (c) is a separate, explicitly-scoped project if the
  threat model demands it — it should not be implied by the current feature.

### D5 — PIN policy (minimum strength, throttling)
- Today: `MIN_PIN_LEN = 4`, no composition/entropy rule, no lockout.
- **Options:** (a) **raise the minimum** (e.g. ≥6 for numeric, or require a passphrase) and/or add a
  simple entropy/blocklist check (reject `1234`, `0000`, repeats); (b) add **on-device throttling** — an
  attempt counter with exponential backoff — to blunt *online* on-device guessing (it does nothing for
  offline attacks, so it complements but does not replace D1/D2); (c) both.
- **Recommendation (Timothy's call):** **(c)** — raise the minimum to a passphrase-capable policy and add
  weak-PIN rejection, *and* add modest on-device backoff. Note honestly: without a memory-hard KDF (D1) or
  wrapping (D2), *no* PIN policy short of a full passphrase makes a short PIN safe against offline attack.

### D6 — At-rest encryption for the rest of the vault (health journal) vs Sanctuary-only
- Sanctuary is encrypted; the **health journal** (`wellfair::journal`) and the other WellFair domains are
  *not* held in this AEAD vault — they persist as ordinary records under `storage_root`. So a stolen,
  powered-off device protects Sanctuary notes but **not** the health journal or other WellFair data.
- **Options:** (a) **status quo** — only Sanctuary is encrypted at rest; the journal is protected only by
  OS-level disk encryption (if any); (b) **extend at-rest encryption** to the health journal (and
  optionally all WellFair domains) — either reuse the sanctuary_crypto lane primitives keyed by a
  master/journal key, or rely on/require full-disk encryption at the OS level with clear guidance.
- **Recommendation (Timothy's call):** **(b), scoped to the health journal first**, since it is the next
  most sensitive store after Sanctuary. Reusing `sanctuary_crypto`'s zero-heap AEAD keyed by a
  device/journal key (unlocked at app start) is the lower-effort path; documenting a hard dependency on
  OS full-disk encryption is the lowest-effort but weakest path. This is the largest-scope item here and
  can be sequenced after D1–D5.

---

## 5. Consequences

- Adopting **D1 (Argon2id)** and **D5 (PIN policy)** together closes the weakest-link scenario (§3-F) and
  is the highest-value pair; either alone is a partial fix.
- Keeping **D2 opt-in** avoids inflicting recovery-loss on vulnerable users but keeps the default posture
  dependent on PIN entropy — which is precisely why D1/D5 matter if D2 stays off.
- **D4** clarifies (and should not overstate) what deniability the decoy actually provides; the honest
  framing is itself a safety feature for people in genuine danger.
- **D6** is the reminder that "the vault is encrypted" is not the same as "WellFair data is encrypted" —
  the health journal is currently not in the vault.

## 6. What only Timothy can decide (out-of-band inputs)

- The **acceptable recovery-loss risk** for the user population (drives D2, D3) — an ethics/harm judgment
  about people in hardship, not an engineering one.
- The **required strength of deniability** (D4) — depends on the real-world threat WellFair users face,
  which is his to characterise.
- Any **sensitive vocabulary/UX wording** for duress, decoy, and recovery (his to coin — per the standing
  project rule on sensitive terminology).

---

## 7. Status: DRAFT — open decisions checklist

- [ ] **D1 — KDF:** keep PBKDF2-310k, or move to Argon2id (recommended: Argon2id, lazy re-derivation).
- [ ] **D2 — Keychain-wrapping default:** keep opt-in (recommended), or default-on.
- [ ] **D3 — Recovery-code UX:** word-list + confirmation gate (recommended) / QR-printable / multiple codes.
- [ ] **D4 — Decoy semantics:** encourage decoy writes + constant-work timing fix + honest "UX-level"
  framing (recommended); hidden-volume deniability is a separate scoped project.
- [ ] **D5 — PIN policy:** raise minimum + weak-PIN rejection + on-device backoff (recommended).
- [ ] **D6 — At-rest for the rest of the vault:** extend encryption to the health journal first
  (recommended), sequenced after D1–D5.

*Once Timothy records his choices here, this ADR moves from DRAFT to Accepted and the corresponding
implementation tasks can be scheduled.*
