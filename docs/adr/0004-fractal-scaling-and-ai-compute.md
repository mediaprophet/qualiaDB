# Architecture Decision Record: 0004
## Fractal Scalability, Energy Opportunism, and AI Compute

**Date**: 2026-06-01  
**Status**: Accepted  

### 1. Context
Qualia-DB maintains a strict 512MB RAM floor and zero-allocation logic. This guarantees mechanical sympathy and absolute sandbox stability on constrained personal devices. However, this artificially throttled high-end personal computing rigs (e.g., 64GB RAM, 8-core CPUs, heavy Nvidia GPUs). 
Additionally, decentralizing the graph via P2P (WebTorrent/IPFS) poses severe risks for users on metered connections (bandwidth poverty) or off-grid solar deployments (energy poverty). 

### 2. Decision: Fractal Sharding
Rather than increasing the 512MB limit and constructing a monolithic 64GB heap (which re-introduces garbage collection pauses, pointer chasing, and lock-contention), the daemon will dynamically detect hardware and spin up a **Swarm of independent 512MB Cells**. 
- A mobile phone runs 1 Cell.
- A 64GB rig runs 64 to 128 parallel Cells mapped directly to specific CPU threads and GPU SMs. 
- The 512MB Cell is treated as a stateless "Shard" to achieve infinite parallelism.

### 3. Decision: The Adaptive Network & Energy Harness
The daemon now natively tracks and respects the physical circumstances of the user:
- **Network Mode**: Defines if the node is `offline`, `metered` (leech-only to protect bandwidth), or `unmetered` (full WebTorrent/IPFS seeding).
- **Energy Mode**: Defines if the node is `strict` (conserve battery), `opportunistic` (run heavy compute only when solar panels detect excess wasted energy or batteries are at 100%), or `unlimited` (grid power).

### 4. Decision: The Sleep-Cycle Swarm (Decentralized AI Compute)
To utilize spare capacity without draining active resources, the daemon implements a **Sleep-Cycle Swarm**.
- We strictly reject inventing a proprietary file format for ML weights. Standard formats like `.gguf` are massive dense matrices.
- Instead, the `.q42` file acts as the **Hybrid Compute Ledger**.
- The `.q42` graph contains Permissive Commons "Compute Bounties" and cryptographic rules.
- The `Metadata` vector of the `.q42` quin contains a Magnet URI. 
- When the device goes to sleep (and Energy rules permit), the daemon downloads the `.gguf` via WebTorrent, utilizes its Fractal GPU cells to perform the inference or training, and submits the cryptographic proof-of-work back to the `.q42` ledger to claim Lightning network compensation.

### 5. Consequences
- **Positive**: We achieve massive parallelism and AI model inference capabilities without abandoning the zero-allocation 512MB constraints.
- **Positive**: Off-grid and low-bandwidth users are mathematically protected from resource drainage.
- **Negative**: The CLI initialization becomes more complex as users must properly define their environment vectors (`--energy-mode`, `--net-mode`, `--workers`).
