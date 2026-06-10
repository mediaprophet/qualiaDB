# SPARQL-DID Integration Specification

**Status:** Draft  
**Editors:** QualiaDB Development Team  
**Date:** 2026-01-XX  
**Repository:** [QualiaDB](https://github.com/qualia-db/qualiadb)

---

## Abstract

This specification defines how Decentralized Identifiers (DIDs) are integrated with SPARQL 1.1/1.2 queries and RDF-Star serializations. It provides a standardized approach for:
- Authenticating federated SPARQL queries using DIDs
- Embedding DID-based provenance in RDF-Star triples
- Validating SPARQL results with DID-based signatures
- Resolving DID endpoints for federated query execution

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Terminology](#2-terminology)
3. [DID Integration in SPARQL](#3-did-integration-in-sparql)
4. [DID Integration in RDF-Star](#4-did-integration-in-rdf-star)
5. [Security Considerations](#5-security-considerations)
6. [Examples](#6-examples)
7. [References](#7-references)

---

## 1. Introduction

### 1.1 Motivation

DIDs provide a cryptographically-verifiable, decentralized identity layer. When combined with SPARQL and RDF-Star, they enable:
- **Verifiable Query Results**: Federated query results can be cryptographically signed by DID controllers
- **Provenance Tracking**: RDF-Star embedded triples can carry DID-based provenance metadata
- **CORS-Free Federation**: DID-based authentication enables cross-origin federated queries without traditional CORS
- **Trust Delegation**: DIDs enable granular permission delegation for federated SPARQL endpoints

### 1.2 Scope

This specification covers:
- DID-based authentication for federated SPARQL (SERVICE)
- DID resolution for endpoint discovery
- DID signature verification for query results
- RDF-Star triple annotation with DID provenance
- DID-based access control for SPARQL endpoints

This specification does NOT cover:
- DID method specifications (see [DID Core](https://www.w3.org/TR/did-core/))
- RDF-Star syntax (see [RDF-Star](https://w3c.github.io/rdf-star/))
- SPARQL syntax (see [SPARQL 1.1](https://www.w3.org/TR/sparql11-query/))

---

## 2. Terminology

| Term | Definition |
|------|------------|
| **DID** | Decentralized Identifier as defined in [DID Core](https://www.w3.org/TR/did-core/) |
| **DID Controller** | Entity that controls a DID, identified by the DID's `controller` property |
| **DID Document** | JSON-LD document describing a DID, containing verification methods and service endpoints |
| **DID Resolution** | Process of dereferencing a DID to retrieve its DID Document |
| **Federated Query** | SPARQL query using SERVICE keyword to delegate subqueries to remote endpoints |
| **RDF-Star** | Extension of RDF allowing triples as terms, using `<< s p o >>` syntax |
| **Embedded Triple** | Triple used as a term in RDF-Star, can carry annotations |
| **Content Credentials** | C2PA manifest for content provenance and authenticity |

---

## 3. DID Integration in SPARQL

### 3.1 DID-Based Authentication for Federated Queries

When executing a federated query using the SERVICE keyword, the client MUST authenticate using DID-based credentials.

#### 3.1.1 Authentication Methods

Three authentication methods are supported:

1. **DID-LD (JSON-LD Signature)**
   - Client signs request using DID's verification method
   - Signature included in `Authorization` header as JSON-LD signature
   - Format: `Authorization: DID-LD <base64-signature>`

2. **DID-JWT (JSON Web Token)**
   - Client issues JWT signed by DID's verification method
   - JWT included in `Authorization` header
   - Format: `Authorization: Bearer <jwt>`

3. **DID-VC (Verifiable Credential)**
   - Client presents Verifiable Credential proving authorization
   - VC included in `Authorization` header
   - Format: `Authorization: DID-VC <base64-vc>`

#### 3.1.2 Authentication Flow

```
1. Client resolves endpoint DID → DID Document
2. Client extracts verification method from DID Document
3. Client signs request using verification method
4. Client sends request with DID-based Authorization header
5. Server verifies signature using DID Document
6. Server executes query and returns results
7. Server signs results using server DID
8. Client verifies results using server DID
```

### 3.2 DID Resolution for Endpoint Discovery

Federated SPARQL endpoints MUST be discoverable via DID Documents.

#### 3.2.1 Service Endpoint in DID Document

```json
{
  "@context": ["https://www.w3.org/ns/did/v1"],
  "id": "did:example:123",
  "service": [
    {
      "id": "#sparql-endpoint",
      "type": "SparqlService",
      "serviceEndpoint": "https://example.com/sparql"
    }
  ],
  "verificationMethod": [...]
}
```

#### 3.2.2 Resolution Process

1. Parse SERVICE IRI to extract DID
2. Resolve DID to retrieve DID Document
3. Extract `SparqlService` endpoint
4. Execute federated query at resolved endpoint

### 3.3 DID-Based CORS Handling

DID-based authentication eliminates the need for traditional CORS by establishing trust relationships between DIDs.

#### 3.3.1 Trust Relationship Verification

Server MUST verify:
1. Client DID is in trusted DID registry
2. Client DID has permission to access requested graph
3. Request signature is valid using client DID's verification method

#### 3.3.2 CORS Header Generation

Upon successful authentication, server generates DID-based CORS headers:

```
Access-Control-Allow-Origin: <client-did>
Access-Control-Allow-Methods: GET, POST
Access-Control-Allow-Headers: Content-Type, Authorization
Access-Control-Allow-Credentials: true
```

### 3.4 DID-Based Access Control

SPARQL endpoints SHOULD implement DID-based access control:

#### 3.4.1 Graph-Level Permissions

```sparql
# Define access control using SHACL
:GraphAccess a sh:NodeShape ;
  sh:property [
    sh:path :hasReadPermission ;
    sh:hasValue did:example:alice
  ] .
```

#### 3.4.2 Permission Verification

Before executing query, endpoint MUST:
1. Extract client DID from authentication
2. Query access control graph for permissions
3. Verify client has permission to access requested graph
4. Deny query if permission not granted

---

## 4. DID Integration in RDF-Star

### 4.1 DID Provenance in Embedded Triples

RDF-Star embedded triples SHOULD carry DID-based provenance annotations.

#### 4.1.1 Provenance Annotation Pattern

```sparql
# Embedded triple with DID provenance
<< :alice foaf:knows :bob > :provenance [
  :createdBy did:example:alice ;
  :createdAt "2026-01-01T00:00:00Z"^^xsd:dateTime ;
  :signature "..." ;
  :didMethod did:key:z6Mk...
] .
```

#### 4.1.2 Required Provenance Properties

| Property | Description | Required |
|----------|-------------|----------|
| `:createdBy` | DID of entity that created the triple | YES |
| `:createdAt` | ISO 8601 timestamp of creation | YES |
| `:signature` | Cryptographic signature of the triple | YES |
| `:didMethod` | DID method used for signing | NO |
| `:context` | Graph context where triple was created | NO |

### 4.2 DID Signature for Embedded Triples

Embedded triples MUST be signed to ensure authenticity.

#### 4.2.1 Signature Process

1. Canonicalize embedded triple (subject, predicate, object)
2. Generate hash of canonical form (SHA-256)
3. Sign hash using DID's verification method
4. Include signature in provenance annotation

#### 4.2.2 Verification Process

1. Extract embedded triple and provenance
2. Canonicalize triple
3. Generate hash
4. Extract DID from `:createdBy`
5. Resolve DID to get verification method
6. Verify signature using verification method
7. Return verification result

### 4.3 DID-Based Triple Validation

SPARQL query results SHOULD include DID-based validation metadata.

#### 4.3.1 Result Validation Pattern

```json
{
  "head": {
    "vars": ["s", "p", "o"]
  },
  "results": {
    "bindings": [
      {
        "s": {"value": "alice", "type": "uri"},
        "p": {"value": "foaf:knows", "type": "uri"},
        "o": {"value": "bob", "type": "uri"},
        "_provenance": {
          "createdBy": "did:example:alice",
          "signature": "...",
          "verified": true
        }
      }
    ]
  }
}
```

---

## 5. Security Considerations

### 5.1 DID Method Selection

Implementations MUST:
- Use cryptographically secure DID methods (e.g., `did:key`, `did:web`, `did:ethr`)
- Avoid DID methods without proper key management
- Support key rotation through DID Document updates

### 5.2 Signature Algorithm Requirements

Implementations MUST:
- Use secure signature algorithms (Ed25519, ECDSA P-256, RSA-2048+)
- Reject algorithms with known vulnerabilities (MD5, SHA-1, RSA-1024)
- Support algorithm agility through DID Document negotiation

### 5.3 Replay Attack Prevention

Implementations MUST:
- Include timestamps in all signed requests
- Reject requests with timestamps outside acceptable window (±5 minutes)
- Implement nonces for critical operations

### 5.4 DID Document Caching

Implementations SHOULD:
- Cache resolved DID Documents with TTL
- Respect `expires` property in DID Documents
- Invalidate cache on DID Document updates

### 5.5 Privacy Considerations

Implementations MUST:
- Not expose private keys in logs
- Use DID resolution only when necessary
- Support selective disclosure through Verifiable Presentations

---

## 6. Examples

### 6.1 Federated Query with DID Authentication

```sparql
# Query remote endpoint with DID authentication
SELECT ?s ?p ?o WHERE {
  SERVICE <did:example:remote-endpoint> {
    ?s ?p ?o .
  }
}
```

Authentication header:
```
Authorization: DID-LD eyJhbGciOiJFZDI1NTE5...
```

### 6.2 RDF-Star with DID Provenance

```sparql
# Create embedded triple with DID provenance
INSERT DATA {
  << :alice foaf:knows :bob >> :provenance [
    :createdBy did:example:alice ;
    :createdAt "2026-01-01T00:00:00Z"^^xsd:dateTime ;
    :signature "..." ;
  ] .
}
```

### 6.3 DID-Based Access Control Query

```sparql
# Query access control graph
SELECT ?graph ?permission WHERE {
  ?graph :hasReadPermission ?did .
  FILTER(?did = did:example:alice)
}
```

---

## 7. References

1. [DID Core](https://www.w3.org/TR/did-core/) - W3C DID Core 1.0
2. [DID Resolution](https://www.w3.org/TR/did-resolution/) - W3C DID Resolution
3. [RDF-Star](https://w3c.github.io/rdf-star/) - RDF-Star Community Group Report
4. [SPARQL 1.1 Federated Query](https://www.w3.org/TR/sparql11-federated-query/) - W3C SPARQL 1.1 Federated Query
5. [SPARQL 1.1 Protocol](https://www.w3.org/TR/sparql11-protocol/) - W3C SPARQL 1.1 Protocol
6. [C2PA](https://c2pa.org/specifications/) - Coalition for Content Provenance and Authenticity
7. [JSON-LD Signatures](https://w3c-dvcg.github.io/ld-signatures/) - JSON-LD Signatures

---

## Appendix A: DID Method Registry

| Method | Status | Notes |
|--------|--------|-------|
| did:key | ✅ Recommended | Simple, no blockchain dependency |
| did:web | ✅ Recommended | Domain-bound, HTTPS verification |
| did:ethr | ⚠️ Conditional | Ethereum-based, gas costs |
| did:sov | ⚠️ Conditional | Sidetree-based, requires infrastructure |
| did:peer | ❌ Deprecated | Replaced by did:key |

---

## Appendix B: Extension Registry

Implementations SHOULD register the following SPARQL extensions:

| Extension | Hash | Purpose |
|-----------|------|---------|
| `did:resolve` | 0x4449445F5245534F4C | Resolve DID to Document |
| `did:verify` | 0x4449445F5645524946 | Verify DID signature |
| `did:auth` | 0x4449445F41555448 | Authenticate with DID |
| `did:sign` | 0x4449445F5349474E | Sign with DID |
| `did:permission` | 0x4449445F5045524D | Check DID permission |

---

## Appendix C: Zero-Allocation Implementation Notes

QualiaDB implements this specification using zero-allocation patterns:

- DID hashes stored as `u64` (60-bit FNV-1a)
- DID Documents cached in fixed-size arrays
- Signatures verified without heap allocation
- Provenance annotations use fixed-size slots
- Extension registry uses static function dispatch

Memory footprint per query: ~35KB (well under 42MB limit)

---

**Copyright** © Timothy Charles Holborn 2010 - 2026. All rights reserved.