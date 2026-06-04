# Webizen Protocol (RFC Draft)

**Layer 1 Application Protocol over Qualia-DB**

This specification defines the `Webizen` protocol—the Layer 1 application standard that provides Identity Management, Social Graph resolution, and the Permissive Commons rules over the bare-metal Qualia-DB (Layer 0) engine.

While Qualia-DB provides the physical 512MB memory boundary and the zero-allocation Webizen logic VM, the Webizen Protocol dictates how human beings establish cryptographic agency, project personas, and legally govern their shared data.

---

## 1. Identity Nyms as N-Dimensional Inferences

In traditional Web3 or database architectures, "Identity" is treated as a static noun—usually a flat JSON document permanently bound to a single public key.

The Webizen Protocol rejects this static model. Identity is a **behavior**. It is an enumerated set of **Identity Nyms** (personas, pseudonyms, facets) that continually develop and resolve over time. 

Because the underlying engine evaluates data as 48-byte Quins across a Spatiotemporal context vector, an Identity Nym is an **n-dimensional inference** derived from:
1. **Subjective Inferences**: Claims made *by* the human author about a specific nym (e.g., self-asserted preferences, local truths, temporal moods).
2. **Objective Inferences**: Claims made *about* the nym by external cryptographic actors in the Address Book (e.g., peer attestations, bilateral edge connections).
3. **Input Format Dynamics**: As the data formats, temporal context (time-of-day, historical epoch), and spatial coordinates shift, the Webizen VM dynamically re-evaluates the active Identity Nym.

As the analysis of these inferences develops over time, the identity is never static. Resolution questions are continuously recalculated by the logic VM based on the rolling window of the temporal context.

---

## 2. The Address Book Directory

The Address Book is not a simple contact list; it is the cryptographic foundation of the local subjective reality. 

By adding a peer's Decentralized Identifier (DID) to the local directory, the user is mathematically authorizing the Webizen VM to include that peer's objective inferences in the evaluation of the user's social graph. If a peer is removed, their Quins are severed from the resolution graph.

---

## 3. The Web-Extension Helper Bridge

To seamlessly interface Webizen Identity Nyms with standard Web3 and Social-Web applications, the protocol relies on a native **Web-Extension Helper Bridge**.

### 3.1 Role of the Extension
The extension acts as the secure, user-facing proxy to the underlying Qualia-DB Daemon.
- **Key Management**: It securely holds the root `ed25519-dalek` keypairs, isolating them from malicious web pages.
- **Nym Injection**: It injects the `window.webizen` provider API into the browser DOM.
- **Configuration Dashboard**: It provides the UI for users to manage their Address Book, select which specific *Identity Nym* they want to project to a specific app, and configure advanced Spatiotemporal context vectors.

### 3.2 The Authorization Flow
When a decentralized app requests to read or write data:
1. **App Request**: The app calls `window.webizen.requestAccess({ scopes: [...] })`.
2. **Intercept & Nym Selection**: The Helper Bridge intercepts the call. The user is prompted via the extension UI to select which Identity Nym (facet) they wish to present to the application.
3. **Delegated Signing**: The app constructs 48-byte Quins and passes them to `window.webizen.signAndInject(quin)`.
4. **Merkle Aggregation**: The Helper Bridge signs the data with the selected Nym's sub-key, injecting the authorized objective inference into the Layer 0 graph.

### 3.3 Serverless Sync via WebTorrent (Layer 1 Transport)
Because Qualia-DB stores its entire memory state as a strictly bounded, flat binary `.q42` file, the Webizen Protocol officially designates **WebTorrent** as a native Layer-1 Transport Protocol. 
Instead of relying on centralized servers for graph replication, the Webizen Browser Extension utilizes WebRTC to natively seed the subjective graph to the DHT community. The Native Local Daemon simultaneously runs a WebTorrent instance to leech and synchronize CRDT deltas offline.

---

## 4. The Permissive Commons Framework

*Note: This framework relies fundamentally on legacy historical models of the Permissive Commons. It is presented here as a skeletal architecture awaiting specific integration of historical works and supports.*

The Permissive Commons defines how shared data (the Bilateral Micro-Commons) is legally, economically, and computationally governed between peers. 

### 4.1 Computational Enforcement
Rules defined within the Permissive Commons are mapped directly to the `Context` and `Metadata` vectors of the 48-byte Quin.
When the Webizen VM attempts to unify an inference across shared data, it hits a hard Permissive Commons Gate. 

### 4.2 The Adaptive Network Harness
WebTorrent and IPFS P2P operations can severely deplete bandwidth on mobile or metered connections. The Webizen protocol enforces an **Adaptive Network Harness**:
- **Offline / Metered (Leech-Only)**: Protects the user's data cap.
- **Unmetered**: Full P2P seeding to the Permissive Commons.
Additionally, the Permissive Commons Lightning RPC tracks bytes seeded and matches it against the Compute Bounty, ensuring users are economically compensated for providing network bandwidth.

### 4.3 Decentralized AI Compute & Energy Opportunism (The Sleep-Cycle Assembly)
The Webizen Protocol utilizes a **Hybrid `.q42` Ledger** approach to Decentralized ML/AI. 
- The `.q42` graph contains the Permissive Commons **Compute Bounties** (the economic and legal rules), accompanied by a WebTorrent Magnet URI pointing to a standard dense ML model (e.g., `.gguf`).
- When a user's device goes to sleep, if their **Energy Circumstance** allows (e.g., connected to grid power, or off-grid solar batteries are at 100% capacity), the Daemon spins up a **Sleep-Cycle Assembly**. It downloads the model, runs the inference utilizing Fractal GPU Sharding, and submits the cryptographic proof-of-work back to the ledger to earn Lightning micropayments.

### 4.4 Neurosymbolic LLM Override (Axiomatic Spatio-Temporal Intercept)
When executing Large Language Models from the Permissive Commons, Webizens are not subjected to the black-box hallucinations or generic vector biases of connectionist AI. 
The protocol enforces a **Neurosymbolic Intercept**:
- The exact procedural vector matrices of the LLM are mapped into 48-byte `.q42` Quins.
- Local Webizens define strict **Spatio-Temporal Qualia Contexts** (e.g., `time=1920`, `location=australia`).
- As the opaque LLM executes natively on the device, the Webizen VM actively monitors the procedural path. If an execution triggers a vector that violates a local Symbolic Axiom (e.g., "thongs" meaning underwear vs. footwear in Australia), the Webizen VM mathematically clips and overrides the active tensor in real-time. This forces the LLM to instantly obey the human-defined, localized boundaries of the Permissive Commons.

### 4.5 Open Directives (Pending Integration)
The following topics require precise definition based on historical Permissive Commons models:
1. **Ramifications of Works**: What are the strict legal and computational consequences when an actor utilizes an inference from the Commons? 
2. **Supports and Entitlements**: How are micropayments, algorithmic proof-of-work, or verifiable credential presentations mathematically gated before access is granted?
3. **Revocation & Epoch Compaction**: If an author revokes consent to a previously shared subjective inference, how does the Permissive Commons dictate the physical erasure of those Quins via Epoch Compaction?
4. **Derivative Works & Licensing**: How do we encode Permissive Commons licensing rules directly into the 48-byte Quin metadata?
