# Qualia Embedded Credential PDF Standard (QECP)

**Status:** Internal draft v0.1
**Date:** 2026-08-18
**Principal / copyright holder:** Timothy Charles Holborn <timothy.holborn@gmail.com>
**Companion documents:**
- `docs/plans/qualia-health-wellbeing-document-credentials-todo-2026-08-18.md` (engine to-do)
- `C:\Projects\NLP\consult\20260818_qualia-health-wellbeing-document-credentials-ui-requirements.md` (UI requirements)

---

## 1. Purpose

This standard defines how a **signed RDF credential** is embedded inside a **PDF document** so that a parser can extract the structured credential directly — rather than OCR-ing the PDF or parsing unstructured text — while remaining **backward-compatible with existing PDF readers** (a reader that does not understand the embedded credential simply renders the PDF as normal).

The motivating use cases are:

1. A pathology company issues a lab-results PDF that contains a **signed, machine-readable credential** with the structured test values, reference ranges, units, and the issuing clinician's DID. The recipient's parser extracts the credential and stores it as a `ClinicalReport` with `EvidenceType::ClinicianObserved` — no manual re-entry, no OCR, no NLP extraction ambiguity.
2. A government agency issues a welfare-decision letter PDF with an embedded signed credential stating the decision, the program, the reference number, and the effective dates.
3. A university issues a transcript PDF with an embedded signed credential stating the qualifications conferred.
4. A bank issues a statement PDF with an embedded signed credential stating the account-holder's balance and transaction summary.

In every case, the **PDF renders normally** for human reading, and the **embedded credential** is a parallel signed RDF structure that a Qualia-aware parser can extract, verify, and ingest as a first-class record.

---

## 2. Relationship to C2PA Content Credentials

### 2.1 What C2PA already provides

The **C2PA (Coalition for Content Provenance and Authenticity) Content Credentials** specification (current version 2.4) defines:

- A **C2PA Manifest** — a digitally signed, JUMBF-box-embedded structure containing assertions, claims, and a claim signature.
- **Embedding into PDFs** — Appendix A.4 of the C2PA specification defines how a C2PA Manifest Store is embedded into a PDF as a content stream. The PDF remains readable by existing PDF readers; the manifest is opaque to them.
- **Assertions** — arbitrary typed claims about the asset (creation, edits, capture device, bindings to content, and many other subjects).
- **CBOR encoding** inside JUMBF boxes; **COSE signatures** with X.509 certificates.
- A **trust model** based on signed claims and a chain of manifests (each edit adds a new manifest).

The Qualia codebase already has a **C2PA vocabulary** in `crates/qualia-core-db/src/sparql_library/sparql_mm.rs`:

- `c2pa::HAS_CREDENTIAL`, `c2pa::HAS_MANIFEST`, `c2pa::HAS_SIGNATURE`, `c2pa::HAS_PROVENANCE`, `c2pa::HAS_ASSERTION`
- `c2pa::CREATED_AT`, `c2pa::CREATED_BY`, `c2pa::MODIFIED_AT`, `c2pa::MODIFIED_BY`, `c2pa::HAS_TOOL`
- `c2pa::DERIVED_FROM`, `c2pa::COMPONENT_OF`, `c2pa::HAS_COMPONENT`
- `c2pa::IS_VERIFIED`, `c2pa::VERIFICATION_STATUS`, `c2pa::HAS_CERTIFICATE`
- `C2paVerificationStatus` enum: `Unsupported`, `ParsedOnly`, `SignatureVerified`, `TrustChainEvaluated`
- An **honest verification ladder**: field presence alone is never "verified" — the engine must run the crypto path before promoting status.

### 2.2 What C2PA does NOT provide (and where QECP extends it)

C2PA is a **provenance and authenticity** standard — it answers "who created this asset, what edits were made, and is the asset tamper-evident?" It is **not** a **verifiable credential** standard — it does not define how to carry a W3C Verifiable Credential (with subject, issuer, claims, and a separate proof) inside the manifest.

The gaps QECP addresses:

| Gap | C2PA | QECP extension |
|-----|------|----------------|
| Carry a W3C Verifiable Credential as a typed assertion | C2PA assertions are typed but there is no normative assertion type for "this asset carries a W3C VC" | QECP defines a `qualia:credential` assertion type whose value is a CBOR-LD-encoded W3C Verifiable Credential |
| Bind the credential to the PDF's *semantic content* (not just the bytes) | C2PA binds the manifest to the asset bytes (hash) | QECP additionally binds the credential to the PDF's *text-content hash* so a parser can confirm the visible text matches the credential claims |
| Use Qualia's native credential format (NQuin-based, Ed25519 or ML-DSA-65) | C2PA uses COSE with X.509 | QECP defines a `qualia:native_credential` assertion type for the native format, alongside the W3C VC type |
| Use Qualia DIDs (did:q42) as issuer/subject | C2PA uses X.509 DNs | QECP allows did:q42 in the credential subject/issuer; the C2PA claim signature still uses X.509 (the credential inside has its own proof) |
| Extract structured RDF without parsing PDF text | C2PA manifests are extractable but the assertion schema is open | QECP defines the assertion schema so a parser knows exactly where the credential lives |

**In short:** QECP uses C2PA's JUMBF embedding and manifest structure as the **transport**, and defines a **typed assertion** that carries a W3C Verifiable Credential (or a Qualia native credential) as the **payload**. This means:

- A **C2PA-aware tool** (e.g. c2patool, contentcredentials.org/verify) can read the manifest and see the provenance — it will see a `qualia:credential` assertion it may not understand, but the manifest itself is valid C2PA.
- A **QECP-aware parser** can extract the credential assertion, decode the CBOR-LD, and verify the credential's own proof (Ed25519 or ML-DSA-65) independently of the C2PA claim signature.

### 2.3 Decision: extend C2PA, do not invent a parallel format

QECP does **not** invent a new PDF embedding format. It uses C2PA's JUMBF-in-PDF embedding (Appendix A.4) and defines:

1. A **C2PA assertion schema** for carrying a verifiable credential.
2. A **content-binding extension** that binds the credential to the PDF's text content (not just bytes).
3. A **Qualia native credential** assertion type as an alternative to W3C VC.

This means any C2PA-compliant reader can parse the PDF's manifest store, and any QECP-aware reader can additionally extract and verify the credential.

---

## 3. QECP assertion types

### 3.1 `qualia:credential` (W3C Verifiable Credential payload)

**Assertion label:** `qualia:credential`

**Assertion value:** A CBOR-LD-encoded W3C Verifiable Credential (per `identity/credentials/mod.rs` `Credential` type), including:

- `@context`: `["https://www.w3.org/2018/credentials/v1", "https://qualia.ai/contexts/wellfair/v1"]`
- `id`: the credential URI (e.g. `urn:wellfair:clinical_report:<uuid>`)
- `type`: `["VerifiableCredential", "<domain-specific-type>"]` (e.g. `ClinicalReportCredential`, `WelfareDecisionCredential`, `TranscriptCredential`, `BankStatementCredential`)
- `issuer`: the issuer DID (e.g. `did:q42:<hash>`)
- `issuanceDate`: ISO-8601
- `credentialSubject`: the subject DID + the domain-specific claims (e.g. test values, reference ranges, decision details)
- `proof`: a Data Integrity Proof (ML-DSA-65 per `FiduciaryCrypto`, or Ed25519 per `crypto/verifiable_credential.rs`)

**The credential's own proof is independent of the C2PA claim signature.** The C2PA manifest signs the assertion bytes (tamper-evidence for the embedding); the credential proof signs the credential itself (authenticity of the issuer's attestation). A verifier checks both.

### 3.2 `qualia:native_credential` (Qualia native credential payload)

**Assertion label:** `qualia:native_credential`

**Assertion value:** A binary-encoded Qualia native credential (per `crypto/verifiable_credential.rs` `encode_credential`), which is:

- 28-byte header (issuer u64 + subject u64 + issued_at u32 + valid_until u32)
- N × 48-byte NQuin claims
- 64-byte Ed25519 signature

This is the compact native format for engine-internal use. It is smaller than the W3C VC CBOR-LD and fits the zero-heap evaluator constraints.

### 3.3 `qualia:content_binding` (text-content hash binding)

**Assertion label:** `qualia:content_binding`

**Assertion value:** A CBOR map:

| Field | Type | Purpose |
|-------|------|---------|
| `text_hash` | byte string (32) | SHA-256 of the PDF's extracted text content (normalized: whitespace-collapsed, Unicode NFC) |
| `text_hash_algorithm` | text | `"sha-256"` |
| `text_normalization` | text | `"whitespace-collapse-nfc"` |
| `credential_id` | text | The `id` of the credential this binding refers to (links the binding to a `qualia:credential` or `qualia:native_credential` assertion in the same manifest) |

**Purpose:** A parser that extracts the credential can also extract the PDF's text, hash it, and compare. If the hashes match, the visible text is the text the issuer signed over (via the credential's binding to the content). If they differ, the PDF has been edited after the credential was embedded — the credential is still cryptographically valid (the issuer's signature is over the credential, not the text), but the **binding** is broken and the parser must flag this.

### 3.4 `qualia:disclosure_profile` (tiered disclosure hint)

**Assertion label:** `qualia:disclosure_profile`

**Assertion value:** A CBOR map:

| Field | Type | Purpose |
|-------|------|---------|
| `profiles` | array | List of disclosure profiles (see §5) |
| `profiles[].id` | text | Profile id (e.g. `clinician`, `social_worker`, `government`, `self`) |
| `profiles[].fields` | array | List of credentialSubject field names visible at this profile |
| `profiles[].redactions` | array | List of field names redacted at this profile |
| `profiles[].epistemic` | text | Epistemic level for this profile (e.g. `clinician_observed`, `self_reported`, `summary_only`) |

**Purpose:** The issuer can declare which fields are appropriate for which recipient class. This is a **hint**, not a cryptographic enforcement — the actual disclosure is enforced by the holder's presentation logic (see §5). The hint lets a parser pre-filter what to show.

---

## 4. PDF embedding (C2PA Appendix A.4 conformance)

### 4.1 JUMBF box in PDF

The C2PA Manifest Store is embedded into the PDF as a JUMBF content stream per C2PA Appendix A.4. The specifics:

- The manifest store is a JUMBF box hierarchy (`jumd` + `jumb` boxes).
- Inside the PDF, the JUMBF box is placed in a PDF content stream (a stream object in the PDF's cross-reference table).
- The PDF's `/AF` (Associated Files) array references the manifest store with a `/AFRelationship` of `/C2PA` (per C2PA spec).
- Existing PDF readers ignore the JUMBF box; they render the PDF normally.

### 4.2 Manifest store structure

The manifest store contains one or more manifests. For a QECP PDF, the manifest contains:

1. **Standard C2PA assertions** (creation, tool, signer) — per C2PA spec.
2. **`qualia:credential`** (or `qualia:native_credential`) assertion — the credential payload.
3. **`qualia:content_binding`** assertion — the text-content hash binding.
4. **`qualia:disclosure_profile`** assertion (optional) — the tiered disclosure hint.
5. **C2PA claim** — signs the assertion bytes.
6. **C2PA claim signature** — COSE signature with X.509 certificate (per C2PA spec).

### 4.3 Backward compatibility

A PDF reader that does not understand C2PA/JUMBF:

- Renders the PDF normally (the JUMBF box is opaque).
- Does not see the credential.
- Does not break.

A C2PA-aware reader that does not understand QECP assertion types:

- Parses the manifest and sees the `qualia:credential` assertion as an unknown assertion type.
- Reports the manifest as valid C2PA with unknown assertions.
- Does not extract the credential.

A QECP-aware reader:

- Parses the manifest, recognises `qualia:credential` / `qualia:native_credential` / `qualia:content_binding` / `qualia:disclosure_profile`.
- Extracts the credential, verifies its proof, checks the content binding, applies the disclosure profile.

---

## 5. Tiered disclosure

### 5.1 The problem

A lab-results PDF contains exacting information (test values, reference ranges, units, clinician notes). Different recipients need different levels of detail:

| Recipient | Needs |
|-----------|-------|
| The person (self) | Full results + plain-language interpretation |
| A clinician (doctor) | Full results + reference ranges + units + clinician notes |
| A social worker | Summary only (e.g. "iron deficiency detected, follow-up recommended") — no exact values |
| A government agency | Summary only (e.g. "medical condition confirmed") — no exact values, no clinical notes |

### 5.2 The mechanism

The **holder** (the person or their guardian/proxy) controls disclosure. The `qualia:disclosure_profile` assertion in the PDF is an **issuer hint** — it declares which fields are appropriate for which recipient class. The actual disclosure is enforced by:

1. The holder's **presentation logic** (per `wellfare-core/credentials.rs` `build_presentation` — currently plain JSON field selection; future: ZK selective disclosure per `identity/credentials/mod.rs` `SelectiveDisclosure` trait).
2. The **disclosure traceability** layer (per `webizen-desktop/commands/wellfair/disclosure.rs` — every disclosure is recorded with recipient, acting delegate, onward-share, and a tracing fingerprint).
3. The **consent gate** (per the consent credentials in `wellfair-core` and the desktop `consent_creds` command).

### 5.3 ZK disclosure for biometrics and sensitive fields

For biometric-derived claims (e.g. "this person's voiceprint matches the credential subject" or "this person is over 18"), the holder should be able to prove the claim **without revealing the biometric**. This uses the ZK proof system in `crypto/zk_proofs.rs` (Halo2 zk-SNARKs):

- The biometric template is never in the credential.
- The credential asserts "subject's biometric hash = H" where H is a commitment.
- The holder generates a ZK proof that "my biometric, hashed, equals H" without revealing the biometric.
- The verifier checks the proof against the credential's H.

This is the `ZkDisclosure` trait in `identity/credentials/mod.rs` (currently not implemented — see the engine to-do).

---

## 6. Parser behaviour

### 6.1 Detection

A QECP-aware parser, given a PDF:

1. Scans for a JUMBF box (`jumd` signature) in the PDF's streams.
2. If found, parses the C2PA Manifest Store.
3. Checks for `qualia:credential` or `qualia:native_credential` assertions.
4. If found, the PDF is **QECP-enabled** — the parser extracts the credential.
5. If not found, the PDF is **not QECP-enabled** — the parser falls back to NLP extraction (with appropriate honesty labelling).

### 6.2 Extraction and verification

For a QECP-enabled PDF:

1. Extract the `qualia:credential` (or `qualia:native_credential`) assertion → decode CBOR-LD (or native binary).
2. Verify the credential's own proof:
   - W3C VC: `VcRuntime::verify_credential` (ML-DSA-65) or `crypto::verifiable_credential::verify` (Ed25519).
   - Native: `crypto::verifiable_credential::verify` (Ed25519) + `verify_grounded` (reject ungrounded AI issuers).
3. Extract the `qualia:content_binding` assertion → extract the PDF's text content (normalized) → SHA-256 → compare. Flag if mismatch.
4. Extract the `qualia:disclosure_profile` assertion (if present) → apply the appropriate profile for the current recipient/standpoint.
5. Verify the C2PA claim signature (COSE/X.509) — this confirms the manifest itself is tamper-evident. (Requires a C2PA crypto path — see the engine to-do; the existing `C2paVerificationStatus` ladder is honest about this being `ParsedOnly` until the crypto path is implemented.)
6. Ingest the credential as a first-class record:
   - Clinical report → `wellfare-core/clinical.rs` `ClinicalReport` with `ClaimStatus::ClinicianConfirmed` (if the issuer is a pathology authority per `authority_attestation.rs`) and `EvidenceType::ClinicianObserved`.
   - Welfare decision → `wellfare-core/welfare_support.rs` `GovernmentLetter` (if the issuer is a government authority).
   - Transcript → `wellfare-core/credentials.rs` `CredentialRecord`.
   - Bank statement → `wellfare-core/credentials.rs` `CredentialRecord`.

### 6.3 Honesty

The parser must never:

- Present an NLP-extracted value as if it came from a signed credential.
- Present a `ParsedOnly` C2PA manifest as `SignatureVerified`.
- Present a credential whose content binding is broken as if the visible text is confirmed.
- Present a self-reported claim as `ClinicianObserved` — only a credential issued by a pathology/clinical authority (per `authority_attestation.rs` `authority_type::PATHOLOGY`) and verified can be `ClinicianObserved`.

---

## 7. Credential types (domain-specific)

### 7.1 `ClinicalReportCredential`

| Field | Type | Purpose |
|-------|------|---------|
| `subject` | DID | The patient |
| `report_type` | text | `pathology` / `imaging` / `discharge` / `referral` (per `ClinicalReportType`) |
| `report_id` | text | The lab/report reference |
| `collected_at` | ISO-8601 | Sample collection date |
| `reported_at` | ISO-8601 | Report issue date |
| `tests` | array | List of test results |
| `tests[].name` | text | Test name (e.g. "Ferritin") |
| `tests[].value` | number | Measured value |
| `tests[].unit` | text | Unit (e.g. "µg/L") |
| `tests[].reference_range_low` | number | Lower bound |
| `tests[].reference_range_high` | number | Upper bound |
| `tests[].flag` | text | `normal` / `low` / `high` / `critical` |
| `clinician_notes` | text | Optional clinician notes |
| `issuing_authority` | object | The `Authority` (per `authority_attestation.rs`) |

### 7.2 `WelfareDecisionCredential`

| Field | Type | Purpose |
|-------|------|---------|
| `subject` | DID | The recipient |
| `program_name` | text | The welfare program |
| `reference` | text | The case/claim reference |
| `decision` | text | `approved` / `rejected` / `suspended` / `ceased` |
| `decision_date` | ISO-8601 | |
| `effective_from` | ISO-8601 | |
| `effective_to` | ISO-8601 | optional |
| `issuing_authority` | object | The `Authority` (government) |

### 7.3 `TranscriptCredential`

| Field | Type | Purpose |
|-------|------|---------|
| `subject` | DID | The student |
| `institution` | text | |
| `qualifications` | array | List of qualifications |
| `qualifications[].name` | text | |
| `qualifications[].awarded_at` | ISO-8601 | |
| `qualifications[].grade` | text | optional |

### 7.4 `BankStatementCredential`

| Field | Type | Purpose |
|-------|------|---------|
| `subject` | DID | The account holder |
| `institution` | text | |
| `account_ref` | text | masked account reference |
| `period_start` | ISO-8601 | |
| `period_end` | ISO-8601 | |
| `opening_balance` | number | |
| `closing_balance` | number | |
| `transaction_count` | number | |
| `transaction_summary` | array | optional aggregated categories |

---

## 8. UI notification requirement

The UI must notify the user whether a PDF is QECP-enabled:

| State | UI badge | Meaning |
|-------|----------|---------|
| QECP-enabled, credential verified | green "Signed credential" | The PDF contains a signed credential that verified; the structured data is authoritative |
| QECP-enabled, credential present but unverified | amber "Credential present, unverified" | The PDF contains a credential but the proof did not verify (or the C2PA manifest is `ParsedOnly`) — treat the structured data as a claim, not confirmed |
| QECP-enabled, content binding broken | red "Content binding broken" | The PDF's text has been edited after the credential was embedded — the visible text may not match the credential |
| Not QECP-enabled | grey "No embedded credential" | The PDF has no embedded credential; any structured data must come from NLP extraction (with appropriate honesty labelling) |

The UI must also show the disclosure profile (if present) and which fields are visible to the current standpoint/recipient.

---

## 9. Open questions for the principal

1. **C2PA vs custom embedding**: this standard extends C2PA (uses JUMBF-in-PDF + a typed assertion). The alternative is a custom PDF attachment (a named embedded file with the credential). C2PA gives us provenance + cross-vendor compatibility + the existing C2PA vocabulary in the codebase; custom is simpler but loses C2PA compatibility. The recommendation is C2PA extension — confirm?
2. **X.509 for C2PA claim signature**: C2PA requires X.509 certificates for the claim signature. Qualia uses DIDs (did:q42) + ML-DSA-65 for credentials. The C2PA claim signature and the credential proof are independent — the C2PA signature proves the manifest is tamper-evident; the credential proof proves the issuer attested the claims. Is it acceptable to require an X.509 certificate for the C2PA layer (separate from the DID-based credential layer)?
3. **Content binding scope**: the `qualia:content_binding` hashes the PDF's *text content*. Should it also hash images (for imaging reports)? Image hashing is heavier and may be redundant with the C2PA asset hash. Recommendation: text-only binding for v0.1; image binding as a future extension.
4. **Disclosure profile enforcement**: the `qualia:disclosure_profile` is an issuer hint. Should it be cryptographically enforced (the issuer signs the profile, and the holder's presentation must conform), or remain a hint with enforcement in the holder's presentation logic? Cryptographic enforcement is stronger but less flexible; hint-only is simpler.
5. **Native credential vs W3C VC**: should both assertion types (`qualia:credential` for W3C VC and `qualia:native_credential` for native) be supported, or should we standardise on one? Supporting both gives engine-internal compactness + external interoperability; one is simpler.

---

## 10. References

| Document | Path |
|----------|------|
| C2PA Technical Specification 2.4 | https://spec.c2pa.org/ |
| C2PA Appendix A.4 (Embedding manifests into PDFs) | C2PA spec Appendix A.4 |
| JUMBF (ISO 19566-5) | JPEG Universal Metadata Box Format |
| W3C Verifiable Credentials Data Model | https://www.w3.org/TR/vc-data-model/ |
| CBOR-LD | https://www.w3.org/community/cbor/ |
| Qualia C2PA vocabulary | `crates/qualia-core-db/src/sparql_library/sparql_mm.rs` |
| Qualia native VC | `crates/qualia-core-db/src/crypto/verifiable_credential.rs` |
| Qualia W3C VC runtime | `crates/qualia-core-db/src/identity/credentials/mod.rs` |
| Qualia authority attestation | `crates/wellfare-core/src/authority_attestation.rs` |
| Qualia clinical reports | `crates/wellfare-core/src/clinical.rs` |
| Qualia disclosure traceability | `crates/webizen-desktop/src/commands/wellfair/disclosure.rs` |
| Engine to-do (companion) | `docs/plans/qualia-health-wellbeing-document-credentials-todo-2026-08-18.md` |
| UI requirements (companion) | `C:\Projects\NLP\consult\20260818_qualia-health-wellbeing-document-credentials-ui-requirements.md` |

---

_End of QECP v0.1._
