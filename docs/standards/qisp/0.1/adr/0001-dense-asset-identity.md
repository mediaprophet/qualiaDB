# ADR 0001 — Dense asset identity: content-addressed URIs, never in-band pointers

**Status:** Accepted (Editor's Draft 0.1, provisional — not a W3C/OGC standard)
**Date:** 2026-07-13
**Requirements:** QISP-R03, QISP-R04
**Plan sections:** §0 layer 2, §2.2 point 5, §3.4, §3.6

## Context

QISP exposes meshes, `.10d` sections, tensor buffers, trajectories, and BVHs. The tempting shortcut
is to expand this dense form into RDF (triplify every vertex/tensor cell) or to smuggle a native
handle — a Rust address, GPU buffer pointer, or raw file offset — across the network API as an RDF
literal. Both are prohibited (§14). Triplification destroys the zero-copy design and the 48-byte
`NQuin` layout; an exported pointer is a security hole (stale-handle reuse, forged offsets) and
leaks process-internal state across a trust boundary.

## Decision

The public RDF term for a dense asset is a **stable URI — preferably content-addressed (or a DID
URL)**. Its descriptor graph carries the digest algorithm and digest, media type, byte length and
optional byte-range/section descriptor, coordinate frame/CRS/dimension order/units, topology
assumptions, provenance, access policy, integrity status, and lifecycle state (§3.4).

Internally — and only internally — the URI resolves to a bounded record such as `DenseAssetRef`
(a ~60-bit token, generation number, section kind, offset, length, and digest prefix). A native
address is **never** placed into an `NQuin`. Generation numbers make stale handles fail closed.
Each alternate representation (WKT, GLB, `.10d`, tensor buffer, descriptor-only RDF) carries its own
representation-specific digest, and transformations between representations record PROV-O activities
with declared loss/error (§3.6).

## Consequences

- **Positive:** external identity is verifiable (digest) and stable; pointer resolution stays
  process-local and validated (QISP-R04); a non-QISP client can still read the descriptor as plain
  RDF and fetch an authorized alternate representation (QISP-R16); forged digest/offset/generation
  attacks fail closed (test target, §12 security).
- **Negative / cost:** every asset needs a descriptor graph and a resolver with generation
  validation; content-addressing requires digesting on write; representation negotiation adds an
  HTTP content-negotiation and `Link`-relation surface.
- **Follow-on:** the canonical external asset URI scheme is decision QISP-D04 (HTTPS
  content-addressed with alternates; DID URL optional, never mandatory). A registered `.10d` media
  type is still pending; a provisional vendor type is used during incubation.
