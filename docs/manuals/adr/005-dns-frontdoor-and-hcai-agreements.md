# ADR 005: DNS Frontdoor and HCAI Agreements

## Status
Accepted (v0.0.4)

## Context
Qualia-DB is an offline-first, zero-telemetry sovereign vault. However, for a user (Principal) to establish a connection with another user or organisation, they must be globally discoverable without leaking their raw `.q42` data or exposing themselves to unauthenticated network probing.

## Decision
We implement a "DNS Frontdoor" utilizing W3C `did:web` standards. 

1. The `qualia-cli webizen dns-frontdoor` command generates a minimal `did.json` document and matching `TXT` (`_did`) records.
2. The user uploads these to their standard Web2 domain (e.g., `webizen.org`).
3. This `did:web` explicitly exposes ONLY the `QualiaAgreementNegotiation` endpoint (routing to the WebRTC mesh).
4. Relationships are defined strictly via **HCAI (Human Centric AI) Agreements**, enforcing the Duty of Care mathematically before any Quins are exchanged.

## Consequences
- Allows seamless interoperability with legacy DNS infrastructure.
- Maintains the strict zero-telemetry posture (the `.q42` database is never exposed to HTTP crawlers).
- Requires users to own a standard domain name or use a trusted federated registrar for their frontdoor.
