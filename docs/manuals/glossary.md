# Qualia-DB Glossary

## Core Structures
- **Super-Quin (QualiaQuin)**: 48-byte struct (subject, predicate, object, context, metadata, parity). Replaces RDF triples.
- **SuperBlock**: 40960-byte (10 sectors) LZ4-compressed block holding ~850 Quins. Supports lazy header scanning.
- **Webizen / Webizen VM**: Core 1 logic engine. Executes SlgOpcodes (CheckDefeaters, CheckThreshold, etc.) on SlgArena (42MB tabling buffer).
- **SlgOpcode**: Bytecode for the Webizen (CheckTable, Unify, Halt, BranchWorld, etc.).
- **Modalities**: Specialized reasoning (spatio_temporal, probabilistic, dl, asp, linear, diffusion) normalized to Webizen.

## Query & Performance
- **Lazy SuperBlock Query**: O(1) header scan + selective LZ4 decompress + WebRTC P2P for remote blocks. Enables massive datasets under 512MB floor. Used by native  ench.
- **mmap_query_subject**: Fast point lookup via OS memory mapping.
- **Allocation Firewall**: Zero-allocation boundary (e.g. `ldp_translator.rs`) that intercepts heavy text protocols (HTTP/JSON-LD) and hashes strings into 64-bit Quins before they hit the core memory space.

## Architecture Extensions
- **HCAI Agreements**: Human Centric AI relationship contracts explicitly defined mathematically in the DB and bound by the Duty of Care.
- **DNS Frontdoor**: Subcommand to generate zero-permission W3C `did:web` and DNS `TXT` records, allowing Webizens to be globally discoverable without leaking telemetry.

## Agency & Economics
- **Permissive Commons**: Shared data governance with automatic Threshold Shift License (TSL) via ILP streams.
- **did:git**: Git-based decentralized identity for Webizen agency and axiomatic evolution (validated by Webizen).
- **Webizen**: Protocol for identity nyms/facets, address book, and Commons enforcement.
- **Neurosymbolic Intercept**: Map LLM tensors to Quins so Webizen can override violations of local Spatio-Temporal contexts.

## Tooling & Harness
- **qualia-cli**: Main binary. Key subcommands: ench --suite full (real measurements + JSON), import, query, daemon --compute-swarm, webizen, export-solid.
- **Dual-Mode Benchmark Harness**: Native (real engine + telemetry) vs browser/JS. Produces llm_benchmark_results.json. Now includes WordNet entries from actual import data.rdf wordnet.q42 (85.1% compression).
- **Fractal Sharding**: Multiple isolated 512MB cells on big hardware for swarm compute.

## Other
- **CBOR-LD**: Strict binary ingress format (no text Turtle/JSON-LD parsing).
- **Qualia Epoch**: Inline timestamp type (ns since 2026-01-01).
- **Author-Scoped Merkle**: Only sign your own Quins, not global root.

See developer-guide.md, AI_INSTRUCTIONS.md (esp. 13 for full CLI list), and RELEASE_NOTES_v0.0.3.md for more.
