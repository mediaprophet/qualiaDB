# DID method for the Human-Centric Internet — `did:qi`, git, hostname, UTXO

- Decision ID: **DID-QI-01**
- Date: 2026-09-10
- Status: **recorded naming and backing split; method not specified; not implemented; not registered**
- Owner: integrator on `0.0.38` after principal questions (git protocol, then `qualia:` / `qi` / multi-chain UTXO)
- Does not unblock: CSCP-08 (live relay), CSCP-12 (datatracker), Internet honesty flags

This is not a W3C DID Method specification. It freezes collisions and the ledger/hostname split so a later spec does not mint a stub or fold human identity into QRC.

## What a DID method is (and is not)

A W3C DID is `did:<method>:<method-specific-id>`. The method must define create, read, update, deactivate, resolution metadata, verification relationships, and security/privacy considerations. QDNF already requires that bar before anyone calls a string a DID method ([identifier-resolution.md](../../../identifier-resolution.md) §3).

A DID method publishes **identity and a DID document**. CSCP may put mailbox keys and approved **relay hints** in `service` entries. It does **not** forward datagrams. Git objects, UTXO commitments, and `did.json` on a hostname do not replace Gate B.

A natural person is not a DID ([identifier-fabric-integration.md](../../../identifier-fabric-integration.md)). Pairwise invitation identifiers remain available (`did:peer`-class). `did:qi` is for HCAI **node / organisation / contextual instrument** identity, not a universal person key.

## Do not reuse `qualia:` as the method name

| Existing form | Job today | Collision if reused as a DID method |
|---|---|---|
| `qualia://…` | Desktop / QApp / Webizen custom URL scheme (`qualia://chora/universe`, `qualia://webid/…`) | Browser and OS already dispatch this as local app content, not DID resolution |
| `qualia:` in RDF / JSON-LD | Ontology prefix (`qualia:credential`, SHACL, HCF) | Compact CURIE, not a resolver |
| `urn:qualia:…` | Profile and registry URNs | Persistence of engine profiles, not controller identity |
| `did:q42:…` | Q42 Resource Coordinate: 60-bit FNV + MSB for VM/storage dispatch (`identity/identifier.rs`) | Not create/read/update/deactivate; not a content digest; not a route |

**Chosen method name:** `did:qi:` — Qualia Identifier. Short, DID-Core shaped, and not one of the four collisions above.

**Compact display form (optional later):** `qi:<id>` MAY expand to `did:qi:<id>` in UI. Do not register a competing IANA URI scheme until a method spec exists. Do not teach software that `qualia:alice` is a DID.

**Rejected:** `did:qualia:` (collides with the RDF prefix and will be mis-CURIE’d). `did:q42:` as the HCAI method (QRC stays QRC; standards-backlog item 2 must not fold HCAI identity into the pointer parser). Bare `qi:` as the only identifier (DID tooling and HCAI `did:web` Frontdoor expect `did:`).

## Hostname is an alias, not the chain

HCAI already uses `did:web:<domain>` as a DNS/HTTPS Frontdoor with `alsoKnownAs: did:q42:…` and exactly one `HCAIAgreementNegotiation` service. That path **requires** DNS and Web PKI. QDNF marks `did:web` legacy-dependent ([cryptographic-profile.md](../../../cryptographic-profile.md) §11).

A hostname method that **is** the DID (`did:qi:example.org` with the same resolution rules as `did:web`) locks the human-centric identifier to ICANN + HTTPS. That is the opposite of chain- and operator-portability.

**Rule:** the stable DID is `did:qi:<self-certifying-id>` (genesis document hash or controller key, multibase). A hostname, if any, is `alsoKnownAs` / Frontdoor / CSCP mailbox hint. Rotating DNS, GitHub Pages, or a registrar MUST NOT change the DID.

`did:web` remains the LIG compatibility Frontdoor. Native HCAI identity is `did:qi`.

## Backings (parameterized; not one ticker)

Resolution is **multi-backing**. The method-specific-id does **not** embed a single chain or git host. The DID document (and resolution metadata) lists attestations. All attestations MUST commit the same canonical document digest.

### 1. Git protocol (not GitHub)

Invitation-scoped and Isolated:

- DID document as a signed git object (commit or annotated tag).
- Resolution: `git fetch` / bundle from an **invitation-scoped remote**, not a public DHT and not github.com as operator.
- `ContactDescriptor.generation` maps to commit / tag. An older commit is stale, not current.
- `git://` is unauthenticated. Use git+ssh, authenticated smart HTTP, or a `.bundle` on sneakernet/IPFS for Isolated receipts.
- A reachable git daemon or shared seed is infrastructure for **documents**, the same class as “someone accepts inbound or both sides push to a third remote.” Radicle seeds replicate objects; they are not CSCP relays.

### 2. UTXO family (multi-chain, not one blockchain)

The principal asked for a hostname method that supports **multiple types of blockchains**, via UTXO (Unspent Transaction Output — not “UXTO”).

UTXO is the right **family** for optional public ledger attestation:

- Create: commit the document digest in an output (OP_RETURN-class, Taproot commitment, or an equivalent bounded script on that chain).
- Update: spend that output and create the next commitment (the `did:btcr` continuation pattern, parameterized).
- Deactivate: spend to a registered tombstone.

**Do not** name the method `did:btc`, `did:xec`, or `did:bch`. A ticker in the method name makes chain migration a new identity.

**Do** parameterize each attestation with a chain id, not a brand nickname. Bitcoin-family networks use CAIP-2 `bip122:<genesis-block-hash>`. eCash (XEC) in this tree already speaks UTXO for **payments** (`qualia-client-core` wallet / Chronik). Payment UTXOs MAY fund a CSCP `RelayLease`; they are not the DID.

Each UTXO attestation names:

```text
chain_id    = CAIP-2 (e.g. bip122:<genesis>)
outpoint    = txid || vout   (endianness and txid encoding fixed per chain profile)
commitment  = digest of the canonical DID document
```

Several chains MAY attest the same digest. Resolvers accept a chain only if the local constitution lists that `chain_id`. Account-based ledgers (EVM, and similar) are a **different** attestation profile; they are not smuggled under UTXO. Cardano-style eUTXO is not Bitcoin script; it needs its own profile or it stays out.

Hostname + UTXO together: the human types a name (DNS Frontdoor or local alias). Control of the **DID** is the controller key plus optional UTXO spends on whatever attested chains the constitution allows. The hostname never owns the method.

### 3. Invitation mailbox (already CSCP)

Default CSCP discovery stays invitation-scoped. `public_dht` remains off unless Direct is permitted. A `did:qi` document under relay-only MUST NOT export access-network locators; relay hints only.

## Comparators (reuse, do not re-implement)

| Method / pattern | Use |
|---|---|
| `did:peer` | Pairwise invitation; closest existing method to CSCP mailbox |
| `did:web` | HCAI Frontdoor / LIG only |
| `did:key` | Static keys; no document updates |
| `did:btcr` | Proof that UTXO spend = update; do not bind HCAI to Bitcoin alone |
| `did:ion` | Sidetree + Bitcoin batching; different architecture |
| `did:pkh` | CAIP-10 account ids; account model, not UTXO |
| Historical `did:git` drafts | Git object backing; do not take GitHub as the method |

## What this does not authorise

- Implementing a resolver, minting test DIDs, or registering `qi` in DID Spec Registries.
- Setting Internet honesty flags, inventing a relay URL, or claiming git/UTXO/DNS is Gate B.
- Treating `parse_did_q42` as DID-Core resolution.
- A public DHT of protected persons.
- One-chain lock-in, GitHub-as-method, or `git://` as the security profile.

## Human input still needed

1. Confirm `did:qi` as the method string (vs a different short name that does not collide).
2. Whether a first UTXO profile is eCash (already in-tree for payments), Bitcoin-family generic, or none until a constitution lists chains.
3. Whether to write the actual DID Method spec next (W3C CG-style; completeness bar: CRUD, test vectors, security/privacy — not a stub).
