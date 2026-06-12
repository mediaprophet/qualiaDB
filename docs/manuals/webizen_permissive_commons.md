# Webizen Permissive Commons Specification
_Version: 1.0.0-draft | Target: Webizen Platform_

The Permissive Commons is a decentralized, zero-knowledge distribution network and semantic caching layer built directly into the QualiaDB ecosystem. This specification defines the architectural and economic mechanisms for the **Permissive Commons**, moving beyond traditional "Open Data" by introducing enforceable licensing, semantic tracking, and economic thresholds before works become universally unencumbered.

### 1.1 Philosophical Foundation: Selfhood vs. Personhood
The conceptual foundation of the Permissive Commons (tracing back to the 2014 W3C WebPayments design goals and formally detailed in Timothy Holborn's 2018 *Human Centric Infosphere* and 2019 *Open Data v3.0*) identifies a structural flaw in the traditional "open data" movement: it inadvertently enabled an extraction model where human labor ("web slavery") provides free resources that are monopolised by a few massive corporate gatekeepers to train AI and act as centralized authorities on human knowledge.

True **"Commons"** differ from **"Open Data"** by instituting a sharp ontological distinction aligned with the legal and social boundaries of the human experience:
*   **Selfhood (Private/Inalienable):** Biometric information and private experiences are an extension of conscious experience and must remain entirely sovereign. This data is strictly excluded from the Permissive Commons unless explicitly escrowed.
*   **Personhood (Social/Economic):** Artefacts produced through the employment of personhood have social associations and are subject to law, rights, and responsibilities. The procedural relationship between private *Selfhood* and permissive *Personhood* forms the basis of the Semantic Inforg.
*   **Custodial Governance:** Resources are available to an intended group under specific rules, managed by stewards, rather than being "freely available to everyone to use and republish as they wish" (which enables extractive enclosure).
*   **Economic Reality:** The system acknowledges that "free doesn't exist." It implements micropayment standards as an "Economic Imperative for the Knowledge Age" to ensure natural persons are compensated for socially valuable works.
*   **Human Dignity over Privacy:** The system prioritizes human agency, ensuring Artificial Intelligence systems (Dynamic Semantic Agents or 'Inforgs') operate *locally* and *privately* as servants of natural agents, democratizing AI against corporate centralization and semantic manipulation.
*   **Provenance and Context (CoolURIs):** Shifting from data points to the DIKW (Data, Information, Knowledge, Wisdom) pyramid using Semantic Web technologies (RDF, SHACL, and specifically N3 for native logical rule representation). Decentralized Identifiers (DIDs) operate as native **CoolURIs**, ensuring that rich semantic context and verifiable history are maintained indefinitely, completely decoupled from the fragility and link rot of traditional HTTP domain name changes.

*Historical Grounding:* The *WellFair* vault and the broader Peace Infrastructure Project (established 2020) act as the operational crucibles where this Selfhood/Personhood dichotomy is practically enforced to safeguard human agency.

This document enumerates the artifacts managed by the commons, the networking layers facilitating distribution, and the low-level billing gates governing access.

## 1. Network Infrastructure

The Permissive Commons does not rely on centralized cloud storage. Instead, it leverages a hybrid peer-to-peer architecture embedded inside the Qualia native daemon.

### 1.1 SocialWebNet (The Transport Layer)
Secure, peer-to-peer networking is established via **SocialWebNet**, a dynamic tunneling system built on WireGuard.
* **DNSSEC Bootstrapping:** Peers resolve each other using zero-allocation CBOR-LD records published via DNSSEC (`_qualia._dnssec` TXT records).
* **Fiduciary Gatekeeping:** Before a WireGuard tunnel is established, the Sentinel VM executes a gatekeeper check (evaluating `q42:TrustGroup` intents). If the peer relationship is not authorized, the connection is structurally denied.

### 1.2 WebTorrent & Information Centric Networking (ICN)
Artifacts are distributed via an in-process HTTP WebTorrent seeder (`webtorrent_seeder.rs`), providing an **Information Centric Networking (ICN)** approach where assets are addressed by cryptographic hash rather than domain name.
* **Deterministic Addressing:** Every artifact is hashed (SHA-1) to create a deterministic magnet link.
* **Solid/RWW Backward Compatibility:** The distribution of containers over the WebTorrent DHT retains backward compatibility with "socially aware cloud storage" frameworks, such as the Linked Data Platform (LDP), Solid, and Read Write Web (RWW).
* **Native Integration:** Browser WebTorrent clients leech via the `ws=` magnet parameter pointing directly at the local Qualia daemon's loopback (`/torrent/webseed/{info_hash}`).
* **Deduplication:** The Distributed Hash Table (DHT)1.  **Quantum Processing Unit (QPU) Snapshots:** `.q42` DAG ledgers representing discrete, frozen states of semantic logic execution.
2.  **Webizen Social Cooperatives (QApps):** Declarative UI manifests (e.g., Kanban boards, dynamic forms, canvases) and application architectures that execute within the N3Logic Webizen VM.
3.  **Semantic Ontologies & Rulesets:** Vocabularies, SHACL validation shapes, drug-interaction databases, and N3 logic rules utilized for local inferencing without API dependencies.
4.  **Local AI Weights:** Pre-quantized neural network states (e.g., GGUF format files) and **HDF5 Container Harmonization** for complex, multidimensional datasets.
5.  **Strict Temporal Version Control:** Deprecation over Deletion. The system maintains deprecated older records to support temporal inferencing mapping, preserving the provenance and causal history required for natural justice and legal rule representation.

---

## 2. Enumeration of Supported Artifacts

The Permissive Commons is payload-agnostic but semantically aware. The following primary artifact types are formally supported and distributed via the network:

### 2.1 Webizen QApps & Cooperative UI
_Examples: `social-cooperative.html`, `yaml-ld-q42` manifests._
Stateless, declarative user interface shells that bind to the sovereign `qualia-db` engine. QApps represent collaborative spaces (Kanban boards, Analytics, Budgets, DocuQuin pipelines) and are fetched and hydrated from the WebTorrent DHT instantly, allowing applications to exist independently of app stores.

### 2.2 Semantic Ontologies and Logic Rules
_Examples: `.c.q42` bundles, `cooperative-projects.ttl`, `cooperative-evaluation.n3`._
Machine-readable definitions of domain knowledge. When a cooperative defines a new ontology or set of defeasible logic rules, these are published as deterministic magnet links, ensuring all participating nodes share the exact same structural understanding without runtime string allocations.

### 2.3 LLM Packages, Compute Weights & Language Sovereignty
_Examples: `GGUF` model files, tokenizers, HDF5 Containers._
Large Language Model weights and contextual embeddings. By distributing these multi-gigabyte files via the Permissive Commons, localized AI inference becomes a shared community resource. **Universal Language Support** is mandated: the semantic ecosystems and localized LLM resources must support all first languages, including all languages of prayer, defending human dignity globally.

### 2.4 Quantum Caches & Solved Topologies
_Examples: Pre-computed logic states, N3 inferences._
By distributing computationally expensive resolved states, the network prevents duplicate work and acts as a massive parallel processing cache.

### 2.5 Distribution Modalities (Compiled vs. Component)
Artifacts packaged via tamper-evident cryptographic instruments are available in two primary modalities to support semantic agents:
*   **Compiled (Offline/Complete):** A fully resolved distribution containing all semantically referenced datasets locally, ensuring functional resilience in disconnected environments.
*   **Component (Linked/Uncompiled):** A lightweight manifest where sub-records are resolved and retrieved via decentralized GET requests only when required by the local semantic agent.
_Examples: Normalized QUBO graph structures._
When an expensive QPU solves a complex combinatorial problem, the structural matrix is stripped of local semantic metadata, hashed, and published to the WebTorrent Commons. Subsequent peers facing the same mathematical topology pull the solution from the cache, completely bypassing the need for redundant quantum compute.

---

## 3. Permissive Commons Billing Gates (ADR 0003)

Decentralized distribution does not imply lack of governance. Data owners require the ability to enforce economic or work-based boundaries. Doing this at the application layer is slow and vulnerable to bypass.

Therefore, access to artifacts within the Permissive Commons is governed by **Billing Gates embedded directly into the bare-metal database routing logic.**

### 3.1 The Fifth Vector (Metadata Slot)
Every piece of data (represented as an `NQuin`) utilizes its 64-bit metadata field to store access and routing constraints. The system checks bitwise signatures on every hardware cycle during data projection.

### 3.2 Permissive Routing Lanes
The network defines strict routing lanes:
* **`PassthroughStandard (0x00)`**: No restrictions. Data flows freely through the WebTorrent swarm.
* **`EnforcePermissiveCommons (0x01)`**: The artifact is public, but the consumer's hardware signature must satisfy baseline network rules utilizing **semantic cryptographic proofs**. This avoids the massive energy consumption of Proof-of-Work (PoW) by relying entirely on social-semantics and cryptographic signatures (e.g., WebID-TLS for connection peers, OIDC for natural agent authorization, and HTTP signatures) wired directly into a `q42:TrustGroup` fabric.
* **`EnforceBilateralMicroCommons (0x02)`**: Highly restrictive. Before the data leaves the disk sector or is seeded over WireGuard, the requester must satisfy an explicit gate. This can be:
  * **Micropayments**: A verified transaction satisfying a `MASK_COMMERCIAL_BILLABLE_GATE`.
  * **Work Obligations**: A verified task completion satisfying `MASK_WORK_OBLIGATION_SATISFIED`.
  * **Credential Presentation**: M:N multisig consensus or specific W3C Verifiable Credentials.

Because this logic is executed at the hardware instruction level, it provides unbeatable security and frictionless micro-economies without the need for expensive, custom application-layer authorization middleware.

---

## 4. Threshold Licensing, Obligation Costs, and Entity-Based Governance

The Permissive Commons is not a monolithic "free-for-all." It natively supports complex economic boundaries designed to protect creators, indigenous knowledge holders, and cooperative groups prior to "fair compensation" being achieved.

### 4.1 The Threshold Shift License (TSL) & Obligation Costs
Producing valuable commons data (e.g., formalizing oral traditions into medicinal ontologies) incurs a calculable **Obligation Cost**. The Permissive Commons enforces the **Threshold Shift License**, which explicitly rejects infinite rent-seeking in favor of a mathematically verifiable cost.

The Obligation Cost is cryptographically bound to the asset using N3Logic and is calculated via the Risk-Compounded Obligation Algorithm:
`Total_Obligation = (Base_Fair_Value_Rate) × (Risk_Multiplier) × (Temporal_Compound)`

The asset undergoes two dynamic states:
*   **State A (Commercial / Pre-Threshold):** The asset is strictly commercial. The network enforces the obligation via the `EnforceBilateralMicroCommons` gate. Access requires satisfying the WebID negotiation endpoint via streamed micro-payments. This payment layer is highly extensible via QApps and Webizen Studio, currently supporting mechanisms such as the Interledger Protocol (ILP), Lightning Network, and eCash (XEC).
*   **State B (Permissive Commons / Post-Threshold):** Once the `Total_Obligation` is met via network contributions or cooperative funding, the N3Logic Webizen VM automatically and irreversibly shifts the license to State B. The economic obligation is satisfied, rent-seeking ceases, and the asset becomes freely available, subject only to **Conditional Network Protection** (users must share-alike by seeding the asset via the WebTorrent DHT to prevent proprietary enclosure).

### 4.2 Semantic Escrow & Dispute Adjudication
To govern provenance disputes over these economic streams, the local N3Logic Webizen VM acts as an adjudicator. If a dispute arises, ILP payment streams are routed into a cryptographic **Escrow State**. The VM evaluates `.q42` DAG ledgers against fundamental predicates (e.g., dismissing claims that attempt to enclose fundamental "Knowledge Axioms" as property, or proportionally splitting escrow based on mathematically calculated relational derivations).

### 4.3 Entity-Based Pricing
Using the Semantic Ontologies and the hardware billing gates, the network differentiates access requirements based on the consumer's entity type:
* **Natural Persons:** May access the resource freely or at a very low cost.
* **Corporations / Commercial Actors:** Trigger the `MASK_COMMERCIAL_BILLABLE_GATE`, requiring micropayments or contractual obligations to utilize the data.

### 4.4 Group-Scoped Commons
While some forms of the permissive commons are universally public, others are strictly ring-fenced for defined groups. For example, traditional knowledge databases may enforce verification of community affiliation (via Verifiable Credentials) through the `EnforceBilateralMicroCommons` routing lane before any data is projected or seeded.

---

## 5. Specific Ecosystem Use-Cases

The Permissive Commons is designed to support the distinct operational needs of various Webizen applications. Based on current system implementations, the following concrete use-cases are supported:

### 5.1 Webizen Social Cooperatives (SaaS Alternative)
Instead of relying on centralized SaaS platforms for project management, a decentralized organization utilizes the Permissive Commons to distribute its operational software.
*   **Artifacts:** Declarative UI shells for Kanban boards, analytic dashboards, budgets, and smart contracts.
*   **Mechanism:** Members pull the `social-cooperative` QApp manifests directly via WebTorrent. Coordination occurs over SocialWebNet WireGuard tunnels, ensuring no corporate intermediary observes or monetizes the cooperative's internal data.

### 5.2 WellFair: The Computational Vault & Peace Infrastructure
The *WellFair* personal health vault acts as an encrypted, local-first extension of the natural person (Peace Infrastructure). It must operate without leaking metadata to centralized servers.
*   **Artifacts:** Medical interaction databases (e.g., drug interaction rulesets), SHACL validation shapes, and N3Logic reasoning rules (e.g., clinical rules mapping sleep/stress to adrenal fatigue).
*   **Mechanism:** The vault retrieves these rulesets from the Permissive Commons natively. Because the vault downloads the structural knowledge rather than querying an API, the natural person can evaluate complex medical contexts locally, ensuring absolute privacy (especially critical when operating in Sanctuary/Duress modes).

### 5.3 Local AI & Verifiable Communications
The ecosystem supports rich, local-first communications (e.g., video calls with local language transcoding or CV analysis) without third-party APIs.
*   **Artifacts:** Local LLM weights (GGUF), standard WebSpeech models, and standardized ODRL "Semantic Handshake" templates. To ensure cultural sovereignty, distributed linguistic models strictly enforce universal language support, defending dignity by enabling native processing of all languages (including languages of prayer).
*   **Mechanism:** A user needing to transcribe a sensitive medical consultation downloads the transcoder weights from the WebTorrent Commons. Furthermore, the structural templates required to legally establish the call (Semantic Handshakes) are fetched from the commons, standardizing trust without a centralized registry.
