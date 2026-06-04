# ADR 004: Sentinel to Webizen Terminology Rebrand

## Status
Accepted (v0.0.4)

## Context
The core execution engine of Qualia-DB was originally named the "Sentinel VM". This terminology heavily implied a passive, observational, or exclusively defensive posture (like a guard or firewall). As the system evolved to support active agency, cryptographic identity (`did:git`), decentralized LLM workflows, and multi-party guardianship consensus, "Sentinel" no longer accurately reflected the entity's role.

## Decision
We decided to rebrand the "Sentinel VM" and all related modules to "Webizen" (and "Webizen VM"). 

"Webizen" (Web Citizen) perfectly encapsulates the active, sovereign nature of the software. A Webizen is an autonomous digital agent that acts on behalf of the Principal, enforcing the Principal's rights, executing their logic, and independently interacting with other Webizens in the peer-to-peer Commons.

## Consequences
- Requires a global refactor of UI strings, documentation, and source code (completed in v0.0.4).
- Aligns the documentation and code directly with the "Principal-Agent Duty of Care" philosophical framework.
- Distinguishes the Qualia-DB execution context from other "Sentinel" trademarked cybersecurity or monitoring products in the legacy Web2 space.
