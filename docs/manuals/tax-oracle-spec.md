# Qualia-DB Tax Oracle Specification

**Target Audience:** Municipal Taxation Authorities, Sovereign Digital Ministries, Regulatory Developers
**Protocol:** N3Logic Tax Ontology via `did:gov` and `did:git`

## The Fiduciary Premise
The Intentional Computing ecosystem is built on the axiom of Fiduciary Supremacy and the Threshold Shift License. When a corporate entity (Provider) pays a natural person (Principal) via Interledger Protocol (ILP) micro-transactions for data routing or processing, this constitutes an economic event. 

To prevent the natural person from incurring unmanaged tax liabilities, the `Qualia-DB` Edge Daemon implements a **Tax Escrow Router**. This system relies on dynamic "Tax Oracles" published by legitimate Taxation Authorities to automatically split and escrow tax obligations.

This document outlines the technical requirements for Taxation Authorities to publish comprehensive, machine-readable tax obligations into the network.

---

## 1. Decentralized Identity (`did:gov`)
Taxation Authorities must not rely on centralized API servers that can go offline or be spoofed. Authorities must establish a Decentralized Identifier (DID) anchored to a cryptographic root of trust (e.g., `did:gov:us:ny` or `did:gov:uk:hmrc`).
- The public keys associated with this DID will be used to digitally sign the `.q42` tax ontologies.
- The `Qualia-DB` daemon natively validates the cryptographic signature of the tax rules against the `did:gov` registry before applying them.

## 2. Publishing the Tax Ontology (`.q42` DAG)
Tax obligations must be published as an immutable N3Logic Directed Acyclic Graph (a `.q42` file). The daemon evaluates this graph against the transaction metadata to determine the exact tax owed.

### Required N3Logic Predicates
The authority's `.q42` DAG must contain the following structural triples:

1. **Geographic/Jurisdictional DOAP Binding**:
   The ontology must define exactly which geographic polygons or logical jurisdictions the tax applies to.
   ```n3
   @prefix tax: <http://qualia.io/ns/tax#> .
   @prefix geo: <http://www.w3.org/2003/01/geo/wgs84_pos#> .
   
   <did:gov:us:ny> tax:claimsJurisdiction [
       geo:polygon "[[40.7128,-74.0060], ...]"
   ] .
   ```

2. **Transaction Classification**:
   The ontology must define rules matching the `data_usage_intent` of the ILP transaction. (e.g., is this a "Telemetry Routing" fee, or a "Digital Healthcare Processing" fee?).
   ```n3
   { ?transaction a tax:TelemetryRoutingFee } => { ?transaction tax:applicableRate "0.10"^^xsd:decimal } .
   { ?transaction a tax:HealthcareDataFee } => { ?transaction tax:applicableRate "0.00"^^xsd:decimal } .
   ```

3. **Stablecoin Escrow Destination**:
   The authority must cryptographically specify the exact stablecoin address (e.g., USDC on Polygon, or a Central Bank Digital Currency address) where the escrowed funds must be routed.
   ```n3
   <did:gov:us:ny> tax:escrowWallet "0x742d35Cc6634C0532925a3b844Bc454e4438f44e" .
   <did:gov:us:ny> tax:acceptedCurrency "USDC" .
   ```

## 3. The Local Daemon Evaluation Flow
When a provider attempts to negotiate terms with a natural person's local daemon:
1. The daemon checks the local physical coordinate or declared DOAP of the user.
2. It fetches the latest cryptographically signed `.q42` ontology from the corresponding Taxation Authority (`did:gov`).
3. The N3Logic Adjudicator (Webizen VM) evaluates the ILP offset against the `applicableRate` predicates.
4. The API mathematically splits the incoming ILP transaction into two streams:
   - **Stream A (Profit)**: Routed to the user's personal wallet.
   - **Stream B (Tax Obligation)**: Routed directly to the `tax:escrowWallet`.

## 4. Updates and Temporal Evolution
Because `.q42` files are immutable, Taxation Authorities cannot "edit" past tax rules. If a tax rate changes (e.g., a new fiscal year), the Authority must publish a *new* `.q42` file, cryptographically linked to the old one using a `did:git` evolutionary commit.

The local daemon will inherently respect the new rules for all future transactions, while maintaining perfect cryptographic provenance of exactly what the tax rate was at the moment of any historical transaction.
