# ADR 0003: Permissive Commons Billing Gates

## Status
Accepted

## Context
To support decentralized human agency and stewardship, data owners must have the ability to enforce economic or work-based boundaries (e.g., micropayments, algorithmic proof-of-work, verifiable credential presentations) before their data is accessed. Doing this at the application layer is slow and vulnerable to bypass.

## Decision
We have embedded economic and access control gates directly into the bare-metal database routing logic using the Fifth Vector (`metadata` slot) of the `QualiaQuin`.

We established specific `PermissiveRoutingLane` definitions:
- `PassthroughStandard = 0x00`
- `EnforcePermissiveCommons = 0x01`
- `EnforceBilateralMicroCommons = 0x02`

These lanes check bitwise signatures (e.g., `MASK_COMMERCIAL_BILLABLE_GATE`, `MASK_WORK_OBLIGATION_SATISFIED`) on every hardware cycle during data projection.

## Consequences
- **Positive:** Unbeatable security. If an entity requests data and their hardware signature lacks the verified permissive access bit, the data never leaves the disk sector.
- **Positive:** Frictionless micro-economies. Applications can hook directly into these core gates without writing expensive custom authorization middleware.
- **Negative:** Access logic is computationally bound to a 16-bit limitation within the 64-bit metadata field. Complex boolean authorization policies must be externalized or compiled down into simpler bitwise flags.
