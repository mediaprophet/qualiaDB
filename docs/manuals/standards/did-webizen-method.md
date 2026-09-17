# Webizen Identifier (`did:webizen`) DID Method Specification

**Status:** W3C Community Group Report–style draft (internal). Not a W3C Technical Report. Not submitted to a Community Group. Not registered in the [W3C DID Specification Registries](https://www.w3.org/TR/did-spec-registries/).
**Method name:** `webizen`
**Prefix:** `did:webizen:`
**Date:** 2026-09-14
**Version of this draft:** 0.1.0
**Source decision:** [Standards Backlog §2b](./standards-backlog.md)
**Implements:** [W3C DID Core 1.0](https://www.w3.org/TR/did-core/) method requirements for Semantic Concepts, Ontology Packages, Bilateral Agreements, and Web of Data Principals with multi-transport resolution (IPFS, WebTorrent, HTTPS).

This document specifies the **Webizen Identifier** (`did:webizen`) method. It is the content-semantic and social coordination layer for the ☉ Human-Centric Internet (HCInet) and the evolved Web of Data. It is distinct from `did:qi` (HCInet network instrument identity) and `did:q42` (topological hardware/memory coordinate).

---

## 1. Status and Scope

This specification defines a decentralized identifier method for the **Web of Data**, establishing an immutable, domain-agnostic semantic architecture. It honors the foundational vision of Tim Berners-Lee and the W3C (Linked Data, open vocabularies, decentralized knowledge graphs) while replacing fragile DNS-coupled HTTP URIs and heavy triplestore overhead with:

1. **Domain Decoupling:** Concepts have permanent, intrinsic identifiers independent of ICANN domain registrations or HTTP server lifecycles.
2. **Multi-Transport Resolution:** Support for content-addressed ingress via IPFS (`ipfs://`), peer-to-peer swarm distribution via WebTorrent/BitTorrent (`magnet:`), local zero-copy database shards (`.q42`), and public HTTPS frontdoor gateways (`https://ns.webizen.org/`).
3. **Declarative Version Evolution:** Explicit Semantic Versioning with declarative migration delta mappings and automated forward-chaining graph rewrites.
4. **Cryptographic Attestation:** Releases cryptographically signed by authoring `did:qi` network instruments.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as described in BCP 14 (RFC 2119, RFC 8174).

---

## 2. The Three-Layer Identifier Topology

QualiaDB and the Webizen ecosystem maintain a strict, non-overlapping three-layer identifier separation:

```
┌───────────────────────────────────────────────────────────────────────────┐
│ Layer 1: SEMANTIC & SOCIAL WEB OF DATA LAYER — did:webizen:               │
│ • Concepts, Ontologies, Agreements, and Web of Data Principals            │
│ • Domain-decoupled, content-addressed, multi-transport                    │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │ attested & signed by
┌─────────────────────────────────────▼─────────────────────────────────────┐
│ Layer 2: NETWORK INSTRUMENT & CARRIER LAYER — did:qi:                     │
│ • HCInet network instruments: nodes, relays, mailboxes, controllers       │
│ • Cryptographic Ed25519 verification methods & UTXO commitments           │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │ compiled into
┌─────────────────────────────────────▼─────────────────────────────────────┐
│ Layer 3: PHYSICAL STORAGE & TOPOLOGICAL LAYER — did:q42: (QRC)            │
│ • Zero-copy hardware/disk coordinates (MSB=1 in 48-byte Super-Quin)       │
│ • Non-network RAM address & disk block dispatch                           │
└───────────────────────────────────────────────────────────────────────────┘
```

| Identifier | Class | Primary Role | SDO Alignment |
|---|---|---|---|
| `did:webizen:` | Semantic / Social DID | Concepts, Ontologies, Agreements, Web of Data | W3C DID Core / RDF / Linked Data |
| `did:qi:` | Network Instrument DID | Nodes, mailboxes, relays, key controllers | W3C DID Core / IETF CSCP |
| `did:q42:` | Topological Coordinate (QRC) | VM execution, RAM/disk physical pointer | Qualia Zero-Copy ABI (non-DID) |

---

## 3. Method Syntax & ABNF

A `did:webizen` identifier consists of the prefix `did:webizen:`, a functional realm, and a realm-specific identifier.

### 3.1 ABNF (RFC 5234)

```abnf
did-webizen          = "did:webizen:" realm ":" realm-specific-id
realm                = "concept" / "ont" / "agreement" / "agent"

realm-specific-id    = *( unreserved / pct-encoded / "@" / ":" / "_" / "-" )

; Realm 1: Semantic Concept (deterministic UUIDv5 or canonical slug)
concept-id           = uuid-str / 1*64( ALPHA / DIGIT / "-" / "_" )
uuid-str             = 8HEXDIG "-" 4HEXDIG "-" 4HEXDIG "-" 4HEXDIG "-" 12HEXDIG

; Realm 2: Ontology Package (name with SemVer tag)
ont-id               = 1*64( ALPHA / DIGIT / "-" / "_" ) "@" semver
semver               = 1*DIGIT "." 1*DIGIT "." 1*DIGIT [ "-" 1*32( ALPHA / DIGIT / "." / "-" ) ]

; Realm 3: Ratified Bilateral Agreement / Micro-Commons
agreement-id         = "z" 32*48BASE58BTC ; multibase SHA-256 genesis agreement digest

; Realm 4: Web of Data Agent / Citizen Principal
agent-id             = "z" 32*48BASE58BTC ; multibase public key fingerprint or DID hash

did-webizen-url      = did-webizen [ "/" path-abempty ] [ "?" query ] [ "#" fragment ]
```

### 3.2 Canonical Form and Normalization
* The scheme `did:` and method name `webizen:` MUST be lowercase.
* The realm names `concept`, `ont`, `agreement`, and `agent` MUST be lowercase.
* Realm-specific identifiers MUST be ASCII.
* UUIDs in `concept` MUST use lowercase hexadecimal characters.

---

## 4. Method Realms

### 4.1 Concept Realm (`did:webizen:concept:`)
Identifies an individual semantic concept, property, predicate, or class.
* **Post-Quantum Deterministic Concept UUID (RFC 9562 UUIDv8):** To ensure quantum collision resistance and zero-coordination consensus across independent nodes, concept UUIDs are deterministically derived using SHA-256 over the canonical Webizen namespace:
  $$\text{UUIDv8}(\text{WEBIZEN\_ONTOLOGY\_NS}, \text{"canonicalTermName"})$$
  Under Grover's quantum search algorithm, SHA-256 retains 128 bits of security, ensuring long-term post-quantum resistance against preimage and collision attacks.
* **Legacy Deterministic Concept UUID (RFC 4122 UUIDv5):** Supported for backward compatibility with classical systems using SHA-1.
* **Canonical URN Alias:** `did:webizen:concept:<uuid>` maps 1-to-1 with `urn:uuid:<uuid>`.
* **Example:** `did:webizen:concept:8f14e45f-cbf0-5f56-9a2d-3d231908b981` (derived from `"hasTrackId"`).

### 4.2 Ontology Package Realm (`did:webizen:ont:`)
Identifies a published, versioned bundle of vocabulary terms, SHACL constraints, and N3 logic rules.
* **Versioned Identifier:** Every release requires an explicit SemVer tag: `did:webizen:ont:perception@1.2.0`.
* **Immutability:** A released version tag MUST NOT be reassigned to different content. Upgrades increment the SemVer version.

### 4.3 Agreement Realm (`did:webizen:agreement:`)
Identifies a ratified bilateral or multi-party micro-commons contract graph (governance agreement, guardianship treaty, data-sharing covenant).
* **Hash Anchoring:** The payload is the multibase SHA-256 digest of the ratified agreement terms Quins.
* **Example:** `did:webizen:agreement:z6MkpTHR8VNs...`

### 4.4 Agent Realm (`did:webizen:agent:`)
Identifies a human participant, autonomous persona, or collective custodian within the Web of Data social layer.
* **Separation from `did:qi`:** While `did:qi` represents the physical/network host or node, `did:webizen:agent` represents the user's sovereign profile, knowledge graph root, and WebID.

---

## 5. Multi-Transport Resolution Contract

A conformant Webizen resolver resolves a `did:webizen` identifier into a **Webizen Entity Document (WED)** or **W3C DID Document** across multiple transport layers:

```
                        did:webizen:<realm>:<id>
                                   │
                 ┌─────────────────┴─────────────────┐
                 ▼                                   ▼
        [Phase 1: Local Resolution]         [Phase 2: Network Resolution]
        • Memory-mapped .q42 shard          • Content: IPFS CID (ipfs://)
        • Zero-heap FNV-1a lookup           • P2P: WebTorrent (magnet:)
        • Latency: < 1 microsecond          • Frontdoor: HTTPS Gateway
```

### 5.1 Resolution Order
1. **Tier 0 (Local Disk/Memory Shard):** The resolver first checks locally mounted `.q42` dictionary shards and active workspace caches. If present, resolution completes with zero network access and zero heap allocation.
2. **Tier 1 (Content-Addressed IPFS):** If a local miss occurs and an IPFS Content Identifier (CIDv1) is declared in the local manifest or peer hint, the payload is retrieved via IPFS bit-swap or gateway.
3. **Tier 2 (WebTorrent Swarm):** For bulk ontology packages, lexicon shards, and multimodal models, the resolver joins the swarm via the declared Info-Hash (`magnet:?xt=urn:btih:...`).
4. **Tier 3 (HTTPS Frontdoor Gateway):** Resolves via standard Web PKI at `https://ns.webizen.org/did/webizen/<realm>/<id>`.

---

## 6. Manifest & Distribution Descriptor Schema

Every published ontology pack or concept set distributes a manifest (in Turtle, JSON-LD, or CBOR-LD) describing its identity and multi-transport carriers:

```turtle
@prefix webizen: <https://ns.webizen.org/vocab/#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dcat:    <http://www.w3.org/ns/dcat#> .
@prefix owl:     <http://www.w3.org/2002/07/owl#> .

<did:webizen:ont:perception@1.2.0> a dcat:Dataset, owl:Ontology ;
    dcterms:identifier "urn:uuid:8f14e45f-cbf0-5f56-9a2d-3d231908b981" ;
    dcterms:title "Q42 Perception & Multimodal Ontology"@en ;
    owl:versionInfo "1.2.0" ;
    webizen:publisher <did:qi:z6MkpTHR8VNs123456789> ;
    
    # Multi-Transport Distribution Endpoints
    dcat:distribution [
        a dcat:Distribution ;
        dcat:mediaType "application/x-q42" ;
        webizen:transport "ipfs" ;
        dcat:accessURL <ipfs://bafybeic7.../perception-1.2.0.q42>
    ] , [
        a dcat:Distribution ;
        dcat:mediaType "application/x-q42" ;
        webizen:transport "webtorrent" ;
        dcat:accessURL <magnet:?xt=urn:btih:6a3b8c9d...&dn=perception-1.2.0.q42>
    ] , [
        a dcat:Distribution ;
        dcat:mediaType "text/turtle" ;
        webizen:transport "https" ;
        dcat:accessURL <https://ns.webizen.org/q42/1.2.0/perception.ttl>
    ] .
```

---

## 7. Version Evolution, Deprecation & Migration Mappings

To ensure that downstream code and graphs dependent on earlier ontology versions remain functional, Webizen ontologies mandate **declarative delta mappings**.

### 7.1 Delta Mapping Table
When terms are upgraded or restructured, the new release MUST include a delta table mapping superseded terms:

```json
{
  "ontologyId": "did:webizen:ont:perception@2.0.0",
  "priorVersion": "did:webizen:ont:perception@1.2.0",
  "delta": [
    {
      "from": "did:webizen:concept:legacyBoundingBox",
      "to": "did:webizen:concept:hasBoundingBox",
      "relation": "supersededBy",
      "mappingRule": "subPropertyOf"
    },
    {
      "from": "did:webizen:concept:oldSoundClass",
      "to": "did:webizen:concept:proposesSoundClass",
      "relation": "equivalentProperty",
      "mappingRule": "equivalent"
    }
  ]
}
```

### 7.2 QualiaDB Forward-Chaining Rewrite Integration
QualiaDB's AOT rule compiler compiles the delta mapping table into forward-chaining N3 rules at graph ingestion:
```n3
# Auto-generated compatibility bridge
{ ?s <did:webizen:concept:legacyBoundingBox> ?o } => { ?s <did:webizen:concept:hasBoundingBox> ?o } .
```
This guarantees that queries formulated against older predicates transparently match newer assertions without data migration.

### 7.3 Fail-Closed Gate: Held / Not Yet (`E300`)
If a breaking version transition occurs where:
1. A concept is deleted without a declared migration target, or
2. A **living sense** (human rights, welfare, kinship) is reclassified as an artifact,

the ingestion engine MUST fail closed and transition the frame to **Held / Not Yet (`E300`)**, halting execution before silent semantic drift occurs.

---

## 8. Post-Quantum Publisher Attestation (`ML-DSA-65` & `DualProof`)

To protect semantic definitions against future harvest-now-decrypt-later attacks and quantum forgery, every published Webizen manifest MUST carry a post-quantum cryptographic proof:

1. **Post-Quantum Primitive (FIPS-204):**
   * Primary signature algorithm is **ML-DSA-65** (Module-Lattice Digital Signature Algorithm, FIPS-204, NIST Security Category 3, 192-bit security) via the repository's pure-Rust `fips204` engine (`fiduciary_crypto.rs`).
   * COSE algorithm identifier: `-49` (RFC 9964).
2. **Hybrid Bound Policy (`DualProof`):**
   * For dual-stack verification across classical Web 2.0 gateways and edge nodes, manifests MAY carry a `DualProof` combining:
     - **ML-DSA-65** post-quantum lattice signature (`PK_LEN` = 1952 bytes, `SIG_LEN` = 3309 bytes).
     - **Ed25519** classical Edwards-curve signature (`PK_LEN` = 32 bytes, `SIG_LEN` = 64 bytes).
   * **Fail-Closed Rule:** Both signatures must verify over the identical canonical QCDE-1 payload digest. Verifiers MUST NOT accept whichever signature happens to verify.
3. **Digest Calculation:**
   * The canonical payload is committed via SHA-384 or SHA-256 (quantum collision-resistant under Grover's algorithm).
4. **Verification Flow:**
   * Resolvers extract the publisher's public key from their `did:qi:` document or embedded assertion method.
   * If the ML-DSA-65 signature is corrupted or missing, the manifest is hard-rejected with error `invalid_pq_proof`.

---

## 9. Binary Wire Encoding & Zero-Heap Engine Representation

To satisfy the 48-byte Super-Quin (`NQuin`) architecture and the 42MB Sentinel budget:

1. **60-bit FNV-1a Folding:**
   A `did:webizen:concept:<id>` is hashed into the Quin's 60-bit payload slot using compile-time FNV-1a:
   $$\text{quin\_token} = \text{fnv1a}(\text{did\_bytes}) \;\&\; \text{0x0FFF\_FFFF\_FFFF\_FFFF}$$
   The `MSB` is set to `0` (denoting a lexicon dictionary entity rather than a `did:q42` physical hardware pointer).
2. **CBOR-LD Tag 37:**
   When serialized into binary CBOR-LD, UUID concepts are transmitted as 16-octet raw byte strings with CBOR Tag 37 (RFC 8949), avoiding 36-byte ASCII string bloat.

---

## 10. Security & Privacy Considerations

1. **Immutability of Concept Semantics:** Changing the meaning of a concept under an existing UUID without bumping the SemVer version is strictly prohibited.
2. **P2P Swarm Poisoning Prevention:** When retrieving `.q42` volumes via WebTorrent or IPFS, nodes MUST verify the SHA-256 root hash against the signed manifest before mounting.
3. **Privacy of Agent Identifiers:** `did:webizen:agent:` identifiers used in sensitive contexts (medical, financial) MUST use pairwise pseudonymous derivations rather than public global keys.
