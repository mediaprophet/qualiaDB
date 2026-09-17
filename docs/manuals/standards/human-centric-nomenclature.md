# Human-centric vs human-centered nomenclature

**Status:** Canonical Qualia / WebCivics wording for manuals, presentations, and identifier prose  
**Date:** 2026-09-10  
**Does not apply to:** DID method-specific identifiers, CSCP TLV bytes, or IETF RFCXML (those stay ASCII)

The semantic distinction between **centered** and **centric** is architectural, not stylistic.

**Centered** names a static design methodology (User-Centered Design, Human-Centered AI). The human is a temporary focal point of a process: a target the designer aims at, then leaves.

**Centric** names the structural topology of the system. The human is the permanent nucleus: the gravitational center or node from which data, credentials, permissions, and agency radiate. Other systemic elements (AI agents, relays, ledgers, DID documents) orbit that nucleus. They do not replace it.

Qualia and CSCP are **human-centric**. They are not Human-Centered AI (the usual expansion of **HCAI**). Short form for the network thesis: **Human-Centric Internet (HCInet)**. Mark in Qualia manuals and slides: **☉**.

A natural person is still not a DID. ☉ marks the *role* of the principal in the topology, not a join key.

## Contrast

| | Centered (static methodology) | Centric (systemic topology) |
|---|---|---|
| Role of the human | Temporary focal point of a design process | Permanent nucleus of the running system |
| Typical field | User-Centered Design, Human-Centered AI | Human-Centric Internet (HCInet), this repository |
| Visual | 🎯 bullseye, ⌖ crosshairs — the human is the **target** | ☉ nucleus / sun, ⊚ concentric — the human is the **core** |
| Failure mode | Ship a product “for users,” keep the platform as the center | Agents, DNS, chains, or relays treated as the nucleus |

## Systemic nucleus (orbit model)

These marks a core node with systems revolving around it, not a dot on a page.

| Symbol | Name | Use |
|---|---|---|
| **☉** | Circled dot / sun (heliocentric) | **Default HCInet mark.** Gravitational center; data, AI agents, and credentials orbit the human. |
| ⊚ | Circled ring | Concentric boundaries or policy layers originating from that agent. |
| ⚛ | Atomic symbol | Complex interrelated parts held by a central nucleus (use sparingly; easy to misread as nuclear energy). |

## Radiating agency (node model)

These marks the human as an active source, not a destination for design.

| Symbol | Name | Use |
|---|---|---|
| ✺ | Sunburst / radiating star | Outward expansion, emission, connectivity from one point. |
| ⟡ | White diamond with centered dot | Geometric node in a larger architecture. |
| ✹ | Sparkle / star | Active energetic focal point, not a passive target. |

## What these symbols must not do

- They are **not** DID method names, URI schemes, or wire opcodes. Do not mint `did:☉` or put ☉ in a Quin.
- IETF Internet-Drafts and RFCXML stay ASCII: write “human-centric (structural nucleus)” and point here.
- ☉ does not merge NaturalAgents, authorize `owl:sameAs`, or substitute for controller proof.

## Related terms

| Term | Mark / short form | Notes |
|---|---|---|
| Human-Centered AI | 🎯 (when contrasting) | Spell out. External field. Not Qualia’s identifier label. |
| Human-Centric Internet | ☉ HCInet | CSCP, fiduciary gates, invitation-scoped discovery. |
| HCAI-ANP | protocol acronym only | Human-Centric AI Agreement Negotiation. Not Human-Centered AI. |
| Qualia Identifier | `did:qi:` | DID method **name** for HCInet instruments. See [did-qi-git-utxo.md](./qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/decisions/did-qi-git-utxo.md). |
