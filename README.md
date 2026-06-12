# QualiaDB / Webizen

> Peace infrastructure for the natural person — running on your hardware, connected to the world on your terms.

[Watch: *The Untransferable Code*](https://www.youtube.com/watch?v=HJJs-Ve-Dhg) — the philosophical foundation of this work.

> **⚠️ Pre-release — active development.**
> This is v0.0.x software. APIs, binary formats, wire protocols, and `.q42` storage layout change without notice until v0.1.0. Do not use in production.

---

## The problem

Every major platform today treats your data, your identity, and your relationships as assets it owns. AI systems compound this: they act on behalf of whoever controls the infrastructure, not on behalf of you.

QualiaDB is built on a different premise. Software running on your device has a fiduciary obligation to *you* — the natural person who owns the hardware. It acts as your agent, not as a data pipeline for a third party.

---

## What it is

**QualiaDB** (desktop application: **Webizen**) is a personal semantic graph engine with a built-in AI governance layer. Four capabilities, all running locally on your hardware:

**1. Semantic graph storage**
Binary `.q42` ledgers store and query RDF knowledge graphs with microsecond latency on a 512 MB RAM budget. WordNet (523 MB of RDF) compresses to 74.6 MB and streams with 6.5 ms first-query latency via demand-paged memory mapping. No cloud required.

**2. Webizen governance VM**
An N3Logic + SHACL + deontic logic engine that evaluates rules, rights, and constraints at query time. Permissions, obligations, and prohibitions are enforced by the VM — not by a remote API call, and not by trusting the application layer.

**3. Fiduciary AI layer**
Every LLM call is pre- and post-validated against your declared rights and capability profile. The model never runs without your consent; its output must carry semantic provenance. Conduct violations are written to a cryptographically auditable, DID-associated log.

**4. Human-Centric with Socially Defined Networking**
DID-based identity with Verifiable Credentials, SocialWebNet peer-to-peer networking (DNSSEC-bootstrapped, WireGuard-encrypted), and W3C Solid interoperability for connecting with institutions and preserving your right to move your data elsewhere.

---

## Who it is for

| | |
|---|---|
| **Individuals** | Own your AI agent and your data. No platform intermediary, no surveillance. |
| **Developers** | Build Webizen qapps on a SPARQL + semantic graph API with full fiduciary guarantees baked in. |
| **Institutions** | Interoperate with Webizen users via W3C Solid Protocol and WebID without adopting the full stack. |

---

## Get started

**Live playground (no install):** [mediaprophet.github.io/qualiaDB/playground/](https://mediaprophet.github.io/qualiaDB/playground/index.html)

**Desktop app (Flutter — Windows, macOS, Linux):** Download from [Releases](https://github.com/mediaprophet/qualiaDB/releases).

**CLI:**
```bash
cargo build --release -p qualia-cli
./target/release/qualia --help
```

**WASM:** `qualia-core-wasm.tar.gz` in [Releases](https://github.com/mediaprophet/qualiaDB/releases) — embed in any web project.

Full build instructions, CLI reference, and benchmark guide: [docs/manuals/DEVELOPMENT.md](docs/manuals/DEVELOPMENT.md).

---

## Current status

**v0.0.11-dev** — active development, pre-release.

This branch includes: in-process LLM inference with GPU dispatch; SPARQL 1.1 + RDF-Star engine; full deontic / epistemic / LTL / paraconsistent modality stack; SHACL biosciences, chemistry, and biomedical extensions; DID Verifiable Credentials; SocialWebNet DNSSEC peer bootstrap; W3C Solid export; 539+ tests.

Known gaps before v0.1.0: ML-DSA (FIPS 204 not yet complete), ZK proof backend, SPARQL engine build errors under resolution. See [TODO.md](TODO.md).

Full release history: [CHANGELOG.md](CHANGELOG.md).

---

## Documentation

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Full technical architecture — Quin bit layout, all modalities, inference stack, every module |
| [docs/manuals/DEVELOPMENT.md](docs/manuals/DEVELOPMENT.md) | Build, test, benchmark, CLI reference, cross-compilation |
| [docs/release-targets.md](docs/release-targets.md) | Feature status across all five release targets (Browser, Mobile PWA, CLI, Desktop, Mobile Native) |
| [docs/manuals/qapp-vault-developer-guide.md](docs/manuals/qapp-vault-developer-guide.md) | Build qapps for the Webizen Qapp Vault |
| [docs/manuals/flutter-api-reference.md](docs/manuals/flutter-api-reference.md) | Flutter FRB API reference |
| [CHANGELOG.md](CHANGELOG.md) | Release history |
| [AGENTS.md](AGENTS.md) / [CLAUDE.md](CLAUDE.md) | AI agent orientation for contributors |

---

## License

[Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International](LICENSE)

For commercial licensing, enterprise integration, or consulting on Intentional Computing:
**Timothy Charles Holborn** · [LinkedIn](https://www.linkedin.com/in/ubiquitous/)

---

*Built to guarantee first-class digital agency.*
