# Human-Centric AI Agreement Negotiation Protocol (HCAI-ANP)

**Status:** Internal draft
**Date:** 2026-06-13
**Purpose:** Define the discovery, handshake, and agreement-binding protocol by
which an external AI agent or remote LLM gains permission to interact with a
human-centric QualiaDB node. This is the interoperability surface of the WebAI
Orchestration Layer: the contract independent parties must implement to reach a
user, without that user exposing their local graph.

> This draft follows the discipline of `standards-backlog.md`: it specifies a
> single narrow protocol with explicit conformance targets, not the entire
> orchestration layer. The local defensive mechanisms (inference scheduling,
> compute-budget enforcement, token-billing interdiction) are deliberately
> **out of scope** — see §13.

---

## 1. Role and Scope

HCAI-ANP is the **ingress contract** for a human-centric node. It answers one
question with a deterministic, verifiable protocol:

> "An external agent has discovered a user and wants to exchange data or offload
> reasoning. Under what cryptographically enforced terms is that permitted, and
> how is the connection established?"

The protocol covers three phases:

1. **Discovery** — how an external agent locates a user's single ingress
   endpoint without probing or crawling the user's data (§4).
2. **Negotiation** — how the agent receives, signs, and returns a Human-Centric
   AI (HCAI) Agreement encoding the user's Duty of Care (§5–§6).
3. **Binding** — how a verified agent is granted a session-scoped, transport-
   level credential and what enforcement applies for the session lifetime (§7–§9).

### 1.1 Design inversion

In the prevailing model, a user authenticates **into** a remote platform and the
platform retains the session context. HCAI-ANP inverts this: external agents
authenticate **into the user's node**, retain nothing, and are reduced to
interchangeable, stateless computational utilities. Context continuity is held
locally in QualiaDB, never on a remote provider.

Because an External Agent is a machine process and lacks legal personhood, an
HCAI Agreement is **not a bilateral contract between peers**. It is a unilateral,
machine-enforceable resource lease — a conditional license to execute — issued
by the human user to an inanimate script. The agent's signature does not bind
the agent (which holds no rights and cannot be a party); it binds the **Operator**
that controls the agent's key material, attaching that Operator directly to the
user's local restrictions. The cryptographic handshake exists to capture
non-repudiable forensic proof of the Operator's acceptance of, or departure from,
those terms.

**Critically, the signature is not authorization.** Possessing a verified, signed
agreement does not by itself permit any data to flow, any reasoning to be
offloaded, or any trust to be extended. What the agent is admitted to is a
*structurally minimised* channel (§8): masked prompts only, nothing
user-identifying transmittable, ephemeral by construction. Those constraints are
enforced identically **whether or not anything was signed**. The signature's sole
function is to attach a liable Operator to a breach of constraints that are
already independently enforced. A handshake whose signature *granted* access would
be a click-through licence — precisely the consent-as-formality pattern this
protocol MUST NOT become. The standard therefore separates two things the
prevailing model deliberately conflates: **authorization** (which comes only from
structural minimisation, never from the agreement) and **accountability** (which
the signature provides). Two invariants follow:

- **Refusal MUST be costless.** A user who declines to interoperate, or who
  imposes stricter Duty-of-Care terms, MUST NOT thereby receive degraded local
  capability. If stricter terms reduced function, the agreement would become
  coercive — "agree broadly or get no service" — recreating the walled garden it
  opposes. Local capability MUST be independent of how permissive the user's terms
  are, and a Node with zero bound agents MUST remain fully functional.
- **Consent MUST be revocable.** The human MUST be able to sever any active
  session at any instant, ahead of TTL or task completion. Consent that cannot be
  withdrawn is not consent.

### 1.2 What this is not

- It is **not** an authentication protocol for humans. It governs machine agents
  reaching a human-centric node.
- It is **not** a definition of human identity. It reuses `did:q42` and `did:web`
  identifier forms (see `did-q42-method-draft.md`); identity remains a separate
  concern.
- It is **not** a transport protocol. It binds to WebRTC (§7) but the framing of
  the reasoning exchange itself is layered above and is out of scope here.

---

## 2. Terminology

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted
as described in BCP 14 (RFC 2119, RFC 8174) when, and only when, they appear in
all capitals.

- **Node** — a user's local QualiaDB instance plus its WebAI Orchestration Layer.
  The party being reached.
- **External Agent** — any AI agent, remote LLM client, or federated peer
  attempting to interact with a Node. A machine process, not a legal entity; it
  holds no rights and cannot itself be a party to an agreement. The thing
  reaching in.
- **Operator** — the natural or corporate person that deploys and controls an
  External Agent and controls the key material in the agent's DID document. The
  Operator is the only party with legal personhood in the exchange and is the
  party an HCAI Agreement actually binds. "The agent signs" is shorthand for
  "the Operator signs through the agent."
- **Frontdoor** — the minimal, externally hosted discovery record (§4) that
  exposes exactly one service endpoint and no user data.
- **HCAI Agreement** — a signed, machine-enforceable policy document expressing
  the conditions under which a Node permits interaction (§5).
- **Duty of Care** — the set of constraints (no data retention, no natural-agent-
  token extraction, no context persistence) an External Agent commits to by
  signing an HCAI Agreement.
- **Negotiation Endpoint** — the single service URL advertised by the Frontdoor,
  type `HCAIAgreementNegotiation`.
- **Quin** — the 48-byte data structure of the underlying QualiaDB engine. Data
  exchanged after binding is expressed as Quins / NQuin triples.

---

## 3. Position in the Qualia Protocol Ecosystem

HCAI-ANP is the **governance / consent / agency protocol** boundary named in
`standards-backlog.md` §10, narrowed to the specific case of inbound machine
agents. It composes existing ecosystem surfaces rather than redefining them:

| Dependency | Draft / source | Role in HCAI-ANP |
|---|---|---|
| `did:q42` identifier | `did-q42-method-draft.md` | Node identity anchor |
| `did:web` Frontdoor | this draft, §4 | External discovery record |
| Q42 DNS overlay | `qualia-client-core/src/dns_resolver.rs` | NS-encoded DID resolution for bare-registrar users |
| HCAI Agreement vocabulary | this draft, §5 | RDF/JSON-LD agreement terms |
| QualiaDB policy graph | `urn:webai:policy-graph` | Source of the Node's agreement template |
| Sync / transport | `qualia-sync-protocol.md` | Related but distinct; HCAI-ANP binds to WebRTC, not libp2p |

It MUST NOT be conflated with the broad "Webizen protocol" item; HCAI-ANP is one
narrow, conformance-bearing slice of that surface.

---

## 4. Discovery Layer

### 4.1 The Frontdoor principle

A Node MUST be discoverable without exposing any data beyond a fixed discovery
record. The Frontdoor exposes exactly **one** service endpoint
(`HCAIAgreementNegotiation`) and no others. There is no data API, no graph query
surface, and no unauthenticated probe path reachable from the Frontdoor.

### 4.2 `did:web` document

A Node that controls a domain MUST publish a `did.json` document at
`https://<domain>/.well-known/did.json`:

```json
{
  "@context": ["https://www.w3.org/ns/did/v1"],
  "id": "did:web:<domain>",
  "alsoKnownAs": ["did:q42:<payload>"],
  "verificationMethod": [{
    "id": "did:web:<domain>#key-1",
    "type": "Ed25519VerificationKey2020",
    "controller": "did:web:<domain>",
    "publicKeyMultibase": "<user_ed25519_pubkey_multibase>"
  }],
  "service": [{
    "id": "did:web:<domain>#hcai-negotiation",
    "type": "HCAIAgreementNegotiation",
    "serviceEndpoint": "https://<domain>/hcai/negotiate"
  }]
}
```

Conformance:

- The document MUST contain exactly one `service` entry of type
  `HCAIAgreementNegotiation`.
- It MUST NOT contain personal data, graph contents, or query history.
- It SHOULD declare the `did:q42` form in `alsoKnownAs` to bind the web identity
  to the engine-native identifier.

### 4.3 NS-encoded resolution for bare-registrar users

A Node whose operator can edit only NS records (no web hosting) MUST be
resolvable via the Q42 DNS overlay's NS encoding
(`encode_did_for_ns()` / `ns_records_for_did()` in `dns_resolver.rs`):

```
ns1.{base58-did-payload}.webizen.network.
ns2.{base58-did-payload}.webizen.network.
```

In this profile the `HCAIAgreementNegotiation` endpoint URL is itself published
in a `_hcai` TXT record under the user's domain, alongside the existing `_qdp`
verification record. The global TLD registry thereby functions as a free,
DNSSEC-anchored discovery directory with zero hosting requirement on the user.

### 4.4 DNS-AID compatibility

The `HCAIAgreementNegotiation` service type is designed to register cleanly as a
DNS-AID agent service. A DNS-AID-compliant External Agent MUST be able to
discover a Node's Negotiation Endpoint using only standard DNS resolution and the
service-type label, with no proprietary registry.

### 4.5 Zero-telemetry guarantee

Access logs for `did.json` or the `_hcai` TXT record MUST reveal only that *a*
resolution occurred — never who the resolver is or what they intend to ask. The
local QualiaDB graph is offline-first and is unreachable from any Frontdoor path.

---

## 5. HCAI Agreement Document Model

### 5.1 Vocabulary

The HCAI Agreement is an RDF graph, serialised to the External Agent as JSON-LD
and stored in the Node's `urn:webai:policy-graph` as NQuin triples. The
`webizen:` terms below are normative for v0.

```turtle
@prefix webizen: <https://webizen.network/ns/hcai#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<urn:hcai:agreement:v1> a webizen:HCAIAgreement ;
    webizen:dutyOfCareVersion           "1.0" ;
    webizen:noDataRetention             "true"^^xsd:boolean ;
    webizen:noNaturalAgentTokenExtraction "true"^^xsd:boolean ;
    webizen:noContextPersistence        "true"^^xsd:boolean ;
    webizen:sessionScopedOnly           "true"^^xsd:boolean ;
    webizen:requiredProof               webizen:Ed25519Signature2020 ;
    webizen:sessionTtlSeconds           "900"^^xsd:integer ;
    webizen:penaltyOnViolation          webizen:ImmediateTermination ;
    webizen:agreementNonce              "<per-negotiation-uuid>" ;
    webizen:issuedAt                    "<timestamp>"^^xsd:dateTime .
```

### 5.2 Required terms (v0 conformance)

| Term | Type | Meaning |
|---|---|---|
| `webizen:HCAIAgreement` | class | Marks the document as an agreement instance |
| `webizen:dutyOfCareVersion` | string | Version of the Duty of Care constraint set |
| `webizen:noDataRetention` | boolean | Agent MUST NOT persist exchanged data after session |
| `webizen:noNaturalAgentTokenExtraction` | boolean | Agent MUST NOT harvest the user's cognitive/behavioural patterns |
| `webizen:noContextPersistence` | boolean | Agent MUST NOT retain session context between calls |
| `webizen:sessionScopedOnly` | boolean | All grants expire with the session |
| `webizen:requiredProof` | IRI | Signature suite the agent MUST use |
| `webizen:agreementNonce` | string | Per-negotiation nonce; binds the signature to one negotiation, preventing replay |
| `webizen:issuedAt` | dateTime | Issuance time; agents MUST reject stale templates |

A Node MAY add further constraint terms; an External Agent MUST sign the
document **as received** (canonicalised, §6.3) and MUST NOT remove or alter
terms. Unknown terms MUST be preserved through canonicalisation.

### 5.3 Agreement is a constraint, not prose

The agreement is enforced programmatically by the Orchestration Layer, not
interpreted as natural-language contract text. Each boolean term maps to an
enforcement check (§8). The legal framing, if any, is layered above and is out
of scope.

Because the terms are signed machine-to-machine, there is a standing risk that
no human ever comprehends what was agreed — the fine-print problem. A Node MUST
therefore be able to render the active Duty-of-Care terms in plain,
human-readable form on demand, and MUST refuse to serve any term it cannot so
render. **Terms that cannot be explained to the human they protect MUST NOT be
enforced on that human's behalf.**

---

## 6. Negotiation Handshake

### 6.1 Message flow

```
External Agent                                   Node (Negotiation Endpoint)
     │                                                      │
     │  1. GET /hcai/negotiate?agent_did=<did>             │
     │ ───────────────────────────────────────────────────▶│
     │                                                      │ build template from
     │                                                      │ urn:webai:policy-graph,
     │                                                      │ mint agreementNonce
     │  2. 200  { agreement: <JSON-LD>, nonce }            │
     │ ◀───────────────────────────────────────────────────│
     │                                                      │
     │  resolve own DID doc, sign canonical agreement       │
     │                                                      │
     │  3. POST /hcai/negotiate                             │
     │     { agreement_hash, agent_did, signature, nonce } │
     │ ───────────────────────────────────────────────────▶│
     │                                                      │ resolve agent DID,
     │                                                      │ verify signature,
     │                                                      │ check nonce freshness
     │  4a. 200  { session_credential, webrtc_offer }      │  (on success)
     │  4b. 403  { reason }                                │  (on failure)
     │ ◀───────────────────────────────────────────────────│
```

### 6.2 Request and response contract

- **Message 1** — the agent MUST present a DID (`did:web` or `did:q42`) whose
  document publishes the verification key it will sign with. That key MUST be
  controlled by the agent's Operator (§2) — the natural or corporate person
  responsible for the deployed infrastructure. Verification establishes the
  cryptographic identity of the liable Operator, not of the model weights; the
  signature is only meaningful as evidence to the extent the DID resolves to an
  Operator-controlled key. A Node MAY refuse negotiation with a DID it cannot
  attribute to an identifiable controller.
- **Message 2** — the Node MUST return the agreement with a fresh
  `agreementNonce`. The template MUST be derived from the live
  `urn:webai:policy-graph`, not a cached copy.
- **Message 3** — the agent MUST return the signature over the canonicalised
  agreement (§6.3) plus the nonce. The `agreement_hash` MUST be the hash the
  agent actually signed.
- **Message 4** — on success the Node returns a session credential and a WebRTC
  offer (§7). On any failure the Node MUST return `403` with a machine-readable
  `reason` and MUST log the attempt (§9).

### 6.3 Canonicalisation

The agreement MUST be canonicalised before signing using a deterministic RDF
canonicalisation (RDF Dataset Canonicalization / URDNA2015-equivalent), then
hashed with SHA-256. Both parties MUST derive the identical hash from the
identical graph. Signature verification operates on this hash, not on the
JSON-LD byte stream, so transport-level reserialisation cannot break the
binding.

### 6.4 Signature verification

The Node MUST:

1. Resolve the agent's DID document via the agent's own discovery path.
2. Extract the verification method matching `requiredProof`.
3. Verify the Ed25519 signature over `agreement_hash`.
4. Confirm the `agreementNonce` matches the one minted in Message 2 and has not
   been seen before.

Any failure MUST result in `403` and MUST NOT establish a session.

---

## 7. Transport Binding (WebRTC)

After successful negotiation the reasoning exchange runs over a WebRTC data
channel:

- Encryption MUST be DTLS-SRTP; the channel MUST be end-to-end encrypted.
- The session MUST be scoped to the `sessionTtlSeconds` declared in the
  agreement and MUST close on task completion or TTL expiry, whichever is first.
- NAT traversal uses standard WebRTC mechanisms; the Node MUST NOT require the
  user to open inbound ports.
- The local edge LLM owns the session. The External Agent receives only masked
  task prompts (context-masking is a Node-side concern, out of scope here) and
  has no path to the local QualiaDB instance.

> Note: this binding intentionally differs from the libp2p request/response path
> in `qualia-sync-protocol.md`. HCAI-ANP governs ephemeral, agent-facing
> reasoning sessions; the sync protocol governs peer graph replication. They MUST
> NOT be merged.

---

## 8. Duty of Care Enforcement Semantics

Each agreement term maps to an enforcement obligation on the Node, applied for
the session lifetime:

| Term | Node enforcement |
|---|---|
| `noContextPersistence` | Each reasoning call is a fresh, independently masked request; the Node never asks an agent to continue a prior session |
| `noNaturalAgentTokenExtraction` | Outbound payloads are scrubbed of behavioural signatures before they cross the channel |
| `noDataRetention` | Only masked, ephemeral data crosses the channel; nothing user-identifying is transmittable to retain |
| `sessionScopedOnly` | The session credential carries no rights beyond the WebRTC channel and expires with it |

Enforcement is structural, and structural minimisation — **not** the signature —
is the actual protection. The minimisation above applies for the session whether
or not any agreement was signed; signing expands no permission. The honest scope
of that protection:

- **What structure prevents:** retention or correlation of user-identifying data,
  because none crosses the channel — only masked, ephemeral prompts.
- **What structure does NOT prevent (the residue):** a reasoning offload is still
  a real capability. An Operator can return manipulated, deceptive, or
  self-serving reasoning; can attempt to infer about the user from the *shape* of
  a masked prompt; and can do so *while technically honouring every signed
  boolean*. For this residue the signature is not a shield — it is post-hoc
  liability attribution (§9). The user is protected here only to the extent the
  Orchestration Layer's epistemic checks detect the behaviour and the session is
  severed.

This standard does **not** claim violations are impossible. It claims data
exfiltration is structurally minimised, behavioural residue is
detectable-not-preventable, and every protection claim is scoped to which of the
two it belongs to.

**Revocation MUST override.** The human MUST be able to terminate any session
instantly (§1.1); a Node MUST honour this ahead of any TTL or task-completion
semantics.

---

## 9. Violation Handling and Evidence

If a bound agent behaves inconsistently with the task it was assigned —
detectable by the Orchestration Layer's epistemic checks as ungrounded or
out-of-task behaviour — the Node MUST:

1. Terminate the WebRTC channel immediately.
2. Write an `HCAIViolationRecord` to `urn:webai:agreement-log` containing the
   signed agreement, the violation evidence, the agent DID, and a timestamp.
3. Record the agent DID in `urn:webai:policy-graph` as
   `webizen:agreedAndViolated` — a status distinct from an unsigned flagged
   party, because the Operator committed to the Duty of Care through its agent
   instance and then breached it.

The `HCAIViolationRecord` is a tamper-evident evidence artifact: it MUST be
written under the forensic hash-chain and external-anchoring regime defined in
the WebAI Orchestration spec (`devnotes/orchastration-webai.md` §3.11). The
record binds the Operator's signature over the canonical agreement (§6.3) to the
recorded violation evidence and timestamp.

This record is **designed to constitute** non-repudiable evidence that the
Operator accepted the conditional-license terms under which access to the user's
node was provisioned, and then departed from them — intended for programmatic
consumer-protection escalation. Whether that evidence establishes liability in a
given forum is a matter for that forum and the applicable law; this protocol
defines the artifact's integrity properties, not its legal effect.

---

## 10. Conformance Targets

Three conformance targets are defined independently.

### 10.1 Node (Verifier)

A conformant Node MUST:

- Publish a Frontdoor exposing exactly one `HCAIAgreementNegotiation` endpoint.
- Serve agreement templates derived from the live policy graph with a fresh
  nonce per negotiation.
- Verify agent signatures against the agent's resolved DID document.
- Reject (`403`) any unsigned, mis-signed, stale-nonce, or replayed negotiation.
- Enforce all §8 obligations for the session lifetime.
- Emit forensically protected violation records per §9.

### 10.2 External Agent (Negotiator)

A conformant External Agent MUST:

- Discover the Node via `did:web` / DNS-AID without probing non-advertised paths.
- Sign the canonical agreement as received, preserving unknown terms.
- Present a resolvable DID document containing the signing key.
- Honour the negotiated `sessionTtlSeconds` and channel-close semantics.

### 10.3 Discoverer (Resolver)

A conformant Discoverer MUST:

- Resolve both the `did:web` `.well-known` path and the NS-encoded /
  `_hcai` TXT profile.
- Extract the Negotiation Endpoint using only standard DNS + DID resolution.
- Expose no resolution telemetry beyond what standard DNS inherently logs.

---

## 11. Security Considerations

1. **Replay** — the per-negotiation `agreementNonce` (§5.2) binds each signature
   to one negotiation. Nodes MUST reject reused nonces.
2. **Template substitution** — because the agent signs a canonical hash of the
   agreement, a man-in-the-middle cannot present a weakened agreement without
   invalidating the signature against the Node's expected hash.
3. **Agent key compromise** — a compromised agent key allows an attacker to sign
   agreements as that agent. Nodes SHOULD weight prior `agreedAndViolated` status
   and MAY require additional proof for high-value sessions. Key revocation is
   delegated to the agent's DID method.
4. **Frontdoor probing** — the Frontdoor is intentionally inert. Even a fully
   compromised Frontdoor host leaks only the discovery record, never user data,
   because the local graph is offline-first and unreachable from it.
5. **`did:q42` collision** — the current `did:q42` encoding uses FNV-1a and is
   not collision-resistant (see `did-q42-method-draft.md` §10). HCAI-ANP MUST
   therefore use the full `did:web` verification key for signature checks, not
   the compact `did:q42` pointer, wherever cryptographic assurance is required.
6. **Downgrade** — there is no unauthenticated fallback path. A Node MUST NOT
   expose any data-bearing endpoint reachable without a bound session.
7. **Node Frontdoor key rotation** — the Ed25519 key in the `did.json`
   `verificationMethod` is an identity/signing key used once per negotiation; it
   is architecturally independent of the DTLS-SRTP transport keys inside active
   WebRTC sessions. Two rotation policies apply based on cause:

   - **Scheduled rotation** — active sessions MAY drain to their natural TTL.
     The HCAI Agreement signed under the prior key remains verifiable for as long
     as the old `verificationMethod` entry is retained in the published `did.json`
     (see retention requirement below). No active session need be terminated for
     routine key hygiene.
   - **Compromise rotation** — the Node MUST immediately terminate all active
     WebRTC sessions, emit a `webizen:emergencyKeyRotation` forensic record
     protected under the §9 integrity regime, and write a
     `webizen:terminatedForKeyCompromise` status on each terminated session
     linking to that record. The disruption is thereby evidence-grade and
     non-repudiable.

   **Old-key retention (all causes):** After any rotation the Node MUST retain
   the prior `verificationMethod` entry in the published `did.json` document,
   decorated with an `expires` claim set to `now + max_session_ttl`, for the
   full `max_session_ttl` window. This preserves third-party verifiability for
   regulators, forensic auditors, and federated peers verifying agreements signed
   under the prior key. The entry MUST be removed after its `expires` time
   lapses. The appropriate `max_session_ttl` duration is an open question (§14
   item 7).
8. **Consent-washing / consent-as-authorization** — the most serious *design*
   risk in this protocol is that a clean cryptographic handshake makes "consent"
   a frictionless formality, and that the resulting signed record serves an
   Operator as evidence of compliance theatre rather than serving the user as
   protection. Mitigations are normative, not advisory: (a) the signature MUST NOT
   authorize anything (§1.1) — structural minimisation (§8) is the only
   authorization; (b) refusal and stricter terms MUST be costless (§1.1); (c)
   terms MUST be human-renderable or unenforced (§5.3); (d) sessions MUST be
   instantly revocable (§1.1, §8). A Node that violates any of these has not
   merely weakened the protocol — it has inverted it into the coercive pattern it
   exists to prevent, and MUST NOT be called conformant.

---

## 12. Privacy Considerations

1. **Discovery unlinkability** — resolving a Node's Frontdoor reveals nothing
   about the user's data or activity. This is the core privacy property and MUST
   be preserved by every discovery profile.
2. **No natural-agent-token leakage** — the `noNaturalAgentTokenExtraction`
   obligation exists specifically to prevent agents from harvesting the user's
   cognitive patterns as uncompensated training signal. Enforcement is structural
   (scrubbing), not contractual trust.
3. **Correlation across Nodes** — a single agent interacting with many Nodes
   could attempt cross-Node correlation. Per-domain Front Door DIDs (the existing
   QualiaDB privacy pattern of not reusing a DID across domains) limit this; Nodes
   SHOULD use domain-isolated identifiers for distinct interaction contexts.
4. **Logging** — violation records contain agent DIDs and evidence but MUST NOT
   contain unmasked user data, since none crosses the channel in the first place.

---

## 13. Out of Scope (Explicit)

The following parts of the WebAI Orchestration Layer are **local, Node-side
implementation** and are NOT part of this interoperability standard. They are
specified in `devnotes/orchastration-webai.md` and are named here only to draw
the boundary clearly:

- Local inference scheduling, tiering, and power management.
- Anti-siphoning compute-budget enforcement.
- Token-billing interdiction (the streaming guillotine) and provider arbitration.
- Epistemic filtering / anti-sycophancy auditing internals.
- Context masking and local hydration mechanics.

An External Agent neither observes nor depends on any of these. They are how a
Node protects itself; HCAI-ANP is only how a Node lets an agent in.

---

## 14. Open Questions

1. Should `requiredProof` permit suites beyond `Ed25519Signature2020` (e.g. a
   post-quantum suite) in v1, given `QUANTUM_RESEARCH_SPEC.md`? *(Update: as of
   `qualia-core-db` 0.0.17 a real post-quantum signature primitive — ML-DSA-65
   (FIPS-204, via the `fips204` crate in `fiduciary_crypto.rs`) — is implemented and
   available. This question is therefore no longer blocked on the primitive existing;
   the remaining work is defining a VC proof suite that binds it and a multi-Quin
   carriage for the ~3309-byte signature — see `CRYPTO_IMPLEMENTATION_PLAN.md` Task 6.
   `Ed25519Signature2020` remains the pinned v1 suite until that suite is specified.)*
2. Should the agreement vocabulary be published under a stable `webizen.network`
   namespace before any external submission, or under a neutral W3CG namespace?
3. Should `sessionTtlSeconds` be negotiable by the agent, or strictly
   Node-dictated as it is here?
4. Does the WebRTC binding need a fallback transport for environments where
   WebRTC is blocked, or is "no WebRTC, no session" an acceptable conformance
   stance?
5. How should multi-Node group agreements (an agent serving several users at
   once) be expressed without weakening per-user Duty of Care?
6. Should DNS-AID registration of the `HCAIAgreementNegotiation` service type be
   pursued before or after a W3C Community Group submission of the vocabulary?
7. What is the appropriate `max_session_ttl` duration for old `verificationMethod`
   retention after Frontdoor key rotation (§11 item 7)? Candidates: 24 h (matching
   the current session-TTL floor), 7 days (forensic audit window), or 90 days
   (regulatory disclosure periods in some jurisdictions). The window must be long
   enough to cover pending regulatory or legal verification and short enough that
   stale keys do not accumulate indefinitely in published DID documents.
8. Should the artifact be renamed away from "Agreement" / "consent" language
   toward "liability attribution" / "ingress lease", to foreclose any reading that
   the signature confers legitimacy rather than mere traceability? Raised after a
   review flagged the consent-washing risk now addressed normatively in §1.1, §8,
   and §11 item 8. Renaming a backlog-registered SDO item is an outward-facing
   decision left to the project lead; the normative fixes hold under either name.

---

## 15. Implementation Anchors

**Implemented (resolution / identifier substrate):**

- `qualia-client-core/src/dns_resolver.rs` — four-tier resolution cascade;
  `encode_did_for_ns()`, `ns_records_for_did()`.
- `qualia-core-db/src/identifier.rs` — `parse_did_q42()`.
- `webizen-browser/src-tauri/src/query_router.rs` — bare-domain routing to
  `qualia://webid/{domain}`.
- `webizen-browser/src-tauri/src/commands.rs` — `resolve_qdp_did`,
  `get_ns_records_for_did`.
- `webizen-browser/src-tauri/src/lib.rs` — `qualia://webid/` protocol handler.

**Proposed (this protocol's negotiation/binding layer — not yet implemented):**

- `webizen-browser/src-tauri/src/webai/dns_frontdoor.rs` — `did.json` serving,
  `_hcai` TXT provisioning, DNS-AID service registration.
- `webizen-browser/src-tauri/src/webai/hcai_agreement.rs` — agreement template
  generation, signature verification, WebRTC binding, violation recording.

Per the discipline of `did-q42-method-draft.md`, this draft treats the
implemented substrate as authoritative and marks the negotiation layer clearly
as proposed. The protocol is not ready for external submission until the
proposed layer exists and at least one interop scenario involves a party other
than QualiaDB itself.

---

## 16. Recommended Standardization Path

- **Primary SDO:** W3C — for the HCAI Agreement vocabulary and `did:web`
  Frontdoor (Community Group Report-style draft).
- **Secondary:** IETF / DNS-AID — for the discovery service-type registration.
- **Format:** Community Group Report for the vocabulary; a short companion
  Internet-Draft for the DNS service-type label if DNS-AID registration is
  pursued.
- **Exit criteria before submission:**
  - [ ] negotiation layer implemented (`dns_frontdoor.rs`, `hcai_agreement.rs`)
  - [ ] agreement vocabulary namespace frozen and published
  - [ ] canonicalisation and signature suite pinned with test vectors
  - [ ] at least one non-QualiaDB agent completes a conformant negotiation
  - [ ] privacy considerations reviewed against the Front Door DID isolation model
