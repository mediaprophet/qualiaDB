# HLT-R1 — Consent-contract review (2026-09-05)

**Status:** instrument review complete · **D5 Review Gate A not closed**  
**Branch:** `cursor/poet-grok-handover-ac52` off `0.0.36-dev` (`37ec26c9`)  
**Packet:** independent review of HLT-03, not a reimplementation

## Verdict

The HLT-03 cryptographic grant already met principal/scope immutability, fail-closed
expiry, principal-only revocation, and “no private key on the grant”. Replay and
reactivation were incomplete (`ReplayDetected` unused; omitting a receipt
re-authorized). Poet projection fail-opened missing scope as “All categories” and
missing expiry as Active. The grant UI offered `clinical_notes`, which is not a
`ConsentScope` bit.

Those defects are repaired. Gate A still needs a human/`D5` close. Next packet:
`HLT-07`.

## How to run

```bash
cargo test -p qualia-core-db --lib consent_contract
cargo test -p poet --lib health_views
```

Measured here (rustc 1.98.1): **12** consent_contract passed; **27** health_views passed.
