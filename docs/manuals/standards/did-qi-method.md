# Qualia Identifier (`did:qi`) DID Method Specification

**Status:** W3C Community Group Report–style draft (internal). Not a W3C Technical Report. Not submitted to a Community Group. Not registered in the [W3C DID Specification Registries](https://www.w3.org/TR/did-spec-registries/).
**Method name:** `qi`
**Prefix:** `did:qi:`
**Date:** 2026-09-10
**Version of this draft:** 0.1.0
**Source decision:** [DID-QI-01](./qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/decisions/did-qi-git-utxo.md)
**Implements:** [W3C DID Core 1.0](https://www.w3.org/TR/did-core/) method requirements for Create, Read, Update, Deactivate, DID documents, resolution metadata, security considerations, and privacy considerations.

This document is the Qualia Identifier method specification for ☉ Human-Centric Internet (HCInet) **instrument** identity (node, organisation, contextual persona). It is not Human-Centered AI (🎯). It is not `did:q42` (Q42 Resource Coordinate). It is not a CSCP relay, not Gate B, and not an IETF Internet-Draft.

---

## 1. Status of this document

This is a **complete DID method specification** in the style of a W3C Community Group Report. An independent reviewer asked “is this a DID method spec?” must answer **yes**: the four DID Core operations are defined here with algorithms, encodings, fail-closed errors, and implementable test vectors. No CRUD operation is left as a stub.

It is **not**:

- registered (`qi` is absent from DID Spec Registries as of this draft);
- implemented as a shipping resolver (see sibling assignment DID-QI-IMPL);
- a claim that CSCP-08 (public MASQUE / live relay URL) or CSCP-12 (datatracker submit) is done;
- authorisation to mint production identifiers or to treat git / UTXO / DNS as a connectivity gate.

SDO path if later submitted: W3C Community Group Report for the method; DID Spec Registries registration is a **separate** step that this draft does not perform.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as described in BCP 14 (RFC 2119, RFC 8174) when, and only when, they appear in all capitals.

---

## 2. Relationship to DID Core

A W3C Decentralized Identifier is `did:<method>:<method-specific-id>`. [DID Core 1.0](https://www.w3.org/TR/did-core/) requires a method to define:

| DID Core operation | This method |
|---|---|
| Create | §11. First signed document; DID derived from the SHA-256 genesis payload digest |
| Read | §12. Resolve from an invitation-scoped git object store, an in-memory store, or caller-supplied UTXO transaction bytes |
| Update | §13. Controller signature; monotonic `qi.generation`; git tag analogue; optional UTXO spend-and-recommit |
| Deactivate | §14. Tombstone generation; empty CSCP services; optional UTXO tombstone; further updates fail closed |

Resolution returns a DID document plus `didDocumentMetadata` and `didResolutionMetadata` (§15). Verification relationships used by this method are `authentication`, `assertionMethod`, and `capabilityInvocation` (§9). Security and privacy considerations are §18 and §19.

QDNF already required this bar before anyone calls a string a DID method ([identifier-resolution.md](./qualia-decentralized-network-fabric/identifier-resolution.md) §3). `did:q42:` does not meet it and remains a Q42 Resource Coordinate.

This method publishes **identity and a DID document**. CSCP MAY put mailbox keys and approved relay **hints** in `service` entries. The method does **not** forward datagrams. Git objects, UTXO commitments, and `did.json` on a hostname do not replace CSCP Gate B.

---

## 3. Nomenclature (☉ centric, not 🎯 centered)

Canonical wording: [human-centric-nomenclature.md](./human-centric-nomenclature.md).

**Centered** is a design methodology: the human is a temporary focal point (User-Centered Design, Human-Centered AI). Visual: 🎯 — the human is the **target**.

**Centric** is structural topology: the human is the permanent nucleus from which data, credentials, and agency radiate. Visual: **☉** (default HCInet mark) — the human is the **core**. Other elements (DID documents, relays, ledgers, git remotes) orbit; they do not replace the principal.

| Term | Expansion | Use in this specification |
|---|---|---|
| Human-Centered AI | 🎯 Spell out | External field. MUST NOT name this method. |
| Human-Centric Internet | ☉ **HCInet** | Structural topology this method serves. First use spelled out. |
| Human-Centric AI Agreement Negotiation Protocol | **HCAI-ANP** only | Ingress-contract draft. Protocol-local acronym. Never shortened to **HCAI** here. |
| Qualia Identifier | `did:qi:` | This DID method. HCInet instrument identity. |
| Q42 Resource Coordinate | `did:q42:` | Storage/VM pointer. Not a DID method. Not this specification. |
| App / RDF `qualia:` | `qualia://`, CURIE, `urn:qualia:` | Local URL scheme and ontology prefix. Not a DID method. |

Do not call `did:qi` “the HCAI DID”. Do not register `did:hcai` or `did:hci` (`hci` is Human-Computer Interaction). Do not put ☉, 🎯, or any orbit/radiation mark in a DID string, method-specific-id, Quin, or wire opcode. IETF drafts stay ASCII; this Qualia manual MAY use ☉ in **prose** only.

A natural person is not a DID. ☉ marks the *role* of the principal in the topology, not a join key. Pairwise invitation identifiers remain available (`did:peer`-class). `did:qi` identifies an **HCInet instrument**, not a universal person key.

HCAI-ANP uses `did:web:<domain>` as a DNS/HTTPS Frontdoor only ([hcai-agreement-negotiation-protocol.md](./hcai-agreement-negotiation-protocol.md) §4). That path requires DNS and Web PKI. Native HCInet instrument identity is `did:qi`. HCAI-ANP MAY list `did:qi` in `alsoKnownAs` once this method exists; it does not own the method name.

Compact display form (optional, UI only): `qi:<id>` MAY expand to `did:qi:<id>`. Software MUST parse the `did:qi:` form. Do not register a competing IANA URI scheme in this draft. Do not teach software that `qualia:alice` is a DID.

---

## 4. Conformance

A conformant **controller** implements Create, Update, and Deactivate as specified, produces QCDE-1 bytes (§8), and never writes a Direct locator into a relay-only document (§10.3).

A conformant **resolver** implements Read, verifies Qi Document Proof v1, compares generations, rejects stale signed generations, rejects `did:q42:` / `did:hcai:` / `did:hci:` / `did:qualia:` as this method, and NEVER treats a public DHT or github.com as the method’s backing store.

A conformant **test harness** (DID-QI-IMPL) MUST pass the four numbered vectors in §20 without network access.

Error tokens used by this method (ASCII, stable):

| Token | Meaning |
|---|---|
| `invalid_did` | Syntax, multibase, or 32-byte length failed |
| `not_found` | No current document in the supplied store |
| `invalid_proof` | Missing, unknown type, or Ed25519 verify failed |
| `stale_generation` | Signed generation is below the store’s current generation |
| `deactivated` | Current document has `qi.deactivated` true; mutation refused |
| `direct_locator_forbidden` | Relay-only document contains an access-network locator |
| `generation_not_monotonic` | Update generation is not exactly previous + 1 |
| `previous_digest_mismatch` | `qi.previousDigest` is not the prior unsigned digest |
| `utxo_commitment_mismatch` | OP_RETURN digest is not the unsigned document digest |
| `unsupported_chain` | `chain_id` is not in the local constitution |
| `document_too_large` | QCDE-1 unsigned document exceeds 8192 octets |
| `buffer_full` | Caller-owned output buffer is too small |

Implementations MUST fail closed on every token above. They MUST NOT return a partial DID document that failed verification.

---

## 5. Method name

The method name is `qi` (lowercase). The DID prefix is `did:qi:`.

Rejected method strings (MUST NOT parse as this method): `qualia`, `q42`, `hcai`, `hci`, `hcinet` (reserved alternative; not specified here), and any string containing non-ASCII marks.

---

## 6. Method syntax

### 6.1 ABNF (RFC 5234)

This grammar is additional to DID Core’s `did` production. The method-specific-id is multibase Bitcoin base58btc (`z` prefix) of a 32-octet SHA-256 digest.

```abnf
did-qi               = "did:qi:" qi-id
qi-id                = "z" 32*48BASE58BTC
                      ; decoded payload MUST be exactly 32 octets
BASE58BTC            = %x31-39 / %x41-48 / %x4A-4E / %x50-5A
                      / %x61-6B / %x6D-7A
                      ; Bitcoin alphabet: 123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz
                      ; excluded: "0" / "I" / "O" / "l"

did-qi-url           = did-qi [ ";" qi-param *( ";" qi-param ) ]
                      [ "/" path-abempty ]
                      [ "?" query ] [ "#" fragment ]
qi-param             = qi-param-name [ "=" qi-param-value ]
qi-param-name        = "versionId" / "qi:backing"
qi-param-value       = 1*param-char
param-char           = ALPHA / DIGIT / "-" / "_" / ":"
path-abempty         = *( "/" segment )
segment              = *pchar
query                = *( pchar / "/" / "?" )
fragment             = *( pchar / "/" / "?" )
pchar                = unreserved / pct-encoded / sub-delims / ":" / "@"
unreserved           = ALPHA / DIGIT / "-" / "." / "_" / "~"
pct-encoded          = "%" HEXDIG HEXDIG
sub-delims           = "!" / "$" / "&" / "'" / "(" / ")"
                      / "*" / "+" / "," / ";" / "="
```

`ALPHA`, `DIGIT`, and `HEXDIG` are from RFC 5234. `versionId` is the decimal `qi.generation` (unsigned 32-bit). `qi:backing` is a resolver hint: `memory` / `git` / `utxo`. Hints are not security authority.

DID URLs with a fragment (`#key-0`, `#mailbox`, `#frontdoor`) identify verification methods and services inside the document. Fragments are not hashed into the method-specific-id.

### 6.2 Normalization and case

1. Scheme `did` and method `qi` MUST be lowercase. A resolver that sees `DID:QI:` MAY ASCII-lowercase those two labels and MUST then apply the remaining rules. Mixed-case `qi-id` MUST NOT be case-folded: base58btc is case-sensitive.
2. Decode `qi-id` without the leading `z` using Bitcoin base58btc. The payload MUST be exactly 32 octets. Shorter or longer payloads are `invalid_did`.
3. Re-encode those 32 octets as `z` || base58btc. The result MUST equal the input `qi-id` octet-for-octet (canonical multibase; reject extra leading `1` padding and non-canonical encodings).
4. The 32 octets ARE the genesis payload digest (§11). They are not a truncated hash, not `q_hash`, and not a QRC.

Hostname MUST NOT appear in `did-qi`. Forms such as `did:qi:example.org` are `invalid_did`.

### 6.3 Size bounds

| Item | Bound |
|---|---|
| DID string (`did:qi:` + `qi-id`) | 8 + 1 + 32..48 = at most 57 ASCII octets; typical test vector is 52 |
| Unsigned QCDE-1 document | 8192 octets |
| Signed QCDE-1 document (with `proof`) | 9216 octets |
| Verification methods | 4 |
| Services | 8 |
| `alsoKnownAs` entries | 4 |
| Relay hints per mailbox | 8 |
| `hintId` | 1..64 ASCII (`ALPHA` / `DIGIT` / `-`) |

---

## 7. Identifier derivation

```
genesisPayload     = QCDE-1(genesis object)          ; §8, §11.1
genesisDigest      = SHA-256(genesisPayload)         ; 32 octets
qi-id              = "z" || base58btc(genesisDigest)
did                = "did:qi:" || qi-id
```

SHA-256 is FIPS 180-4. Multibase is [multiformats/multibase](https://github.com/multiformats/multibase) with Bitcoin base58btc, prefix `z`. This is the same alphabet as Bitcoin addresses, not base58check (no version byte, no 4-byte checksum). Integrity is the SHA-256 digest itself plus the controller signature on published documents.

`q_hash` (60-bit FNV-1a) MAY index local tables. It MUST NOT decide cryptographic equality of two `did:qi` values, key authorisation, content integrity, or route ownership ([identifier-resolution.md](./qualia-decentralized-network-fabric/identifier-resolution.md) §2).

---

## 8. Canonical encoding (QCDE-1)

**QCDE-1** (Qualia Canonical Document Encoding, version 1) is a restricted profile of RFC 8785 JSON Canonicalization Scheme. Implementations MAY use a full RFC 8785 library if and only if the output matches this profile on the documents this method allows.

Allowed JSON values: objects, arrays, UTF-8 strings, integers in the inclusive range 0..2^53-1, booleans, and `null`. **Floats are forbidden.** Documents that need a time use `qi.createdUnix` as an integer Unix seconds value.

Rules:

1. No insignificant whitespace (no U+0020 / U+000A / U+000D / U+0009 outside strings).
2. Object members emitted in lexicographic order of keys by UTF-8 octet comparison. For the ASCII keys this method defines, that is ordinary ASCII sort (`@context` before `assertionMethod` because `@` is 0x40 and `a` is 0x61).
3. Arrays preserve author order.
4. Strings: wrap in U+0022; escape U+0022 and U+005C; encode U+0000..U+001F as `\u00xx` lowercase hex. This method’s test vectors contain no other escapes.
5. Integers: base-10, no leading zeros (except the number `0`), no plus sign, no exponent.
6. Booleans: `true` / `false`. Null: `null`.
7. UTF-8 encoding of the resulting Unicode text, no BOM.

The **unsigned document** is the DID document object **without** the `proof` member. The **signed document** is the unsigned document plus `proof`.

```
unsignedDigest = SHA-256(QCDE-1(unsigned document))
```

Git objects and UTXO commitments in this method use these digests as specified in §16 and §17. Pretty-printed JSON is **not** hashed.

---

## 9. DID document

### 9.1 JSON-LD `@context`

A published document MUST include `@context` as an array whose first element is `https://www.w3.org/ns/did/v1`. This method’s documents also include:

- `https://w3id.org/security/suites/ed25519-2020/v1` — `Ed25519VerificationKey2020`
- `https://webizen.network/ns/did-qi/v1` — `qi`, `CscpMailbox`, `HostnameAlias`, `QiDocumentSignature2026`

The third IRI is a stable name for this draft’s JSON-LD terms. The vocabulary document need not be fetched to verify proofs; verification uses QCDE-1 and Ed25519, not remote context expansion. Resolvers MUST NOT dereference `@context` URLs as a Read dependency (offline test vectors).

### 9.2 Required properties

| Property | Rule |
|---|---|
| `id` | Exactly the `did:qi:` string derived in §7. MUST match the store key. |
| `controller` | The same DID (self-controlled instrument). Controller rotation is an Update that keeps `id` and changes `verificationMethod`. |
| `verificationMethod` | At least one `Ed25519VerificationKey2020` with `publicKeyMultibase` (`z` \|\| base58btc(`0xed 0x01` \|\| 32-octet public key)). `id` is a DID URL fragment on this DID. `controller` is this DID. |
| `authentication` | DID URL(s) of keys allowed to authenticate as the instrument. |
| `assertionMethod` | DID URL(s) of keys allowed to assert credentials as the instrument. |
| `capabilityInvocation` | DID URL(s) of keys allowed to Update/Deactivate this document and to invoke QDNF route-update capability ([identifier-resolution.md](./qualia-decentralized-network-fabric/identifier-resolution.md) §2 item 4). A session `authentication` key MUST NOT Update the document unless it is also listed here. |
| `service` | Array, possibly empty. Types defined in §10. |
| `qi` | Object: `generation` (uint32), `deactivated` (boolean), `createdUnix` (uint32 Unix seconds of genesis, immutable), optional `previousDigest` (multibase SHA-256 of the prior unsigned document), optional `disclosure` duplicate of mailbox disclosure for document-level policy. |
| `proof` | Qi Document Proof v1 (§9.4). |

`alsoKnownAs` is OPTIONAL. It MAY contain `did:web:<domain>` Frontdoor aliases and other DID URLs. A hostname belongs here or in `HostnameAlias` — **never** in `id`. Rotating DNS, GitHub Pages, or a registrar MUST NOT change the DID.

Relative verification-method ids (`#key-0`) appear only in the **genesis payload**. Published documents MUST use absolute DID URLs.

### 9.3 `qi` extension object

```json
{
  "createdUnix": 1788998400,
  "deactivated": false,
  "generation": 0,
  "previousDigest": "z..."
}
```

- `generation` 0 is Create. Each Update or Deactivate MUST set `generation` to exactly previous + 1. The value MUST fit in an unsigned 32-bit integer (CSCP `ContactDescriptor.generation` is u32). Mapping: `ContactDescriptor.generation` == `qi.generation` == git annotated-tag analogue `qi/<generation>`.
- `previousDigest` MUST be omitted at generation 0 and MUST be present and equal to the prior generation’s `unsignedDigest` (multibase-z) at generation ≥ 1.
- `createdUnix` is copied unchanged from genesis through all generations (document birth time, not update time).
- `deactivated` false until Deactivate; then true forever for this DID.

### 9.4 Qi Document Proof v1

This method does **not** use RDF Dataset Canonicalization (URDNA2015) and does **not** claim conformance to W3C Data Integrity `eddsa-rdfc-2022` or `Ed25519Signature2020` as a Linked Data proof suite. Keys are still `Ed25519VerificationKey2020` so the public-key encoding matches HCAI-ANP Frontdoor documents.

Proof type name: `QiDocumentSignature2026`.

```
message = "did:qi:document:v1" || 0x00 || unsignedDigest
proofValue_raw = Ed25519Sign(controller_sk, message)     ; 64 octets, RFC 8032
proof.proofValue = "z" || base58btc(proofValue_raw)
```

`proof` object (keys shown in QCDE-1 order):

| Key | Value |
|---|---|
| `created` | XML Schema `dateTime` in UTC, second precision, `Z` suffix. Informative; the signed time is `qi.createdUnix` plus `qi.generation`. |
| `proofPurpose` | `capabilityInvocation` for Create/Update/Deactivate |
| `proofValue` | multibase-z of the 64-octet signature |
| `type` | `QiDocumentSignature2026` |
| `verificationMethod` | DID URL of the `capabilityInvocation` key used |

Verify: drop `proof`; QCDE-1; SHA-256; rebuild `message`; Ed25519 verify against the referenced public key. Failure is `invalid_proof`.

---

## 10. Services

### 10.1 `CscpMailbox`

Service `type` MUST be `CscpMailbox`. This is a CSCP mailbox / relay **hint**, not a datagram forwarder and not a public DHT.

`serviceEndpoint` is a JSON object (not a URL string) with:

| Field | Type | Rule |
|---|---|---|
| `contactKeyMultibase` | `z` \|\| base58btc(32 octets) | CSCP ContactDescriptor contact key |
| `disclosure` | string | `DirectPermitted` / `ApprovedRelaysOnly` / `QualifiedMultiHop` / `Isolated` (CSCP disclosure 1..4) |
| `generation` | uint32 | MUST equal `qi.generation` |
| `locatorKind` | string | `mailbox` (CSCP kind 1) or `relayHint` (kind 2). `direct` (kind 3) is §10.3 |
| `publicDht` | boolean | MUST be `false` unless disclosure is `DirectPermitted` **and** the local constitution opts in. Default false. |
| `relayHints` | array | Opaque invitation-scoped hints. Each item: `hintId` (token) and `operatorHashHex` (64 lowercase hex chars, SHA-256 of an operator identifier). MUST NOT contain IP addresses, hostnames, ports, or URLs |

No `serviceEndpoint` string URL is used for `CscpMailbox`. There is no `wss://`, `quic://`, `ip://`, or `/dns4/` multiaddr in this service type.

### 10.2 `HostnameAlias` (Frontdoor, not the DID)

Service `type` `HostnameAlias` MAY bind a DNS/HTTPS Frontdoor without making the hostname the identifier:

```json
{
  "id": "did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#frontdoor",
  "type": "HostnameAlias",
  "serviceEndpoint": {
    "didWeb": "did:web:example.invalid"
  }
}
```

`didWeb` MUST be a `did:web:` DID. The corresponding `did.json` is HCAI-ANP LIG compatibility. Resolving `did:web` is not resolving `did:qi`. Rotating the hostname MUST be an Update of this service / `alsoKnownAs` only.

### 10.3 Relay-only forbids Direct locators

A document is **relay-only** when any `CscpMailbox` `disclosure` is `ApprovedRelaysOnly`, `QualifiedMultiHop`, or `Isolated` (CSCP values 2, 3, 4). Isolated additionally forbids live-network path classes in CSCP; the DID document still MUST NOT export access-network locators.

When the document is relay-only, Create and Update MUST fail closed with `direct_locator_forbidden` if any of the following hold:

1. Any service has `locatorKind` equal to `direct` (CSCP locator kind 3).
2. Any `serviceEndpoint` object contains a forbidden locator field name: `ipv4`, `ipv6`, `ip`, `host`, `hostname`, `port`, `socketAddress`, `socket`, `addr`, `multiaddr`, `peerAddr`, `directLocator`, `wss`, `quic`, `udp`, `tcp`.
3. Any string value in `service` matches an IPv4 dotted-quad, an IPv6 literal (including `2001:db8:` documentation addresses), or a `host:port` pair.
4. `publicDht` is true.

Allowed under relay-only: `locatorKind` `mailbox` or `relayHint`; `relayHints` of operator hashes and invitation tokens; `HostnameAlias` / `alsoKnownAs` `did:web:` (DNS Frontdoor is not a CSCP Direct locator and MUST NOT be probed as one).

A resolver that is handed a relay-only document containing a Direct locator MUST NOT treat it as current. It MUST return `direct_locator_forbidden` and MUST NOT copy the locator into CSCP ContactDescriptor TLVs.

This rule is the document-layer counterpart of CSCP “Direct locators MUST NOT be probed when disclosure forbids peer-IP disclosure.” Encoding a locator in the DID document **is** an export.

---

## 11. Create

### 11.1 Genesis payload

The controller constructs a genesis object that **omits** `id`, `controller`, `proof`, `qi`, and `alsoKnownAs`. Verification method and service ids are relative fragments. `verificationMethod` entries omit `controller` (filled after DID assignment).

Algorithm:

1. Build the genesis object. If it is relay-only, apply §10.3 **before** hashing.
2. `genesisPayload = QCDE-1(genesis)`. If larger than 8192 octets, fail `document_too_large`.
3. `genesisDigest = SHA-256(genesisPayload)`.
4. `did = "did:qi:" || "z" || base58btc(genesisDigest)`.
5. Expand relative ids to absolute DID URLs; set `id` and `controller` to `did`; set `qi.generation = 0`, `qi.deactivated = false`, `qi.createdUnix =` now (test vectors freeze 1788998400); omit `previousDigest`.
6. Sign the unsigned published document (§9.4) with a `capabilityInvocation` key from the genesis verification methods.
7. Optionally write a git blob (§16) and/or a UTXO commitment of `unsignedDigest` (§17).
8. Store the signed document under key `genesisDigest` at generation 0.

The DID is self-certifying: it is a content digest of the genesis payload. Changing initial keys or the initial mailbox configuration changes the DID. Later Updates do **not** change the DID.

### 11.2 Optional git backing at Create

If git backing is used, store QCDE-1(signed document) as a git blob (§16) and record annotated-tag analogue `qi/0`. Absence of git does not make Create fail; the in-memory store is sufficient.

### 11.3 Optional UTXO attestation at Create

If UTXO attestation is used, create an output whose script commits `unsignedDigest` under the Bitcoin-family profile (§17). The method-specific-id does **not** embed a chain id or ticker. Several chains MAY attest the same digest. Resolvers accept a chain only if the local constitution lists that `chain_id`.

Create MUST succeed without any UTXO. Ledger attestation is optional public evidence, not the identifier.

---

## 12. Read

Input: `did:qi:` string, a `QiStore` (memory and/or invitation-scoped git), and optionally caller-supplied UTXO transaction bytes. Output: current unsigned-or-signed document, generation, and resolution metadata. No network is required.

Algorithm:

1. Parse and canonicalize the DID (§6.2). Foreign methods (`did:q42:`, `did:web:`, `did:hcai:`, …) return `invalid_did` for **this** method (a multi-method resolver may dispatch elsewhere; this method MUST NOT interpret them).
2. Look up `genesisDigest` in the in-memory store. If present, take the highest generation whose proof verifies.
3. If missing, look up git objects in the **invitation-scoped** store: blob by recorded object id, or tag analogue `qi/<n>` for the largest n. Verify the blob bytes are QCDE-1 of a signed document whose `id` matches and whose proof verifies. Git fetch from a public DHT or from github.com as operator is out of scope and MUST NOT be performed by a conformant Read of this method.
4. If UTXO bytes are supplied, parse the Bitcoin-family transaction (§17), extract the commitment, and match `unsignedDigest`. A matching live commitment corroborates; a tombstone corroborates Deactivate. UTXO bytes are **caller-supplied**; the resolver MUST NOT query Chronik, Electrum, or any chain RPC.
5. If nothing verifies, `not_found`.
6. If the current document has `qi.deactivated` true, return the tombstone document with `didDocumentMetadata.deactivated = true`. That is a successful Read of a deactivated DID, not `not_found`.
7. A verified signature on generation *n* when the store’s current generation is *m* > *n* is `stale_generation`. Validity of an old signature does not make it current (same rule as CSCP ContactDescriptor).
8. Relay-only Direct locators: `direct_locator_forbidden` (§10.3).
9. Return the document plus metadata (§15).

Read is **not** a public DHT. Default CSCP discovery stays invitation-scoped. `public_dht` remains off unless Direct is permitted.

---

## 13. Update

Algorithm:

1. Read the current document. If `not_found`, fail. If `qi.deactivated`, fail `deactivated`.
2. Caller supplies a new unsigned document with the **same** `id` and `controller`, `qi.generation = current + 1`, `qi.previousDigest =` multibase-z of the current `unsignedDigest`, and `qi.createdUnix` unchanged.
3. Verify `generation` is exactly current + 1 (`generation_not_monotonic` otherwise) and `previousDigest` matches (`previous_digest_mismatch` otherwise).
4. Apply §10.3 if relay-only. Hostname changes belong in `alsoKnownAs` / `HostnameAlias` only.
5. Sign with a key in the **current** document’s `capabilityInvocation` list (the pre-update document). Key rotation: the new document MAY replace `verificationMethod`; the signature on this Update MUST still verify under the old capabilityInvocation key. A subsequent Update uses the new key.
6. Store as current. Map git annotated-tag analogue to `qi/<new-generation>`. An older commit or tag is stale, not current.
7. If UTXO attestation is in use, spend the previous commitment outpoint and create a new commitment of the new `unsignedDigest` (the `did:btcr` continuation pattern, parameterized by CAIP-2; §17).

CSCP mapping: `ContactDescriptor.generation` MUST equal `qi.generation` after a successful Update. Receivers that see a descriptor with a smaller generation MUST treat it as stale even if a signature on that descriptor verifies.

---

## 14. Deactivate

Algorithm:

1. Read current. If already deactivated, return the current tombstone (idempotent Read-side); a second Deactivate operation MUST fail `deactivated` (no generation bump after death).
2. Build a tombstone unsigned document: same `id`; `qi.generation = current + 1`; `qi.deactivated = true`; `qi.previousDigest` set; `service` MUST be the empty array (no mailbox, no Direct, no relay hints, no HostnameAlias locators); `verificationMethod` SHOULD retain the deactivating key so the tombstone remains verifiable; `authentication` / `assertionMethod` / `capabilityInvocation` MAY continue to reference that key for historical verification only.
3. Sign with current `capabilityInvocation`.
4. Store. Further Update MUST fail `deactivated`.
5. Git: annotated-tag analogue `qi/<generation>` on the tombstone blob. Higher tags MUST NOT be written.
6. UTXO: spend the live commitment to a **registered tombstone** output (§17.3). After a verified tombstone outpoint, live commitments for this DID on that chain are stale.

Deactivate is permanent for this method-specific-id. Recovery of control after key loss is out of band (human-governed recovery quorum) and would mint a **new** `did:qi:` if a new genesis payload is hashed — it cannot resurrect a deactivated id.

---

## 15. Resolution metadata

A successful Read returns the DID Core resolution structure:

```json
{
  "didDocument": { "...signed or unsigned document..." },
  "didDocumentMetadata": {
    "created": "2026-09-10T00:00:00Z",
    "updated": "2026-09-10T00:00:00Z",
    "deactivated": false,
    "versionId": "0",
    "qiBacking": "memory",
    "qiGeneration": 0,
    "qiDocumentDigest": "z8kwJYVreKg6qPC6zRhC5z7jZ5KiU6AqvRsgVQVAaYToh",
    "qiGenesisDigest": "zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu",
    "qiGitObjectId": "18962a639bdbe09b3998105716944d34b9da606d2d945701e6d68eff90aacddd",
    "qiAttestations": [
      {
        "chainId": "bip122:000000000019d6689c085ae165831e93",
        "txid": "e9dfb2471de55f2d1702a5dd1c560a28c4a632640d5f8d0648613b69f6a5a176",
        "vout": 0,
        "commitment": "z8kwJYVreKg6qPC6zRhC5z7jZ5KiU6AqvRsgVQVAaYToh",
        "kind": "live"
      }
    ]
  },
  "didResolutionMetadata": {
    "contentType": "application/did+ld+json",
    "error": null
  }
}
```

Rules:

- `qiBacking` is `memory`, `git`, `utxo`, or a comma-free single token naming the backing that supplied the **chosen** current document. Additional attestations are listed, not mixed into the backing name.
- `qiGeneration` equals `qi.generation`.
- `qiDocumentDigest` is multibase-z of `unsignedDigest`.
- `qiGenesisDigest` is the method-specific-id (multibase-z of the genesis payload digest). It never changes.
- `qiGitObjectId` is 64 lowercase hex SHA-256 of the git object (§16), or omitted if git was not used.
- `qiAttestations` MAY be empty. Every listed attestation MUST commit the same `qiDocumentDigest` (or a tombstone, when `kind` is `tombstone`).
- `didResolutionMetadata.error` uses the tokens in §4 on failure; `didDocument` MUST then be omitted or null.

`updated` is informational. Generation, not wall-clock, is the conflict rule.

---

## 16. Git protocol backing (not GitHub)

### 16.1 What is stored

```
signed_bytes = QCDE-1(signed document)
header       = "blob " || decimal_ascii(len(signed_bytes)) || 0x00
git_object   = header || signed_bytes
qiGitObjectId = SHA-256(git_object)            ; 32 octets, hex for metadata
```

This is the classical git object header with **SHA-256** instead of SHA-1. It is the in-process / invitation-scoped store format for this method (Qi Git Object Id v1). It is not a requirement to run `git-daemon`. It is not byte-identical to `git hash-object` on a SHA-1 repository.

Generation analogue: an annotated tag name `qi/<generation>` (decimal, no leading zeros) whose object is that blob (or a commit whose tree contains a single `did.json` blob of those bytes). DID-QI-IMPL MAY store only `(generation, object_id, blob)` in a fixed-slot array and still be conformant.

`ContactDescriptor.generation` maps to that generation. An older commit is stale, not current.

### 16.2 Invitation-scoped remotes

Resolution MAY `git fetch` or consume a `.bundle` from a remote **named in the invitation** that established the relationship. Isolated receipts MAY move a `.bundle` on sneakernet or IPFS. Those transports carry **bytes**. Authority remains the Ed25519 proof.

A reachable git daemon or Radicle seed replicates objects. It is not a CSCP relay and not Gate B.

### 16.3 `git://` is unauthenticated

The `git://` protocol has no transport authentication. It MUST NOT be this method’s security profile. Prefer git+ssh, authenticated smart HTTP, or a `.bundle`. Objects received over `git://` MUST still verify Qi Document Proof v1 and generation; they MUST NOT be trusted because they arrived from a daemon.

### 16.4 GitHub-as-operator is forbidden

github.com MUST NOT be the method’s backing operator of record. A conformant resolver MUST NOT clone `https://github.com/…` as DID Read. A GitHub Pages hostname MAY appear only as `did:web:` / `HostnameAlias` (the same class as any DNS Frontdoor). Historical `did:git` drafts that treated GitHub as the method are not this specification.

Do not take a ticker, a forge, or a CDN as the identifier.

---

## 17. Parameterized UTXO attestation (CAIP-2 `bip122`, not one ticker)

### 17.1 Why not `did:btc` / `did:xec` / `did:bch`

A ticker in the method name makes chain migration a new identity. This method’s id is the genesis document digest. Each attestation names:

```
chain_id    = CAIP-2  (Bitcoin family: bip122:<32-hex-chars>)
outpoint    = txid (Bitcoin RPC display hex, 64 lowercase chars)  ||  vout (uint32)
commitment  = unsignedDigest of the canonical DID document
```

`bip122` uses the first 16 bytes (32 hex characters) of a block hash that **uniquely identifies** the chain ([CAIP-2](https://github.com/ChainAgnostic/CAIPs/blob/main/CAIPs/caip-2.md)). Bitcoin mainnet’s conventional value is:

```
bip122:000000000019d6689c085ae165831e93
```

Bitcoin testnet3 (distinct genesis) is an example of parameterization without renaming the DID:

```
bip122:000000000933ea01ad0ee984209779ba
```

Same-genesis forks (Bitcoin Cash, eCash / XEC, and others that share Bitcoin’s genesis) MUST NOT reuse Bitcoin mainnet’s `chain_id` if the constitution means a different ledger. They MUST use a uniquely identifying later block hash in the `bip122` namespace, as CAIP-2 allows, **not** a ticker in the DID. Payment UTXOs already in-tree for eCash MAY fund a CSCP `RelayLease`; they are not the DID.

Account-based ledgers (EVM and similar) are a **different** attestation profile and MUST NOT be smuggled under this UTXO profile. Cardano-style eUTXO is not Bitcoin script; it needs its own profile or it stays out.

Resolvers accept a `chain_id` only if the local constitution lists it; otherwise `unsupported_chain`. Several listed chains MAY attest the same digest.

### 17.2 Bitcoin-family commitment profile v1

Magic ASCII `QI` (0x51 0x49). Script on a zero-value output:

```
OP_RETURN  <35-byte push: 0x51 0x49 0x01 || unsignedDigest>
```

- Opcode `0x6a`, push length `0x23` (35), then `5149 01` and 32 digest octets.
- `kind` = `live`.
- The committed digest is `unsignedDigest` of the generation being attested (not the genesis digest, unless they coincide — they do not, because the published document includes `id`).

Transaction parsing for this profile (caller-supplied raw bytes, no witness required in test vectors):

1. Version uint32 little-endian.
2. Input count as Bitcoin CompactSize. Test vectors use 1.
3. For each input: 32-octet prevout txid **wire order**, uint32 le vout, CompactSize script, script, uint32 le sequence.
4. Output count CompactSize.
5. For each output: uint64 le value, CompactSize script, script. A script matching `6a 23 51 49 01` || 32 bytes is a live commitment; `6a 23 51 49 ff` || 32 zero bytes is a tombstone.
6. Locktime uint32 le.

`txid` in metadata is Bitcoin **RPC display order**: SHA-256d of the raw transaction (no witness), then byte-reverse to hex. Test vectors give both display and wire hex. Implementations that extract OP_RETURN from supplied bytes MUST NOT fetch the transaction from the network.

Endianness: prevout txids inside the raw transaction are wire order (little-endian display). Metadata `txid` is display order. Do not mix them.

### 17.3 Update = spend-and-recommit; Deactivate = tombstone

- **Update:** spend the previous live outpoint as an input; create a new live OP_RETURN for the new `unsignedDigest`. This is the `did:btcr` continuation pattern with `chain_id` as a parameter.
- **Deactivate:** spend the live outpoint; create a tombstone:

```
OP_RETURN  <35-byte push: 0x51 0x49 0xFF || 32 zero octets>
```

`kind` = `tombstone`. After a verified tombstone, further live commitments for this DID on that chain MUST be ignored by resolvers (fail closed toward deactivated).

A constitution MAY require UTXO attestation. This specification does not.

---

## 18. Security considerations

1. **Controller proof is Ed25519 over QCDE-1.** Transport (git, sneakernet, a bundle, an unauthenticated daemon) is not a substitute. `git://` is unauthenticated (§16.3).
2. **GitHub-as-operator is forbidden** (§16.4). A forge outage or account takeover MUST NOT change or seize a `did:qi:`.
3. **Stale signed generation.** An attacker who caches a valid generation-*n* document cannot make it current after generation *n+1* exists. Resolvers MUST compare `qi.generation` to the store’s current value. CSCP ContactDescriptor uses the same number.
4. **Hostname rotation MUST NOT change the DID.** Binding identity to ICANN + HTTPS (`did:qi:example.org` with did:web resolution rules) is forbidden. Frontdoor compromise leaks a discovery alias, not the identifier.
5. **`q_hash` 60-bit is not cryptographic equality.** Do not truncate `genesisDigest` to 60 bits for authorisation. QRC (`did:q42:`) uses FNV-1a and is not collision-resistant; this method MUST NOT fall back to it for signature checks (HCAI-ANP already warns on that collision for Frontdoor keys).
6. **Capability vs authentication.** Route updates and document Updates require `capabilityInvocation`. A stolen session authentication key does not by itself rewrite the DID document.
7. **UTXO reorgs and constitution.** A short chain reorg can hide an attestation. Resolvers that use UTXO as corroboration SHOULD treat it as evidence, not as the only Read path. `unsupported_chain` fails closed. Same-genesis forks must not be confused (§17.1).
8. **No public DHT of protected persons.** Publishing a `did:qi` document to a global index is a disclosure choice and MUST NOT be the default for clinical or otherwise protected roles.
9. **Proof type confusion.** Verifiers MUST require `QiDocumentSignature2026` and the domain separator `did:qi:document:v1`. A signature over raw JSON, over URDNA2015, or over a different domain is `invalid_proof`.
10. **This method is not Gate B and not a relay.** A valid DID document does not admit a CSCP path, does not punch NAT, and does not replace a RelayLease. Git seeds and UTXO miners are not CSCP operators.
11. **Key compromise.** Rotate via Update (old capabilityInvocation signs the new methods). Compromise of the only capabilityInvocation key without a recovery path is loss of Update; Deactivate cannot be issued. Do not put recovery shares in the DID document.
12. **Equivocation.** Two valid different documents for the same DID and generation are a conflict. Neither arrival order nor last-write-wins may pick a winner. Store both in an isolated paraconsistent context and wait for a higher authorized generation or a human decision (QDNF RAR equivocation rule applied to documents).

---

## 19. Privacy considerations

1. **Pairwise vs public.** `did:qi` is instrument identity and MAY be published. Relationship-specific work SHOULD still use pairwise (`did:peer`-class) identifiers so a public instrument DID does not become a join key across invitations.
2. **Classified data MUST NOT appear in the DID document.** No graph contents, no clinical observations, no natural-person attributes, no `webizen:SensitivityLabel` classified payload. Service objects are hints, not a data API. Sensitivity class of the instrument’s **graph** is a Quin concern; the DID document is at most public-equivalent metadata.
3. **Invitation-scoped remotes.** Git remotes used for Read are those the invitation named. Do not scrape public forges to discover protected instruments.
4. **Relay-only documents export no access-network locators** (§10.3). A mailbox contact key and operator hash are still correlatable identifiers; they are not IP addresses. Relay-only is not anonymity (CSCP says this explicitly).
5. **UTXO privacy.** A public-chain OP_RETURN links `unsignedDigest` to a txid. That is intentional public attestation. Controllers who need no public ledger MUST omit UTXO backing. Do not reuse payment addresses as DID commitment outpoints if that would cluster financial activity with instrument identity.
6. **Frontdoor logs.** Resolving `did:web` `did.json` reveals that *a* resolution occurred. That is HCAI-ANP’s discovery path, not this method’s Read. Native Read of `did:qi` from a local store leaves no DNS trace.
7. **`alsoKnownAs` clustering.** Listing many aliases in one document lets observers merge contexts. Keep contextual personas on **separate** `did:qi:` genesis payloads.

---

## 20. Test vectors

All vectors are offline. DID-QI-IMPL MUST implement them without network, Chronik, git-daemon, or github.com. Controller secret is RFC 8032 Ed25519 test vector 1 (this is a **test-only** key; MUST NOT be used in production):

```
secret  9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60
public  d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a
publicKeyMultibase  z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw
```

`qi.createdUnix` is frozen at `1788998400` (`2026-09-10T00:00:00Z`). Contact key = SHA-256(`did:qi:test-vector:contact-key-0`). Operator hash = SHA-256(`did:qi:test-vector:operator-0`).

Domain separator for all proofs: ASCII `did:qi:document:v1` || `0x00` || `unsignedDigest`.

### 20.1 Vector 1 — Create

Genesis payload (QCDE-1, 726 octets) — this is the hashing input for the DID:

```
{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://webizen.network/ns/did-qi/v1"],"assertionMethod":["#key-0"],"authentication":["#key-0"],"capabilityInvocation":["#key-0"],"service":[{"id":"#mailbox","serviceEndpoint":{"contactKeyMultibase":"zX8CKVGdsbqkmGWCUgEaiFBMxvRuaTzZM1ESdfwnrEbk","disclosure":"ApprovedRelaysOnly","generation":0,"locatorKind":"mailbox","publicDht":false,"relayHints":[{"hintId":"invite-1","operatorHashHex":"c338d16aa019a88e5e1c1ba848d70db97aee6b73fe17928f4c94a738f8fc9737"}]},"type":"CscpMailbox"}],"verificationMethod":[{"id":"#key-0","publicKeyMultibase":"z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw","type":"Ed25519VerificationKey2020"}]}
```

| Field | Value |
|---|---|
| `genesisDigest` hex | `bc845eeecf604e7909e67af54a85f047ce7863c24706ef1b9441567861c1491c` |
| DID | `did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu` |
| `unsignedDigest` hex | `73432eeabe01888f4770654fcd9a85b59605a2b3eb9d82b16b700c17345c6744` |
| `qiDocumentDigest` | `z8kwJYVreKg6qPC6zRhC5z7jZ5KiU6AqvRsgVQVAaYToh` |
| `proofValue` | `z651bPNYS2aodGhrnGJ5i8oRpbdghCtVRQf3u21aih2igpxfoAULJpj2atgGQxi4CreM1tPgd17F2wNctKZPSySLg` |
| signature hex (64) | `fd8b12c03ff43f40de5521b0a14b53e82f6c62c054d9454141c01dd26834c5455e46416148758fd76d8b21a01d230dbe4efdd08da351239abaae987698a34209` |
| `qiGitObjectId` | `18962a639bdbe09b3998105716944d34b9da606d2d945701e6d68eff90aacddd` |
| signed QCDE-1 length | 1550 octets (`blob 1550\0` + bytes) |
| disclosure | `ApprovedRelaysOnly` |
| `locatorKind` | `mailbox` (not `direct`) |

Unsigned published document (QCDE-1):

```
{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://webizen.network/ns/did-qi/v1"],"assertionMethod":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"authentication":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"capabilityInvocation":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","qi":{"createdUnix":1788998400,"deactivated":false,"generation":0},"service":[{"id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#mailbox","serviceEndpoint":{"contactKeyMultibase":"zX8CKVGdsbqkmGWCUgEaiFBMxvRuaTzZM1ESdfwnrEbk","disclosure":"ApprovedRelaysOnly","generation":0,"locatorKind":"mailbox","publicDht":false,"relayHints":[{"hintId":"invite-1","operatorHashHex":"c338d16aa019a88e5e1c1ba848d70db97aee6b73fe17928f4c94a738f8fc9737"}]},"type":"CscpMailbox"}],"verificationMethod":[{"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0","publicKeyMultibase":"z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw","type":"Ed25519VerificationKey2020"}]}
```

Signed document: the unsigned object plus:

```
"proof":{"created":"2026-09-10T00:00:00Z","proofPurpose":"capabilityInvocation","proofValue":"z651bPNYS2aodGhrnGJ5i8oRpbdghCtVRQf3u21aih2igpxfoAULJpj2atgGQxi4CreM1tPgd17F2wNctKZPSySLg","type":"QiDocumentSignature2026","verificationMethod":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"}
```

Optional UTXO live attestation (caller-supplied tx bytes; **not** broadcast):

| Field | Value |
|---|---|
| `chainId` | `bip122:000000000019d6689c085ae165831e93` |
| OP_RETURN payload | `514901` \|\| `unsignedDigest` = `51490173432eeabe01888f4770654fcd9a85b59605a2b3eb9d82b16b700c17345c6744` |
| raw tx hex | `010000000111111111111111111111111111111111111111111111111111111111111111110000000000ffffffff010000000000000000256a2351490173432eeabe01888f4770654fcd9a85b59605a2b3eb9d82b16b700c17345c674400000000` |
| `txid` display | `e9dfb2471de55f2d1702a5dd1c560a28c4a632640d5f8d0648613b69f6a5a176` |
| `txid` wire (SHA-256d) | `76a1a5f6693b6148068d5f0d6432a6c4280a561cdda502172d5fe51d47b2dfe9` |
| `vout` | `0` |
| `kind` | `live` |

A second constitution-listed chain id (no second tx required for this vector): `bip122:000000000933ea01ad0ee984209779ba` (testnet3). Same DID, different attestation slot.

**Pass criteria:** parse DID; recompute genesis digest; QCDE-1 unsigned matches; Ed25519 verify; git object id matches; OP_RETURN extract matches `unsignedDigest`; Read returns generation 0; `locatorKind` is not `direct`.

### 20.2 Vector 2 — Update

Generation 1 adds `alsoKnownAs: ["did:web:example.invalid"]` and a `HostnameAlias` service. The DID string is unchanged. `qi.previousDigest` is Vector 1’s `qiDocumentDigest`. Mailbox `generation` is 1. Hostname is an alias, not the id.

| Field | Value |
|---|---|
| DID (unchanged) | `did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu` |
| `qi.generation` | `1` |
| `qi.previousDigest` | `z8kwJYVreKg6qPC6zRhC5z7jZ5KiU6AqvRsgVQVAaYToh` |
| `unsignedDigest` hex | `6180c97eb650da657c0e3801d421da7d6e9f0c39da568cb9fa0311d1d07dd88a` |
| `qiDocumentDigest` | `z7ZcSuVQbqDmhspuFXSKNBApt1733xoJbP7ito6GXBt6m` |
| `proofValue` | `z2qoBhr1hvJVLV6FasQBmkdyFJz2yqYRMHu1gjegaxaWqm2nZfoBDiyEhJv7pyPjC4TeWSHD73yUSEvvgRYoG8z3G` |
| signature hex | `5c18d919cc57f7d82cce106507decb218e181d19ba5292c3999090fc274471d8b795b42c538edcaff50164c7ad2e2406e4410dfa8e80bac92c13244263675e0f` |
| `qiGitObjectId` | `cf41711376c122981fbaa5873464cc618e160d81cdeb48acdb21e268d4175daf` |
| signed QCDE-1 length | 1807 octets |

Unsigned QCDE-1:

```
{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://webizen.network/ns/did-qi/v1"],"alsoKnownAs":["did:web:example.invalid"],"assertionMethod":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"authentication":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"capabilityInvocation":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","qi":{"createdUnix":1788998400,"deactivated":false,"generation":1,"previousDigest":"z8kwJYVreKg6qPC6zRhC5z7jZ5KiU6AqvRsgVQVAaYToh"},"service":[{"id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#mailbox","serviceEndpoint":{"contactKeyMultibase":"zX8CKVGdsbqkmGWCUgEaiFBMxvRuaTzZM1ESdfwnrEbk","disclosure":"ApprovedRelaysOnly","generation":1,"locatorKind":"mailbox","publicDht":false,"relayHints":[{"hintId":"invite-1","operatorHashHex":"c338d16aa019a88e5e1c1ba848d70db97aee6b73fe17928f4c94a738f8fc9737"}]},"type":"CscpMailbox"},{"id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#frontdoor","serviceEndpoint":{"didWeb":"did:web:example.invalid"},"type":"HostnameAlias"}],"verificationMethod":[{"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0","publicKeyMultibase":"z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw","type":"Ed25519VerificationKey2020"}]}
```

**Pass criteria:** Update bumps generation to 1; Read returns generation 1; presenting Vector 1 as current is `stale_generation`; `id` still equals Vector 1 DID; `did:web:example.invalid` is alias-only.

### 20.3 Vector 3 — Deactivate

Generation 2 tombstone. `service` is empty. `qi.deactivated` is true. Further Update fails closed.

| Field | Value |
|---|---|
| `qi.generation` | `2` |
| `qi.deactivated` | `true` |
| `qi.previousDigest` | `z7ZcSuVQbqDmhspuFXSKNBApt1733xoJbP7ito6GXBt6m` |
| `unsignedDigest` hex | `03d83bce6f4abfcf88fc48da535e407d61090f0a95678a02e02cae50ecef311d` |
| `qiDocumentDigest` | `zG1TkV1depiuTNVx5mzjDVHYrJYMP6SLqqE6SK6oETLY` |
| `proofValue` | `z2DHuvumf7jLSXQpdE93THmQ2gGPsEFv7cGqw9nreNuVDooQ6SuRBTQeLTF3pkhmRQSEqn4MRfYJ3MUfJXV17kHCn` |
| signature hex | `3c9de6e392eef7b1c5a367e705cbab7b93c6e34f56b635827a4251f8402bb1f10e3b7be11a7e557cae88470814b3b6edf5f017396d193532cba03480ada7cb03` |
| `qiGitObjectId` | `834846958cca9eb34fd2abcf559372f703fbe87da7d37f7b0f876673bce21860` |
| signed QCDE-1 length | 1222 octets |

Unsigned QCDE-1:

```
{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://webizen.network/ns/did-qi/v1"],"assertionMethod":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"authentication":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"capabilityInvocation":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","qi":{"createdUnix":1788998400,"deactivated":true,"generation":2,"previousDigest":"z7ZcSuVQbqDmhspuFXSKNBApt1733xoJbP7ito6GXBt6m"},"service":[],"verificationMethod":[{"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0","publicKeyMultibase":"z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw","type":"Ed25519VerificationKey2020"}]}
```

UTXO tombstone (caller-supplied; **not** broadcast):

| Field | Value |
|---|---|
| `chainId` | `bip122:000000000019d6689c085ae165831e93` |
| OP_RETURN payload | `5149ff` \|\| 32 zero bytes = `5149ff0000000000000000000000000000000000000000000000000000000000000000` |
| raw tx hex | `010000000111111111111111111111111111111111111111111111111111111111111111110000000000ffffffff010000000000000000256a235149ff000000000000000000000000000000000000000000000000000000000000000000000000` |
| `txid` display | `15079e3abf4375a6eefdac8695a122787615ec783d20a009698e9f33eefb1183` |
| `vout` | `0` |
| `kind` | `tombstone` |

**Pass criteria:** Read returns `deactivated: true` and empty `service`; Update after this document returns `deactivated`; tombstone OP_RETURN parses as `kind=tombstone`.

### 20.4 Vector 4 — Relay-only forbids Direct locator

Same DID and keys as Vector 1. The controller attempts to Create (or Update) a document with `disclosure: "ApprovedRelaysOnly"` **and** `locatorKind: "direct"` plus access-network fields `ipv6` / `port`. The operation MUST fail closed with `direct_locator_forbidden` **before** the document is stored. No git object and no UTXO MAY be written.

Unsigned document that MUST be rejected (QCDE-1):

```
{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://webizen.network/ns/did-qi/v1"],"assertionMethod":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"authentication":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"capabilityInvocation":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","qi":{"createdUnix":1788998400,"deactivated":false,"generation":0},"service":[{"id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#mailbox","serviceEndpoint":{"contactKeyMultibase":"zX8CKVGdsbqkmGWCUgEaiFBMxvRuaTzZM1ESdfwnrEbk","disclosure":"ApprovedRelaysOnly","generation":0,"ipv6":"2001:db8::1","locatorKind":"direct","port":4242,"publicDht":false,"relayHints":[]},"type":"CscpMailbox"}],"verificationMethod":[{"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0","publicKeyMultibase":"z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw","type":"Ed25519VerificationKey2020"}]}
```

Forbidden members present: `locatorKind` = `direct`; `ipv6` = `2001:db8::1`; `port` = `4242`.

**Pass criteria:** Create/Update returns `direct_locator_forbidden`; store remains empty or remains at the previous valid generation; a Read does not yield this document; implementations MUST NOT copy `2001:db8::1` into a CSCP Direct locator TLV.

Positive control: Vector 1 is the same disclosure with `locatorKind: "mailbox"` and is accepted.

### 20.5 Parse rejects (supporting, same harness)

These inputs are `invalid_did` for method `qi` (no store lookup):

| Input | Reason |
|---|---|
| `did:q42:ptr/00` | Wrong method (QRC) |
| `did:hcai:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu` | Wrong method (Human-Centered AI collision) |
| `did:hci:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu` | Wrong method (HCI) |
| `did:qualia:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu` | RDF/app-scheme collision |
| `did:qi:example.invalid` | Hostname in the DID |
| `did:qi:z` | Multibase payload not 32 octets |
| empty store Read of Vector 1 DID | `not_found` |

Empty-store Read MUST fail closed (`not_found`), not invent a document from the DID string alone.

---

## 21. Explicit non-claims

This specification does **not**:

- register `qi` in DID Spec Registries or any IANA registry;
- implement a Rust resolver, git daemon, or chain client;
- complete CSCP-08 (live public relay URL / MASQUE Internet) or CSCP-12 (IETF datatracker submit);
- set Internet honesty flags;
- replace Gate B, QSession, or a RelayLease;
- treat `parse_did_q42` as DID Core resolution;
- define a public DHT of protected persons;
- lock the method to one ticker, one forge, or `git://` as the security profile;
- put ☉ in DID strings;
- claim HCAI-ANP handshake/WebRTC binding is implemented;
- authorise `owl:sameAs` merger of NaturalAgents.

`did:hcinet` remains a reserved alternative method string if `qi` is later judged too opaque. It is not specified and not registered here.

---

## 22. Comparators (reuse, do not re-implement)

| Method / pattern | Use relative to `did:qi` |
|---|---|
| `did:peer` | Pairwise invitation; closest existing method to CSCP mailbox |
| `did:web` | HCAI-ANP Frontdoor / LIG only; alias, not this method |
| `did:key` | Static keys; no document updates — insufficient for generation |
| `did:btcr` | Proof that UTXO spend = update; parameterized here, not Bitcoin-only |
| `did:ion` | Sidetree + Bitcoin batching; different architecture |
| `did:pkh` | CAIP-10 account ids; account model, not this UTXO profile |
| Historical `did:git` drafts | Git object backing; do not take GitHub as the method |
| `did:q42` | QRC pointer; not CRUD |

---

## 23. References (informative)

- W3C DID Core 1.0, https://www.w3.org/TR/did-core/
- W3C DID Specification Registries, https://www.w3.org/TR/did-spec-registries/ (this method is not listed)
- RFC 8032 Ed25519, RFC 8785 JCS, RFC 4648 / multibase base58btc, FIPS 180-4 SHA-256
- CAIP-2 blockchain namespace identifiers
- [DID-QI-01](./qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/decisions/did-qi-git-utxo.md)
- [human-centric-nomenclature.md](./human-centric-nomenclature.md)
- [identifier-resolution.md](./qualia-decentralized-network-fabric/identifier-resolution.md) §3 QRC
- [hcai-agreement-negotiation-protocol.md](./hcai-agreement-negotiation-protocol.md) (`did:web` Frontdoor; nomenclature)
- CSCP `-00` ContactDescriptor generation and relay-only Direct exclusion

---

## 24. Document history

| Date | Change |
|---|---|
| 2026-09-10 | Initial complete method specification (Create/Read/Update/Deactivate, QCDE-1, git SHA-256 objects, parameterized UTXO, four offline vectors). Method not registered. Runtime not in this assignment. |
