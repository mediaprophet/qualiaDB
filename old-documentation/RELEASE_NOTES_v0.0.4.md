# Qualia-DB v0.0.4 Release Notes

## Major Highlights

### 1. The Webizen Rebrand
The legacy "Sentinel VM" terminology has been officially deprecated. The core logic execution engine, responsible for decentralized agency and HCAI governance, is now known as the **Webizen VM**. This aligns perfectly with the Principal-Agent philosophical model at the heart of the Qualia ecosystem.

### 2. W3C Solid Interoperability Bridge
We have deployed the `qualia-solid-bridge`. This module spins up a sandboxed `warp/tokio` server that acts as an **Allocation Firewall**. It successfully intercepts and translates heavy, string-based W3C Solid protocols (HTTP REST, JSON-LD, standard Turtle) into native 64-bit Quin hashes without violating the core 512MB RAM floor or causing heap bleeding.
- Legacy W3C Solid Pods can now ingest/export `.q42` CBOR-LD ledgers.
- Profiled aggressively with `dhat-rs` to ensure 0-byte heap boundaries during query routing.

### 3. Native "Hard Science" SHACL Extensions
The Webizen VM compiler has been extended to map custom `qualia:` semantic properties directly to pure-Rust mathematical solvers.
- `NativeThermodynamics` (MCMC Sampler / Gibbs Free Energy)
- `NativeOdeSolver` (RK4 Continuous Dynamics)
- `NativeQuantumDft` (Kohn-Sham Hamiltonian bounding)
- `NativeBioinformatics` (Hardware-accelerated SIMD Sequence Alignment)

These off-heap operations allow the Webizen VM to transparently step out of logical N3 resolution and natively compute advanced physics and biological interactions at query time.

### 4. UI: KaTeX LaTeX Rendering
The `qualia-desktop` Neuro-Chat UI has been upgraded with **KaTeX**. Any responses from the Hard Science extensions (e.g., differential equations or Hamiltonians) are now beautifully rendered into mathematically correct LaTeX natively inside the chat bubbles.

### 5. HCAI DNS Frontdoor
We have implemented a powerful new discovery mechanism for the peer-to-peer WebRTC mesh. 
The `qualia-cli webizen dns-frontdoor` command effortlessly generates a compliant `did:web` and DNS `TXT` (`_did`) payload. This allows Webizens to be globally discoverable via traditional web domains (e.g., `webizen.org`) while retaining their strict offline-first, zero-telemetry posture.

---

## Component Version Bumps

- `qualia-core-db`: **0.0.4**
- `qualia-cli`: **0.0.4**
- `qualia-desktop`: **0.0.4**
- `qualia-solid-bridge`: **0.0.4**

*For a full technical breakdown of the architecture, see [ARCHITECTURE.md](ARCHITECTURE.md).*
